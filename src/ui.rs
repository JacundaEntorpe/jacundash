//! A janela: campo de busca em cima, lista por categoria embaixo. Fica criada o tempo todo
//! (escondida) pra abrir na hora.

use crate::config::{Compartilhada, Config, pasta_config};
use crate::engine::{self, Batch, Engine, Row};
use crate::model::{Effect, Item};
use crate::marcas::{self, Marca, Marcas};
use crate::model::Action;
use crate::providers::apps;
use crate::sistema;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

const CSS_PADRAO: &str = include_str!("estilo.css");

pub struct Ui {
    pub window: gtk::ApplicationWindow,
    pub app: gtk::Application,
    caixa: gtk::Box,
    entry: gtk::Entry,
    /// Etiqueta da palavra-chave ("vs " vira "Repositórios" do lado do campo).
    etiqueta: gtk::Label,
    palavra_ativa: RefCell<Option<String>>,
    list: gtk::ListBox,
    scroll: gtk::ScrolledWindow,
    rodape: gtk::Label,
    /// Coluna da prévia, à direita da lista.
    previa: gtk::Box,
    /// O próprio Rc, pros botões das linhas chamarem a tela de volta.
    eu: RefCell<std::rc::Weak<Ui>>,
    /// Os botões de ação de cada linha (só aparecem na selecionada e na com o mouse em cima).
    botoes: RefCell<Vec<Option<gtk::Box>>>,
    pub engine: Rc<Engine>,
    /// Muda com o app aberto (aba de configurações).
    pub cfg: Compartilhada,
    pub configuracoes: RefCell<Option<gtk::Window>>,
    /// Resultados fixados no topo e escondidos.
    pub marcas: RefCell<Marcas>,
    /// Cores do tema (claro/sistema) e o estilo.css do usuário: recarregam sem reiniciar.
    css_tema: gtk::CssProvider,
    css_usuario: gtk::CssProvider,
    /// Vigias da pasta de config (precisam ficar vivas).
    vigias: RefCell<Vec<gio::FileMonitor>>,
    /// Plugin ou atalho web mudou: reinicia quando a janela fechar.
    reiniciar_ao_fechar: std::cell::Cell<bool>,
    /// Ações que já apareceram, por categoria (a aba de configurações lista elas pra esconder).
    pub acoes_vistas: RefCell<std::collections::BTreeMap<String, Vec<String>>>,
    tx: async_channel::Sender<Batch>,
    efeitos_tx: async_channel::Sender<Vec<Effect>>,
    uso_apps: Arc<Mutex<std::collections::HashMap<String, u32>>>,
    recentes: Arc<Mutex<Vec<Item>>>,
    estado: RefCell<Estado>,
}

#[derive(Default)]
struct Estado {
    geracao: u64,
    resultados: Vec<Option<Vec<Item>>>,
    linhas: Vec<Row>,
    selecionado: usize,
    modo_palavra: bool,
    desenho_marcado: bool,
    /// Ações da linha selecionada abertas (Shift+Enter): (linha, índice da ação)
    acoes: Option<(usize, usize)>,
    /// Linha com o mouse em cima.
    hover: Option<usize>,
}

impl Ui {
    pub fn new(app: &gtk::Application, engine: Engine, cfg: Compartilhada, uso_apps: Arc<Mutex<std::collections::HashMap<String, u32>>>, recentes: Arc<Mutex<Vec<Item>>>) -> Rc<Ui> {
        let (css_tema, css_usuario) = carregar_css();
        let cfg_compartilhada = cfg;
        let cfg = cfg_compartilhada.read().unwrap().clone();
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Jacundash")
            .decorated(false)
            .resizable(false)
            .default_width(cfg.aparencia.largura)
            .build();
        window.add_css_class("jacundash");
        posicionar(&window, &cfg);

        let caixa = gtk::Box::new(gtk::Orientation::Vertical, 0);
        caixa.add_css_class("caixa");
        caixa.set_size_request(cfg.aparencia.largura, -1);

        let entry = gtk::Entry::builder()
            .placeholder_text("Buscar apps, contas, web... (help mostra tudo)")
            .primary_icon_name("system-search-symbolic")
            .hexpand(true)
            .build();
        entry.add_css_class("busca");

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_activate_on_single_click(true);
        list.add_css_class("lista");

        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(cfg.aparencia.altura_max)
            .child(&list)
            .build();
        scroll.set_visible(false);

        let rodape = gtk::Label::new(Some("Enter abre  ·  Shift+Enter ações  ·  Esc fecha"));
        rodape.add_css_class("rodape");
        rodape.set_xalign(0.0);
        rodape.set_visible(false);

        let previa = gtk::Box::new(gtk::Orientation::Vertical, 0);
        // Largura travada (External: o conteúdo não consegue pedir mais), e rola se for comprido.
        // Senão um bloco de código largo no README estica a prévia e aperta a lista.
        let previa_rolagem = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::External)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .propagate_natural_height(true)
            .max_content_height(cfg.aparencia.altura_max)
            .width_request(crate::previa::LARGURA)
            .hexpand(false)
            .child(&previa)
            .build();
        previa_rolagem.set_visible(false);
        let corpo = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        scroll.set_hexpand(true);
        corpo.append(&scroll);
        corpo.append(&previa_rolagem);

        let etiqueta = gtk::Label::new(None);
        etiqueta.add_css_class("etiqueta");
        etiqueta.set_visible(false);
        etiqueta.set_valign(gtk::Align::Center);
        let linha_busca = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        linha_busca.add_css_class("linha-busca");
        linha_busca.append(&etiqueta);
        linha_busca.append(&entry);
        caixa.append(&linha_busca);
        caixa.append(&corpo);
        caixa.append(&rodape);
        window.set_child(Some(&caixa));
        window.set_default_size(cfg.aparencia.largura, -1);

        let (tx, rx) = async_channel::unbounded::<Batch>();
        let (efeitos_tx, efeitos_rx) = async_channel::unbounded::<Vec<Effect>>();
        let n = engine.providers.len();
        let ui = Rc::new(Ui {
            window,
            app: app.clone(),
            caixa,
            entry,
            etiqueta,
            palavra_ativa: RefCell::default(),
            list,
            scroll,
            rodape,
            previa,
            eu: RefCell::default(),
            botoes: RefCell::default(),
            engine: Rc::new(engine),
            cfg: cfg_compartilhada,
            configuracoes: RefCell::default(),
            marcas: RefCell::new(Marcas::carregar()),
            css_tema,
            css_usuario,
            vigias: RefCell::default(),
            reiniciar_ao_fechar: std::cell::Cell::new(false),
            acoes_vistas: RefCell::new(
                std::fs::read_to_string(crate::config::pasta_estado().join("acoes-vistas.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default(),
            ),
            tx,
            efeitos_tx,
            uso_apps,
            recentes,
            estado: RefCell::new(Estado { resultados: vec![None; n], ..Default::default() }),
        });

        *ui.eu.borrow_mut() = Rc::downgrade(&ui);
        ui.aplicar_tema();
        ui.vigiar();

        // Respostas dos provedores chegando
        let u = ui.clone();
        glib::spawn_future_local(async move {
            while let Ok(b) = rx.recv().await {
                {
                    let mut e = u.estado.borrow_mut();
                    if b.generation != e.geracao {
                        continue; // resposta de uma busca antiga
                    }
                    e.resultados[b.provider] = Some(b.items);
                }
                u.marcar_desenho();
            }
        });
        // Efeitos devolvidos por callbacks de plugin
        let u = ui.clone();
        glib::spawn_future_local(async move {
            while let Ok(efeitos) = efeitos_rx.recv().await {
                if efeitos.is_empty() {
                    u.esconder();
                }
                for ef in efeitos {
                    u.executar(&ef, None);
                }
            }
        });

        // Provedor pediu pra refazer a busca (resposta da IA chegando)
        let (atualizar_tx, atualizar_rx) = async_channel::bounded::<()>(1);
        engine::ligar_atualizacao(atualizar_tx);
        let u = ui.clone();
        glib::spawn_future_local(async move {
            while atualizar_rx.recv().await.is_ok() {
                if u.window.is_visible() {
                    u.rebuscar();
                }
            }
        });

        let (copiar_tx, copiar_rx) = async_channel::unbounded::<String>();
        engine::ligar_copiar(copiar_tx);
        let u = ui.clone();
        glib::spawn_future_local(async move {
            while let Ok(texto) = copiar_rx.recv().await {
                copiar(&u.window, &texto);
            }
        });

        let u = ui.clone();
        ui.entry.connect_changed(move |_| {
            // "vs " no começo vira a etiqueta "Repositórios" e sai do campo
            if u.palavra_ativa.borrow().is_none() {
                let texto = u.entry.text().to_string();
                if let Some((palavra, resto)) = texto.split_once(' ') {
                    if u.engine.providers.iter().any(|p| p.keyword() == palavra) {
                        u.trocar_etiqueta(Some(palavra.to_string()));
                        u.entry.set_text(resto); // isso chama de novo, já com a etiqueta
                        u.entry.set_position(-1);
                        return;
                    }
                }
            }
            u.buscar()
        });

        let u = ui.clone();
        ui.list.connect_row_activated(move |_, row| {
            let i = row.index() as usize;
            u.estado.borrow_mut().selecionado = i;
            u.ativar(false);
        });

        let teclas = gtk::EventControllerKey::new();
        teclas.set_propagation_phase(gtk::PropagationPhase::Capture);
        let u = ui.clone();
        teclas.connect_key_pressed(move |_, tecla, _, mods| u.tecla(tecla, mods));
        ui.window.add_controller(teclas);

        {
            let u = ui.clone();
            ui.window.connect_is_active_notify(move |w| {
                if !w.is_active() && w.is_visible() && u.cfg.read().unwrap().busca.fechar_ao_perder_foco {
                    u.esconder();
                }
            });
        }
        ui
    }

    pub fn alternar(self: &Rc<Self>) {
        if self.window.is_visible() {
            self.esconder();
        } else {
            self.mostrar();
        }
    }

    pub fn mostrar(self: &Rc<Self>) {
        self.mostrar_com("");
    }

    pub fn mostrar_com(self: &Rc<Self>, texto: &str) {
        engine::VISIVEL.store(true, std::sync::atomic::Ordering::Relaxed);
        self.trocar_etiqueta(None);
        self.entry.set_text(texto);
        self.buscar();
        self.window.present();
        self.entry.grab_focus();
        self.entry.set_position(-1);
    }

    /// O texto da busca com a palavra-chave da etiqueta na frente ("vs" + "poke" = "vs poke").
    fn texto_completo(&self) -> String {
        let texto = self.entry.text().to_string();
        match self.palavra_ativa.borrow().as_ref() {
            Some(p) => format!("{p} {texto}"),
            None => texto,
        }
    }

    fn trocar_etiqueta(&self, palavra: Option<String>) {
        let categoria = palavra.as_ref().and_then(|p| self.engine.providers.iter().find(|x| x.keyword() == p)).map(|p| p.category().to_string());
        self.etiqueta.set_text(categoria.as_deref().unwrap_or(""));
        self.etiqueta.set_visible(categoria.is_some());
        self.entry.set_placeholder_text(Some(if categoria.is_some() { "buscar aqui dentro... (Backspace volta)" } else { "Buscar apps, contas, web... (help mostra tudo)" }));
        *self.palavra_ativa.borrow_mut() = palavra;
    }

    pub fn esconder(&self) {
        engine::VISIVEL.store(false, std::sync::atomic::Ordering::Relaxed);
        self.window.set_visible(false);
        if self.reiniciar_ao_fechar.get() {
            self.recarregar();
            return;
        }
        // Guarda as ações vistas (pra aba de configurações) quando a janela fecha
        if let Ok(json) = serde_json::to_string(&*self.acoes_vistas.borrow()) {
            let _ = std::fs::create_dir_all(crate::config::pasta_estado());
            let _ = std::fs::write(crate::config::pasta_estado().join("acoes-vistas.json"), json);
        }
    }

    /// Refaz a busca sem limpar a lista nem perder a linha selecionada: cada provedor
    /// troca os seus resultados quando responder.
    fn rebuscar(self: &Rc<Self>) {
        let texto = self.texto_completo();
        let geracao = {
            let mut e = self.estado.borrow_mut();
            e.geracao += 1;
            e.geracao
        };
        self.engine.search(&texto, geracao, &self.tx, &self.cfg.read().unwrap());
    }

    /// As cores do tema escolhido (claro, escuro ou do sistema).
    pub fn aplicar_tema(&self) {
        let tema = self.cfg.read().unwrap().aparencia.tema.clone();
        self.css_tema.load_from_string(&crate::tema::css(&tema));
    }

    /// Vigia ~/.config/jacundash: config.toml editado na mão recarrega na hora, estilo.css troca
    /// o visual na hora, e plugin ou atalho web novo reinicia o jacundash quando a janela fechar.
    fn vigiar(self: &Rc<Self>) {
        let base = pasta_config();
        let plugins = base.join("plugins");
        let mut pastas = vec![base.clone(), plugins.clone(), base.join("atalhos-web")];
        if let Ok(e) = std::fs::read_dir(&plugins) {
            pastas.extend(e.flatten().map(|x| x.path()).filter(|p| p.is_dir()));
        }
        for pasta in pastas {
            let _ = std::fs::create_dir_all(&pasta);
            let Ok(vigia) = gio::File::for_path(&pasta).monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE) else { continue };
            vigia.set_rate_limit(500);
            let fraco = Rc::downgrade(self);
            let base = base.clone();
            vigia.connect_changed(move |_, arquivo, _, evento| {
                use gio::FileMonitorEvent as E;
                if !matches!(evento, E::ChangesDoneHint | E::Created | E::Deleted | E::MovedIn | E::MovedOut | E::Renamed) {
                    return;
                }
                let Some(u) = fraco.upgrade() else { return };
                let Some(caminho) = arquivo.path() else { return };
                let nome = caminho.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if caminho.parent() == Some(base.as_path()) && nome == "config.toml" {
                    if crate::config::Config::mudou_por_fora() {
                        *u.cfg.write().unwrap() = crate::config::Config::carregar();
                        u.aplicar_aparencia();
                        u.aplicar_tema();
                    }
                } else if caminho.parent() == Some(base.as_path()) && nome == "estilo.css" {
                    u.recarregar_estilo();
                } else if caminho.parent() != Some(base.as_path()) || nome == "plugins" || nome == "atalhos-web" {
                    // Plugins e atalhos web são carregados uma vez só: reinicia (com a janela fechada)
                    if u.window.is_visible() {
                        u.reiniciar_ao_fechar.set(true);
                    } else {
                        u.recarregar();
                    }
                }
            });
            self.vigias.borrow_mut().push(vigia);
        }
    }

    fn recarregar_estilo(&self) {
        let arquivo = pasta_config().join("estilo.css");
        if arquivo.exists() {
            self.css_usuario.load_from_path(&arquivo);
        } else {
            self.css_usuario.load_from_string("");
        }
    }

    /// Aplica largura, altura e distância do topo depois de mudar a config.
    pub fn aplicar_aparencia(&self) {
        let cfg = self.cfg.read().unwrap();
        self.caixa.set_size_request(cfg.aparencia.largura, -1);
        self.window.set_default_size(cfg.aparencia.largura, -1);
        self.scroll.set_max_content_height(cfg.aparencia.altura_max);
        posicionar_topo(&self.window, &cfg);
    }

    /// Encerra e sobe de novo em segundo plano: relê plugins, atalhos web e tema.
    pub fn recarregar(&self) {
        if let Ok(eu) = std::env::current_exe() {
            // Espera este sair (e soltar o nome no D-Bus) antes do novo subir
            let _ = std::process::Command::new("sh")
                .args(["-c", "sleep 0.4; exec \"$0\" --fundo", &eu.to_string_lossy()])
                .stdin(std::process::Stdio::null())
                .spawn();
        }
        self.app.quit();
    }

    fn buscar(self: &Rc<Self>) {
        let texto = self.texto_completo();
        let geracao = {
            let mut e = self.estado.borrow_mut();
            e.geracao += 1;
            for r in e.resultados.iter_mut() {
                *r = None;
            }
            e.selecionado = 0;
            e.acoes = None;
            let palavra = texto.split(' ').next().unwrap_or("");
            e.modo_palavra = texto.contains(' ') && self.engine.providers.iter().any(|p| p.keyword() == palavra);
            e.geracao
        };
        self.engine.search(&texto, geracao, &self.tx, &self.cfg.read().unwrap());
        // Busca vazia: a lista some na hora, sem esperar ninguém
        if texto.trim().is_empty() {
            self.marcar_desenho();
        }
    }

    /// Junta várias respostas que chegam juntas num desenho só.
    fn marcar_desenho(self: &Rc<Self>) {
        if std::mem::replace(&mut self.estado.borrow_mut().desenho_marcado, true) {
            return;
        }
        let u = self.clone();
        glib::idle_add_local_once(move || {
            u.estado.borrow_mut().desenho_marcado = false;
            u.desenhar();
        });
    }

    /// Tira os escondidos, sobe os fixados e coloca "Fixar" e "Esconder" nas ações de cada item.
    fn com_marcas(&self, resultados: &[Option<Vec<Item>>]) -> Vec<Option<Vec<Item>>> {
        let m = self.marcas.borrow();
        let acoes = self.cfg.read().unwrap().acoes.clone();
        let mut vistas = self.acoes_vistas.borrow_mut();
        resultados
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let r = r.as_ref()?;
                let categoria_padrao = self.engine.providers[i].category();
                Some(
                    r.iter()
                        .filter_map(|it| {
                            if matches!(it.effect, Effect::None) {
                                return Some(it.clone()); // dicas e avisos não se marcam
                            }
                            let ch = marcas::chave(it.category.as_deref().unwrap_or(categoria_padrao), it);
                            if m.escondido(&ch) {
                                return None;
                            }
                            let mut it = it.clone();
                            let fixado = m.fixado(&ch);
                            if fixado {
                                it.score += 1.0;
                                it.subtitle = if it.subtitle.is_empty() { "fixado".into() } else { format!("fixado  ·  {}", it.subtitle) };
                            }
                            let rotulo = if fixado { "Desafixar" } else { "Fixar no topo" };
                            it.actions.push(Action { label: rotulo.into(), icon: "pin".into(), effect: Effect::Internal { action: "fixar".into() } });
                            it.actions.push(Action { label: "Esconder este resultado".into(), icon: "view-hidden".into(), effect: Effect::Internal { action: "esconder".into() } });
                            // Anota as ações da categoria e tira as que estão escondidas nas configurações
                            let categoria = it.category.clone().unwrap_or(categoria_padrao.to_string());
                            let lista = vistas.entry(categoria.clone()).or_default();
                            for a in &it.actions {
                                if !matches!(a.effect, Effect::Internal { .. }) && !lista.contains(&a.label) {
                                    lista.push(a.label.clone());
                                }
                            }
                            it.actions.retain(|a| !acoes.escondida(&categoria, &a.label));
                            Some(it)
                        })
                        .collect(),
                )
            })
            .collect()
    }

    /// Fixa/desafixa ou esconde a linha selecionada.
    fn marcar(self: &Rc<Self>, esconder: bool) {
        let marca = {
            let e = self.estado.borrow();
            let Some(linha) = e.linhas.get(e.selecionado) else { return };
            Marca { chave: marcas::chave(&linha.category, &linha.item), titulo: linha.item.title.clone(), categoria: linha.category.clone() }
        };
        if esconder {
            self.marcas.borrow_mut().esconder(marca);
        } else {
            self.marcas.borrow_mut().alternar_fixado(marca);
        }
        self.estado.borrow_mut().acoes = None;
        self.desenhar();
    }

    fn desenhar(&self) {
        let mut e = self.estado.borrow_mut();
        let resultados = self.com_marcas(&e.resultados);
        e.linhas = engine::arrange(&self.engine, &resultados, &self.cfg.read().unwrap(), e.modo_palavra);
        self.list.remove_all();
        self.botoes.borrow_mut().clear();
        e.hover = None;
        for (i, linha) in e.linhas.iter().enumerate() {
            let acoes = e.acoes.filter(|(l, _)| *l == i).map(|(_, a)| a);
            self.list.append(&self.linha(i, linha, acoes));
        }
        // Reserva o espaço da prévia enquanto algum resultado tiver (assim a janela não pula a cada seta)
        let cfg = self.cfg.read().unwrap();
        let com_previa = cfg.extras.previa && e.linhas.iter().any(|l| l.item.preview.is_some());
        if let Some(rolagem) = self.previa.parent().and_then(|v| v.parent()) {
            rolagem.set_visible(com_previa); // a prévia fica dentro de um viewport da rolagem
        }
        let largura = cfg.aparencia.largura + if com_previa { crate::previa::LARGURA + 8 } else { 0 };
        self.caixa.set_size_request(largura, -1);
        drop(cfg);
        if e.selecionado >= e.linhas.len() {
            e.selecionado = 0;
        }
        let tem = !e.linhas.is_empty();
        self.scroll.set_visible(tem);
        self.rodape.set_visible(tem);
        if let Some(row) = self.list.row_at_index(e.selecionado as i32) {
            self.list.select_row(Some(&row));
        }
        drop(e);
        self.atualizar_previa();
        self.mostrar_botoes();
    }

    /// Os botões de ação só ocupam espaço na linha selecionada e na com o mouse em cima: nas
    /// outras, o espaço fica pro texto.
    fn mostrar_botoes(&self) {
        let Ok(e) = self.estado.try_borrow() else { return };
        for (i, b) in self.botoes.borrow().iter().enumerate() {
            if let Some(b) = b {
                b.set_visible(i == e.selecionado || e.hover == Some(i));
            }
        }
    }

    /// Seleciona a linha i (rola até ela, atualiza prévia e botões).
    fn selecionar(&self, i: usize) {
        self.estado.borrow_mut().selecionado = i;
        if let Some(row) = self.list.row_at_index(i as i32) {
            self.list.select_row(Some(&row));
            mostrar_linha(&self.scroll, &self.list, &row);
        }
        self.atualizar_previa();
        self.mostrar_botoes();
    }

    /// Tab: começo da próxima categoria. Shift+Tab: começo desta (ou da anterior, se já estiver nele).
    fn pular_categoria(&self, frente: bool) {
        let (sel, inicios) = {
            let e = self.estado.borrow();
            if e.acoes.is_some() {
                drop(e);
                return self.mover(if frente { 1 } else { -1 });
            }
            (e.selecionado, e.linhas.iter().enumerate().filter(|(_, l)| l.first_of_group).map(|(i, _)| i).collect::<Vec<_>>())
        };
        let Some((&primeiro, &ultimo)) = inicios.first().zip(inicios.last()) else { return };
        let alvo = if frente {
            inicios.iter().copied().find(|&i| i > sel).unwrap_or(primeiro)
        } else {
            let atual = inicios.iter().copied().filter(|&i| i <= sel).last().unwrap_or(primeiro);
            if sel > atual { atual } else { inicios.iter().copied().filter(|&i| i < atual).last().unwrap_or(ultimo) }
        };
        self.selecionar(alvo);
    }

    /// Mostra a prévia da linha selecionada (se for a mesma de antes, não refaz).
    fn atualizar_previa(&self) {
        if !self.previa.parent().and_then(|v| v.parent()).is_some_and(|r| r.is_visible()) {
            return;
        }
        let caminho = {
            let e = self.estado.borrow();
            e.linhas.get(e.selecionado).and_then(|l| l.item.preview.clone())
        };
        if self.previa.first_child().and_then(|w| w.widget_name().strip_prefix("previa:").map(String::from)) == caminho {
            return;
        }
        while let Some(filho) = self.previa.first_child() {
            self.previa.remove(&filho);
        }
        if let Some(c) = caminho {
            let p = crate::previa::montar(&c);
            p.set_widget_name(&format!("previa:{c}"));
            self.previa.append(&p);
        }
    }

    /// Abre as ações de uma linha (Shift+Enter, clique direito ou o botão "...").
    fn abrir_acoes(&self, i: usize) {
        {
            let mut e = self.estado.borrow_mut();
            if e.linhas.get(i).is_none_or(|l| l.item.actions.is_empty()) {
                return;
            }
            e.selecionado = i;
            e.acoes = if e.acoes.is_some_and(|(l, _)| l == i) { None } else { Some((i, 0)) };
        }
        self.desenhar();
    }

    fn linha(&self, indice: usize, linha: &Row, acao_aberta: Option<usize>) -> gtk::ListBoxRow {
        let caixa = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        caixa.add_css_class("item");

        let categoria = gtk::Label::new(Some(if linha.first_of_group { &linha.category } else { "" }));
        categoria.add_css_class("categoria");
        categoria.set_xalign(1.0);
        categoria.set_width_chars(11);
        categoria.set_max_width_chars(11);
        categoria.set_ellipsize(gtk::pango::EllipsizeMode::End);
        caixa.append(&categoria);

        caixa.append(&icone(&linha.item.icon, self.cfg.read().unwrap().aparencia.icone));

        let textos = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        textos.set_hexpand(true);
        let titulo = gtk::Label::new(Some(&linha.item.title));
        titulo.add_css_class("titulo");
        titulo.set_xalign(0.0);
        if linha.item.multiline {
            titulo.set_wrap(true);
            titulo.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            titulo.set_hexpand(true);
            titulo.set_max_width_chars(1);
            titulo.set_selectable(false);
            textos.set_orientation(gtk::Orientation::Vertical);
        } else {
            titulo.set_ellipsize(gtk::pango::EllipsizeMode::End);
            // Um mínimo pro título: quando falta espaço, quem encolhe primeiro é o subtítulo
            titulo.set_width_chars(linha.item.title.chars().count().min(26) as i32);
        }
        textos.append(&titulo);
        if !linha.item.subtitle.is_empty() {
            let sub = gtk::Label::new(Some(&linha.item.subtitle));
            sub.add_css_class("subtitulo");
            sub.set_xalign(0.0);
            sub.set_hexpand(true);
            sub.set_ellipsize(gtk::pango::EllipsizeMode::End);
            textos.append(&sub);
        }
        caixa.append(&textos);

        // Pro mouse: as primeiras ações viram botões na linha (aparecem com o mouse em cima ou
        // na linha selecionada), e o "..." abre todas
        let eu = self.eu.borrow().upgrade();
        if let (Some(eu), false) = (eu, linha.item.actions.is_empty()) {
            let botoes = gtk::Box::new(gtk::Orientation::Horizontal, 2);
            botoes.add_css_class("botoes");
            botoes.set_visible(false);
            self.botoes.borrow_mut().push(Some(botoes.clone()));
            // As escolhidas nas configurações (na ordem delas); sem escolha, as 3 primeiras
            let escolhidas = self.cfg.read().unwrap().acoes.botoes.get(&linha.category).cloned();
            let botoes_da_linha: Vec<&Action> = match &escolhidas {
                Some(rotulos) => rotulos.iter().filter_map(|r| linha.item.actions.iter().find(|a| &a.label == r)).collect(),
                None => linha.item.actions.iter().filter(|a| !matches!(a.effect, Effect::Internal { .. })).take(3).collect(),
            };
            for a in botoes_da_linha {
                let b = gtk::Button::from_icon_name(if a.icon.is_empty() { "system-run" } else { &a.icon });
                b.add_css_class("flat");
                b.add_css_class("botao-acao");
                b.set_tooltip_text(Some(&a.label));
                b.set_focusable(false);
                let (u, ef, prov) = (eu.clone(), a.effect.clone(), linha.provider);
                b.connect_clicked(move |_| {
                    u.estado.borrow_mut().selecionado = indice;
                    u.executar(&ef, Some(prov));
                });
                botoes.append(&b);
            }
            let mais = gtk::Button::from_icon_name("view-more-symbolic");
            mais.add_css_class("flat");
            mais.add_css_class("botao-acao");
            mais.set_tooltip_text(Some("Todas as ações (Shift+Enter)"));
            mais.set_focusable(false);
            let u = eu.clone();
            mais.connect_clicked(move |_| u.abrir_acoes(indice));
            botoes.append(&mais);
            caixa.append(&botoes);
        }

        // Shift+Enter abre as ações dessa linha embaixo dela
        let fora = gtk::Box::new(gtk::Orientation::Vertical, 0);
        fora.append(&caixa);
        if let Some(sel) = acao_aberta {
            let acoes = gtk::Box::new(gtk::Orientation::Vertical, 0);
            acoes.add_css_class("acoes");
            for (i, (rotulo, icone_nome)) in std::iter::once(("Abrir".to_string(), String::new()))
                .chain(linha.item.actions.iter().map(|a| (a.label.clone(), a.icon.clone())))
                .enumerate()
            {
                let b = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                b.add_css_class("acao");
                if i == sel {
                    b.add_css_class("acao-selecionada");
                }
                b.append(&icone(&icone_nome, 16));
                b.append(&gtk::Label::new(Some(&rotulo)));
                acoes.append(&b);
            }
            fora.append(&acoes);
        }
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&fora));
        if self.botoes.borrow().len() <= indice {
            self.botoes.borrow_mut().push(None); // linha sem ações
        }
        // Clique direito também abre as ações; o mouse em cima mostra os botões
        if let Some(eu) = self.eu.borrow().upgrade() {
            let direito = gtk::GestureClick::builder().button(3).build();
            let u = eu.clone();
            direito.connect_pressed(move |_, _, _, _| u.abrir_acoes(indice));
            row.add_controller(direito);
            // O GTK avisa entrar/sair no meio do redesenho (a linha embaixo do mouse some): aí o
            // estado está ocupado, e o aviso fica pra quando ele terminar
            let mouse = gtk::EventControllerMotion::new();
            let u = eu.clone();
            mouse.connect_enter(move |_, _, _| {
                let u = u.clone();
                glib::idle_add_local_once(move || {
                    if let Ok(mut e) = u.estado.try_borrow_mut() {
                        e.hover = Some(indice);
                    }
                    u.mostrar_botoes();
                });
            });
            let u = eu.clone();
            mouse.connect_leave(move |_| {
                let u = u.clone();
                glib::idle_add_local_once(move || {
                    if let Ok(mut e) = u.estado.try_borrow_mut() {
                        if e.hover == Some(indice) {
                            e.hover = None;
                        }
                    }
                    u.mostrar_botoes();
                });
            });
            row.add_controller(mouse);
        }
        if linha.first_of_group {
            row.add_css_class("comeco-grupo");
        }
        row
    }

    fn tecla(self: &Rc<Self>, tecla: gdk::Key, mods: gdk::ModifierType) -> glib::Propagation {
        let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);
        // Backspace com o campo vazio tira a etiqueta da palavra-chave
        if tecla == gdk::Key::BackSpace && self.entry.text().is_empty() && self.palavra_ativa.borrow().is_some() {
            self.trocar_etiqueta(None);
            self.buscar();
            return glib::Propagation::Stop;
        }
        if mods.contains(gdk::ModifierType::CONTROL_MASK) && tecla == gdk::Key::comma {
            self.esconder();
            crate::config_ui::abrir(self);
            return glib::Propagation::Stop;
        }
        match tecla {
            gdk::Key::Escape => {
                if self.estado.borrow().acoes.is_some() {
                    self.estado.borrow_mut().acoes = None;
                    self.desenhar();
                } else {
                    self.esconder();
                }
            }
            gdk::Key::Down => self.mover(1),
            gdk::Key::Up => self.mover(-1),
            gdk::Key::Tab => self.pular_categoria(true),
            gdk::Key::ISO_Left_Tab => self.pular_categoria(false),
            gdk::Key::Page_Down => self.mover(5),
            gdk::Key::Page_Up => self.mover(-5),
            gdk::Key::Return | gdk::Key::KP_Enter => self.ativar(shift),
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    }

    fn mover(&self, delta: i32) {
        let mut e = self.estado.borrow_mut();
        if e.linhas.is_empty() {
            return;
        }
        if let Some((linha, acao)) = e.acoes {
            let n = e.linhas[linha].item.actions.len() as i32 + 1;
            e.acoes = Some((linha, (acao as i32 + delta).rem_euclid(n) as usize));
            drop(e);
            self.desenhar();
            return;
        }
        let n = e.linhas.len() as i32;
        let i = (e.selecionado as i32 + delta).rem_euclid(n) as usize;
        drop(e);
        self.selecionar(i);
    }

    /// O mesmo que apertar Enter (jacundash --enter).
    pub fn enter(self: &Rc<Self>) {
        if self.window.is_visible() {
            self.ativar(false);
        }
    }

    /// Enter: ação padrão. Shift+Enter: abre (ou escolhe) as ações da linha.
    fn ativar(self: &Rc<Self>, mais: bool) {
        let (efeito, provedor) = {
            let mut e = self.estado.borrow_mut();
            let Some(linha) = e.linhas.get(e.selecionado) else {
                return;
            };
            match e.acoes {
                Some((l, 0)) if l == e.selecionado => (linha.item.effect.clone(), linha.provider),
                Some((l, a)) if l == e.selecionado => (linha.item.actions[a - 1].effect.clone(), linha.provider),
                _ if mais && !linha.item.actions.is_empty() => {
                    e.acoes = Some((e.selecionado, 0));
                    drop(e);
                    self.desenhar();
                    return;
                }
                _ => {
                    // O Enter de verdade (não as ações) entra nos recentes, sem o Fixar/Esconder que a tela pôs
                    let mut original = linha.item.clone();
                    if !matches!(original.effect, Effect::None) {
                        original.actions.truncate(original.actions.len().saturating_sub(2));
                    }
                    crate::providers::recentes::registrar(&self.recentes, &original, &linha.category);
                    (linha.item.effect.clone(), linha.provider)
                }
            }
        };
        self.executar(&efeito, Some(provedor));
    }

    fn executar(self: &Rc<Self>, efeito: &Effect, provedor: Option<usize>) {
        // O callback decide depois (pela resposta) se fecha
        let fecha = !matches!(efeito, Effect::Query { .. } | Effect::None | Effect::Callback { .. })
            && !matches!(efeito, Effect::Internal { action } if action == "fixar" || action == "esconder");
        match efeito {
            Effect::Launch { app } => {
                if let Some(info) = gio_unix::DesktopAppInfo::new(app) {
                    let ctx = WidgetExt::display(&self.window).app_launch_context();
                    if let Err(e) = info.launch(&[], Some(&ctx)) {
                        eprintln!("jacundash: não abriu {app}: {e}");
                    }
                    apps::contar_uso(&self.uso_apps, app);
                }
            }
            Effect::Open { uri } => {
                let ctx = WidgetExt::display(&self.window).app_launch_context();
                if let Err(e) = gio::AppInfo::launch_default_for_uri(uri, Some(&ctx)) {
                    eprintln!("jacundash: não abriu {uri}: {e}");
                }
            }
            Effect::Copy { text } => copiar(&self.window, text),
            Effect::Exec { cmd, terminal, cwd, hold, tab } => {
                let cfg = self.cfg.read().unwrap();
                rodar(cmd, *terminal, cwd.as_deref(), *hold, tab.unwrap_or(cfg.busca.preferir_aba), &cfg)
            }
            Effect::Task { cmd, cwd, label } => sistema::tarefa(cmd.clone(), cwd.clone(), label.clone()),
            Effect::Internal { action } => match action.as_str() {
                "config" => crate::config_ui::abrir(self),
                "recarregar" => self.recarregar(),
                "fixar" => self.marcar(false),
                "esconder" => self.marcar(true),
                _ => eprintln!("jacundash: ação interna desconhecida: {action}"),
            },
            Effect::Query { text } => {
                self.trocar_etiqueta(None);
                self.entry.set_text(text);
                self.entry.set_position(-1);
            }
            Effect::Callback { data } => {
                if let Some(p) = provedor.and_then(|i| self.engine.providers.get(i)).cloned() {
                    let data = data.clone();
                    let tx = self.efeitos_tx.clone();
                    std::thread::spawn(move || {
                        let _ = tx.send_blocking(p.callback(&data));
                    });
                }
            }
            Effect::None => {}
        }
        if fecha {
            self.esconder();
        }
    }
}

/// Copia pra área de transferência. No Wayland o desktop só deixa o app copiar logo depois de
/// uma tecla ou clique de verdade (não vale pra resposta da IA que chega depois, nem pro
/// jacundash --enter); o wl-copy não tem esse problema, então ele vai na frente quando existe.
fn copiar(window: &gtk::ApplicationWindow, texto: &str) {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && sistema::tem("wl-copy") {
        use std::io::Write;
        if let Ok(mut filho) = std::process::Command::new("wl-copy").stdin(std::process::Stdio::piped()).spawn() {
            if let Some(mut entrada) = filho.stdin.take() {
                let _ = entrada.write_all(texto.as_bytes());
            }
            std::thread::spawn(move || filho.wait());
            return;
        }
    }
    WidgetExt::display(window).clipboard().set_text(texto);
}

/// Rola a lista até a linha aparecer, sem tirar o foco do campo de busca.
fn mostrar_linha(scroll: &gtk::ScrolledWindow, list: &gtk::ListBox, row: &gtk::ListBoxRow) {
    let Some(r) = row.compute_bounds(list) else { return };
    let adj = scroll.vadjustment();
    let (topo, fundo) = (r.y() as f64, (r.y() + r.height()) as f64);
    if topo < adj.value() {
        adj.set_value(topo);
    } else if fundo > adj.value() + adj.page_size() {
        adj.set_value(fundo - adj.page_size());
    }
}

fn icone(nome: &str, tamanho: i32) -> gtk::Image {
    let tema = gtk::IconTheme::for_display(&gdk::Display::default().unwrap());
    let img = if nome.is_empty() {
        gtk::Image::new()
    } else if nome.starts_with('/') {
        gtk::Image::from_file(nome)
    } else if !nome.contains(' ') && !nome.contains('.') {
        // Nome solto ("discord"): se o tema não tiver, um ícone genérico em vez do quebrado
        gtk::Image::from_icon_name(if tema.has_icon(nome) { nome } else { "application-x-executable" })
    } else {
        match gio::Icon::for_string(nome) {
            Ok(i) => gtk::Image::from_gicon(&i),
            Err(_) => gtk::Image::from_icon_name(nome),
        }
    };
    img.set_pixel_size(tamanho);
    img.add_css_class("icone");
    img
}

/// Roda um comando solto (fora do jacundash), opcionalmente dentro de um terminal.
fn rodar(cmd: &[String], terminal: bool, cwd: Option<&str>, segurar: bool, aba: bool, cfg: &Config) {
    use std::os::unix::process::CommandExt;
    // Aba nova: cada terminal tem seu jeito (Konsole pelo D-Bus, WezTerm e kitty pelo cli...).
    // Se não der, cai na janela nova logo abaixo.
    let casa = crate::config::home().to_string_lossy().into_owned();
    if terminal && aba && sistema::aba_nova(&cfg.busca.terminal, cwd.unwrap_or(&casa), cmd, segurar) {
        return;
    }
    let argv = if terminal { sistema::no_terminal(&cfg.busca.terminal, cmd, cwd, segurar, aba) } else { cmd.to_vec() };
    let Some((prog, args)) = argv.split_first() else { return };
    let mut c = std::process::Command::new(prog);
    c.args(args).stdin(std::process::Stdio::null()).process_group(0);
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    if let Err(e) = c.spawn() {
        sistema::avisar("Não deu pra rodar", &format!("{prog}: {e}"), "dialog-error");
    }
}

/// Estilo padrão, por cima as cores do tema, e por cima de tudo o estilo.css do usuário.
fn carregar_css() -> (gtk::CssProvider, gtk::CssProvider) {
    let display = gdk::Display::default().expect("sem display");
    let padrao = gtk::CssProvider::new();
    padrao.load_from_string(CSS_PADRAO);
    gtk::style_context_add_provider_for_display(&display, &padrao, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    let tema = gtk::CssProvider::new();
    gtk::style_context_add_provider_for_display(&display, &tema, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    let usuario = gtk::CssProvider::new();
    let arquivo = pasta_config().join("estilo.css");
    if arquivo.exists() {
        usuario.load_from_path(&arquivo);
    }
    gtk::style_context_add_provider_for_display(&display, &usuario, gtk::STYLE_PROVIDER_PRIORITY_USER);
    (tema, usuario)
}

/// No Wayland com layer-shell (KDE, Hyprland, Sway...): colada no topo, por cima de tudo.
/// Sem isso (X11, GNOME): uma janela comum sem borda.
fn posicionar(window: &gtk::ApplicationWindow, cfg: &Config) {
    #[cfg(feature = "layer-shell")]
    {
        use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
        if gtk4_layer_shell::is_supported() {
            window.init_layer_shell();
            window.set_namespace(Some("jacundash"));
            window.set_layer(Layer::Overlay);
            window.set_anchor(Edge::Top, true);
            window.set_keyboard_mode(KeyboardMode::OnDemand);
        }
    }
    posicionar_topo(window, cfg);
}

fn posicionar_topo(window: &gtk::ApplicationWindow, cfg: &Config) {
    #[cfg(feature = "layer-shell")]
    {
        use gtk4_layer_shell::{Edge, LayerShell};
        if gtk4_layer_shell::is_supported() {
            window.set_margin(Edge::Top, cfg.aparencia.topo);
        }
    }
    let _ = (window, cfg);
}

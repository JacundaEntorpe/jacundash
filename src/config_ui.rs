//! Aba de configurações (Ctrl+, ou busque "config"). Cada mudança vale na hora e já
//! fica salva no config.toml.

use crate::config::{Config, pasta_config};
use crate::providers::plugin;
use crate::ui::Ui;
use gtk::prelude::*;
use gtk::{gio, glib};
use std::rc::Rc;

/// Categorias que aparecem sem ser de um provedor (os arquivos do Baloo se separam nelas).
const EXTRAS: &[&str] = &["Pastas", "Imagens", "Documentos", "Áudio", "Vídeos"];

pub fn abrir(ui: &Rc<Ui>) {
    abrir_em(ui, None);
}

/// Abre as configurações já numa página ("geral", "extras", "acoes"...).
pub fn abrir_em(ui: &Rc<Ui>, pagina_inicial: Option<&str>) {
    if let Some(w) = ui.configuracoes.borrow().as_ref() {
        if let Some(p) = pagina_inicial {
            if let Some(pilha) = w.child().and_then(|c| c.last_child()).and_then(|p| p.downcast::<gtk::Stack>().ok()) {
                pilha.set_visible_child_name(p);
            }
        }
        w.present();
        return;
    }
    let janela = gtk::Window::builder()
        .application(&ui.app)
        .title("Configurações do Jacundash")
        .default_width(760)
        .default_height(560)
        .build();
    janela.add_css_class("configuracoes");

    let pilha = gtk::Stack::new();
    pilha.set_transition_type(gtk::StackTransitionType::Crossfade);
    pilha.set_hexpand(true);
    pilha.add_titled(&pagina(geral(ui)), Some("geral"), "Geral");
    pilha.add_titled(&pagina(aparencia(ui)), Some("aparencia"), "Aparência");
    pilha.add_titled(&pagina(extras(ui)), Some("extras"), "Extras");
    pilha.add_titled(&pagina(categorias(ui)), Some("categorias"), "Categorias");
    pilha.add_titled(&pagina(acoes(ui)), Some("acoes"), "Ações");
    pilha.add_titled(&pagina(textos(ui)), Some("textos"), "Textos prontos");
    pilha.add_titled(&pagina(programacao(ui)), Some("programacao"), "Programação");
    pilha.add_titled(&pagina(ia(ui)), Some("ia"), "IA");
    pilha.add_titled(&pagina(plugins(ui)), Some("plugins"), "Plugins");

    if let Some(p) = pagina_inicial {
        pilha.set_visible_child_name(p);
    }
    let lado = gtk::StackSidebar::new();
    lado.set_stack(&pilha);
    lado.set_width_request(170);

    let tudo = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tudo.append(&lado);
    tudo.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    tudo.append(&pilha);
    janela.set_child(Some(&tudo));

    let u = ui.clone();
    janela.connect_close_request(move |_| {
        u.configuracoes.borrow_mut().take();
        glib::Propagation::Proceed
    });
    *ui.configuracoes.borrow_mut() = Some(janela.clone());
    janela.present();
}

/// Muda a config, salva e aplica.
fn mudar(ui: &Rc<Ui>, f: impl FnOnce(&mut Config)) {
    f(&mut ui.cfg.write().unwrap());
    let cfg = ui.cfg.read().unwrap().clone();
    cfg.salvar();
    ui.aplicar_aparencia();
}

fn pagina(conteudo: gtk::Box) -> gtk::ScrolledWindow {
    conteudo.set_margin_top(18);
    conteudo.set_margin_bottom(18);
    conteudo.set_margin_start(24);
    conteudo.set_margin_end(24);
    gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).child(&conteudo).build()
}

fn coluna() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, 14)
}

fn titulo(texto: &str) -> gtk::Label {
    let l = gtk::Label::new(None);
    l.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(texto)));
    l.set_xalign(0.0);
    l
}

fn explicacao(texto: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(texto));
    l.set_xalign(0.0);
    l.set_wrap(true);
    l.add_css_class("dim-label");
    l
}

/// Uma opção: nome e explicação à esquerda, o controle à direita.
fn opcao(nome: &str, desc: &str, controle: &impl IsA<gtk::Widget>) -> gtk::Box {
    let textos = gtk::Box::new(gtk::Orientation::Vertical, 2);
    textos.set_hexpand(true);
    textos.append(&titulo(nome));
    if !desc.is_empty() {
        textos.append(&explicacao(desc));
    }
    let linha = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    linha.append(&textos);
    controle.set_valign(gtk::Align::Center);
    linha.append(controle);
    linha
}

fn numero(min: f64, max: f64, valor: i32) -> gtk::SpinButton {
    let s = gtk::SpinButton::with_range(min, max, 1.0);
    s.set_value(valor as f64);
    s
}

fn texto_entrada(valor: &str, largura: i32) -> gtk::Entry {
    let e = gtk::Entry::new();
    e.set_text(valor);
    e.set_width_chars(largura);
    e
}

fn geral(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    let cfg = ui.cfg.read().unwrap().clone();

    let foco = gtk::Switch::builder().active(cfg.busca.fechar_ao_perder_foco).build();
    let u = ui.clone();
    foco.connect_active_notify(move |s| mudar(&u, |c| c.busca.fechar_ao_perder_foco = s.is_active()));
    c.append(&opcao("Fechar ao perder o foco", "Clicar fora ou trocar de janela fecha o jacundash.", &foco));

    let web = texto_entrada(&cfg.busca.web, 34);
    let u = ui.clone();
    web.connect_changed(move |e| mudar(&u, |c| c.busca.web = e.text().into()));
    c.append(&opcao("Busca na web", "Usada no \"Pesquisar na web\". {} vira o texto digitado.", &web));

    let aba = gtk::Switch::builder().active(cfg.busca.preferir_aba).build();
    let u = ui.clone();
    aba.connect_active_notify(move |s| mudar(&u, |c| c.busca.preferir_aba = s.is_active()));
    c.append(&opcao(
        "Comandos numa aba nova",
        "Logs, SSH, >comando... abrem numa aba da janela do terminal que já está aberta, em vez de numa janela nova (Konsole, GNOME Terminal, Xfce, Ptyxis, WezTerm, kitty).",
        &aba,
    ));

    let term = texto_entrada(&cfg.busca.terminal, 16);
    term.set_placeholder_text(Some("acha sozinho"));
    let u = ui.clone();
    term.connect_changed(move |e| mudar(&u, |c| c.busca.terminal = e.text().into()));
    c.append(&opcao("Terminal", "Onde rodam os comandos de terminal (SSH, plugins...). Ex.: konsole, kitty.", &term));

    c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    c.append(&titulo("Tecla de atalho"));
    if crate::atalho::eh_kde() {
        let tecla = texto_entrada(&crate::atalho::atual().unwrap_or_default(), 14);
        tecla.set_placeholder_text(Some("Meta+Space"));
        let ligar = gtk::Button::with_label("Ligar");
        // Aparece quando a tecla é de outro atalho
        let assumir = gtk::Button::with_label("Passar pro jacundash");
        assumir.set_visible(false);
        let resultado = explicacao("");
        let (t, r, a) = (tecla.clone(), resultado.clone(), assumir.clone());
        let fazer = move |tomar: bool| {
            let res = crate::atalho::ligar(t.text().trim(), tomar);
            a.set_visible(matches!(&res, Err(e) if e.contains(crate::atalho::OCUPADA)));
            let msg = match res {
                Ok(m) => m,
                // a dica do terminal não serve aqui: o botão faz isso
                Err(m) => m.split(" Pra passar").next().unwrap_or(&m).to_string(),
            };
            r.set_text(&msg);
        };
        let f = fazer.clone();
        ligar.connect_clicked(move |_| f(false));
        let f = fazer.clone();
        assumir.connect_clicked(move |_| f(true));
        tecla.connect_activate(move |_| fazer(false));
        let linha = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        linha.append(&tecla);
        linha.append(&ligar);
        linha.append(&assumir);
        c.append(&opcao("Abrir e fechar o jacundash", "Ex.: Meta+Space, Ctrl+Alt+K, Meta+F1. Vale na hora.", &linha));
        c.append(&resultado);
    }
    c.append(&explicacao(
        "Fora do KDE, ligue o comando \"jacundash\" a uma tecla nas configurações do seu desktop. Ele abre se estiver \
         fechado e fecha se estiver aberto.\n\
         KDE: Configurações do Sistema → Teclado → Atalhos → Adicionar novo → Comando.\n\
         Hyprland: bind = SUPER, SPACE, exec, jacundash   ·   Sway: bindsym $mod+space exec jacundash\n\
         Pra já ficar pronto quando a sessão começar, coloque \"jacundash --fundo\" pra iniciar junto.",
    ));
    c
}

fn aparencia(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    let a = ui.cfg.read().unwrap().aparencia.clone();
    let campos: [(&str, &str, f64, f64, i32, fn(&mut Config, i32)); 4] = [
        ("Largura", "Em pixels.", 360.0, 1600.0, a.largura, |c, v| c.aparencia.largura = v),
        ("Altura máxima da lista", "A lista encolhe quando tem pouca coisa; isto é o máximo.", 120.0, 1200.0, a.altura_max, |c, v| {
            c.aparencia.altura_max = v
        }),
        ("Distância do topo", "0 = colado no topo da tela, como o KRunner.", 0.0, 800.0, a.topo, |c, v| c.aparencia.topo = v),
        ("Tamanho dos ícones", "Em pixels.", 12.0, 64.0, a.icone, |c, v| c.aparencia.icone = v),
    ];
    for (nome, desc, min, max, valor, aplicar) in campos {
        let n = numero(min, max, valor);
        let u = ui.clone();
        n.connect_value_changed(move |s| mudar(&u, |c| aplicar(c, s.value_as_int())));
        c.append(&opcao(nome, desc, &n));
    }

    let temas = ["escuro", "claro", "sistema"];
    let escolha = gtk::DropDown::from_strings(&["Escuro", "Claro", "Do sistema (cores do KDE ou do GNOME)"]);
    escolha.set_selected(temas.iter().position(|t| *t == a.tema).unwrap_or(0) as u32);
    let u = ui.clone();
    escolha.connect_selected_notify(move |d| {
        let t = temas.get(d.selected() as usize).unwrap_or(&"escuro").to_string();
        mudar(&u, |c| c.aparencia.tema = t);
        u.aplicar_tema();
    });
    c.append(&opcao("Tema", "As cores do jacundash.", &escolha));

    c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let botao = gtk::Button::with_label("Editar o tema");
    botao.connect_clicked(|_| {
        let arquivo = pasta_config().join("estilo.css");
        if !arquivo.exists() {
            let _ = std::fs::create_dir_all(pasta_config());
            let _ = std::fs::write(&arquivo, MODELO_TEMA);
        }
        let _ = gio::AppInfo::launch_default_for_uri(&gio::File::for_path(&arquivo).uri(), gio::AppLaunchContext::NONE);
    });
    c.append(&opcao(
        "Cores e fontes",
        "O estilo.css vale por cima do tema: só precisa do que mudar. Salvou, já vale.",
        &botao,
    ));
    c
}

const MODELO_TEMA: &str = "/* Tema do jacundash: vale por cima do padrão. Descomente e mude o que quiser.\n   Salvou, já vale (não precisa reiniciar). */\n\n\
/* @define-color jd_fundo #19191e; */\n/* @define-color jd_caixa #232328; */\n/* @define-color jd_texto #dcdce1; */\n\
/* @define-color jd_apagado #8c8c91; */\n/* @define-color jd_destaque #e92f17; */\n/* @define-color jd_selecionado #2e2e35; */\n\n\
/* .caixa { font-size: 11pt; } */\n";

/// O que dá pra ligar e desligar além do básico.
fn extras(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    let e = ui.cfg.read().unwrap().extras.clone();
    let opcoes: [(&str, &str, bool, fn(&mut Config, bool)); 4] = [
        ("Recentes na busca vazia", "Ao abrir o jacundash, mostra o que você abriu por último (apps, repositórios, pastas, arquivos...).", e.recentes, |c, v| c.extras.recentes = v),
        ("Comandos do sistema", "Bloquear, suspender, hibernar, reiniciar, desligar e sair da sessão. Reiniciar, desligar e sair pedem confirmação no KDE.", e.sistema, |c, v| c.extras.sistema = v),
        ("Abas e histórico do navegador", "Precisa da integração do navegador do KDE (a extensão Plasma Integration no navegador).", e.navegador, |c, v| c.extras.navegador = v),
        ("Prévia ao lado da lista", "Imagem, começo do texto, ou o README de uma pasta, do resultado selecionado.", e.previa, |c, v| c.extras.previa = v),
    ];
    for (nome, desc, valor, aplicar) in opcoes {
        let s = gtk::Switch::builder().active(valor).build();
        let u = ui.clone();
        s.connect_active_notify(move |s| mudar(&u, |c| aplicar(c, s.is_active())));
        c.append(&opcao(nome, desc, &s));
    }
    c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    c.append(&titulo("Sempre ligados"));
    c.append(&explicacao(
        "Conversor: 10 usd em brl, 5 km em milhas, 100 f em c, 2 gb em mb\n\
         Comando no terminal: >htop\n\
         Emoji: :fogo  (ou emoji fogo)\n\
         Área de transferência: cb  (o que você copiou antes)",
    ));
    c
}

/// As ações que o jacundash já tem, por categoria (as dos plugins entram quando aparecem).
const ACOES_CONHECIDAS: &[(&str, &[&str])] = &[
    ("Todas", &["Fixar no topo", "Esconder este resultado"]),
    ("Repositórios", &["Abrir no terminal (janela nova)", "Abrir numa aba nova do terminal", "Abrir o Claude Code aqui", "Abrir no gerenciador de arquivos", "Abrir o remote no navegador", "git pull", "Copiar o caminho"]),
    ("Pastas", &["Abrir no terminal (janela nova)", "Abrir numa aba nova do terminal", "Abrir o Claude Code aqui", "Abrir no editor", "Abrir a pasta onde ele está", "Copiar o caminho"]),
    ("Git", &["Abrir no terminal (janela nova)", "Abrir numa aba nova do terminal", "Abrir o Claude Code aqui", "git pull", "git push", "Abrir o remote no navegador"]),
    ("Docker", &["Abrir um shell dentro", "Reiniciar", "Parar", "Ver os logs", "Copiar o nome"]),
    ("Serviços", &["Ver os logs ao vivo", "Reiniciar", "Parar", "Iniciar"]),
    ("Tailscale", &["SSH", "Copiar o nome (MagicDNS)", "Abrir no navegador", "Ping"]),
    ("Aplicativos", &["Mostrar o arquivo .desktop"]),
    ("Comando", &["Rodar em segundo plano (avisa quando terminar)", "Copiar o comando"]),
    ("Histórico de comandos", &["Copiar o comando", "Rodar em segundo plano (avisa quando terminar)"]),
    ("IA", &["Continuar no Open WebUI", "Perguntar de novo"]),
    ("Hermes", &["Rodar em segundo plano (resposta numa notificação)"]),
];

/// Quais ações aparecem e quais viram botão na linha, por categoria.
fn acoes(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    c.append(&explicacao(
        "Desligue o que você não usa: some do Shift+Enter e dos botões. Marque \"botão\" nas que devem aparecer \
         como botão na linha (com o mouse em cima ou na linha selecionada). Sem nada marcado, os botões são as 3 primeiras. \
         \"Todas\" vale pra todas as categorias. Ações de plugins e dos arquivos aparecem aqui depois de aparecerem num resultado.",
    ));
    // As conhecidas e as que já apareceram, sem repetir
    let mut lista: Vec<(String, Vec<String>)> = ACOES_CONHECIDAS.iter().map(|(c, l)| (c.to_string(), l.iter().map(|s| s.to_string()).collect())).collect();
    for (cat, rotulos) in ui.acoes_vistas.borrow().iter() {
        match lista.iter_mut().find(|(c, _)| c == cat) {
            Some((_, l)) => l.extend(rotulos.iter().filter(|r| !l.contains(r)).cloned().collect::<Vec<_>>()),
            None => lista.push((cat.clone(), rotulos.clone())),
        }
    }
    let cfg = ui.cfg.read().unwrap().acoes.clone();
    for (cat, rotulos) in lista {
        if rotulos.is_empty() {
            continue;
        }
        c.append(&titulo(&cat));
        let caixa = gtk::ListBox::new();
        caixa.set_selection_mode(gtk::SelectionMode::None);
        caixa.add_css_class("boxed-list");
        let marcas: Rc<std::cell::RefCell<Vec<(String, gtk::CheckButton)>>> = Rc::default();
        let padrao_botoes: Vec<&String> = rotulos.iter().filter(|r| !cfg.escondida(&cat, r)).take(3).collect();
        for r in &rotulos {
            let linha = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            linha.set_margin_top(4);
            linha.set_margin_bottom(4);
            linha.set_margin_start(10);
            linha.set_margin_end(10);
            let aparece = gtk::Switch::builder().active(!cfg.escondida(&cat, r)).valign(gtk::Align::Center).build();
            let nome = gtk::Label::new(Some(r));
            nome.set_hexpand(true);
            nome.set_xalign(0.0);
            linha.append(&aparece);
            linha.append(&nome);
            if cat != "Todas" {
                let botao = gtk::CheckButton::with_label("botão");
                botao.set_active(match cfg.botoes.get(&cat) {
                    Some(l) => l.contains(r),
                    None => padrao_botoes.contains(&r),
                });
                let (u, m, categoria) = (ui.clone(), marcas.clone(), cat.clone());
                botao.connect_toggled(move |_| {
                    let escolhidas: Vec<String> = m.borrow().iter().filter(|(_, b)| b.is_active()).map(|(n, _)| n.clone()).collect();
                    mudar(&u, |c| {
                        c.acoes.botoes.insert(categoria.clone(), escolhidas);
                    })
                });
                marcas.borrow_mut().push((r.clone(), botao.clone()));
                linha.append(&botao);
            }
            let (u, categoria, rotulo) = (ui.clone(), cat.clone(), r.clone());
            aparece.connect_active_notify(move |s| {
                mudar(&u, |c| {
                    let l = c.acoes.escondidas.entry(categoria.clone()).or_default();
                    l.retain(|x| x != &rotulo);
                    if !s.is_active() {
                        l.push(rotulo.clone());
                    }
                })
            });
            caixa.append(&linha);
        }
        c.append(&caixa);
    }
    c
}

/// Textos prontos: nome e texto, com botão pra tirar e pra pôr mais.
fn textos(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    c.append(&explicacao("Digitando o nome no jacundash (ou \"tx\" pra ver todos), o Enter copia o texto. Bom pra e-mail, endereço, CPF, respostas que você manda sempre, comandos compridos..."));
    let lista = gtk::Box::new(gtk::Orientation::Vertical, 10);
    preencher_textos(ui, &lista);
    c.append(&lista);
    let mais = gtk::Button::with_label("Novo texto");
    mais.set_halign(gtk::Align::Start);
    let (u, l) = (ui.clone(), lista.clone());
    mais.connect_clicked(move |_| {
        mudar(&u, |c| c.textos.push(Default::default()));
        preencher_textos(&u, &l);
    });
    c.append(&mais);
    c
}

fn preencher_textos(ui: &Rc<Ui>, lista: &gtk::Box) {
    while let Some(f) = lista.first_child() {
        lista.remove(&f);
    }
    let textos = ui.cfg.read().unwrap().textos.clone();
    for (i, t) in textos.iter().enumerate() {
        let cartao = gtk::Box::new(gtk::Orientation::Vertical, 6);
        cartao.add_css_class("card");
        cartao.set_margin_bottom(2);
        let topo = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let nome = texto_entrada(&t.nome, 20);
        nome.set_placeholder_text(Some("nome (ex.: email)"));
        nome.set_hexpand(true);
        let u = ui.clone();
        nome.connect_changed(move |e| mudar(&u, |c| if let Some(t) = c.textos.get_mut(i) { t.nome = e.text().into() }));
        let tirar = gtk::Button::from_icon_name("edit-delete");
        tirar.add_css_class("flat");
        tirar.set_tooltip_text(Some("Tirar este texto"));
        let (u, l) = (ui.clone(), lista.clone());
        tirar.connect_clicked(move |_| {
            mudar(&u, |c| {
                if i < c.textos.len() {
                    c.textos.remove(i);
                }
            });
            let (u, l) = (u.clone(), l.clone());
            glib::idle_add_local_once(move || preencher_textos(&u, &l));
        });
        topo.append(&nome);
        topo.append(&tirar);
        let corpo = gtk::TextView::new();
        corpo.set_wrap_mode(gtk::WrapMode::WordChar);
        corpo.buffer().set_text(&t.texto);
        corpo.set_height_request(60);
        corpo.set_top_margin(4);
        corpo.set_left_margin(4);
        let u = ui.clone();
        corpo.buffer().connect_changed(move |b| {
            let texto = b.text(&b.start_iter(), &b.end_iter(), false).to_string();
            mudar(&u, |c| if let Some(t) = c.textos.get_mut(i) { t.texto = texto })
        });
        cartao.append(&topo);
        cartao.append(&corpo);
        lista.append(&cartao);
    }
}

/// Todas as categorias conhecidas, na ordem da config (as que faltarem vão pro fim).
fn todas_categorias(ui: &Ui) -> Vec<String> {
    let mut lista = ui.cfg.read().unwrap().categorias.ordem.clone();
    let novas = ui.engine.providers.iter().map(|p| p.category().to_string()).chain(EXTRAS.iter().map(|s| s.to_string()));
    for c in novas {
        if !lista.contains(&c) {
            lista.push(c);
        }
    }
    lista
}

fn categorias(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    c.append(&explicacao(
        "Quando duas categorias batem igual com a busca, a de cima aparece primeiro. Desligue o que não quer ver \
         e ajuste quantos resultados cada uma mostra (digitando a palavra-chave, como \"app \", aparecem todos).",
    ));
    let lista = gtk::ListBox::new();
    lista.set_selection_mode(gtk::SelectionMode::None);
    lista.add_css_class("boxed-list");
    preencher_categorias(ui, &lista);
    c.append(&lista);

    c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    c.append(&titulo("Fixados e escondidos"));
    c.append(&explicacao("No jacundash, Shift+Enter num resultado tem \"Fixar no topo\" e \"Esconder este resultado\". Pra desfazer, é aqui."));
    let marcas = gtk::ListBox::new();
    marcas.set_selection_mode(gtk::SelectionMode::None);
    marcas.add_css_class("boxed-list");
    preencher_marcas(ui, &marcas);
    c.append(&marcas);
    c
}

fn preencher_marcas(ui: &Rc<Ui>, lista: &gtk::ListBox) {
    lista.remove_all();
    let m = ui.marcas.borrow().clone();
    if m.fixados.is_empty() && m.escondidos.is_empty() {
        let l = explicacao("Nada fixado nem escondido.");
        l.set_margin_top(8);
        l.set_margin_bottom(8);
        l.set_margin_start(10);
        lista.append(&l);
        return;
    }
    for (escondido, marca) in m.fixados.iter().map(|x| (false, x)).chain(m.escondidos.iter().map(|x| (true, x))) {
        let linha = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        linha.set_margin_top(4);
        linha.set_margin_bottom(4);
        linha.set_margin_start(10);
        linha.set_margin_end(10);
        let nome = gtk::Label::new(Some(&format!("{}  ·  {}", marca.titulo, marca.categoria)));
        nome.set_hexpand(true);
        nome.set_xalign(0.0);
        nome.set_ellipsize(gtk::pango::EllipsizeMode::End);
        linha.append(&gtk::Image::from_icon_name(if escondido { "view-hidden" } else { "pin" }));
        linha.append(&nome);
        let botao = gtk::Button::with_label(if escondido { "Mostrar de novo" } else { "Desafixar" });
        botao.add_css_class("flat");
        let (u, l, chave) = (ui.clone(), lista.clone(), marca.chave.clone());
        botao.connect_clicked(move |_| {
            {
                let mut m = u.marcas.borrow_mut();
                m.fixados.retain(|x| x.chave != chave);
                m.escondidos.retain(|x| x.chave != chave);
                m.salvar();
            }
            let (u, l) = (u.clone(), l.clone());
            glib::idle_add_local_once(move || preencher_marcas(&u, &l));
        });
        linha.append(&botao);
        lista.append(&linha);
    }
}

fn preencher_categorias(ui: &Rc<Ui>, lista: &gtk::ListBox) {
    lista.remove_all();
    let todas = todas_categorias(ui);
    let cfg = ui.cfg.read().unwrap().clone();
    for (i, nome) in todas.iter().enumerate() {
        let linha = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        linha.set_margin_top(6);
        linha.set_margin_bottom(6);
        linha.set_margin_start(10);
        linha.set_margin_end(10);

        let ligada = gtk::Switch::builder().active(!cfg.categorias.esconder.contains(nome)).valign(gtk::Align::Center).build();
        let (u, n) = (ui.clone(), nome.clone());
        ligada.connect_active_notify(move |s| {
            mudar(&u, |c| {
                c.categorias.esconder.retain(|x| x != &n);
                if !s.is_active() {
                    c.categorias.esconder.push(n.clone());
                }
            })
        });
        linha.append(&ligada);

        let rotulo = gtk::Label::new(Some(nome));
        rotulo.set_hexpand(true);
        rotulo.set_xalign(0.0);
        linha.append(&rotulo);

        linha.append(&explicacao("mostra"));
        let limite = numero(1.0, 50.0, cfg.limite_de(nome) as i32);
        let (u, n) = (ui.clone(), nome.clone());
        limite.connect_value_changed(move |s| mudar(&u, |c| {
            c.categorias.limites.insert(n.clone(), s.value_as_int() as usize);
        }));
        linha.append(&limite);

        for (icone, delta) in [("go-up-symbolic", -1i32), ("go-down-symbolic", 1)] {
            let b = gtk::Button::from_icon_name(icone);
            b.add_css_class("flat");
            let destino = i as i32 + delta;
            b.set_sensitive(destino >= 0 && (destino as usize) < todas.len());
            let (u, l, ordem) = (ui.clone(), lista.clone(), todas.clone());
            b.connect_clicked(move |_| {
                let mut ordem = ordem.clone();
                ordem.swap(i, destino as usize);
                mudar(&u, |c| c.categorias.ordem = ordem);
                // Redesenha depois do clique terminar (o botão é destruído no redesenho)
                let (u, l) = (u.clone(), l.clone());
                glib::idle_add_local_once(move || preencher_categorias(&u, &l));
            });
            linha.append(&b);
        }
        lista.append(&linha);
    }
}

fn lista(v: &[String]) -> String {
    v.join(", ")
}

fn de_lista(t: &str) -> Vec<String> {
    t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// Repositórios, pastas, git, serviços e Tailscale.
fn programacao(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    let cfg = ui.cfg.read().unwrap().clone();
    let r = &cfg.repos;

    c.append(&titulo("Repositórios e pastas (vs, pa, gs)"));
    let pastas = texto_entrada(&lista(&r.pastas), 34);
    let u = ui.clone();
    pastas.connect_changed(move |e| mudar(&u, |c| c.repos.pastas = de_lista(&e.text())));
    c.append(&opcao("Onde procurar", "Separadas por vírgula. Ex.: ~/Projetos, ~/Documentos", &pastas));

    let prof = numero(1.0, 10.0, r.profundidade as i32);
    let u = ui.clone();
    prof.connect_value_changed(move |s| mudar(&u, |c| c.repos.profundidade = s.value_as_int() as usize));
    c.append(&opcao("Profundidade", "Quantas pastas pra dentro ele procura.", &prof));

    let ignorar = texto_entrada(&lista(&r.ignorar), 34);
    let u = ui.clone();
    ignorar.connect_changed(move |e| mudar(&u, |c| c.repos.ignorar = de_lista(&e.text())));
    c.append(&opcao("Ignorar", "Pastas que nunca entram (as escondidas já ficam de fora).", &ignorar));

    let vscode = gtk::Switch::builder().active(r.recentes_vscode).build();
    let u = ui.clone();
    vscode.connect_active_notify(move |s| mudar(&u, |c| c.repos.recentes_vscode = s.is_active()));
    c.append(&opcao("Recentes do VS Code", "Inclui as pastas abertas no VS Code e sobe as dos últimos 30 dias.", &vscode));

    let editor = texto_entrada(&r.editor, 20);
    let u = ui.clone();
    editor.connect_changed(move |e| mudar(&u, |c| c.repos.editor = e.text().into()));
    c.append(&opcao("Editor", "O Enter num repositório abre aqui. Ex.: code, codium, kate.", &editor));

    let claude = texto_entrada(&r.claude, 20);
    let u = ui.clone();
    claude.connect_changed(move |e| mudar(&u, |c| c.repos.claude = e.text().into()));
    c.append(&opcao("Claude Code", "Comando da ação \"Abrir o Claude Code aqui\".", &claude));

    c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    c.append(&titulo("Serviços (sv)"));
    let favs = texto_entrada(&lista(&cfg.servicos.favoritos), 34);
    let u = ui.clone();
    favs.connect_changed(move |e| mudar(&u, |c| c.servicos.favoritos = de_lista(&e.text())));
    c.append(&opcao("Favoritos", "O que aparece com \"sv\" sozinho, sem o .service. Os que falharam aparecem sempre.", &favs));

    c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    c.append(&titulo("Tailscale (ts)"));
    let mut pares: Vec<String> = cfg.tailscale.usuarios.iter().map(|(a, u)| format!("{a}={u}")).collect();
    pares.sort();
    let usuarios = texto_entrada(&pares.join(", "), 34);
    let u = ui.clone();
    usuarios.connect_changed(move |e| {
        let mapa = de_lista(&e.text()).into_iter().filter_map(|p| p.split_once('=').map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))).collect();
        mudar(&u, |c| c.tailscale.usuarios = mapa)
    });
    c.append(&opcao("Usuário do SSH", "Por aparelho: vps=ubuntu, raspberry=pi", &usuarios));
    c
}

/// IA local (Ollama) e Hermes.
fn ia(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    let cfg = ui.cfg.read().unwrap().clone();
    c.append(&titulo("IA local (ia <pergunta>)"));
    c.append(&explicacao("Usa o Ollama. A resposta aparece no próprio jacundash, enquanto o modelo escreve."));

    let campos: [(&str, &str, String, i32, fn(&mut Config, String)); 4] = [
        ("Modelo", "Ex.: gemma3:12b. Vazio = o primeiro instalado.", cfg.ia.modelo.clone(), 20, |c, v| c.ia.modelo = v),
        ("Endereço do Ollama", "", cfg.ia.url.clone(), 26, |c, v| c.ia.url = v),
        ("Open WebUI", "Pra ação \"Continuar no Open WebUI\". Vazio = sem a ação. Ex.: http://localhost:8080", cfg.ia.webui.clone(), 26, |c, v| c.ia.webui = v),
        ("Manter na memória", "Quanto tempo o modelo fica carregado depois de responder: 5m, 1h, 0.", cfg.ia.manter_carregado.clone(), 8, |c, v| c.ia.manter_carregado = v),
    ];
    for (nome, desc, valor, largura, aplicar) in campos {
        let e = texto_entrada(&valor, largura);
        let u = ui.clone();
        e.connect_changed(move |e| mudar(&u, |c| aplicar(c, e.text().to_string())));
        c.append(&opcao(nome, desc, &e));
    }

    let copiar = gtk::Switch::builder().active(cfg.ia.copiar_resposta).build();
    let u = ui.clone();
    copiar.connect_active_notify(move |s| mudar(&u, |c| c.ia.copiar_resposta = s.is_active()));
    c.append(&opcao("Copiar a resposta", "Quando termina, a resposta já vai pra área de transferência.", &copiar));

    c.append(&titulo("Instruções"));
    c.append(&explicacao("Vão antes de toda pergunta."));
    let instrucoes = gtk::TextView::new();
    instrucoes.set_wrap_mode(gtk::WrapMode::WordChar);
    instrucoes.buffer().set_text(&cfg.ia.instrucoes);
    instrucoes.set_height_request(80);
    instrucoes.add_css_class("card");
    instrucoes.set_top_margin(6);
    instrucoes.set_left_margin(6);
    let u = ui.clone();
    instrucoes.buffer().connect_changed(move |b| {
        let texto = b.text(&b.start_iter(), &b.end_iter(), false).to_string();
        mudar(&u, |c| c.ia.instrucoes = texto)
    });
    c.append(&instrucoes);

    c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    c.append(&titulo("Hermes (hm <tarefa>)"));
    let hermes = texto_entrada(&cfg.hermes.comando, 16);
    let u = ui.clone();
    hermes.connect_changed(move |e| mudar(&u, |c| c.hermes.comando = e.text().into()));
    c.append(&opcao("Comando", "O agente que mexe no PC.", &hermes));
    c
}

fn plugins(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    c.append(&explicacao(
        "Plugin é qualquer programa numa pasta de ~/.config/jacundash/plugins/ com um plugin.toml. Ele recebe a \
         busca em JSON e devolve resultados. O formato está no PLUGINS.md do repositório.",
    ));
    let instalados = plugin::carregar();
    if instalados.is_empty() {
        c.append(&titulo("Nenhum plugin instalado"));
    }
    let cfg = ui.cfg.read().unwrap().clone();
    for p in &instalados {
        let m = &p.manifesto;
        let ligado = gtk::Switch::builder().active(!cfg.categorias.esconder.contains(&m.nome)).build();
        let (u, n) = (ui.clone(), m.nome.clone());
        ligado.connect_active_notify(move |s| {
            mudar(&u, |c| {
                c.categorias.esconder.retain(|x| x != &n);
                if !s.is_active() {
                    c.categorias.esconder.push(n.clone());
                }
            })
        });
        let palavra = if m.palavra.is_empty() { "sem palavra-chave".into() } else { format!("palavra-chave: {}", m.palavra) };
        c.append(&opcao(&m.nome, &format!("{palavra}  ·  {}", m.exec), &ligado));
    }

    c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let botoes = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let pasta = gtk::Button::with_label("Abrir a pasta dos plugins");
    pasta.connect_clicked(|_| {
        let p = pasta_config().join("plugins");
        let _ = std::fs::create_dir_all(&p);
        let _ = gio::AppInfo::launch_default_for_uri(&gio::File::for_path(&p).uri(), gio::AppLaunchContext::NONE);
    });
    let recarregar = gtk::Button::with_label("Recarregar o Jacundash");
    recarregar.add_css_class("suggested-action");
    let u = ui.clone();
    recarregar.connect_clicked(move |_| u.recarregar());
    botoes.append(&pasta);
    botoes.append(&recarregar);
    c.append(&botoes);
    c.append(&explicacao("Recarregar relê os plugins, os atalhos de busca da web e o tema."));
    c
}

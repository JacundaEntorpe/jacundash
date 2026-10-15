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
    if let Some(w) = ui.configuracoes.borrow().as_ref() {
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
    pilha.add_titled(&pagina(categorias(ui)), Some("categorias"), "Categorias");
    pilha.add_titled(&pagina(plugins(ui)), Some("plugins"), "Plugins");

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
    f(&mut ui.cfg.borrow_mut());
    ui.cfg.borrow().salvar();
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
    let cfg = ui.cfg.borrow().clone();

    let foco = gtk::Switch::builder().active(cfg.busca.fechar_ao_perder_foco).build();
    let u = ui.clone();
    foco.connect_active_notify(move |s| mudar(&u, |c| c.busca.fechar_ao_perder_foco = s.is_active()));
    c.append(&opcao("Fechar ao perder o foco", "Clicar fora ou trocar de janela fecha o jacundash.", &foco));

    let web = texto_entrada(&cfg.busca.web, 34);
    let u = ui.clone();
    web.connect_changed(move |e| mudar(&u, |c| c.busca.web = e.text().into()));
    c.append(&opcao("Busca na web", "Usada no \"Pesquisar na web\". {} vira o texto digitado.", &web));

    let term = texto_entrada(&cfg.busca.terminal, 16);
    term.set_placeholder_text(Some("acha sozinho"));
    let u = ui.clone();
    term.connect_changed(move |e| mudar(&u, |c| c.busca.terminal = e.text().into()));
    c.append(&opcao("Terminal", "Onde rodam os comandos de terminal (SSH, plugins...). Ex.: konsole, kitty.", &term));

    c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    c.append(&titulo("Tecla de atalho"));
    c.append(&explicacao(
        "Ligue o comando \"jacundash\" a uma tecla nas configurações do seu desktop. Ele abre se estiver \
         fechado e fecha se estiver aberto.\n\
         KDE: Configurações do Sistema → Teclado → Atalhos → Adicionar novo → Comando.\n\
         Hyprland: bind = SUPER, SPACE, exec, jacundash   ·   Sway: bindsym $mod+space exec jacundash\n\
         Pra já ficar pronto quando a sessão começar, coloque \"jacundash --fundo\" pra iniciar junto.",
    ));
    c
}

fn aparencia(ui: &Rc<Ui>) -> gtk::Box {
    let c = coluna();
    let a = ui.cfg.borrow().aparencia.clone();
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
        "O tema é um CSS (estilo.css) por cima do padrão: só precisa do que mudar. Depois de salvar, use \"Recarregar\" na aba Plugins.",
        &botao,
    ));
    c
}

const MODELO_TEMA: &str = "/* Tema do jacundash: vale por cima do padrão. Descomente e mude o que quiser.\n   Depois de salvar: busque \"recarregar\" no jacundash. */\n\n\
/* @define-color jd_fundo #19191e; */\n/* @define-color jd_caixa #232328; */\n/* @define-color jd_texto #dcdce1; */\n\
/* @define-color jd_apagado #8c8c91; */\n/* @define-color jd_destaque #e92f17; */\n/* @define-color jd_selecionado #2e2e35; */\n\n\
/* .caixa { font-size: 11pt; } */\n";

/// Todas as categorias conhecidas, na ordem da config (as que faltarem vão pro fim).
fn todas_categorias(ui: &Ui) -> Vec<String> {
    let mut lista = ui.cfg.borrow().categorias.ordem.clone();
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
    c
}

fn preencher_categorias(ui: &Rc<Ui>, lista: &gtk::ListBox) {
    lista.remove_all();
    let todas = todas_categorias(ui);
    let cfg = ui.cfg.borrow().clone();
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
    let cfg = ui.cfg.borrow().clone();
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

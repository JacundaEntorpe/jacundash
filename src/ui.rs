//! A janela: campo de busca em cima, lista por categoria embaixo. Fica criada o tempo todo
//! (escondida) pra abrir na hora.

use crate::config::{Config, pasta_config};
use crate::engine::{self, Batch, Engine, Row};
use crate::model::{Effect, Item};
use crate::providers::apps;
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
    list: gtk::ListBox,
    scroll: gtk::ScrolledWindow,
    rodape: gtk::Label,
    pub engine: Rc<Engine>,
    /// Muda com o app aberto (aba de configurações).
    pub cfg: RefCell<Config>,
    pub configuracoes: RefCell<Option<gtk::Window>>,
    tx: async_channel::Sender<Batch>,
    efeitos_tx: async_channel::Sender<Vec<Effect>>,
    uso_apps: Arc<Mutex<std::collections::HashMap<String, u32>>>,
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
}

impl Ui {
    pub fn new(app: &gtk::Application, engine: Engine, cfg: Config, uso_apps: Arc<Mutex<std::collections::HashMap<String, u32>>>) -> Rc<Ui> {
        carregar_css();
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
            .placeholder_text("Buscar apps, contas, web... (gh:texto, =2^10)")
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

        caixa.append(&entry);
        caixa.append(&scroll);
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
            list,
            scroll,
            rodape,
            engine: Rc::new(engine),
            cfg: RefCell::new(cfg),
            configuracoes: RefCell::default(),
            tx,
            efeitos_tx,
            uso_apps,
            estado: RefCell::new(Estado { resultados: vec![None; n], ..Default::default() }),
        });

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
                for ef in efeitos {
                    u.executar(&ef, None);
                }
            }
        });

        let u = ui.clone();
        ui.entry.connect_changed(move |_| u.buscar());

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
                if !w.is_active() && w.is_visible() && u.cfg.borrow().busca.fechar_ao_perder_foco {
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
        self.entry.set_text(texto);
        self.buscar();
        self.window.present();
        self.entry.grab_focus();
        self.entry.set_position(-1);
    }

    pub fn esconder(&self) {
        self.window.set_visible(false);
    }

    /// Aplica largura, altura e distância do topo depois de mudar a config.
    pub fn aplicar_aparencia(&self) {
        let cfg = self.cfg.borrow();
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
        let texto = self.entry.text().to_string();
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
        self.engine.search(&texto, geracao, &self.tx, &self.cfg.borrow());
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

    fn desenhar(&self) {
        let mut e = self.estado.borrow_mut();
        e.linhas = engine::arrange(&self.engine, &e.resultados, &self.cfg.borrow(), e.modo_palavra);
        self.list.remove_all();
        for (i, linha) in e.linhas.iter().enumerate() {
            let acoes = e.acoes.filter(|(l, _)| *l == i).map(|(_, a)| a);
            self.list.append(&self.linha(linha, acoes));
        }
        if e.selecionado >= e.linhas.len() {
            e.selecionado = 0;
        }
        let tem = !e.linhas.is_empty();
        self.scroll.set_visible(tem);
        self.rodape.set_visible(tem);
        if let Some(row) = self.list.row_at_index(e.selecionado as i32) {
            self.list.select_row(Some(&row));
        }
    }

    fn linha(&self, linha: &Row, acao_aberta: Option<usize>) -> gtk::ListBoxRow {
        let caixa = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        caixa.add_css_class("item");

        let categoria = gtk::Label::new(Some(if linha.first_of_group { &linha.category } else { "" }));
        categoria.add_css_class("categoria");
        categoria.set_xalign(1.0);
        categoria.set_width_chars(11);
        categoria.set_max_width_chars(11);
        categoria.set_ellipsize(gtk::pango::EllipsizeMode::End);
        caixa.append(&categoria);

        caixa.append(&icone(&linha.item.icon, self.cfg.borrow().aparencia.icone));

        let textos = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        textos.set_hexpand(true);
        let titulo = gtk::Label::new(Some(&linha.item.title));
        titulo.add_css_class("titulo");
        titulo.set_xalign(0.0);
        titulo.set_ellipsize(gtk::pango::EllipsizeMode::End);
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
        if linha.first_of_group {
            row.add_css_class("comeco-grupo");
        }
        row
    }

    fn tecla(self: &Rc<Self>, tecla: gdk::Key, mods: gdk::ModifierType) -> glib::Propagation {
        let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);
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
            gdk::Key::Down | gdk::Key::Tab => self.mover(1),
            gdk::Key::Up | gdk::Key::ISO_Left_Tab => self.mover(-1),
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
        e.selecionado = (e.selecionado as i32 + delta).rem_euclid(n) as usize;
        if let Some(row) = self.list.row_at_index(e.selecionado as i32) {
            self.list.select_row(Some(&row));
            mostrar_linha(&self.scroll, &self.list, &row);
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
                _ => (linha.item.effect.clone(), linha.provider),
            }
        };
        self.executar(&efeito, Some(provedor));
    }

    fn executar(self: &Rc<Self>, efeito: &Effect, provedor: Option<usize>) {
        let fecha = !matches!(efeito, Effect::Query { .. } | Effect::None);
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
            Effect::Copy { text } => WidgetExt::display(&self.window).clipboard().set_text(text),
            Effect::Exec { cmd, terminal, cwd } => rodar(cmd, *terminal, cwd.as_deref(), &self.cfg.borrow()),
            Effect::Internal { action } => match action.as_str() {
                "config" => crate::config_ui::abrir(self),
                "recarregar" => self.recarregar(),
                _ => eprintln!("jacundash: ação interna desconhecida: {action}"),
            },
            Effect::Query { text } => {
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
fn rodar(cmd: &[String], terminal: bool, cwd: Option<&str>, cfg: &Config) {
    use std::os::unix::process::CommandExt;
    let mut argv: Vec<String> = cmd.to_vec();
    if terminal {
        let t = if cfg.busca.terminal.is_empty() { achar_terminal() } else { cfg.busca.terminal.clone() };
        let mut t: Vec<String> = t.split_whitespace().map(String::from).collect();
        if !argv.is_empty() {
            t.push(if t[0].ends_with("wezterm") { "start".into() } else { "-e".into() });
        }
        t.extend(argv);
        argv = t;
    }
    let Some((prog, args)) = argv.split_first() else { return };
    let mut c = std::process::Command::new(prog);
    c.args(args).stdin(std::process::Stdio::null()).process_group(0);
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    if let Err(e) = c.spawn() {
        eprintln!("jacundash: não rodou {prog}: {e}");
    }
}

fn achar_terminal() -> String {
    if let Ok(t) = std::env::var("TERMINAL") {
        return t;
    }
    for t in ["konsole", "kitty", "foot", "alacritty", "wezterm", "gnome-terminal", "ptyxis", "xfce4-terminal", "xterm"] {
        if std::env::var_os("PATH")
            .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(t).is_file()))
        {
            return t.into();
        }
    }
    "xterm".into()
}

fn carregar_css() {
    let display = gdk::Display::default().expect("sem display");
    let padrao = gtk::CssProvider::new();
    padrao.load_from_string(CSS_PADRAO);
    gtk::style_context_add_provider_for_display(&display, &padrao, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    // ~/.config/jacundash/estilo.css por cima do padrão
    let usuario = pasta_config().join("estilo.css");
    if usuario.exists() {
        let p = gtk::CssProvider::new();
        p.load_from_path(&usuario);
        gtk::style_context_add_provider_for_display(&display, &p, gtk::STYLE_PROVIDER_PRIORITY_USER);
    }
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

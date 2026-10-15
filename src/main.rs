//! jacundash: lançador rápido pra qualquer Linux, com categorias e plugins.
//!
//! O primeiro `jacundash` fica rodando em segundo plano com a janela pronta (escondida).
//! Cada `jacundash` depois disso só avisa ele pelo D-Bus: abre se estiver fechado, fecha se
//! estiver aberto. Por isso abre na hora.

mod atalho;
mod config;
mod config_ui;
mod engine;
mod konsole;
mod krunner;
mod kwin;
mod marcas;
mod markdown;
mod matcher;
mod model;
mod previa;
mod providers;
mod sistema;
mod tema;
mod ui;

use engine::{Engine, Provider};
use gtk::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const APP_ID: &str = "io.github.JacundaEntorpe.Jacundash";

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "jacundash: lançador com categorias e plugins\n\n\
             jacundash             abre (ou fecha, se já estiver aberto). Ligue isso a uma tecla.\n\
             jacundash \"texto\"     abre já buscando o texto\n\
             jacundash --config    abre as configurações (--config acoes: já na página Ações)\n\
             jacundash --enter     aperta Enter na janela aberta (pra scripts)\n\
             jacundash --atalho Meta+Space   liga a tecla (no KDE vale na hora); sem a tecla, mostra a atual\n\
             jacundash --atalho Alt+Space --assumir   tira a tecla de quem estiver usando e liga\n\
             jacundash --fundo     só deixa rodando em segundo plano, sem abrir (pra iniciar com a sessão)\n\
             jacundash --sair      encerra o que está em segundo plano\n\n\
             Config: ~/.config/jacundash/config.toml  ·  tema: ~/.config/jacundash/estilo.css\n\
             Plugins: ~/.config/jacundash/plugins/<nome>/plugin.toml"
        );
        return glib::ExitCode::SUCCESS;
    }

    // Tecla de atalho: não precisa abrir a janela pra isso
    if args.get(1).map(String::as_str) == Some("--atalho") {
        return match args.get(2) {
            None => match atalho::atual() {
                Some(t) => {
                    println!("{t}");
                    glib::ExitCode::SUCCESS
                }
                None => {
                    eprintln!("O jacundash não tem tecla. Ligue com: jacundash --atalho Meta+Space");
                    glib::ExitCode::FAILURE
                }
            },
            Some(tecla) => match atalho::ligar(tecla, args.iter().any(|a| a == "--assumir")) {
                Ok(msg) => {
                    println!("{msg}");
                    glib::ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    // 2 = a tecla é de outro atalho (o instalador pergunta se pode assumir)
                    glib::ExitCode::from(if e.contains(atalho::OCUPADA) { 2 } else { 1 })
                }
            },
        };
    }

    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    let ui: Rc<RefCell<Option<Rc<ui::Ui>>>> = Rc::default();

    let u = ui.clone();
    app.connect_startup(move |app| {
        let cfg: config::Compartilhada = Arc::new(std::sync::RwLock::new(config::Config::carregar()));
        let web = cfg.read().unwrap().busca.web.clone();
        let apps = providers::apps::Apps::new();
        let uso = apps.uso.clone();
        // Instalou ou desinstalou app: relê a lista
        let lista = apps.lista.clone();
        gio::AppInfoMonitor::get().connect_changed(move |_| {
            *lista.write().unwrap() = providers::apps::ler_apps();
        });
        let varredura = providers::repos::Varredura::new(cfg.clone());
        let recentes = providers::recentes::Recentes::new(cfg.clone());
        let lista_recentes = recentes.lista.clone();
        let mut provedores: Vec<Arc<dyn Provider>> = vec![
            Arc::new(providers::calc::Calc),
            Arc::new(providers::conversor::Conversor::new()),
            Arc::new(providers::comando::Comando),
            Arc::new(providers::historico::Historico::new()),
            Arc::new(recentes),
            Arc::new(providers::jacundash::Jacundash),
            Arc::new(apps),
            Arc::new(providers::janelas::Janelas),
            Arc::new(providers::repos::Repos { varredura: varredura.clone(), cfg: cfg.clone() }),
            Arc::new(providers::git::Git::new(varredura.clone(), cfg.clone())),
            Arc::new(providers::docker::Docker::default()),
            Arc::new(providers::servicos::Servicos::new(cfg.clone())),
            Arc::new(providers::tailscale::Tailscale::new(cfg.clone())),
            Arc::new(providers::ia::Ia::new(cfg.clone())),
            Arc::new(providers::hermes::Hermes::new(cfg.clone())),
            Arc::new(providers::emoji::Emoji::new()),
            Arc::new(providers::area::Area),
            Arc::new(providers::processos::Processos),
            Arc::new(providers::timer::Timers::new()),
            Arc::new(providers::textos::Textos { cfg: cfg.clone() }),
            Arc::new(providers::sistema::Sistema { cfg: cfg.clone() }),
            Arc::new(providers::navegador::Navegador { cfg: cfg.clone() }),
            // As nossas pastas antes das do Baloo: na repetida, fica a que tem mais ações
            Arc::new(providers::repos::Pastas { varredura: varredura.clone(), cfg: cfg.clone() }),
            Arc::new(providers::arquivos::Arquivos),
            Arc::new(providers::web::Web::new(&web)),
        ];
        for p in providers::plugin::carregar() {
            provedores.push(Arc::new(p));
        }
        let palavras = provedores.iter().map(|p| (p.keyword().to_string(), p.category().to_string())).collect();
        provedores.push(Arc::new(providers::ajuda::Ajuda::new(palavras)));
        *u.borrow_mut() = Some(ui::Ui::new(app, Engine { providers: provedores }, cfg, uso, lista_recentes));
        // Continua rodando com a janela escondida
        std::mem::forget(app.hold());
    });

    let u = ui.clone();
    app.connect_command_line(move |app, linha| {
        let args: Vec<String> = linha.arguments().iter().map(|a| a.to_string_lossy().into_owned()).collect();
        if args.iter().any(|a| a == "--sair") {
            app.quit();
        } else if args.iter().any(|a| a == "--enter") {
            if let Some(ui) = u.borrow().as_ref() {
                ui.enter();
            }
        } else if let Some(i) = args.iter().position(|a| a == "--config") {
            if let Some(ui) = u.borrow().as_ref() {
                config_ui::abrir_em(ui, args.get(i + 1).map(String::as_str));
            }
        } else if !args.iter().any(|a| a == "--fundo") {
            if let Some(ui) = u.borrow().as_ref() {
                match args.iter().skip(1).find(|a| !a.starts_with("--")) {
                    Some(texto) => ui.mostrar_com(texto),
                    None => ui.alternar(),
                }
            }
        }
        glib::ExitCode::SUCCESS
    });

    app.run_with_args(&args)
}

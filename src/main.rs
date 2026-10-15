//! jacundash: lançador rápido pra qualquer Linux, com categorias e plugins.
//!
//! O primeiro `jacundash` fica rodando em segundo plano com a janela pronta (escondida).
//! Cada `jacundash` depois disso só avisa ele pelo D-Bus: abre se estiver fechado, fecha se
//! estiver aberto. Por isso abre na hora.

mod config;
mod config_ui;
mod engine;
mod krunner;
mod matcher;
mod model;
mod providers;
mod sistema;
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
             jacundash --config    abre as configurações\n\
             jacundash --fundo     só deixa rodando em segundo plano, sem abrir (pra iniciar com a sessão)\n\
             jacundash --sair      encerra o que está em segundo plano\n\n\
             Config: ~/.config/jacundash/config.toml  ·  tema: ~/.config/jacundash/estilo.css\n\
             Plugins: ~/.config/jacundash/plugins/<nome>/plugin.toml"
        );
        return glib::ExitCode::SUCCESS;
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
        let mut provedores: Vec<Arc<dyn Provider>> = vec![
            Arc::new(providers::calc::Calc),
            Arc::new(providers::jacundash::Jacundash),
            Arc::new(apps),
            Arc::new(providers::janelas::Janelas),
            Arc::new(providers::repos::Repos { varredura: varredura.clone(), cfg: cfg.clone() }),
            Arc::new(providers::git::Git::new(varredura.clone(), cfg.clone())),
            Arc::new(providers::docker::Docker::default()),
            Arc::new(providers::servicos::Servicos::new(cfg.clone())),
            Arc::new(providers::tailscale::Tailscale::new(cfg.clone())),
            // As nossas pastas antes das do Baloo: na repetida, fica a que tem mais ações
            Arc::new(providers::repos::Pastas { varredura: varredura.clone(), cfg: cfg.clone() }),
            Arc::new(providers::arquivos::Arquivos),
            Arc::new(providers::web::Web::new(&web)),
        ];
        for p in providers::plugin::carregar() {
            provedores.push(Arc::new(p));
        }
        *u.borrow_mut() = Some(ui::Ui::new(app, Engine { providers: provedores }, cfg, uso));
        // Continua rodando com a janela escondida
        std::mem::forget(app.hold());
    });

    let u = ui.clone();
    app.connect_command_line(move |app, linha| {
        let args: Vec<String> = linha.arguments().iter().map(|a| a.to_string_lossy().into_owned()).collect();
        if args.iter().any(|a| a == "--sair") {
            app.quit();
        } else if args.iter().any(|a| a == "--config") {
            if let Some(ui) = u.borrow().as_ref() {
                config_ui::abrir(ui);
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

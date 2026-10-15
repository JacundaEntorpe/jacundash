//! Apps instalados, pela lista do próprio GIO (os mesmos do menu do desktop, já traduzidos).
//! Os mais abertos sobem.

use crate::config::pasta_estado;
use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Action, Effect, Item};
use gtk::gio;
use gtk::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone)]
pub struct App {
    pub id: String,
    pub nome: String,
    /// Nome genérico, descrição, palavras-chave e o executável: valem menos na busca.
    pub extra: String,
    pub descricao: String,
    pub icone: String,
    pub arquivo: Option<String>,
}

pub struct Apps {
    pub lista: Arc<RwLock<Vec<App>>>,
    pub uso: Arc<Mutex<HashMap<String, u32>>>,
}

/// Lê os apps. Precisa rodar na thread principal (o GIO não é thread-safe aqui).
pub fn ler_apps() -> Vec<App> {
    let mut apps = Vec::new();
    for info in gio::AppInfo::all() {
        if !info.should_show() {
            continue;
        }
        let Some(id) = info.id() else { continue };
        let desktop = info.downcast_ref::<gio_unix::DesktopAppInfo>();
        let generico = desktop.and_then(|d| d.generic_name()).map(|s| s.to_string()).unwrap_or_default();
        let palavras = desktop.map(|d| d.keywords().iter().map(|k| k.to_string()).collect::<Vec<_>>().join(" "));
        let descricao = info.description().map(|s| s.to_string()).unwrap_or_default();
        let exec = info.executable().to_string_lossy().into_owned();
        apps.push(App {
            id: id.to_string(),
            nome: info.display_name().to_string(),
            extra: format!("{generico} {descricao} {} {exec}", palavras.unwrap_or_default()),
            descricao: if generico.is_empty() { descricao } else { generico },
            icone: info.icon().and_then(|i| IconExt::to_string(&i)).map(|s| s.to_string()).unwrap_or_default(),
            arquivo: desktop.and_then(|d| d.filename()).map(|f| f.to_string_lossy().into_owned()),
        });
    }
    apps
}

fn arquivo_uso() -> std::path::PathBuf {
    pasta_estado().join("uso-apps.json")
}

impl Apps {
    pub fn new() -> Apps {
        let uso = std::fs::read_to_string(arquivo_uso())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Apps { lista: Arc::new(RwLock::new(ler_apps())), uso: Arc::new(Mutex::new(uso)) }
    }
}

/// Conta mais um uso do app (chamado quando ele é aberto pelo jacundash).
pub fn contar_uso(uso: &Mutex<HashMap<String, u32>>, id: &str) {
    let mut uso = uso.lock().unwrap();
    *uso.entry(id.to_string()).or_default() += 1;
    let _ = std::fs::create_dir_all(pasta_estado());
    if let Ok(json) = serde_json::to_string(&*uso) {
        let _ = std::fs::write(arquivo_uso(), json);
    }
}

impl Provider for Apps {
    fn category(&self) -> &str {
        "Aplicativos"
    }
    fn keyword(&self) -> &str {
        "app"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let uso = self.uso.lock().unwrap().clone();
        let lista = self.lista.read().unwrap();
        let mut itens = Vec::new();
        for app in lista.iter() {
            let usos = uso.get(&app.id).copied().unwrap_or(0);
            let mut nota = if q.norm.is_empty() {
                // Busca vazia: só os que você já abriu por aqui, os mais usados primeiro
                if usos == 0 && !q.with_keyword {
                    continue;
                }
                0.5
            } else {
                matcher::score(&q.norm, &app.nome, &app.extra)
            };
            if nota == 0.0 {
                continue;
            }
            nota += (usos as f32 * 0.01).min(0.09);
            let mut item = Item::new(&app.nome, Effect::Launch { app: app.id.clone() })
                .subtitle(&app.descricao)
                .icon(&app.icone)
                .score(nota);
            if let Some(arquivo) = &app.arquivo {
                item.actions.push(Action {
                    label: "Mostrar o arquivo .desktop".into(),
                    icon: "document-open-folder".into(),
                    effect: Effect::Open {
                        uri: format!("file://{}", std::path::Path::new(arquivo).parent().unwrap().display()),
                    },
                });
            }
            itens.push(item);
        }
        itens
    }
}

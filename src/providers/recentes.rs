//! Busca vazia mostra o que você abriu por último (opção das configurações, desligada de fábrica).

use crate::config::{Compartilhada, pasta_estado};
use crate::engine::{Provider, Query};
use crate::marcas;
use crate::model::{Effect, Item};
use std::sync::{Arc, Mutex};

const GUARDAR: usize = 30;

pub struct Recentes {
    pub cfg: Compartilhada,
    pub lista: Arc<Mutex<Vec<Item>>>,
}

fn arquivo() -> std::path::PathBuf {
    pasta_estado().join("recentes.json")
}

impl Recentes {
    pub fn new(cfg: Compartilhada) -> Recentes {
        let lista = std::fs::read_to_string(arquivo()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Recentes { cfg, lista: Arc::new(Mutex::new(lista)) }
    }
}

/// Guarda o que acabou de ser aberto (só o que abre alguma coisa: app, pasta, arquivo, comando).
pub fn registrar(lista: &Mutex<Vec<Item>>, item: &Item, categoria: &str) {
    if !matches!(item.effect, Effect::Launch { .. } | Effect::Open { .. } | Effect::Exec { .. }) {
        return;
    }
    let mut it = item.clone();
    it.category = Some(categoria.to_string());
    it.preview = item.preview.clone();
    let chave = marcas::chave(categoria, &it);
    let mut l = lista.lock().unwrap();
    l.retain(|x| marcas::chave(x.category.as_deref().unwrap_or(""), x) != chave);
    l.insert(0, it);
    l.truncate(GUARDAR);
    let _ = std::fs::create_dir_all(pasta_estado());
    if let Ok(json) = serde_json::to_string(&*l) {
        let _ = std::fs::write(arquivo(), json);
    }
}

impl Provider for Recentes {
    fn category(&self) -> &str {
        "Recentes"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if !q.norm.is_empty() || !self.cfg.read().unwrap().extras.recentes {
            return vec![];
        }
        self.lista
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, it)| {
                let mut it = it.clone();
                // Aparecem todos juntos, na ordem em que foram abertos
                it.subtitle = match it.category.take() {
                    Some(c) if it.subtitle.is_empty() => c,
                    Some(c) => format!("{c}  ·  {}", it.subtitle),
                    None => it.subtitle,
                };
                it.score = 0.9 - i as f32 * 0.01;
                it
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::sync::RwLock;

    #[test]
    fn guarda_o_ultimo_primeiro_e_respeita_a_opcao() {
        let pasta = std::env::temp_dir().join(format!("jacundash-teste-{}", std::process::id()));
        // SAFETY: só este teste mexe nessa variável
        unsafe { std::env::set_var("XDG_STATE_HOME", &pasta) };
        let mut cfg = Config::default();
        cfg.extras.recentes = true;
        let r = Recentes { cfg: Arc::new(RwLock::new(cfg)), lista: Arc::default() };
        let app = |n: &str| Item::new(n, Effect::Launch { app: format!("{n}.desktop") });
        registrar(&r.lista, &app("kate"), "Aplicativos");
        registrar(&r.lista, &app("krita"), "Aplicativos");
        registrar(&r.lista, &app("kate"), "Aplicativos"); // de novo: sobe, sem repetir
        registrar(&r.lista, &Item::new("= 4", Effect::Copy { text: "4".into() }), "Calculadora"); // copiar não entra
        let q = Query { text: String::new(), norm: String::new(), with_keyword: false };
        let nomes: Vec<String> = r.query(&q).into_iter().map(|i| i.title).collect();
        assert_eq!(nomes, ["kate", "krita"]);
        r.cfg.write().unwrap().extras.recentes = false;
        assert!(r.query(&q).is_empty());
        let _ = std::fs::remove_dir_all(pasta);
    }
}

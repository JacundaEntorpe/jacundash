//! Abas abertas e histórico do navegador, pela integração do navegador do KDE
//! (plasma-browser-integration). Opção das configurações, ligada de fábrica.

use crate::config::Compartilhada;
use crate::engine::{Provider, Query};
use crate::krunner;
use crate::matcher;
use crate::model::{Effect, Item};

const SERVICO: &str = "org.kde.plasma.browser_integration";

pub struct Navegador {
    pub cfg: Compartilhada,
}

impl Provider for Navegador {
    fn category(&self) -> &str {
        "Abas abertas"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if q.norm.chars().count() < 3 || q.text.trim_start().starts_with([':', '>']) || !self.cfg.read().unwrap().extras.navegador || !krunner::existe(SERVICO) {
            return vec![];
        }
        let mut itens = Vec::new();
        for (caminho, categoria, prefixo, peso) in [("/TabsRunner", "Abas abertas", "aba", 0.9), ("/HistoryRunner", "Histórico do navegador", "hist", 0.6)] {
            for m in krunner::buscar(SERVICO, caminho, &q.text).unwrap_or_default() {
                // O navegador às vezes manda coisa que não bate: confere pelo título e pelo endereço
                let nota = matcher::score(&q.norm, &m.text, &m.subtext);
                if nota == 0.0 {
                    continue;
                }
                let icone = if m.icon.is_empty() { "internet-web-browser".to_string() } else { m.icon };
                itens.push(
                    Item::new(m.text, Effect::Callback { data: format!("{prefixo}:{}", m.id) })
                        .subtitle(m.subtext)
                        .icon(icone)
                        .category(categoria)
                        .score(nota * peso),
                );
            }
        }
        itens
    }
    fn callback(&self, data: &str) -> Vec<Effect> {
        match data.split_once(':') {
            Some(("aba", id)) => krunner::rodar(SERVICO, "/TabsRunner", id, ""),
            Some(("hist", id)) => krunner::rodar(SERVICO, "/HistoryRunner", id, ""),
            _ => false,
        };
        vec![]
    }
}

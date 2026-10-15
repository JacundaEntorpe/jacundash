//! Textos prontos (Configurações → Textos): o nome acha, o Enter copia. "tx" lista todos.

use crate::config::Compartilhada;
use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};

pub struct Textos {
    pub cfg: Compartilhada,
}

impl Provider for Textos {
    fn category(&self) -> &str {
        "Textos prontos"
    }
    fn keyword(&self) -> &str {
        "tx"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if !q.with_keyword && q.norm.chars().count() < 2 {
            return vec![];
        }
        self.cfg
            .read()
            .unwrap()
            .textos
            .iter()
            .filter(|t| !t.nome.trim().is_empty())
            .filter_map(|t| {
                let nota = if q.norm.is_empty() { 0.5 } else { matcher::score(&q.norm, &t.nome, &t.texto) };
                // Sem o "tx", só quando bate pelo nome
                if nota == 0.0 || (!q.with_keyword && nota < 0.8) {
                    return None;
                }
                let resumo: String = t.texto.lines().next().unwrap_or("").chars().take(120).collect();
                Some(Item::new(&t.nome, Effect::Copy { text: t.texto.clone() }).subtitle(format!("{resumo}  ·  Enter copia")).icon("insert-text").score(nota))
            })
            .collect()
    }
}

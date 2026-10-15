//! Coisas do próprio jacundash na busca: "config" abre as configurações.

use crate::engine::{Provider, Query};
use crate::model::{Effect, Item};

pub struct Jacundash;

const ITENS: &[(&str, &str, &str, &str, &[&str])] = &[
    ("config", "Configurações do Jacundash", "Categorias, aparência, plugins  ·  Ctrl+,", "preferences-system",
     &["configuracoes", "config", "jacundash", "preferencias", "ajustes", "settings", "opcoes"]),
    ("recarregar", "Recarregar o Jacundash", "Relê plugins, atalhos web e o tema", "view-refresh",
     &["recarregar", "reiniciar", "jacundash", "reload"]),
];

impl Provider for Jacundash {
    fn category(&self) -> &str {
        "Jacundash"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if q.norm.chars().count() < 3 {
            return vec![];
        }
        ITENS
            .iter()
            .filter(|(.., palavras)| palavras.iter().any(|p| p.starts_with(q.norm.as_str())))
            .map(|(acao, titulo, sub, icone, _)| {
                Item::new(*titulo, Effect::Internal { action: acao.to_string() }).subtitle(*sub).icon(*icone).score(0.85)
            })
            .collect()
    }
}

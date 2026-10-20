//! Resultados fixados no topo e escondidos (Ctrl+Enter → Fixar / Esconder). Ficam em
//! ~/.local/state/jacundash/marcas.json.

use crate::config::pasta_estado;
use crate::model::Item;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Marca {
    pub chave: String,
    /// Pra mostrar na aba de configurações.
    pub titulo: String,
    pub categoria: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Marcas {
    pub fixados: Vec<Marca>,
    pub escondidos: Vec<Marca>,
}

/// O mesmo resultado tem a mesma chave mesmo se o texto mudar (a branch de um repositório,
/// por exemplo): o que conta é a categoria e o que ele faz.
pub fn chave(categoria: &str, item: &Item) -> String {
    format!("{categoria}\u{1f}{}", serde_json::to_string(&item.effect).unwrap_or_default())
}

fn arquivo() -> std::path::PathBuf {
    pasta_estado().join("marcas.json")
}

impl Marcas {
    pub fn carregar() -> Marcas {
        std::fs::read_to_string(arquivo()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn salvar(&self) {
        let _ = std::fs::create_dir_all(pasta_estado());
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(arquivo(), json);
        }
    }

    pub fn fixado(&self, chave: &str) -> bool {
        self.fixados.iter().any(|m| m.chave == chave)
    }

    pub fn escondido(&self, chave: &str) -> bool {
        self.escondidos.iter().any(|m| m.chave == chave)
    }

    pub fn alternar_fixado(&mut self, marca: Marca) {
        if self.fixado(&marca.chave) {
            self.fixados.retain(|m| m.chave != marca.chave);
        } else {
            self.fixados.push(marca);
        }
        self.salvar();
    }

    pub fn esconder(&mut self, marca: Marca) {
        self.fixados.retain(|m| m.chave != marca.chave);
        if !self.escondido(&marca.chave) {
            self.escondidos.push(marca);
        }
        self.salvar();
    }
}

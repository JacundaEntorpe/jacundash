//! Atalhos de busca ("gh:react", "yt lofi", "protondb elden ring") e o "Pesquisar na web" do
//! texto solto. Lê os atalhos do KDE (se tiver) e os da pasta de config do jacundash.

use crate::config::{home, pasta_config};
use crate::engine::{Provider, Query};
use crate::model::{Effect, Item};
use std::path::PathBuf;

pub struct Atalho {
    pub chaves: Vec<String>,
    pub nome: String,
    /// URL com {} no lugar do texto.
    pub url: String,
}

pub struct Web {
    pub atalhos: Vec<Atalho>,
    pub padrao: String,
}

/// Os .desktop de atalho de busca do KDE (Keys=, Name=, Query= com \{@}).
fn ler_pasta(pasta: PathBuf, atalhos: &mut Vec<Atalho>) {
    let Ok(entradas) = std::fs::read_dir(pasta) else { return };
    for e in entradas.flatten() {
        let Ok(texto) = std::fs::read_to_string(e.path()) else { continue };
        let campo = |nome: &str| {
            texto.lines().find_map(|l| l.strip_prefix(nome).and_then(|r| r.strip_prefix('='))).map(str::to_string)
        };
        let (Some(chaves), Some(url)) = (campo("Keys"), campo("Query")) else { continue };
        let chaves: Vec<String> = chaves.split(',').map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect();
        if atalhos.iter().any(|a| a.chaves.iter().any(|k| chaves.contains(k))) {
            continue; // o primeiro que achou ganha (o da home vem antes do do sistema)
        }
        let url = url.replace("\\\\{@}", "{}").replace("\\{@}", "{}");
        atalhos.push(Atalho { chaves, nome: campo("Name").unwrap_or_default(), url });
    }
}

impl Web {
    pub fn new(padrao: &str) -> Web {
        let mut atalhos = Vec::new();
        ler_pasta(pasta_config().join("atalhos-web"), &mut atalhos);
        ler_pasta(home().join(".local/share/kf6/searchproviders"), &mut atalhos);
        ler_pasta(PathBuf::from("/usr/share/kf6/searchproviders"), &mut atalhos);
        Web { atalhos, padrao: padrao.to_string() }
    }

    fn atalho(&self, chave: &str) -> Option<&Atalho> {
        self.atalhos.iter().find(|a| a.chaves.iter().any(|k| k == chave))
    }
}

pub fn codificar(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl Provider for Web {
    fn category(&self) -> &str {
        "Web"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let texto = q.text.trim();
        if texto.is_empty() {
            return vec![];
        }
        // "gh:react" ou "gh react"
        let separado = texto.split_once(':').filter(|(k, _)| !k.contains(' ')).or_else(|| texto.split_once(' '));
        if let Some((chave, termo)) = separado {
            let termo = termo.trim();
            if let (Some(a), false) = (self.atalho(&chave.to_lowercase()), termo.is_empty()) {
                return vec![
                    Item::new(format!("Pesquisar \"{termo}\" no {}", a.nome), Effect::Open { uri: a.url.replace("{}", &codificar(termo)) })
                        .subtitle(format!("atalho {chave}"))
                        .icon("internet-web-browser")
                        .score(1.0),
                ];
            }
        }
        vec![
            Item::new(format!("Pesquisar \"{texto}\" na web"), Effect::Open { uri: self.padrao.replace("{}", &codificar(texto)) })
                .icon("internet-web-browser")
                .score(0.01),
        ]
    }
}

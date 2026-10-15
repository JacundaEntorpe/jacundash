//! ":fogo" ou "emoji fogo": emojis pelo nome em português. A lista (nomes oficiais do
//! Unicode, CLDR) é baixada uma vez e fica em ~/.local/share/jacundash/emoji-pt.json.

use crate::config::home;
use crate::engine::{self, Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};
use crate::sistema;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const URL: &str = "https://cdn.jsdelivr.net/npm/cldr-annotations-full@45.0.0/annotations/pt/annotations.json";

pub struct Emoji {
    /// (emoji, nomes, nomes normalizados)
    lista: Arc<Mutex<Option<Vec<(String, String, String)>>>>,
    baixando: Arc<AtomicBool>,
    falhou: Arc<AtomicBool>,
}

fn arquivo() -> std::path::PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| home().join(".local/share")).join("jacundash/emoji-pt.json")
}

fn ler() -> Option<Vec<(String, String, String)>> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(arquivo()).ok()?).ok()?;
    let mapa = v.pointer("/annotations/annotations")?.as_object()?;
    let mut lista: Vec<(String, String, String)> = mapa
        .iter()
        // Só emoji de verdade (a lista também tem símbolos tipo "©" e letras)
        .filter(|(e, _)| e.chars().any(|c| c as u32 >= 0x1F000 || ('\u{2600}'..='\u{27BF}').contains(&c)))
        .filter_map(|(e, d)| {
            let nomes: Vec<&str> = d.get("default")?.as_array()?.iter().filter_map(|x| x.as_str()).collect();
            let nome = d.pointer("/tts/0").and_then(|x| x.as_str()).unwrap_or(nomes.first()?);
            let todos = format!("{nome} · {}", nomes.iter().filter(|n| **n != nome).copied().collect::<Vec<_>>().join(" · "));
            Some((e.clone(), todos.trim_end_matches(" · ").to_string(), matcher::norm(&format!("{nome} {}", nomes.join(" ")))))
        })
        .collect();
    lista.sort();
    Some(lista)
}

impl Emoji {
    pub fn new() -> Emoji {
        Emoji { lista: Arc::new(Mutex::new(ler())), baixando: Arc::default(), falhou: Arc::default() }
    }

    /// Baixa a lista por trás; quando chega, a busca se refaz sozinha.
    fn baixar(&self) {
        if self.baixando.swap(true, Ordering::SeqCst) {
            return;
        }
        let (lista, baixando, falhou) = (self.lista.clone(), self.baixando.clone(), self.falhou.clone());
        std::thread::spawn(move || {
            let destino = arquivo();
            if let Some(pasta) = destino.parent() {
                let _ = std::fs::create_dir_all(pasta);
            }
            let ok = sistema::rodar(&["curl", "-sfL", "--max-time", "20", "-o", &destino.to_string_lossy(), URL], None, Duration::from_secs(25)).is_some_and(|o| o.status.success());
            let nova = if ok { ler() } else { None };
            falhou.store(nova.is_none(), Ordering::SeqCst);
            *lista.lock().unwrap() = nova;
            baixando.store(false, Ordering::SeqCst);
            engine::pedir_atualizacao();
        });
    }
}

impl Provider for Emoji {
    fn category(&self) -> &str {
        "Emoji"
    }
    fn keyword(&self) -> &str {
        "emoji"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        // ":fogo" sem a palavra-chave
        let termo = if q.with_keyword {
            q.norm.clone()
        } else if let Some(t) = q.text.trim().strip_prefix(':').filter(|t| !t.is_empty()) {
            matcher::norm(t)
        } else {
            return vec![];
        };
        let lista = self.lista.lock().unwrap().clone();
        let Some(lista) = lista else {
            // Deu erro da última vez: avisa, e a próxima busca tenta de novo
            if self.falhou.swap(false, Ordering::SeqCst) {
                return vec![Item::new("Não consegui baixar a lista de emojis", Effect::None).subtitle("Precisa de internet só da primeira vez; digite de novo pra tentar outra vez").icon("dialog-warning").score(1.0)];
            }
            self.baixar();
            return vec![Item::new("Baixando a lista de emojis...", Effect::None).subtitle("Só da primeira vez (nomes em português, do Unicode)").icon("emblem-downloads").score(1.0)];
        };
        let mut itens = lista
            .iter()
            .filter_map(|(e, nomes, busca)| {
                // O nome principal pesa mais: ":fogo" põe 🔥 antes de "fogos de artifício"
                let principal = nomes.split(" · ").next().unwrap_or("");
                let nota = if termo.is_empty() { 0.5 } else { matcher::score(&termo, principal, busca) };
                (nota > 0.0).then(|| Item::new(format!("{e}   {}", nomes.split(" · ").next().unwrap_or("")), Effect::Copy { text: e.clone() }).subtitle(nomes.clone()).score(nota))
            })
            .collect::<Vec<_>>();
        // Os melhores primeiro, e só eles (a lista inteira tem quase 2 mil)
        itens.sort_by(|a, b| b.score.total_cmp(&a.score));
        itens.truncate(60);
        itens
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn le_a_lista_baixada() {
        if super::arquivo().exists() {
            let l = super::ler().expect("não leu");
            assert!(l.iter().any(|(e, ..)| e == "🔥"), "{} emojis, sem o fogo", l.len());
        }
    }
}

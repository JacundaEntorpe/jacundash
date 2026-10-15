//! ">git pu": os comandos que você já rodou, pelo Atuin se tiver; senão pelo histórico do
//! bash, do zsh ou do fish. Enter roda no terminal; Shift+Enter copia.

use crate::config::home;
use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};
use crate::sistema;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

pub struct Historico {
    /// Comandos dos arquivos de histórico, o mais recente primeiro, e quando foram lidos.
    cache: Mutex<Option<(Vec<SystemTime>, Vec<String>)>>,
}

fn arquivos() -> Vec<std::path::PathBuf> {
    let h = home();
    let mut v = vec![h.join(".bash_history"), h.join(".zsh_history"), h.join(".local/share/fish/fish_history")];
    if let Some(f) = std::env::var_os("HISTFILE") {
        v.insert(0, f.into());
    }
    v
}

/// Uma linha do arquivo de histórico -> o comando (zsh: ": 1700000000:0;cmd"; fish: "- cmd: x").
fn comando_da_linha(linha: &str) -> Option<String> {
    let c = if let Some(resto) = linha.strip_prefix(": ") {
        resto.split_once(';')?.1
    } else if let Some(resto) = linha.strip_prefix("- cmd: ") {
        resto
    } else if linha.starts_with("  when:") || linha.starts_with("#") {
        return None;
    } else {
        linha
    };
    let c = c.trim();
    (!c.is_empty()).then(|| c.to_string())
}

impl Historico {
    pub fn new() -> Historico {
        Historico { cache: Mutex::default() }
    }

    /// Lê os arquivos de novo só se algum mudou.
    fn dos_arquivos(&self) -> Vec<String> {
        let datas: Vec<SystemTime> = arquivos().iter().map(|a| std::fs::metadata(a).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH)).collect();
        let mut cache = self.cache.lock().unwrap();
        if let Some((d, lista)) = cache.as_ref() {
            if *d == datas {
                return lista.clone();
            }
        }
        let mut lista: Vec<String> = Vec::new();
        for a in arquivos() {
            let Ok(bytes) = std::fs::read(&a) else { continue };
            lista.extend(String::from_utf8_lossy(&bytes).lines().filter_map(comando_da_linha));
        }
        // Mais recente primeiro, sem repetir
        lista.reverse();
        let mut vistos = std::collections::HashSet::new();
        lista.retain(|c| vistos.insert(c.clone()));
        *cache = Some((datas, lista.clone()));
        lista
    }

    fn buscar(&self, termo: &str) -> Vec<String> {
        if sistema::tem("atuin") {
            let saida = sistema::texto(&["atuin", "search", "--cmd-only", "--limit", "30", "--search-mode", "fuzzy", termo], Duration::from_secs(2));
            let mut v: Vec<String> = saida.lines().map(str::to_string).filter(|l| !l.trim().is_empty()).collect();
            v.reverse(); // o atuin manda o mais recente no fim
            if !v.is_empty() {
                return v;
            }
        }
        let t = matcher::norm(termo);
        self.dos_arquivos().into_iter().filter(|c| matcher::norm(c).contains(&t)).take(30).collect()
    }
}

impl Provider for Historico {
    fn category(&self) -> &str {
        "Histórico de comandos"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let Some(termo) = q.text.trim().strip_prefix('>').map(str::trim).filter(|t| !t.is_empty()) else {
            return vec![];
        };
        self.buscar(termo)
            .into_iter()
            .filter(|c| c != termo)
            .enumerate()
            .map(|(i, c)| {
                let rodar = Effect::Exec { cmd: vec!["sh".into(), "-c".into(), c.clone()], terminal: true, cwd: None, hold: true, tab: false };
                Item::new(c.clone(), rodar)
                    .subtitle(if i == 0 { "Enter roda no terminal  ·  Shift+Enter copia" } else { "" })
                    .icon("utilities-terminal")
                    .score(0.8 - i as f32 * 0.005)
                    .action("Copiar o comando", "edit-copy", Effect::Copy { text: c.clone() })
                    .action("Rodar em segundo plano (avisa quando terminar)", "system-run", Effect::Task { cmd: vec!["sh".into(), "-c".into(), c.clone()], cwd: None, label: c })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::comando_da_linha;

    #[test]
    fn formatos() {
        assert_eq!(comando_da_linha("git status").as_deref(), Some("git status"));
        assert_eq!(comando_da_linha(": 1700000000:0;ls -la").as_deref(), Some("ls -la"));
        assert_eq!(comando_da_linha("- cmd: cargo build").as_deref(), Some("cargo build"));
        assert_eq!(comando_da_linha("  when: 1700000000"), None);
    }
}

//! Plugins externos: qualquer programa (Python, shell, Rust...) numa pasta de
//! ~/.config/jacundash/plugins/<nome>/ com um plugin.toml. O jacundash abre o programa uma vez
//! e conversa com ele por JSON, uma mensagem por linha (veja PLUGINS.md).

use crate::config::pasta_config;
use crate::engine::{Provider, Query};
use crate::model::{Effect, Item};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, Deserialize)]
pub struct Manifesto {
    /// Categoria onde os resultados aparecem.
    pub nome: String,
    /// Comando, relativo à pasta do plugin ("./repos.py") ou no PATH.
    pub exec: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub palavra: String,
    /// true: só responde com a palavra-chave.
    #[serde(default)]
    pub so_com_palavra: bool,
    /// Sem a palavra-chave, só pergunta a partir de tantas letras.
    #[serde(default = "minimo_padrao")]
    pub minimo: usize,
    /// Quanto tempo esperar a resposta de cada busca.
    #[serde(default = "espera_padrao")]
    pub espera_ms: u64,
}

fn minimo_padrao() -> usize {
    2
}
fn espera_padrao() -> u64 {
    1500
}

type Pendentes = Arc<Mutex<HashMap<u64, mpsc::Sender<serde_json::Value>>>>;

struct Processo {
    filho: Child,
    entrada: ChildStdin,
}

pub struct Plugin {
    pub manifesto: Manifesto,
    pasta: PathBuf,
    processo: Mutex<Option<Processo>>,
    pendentes: Pendentes,
    proximo: Mutex<u64>,
}

/// Acha os plugins instalados.
pub fn carregar() -> Vec<Plugin> {
    let mut plugins = Vec::new();
    let Ok(pastas) = std::fs::read_dir(pasta_config().join("plugins")) else { return plugins };
    let mut pastas: Vec<_> = pastas.flatten().map(|e| e.path()).collect();
    pastas.sort();
    for pasta in pastas {
        let manifesto = pasta.join("plugin.toml");
        let Ok(texto) = std::fs::read_to_string(&manifesto) else { continue };
        match toml::from_str::<Manifesto>(&texto) {
            Ok(m) => plugins.push(Plugin {
                manifesto: m,
                pasta,
                processo: Mutex::new(None),
                pendentes: Arc::default(),
                proximo: Mutex::new(0),
            }),
            Err(e) => eprintln!("jacundash: {} inválido: {e}", manifesto.display()),
        }
    }
    plugins
}

impl Plugin {
    fn comando(&self) -> PathBuf {
        let exec = Path::new(&self.manifesto.exec);
        if exec.starts_with("./") || exec.starts_with("../") { self.pasta.join(exec) } else { exec.to_path_buf() }
    }

    /// Abre o processo se ainda não está rodando (ou se morreu) e devolve a entrada dele.
    fn garantir(&self) -> std::io::Result<std::sync::MutexGuard<'_, Option<Processo>>> {
        let mut guarda = self.processo.lock().unwrap();
        let vivo = guarda.as_mut().is_some_and(|p| matches!(p.filho.try_wait(), Ok(None)));
        if !vivo {
            let mut filho = Command::new(self.comando())
                .args(&self.manifesto.args)
                .current_dir(&self.pasta)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()?;
            let saida = filho.stdout.take().unwrap();
            let entrada = filho.stdin.take().unwrap();
            let pendentes = self.pendentes.clone();
            let nome = self.manifesto.nome.clone();
            // Lê as respostas e entrega pra quem perguntou (pelo id)
            std::thread::spawn(move || {
                for linha in BufReader::new(saida).lines() {
                    let Ok(linha) = linha else { break };
                    match serde_json::from_str::<serde_json::Value>(&linha) {
                        Ok(v) => {
                            let id = v.get("id").and_then(|i| i.as_u64()).unwrap_or(u64::MAX);
                            if let Some(tx) = pendentes.lock().unwrap().remove(&id) {
                                let _ = tx.send(v);
                            }
                        }
                        Err(e) => eprintln!("jacundash: plugin {nome} mandou algo que não é JSON ({e}): {linha}"),
                    }
                }
            });
            *guarda = Some(Processo { filho, entrada });
        }
        Ok(guarda)
    }

    /// Manda uma mensagem e espera a resposta com o mesmo id.
    fn perguntar(&self, mut msg: serde_json::Value) -> Option<serde_json::Value> {
        let id = {
            let mut p = self.proximo.lock().unwrap();
            *p += 1;
            *p
        };
        msg["id"] = json!(id);
        let (tx, rx) = mpsc::channel();
        self.pendentes.lock().unwrap().insert(id, tx);
        {
            let mut guarda = match self.garantir() {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("jacundash: não deu pra abrir o plugin {}: {e}", self.manifesto.nome);
                    return None;
                }
            };
            let entrada = &mut guarda.as_mut().unwrap().entrada;
            if writeln!(entrada, "{msg}").and_then(|_| entrada.flush()).is_err() {
                *guarda = None;
                return None;
            }
        }
        let resposta = rx.recv_timeout(Duration::from_millis(self.manifesto.espera_ms)).ok();
        self.pendentes.lock().unwrap().remove(&id);
        resposta
    }
}

impl Drop for Plugin {
    fn drop(&mut self) {
        if let Some(p) = self.processo.lock().unwrap().as_mut() {
            let _ = p.filho.kill();
        }
    }
}

impl Provider for Plugin {
    fn category(&self) -> &str {
        &self.manifesto.nome
    }
    fn keyword(&self) -> &str {
        &self.manifesto.palavra
    }
    fn needs_keyword(&self) -> bool {
        self.manifesto.so_com_palavra
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if !q.with_keyword && q.norm.chars().count() < self.manifesto.minimo {
            return vec![];
        }
        let Some(resp) = self.perguntar(json!({"type": "query", "text": q.text, "keyword": q.with_keyword})) else {
            return vec![];
        };
        match serde_json::from_value::<Vec<Item>>(resp.get("items").cloned().unwrap_or_default()) {
            Ok(itens) => itens,
            Err(e) => {
                eprintln!("jacundash: plugin {} mandou itens inválidos: {e}", self.manifesto.nome);
                vec![]
            }
        }
    }
    fn callback(&self, data: &str) -> Vec<Effect> {
        let Some(resp) = self.perguntar(json!({"type": "callback", "data": data})) else { return vec![] };
        serde_json::from_value(resp.get("effects").cloned().unwrap_or_default()).unwrap_or_default()
    }
}

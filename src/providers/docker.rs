//! "dk": containers Docker. Enter = logs (ou inicia, se estiver parado).

use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};
use crate::sistema;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Container {
    nome: String,
    imagem: String,
    status: String,
    rodando: bool,
    projeto: String,
}

#[derive(Default)]
pub struct Docker {
    cache: Mutex<Option<(Instant, Vec<Container>)>>,
}

impl Docker {
    fn containers(&self) -> Vec<Container> {
        let mut cache = self.cache.lock().unwrap();
        if let Some((quando, lista)) = cache.as_ref() {
            if quando.elapsed() < Duration::from_secs(3) {
                return lista.clone();
            }
        }
        let lista: Vec<Container> = if sistema::tem("docker") {
            sistema::texto(&["docker", "ps", "-a", "--format", "{{json .}}"], Duration::from_secs(3))
                .lines()
                .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                .map(|c| {
                    let s = |k: &str| c.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let projeto = s("Labels")
                        .split(',')
                        .find_map(|r| r.strip_prefix("com.docker.compose.project=").map(String::from))
                        .unwrap_or_default();
                    Container { nome: s("Names"), imagem: s("Image"), status: s("Status"), rodando: s("State") == "running", projeto }
                })
                .collect()
        } else {
            vec![]
        };
        *cache = Some((Instant::now(), lista.clone()));
        lista
    }
}

impl Provider for Docker {
    fn category(&self) -> &str {
        "Docker"
    }
    fn keyword(&self) -> &str {
        "dk"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if !q.with_keyword && q.norm.chars().count() < 2 {
            return vec![];
        }
        let mut itens = Vec::new();
        for c in self.containers() {
            let nota = matcher::score(&q.norm, &c.nome, &c.projeto);
            if nota == 0.0 || (!q.with_keyword && nota < 0.8) {
                continue;
            }
            let n = c.nome.as_str();
            let logs = Effect::terminal(&["docker", "logs", "-f", "--tail", "300", n], None);
            let iniciar = Effect::tarefa(&["docker", "start", n], None, format!("{n}: iniciar"));
            let mut sub = vec![c.status.clone(), c.imagem.clone()];
            if !c.projeto.is_empty() {
                sub.push(format!("compose: {}", c.projeto));
            }
            let mut item = Item::new(n, if c.rodando { logs.clone() } else { iniciar.clone() })
                .subtitle(sub.join("  ·  "))
                .icon(if c.rodando { "docker" } else { "media-playback-paused" })
                .score(nota - if c.rodando { 0.0 } else { 0.1 });
            if c.rodando {
                let shell = Effect::terminal(&["docker", "exec", "-it", n, "sh", "-c", "command -v bash >/dev/null && exec bash || exec sh"], None);
                item = item
                    .action("Abrir um shell dentro", "utilities-terminal", shell)
                    .action("Reiniciar", "view-refresh", Effect::tarefa(&["docker", "restart", n], None, format!("{n}: reiniciar")))
                    .action("Parar", "media-playback-stop", Effect::tarefa(&["docker", "stop", n], None, format!("{n}: parar")));
            } else {
                item = item.action("Ver os logs", "view-list-text", logs);
            }
            itens.push(item.action("Copiar o nome", "edit-copy", Effect::Copy { text: c.nome.clone() }));
        }
        itens
    }
}

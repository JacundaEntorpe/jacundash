//! "gs": repositórios com alteração, commit sem push ou atrás do remote.

use crate::config::Compartilhada;
use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};
use crate::providers::repos::{Repo, Varredura};
use crate::sistema;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
struct Status {
    branch: String,
    upstream: bool,
    frente: u32,
    atras: u32,
    alterados: u32,
    novos: u32,
}

impl Status {
    fn pendente(&self) -> bool {
        self.alterados + self.novos + self.frente + self.atras > 0
    }
}

/// Resumo do `git status` (à frente/atrás é do último fetch).
fn ler_status(caminho: &str) -> Option<Status> {
    let o = sistema::rodar(&["git", "status", "--porcelain=v2", "--branch"], Some(caminho), Duration::from_secs(5))?;
    if !o.status.success() {
        return None;
    }
    let mut st = Status::default();
    for linha in String::from_utf8_lossy(&o.stdout).lines() {
        if let Some(b) = linha.strip_prefix("# branch.head ") {
            st.branch = b.to_string();
        } else if linha.starts_with("# branch.upstream ") {
            st.upstream = true;
        } else if let Some(ab) = linha.strip_prefix("# branch.ab ") {
            let mut p = ab.split_whitespace();
            st.frente = p.next().and_then(|x| x.trim_start_matches('+').parse().ok()).unwrap_or(0);
            st.atras = p.next().and_then(|x| x.trim_start_matches('-').parse().ok()).unwrap_or(0);
        } else if linha.starts_with('?') {
            st.novos += 1;
        } else if !linha.is_empty() && !linha.starts_with('#') {
            st.alterados += 1;
        }
    }
    Some(st)
}

fn plural(n: u32, um: &str, varios: &str) -> String {
    format!("{n} {}", if n == 1 { um } else { varios })
}

pub struct Git {
    pub varredura: Arc<Varredura>,
    pub cfg: Compartilhada,
    cache: Mutex<Option<(Instant, HashMap<String, Status>)>>,
}

impl Git {
    pub fn new(varredura: Arc<Varredura>, cfg: Compartilhada) -> Git {
        Git { varredura, cfg, cache: Mutex::default() }
    }

    /// Status de todos os repositórios, em paralelo. Guarda por 15 segundos.
    fn status(&self, repos: &[Repo]) -> HashMap<String, Status> {
        let mut cache = self.cache.lock().unwrap();
        if let Some((quando, st)) = cache.as_ref() {
            if quando.elapsed() < Duration::from_secs(15) {
                return st.clone();
            }
        }
        let caminhos: Vec<&str> = repos.iter().filter(|r| r.git).map(|r| r.caminho.as_str()).collect();
        let mut st = HashMap::new();
        std::thread::scope(|s| {
            let tarefas: Vec<_> = caminhos.chunks(caminhos.len().div_ceil(8).max(1)).map(|grupo| s.spawn(move || grupo.iter().filter_map(|c| Some((c.to_string(), ler_status(c)?))).collect::<Vec<_>>())).collect();
            for t in tarefas {
                st.extend(t.join().unwrap_or_default());
            }
        });
        *cache = Some((Instant::now(), st.clone()));
        st
    }

    fn buscar_todos(&self) {
        sistema::avisar("git fetch", "Buscando atualizações de todos os repositórios...", "vcs-pull");
        let repos = self.varredura.repos();
        let com_upstream: Vec<String> = self.status(&repos).into_iter().filter(|(_, s)| s.upstream).map(|(c, _)| c).collect();
        let ok: usize = std::thread::scope(|s| {
            let tarefas: Vec<_> = com_upstream.iter().map(|c| s.spawn(move || sistema::rodar(&["git", "fetch", "--quiet"], Some(c), Duration::from_secs(60)).is_some_and(|o| o.status.success()))).collect();
            tarefas.into_iter().map(|t| t.join().unwrap_or(false) as usize).sum()
        });
        *self.cache.lock().unwrap() = None;
        let st = self.status(&repos);
        let mut atrasados: Vec<String> = repos.iter().filter(|r| st.get(&r.caminho).is_some_and(|s| s.atras > 0)).map(|r| r.nome.clone()).collect();
        atrasados.sort();
        let mut texto = format!("{ok} de {} atualizados.", com_upstream.len());
        texto += &if atrasados.is_empty() { " Nada novo pra puxar.".to_string() } else { format!(" Pra puxar: {}", atrasados.join(", ")) };
        sistema::avisar("git fetch terminou", &texto, "vcs-normal");
    }
}

impl Provider for Git {
    fn category(&self) -> &str {
        "Git"
    }
    fn keyword(&self) -> &str {
        "gs"
    }
    fn needs_keyword(&self) -> bool {
        true // sem o "gs", rodar git status em todo repositório a cada letra seria desperdício
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let editor = self.cfg.read().unwrap().repos.editor.clone();
        let claude = self.cfg.read().unwrap().repos.claude.clone();
        let terminal = self.cfg.read().unwrap().busca.terminal.clone();
        let repos = self.varredura.repos();
        let st = self.status(&repos);
        let mut itens = Vec::new();
        for r in &repos {
            let Some(s) = st.get(&r.caminho) else { continue };
            if q.norm.is_empty() && !s.pendente() {
                continue; // "gs" sozinho: só o que tem pendência
            }
            if !q.norm.is_empty() && matcher::score(&q.norm, &r.nome, "") == 0.0 {
                continue;
            }
            let mut partes = vec![if s.branch.is_empty() { "?".into() } else { s.branch.clone() }];
            if s.alterados > 0 {
                partes.push(plural(s.alterados, "alterado", "alterados"));
            }
            if s.novos > 0 {
                partes.push(plural(s.novos, "novo", "novos"));
            }
            if s.frente > 0 {
                partes.push(format!("↑{} sem push", s.frente));
            }
            if s.atras > 0 {
                partes.push(format!("↓{} pra puxar", s.atras));
            }
            if !s.pendente() {
                partes.push("em dia".into());
            }
            if !s.upstream {
                partes.push("sem upstream".into());
            }
            let icone = if s.atras > 0 {
                "vcs-update-required"
            } else if s.alterados + s.novos > 0 {
                "vcs-locally-modified"
            } else if s.frente > 0 {
                "vcs-push"
            } else {
                "vcs-normal"
            };
            let mut abrir: Vec<String> = editor.split_whitespace().map(String::from).collect();
            abrir.push(r.caminho.clone());
            let claude: Vec<&str> = claude.split_whitespace().collect();
            let mut item = Item::new(&r.nome, Effect::Exec { cmd: abrir, terminal: false, cwd: Some(r.caminho.clone()), hold: false, tab: None })
                .subtitle(partes.join("  ·  "))
                .icon(icone)
                .score(if s.pendente() { 0.9 } else { 0.5 });
            item = crate::providers::repos::acoes_de_terminal(item, &r.caminho, &terminal)
                .action("Abrir o Claude Code aqui", "dialog-messages", Effect::terminal(&claude, Some(&r.caminho)));
            if s.upstream {
                item = item
                    .action("git pull", "vcs-pull", Effect::tarefa(&["git", "pull", "--ff-only"], Some(&r.caminho), format!("{}: git pull", r.nome)))
                    .action("git push", "vcs-push", Effect::tarefa(&["git", "push"], Some(&r.caminho), format!("{}: git push", r.nome)));
            }
            if let Some(web) = &r.web {
                item = item.action("Abrir o remote no navegador", "internet-web-browser", Effect::Open { uri: web.clone() });
            }
            itens.push(item);
        }
        if q.norm.is_empty() {
            if itens.is_empty() {
                itens.push(Item::new("Tudo em dia", Effect::None).subtitle(format!("{} repositórios sem alteração nem commit pendente", st.len())).icon("vcs-normal").score(1.0));
            }
            itens.push(
                Item::new("Buscar atualizações de todos (git fetch)", Effect::Callback { data: "fetch".into() })
                    .subtitle("Atualiza o ↓ de cada repositório, sem mexer nos seus arquivos")
                    .icon("vcs-pull")
                    .score(0.1),
            );
        }
        itens
    }
    fn callback(&self, data: &str) -> Vec<Effect> {
        if data == "fetch" {
            // Demora: roda por trás e a janela já fecha
            let eu = Git { varredura: self.varredura.clone(), cfg: self.cfg.clone(), cache: Mutex::default() };
            std::thread::spawn(move || eu.buscar_todos());
        }
        vec![]
    }
}

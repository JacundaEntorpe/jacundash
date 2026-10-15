//! "sv": serviços do systemd, do sistema e do usuário. "sv" sozinho mostra os favoritos
//! e os que falharam; com texto, busca em todos. Enter = status com as últimas linhas do log.

use crate::config::Compartilhada;
use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};
use crate::sistema;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Unidade {
    nome: String,
    usuario: bool,
    ativo: String,
    sub: String,
    habilitado: String,
    desc: String,
}

pub struct Servicos {
    pub cfg: Compartilhada,
    cache: Mutex<Option<(Instant, Vec<Unidade>)>>,
}

fn base(usuario: bool) -> Vec<&'static str> {
    if usuario { vec!["systemctl", "--user"] } else { vec!["systemctl"] }
}

fn json(cmd: &[&str]) -> Vec<serde_json::Value> {
    serde_json::from_str(&sistema::texto(cmd, Duration::from_secs(3))).unwrap_or_default()
}

fn s(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

/// Todos os serviços e timers (~300 ms: por isso só com texto digitado, e com cache).
fn listar_todos() -> Vec<Unidade> {
    let mut saida: Vec<Unidade> = Vec::new();
    for usuario in [false, true] {
        let mut b = base(usuario);
        b.extend(["list-unit-files", "--type=service,timer", "-o", "json"]);
        for u in json(&b) {
            let nome = s(&u, "unit_file");
            if !nome.contains('@') {
                // templates (foo@.service) não dá pra iniciar direto
                saida.push(Unidade { nome, usuario, ativo: "inactive".into(), sub: "dead".into(), habilitado: s(&u, "state"), desc: String::new() });
            }
        }
        let mut b = base(usuario);
        b.extend(["list-units", "--type=service,timer", "--all", "-o", "json"]);
        for u in json(&b) {
            if s(&u, "load") == "not-found" {
                continue;
            }
            let nome = s(&u, "unit");
            let i = match saida.iter().position(|x| x.nome == nome && x.usuario == usuario) {
                Some(i) => i,
                None => {
                    saida.push(Unidade { nome, usuario, ativo: String::new(), sub: String::new(), habilitado: String::new(), desc: String::new() });
                    saida.len() - 1
                }
            };
            saida[i].ativo = s(&u, "active");
            saida[i].sub = s(&u, "sub");
            saida[i].desc = s(&u, "description");
        }
    }
    saida
}

/// Só os favoritos e os que falharam (~10 ms): é o que aparece com "sv" sozinho.
fn essenciais(favoritos: &[String]) -> Vec<Unidade> {
    let nomes: Vec<String> = favoritos.iter().map(|f| if f.contains('.') { f.clone() } else { format!("{f}.service") }).collect();
    let mut saida = Vec::new();
    for usuario in [false, true] {
        let mut b = base(usuario);
        b.extend(["show", "-p", "Id,LoadState,ActiveState,SubState,UnitFileState,Description"]);
        b.extend(nomes.iter().map(String::as_str));
        for bloco in sistema::texto(&b, Duration::from_secs(3)).split("\n\n") {
            let p = |k: &str| bloco.lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('='))).unwrap_or_default().to_string();
            if p("LoadState") == "loaded" {
                saida.push(Unidade { nome: p("Id"), usuario, ativo: p("ActiveState"), sub: p("SubState"), habilitado: p("UnitFileState"), desc: p("Description") });
            }
        }
        let mut b = base(usuario);
        b.extend(["list-units", "--state=failed", "-o", "json"]);
        for u in json(&b) {
            saida.push(Unidade { nome: s(&u, "unit"), usuario, ativo: s(&u, "active"), sub: s(&u, "sub"), habilitado: String::new(), desc: s(&u, "description") });
        }
    }
    saida
}

impl Servicos {
    pub fn new(cfg: Compartilhada) -> Servicos {
        Servicos { cfg, cache: Mutex::default() }
    }

    fn todos(&self) -> Vec<Unidade> {
        let mut cache = self.cache.lock().unwrap();
        if let Some((quando, lista)) = cache.as_ref() {
            if quando.elapsed() < Duration::from_secs(5) {
                return lista.clone();
            }
        }
        let lista = listar_todos();
        *cache = Some((Instant::now(), lista.clone()));
        lista
    }
}

impl Provider for Servicos {
    fn category(&self) -> &str {
        "Serviços"
    }
    fn keyword(&self) -> &str {
        "sv"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if !q.with_keyword && q.norm.chars().count() < 3 {
            return vec![];
        }
        let favoritos = self.cfg.read().unwrap().servicos.favoritos.clone();
        let favs: Vec<String> = favoritos.iter().map(|f| matcher::norm(f)).collect();
        let fonte = if q.norm.is_empty() { essenciais(&favoritos) } else { self.todos() };
        let mut itens = Vec::new();
        for u in fonte {
            let sem_tipo = u.nome.rsplit_once('.').map_or(u.nome.as_str(), |(b, _)| b).to_string();
            let fav = favs.iter().position(|f| *f == matcher::norm(&sem_tipo));
            let nota = if q.norm.is_empty() {
                if u.ativo == "failed" { 1.0 } else { 0.9 - fav.unwrap_or(0) as f32 * 0.01 }
            } else {
                let n = matcher::score(&q.norm, &sem_tipo, &u.desc);
                // Sem a palavra-chave, só quando o nome bate bem
                if n == 0.0 || (!q.with_keyword && n < 0.8) {
                    continue;
                }
                (n + if fav.is_some() { 0.05 } else { 0.0 }).min(1.0)
            };
            let ativo = u.ativo == "active";
            let estado = if u.sub.is_empty() { u.ativo.clone() } else { format!("{} ({})", u.ativo, u.sub) };
            let sub: Vec<&str> = [estado.as_str(), &u.habilitado, if u.usuario { "usuário" } else { "sistema" }, &u.desc]
                .into_iter()
                .filter(|x| !x.is_empty())
                .collect();
            let b = base(u.usuario);
            let mut status = b.clone();
            status.extend(["status", "--no-pager", "-n", "30", &u.nome]);
            let logs = Effect::terminal(&["journalctl", if u.usuario { "--user-unit" } else { "-u" }, &u.nome, "-f", "-n", "200"], None);
            let verbo = |v: &str, rotulo: &str| {
                let mut c = b.clone();
                c.extend([v, &u.nome]);
                // Serviço do sistema: o polkit pede a senha numa janela
                Effect::tarefa(&c, None, format!("{}: {rotulo}", u.nome))
            };
            let icone = if u.ativo == "failed" { "dialog-error" } else if ativo { "media-playback-start" } else { "media-playback-stopped" };
            let mut item = Item::new(&u.nome, Effect::terminal_segurando(&status))
                .subtitle(sub.join("  ·  "))
                .icon(icone)
                .score(nota)
                .action("Ver os logs ao vivo", "view-list-text", logs);
            item = if ativo {
                item.action("Reiniciar", "view-refresh", verbo("restart", "reiniciar")).action("Parar", "media-playback-stop", verbo("stop", "parar"))
            } else {
                item.action("Iniciar", "media-playback-start", verbo("start", "iniciar"))
            };
            itens.push(item);
        }
        itens
    }
}

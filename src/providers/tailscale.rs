//! "ts": aparelhos da rede Tailscale, online primeiro. Enter copia o IP.

use crate::config::Compartilhada;
use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};
use crate::sistema;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Aparelho {
    ip: String,
    nome: String,
    dns: String,
    so: String,
    online: bool,
    eu: bool,
}

pub struct Tailscale {
    pub cfg: Compartilhada,
    cache: Mutex<Option<(Instant, Vec<Aparelho>)>>,
}

impl Tailscale {
    pub fn new(cfg: Compartilhada) -> Tailscale {
        Tailscale { cfg, cache: Mutex::default() }
    }

    fn aparelhos(&self) -> Vec<Aparelho> {
        let mut cache = self.cache.lock().unwrap();
        if let Some((quando, lista)) = cache.as_ref() {
            if quando.elapsed() < Duration::from_secs(10) {
                return lista.clone();
            }
        }
        let mut lista = Vec::new();
        if sistema::tem("tailscale") {
            let texto = sistema::texto(&["tailscale", "status", "--json"], Duration::from_secs(3));
            if let Ok(d) = serde_json::from_str::<serde_json::Value>(&texto) {
                let pares = d.get("Peer").and_then(|p| p.as_object()).into_iter().flat_map(|p| p.values().map(|v| (v, false)));
                for (p, eu) in d.get("Self").map(|s| (s, true)).into_iter().chain(pares) {
                    let Some(ip) = p.pointer("/TailscaleIPs/0").and_then(|i| i.as_str()) else { continue };
                    let s = |k: &str| p.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    lista.push(Aparelho {
                        ip: ip.to_string(),
                        nome: s("HostName"),
                        dns: s("DNSName").trim_end_matches('.').to_string(),
                        so: s("OS"),
                        online: eu || p.get("Online").and_then(|o| o.as_bool()).unwrap_or(false),
                        eu,
                    });
                }
            }
        }
        *cache = Some((Instant::now(), lista.clone()));
        lista
    }
}

impl Provider for Tailscale {
    fn category(&self) -> &str {
        "Tailscale"
    }
    fn keyword(&self) -> &str {
        "ts"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if !q.with_keyword && q.norm.chars().count() < 2 {
            return vec![];
        }
        let usuarios = self.cfg.read().unwrap().tailscale.usuarios.clone();
        let mut itens = Vec::new();
        for a in self.aparelhos() {
            let nota = matcher::score(&q.norm, &a.nome, &format!("{} {} {}", a.dns, a.ip, a.so));
            if nota == 0.0 || (!q.with_keyword && nota < 0.8) {
                continue;
            }
            let estado = if a.eu { "este PC" } else if a.online { "online" } else { "offline" };
            let sub: Vec<&str> = [a.ip.as_str(), &a.so, estado, &a.dns].into_iter().filter(|x| !x.is_empty()).collect();
            let celular = matches!(a.so.as_str(), "android" | "iOS");
            let icone = if !a.online { "network-disconnect" } else if celular { "smartphone" } else { "computer" };
            let host = a.dns.split('.').next().filter(|h| !h.is_empty()).unwrap_or(&a.ip).to_string();
            let mut item = Item::new(&a.nome, Effect::Copy { text: a.ip.clone() })
                .subtitle(sub.join("  ·  "))
                .icon(icone)
                .score(nota - if a.online { 0.0 } else { 0.3 });
            if !celular {
                let alvo = match usuarios.get(&a.nome) {
                    Some(u) => format!("{u}@{host}"),
                    None => host.clone(),
                };
                item = item.action("SSH", "utilities-terminal", Effect::terminal(&["ssh", &alvo], None));
            }
            let dns = if a.dns.is_empty() { a.ip.clone() } else { a.dns.clone() };
            itens.push(
                item.action("Copiar o nome (MagicDNS)", "edit-copy", Effect::Copy { text: dns.clone() })
                    .action("Abrir no navegador", "internet-web-browser", Effect::Open { uri: format!("http://{dns}") })
                    .action("Ping", "network-wired", Effect::terminal_segurando(&["ping", "-c", "10", &a.ip])),
            );
        }
        itens
    }
}

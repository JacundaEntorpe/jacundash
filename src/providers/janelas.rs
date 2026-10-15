//! Janelas abertas. Cada desktop tem um jeito de listar e de ir pra janela:
//! KDE (KWin pelo D-Bus), Hyprland, Sway, Niri e X11 (wmctrl). O GNOME não deixa.

use crate::engine::{Provider, Query};
use crate::krunner;
use crate::matcher;
use crate::model::{Effect, Item};
use std::process::Command;

pub struct Janelas;

struct Janela {
    titulo: String,
    app: String,
    icone: String,
    ir: Effect,
}

fn env(nome: &str) -> bool {
    std::env::var_os(nome).is_some_and(|v| !v.is_empty())
}

fn json(cmd: &[&str]) -> Option<serde_json::Value> {
    let out = Command::new(cmd[0]).args(&cmd[1..]).output().ok()?;
    serde_json::from_slice(&out.stdout).ok()
}

fn exec(cmd: &[&str]) -> Effect {
    Effect::Exec { cmd: cmd.iter().map(|s| s.to_string()).collect(), terminal: false, cwd: None }
}

fn texto(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

/// "Amigos - Discord" -> "discord": o KWin não manda o ícone de alguns apps.
fn icone_do_titulo(titulo: &str) -> String {
    let parte = titulo.rsplit(" — ").next().unwrap_or(titulo);
    let parte = parte.rsplit(" - ").next().unwrap_or(parte);
    parte.trim().to_lowercase().replace(' ', "-")
}

/// O que sobra da linha depois de pular `n` palavras.
fn depois_de(linha: &str, n: usize) -> Option<&str> {
    let mut s = linha.trim_start();
    for _ in 0..n {
        let fim = s.find(char::is_whitespace)?;
        s = s[fim..].trim_start();
    }
    Some(s)
}

fn listar() -> Vec<Janela> {
    if env("HYPRLAND_INSTANCE_SIGNATURE") {
        let Some(v) = json(&["hyprctl", "clients", "-j"]) else { return vec![] };
        return v
            .as_array()
            .into_iter()
            .flatten()
            .filter(|c| c.get("mapped").and_then(|m| m.as_bool()).unwrap_or(true))
            .map(|c| {
                let classe = texto(c, "class");
                Janela {
                    titulo: texto(c, "title"),
                    icone: classe.to_lowercase(),
                    app: classe,
                    ir: exec(&["hyprctl", "dispatch", "focuswindow", &format!("address:{}", texto(c, "address"))]),
                }
            })
            .collect();
    }
    if env("NIRI_SOCKET") {
        let Some(v) = json(&["niri", "msg", "--json", "windows"]) else { return vec![] };
        return v
            .as_array()
            .into_iter()
            .flatten()
            .map(|w| {
                let app = texto(w, "app_id");
                let id = w.get("id").and_then(|i| i.as_u64()).unwrap_or(0).to_string();
                Janela { titulo: texto(w, "title"), icone: app.to_lowercase(), app, ir: exec(&["niri", "msg", "action", "focus-window", "--id", &id]) }
            })
            .collect();
    }
    if env("SWAYSOCK") {
        let Some(arvore) = json(&["swaymsg", "-t", "get_tree", "-r"]) else { return vec![] };
        let mut saida = Vec::new();
        fn andar(n: &serde_json::Value, saida: &mut Vec<Janela>) {
            if n.get("pid").is_some() && matches!(n.get("type").and_then(|t| t.as_str()), Some("con" | "floating_con")) {
                let app = n
                    .get("app_id")
                    .and_then(|a| a.as_str())
                    .or_else(|| n.pointer("/window_properties/class").and_then(|c| c.as_str()))
                    .unwrap_or_default()
                    .to_string();
                let id = n.get("id").and_then(|i| i.as_u64()).unwrap_or(0);
                saida.push(Janela { titulo: texto(n, "name"), icone: app.to_lowercase(), app, ir: exec(&["swaymsg", &format!("[con_id={id}] focus")]) });
            }
            for filhos in ["nodes", "floating_nodes"] {
                for f in n.get(filhos).and_then(|f| f.as_array()).into_iter().flatten() {
                    andar(f, saida);
                }
            }
        }
        andar(&arvore, &mut saida);
        return saida;
    }
    if krunner::existe("org.kde.KWin") {
        let Some(ms) = krunner::buscar("org.kde.KWin", "/WindowsRunner", "") else { return vec![] };
        let mut vistas = std::collections::HashSet::new();
        // "0_..." são janelas (vêm repetidas); o resto são áreas de trabalho
        return ms
            .into_iter()
            .filter(|m| m.id.starts_with("0_") && vistas.insert(m.id.clone()))
            .map(|m| Janela {
                icone: if m.icon.is_empty() { icone_do_titulo(&m.text) } else { m.icon },
                titulo: m.text,
                app: String::new(),
                ir: Effect::Callback { data: m.id },
            })
            .collect();
    }
    if env("DISPLAY") && !env("WAYLAND_DISPLAY") {
        // X11: wmctrl -lx -> "0x03a00004  0 konsole.konsole  host Título"
        let Ok(out) = Command::new("wmctrl").arg("-lx").output() else { return vec![] };
        return String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let palavras: Vec<&str> = l.split_whitespace().collect();
                let (id, classe) = (*palavras.first()?, palavras.get(2)?.rsplit('.').next()?.to_string());
                let titulo = depois_de(l, 4)?.to_string();
                Some(Janela { titulo, icone: classe.to_lowercase(), app: classe, ir: exec(&["wmctrl", "-ia", id]) })
            })
            .collect();
    }
    vec![]
}

impl Provider for Janelas {
    fn category(&self) -> &str {
        "Janelas"
    }
    fn keyword(&self) -> &str {
        "ja"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if q.norm.is_empty() && !q.with_keyword {
            return vec![];
        }
        listar()
            .into_iter()
            .filter(|j| !j.titulo.is_empty())
            .filter_map(|j| {
                let nota = matcher::score(&q.norm, &j.titulo, &j.app);
                (nota > 0.0).then(|| {
                    // Um pouco abaixo dos apps: "konsole" abre o app novo antes de ir pra janela
                    Item::new(j.titulo, j.ir).subtitle(j.app).icon(j.icone).score(nota * 0.95)
                })
            })
            .collect()
    }
    fn callback(&self, id: &str) -> Vec<Effect> {
        krunner::rodar("org.kde.KWin", "/WindowsRunner", id, "");
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titulos() {
        assert_eq!(icone_do_titulo("Amigos - Discord"), "discord");
        assert_eq!(icone_do_titulo("krocket : claude — Konsole"), "konsole");
        assert_eq!(depois_de("0x03a00004  0 konsole.Konsole  pc  meu  título", 4), Some("meu  título"));
    }
}

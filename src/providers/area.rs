//! "cb": histórico da área de transferência. No KDE pelo Klipper; no Hyprland/Sway, pelo
//! cliphist. Enter copia de novo.

use crate::engine::{Provider, Query};
use crate::krunner;
use crate::matcher;
use crate::model::{Effect, Item};
use crate::sistema;
use gtk::{gio, glib};
use std::time::Duration;

pub struct Area;

fn klipper() -> Option<Vec<String>> {
    if !krunner::existe("org.kde.klipper") {
        return None;
    }
    let bus = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()?;
    let r = bus
        .call_sync(Some("org.kde.klipper"), "/klipper", "org.kde.klipper.klipper", "getClipboardHistoryMenu", None, Some(glib::VariantTy::new("(as)").unwrap()), gio::DBusCallFlags::NONE, 2000, gio::Cancellable::NONE)
        .ok()?;
    r.child_value(0).get::<Vec<String>>()
}

/// cliphist: "id\ttexto" por linha (o texto vem numa linha só).
fn cliphist() -> Option<Vec<(String, String)>> {
    if !sistema::tem("cliphist") {
        return None;
    }
    Some(sistema::texto(&["cliphist", "list"], Duration::from_secs(2)).lines().filter_map(|l| l.split_once('\t').map(|(a, b)| (a.to_string(), b.to_string()))).collect())
}

fn resumo(texto: &str) -> (String, String) {
    let linhas = texto.lines().count().max(1);
    let primeira = texto.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").chars().take(200).collect::<String>();
    let sub = if linhas > 1 { format!("{linhas} linhas  ·  {} caracteres", texto.chars().count()) } else { format!("{} caracteres", texto.chars().count()) };
    (primeira, sub)
}

impl Provider for Area {
    fn category(&self) -> &str {
        "Área de transferência"
    }
    fn keyword(&self) -> &str {
        "cb"
    }
    fn needs_keyword(&self) -> bool {
        true
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let bate = |t: &str| q.norm.is_empty() || matcher::norm(t).contains(&q.norm);
        if let Some(lista) = klipper() {
            return lista
                .into_iter()
                .filter(|t| !t.trim().is_empty() && bate(t))
                .enumerate()
                .map(|(i, t)| {
                    let (titulo, sub) = resumo(&t);
                    Item::new(titulo, Effect::Copy { text: t }).subtitle(sub).icon("edit-paste").score(0.9 - i as f32 * 0.001)
                })
                .collect();
        }
        if let Some(lista) = cliphist() {
            return lista
                .into_iter()
                .filter(|(_, t)| bate(t))
                .enumerate()
                .map(|(i, (id, t))| {
                    let copia = Effect::Exec { cmd: vec!["sh".into(), "-c".into(), format!("cliphist decode {id} | wl-copy")], terminal: false, cwd: None, hold: false, tab: false };
                    Item::new(t.chars().take(200).collect::<String>(), copia).icon("edit-paste").score(0.9 - i as f32 * 0.001)
                })
                .collect();
        }
        vec![Item::new("Sem histórico da área de transferência", Effect::None).subtitle("Precisa do Klipper (KDE) ou do cliphist (Hyprland, Sway...)").icon("dialog-information").score(1.0)]
    }
}

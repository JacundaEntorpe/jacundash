//! Conversa com os plugins do KRunner pelo D-Bus (org.kde.krunner1). No KDE é assim que
//! o jacundash pega as janelas (KWin) e os arquivos (Baloo), do mesmo jeito que o KRunner.

use gtk::prelude::*;
use gtk::{gio, glib};

pub struct Match {
    pub id: String,
    pub text: String,
    pub icon: String,
    pub relevance: f64,
    pub subtext: String,
    pub category: String,
}

fn bus() -> Option<gio::DBusConnection> {
    gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()
}

/// true se alguém está respondendo por esse nome no D-Bus (o serviço existe e está rodando).
pub fn existe(servico: &str) -> bool {
    let Some(bus) = bus() else { return false };
    bus.call_sync(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameHasOwner",
        Some(&(servico,).to_variant()),
        Some(glib::VariantTy::new("(b)").unwrap()),
        gio::DBusCallFlags::NONE,
        500,
        gio::Cancellable::NONE,
    )
    .ok()
    .and_then(|v| v.get::<(bool,)>())
    .is_some_and(|(tem,)| tem)
}

pub fn buscar(servico: &str, caminho: &str, texto: &str) -> Option<Vec<Match>> {
    let r = bus()?
        .call_sync(
            Some(servico),
            caminho,
            "org.kde.krunner1",
            "Match",
            Some(&(texto,).to_variant()),
            Some(glib::VariantTy::new("(a(sssida{sv}))").unwrap()),
            gio::DBusCallFlags::NONE,
            2000,
            gio::Cancellable::NONE,
        )
        .ok()?;
    let mut saida = Vec::new();
    for m in r.child_value(0).iter() {
        let s = |i| m.child_value(i).str().unwrap_or_default().to_string();
        let props = glib::VariantDict::new(Some(&m.child_value(5)));
        let texto_prop = |k| props.lookup_value(k, None).and_then(|v| v.str().map(String::from)).unwrap_or_default();
        saida.push(Match {
            id: s(0),
            text: s(1),
            icon: s(2),
            relevance: m.child_value(4).get::<f64>().unwrap_or(0.0),
            subtext: texto_prop("subtext"),
            category: texto_prop("category"),
        });
    }
    Some(saida)
}

pub fn rodar(servico: &str, caminho: &str, id: &str, acao: &str) -> bool {
    bus().is_some_and(|b| {
        b.call_sync(
            Some(servico),
            caminho,
            "org.kde.krunner1",
            "Run",
            Some(&(id, acao).to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            2000,
            gio::Cancellable::NONE,
        )
        .is_ok()
    })
}

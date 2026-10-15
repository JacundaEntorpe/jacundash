//! Aba nova no Konsole que já está aberto. O "konsole --new-tab" não serve: cada janela do
//! Konsole tem um nome próprio no D-Bus (org.kde.konsole-<pid>), então um konsole iniciado de
//! fora não acha a que está aberta e abre outra janela. Aqui a aba é pedida direto pra ela.

use gtk::prelude::*;
use gtk::{gio, glib};

fn bus() -> Option<gio::DBusConnection> {
    gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()
}

fn chamar(bus: &gio::DBusConnection, servico: &str, caminho: &str, iface: &str, metodo: &str, args: Option<glib::Variant>, tipo: Option<&str>) -> Option<glib::Variant> {
    bus.call_sync(Some(servico), caminho, iface, metodo, args.as_ref(), tipo.map(|t| glib::VariantTy::new(t).unwrap()), gio::DBusCallFlags::NONE, 2000, gio::Cancellable::NONE).ok()
}

/// O Konsole aberto mais novo: (nome no D-Bus, pid).
fn konsole_aberto(bus: &gio::DBusConnection) -> Option<(String, u32)> {
    let nomes = chamar(bus, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames", None, Some("(as)"))?;
    nomes
        .child_value(0)
        .get::<Vec<String>>()?
        .into_iter()
        .filter_map(|n| Some((n.clone(), n.strip_prefix("org.kde.konsole-")?.parse::<u32>().ok()?)))
        .max_by_key(|(_, pid)| *pid)
}

/// A janela mais nova desse Konsole (/Windows/<n>).
fn ultima_janela(bus: &gio::DBusConnection, servico: &str) -> Option<String> {
    let xml = chamar(bus, servico, "/Windows", "org.freedesktop.DBus.Introspectable", "Introspect", None, Some("(s)"))?;
    let xml = xml.child_value(0).str()?.to_string();
    xml.split("<node name=\"").skip(1).filter_map(|p| p.split('"').next()?.parse::<u32>().ok()).max().map(|n| format!("/Windows/{n}"))
}

/// Traz pra frente a janela do processo `pid` (script do KWin; o Wayland não deixa um app
/// ativar a janela de outro).
fn ativar(pid: u32) {
    let Some(bus) = bus() else { return };
    let script = format!(
        "const ws = workspace.windowList ? workspace.windowList() : workspace.clientList();\n\
         for (const w of ws) {{ if (w.pid === {pid}) {{ workspace.activeWindow = w; break; }} }}\n"
    );
    let arquivo = std::env::temp_dir().join(format!("jacundash-ativar-{}.js", std::process::id()));
    if std::fs::write(&arquivo, script).is_err() {
        return;
    }
    let nome = "jacundash-ativar";
    let _ = chamar(&bus, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting", "unloadScript", Some((nome,).to_variant()), Some("(b)"));
    if let Some(id) = chamar(&bus, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting", "loadScript", Some((arquivo.to_string_lossy().as_ref(), nome).to_variant()), Some("(i)"))
        .and_then(|v| v.child_value(0).get::<i32>())
        .filter(|id| *id >= 0)
    {
        let _ = chamar(&bus, "org.kde.KWin", &format!("/Scripting/Script{id}"), "org.kde.kwin.Script", "run", None, None);
        let _ = chamar(&bus, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting", "unloadScript", Some((nome,).to_variant()), Some("(b)"));
    }
    let _ = std::fs::remove_file(arquivo);
}

/// Abre uma aba na pasta, no Konsole que já está aberto. false se não tem Konsole aberto
/// (aí quem chamou abre uma janela nova).
pub fn nova_aba(pasta: &str) -> bool {
    let Some(bus) = bus() else { return false };
    let Some((servico, pid)) = konsole_aberto(&bus) else { return false };
    let Some(janela) = ultima_janela(&bus, &servico) else { return false };
    let Some(sessao) = chamar(&bus, &servico, &janela, "org.kde.konsole.Window", "newSession", Some(("", pasta).to_variant()), Some("(i)")).and_then(|v| v.child_value(0).get::<i32>()) else {
        return false;
    };
    let _ = chamar(&bus, &servico, &janela, "org.kde.konsole.Window", "setCurrentSession", Some((sessao,).to_variant()), None);
    ativar(pid);
    true
}

#[cfg(test)]
mod tests {
    /// Abre uma aba de verdade no Konsole aberto: só roda com --ignored.
    #[test]
    #[ignore]
    fn aba_no_konsole_aberto() {
        assert!(super::nova_aba(&format!("{}/Projetos/jacundash", std::env::var("HOME").unwrap())));
    }
}

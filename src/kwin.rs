//! Scripts do KWin: o Wayland não deixa um app mexer na janela de outro (ativar, fechar,
//! minimizar...), mas o KWin roda um script nosso que faz isso por nós.

use gtk::prelude::*;
use gtk::{gio, glib};

fn chamar(bus: &gio::DBusConnection, caminho: &str, iface: &str, metodo: &str, args: Option<glib::Variant>, tipo: Option<&str>) -> Option<glib::Variant> {
    bus.call_sync(Some("org.kde.KWin"), caminho, iface, metodo, args.as_ref(), tipo.map(|t| glib::VariantTy::new(t).unwrap()), gio::DBusCallFlags::NONE, 2000, gio::Cancellable::NONE).ok()
}

/// Roda o JavaScript no KWin. false se não tem KWin.
pub fn rodar(js: &str) -> bool {
    let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else { return false };
    let arquivo = std::env::temp_dir().join(format!("jacundash-kwin-{}.js", std::process::id()));
    if std::fs::write(&arquivo, js).is_err() {
        return false;
    }
    let nome = "jacundash";
    let _ = chamar(&bus, "/Scripting", "org.kde.kwin.Scripting", "unloadScript", Some((nome,).to_variant()), Some("(b)"));
    let id = chamar(&bus, "/Scripting", "org.kde.kwin.Scripting", "loadScript", Some((arquivo.to_string_lossy().as_ref(), nome).to_variant()), Some("(i)")).and_then(|v| v.child_value(0).get::<i32>()).filter(|id| *id >= 0);
    if let Some(id) = id {
        let _ = chamar(&bus, &format!("/Scripting/Script{id}"), "org.kde.kwin.Script", "run", None, None);
        let _ = chamar(&bus, "/Scripting", "org.kde.kwin.Scripting", "unloadScript", Some((nome,).to_variant()), Some("(b)"));
    }
    let _ = std::fs::remove_file(arquivo);
    id.is_some()
}

/// Roda `acao` (JavaScript com a janela em `w`) na janela que bate com `filtro` (ex.: "w.pid === 123").
pub fn na_janela(filtro: &str, acao: &str) -> bool {
    rodar(&format!(
        "const ws = workspace.windowList ? workspace.windowList() : workspace.clientList();\n\
         for (const w of ws) {{ if ({filtro}) {{ {acao} break; }} }}\n"
    ))
}

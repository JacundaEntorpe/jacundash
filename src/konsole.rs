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

/// Traz pra frente a janela do processo `pid`.
fn ativar(pid: u32) {
    crate::kwin::na_janela(&format!("w.pid === {pid}"), "workspace.activeWindow = w;");
}

/// O perfil do Konsole que roda o comando da aba (criado pelo instalar.sh).
fn perfil() -> std::path::PathBuf {
    crate::config::home().join(".local/share/konsole/Jacundash.profile")
}

/// Onde os comandos das próximas abas esperam: um arquivo por comando, em ordem (o aba.sh do
/// perfil pega o mais antigo). Assim várias abas abertas juntas não trocam de comando.
fn fila() -> std::path::PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir).join("jacundash-abas")
}

fn por_na_fila(comando: &str) -> Option<std::path::PathBuf> {
    let pasta = fila();
    std::fs::create_dir_all(&pasta).ok()?;
    let nome = format!("{:020}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos());
    let arquivo = pasta.join(nome);
    std::fs::write(&arquivo, comando).ok()?;
    Some(arquivo)
}

/// O Konsole só conhece os perfis que existiam quando ele abriu: se ele é mais velho que o
/// perfil do jacundash, não dá pra rodar comando numa aba dele.
fn conhece_o_perfil(pid: u32) -> bool {
    let quando = |p: &std::path::Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    match (quando(&perfil()), quando(std::path::Path::new(&format!("/proc/{pid}")))) {
        (Some(perfil), Some(konsole)) => konsole > perfil,
        _ => false,
    }
}

/// Abre uma aba na pasta, no Konsole que já está aberto, rodando `comando` (se tiver). Depois
/// do comando a aba continua no shell. false se não deu (aí quem chamou abre uma janela nova).
///
/// O Konsole não deixa outro programa digitar na aba (runCommand dá "acesso negado"), então o
/// comando vai pelo perfil "Jacundash": o jacundash deixa o comando na fila e o perfil roda.
pub fn nova_aba(pasta: &str, comando: Option<&str>) -> bool {
    let Some(bus) = bus() else { return false };
    let Some((servico, pid)) = konsole_aberto(&bus) else { return false };
    if comando.is_some() && !conhece_o_perfil(pid) {
        return false;
    }
    let Some(janela) = ultima_janela(&bus, &servico) else { return false };
    let na_fila = match comando {
        Some(c) => match por_na_fila(c) {
            Some(f) => Some(f),
            None => return false,
        },
        None => None,
    };
    let perfil = if na_fila.is_some() { "Jacundash" } else { "" };
    let Some(sessao) = chamar(&bus, &servico, &janela, "org.kde.konsole.Window", "newSession", Some((perfil, pasta).to_variant()), Some("(i)")).and_then(|v| v.child_value(0).get::<i32>()) else {
        if let Some(f) = na_fila {
            let _ = std::fs::remove_file(f);
        }
        return false;
    };
    let _ = chamar(&bus, &servico, &janela, "org.kde.konsole.Window", "setCurrentSession", Some((sessao,).to_variant()), None);
    ativar(pid);
    true
}

#[cfg(test)]
mod tests {
    /// Abre um Konsole novo, roda um comando numa aba dele e confere que rodou: só com --ignored.
    #[test]
    #[ignore]
    fn comando_em_aba_nova() {
        let novo = std::process::Command::new("konsole").spawn().expect("sem konsole");
        std::thread::sleep(std::time::Duration::from_millis(2500));
        let marca = std::env::temp_dir().join("jacundash-teste-aba");
        let _ = std::fs::remove_file(&marca);
        let ok = super::nova_aba(&std::env::var("HOME").unwrap(), Some(&format!("touch {}", marca.display())));
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let rodou = marca.exists();
        let _ = std::process::Command::new("kill").arg(novo.id().to_string()).status();
        assert!(ok && rodou, "abriu a aba: {ok}, rodou o comando: {rodou}");
    }
}

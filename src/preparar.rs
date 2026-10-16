//! Primeira vez que o jacundash roda pra esse usuário: faz na home o que o instalar.sh faz, pra
//! quem instalou por pacote (AUR...). Quem usou o instalar.sh já tem tudo, e aqui só confere.

use crate::config::{home, pasta_config};
use std::path::{Path, PathBuf};

/// Onde o pacote põe os arquivos que não são o programa (aba.sh, atalhos-web).
const SISTEMA: &str = "/usr/share/jacundash";

fn dados() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share"))
}

fn marca() -> PathBuf {
    pasta_config().join(".preparado")
}

/// Roda numa thread, no início do que fica em segundo plano.
pub fn primeira_vez() {
    if marca().exists() {
        return;
    }
    std::thread::spawn(|| {
        perfil_konsole();
        atalhos_web();
        tecla();
        let _ = std::fs::create_dir_all(pasta_config());
        let _ = std::fs::write(marca(), "");
    });
}

/// Perfil do Konsole que roda os comandos nas abas abertas pelo jacundash (ver konsole.rs).
fn perfil_konsole() {
    let perfil = dados().join("konsole/Jacundash.profile");
    if perfil.exists() || !crate::sistema::tem("konsole") {
        return;
    }
    let Some(aba) = [dados().join("jacundash/aba.sh"), Path::new(SISTEMA).join("aba.sh")].into_iter().find(|p| p.exists()) else { return };
    let _ = std::fs::create_dir_all(perfil.parent().unwrap());
    let _ = std::fs::write(&perfil, format!("[General]\nName=Jacundash\nParent=FALLBACK/\nCommand={}\n", aba.display()));
}

/// Atalhos de busca próprios (aur:, bp:, ghme:...), com o usuário do GitHub no ghme.
fn atalhos_web() {
    let destino = pasta_config().join("atalhos-web");
    let Ok(entradas) = std::fs::read_dir(Path::new(SISTEMA).join("atalhos-web")) else { return };
    // Já tem atalhos (do instalar.sh ou da pessoa): não mexe. Vazia ela pode estar, quem vigia a config cria
    if std::fs::read_dir(&destino).is_ok_and(|mut d| d.next().is_some()) {
        return;
    }
    let usuario = std::process::Command::new("git")
        .args(["config", "--global", "user.name"])
        .output()
        .ok()
        .map(|s| String::from_utf8_lossy(&s.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_default();
    let _ = std::fs::create_dir_all(&destino);
    for e in entradas.flatten() {
        if let Ok(texto) = std::fs::read_to_string(e.path()) {
            let _ = std::fs::write(destino.join(e.file_name()), texto.replace("@GITHUB_USER@", &usuario));
        }
    }
}

/// No KDE, liga a primeira tecla livre, igual ao instalar.sh (nunca tira a de outro atalho).
fn tecla() {
    if !crate::atalho::eh_kde() || crate::atalho::atual().is_some() {
        return;
    }
    for t in ["Alt+Space", "Meta+Space", "Ctrl+Space", "Meta+J", "Ctrl+Alt+Space", "Meta+F1"] {
        if let Ok(msg) = crate::atalho::ligar(t, false) {
            crate::sistema::avisar("Jacundash", &format!("{msg} Pra trocar: Ctrl+, nas configurações."), "system-search");
            return;
        }
    }
}

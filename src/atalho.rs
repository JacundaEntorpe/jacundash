//! Tecla de atalho do jacundash. No KDE liga direto no kglobalaccel (vale na hora); nos
//! outros desktops diz a linha pra colocar na config.

use gtk::prelude::*;
use gtk::{gio, glib};

/// O .desktop que o KDE usa pra abrir o jacundash quando a tecla é apertada.
pub const DESKTOP: &str = "jacundash.desktop";

const META: i32 = 0x10000000;
const CTRL: i32 = 0x04000000;
const ALT: i32 = 0x08000000;
const SHIFT: i32 = 0x02000000;
const ESPECIAIS: &[(&str, i32)] = &[("space", 0x20), ("espaco", 0x20), ("return", 0x01000004), ("enter", 0x01000004), ("tab", 0x01000001), ("escape", 0x01000000), ("print", 0x01000009)];

/// "Meta+Space" -> código do Qt que o KDE entende.
pub fn codigo(tecla: &str) -> Result<i32, String> {
    let mut c = 0;
    for parte in tecla.split('+') {
        let nome = crate::matcher::norm(parte.trim());
        c |= match nome.as_str() {
            "meta" | "super" | "win" => META,
            "ctrl" | "control" => CTRL,
            "alt" => ALT,
            "shift" => SHIFT,
            _ => {
                if let Some((_, v)) = ESPECIAIS.iter().find(|(n, _)| *n == nome) {
                    *v
                } else if let Some(n) = nome.strip_prefix('f').and_then(|n| n.parse::<i32>().ok()).filter(|n| (1..=12).contains(n)) {
                    0x01000030 + n - 1
                } else if nome.chars().count() == 1 {
                    nome.to_uppercase().chars().next().unwrap() as i32
                } else {
                    return Err(format!("Não entendi a tecla \"{parte}\". Exemplos: Meta+Space, Ctrl+Alt+K, Meta+F1"));
                }
            }
        };
    }
    Ok(c)
}

/// Código do Qt -> "Meta+Space"
pub fn nome(c: i32) -> String {
    let mut partes: Vec<String> = [(META, "Meta"), (CTRL, "Ctrl"), (ALT, "Alt"), (SHIFT, "Shift")].iter().filter(|(b, _)| c & b != 0).map(|(_, n)| n.to_string()).collect();
    let base = c & 0x01FFFFFF;
    partes.push(match base {
        0x20 => "Space".into(),
        0x01000004 => "Return".into(),
        0x01000001 => "Tab".into(),
        0x01000000 => "Escape".into(),
        0x01000009 => "Print".into(),
        0x01000092 => "Search".into(),
        b if (0x01000030..=0x0100003B).contains(&b) => format!("F{}", b - 0x01000030 + 1),
        b if (0x21..0x7f).contains(&b) => char::from_u32(b as u32).map(String::from).unwrap_or_default(),
        b => format!("tecla {b:#x}"),
    });
    partes.join("+")
}

fn bus() -> Option<gio::DBusConnection> {
    gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()
}

fn kglobalaccel(metodo: &str, args: glib::Variant, tipo: &str) -> Result<glib::Variant, String> {
    let bus = bus().ok_or("sem D-Bus")?;
    bus.call_sync(
        Some("org.kde.kglobalaccel"),
        "/kglobalaccel",
        "org.kde.KGlobalAccel",
        metodo,
        Some(&args),
        Some(glib::VariantTy::new(tipo).unwrap()),
        gio::DBusCallFlags::NONE,
        2000,
        gio::Cancellable::NONE,
    )
    .map_err(|e| e.to_string())
}

fn acao() -> Vec<String> {
    vec![DESKTOP.into(), "_launch".into(), "Jacundash".into(), "Jacundash".into()]
}

pub fn eh_kde() -> bool {
    crate::krunner::existe("org.kde.kglobalaccel")
}

/// A tecla atual do jacundash no KDE.
pub fn atual() -> Option<String> {
    let r = kglobalaccel("shortcutKeys", (acao(),).to_variant(), "(a(ai))").ok()?;
    r.child_value(0).iter().find_map(|t| t.child_value(0).get::<Vec<i32>>()?.first().copied().filter(|c| *c != 0)).map(nome)
}

/// Quem mais usa a tecla: (id da ação, nome pra mostrar).
fn donos(c: i32) -> Result<Vec<(Vec<String>, String)>, String> {
    let r = kglobalaccel("globalShortcutsByKey", ((vec![c, 0, 0, 0],), (0i32,)).to_variant(), "(a(ssssssaiai))")?;
    Ok(r.child_value(0)
        .iter()
        .filter(|d| d.child_value(2).str() != Some(DESKTOP))
        .map(|d| {
            let s = |i| d.child_value(i).str().unwrap_or_default().to_string();
            // id da ação no kglobalaccel: componente, ação, nome do componente, nome da ação
            let nome = if s(3).is_empty() { s(2) } else { s(3) };
            (vec![s(2), s(0), s(3), s(1)], nome)
        })
        .collect())
}

/// Teclas de um atalho no KDE, cada uma como o KDE guarda ([tecla, 0, 0, 0]).
fn teclas_de(id: &[String]) -> Result<Vec<Vec<i32>>, String> {
    let r = kglobalaccel("shortcutKeys", (id.to_vec(),).to_variant(), "(a(ai))")?;
    Ok(r.child_value(0).iter().filter_map(|t| t.child_value(0).get::<Vec<i32>>()).collect())
}

fn nomes_teclas(teclas: &[Vec<i32>]) -> Vec<String> {
    teclas.iter().filter_map(|v| v.first().copied().filter(|x| *x != 0)).map(nome).collect()
}

fn pra_variant(teclas: &[Vec<i32>]) -> Vec<(Vec<i32>,)> {
    teclas.iter().map(|v| (v.clone(),)).collect()
}

/// Tira a tecla de outro atalho, deixando as outras teclas dele (o KRunner continua no Alt+F2).
/// Devolve as teclas que ele tinha antes.
fn tirar_de(id: &[String], c: i32) -> Result<Vec<Vec<i32>>, String> {
    let antes = teclas_de(id)?;
    let resto: Vec<Vec<i32>> = antes.iter().filter(|v| v.first() != Some(&c)).cloned().collect();
    kglobalaccel("setForeignShortcutKeys", (id.to_vec(), pra_variant(&resto)).to_variant(), "()")?;
    Ok(antes)
}

/// O que a tecla fazia antes do jacundash assumir, pra dar pra desfazer.
#[derive(serde::Serialize, serde::Deserialize)]
struct Anterior {
    id: Vec<String>,
    nome: String,
    teclas: Vec<Vec<i32>>,
}

fn arquivo_anterior() -> std::path::PathBuf {
    crate::config::pasta_config().join("atalho-anterior.json")
}

fn ler_anteriores() -> Vec<Anterior> {
    std::fs::read_to_string(arquivo_anterior()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Guarda junto com os de antes; de quem já estava guardado fica o original (o de antes de tudo).
fn guardar_anteriores(novos: Vec<Anterior>) {
    let mut todos = ler_anteriores();
    for a in novos {
        if !todos.iter().any(|t| t.id == a.id) {
            todos.push(a);
        }
    }
    let _ = std::fs::create_dir_all(crate::config::pasta_config());
    let _ = std::fs::write(arquivo_anterior(), serde_json::to_string_pretty(&todos).unwrap_or_default());
}

/// Devolve as teclas que o jacundash tomou pros donos de antes. O jacundash fica sem tecla.
pub fn desfazer() -> Result<String, String> {
    let anteriores = ler_anteriores();
    if anteriores.is_empty() {
        return Err("O jacundash não tirou a tecla de ninguém, nada pra desfazer.".into());
    }
    // Primeiro solta a tecla, senão o KDE recusa devolver
    kglobalaccel("setShortcutKeys", (acao(), Vec::<(Vec<i32>,)>::new(), 2u32 | 4).to_variant(), "(a(ai))")?;
    let mut voltou = Vec::new();
    for a in &anteriores {
        kglobalaccel("setForeignShortcutKeys", (a.id.clone(), pra_variant(&a.teclas)).to_variant(), "()")?;
        voltou.push(format!("{} com {}", a.nome, nomes_teclas(&a.teclas).join(", ")));
    }
    let _ = std::fs::remove_file(arquivo_anterior());
    Ok(format!("Voltou: {}. O jacundash ficou sem tecla; ligue outra com: jacundash --atalho <tecla>", voltou.join("; ")))
}

/// Erro de tecla ocupada (o instalador usa pra perguntar se pode assumir).
pub const OCUPADA: &str = "já é de";

/// Liga o jacundash à tecla. No KDE vale na hora; nos outros devolve o que fazer.
/// Com `assumir`, tira a tecla de quem estiver usando (só essa tecla; as outras dele ficam).
pub fn ligar(tecla: &str, assumir: bool) -> Result<String, String> {
    let c = codigo(tecla)?;
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        return Ok(format!("No Hyprland, coloque no hyprland.conf:\n  bind = {}, exec, jacundash", hypr(c)));
    }
    if std::env::var_os("SWAYSOCK").is_some() {
        return Ok(format!("No Sway, coloque na config:\n  bindsym {} exec jacundash", sway(c)));
    }
    if !eh_kde() {
        return Ok("Ligue o comando \"jacundash\" a essa tecla nas configurações de teclado do seu desktop.".into());
    }
    // O KDE recusa calado uma tecla que já é de outro atalho (Alt+Espaço é do KRunner, por exemplo)
    let outros = donos(c)?;
    let mut avisos = Vec::new();
    let mut anteriores = Vec::new();
    if !outros.is_empty() {
        let nomes: Vec<&str> = outros.iter().map(|(_, n)| n.as_str()).collect();
        if !assumir {
            return Err(format!("{} {OCUPADA} {}. Pra passar pro jacundash: jacundash --atalho {} --assumir", nome(c), nomes.join(", "), nome(c)));
        }
        for (id, n) in &outros {
            let antes = tirar_de(id, c)?;
            let resto: Vec<String> = nomes_teclas(&antes).into_iter().filter(|t| *t != nome(c)).collect();
            let tinha = nomes_teclas(&antes).join(", ");
            avisos.push(if resto.is_empty() { format!("{n} tinha {tinha} e ficou sem tecla") } else { format!("{n} tinha {tinha}, continua com {}", resto.join(", ")) });
            anteriores.push(Anterior { id: id.clone(), nome: n.clone(), teclas: antes });
        }
    }
    kglobalaccel("doRegister", (acao(),).to_variant(), "()")?;
    // SetPresent (2): o atalho vale de verdade; NoAutoloading (4): fica a tecla passada, não a salva
    let ficou = kglobalaccel("setShortcutKeys", (acao(), vec![(vec![c, 0, 0, 0],)], 2u32 | 4).to_variant(), "(a(ai))")?;
    let aceitou = ficou.child_value(0).iter().any(|t| t.child_value(0).get::<Vec<i32>>().and_then(|v| v.first().copied()) == Some(c));
    if !aceitou {
        return Err(format!("O KDE não aceitou {}. Ligue em Configurações do Sistema → Teclado → Atalhos → Jacundash.", nome(c)));
    }
    let mut msg = format!("Jacundash ligado a {}.", nome(c));
    if !avisos.is_empty() {
        msg += &format!(" ({}). Pra desfazer: jacundash --atalho --desfazer", avisos.join("; "));
        guardar_anteriores(anteriores);
    }
    Ok(msg)
}

fn hypr(c: i32) -> String {
    let mods: Vec<&str> = [(META, "SUPER"), (CTRL, "CTRL"), (ALT, "ALT"), (SHIFT, "SHIFT")].iter().filter(|(b, _)| c & b != 0).map(|(_, n)| *n).collect();
    format!("{}, {}", mods.join(" "), nome(c & 0x01FFFFFF).to_uppercase())
}

fn sway(c: i32) -> String {
    let mut p: Vec<String> = [(META, "$mod"), (CTRL, "Ctrl"), (ALT, "Mod1"), (SHIFT, "Shift")].iter().filter(|(b, _)| c & b != 0).map(|(_, n)| n.to_string()).collect();
    p.push(nome(c & 0x01FFFFFF).to_lowercase());
    p.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teclas() {
        assert_eq!(codigo("Meta+Space"), Ok(0x10000020));
        assert_eq!(nome(codigo("ctrl+alt+k").unwrap()), "Ctrl+Alt+K");
        assert_eq!(nome(codigo("Meta+F1").unwrap()), "Meta+F1");
        assert!(codigo("Meta+Banana").is_err());
        assert_eq!(hypr(codigo("Meta+Space").unwrap()), "SUPER, SPACE");
        assert_eq!(sway(codigo("Meta+Space").unwrap()), "$mod+space");
    }
}

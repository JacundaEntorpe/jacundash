//! Temas de cor: "escuro" (o padrão), "claro" e "sistema" (as cores do KDE, ou o claro/escuro
//! e a cor de destaque do GNOME). Vira um CSS que só redefine as cores do estilo padrão.

use crate::config::home;
use crate::sistema;
use std::time::Duration;

struct Cores {
    fundo: String,
    caixa: String,
    texto: String,
    apagado: String,
    destaque: String,
    selecionado: String,
    linha: String,
}

fn css_de(c: &Cores) -> String {
    format!(
        "@define-color jd_fundo {};\n@define-color jd_caixa {};\n@define-color jd_texto {};\n@define-color jd_apagado {};\n\
         @define-color jd_destaque {};\n@define-color jd_selecionado {};\n@define-color jd_linha {};\n",
        c.fundo, c.caixa, c.texto, c.apagado, c.destaque, c.selecionado, c.linha
    )
}

fn claro() -> Cores {
    Cores {
        fundo: "#f6f6f8".into(),
        caixa: "#e9e9ee".into(),
        texto: "#1d1d22".into(),
        apagado: "#6a6a72".into(),
        destaque: "#d0281a".into(),
        selecionado: "#dcdce4".into(),
        linha: "#d6d6dd".into(),
    }
}

/// "r,g,b" do kdeglobals -> "#rrggbb"
fn rgb(v: &str) -> Option<String> {
    let p: Vec<u8> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    (p.len() >= 3).then(|| format!("#{:02x}{:02x}{:02x}", p[0], p[1], p[2]))
}

/// As cores do KDE, do ~/.config/kdeglobals.
fn kde() -> Option<Cores> {
    let texto = std::fs::read_to_string(home().join(".config/kdeglobals")).ok()?;
    let mut secao = "";
    let mut v = std::collections::HashMap::new();
    for linha in texto.lines() {
        let l = linha.trim();
        if l.starts_with('[') {
            secao = l;
        } else if let Some((k, x)) = l.split_once('=') {
            v.insert(format!("{secao}{k}"), x.to_string());
        }
    }
    let c = |s: &str, k: &str| v.get(&format!("[Colors:{s}]{k}")).and_then(|x| rgb(x));
    let destaque = v.get("[General]AccentColor").and_then(|x| rgb(x)).or_else(|| c("Selection", "BackgroundNormal"))?;
    Some(Cores {
        fundo: c("Window", "BackgroundNormal")?,
        caixa: c("View", "BackgroundNormal").or_else(|| c("Window", "BackgroundAlternate"))?,
        texto: c("Window", "ForegroundNormal")?,
        apagado: c("Window", "ForegroundInactive").unwrap_or_else(|| "#8c8c91".into()),
        selecionado: format!("alpha({destaque}, 0.25)"),
        linha: c("Window", "BackgroundAlternate").unwrap_or_else(|| "alpha(currentColor, 0.1)".into()),
        destaque,
    })
}

/// Claro/escuro e a cor de destaque do GNOME (gsettings).
fn gnome() -> Option<Cores> {
    let ler = |chave: &str| sistema::texto(&["gsettings", "get", "org.gnome.desktop.interface", chave], Duration::from_secs(1)).trim().trim_matches('\'').to_string();
    let esquema = ler("color-scheme");
    if esquema.is_empty() {
        return None;
    }
    let destaque = match ler("accent-color").as_str() {
        "teal" => "#2190a4",
        "green" => "#3a944a",
        "yellow" => "#c88800",
        "orange" => "#ed5b00",
        "red" => "#e62d42",
        "pink" => "#d56199",
        "purple" => "#9141ac",
        "slate" => "#6f8396",
        _ => "#3584e4",
    };
    let mut c = if esquema == "prefer-dark" { escuro() } else { claro() };
    c.destaque = destaque.into();
    Some(c)
}

fn escuro() -> Cores {
    Cores {
        fundo: "#19191e".into(),
        caixa: "#232328".into(),
        texto: "#dcdce1".into(),
        apagado: "#8c8c91".into(),
        destaque: "#e92f17".into(),
        selecionado: "#2e2e35".into(),
        linha: "#2a2a30".into(),
    }
}

/// O CSS das cores do tema escolhido ("escuro", "claro" ou "sistema").
pub fn css(tema: &str) -> String {
    match tema {
        "claro" => css_de(&claro()),
        "sistema" => css_de(&kde().or_else(gnome).unwrap_or_else(escuro)),
        _ => String::new(), // o escuro já é o padrão do estilo.css
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cores() {
        assert_eq!(super::rgb("233,47,23").as_deref(), Some("#e92f17"));
        assert!(super::css("claro").contains("@define-color jd_fundo #f6f6f8;"));
        assert_eq!(super::css("escuro"), "");
    }
}

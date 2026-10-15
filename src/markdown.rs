//! Markdown -> markup do Pango, pra prévia mostrar o .md formatado: títulos, negrito,
//! itálico, código, listas, citações e links. Não é um Markdown completo: é o que aparece
//! num README.

use gtk::glib::markup_escape_text;

const LINHAS: usize = 80;

/// Formatação dentro da linha: `código`, **negrito**, *itálico*, [link](url), ![imagem](url).
fn inline(linha: &str) -> String {
    let mut out = String::new();
    // Código primeiro: dentro dele nada mais vale
    for (i, parte) in linha.split('`').enumerate() {
        if i % 2 == 1 {
            out += &format!("<tt><span background='#ffffff14'>{}</span></tt>", markup_escape_text(parte));
            continue;
        }
        out += &enfeites(parte);
    }
    // Crase sem par: o que sobrou fica como estava
    if linha.matches('`').count() % 2 == 1 {
        return markup_escape_text(linha).to_string();
    }
    out
}

fn enfeites(texto: &str) -> String {
    // Links e imagens viram só o texto (sublinhado)
    let mut sem_links = String::new();
    let mut resto = texto;
    while let Some(ini) = resto.find('[') {
        let Some(fim) = resto[ini..].find("](").map(|f| ini + f) else { break };
        let Some(fecha) = resto[fim..].find(')').map(|f| fim + f) else { break };
        let imagem = ini > 0 && resto.as_bytes()[ini - 1] == b'!';
        sem_links += &resto[..if imagem { ini - 1 } else { ini }];
        let rotulo = &resto[ini + 1..fim];
        sem_links += &if imagem { format!("\u{E000}🖼 {rotulo}\u{E001}") } else { format!("\u{E000}{rotulo}\u{E001}") };
        resto = &resto[fecha + 1..];
    }
    sem_links += resto;
    let mut s = markup_escape_text(&sem_links).to_string();
    s = s.replace('\u{E000}', "<u>").replace('\u{E001}', "</u>");
    s = pares(&s, "**", "<b>", "</b>");
    s = pares(&s, "__", "<b>", "</b>");
    s = pares(&s, "*", "<i>", "</i>");
    pares(&s, "~~", "<s>", "</s>")
}

/// Troca marcadores em pares ("**a** e **b**") por abre/fecha. Marcador sem par fica como está.
fn pares(s: &str, marca: &str, abre: &str, fecha: &str) -> String {
    let partes: Vec<&str> = s.split(marca).collect();
    if partes.len() < 3 {
        return s.to_string();
    }
    let mut out = String::new();
    let fechados = (partes.len() - 1) / 2 * 2; // só os pares completos
    for (i, p) in partes.iter().enumerate() {
        if i > 0 {
            out += if i > fechados { marca } else if i % 2 == 1 { abre } else { fecha };
        }
        out += p;
    }
    out
}

/// Um pedaço da prévia: texto normal (quebra linha) ou bloco de código (não quebra, pra não
/// desmontar desenhos e alinhamentos; o que passar da largura fica cortado).
pub enum Bloco {
    Texto(String),
    Codigo(String),
}

/// O texto convertido em blocos. None se algum pedaço não virar markup válido (aí a prévia
/// mostra o texto puro).
pub fn blocos(texto: &str) -> Option<Vec<Bloco>> {
    let mut saida = Vec::new();
    let mut atual = String::new();
    let mut codigo = String::new();
    let mut em_codigo = false;
    for linha in texto.lines().take(LINHAS) {
        let t = linha.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            if em_codigo {
                saida.push(Bloco::Codigo(std::mem::take(&mut codigo)));
            } else if !atual.trim().is_empty() {
                saida.push(Bloco::Texto(pango(&std::mem::take(&mut atual))?));
            }
            atual.clear();
            em_codigo = !em_codigo;
            continue;
        }
        let destino = if em_codigo { &mut codigo } else { &mut atual };
        destino.push_str(linha);
        destino.push('\n');
    }
    if em_codigo && !codigo.is_empty() {
        saida.push(Bloco::Codigo(codigo));
    }
    if !atual.trim().is_empty() {
        saida.push(Bloco::Texto(pango(&atual)?));
    }
    // O código vai escapado e monoespaçado
    Some(
        saida
            .into_iter()
            .map(|b| match b {
                Bloco::Codigo(c) => Bloco::Codigo(format!("<tt>{}</tt>", markup_escape_text(c.trim_end_matches('\n')))),
                t => t,
            })
            .collect(),
    )
}

/// Markdown sem blocos de código convertido em markup.
fn pango(texto: &str) -> Option<String> {
    let mut out = Vec::new();
    for linha in texto.lines() {
        let t = linha.trim_start();
        let nivel = t.chars().take_while(|c| *c == '#').count();
        let r = if (1..=6).contains(&nivel) && t[nivel..].starts_with(' ') {
            let tamanho = match nivel {
                1 => "x-large",
                2 => "large",
                _ => "medium",
            };
            format!("<span size='{tamanho}' weight='bold'>{}</span>", inline(t[nivel..].trim()))
        } else if let Some(item) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("+ ")) {
            let recuo = " ".repeat((linha.len() - t.len()).min(8));
            let item = item.strip_prefix("[ ] ").map(|x| format!("☐ {x}")).or_else(|| item.strip_prefix("[x] ").map(|x| format!("☑ {x}"))).unwrap_or(item.to_string());
            format!("{recuo}  •  {}", inline(&item))
        } else if let Some(cit) = t.strip_prefix('>') {
            format!("<span foreground='#9a9aa2'><i>│ {}</i></span>", inline(cit.trim()))
        } else if t.len() >= 3 && t.chars().all(|c| c == '-' || c == '*' || c == '_') {
            "<span foreground='#55555c'>──────────────</span>".to_string()
        } else if t.starts_with('|') {
            // Tabela não cabe numa coluna estreita: cada linha vira um item, com a primeira
            // célula em negrito. A linha de separação (|---|---|) some.
            let celulas: Vec<&str> = t.trim_matches('|').split('|').map(str::trim).collect();
            if celulas.iter().all(|c| c.chars().all(|x| matches!(x, '-' | ':' | ' ')) ) {
                continue;
            }
            let (primeira, resto) = celulas.split_first().unwrap_or((&"", &[]));
            let resto: Vec<String> = resto.iter().filter(|c| !c.is_empty()).map(|c| inline(c)).collect();
            if resto.is_empty() {
                format!("<b>{}</b>", inline(primeira))
            } else {
                format!("<b>{}</b>  <span foreground='#9a9aa2'>—</span>  {}", inline(primeira), resto.join("  <span foreground='#9a9aa2'>·</span>  "))
            }
        } else {
            inline(linha)
        };
        out.push(r);
    }
    let markup = out.join("\n");
    gtk::pango::parse_markup(&markup, '\0').is_ok().then_some(markup)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converte() {
        assert_eq!(inline("um **dois** e *três*"), "um <b>dois</b> e <i>três</i>");
        assert_eq!(inline("veja [o site](http://x) e `a < b`"), "veja <u>o site</u> e <tt><span background='#ffffff14'>a &lt; b</span></tt>");
        assert_eq!(inline("2 * 3 = 6"), "2 * 3 = 6");
        let b = blocos("# Título\n\n- item **forte**\n\n```\nlet x = 1 < 2;\n```").unwrap();
        assert!(matches!(&b[0], Bloco::Texto(t) if t.contains("<span size='x-large' weight='bold'>Título</span>")));
        assert!(matches!(&b[1], Bloco::Codigo(c) if c == "<tt>let x = 1 &lt; 2;</tt>"));
        let tabela = pango("| A | B |\n|---|---|\n| `vs` | repos |").unwrap();
        assert!(!tabela.contains("---"));
        assert!(tabela.contains("<b><tt>"));
    }
}

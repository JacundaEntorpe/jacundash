//! Arquivos e pastas. No KDE usa o Baloo (a mesma busca do KRunner, que olha nome e
//! conteúdo); nos outros, o índice do locate/plocate ou, sem ele, o fd na home.
//! Os resultados se separam em categorias: Pastas, Imagens, Documentos...

use crate::config::home;
use crate::sistema::til;
use crate::engine::{Provider, Query};
use crate::krunner;
use crate::matcher;
use crate::model::{Effect, Item};
use gtk::gio;
use gtk::prelude::*;
use std::path::Path;
use std::process::Command;

pub struct Arquivos;

const BALOO: (&str, &str) = ("org.kde.runners.baloo", "/runner");
const LIMITE: usize = 40;

fn caminho_de(uri: &str) -> String {
    gio::File::for_uri(uri).path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| uri.to_string())
}

fn item(uri: String, nome: String, pasta: String, icone: String, categoria: String, nota: f32, kde: bool) -> Item {
    let caminho = caminho_de(&uri);
    let pai = Path::new(&caminho).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let abrir_pasta = if kde {
        Effect::Callback { data: format!("pasta:{uri}") } // o Baloo abre já com o arquivo selecionado
    } else {
        Effect::Open { uri: gio::File::for_path(&pai).uri().to_string() }
    };
    Item::new(nome, Effect::Open { uri })
        .preview(caminho.clone())
        .subtitle(pasta)
        .icon(icone)
        .category(categoria)
        .score(nota)
        .action("Abrir a pasta onde ele está", "document-open-folder", abrir_pasta)
        .action("Copiar o caminho", "edit-copy", Effect::Copy { text: caminho })
}

/// Sem Baloo: locate (índice do sistema, instantâneo) ou fd na home.
fn sem_baloo(texto: &str) -> Vec<Item> {
    let casa = home().to_string_lossy().into_owned();
    let mut caminhos: Vec<String> = Vec::new();
    for cmd in [vec!["plocate", "-i", "-b", "-l", "200", "--", texto], vec!["locate", "-i", "-b", "-l", "200", "--", texto]] {
        if let Ok(out) = Command::new(cmd[0]).args(&cmd[1..]).output() {
            if out.status.success() {
                caminhos = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                break;
            }
        }
    }
    if caminhos.is_empty() {
        if let Ok(out) = Command::new("fd").args(["-i", "-a", "--max-results", "200", "--", texto, &casa]).output() {
            caminhos = String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim_end_matches('/').to_string()).collect();
        }
    }
    caminhos
        .into_iter()
        // Só a home, sem pastas escondidas (.cache, .local...)
        .filter(|c| c.starts_with(&casa) && !c[casa.len()..].contains("/."))
        .take(LIMITE)
        .map(|c| {
            let p = Path::new(&c);
            let pasta = p.is_dir();
            let (tipo, _) = gio::content_type_guess(Some(p), None);
            let icone = if pasta {
                "folder".to_string()
            } else {
                IconExt::to_string(&gio::content_type_get_icon(&tipo)).map(|s| s.to_string()).unwrap_or_default()
            };
            let nome = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let pai = til(&p.parent().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default());
            let categoria = if pasta { "Pastas" } else { "Arquivos" };
            // Nome mais curto = bate mais certinho
            let nota = (0.6 - nome.len() as f32 * 0.003).max(0.3);
            item(gio::File::for_path(p).uri().to_string(), nome, pai, icone, categoria.into(), nota, false)
        })
        .collect()
}

impl Provider for Arquivos {
    fn category(&self) -> &str {
        "Arquivos"
    }
    fn keyword(&self) -> &str {
        "ar"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let texto = q.text.trim();
        // Sem a palavra-chave, só a partir de 3 letras: com menos vem arquivo demais.
        // ":" (emoji) e ">" (comando) não são busca de arquivo.
        if texto.chars().count() < if q.with_keyword { 1 } else { 3 } || (!q.with_keyword && texto.starts_with([':', '>'])) {
            return vec![];
        }
        if krunner::existe(BALOO.0) {
            let Some(ms) = krunner::buscar(BALOO.0, BALOO.1, texto) else { return vec![] };
            return ms
                .into_iter()
                .take(LIMITE)
                .filter_map(|m| {
                    let categoria = if m.category.is_empty() { "Arquivos".into() } else { m.category };
                    // O Baloo também acha pelo conteúdo. Na busca normal isso só polui ("desli"
                    // traz todo texto que fala de desligar): só entra quem tem a busca no nome.
                    // Com "ar texto", entra o conteúdo também, depois dos que batem pelo nome.
                    let pelo_nome = matcher::score(&q.norm, &m.text, "");
                    if pelo_nome == 0.0 && !q.with_keyword {
                        return None;
                    }
                    // Abaixo dos apps e das janelas: arquivo raramente é o que se quer de primeira
                    let nota = if pelo_nome > 0.0 { pelo_nome * 0.75 } else { 0.2 } + (m.relevance as f32) * 0.05;
                    Some(item(m.id, m.text, m.subtext, m.icon, categoria, nota, true))
                })
                .collect();
        }
        sem_baloo(texto)
    }
    fn callback(&self, data: &str) -> Vec<Effect> {
        if let Some(uri) = data.strip_prefix("pasta:") {
            krunner::rodar(BALOO.0, BALOO.1, uri, "openParentDir");
        }
        vec![]
    }
}

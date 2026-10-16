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
    caminhos_sem_baloo(texto)
        .into_iter()
        .take(LIMITE)
        .map(|c| {
            // Nome mais curto = bate mais certinho
            let nota = (0.6 - c.rsplit('/').next().unwrap_or_default().len() as f32 * 0.003).max(0.3);
            item_de_caminho(&c, nota)
        })
        .collect()
}

/// Os caminhos que o locate/plocate (ou o fd) acham com o texto no nome, só da home.
fn caminhos_sem_baloo(texto: &str) -> Vec<String> {
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
    // Só a home, sem pastas escondidas (.cache, .local...)
    caminhos.retain(|c| c.starts_with(&casa) && !c[casa.len()..].contains("/."));
    caminhos
}

fn item_de_caminho(c: &str, nota: f32) -> Item {
    let p = Path::new(c);
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
    item(gio::File::for_path(p).uri().to_string(), nome, pai, icone, categoria.into(), nota, false)
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

/// "img texto" e "vid texto": só imagens ou só vídeos. Sem texto, os mais novos.
/// No KDE pelo Baloo (que já sabe o tipo de cada arquivo); nos outros, pelo nome e pela extensão.
pub struct Midia {
    pub palavra: &'static str,
    pub categoria: &'static str,
    /// Tipo pro "baloosearch6 -t" (Image, Video).
    pub tipo_baloo: &'static str,
    /// Começo do tipo MIME (image/, video/).
    pub mime: &'static str,
}

pub const IMAGENS: Midia = Midia { palavra: "img", categoria: "Busca de imagens", tipo_baloo: "Image", mime: "image/" };
pub const VIDEOS: Midia = Midia { palavra: "vid", categoria: "Busca de vídeos", tipo_baloo: "Video", mime: "video/" };

impl Midia {
    fn pelo_baloo(&self, texto: &str) -> Option<Vec<String>> {
        let mut cmd = Command::new("baloosearch6");
        cmd.args(["-t", self.tipo_baloo, "-l", "200"]);
        if texto.is_empty() {
            cmd.args(["-s", "time"]);
        }
        let out = cmd.arg("--").arg(texto).output().ok().filter(|o| o.status.success())?;
        Some(String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.starts_with('/')).map(String::from).collect())
    }
}

impl Provider for Midia {
    fn category(&self) -> &str {
        self.categoria
    }
    fn keyword(&self) -> &str {
        self.palavra
    }
    fn needs_keyword(&self) -> bool {
        true
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let texto = q.text.trim();
        // Baloo desligado responde vazio: aí tenta pelo nome
        let baloo = if crate::sistema::tem("baloosearch6") { self.pelo_baloo(texto).filter(|c| !c.is_empty()) } else { None };
        let caminhos = match baloo {
            Some(c) => c,
            None if texto.is_empty() => return vec![],
            None => caminhos_sem_baloo(texto)
                .into_iter()
                .filter(|c| gio::content_type_guess(Some(Path::new(c)), None).0.starts_with(self.mime))
                .collect(),
        };
        let casa = home().to_string_lossy().into_owned();
        caminhos
            .iter()
            // Sem as pastas escondidas (cache de navegador, miniaturas...)
            .filter(|c| !c.strip_prefix(&casa).unwrap_or(c).contains("/."))
            .take(LIMITE)
            .enumerate()
            .map(|(i, c)| {
                let nome = c.rsplit('/').next().unwrap_or_default();
                // Quem tem o texto no nome vem antes; o resto na ordem que veio (sem texto: mais novo primeiro)
                let pelo_nome = if texto.is_empty() { 0.0 } else { matcher::score(&q.norm, nome, "") };
                let nota = pelo_nome + 0.5 - i as f32 * 0.001;
                let mut it = item_de_caminho(c, nota);
                it.category = None; // fica na categoria deste provedor (que dá pra desligar)
                it
            })
            .collect()
    }
}

#[cfg(test)]
mod teste_midia {
    use super::*;

    #[test]
    #[ignore = "precisa do Baloo"]
    fn imagens_e_videos() {
        for (m, t) in [(IMAGENS, "pokecroc"), (VIDEOS, "")] {
            let r = m.query(&Query { text: t.into(), norm: matcher::norm(t), with_keyword: true });
            eprintln!("{} {t:?}: {}", m.palavra, r.len());
            for i in r.iter().take(5) {
                eprintln!("  {} {} {}", i.score, i.title, i.subtitle);
            }
        }
    }
}

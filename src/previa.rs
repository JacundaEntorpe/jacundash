//! Prévia ao lado da lista: imagem, começo do texto, README (ou o conteúdo) de uma pasta.

use crate::sistema::til;
use gtk::prelude::*;
use gtk::{gdk, gio};
use std::io::Read;
use std::path::Path;

pub const LARGURA: i32 = 300;
const LINHAS: usize = 40;
const IMAGENS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "tif", "tiff", "avif"];

fn rotulo(texto: &str, classe: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(texto));
    l.set_xalign(0.0);
    l.set_wrap(true);
    l.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    l.set_max_width_chars(1);
    l.set_hexpand(true);
    l.add_css_class(classe);
    l
}

/// O texto do arquivo: formatado se for Markdown, monoespaçado se não.
fn bloco_de_texto(caminho: &Path, texto: &str) -> gtk::Widget {
    use crate::markdown::Bloco;
    let md = caminho.extension().is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"));
    if let Some(blocos) = md.then(|| crate::markdown::blocos(texto)).flatten() {
        let caixa = gtk::Box::new(gtk::Orientation::Vertical, 8);
        for b in blocos {
            match b {
                Bloco::Texto(m) => {
                    let l = rotulo("", "previa-md");
                    l.set_markup(&m);
                    caixa.append(&l);
                }
                Bloco::Codigo(m) => {
                    // Sem quebrar linha (desenho e alinhamento ficam inteiros), numa rolagem
                    // própria: o código largo rola pro lado em vez de alargar a prévia toda
                    let l = gtk::Label::new(None);
                    l.set_markup(&m);
                    l.set_xalign(0.0);
                    l.add_css_class("previa-codigo");
                    let rolagem = gtk::ScrolledWindow::builder()
                        .hscrollbar_policy(gtk::PolicyType::Automatic)
                        .vscrollbar_policy(gtk::PolicyType::Never)
                        .propagate_natural_height(true)
                        .child(&l)
                        .build();
                    caixa.append(&rolagem);
                }
            }
        }
        return caixa.upcast();
    }
    rotulo(texto, "previa-texto").upcast()
}

/// Começo de um arquivo de texto (None se parecer binário).
fn comeco(caminho: &Path) -> Option<String> {
    let mut buf = vec![0u8; 16 * 1024];
    let n = std::fs::File::open(caminho).ok()?.read(&mut buf).ok()?;
    buf.truncate(n);
    if buf.contains(&0) {
        return None;
    }
    let texto = String::from_utf8_lossy(&buf);
    Some(texto.lines().take(LINHAS).collect::<Vec<_>>().join("\n"))
}

fn tamanho(b: u64) -> String {
    let (v, u) = match b {
        b if b >= 1 << 30 => (b as f64 / (1u64 << 30) as f64, "GB"),
        b if b >= 1 << 20 => (b as f64 / (1u64 << 20) as f64, "MB"),
        b if b >= 1 << 10 => (b as f64 / 1024.0, "KB"),
        b => (b as f64, "bytes"),
    };
    if u == "bytes" { format!("{b} bytes") } else { format!("{v:.1} {u}") }
}

/// Monta a prévia de um arquivo ou pasta.
pub fn montar(caminho: &str) -> gtk::Box {
    let caixa = gtk::Box::new(gtk::Orientation::Vertical, 8);
    caixa.add_css_class("previa");
    let p = Path::new(caminho);
    let nome = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| caminho.to_string());
    caixa.append(&rotulo(&nome, "previa-titulo"));

    let Ok(meta) = std::fs::metadata(p) else {
        caixa.append(&rotulo("Não existe mais", "previa-info"));
        return caixa;
    };
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();

    if meta.is_dir() {
        caixa.append(&rotulo(&til(caminho), "previa-info"));
        // README, se tiver; senão o que tem dentro
        let readme = ["README.md", "readme.md", "README", "README.txt", "LEIA-ME.md", "Readme.md"].iter().map(|n| p.join(n)).find(|x| x.is_file());
        if let Some((arq, texto)) = readme.as_deref().and_then(|r| Some((r, comeco(r)?))) {
            caixa.append(&bloco_de_texto(arq, &texto));
        } else if let Ok(entradas) = std::fs::read_dir(p) {
            let mut nomes: Vec<String> = entradas
                .flatten()
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .map(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    if e.file_type().is_ok_and(|t| t.is_dir()) { format!("{n}/") } else { n }
                })
                .collect();
            nomes.sort_by_key(|n| (!n.ends_with('/'), n.to_lowercase()));
            let total = nomes.len();
            nomes.truncate(LINHAS);
            let mut texto = nomes.join("\n");
            if total > LINHAS {
                texto += &format!("\n... e mais {}", total - LINHAS);
            }
            caixa.append(&rotulo(if texto.is_empty() { "(vazia)" } else { &texto }, "previa-texto"));
        }
        return caixa;
    }

    let quando = meta.modified().ok().map(|t| glib_data(t)).unwrap_or_default();
    caixa.append(&rotulo(&format!("{}  ·  {quando}", tamanho(meta.len())), "previa-info"));

    if IMAGENS.contains(&ext.as_str()) {
        // Carrega por trás, já no tamanho da prévia: foto grande não trava a janela
        let img = gtk::Picture::new();
        img.set_content_fit(gtk::ContentFit::Contain);
        img.set_can_shrink(true);
        img.set_size_request(-1, 120);
        caixa.append(&img);
        let arquivo = gio::File::for_path(p);
        gtk::glib::spawn_future_local(async move {
            let Ok(leitura) = arquivo.read_future(gtk::glib::Priority::DEFAULT).await else { return };
            if let Ok(pix) = gtk::gdk_pixbuf::Pixbuf::from_stream_at_scale_future(&leitura, LARGURA - 20, 360, true).await {
                img.set_size_request(-1, pix.height().min(360));
                img.set_paintable(Some(&gdk::Texture::for_pixbuf(&pix)));
            }
        });
        return caixa;
    }
    if meta.len() < 50 * 1024 * 1024 {
        if let Some(texto) = comeco(p) {
            caixa.append(&bloco_de_texto(p, &texto));
            return caixa;
        }
    }
    // Sem prévia: o ícone grande do tipo do arquivo
    let (tipo, _) = gio::content_type_guess(Some(p), None);
    let img = gtk::Image::from_gicon(&gio::content_type_get_icon(&tipo));
    img.set_pixel_size(96);
    img.set_margin_top(20);
    caixa.append(&img);
    caixa.append(&rotulo(&gio::content_type_get_description(&tipo), "previa-info"));
    caixa
}

fn glib_data(t: std::time::SystemTime) -> String {
    let segundos = t.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    gtk::glib::DateTime::from_unix_local(segundos).ok().and_then(|d| d.format("%d/%m/%Y %H:%M").ok()).map(|s| s.to_string()).unwrap_or_default()
}

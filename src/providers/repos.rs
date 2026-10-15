//! Repositórios git ("vs") e pastas comuns ("pa"). Os dois saem da mesma varredura das
//! pastas da config, que roda em segundo plano e se refaz de tempos em tempos.

use crate::config::{Compartilhada, home};
use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};
use crate::sistema::{expandir, til};
use gtk::prelude::FileExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Repo {
    pub caminho: String,
    pub nome: String,
    pub git: bool,
    pub branch: Option<String>,
    /// Página do remote (GitHub, GitLab...), se tiver.
    pub web: Option<String>,
    /// Quando foi aberto no VS Code (segundos desde 1970), 0 = nunca.
    pub recente: f64,
}

#[derive(Default)]
struct Dados {
    repos: Vec<Repo>,
    pastas: Vec<String>,
    quando: Option<Instant>,
    /// A config usada nessa varredura: se mudar (aba de configurações), escaneia de novo na hora.
    usada: String,
}

fn chave(c: &crate::config::Repos) -> String {
    format!("{:?} {} {:?} {}", c.pastas, c.profundidade, c.ignorar, c.recentes_vscode)
}

pub struct Varredura {
    cfg: Compartilhada,
    dados: RwLock<Dados>,
    rodando: AtomicBool,
}

impl Varredura {
    pub fn new(cfg: Compartilhada) -> Arc<Varredura> {
        let v = Arc::new(Varredura { cfg, dados: RwLock::default(), rodando: AtomicBool::new(false) });
        v.talvez_escanear();
        v
    }

    /// Escaneia de novo em segundo plano se a última varredura ficou velha.
    fn talvez_escanear(self: &Arc<Self>) {
        let cfg = self.cfg.read().unwrap().repos.clone();
        let velho = {
            let d = self.dados.read().unwrap();
            d.usada != chave(&cfg) || d.quando.is_none_or(|q| q.elapsed() > Duration::from_secs(cfg.reescanear))
        };
        if velho && !self.rodando.swap(true, Ordering::SeqCst) {
            let v = self.clone();
            std::thread::spawn(move || {
                let (repos, pastas) = escanear(&cfg);
                *v.dados.write().unwrap() = Dados { repos, pastas, quando: Some(Instant::now()), usada: chave(&cfg) };
                v.rodando.store(false, Ordering::SeqCst);
            });
        }
    }

    pub fn repos(self: &Arc<Self>) -> Vec<Repo> {
        self.talvez_escanear();
        self.dados.read().unwrap().repos.clone()
    }

    pub fn pastas(self: &Arc<Self>) -> Vec<String> {
        self.talvez_escanear();
        self.dados.read().unwrap().pastas.clone()
    }
}

fn escanear(cfg: &crate::config::Repos) -> (Vec<Repo>, Vec<String>) {
    let mut repos: Vec<PathBuf> = Vec::new();
    let mut pastas: Vec<PathBuf> = Vec::new();
    fn andar(d: &Path, nivel: usize, cfg: &crate::config::Repos, repos: &mut Vec<PathBuf>, pastas: &mut Vec<PathBuf>) {
        if d.join(".git").exists() {
            repos.push(d.to_path_buf());
            return; // o que tem dentro de um repositório não entra
        }
        pastas.push(d.to_path_buf());
        if nivel >= cfg.profundidade {
            return;
        }
        let Ok(entradas) = std::fs::read_dir(d) else { return };
        for e in entradas.flatten() {
            let nome = e.file_name();
            let nome = nome.to_string_lossy();
            if nome.starts_with('.') || cfg.ignorar.iter().any(|i| i == nome.as_ref()) {
                continue;
            }
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                andar(&e.path(), nivel + 1, cfg, repos, pastas);
            }
        }
    }
    for p in &cfg.pastas {
        let p = expandir(p);
        if p.is_dir() {
            andar(&p, 0, cfg, &mut repos, &mut pastas);
        }
    }
    let recentes = if cfg.recentes_vscode { recentes_vscode() } else { vec![] };
    let mut saida: Vec<Repo> = repos.iter().map(|p| repo(p, true, 0.0)).collect();
    for (pasta, quando) in recentes {
        match saida.iter_mut().find(|r| r.caminho == pasta) {
            Some(r) => r.recente = r.recente.max(quando),
            None => {
                let p = Path::new(&pasta);
                saida.push(repo(p, p.join(".git").exists(), quando));
            }
        }
    }
    let caminhos: Vec<String> = pastas.into_iter().map(|p| p.to_string_lossy().into_owned()).filter(|p| !saida.iter().any(|r| &r.caminho == p)).collect();
    (saida, caminhos)
}

fn repo(p: &Path, git: bool, recente: f64) -> Repo {
    let (branch, web) = if git { ler_git(p) } else { (None, None) };
    Repo {
        caminho: p.to_string_lossy().into_owned(),
        nome: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        git,
        branch,
        web,
        recente,
    }
}

/// Branch e página do remote lidos direto da pasta .git (sem chamar o git: é instantâneo).
fn ler_git(pasta: &Path) -> (Option<String>, Option<String>) {
    let mut git = pasta.join(".git");
    if git.is_file() {
        // worktree ou submódulo: "gitdir: <caminho>"
        let Some(dir) = std::fs::read_to_string(&git).ok().and_then(|t| t.trim().strip_prefix("gitdir:").map(|d| d.trim().to_string())) else {
            return (None, None);
        };
        git = pasta.join(dir);
    }
    let branch = std::fs::read_to_string(git.join("HEAD")).ok().map(|h| {
        let h = h.trim();
        h.strip_prefix("ref: refs/heads/").map(String::from).unwrap_or_else(|| h.chars().take(8).collect())
    });
    let web = std::fs::read_to_string(git.join("config")).ok().and_then(|conf| {
        // url do [remote "origin"], ou a primeira que aparecer
        let mut na_origin = false;
        let mut primeira = None;
        for linha in conf.lines() {
            let l = linha.trim();
            if l.starts_with('[') {
                na_origin = l == "[remote \"origin\"]";
            } else if let Some(url) = l.strip_prefix("url").map(|r| r.trim_start().trim_start_matches('=').trim()) {
                if na_origin {
                    return url_web(url);
                }
                primeira.get_or_insert_with(|| url.to_string());
            }
        }
        primeira.and_then(|u| url_web(&u))
    });
    (branch, web)
}

/// git@github.com:x/y.git -> https://github.com/x/y
pub fn url_web(remote: &str) -> Option<String> {
    let r = remote.trim_end_matches(".git");
    let r2 = r.strip_prefix("ssh://").unwrap_or(r);
    if let Some(resto) = r2.strip_prefix("git@") {
        let (host, caminho) = resto.split_once([':', '/'])?;
        return Some(format!("https://{host}/{caminho}"));
    }
    if r.starts_with("http://") || r.starts_with("https://") {
        // tira usuário/token da URL (https://user:token@github.com/...)
        let (esquema, resto) = r.split_once("//")?;
        let (host, caminho) = resto.split_once('/').unwrap_or((resto, ""));
        let host = host.rsplit('@').next().unwrap_or(host);
        return Some(if caminho.is_empty() { format!("{esquema}//{host}") } else { format!("{esquema}//{host}/{caminho}") });
    }
    None
}

/// {pasta: quando foi aberta} pelo workspaceStorage do VS Code (e do VSCodium).
fn recentes_vscode() -> Vec<(String, f64)> {
    let mut saida = Vec::new();
    for app in ["Code", "VSCodium", "Code - OSS"] {
        let Ok(entradas) = std::fs::read_dir(home().join(".config").join(app).join("User/workspaceStorage")) else { continue };
        for e in entradas.flatten() {
            let Ok(texto) = std::fs::read_to_string(e.path().join("workspace.json")) else { continue };
            let Some(uri) = serde_json::from_str::<serde_json::Value>(&texto).ok().and_then(|v| v.get("folder")?.as_str().map(String::from)) else {
                continue;
            };
            let Some(pasta) = gtk::gio::File::for_uri(&uri).path() else { continue };
            if !pasta.is_dir() {
                continue;
            }
            let quando = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0.0, |d| d.as_secs_f64());
            saida.push((pasta.to_string_lossy().into_owned(), quando));
        }
    }
    saida
}

fn agora() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

fn comando(texto: &str, caminho: &str) -> Vec<String> {
    let mut c: Vec<String> = texto.split_whitespace().map(String::from).collect();
    c.push(caminho.to_string());
    c
}

/// As ações de pasta que repositórios e pastas comuns dividem.
/// Terminal em janela nova e, se o terminal tiver abas, numa aba nova.
pub fn acoes_de_terminal(mut item: Item, caminho: &str, terminal: &str) -> Item {
    item = item.action("Abrir no terminal (janela nova)", "utilities-terminal", Effect::Exec { cmd: vec![], terminal: true, cwd: Some(caminho.into()), hold: false, tab: Some(false) });
    if crate::sistema::tem_aba(terminal) {
        item = item.action("Abrir numa aba nova do terminal", "utilities-terminal", Effect::Exec { cmd: vec![], terminal: true, cwd: Some(caminho.into()), hold: false, tab: Some(true) });
    }
    item
}

fn acoes_de_pasta(item: Item, caminho: &str, cfg: &crate::config::Repos, terminal: &str, editor_tambem: bool) -> Item {
    let claude: Vec<&str> = cfg.claude.split_whitespace().collect();
    let mut item = acoes_de_terminal(item, caminho, terminal).action("Abrir o Claude Code aqui", "dialog-messages", Effect::terminal(&claude, Some(caminho)));
    if editor_tambem {
        item = item.action(
            "Abrir no editor",
            "accessories-text-editor",
            Effect::Exec { cmd: comando(&cfg.editor, caminho), terminal: false, cwd: Some(caminho.into()), hold: false, tab: None },
        );
    }
    item
}

pub struct Repos {
    pub varredura: Arc<Varredura>,
    pub cfg: Compartilhada,
}

impl Provider for Repos {
    fn category(&self) -> &str {
        "Repositórios"
    }
    fn keyword(&self) -> &str {
        "vs"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if !q.with_keyword && q.norm.chars().count() < 2 {
            return vec![];
        }
        let cfg = self.cfg.read().unwrap().repos.clone();
        let terminal = self.cfg.read().unwrap().busca.terminal.clone();
        let agora = agora();
        let mut itens = Vec::new();
        for r in self.varredura.repos() {
            let mut nota = matcher::score(&q.norm, &r.nome, &til(&r.caminho));
            // Sem a palavra-chave, só quando bate bem pelo nome
            if nota == 0.0 || (!q.with_keyword && nota < 0.8) {
                continue;
            }
            if r.recente > 0.0 {
                nota += 0.1 * (1.0 - (agora - r.recente) / (30.0 * 86400.0)).max(0.0) as f32;
            }
            let mut sub = til(&r.caminho);
            if let Some(b) = &r.branch {
                sub += &format!("  ·  {b}");
            }
            if !r.git {
                sub += "  ·  recente do VS Code";
            }
            let editor = Effect::Exec { cmd: comando(&cfg.editor, &r.caminho), terminal: false, cwd: Some(r.caminho.clone()), hold: false, tab: None };
            let mut item = Item::new(&r.nome, editor).subtitle(sub).icon(if r.git { "folder-git" } else { "folder" }).score(nota.min(1.0)).preview(r.caminho.clone());
            item = acoes_de_pasta(item, &r.caminho, &cfg, &terminal, false)
                .action("Abrir no gerenciador de arquivos", "system-file-manager", Effect::abrir_caminho(&r.caminho));
            if let Some(web) = &r.web {
                item = item
                    .action("Abrir o remote no navegador", "internet-web-browser", Effect::Open { uri: web.clone() })
                    .action("git pull", "vcs-pull", Effect::tarefa(&["git", "pull", "--ff-only"], Some(&r.caminho), format!("{}: git pull", r.nome)));
            }
            itens.push(item.action("Copiar o caminho", "edit-copy", Effect::Copy { text: r.caminho.clone() }));
        }
        itens
    }
}

pub struct Pastas {
    pub varredura: Arc<Varredura>,
    pub cfg: Compartilhada,
}

impl Provider for Pastas {
    fn category(&self) -> &str {
        "Pastas"
    }
    fn keyword(&self) -> &str {
        "pa"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if !q.with_keyword && q.norm.chars().count() < 3 {
            return vec![];
        }
        let cfg = self.cfg.read().unwrap().repos.clone();
        let terminal = self.cfg.read().unwrap().busca.terminal.clone();
        let mut itens = Vec::new();
        for caminho in self.varredura.pastas() {
            let p = Path::new(&caminho);
            let nome = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let nota = matcher::score(&q.norm, &nome, "");
            if nota == 0.0 || (!q.with_keyword && nota < 0.8) {
                continue;
            }
            // O subtítulo é a pasta de cima, igual o Baloo: assim a mesma pasta não aparece duas vezes
            let pai = til(&p.parent().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default());
            let item = Item::new(&nome, Effect::abrir_caminho(&caminho)).subtitle(pai).icon("folder").score(nota + 0.01).preview(caminho.clone());
            itens.push(acoes_de_pasta(item, &caminho, &cfg, &terminal, true).action("Copiar o caminho", "edit-copy", Effect::Copy { text: caminho.clone() }));
        }
        itens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remotes() {
        assert_eq!(url_web("git@github.com:JacundaEntorpe/krocket.git").as_deref(), Some("https://github.com/JacundaEntorpe/krocket"));
        assert_eq!(url_web("ssh://git@gitlab.com/a/b.git").as_deref(), Some("https://gitlab.com/a/b"));
        assert_eq!(url_web("https://user:token@github.com/a/b.git").as_deref(), Some("https://github.com/a/b"));
        assert_eq!(url_web("https://github.com/a/b").as_deref(), Some("https://github.com/a/b"));
        assert_eq!(url_web("/srv/repo.git"), None);
    }
}

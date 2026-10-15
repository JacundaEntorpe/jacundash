//! O que um provedor (embutido ou plugin) devolve. Tudo aqui também é o formato JSON dos plugins.

use serde::{Deserialize, Serialize};

/// O que acontece quando um resultado (ou uma ação dele) é escolhido.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effect {
    /// Abre um app pelo id do .desktop ("org.kde.kate.desktop").
    Launch { app: String },
    /// Abre um arquivo, pasta ou URL no programa padrão.
    Open { uri: String },
    /// Copia o texto pra área de transferência.
    Copy { text: String },
    /// Roda um comando. Com terminal = true, roda dentro do terminal configurado (cmd vazio =
    /// só abre o terminal na pasta). hold deixa o terminal aberto quando o comando termina.
    /// tab: true = aba nova na janela do terminal que já está aberta; false = janela nova;
    /// sem nada = o que estiver nas configurações (de fábrica, tenta a aba primeiro).
    Exec {
        cmd: Vec<String>,
        #[serde(default)]
        terminal: bool,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        hold: bool,
        #[serde(default)]
        tab: Option<bool>,
    },
    /// Roda um comando em segundo plano e avisa numa notificação quando terminar
    /// ("git pull", "docker restart"...). `label` é o nome que aparece no aviso.
    Task {
        cmd: Vec<String>,
        #[serde(default)]
        cwd: Option<String>,
        label: String,
    },
    /// Troca o texto da busca e deixa a janela aberta ("vs " pra entrar num plugin).
    Query { text: String },
    /// Devolve pro provedor que criou o resultado decidir (o plugin recebe `data`).
    Callback { data: String },
    /// Coisas do próprio jacundash ("config" abre as configurações).
    Internal { action: String },
    /// Não faz nada (linhas só de informação).
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub label: String,
    #[serde(default)]
    pub icon: String,
    pub effect: Effect,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    /// Nome de ícone do tema ("folder"), caminho de arquivo ou GIcon serializado.
    #[serde(default)]
    pub icon: String,
    /// 0 a 1: quanto bate com a busca. Ordena dentro da categoria e as categorias entre si.
    #[serde(default)]
    pub score: f32,
    pub effect: Effect,
    #[serde(default)]
    pub actions: Vec<Action>,
    /// Categoria deste item, se for diferente da do provedor (os arquivos se separam em
    /// "Pastas", "Imagens"...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Título longo em várias linhas (a resposta da IA), em vez de cortar com "...".
    #[serde(default)]
    pub multiline: bool,
    /// Arquivo ou pasta pra prévia ao lado da lista (imagem, começo do texto, README).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

impl Effect {
    /// Abre um terminal na pasta, opcionalmente rodando um comando.
    pub fn terminal(cmd: &[&str], cwd: Option<&str>) -> Effect {
        Effect::Exec {
            cmd: cmd.iter().map(|s| s.to_string()).collect(),
            terminal: true,
            cwd: cwd.map(String::from),
            hold: false,
            tab: None,
        }
    }
    /// Comando no terminal que fica aberto no fim (status, ping...).
    pub fn terminal_segurando(cmd: &[&str]) -> Effect {
        Effect::Exec { cmd: cmd.iter().map(|s| s.to_string()).collect(), terminal: true, cwd: None, hold: true, tab: None }
    }
    /// Comando solto, fora do terminal.
    pub fn exec(cmd: &[&str]) -> Effect {
        Effect::Exec { cmd: cmd.iter().map(|s| s.to_string()).collect(), terminal: false, cwd: None, hold: false, tab: None }
    }
    pub fn tarefa(cmd: &[&str], cwd: Option<&str>, label: impl Into<String>) -> Effect {
        Effect::Task { cmd: cmd.iter().map(|s| s.to_string()).collect(), cwd: cwd.map(String::from), label: label.into() }
    }
    pub fn abrir_caminho(caminho: &str) -> Effect {
        Effect::Open { uri: gtk::gio::prelude::FileExt::uri(&gtk::gio::File::for_path(caminho)).to_string() }
    }
}

impl Item {
    pub fn new(title: impl Into<String>, effect: Effect) -> Self {
        Item { title: title.into(), subtitle: String::new(), icon: String::new(), score: 0.5, effect, actions: vec![], category: None, multiline: false, preview: None }
    }
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    pub fn icon(mut self, s: impl Into<String>) -> Self {
        self.icon = s.into();
        self
    }
    pub fn score(mut self, s: f32) -> Self {
        self.score = s;
        self
    }
    pub fn preview(mut self, caminho: impl Into<String>) -> Self {
        self.preview = Some(caminho.into());
        self
    }
    pub fn category(mut self, c: impl Into<String>) -> Self {
        self.category = Some(c.into());
        self
    }
    pub fn action(mut self, label: impl Into<String>, icon: impl Into<String>, effect: Effect) -> Self {
        self.actions.push(Action { label: label.into(), icon: icon.into(), effect });
        self
    }
}

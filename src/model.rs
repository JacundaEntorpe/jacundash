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
    /// Roda um comando. Com terminal = true, roda dentro do terminal configurado.
    Exec {
        cmd: Vec<String>,
        #[serde(default)]
        terminal: bool,
        #[serde(default)]
        cwd: Option<String>,
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
}

impl Item {
    pub fn new(title: impl Into<String>, effect: Effect) -> Self {
        Item { title: title.into(), subtitle: String::new(), icon: String::new(), score: 0.5, effect, actions: vec![], category: None }
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
    pub fn category(mut self, c: impl Into<String>) -> Self {
        self.category = Some(c.into());
        self
    }
    pub fn action(mut self, label: impl Into<String>, icon: impl Into<String>, effect: Effect) -> Self {
        self.actions.push(Action { label: label.into(), icon: icon.into(), effect });
        self
    }
}

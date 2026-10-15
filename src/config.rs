//! ~/.config/jacundash/config.toml. Tudo tem padrão: o arquivo só precisa do que mudar.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// A config que a tela e os provedores dividem (a aba de configurações muda ela na hora).
pub type Compartilhada = Arc<RwLock<Config>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub aparencia: Aparencia,
    pub busca: Busca,
    pub categorias: Categorias,
    pub repos: Repos,
    pub servicos: Servicos,
    pub tailscale: Tailscale,
    pub ia: Ia,
    pub hermes: Hermes,
    pub extras: Extras,
    pub acoes: Acoes,
    /// Textos prontos: o nome acha, o Enter copia o texto.
    pub textos: Vec<Texto>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Texto {
    pub nome: String,
    pub texto: String,
}

/// Quais ações (Shift+Enter e botões da linha) aparecem, por categoria.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Acoes {
    /// Ações escondidas: { "Repositórios" = ["git pull"] }. "Todas" vale pra todas as categorias.
    pub escondidas: HashMap<String, Vec<String>>,
    /// Quais viram botão na linha, na ordem. Categoria que não está aqui: as 3 primeiras.
    pub botoes: HashMap<String, Vec<String>>,
}

impl Acoes {
    pub fn escondida(&self, categoria: &str, rotulo: &str) -> bool {
        // "Desafixar" é o mesmo lugar do "Fixar no topo"
        let rotulo = if rotulo == "Desafixar" { "Fixar no topo" } else { rotulo };
        [categoria, "Todas"].iter().any(|c| self.escondidas.get(*c).is_some_and(|l| l.iter().any(|x| x == rotulo)))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Extras {
    /// Busca vazia mostra o que você abriu por último (apps, repositórios, pastas...).
    pub recentes: bool,
    /// Bloquear, suspender, reiniciar, desligar, sair da sessão.
    pub sistema: bool,
    /// Abas abertas e histórico do navegador (KDE, com a integração do navegador).
    pub navegador: bool,
    /// Prévia ao lado da lista: imagem, começo do texto, README do repositório.
    pub previa: bool,
}

impl Default for Extras {
    fn default() -> Self {
        Extras { recentes: false, sistema: false, navegador: true, previa: true }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Ia {
    /// Modelo do Ollama ("gemma3:12b"). Vazio = o primeiro que estiver instalado.
    pub modelo: String,
    /// Endereço do Ollama.
    pub url: String,
    /// Open WebUI, pra ação "Continuar no Open WebUI". Vazio = sem a ação.
    pub webui: String,
    /// Quando a resposta termina, ela já vai pra área de transferência.
    pub copiar_resposta: bool,
    /// Quanto tempo o modelo fica na memória depois de responder ("5m", "1h", "0").
    pub manter_carregado: String,
    /// Instruções que vão antes de toda pergunta.
    pub instrucoes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Hermes {
    /// Comando do Hermes (o agente que mexe no PC).
    pub comando: String,
}

impl Default for Ia {
    fn default() -> Self {
        Ia {
            modelo: String::new(),
            url: "http://localhost:11434".into(),
            webui: String::new(),
            copiar_resposta: true,
            manter_carregado: "5m".into(),
            instrucoes: "Responda em português do Brasil, curto e direto (no máximo 3 frases). \
                         Se a pergunta pedir um comando, responda só com o comando, sem crases."
                .into(),
        }
    }
}

impl Default for Hermes {
    fn default() -> Self {
        Hermes { comando: "hermes".into() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Repos {
    /// Onde procurar repositórios git (e as pastas comuns do "pa").
    pub pastas: Vec<String>,
    /// Quantas pastas pra dentro ele procura.
    pub profundidade: usize,
    /// Pastas que nunca entram (além das escondidas, que começam com ponto).
    pub ignorar: Vec<String>,
    /// Inclui as pastas abertas no VS Code (mesmo sem git) e sobe as recentes.
    pub recentes_vscode: bool,
    /// Editor do Enter ("code", "codium", "code --new-window", "kate"...).
    pub editor: String,
    /// Comando da ação "Abrir o Claude Code aqui".
    pub claude: String,
    /// Segundos entre uma varredura e outra.
    pub reescanear: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Servicos {
    /// O que aparece com "sv" sozinho (nome sem o .service). Os que falharam aparecem sempre.
    pub favoritos: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Tailscale {
    /// Usuário do SSH por aparelho: { vps = "ubuntu" }
    pub usuarios: HashMap<String, String>,
}

impl Default for Repos {
    fn default() -> Self {
        Repos {
            pastas: ["~/Projetos", "~/Projects", "~/Documentos", "~/Documents", "~/dev", "~/src", "~/code"].map(String::from).to_vec(),
            profundidade: 4,
            ignorar: ["node_modules", "vendor", "build", "dist", "target", "venv", "__pycache__"].map(String::from).to_vec(),
            recentes_vscode: true,
            editor: "code".into(),
            claude: "claude".into(),
            reescanear: 120,
        }
    }
}

impl Default for Servicos {
    fn default() -> Self {
        Servicos { favoritos: ["docker", "tailscaled", "ollama", "bluetooth", "NetworkManager", "sshd"].map(String::from).to_vec() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Aparencia {
    /// Largura da caixa em pixels.
    pub largura: i32,
    /// Altura máxima da lista em pixels (a caixa encolhe quando tem pouca coisa).
    pub altura_max: i32,
    /// Distância do topo da tela em pixels (0 = colado, como o KRunner).
    pub topo: i32,
    /// Tamanho dos ícones em pixels.
    pub icone: i32,
    /// "escuro", "claro" ou "sistema" (as cores do KDE ou do GNOME).
    pub tema: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Busca {
    /// Texto solto: "Pesquisar na web" usa isso ({} vira o texto).
    pub web: String,
    pub fechar_ao_perder_foco: bool,
    /// Terminal dos comandos com terminal = true ("konsole", "kitty"...). Vazio = acha sozinho.
    pub terminal: String,
    /// Comandos no terminal abrem numa aba nova da janela aberta (se o terminal tiver abas).
    pub preferir_aba: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Categorias {
    /// Desempate quando duas categorias batem igual com a busca.
    pub ordem: Vec<String>,
    /// Quantos resultados cada categoria mostra (sem palavra-chave).
    pub limite: usize,
    /// Limite por categoria, pelo nome ("Aplicativos" = 10).
    pub limites: HashMap<String, usize>,
    /// Categorias desligadas.
    pub esconder: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            aparencia: Aparencia::default(),
            busca: Busca::default(),
            categorias: Categorias::default(),
            repos: Repos::default(),
            servicos: Servicos::default(),
            tailscale: Tailscale::default(),
            ia: Ia::default(),
            hermes: Hermes::default(),
            extras: Extras::default(),
            acoes: Acoes::default(),
            textos: vec![],
        }
    }
}

impl Default for Aparencia {
    fn default() -> Self {
        Aparencia { largura: 640, altura_max: 460, topo: 0, icone: 22, tema: "escuro".into() }
    }
}

impl Default for Busca {
    fn default() -> Self {
        Busca {
            web: "https://www.google.com/search?q={}".into(),
            fechar_ao_perder_foco: true,
            terminal: String::new(),
            preferir_aba: true,
        }
    }
}

impl Default for Categorias {
    fn default() -> Self {
        Categorias {
            ordem: ["Ajuda", "Calculadora", "Conversor", "Comando", "Histórico de comandos", "Recentes", "Aplicativos", "Janelas", "Repositórios", "Git", "Docker", "Serviços", "Tailscale", "IA", "Hermes", "Textos prontos", "Emoji", "Área de transferência", "Sistema", "Jacundash", "Abas abertas", "Pastas", "Documentos", "Imagens", "Arquivos", "Histórico do navegador", "Web"].map(String::from).to_vec(),
            limite: 6,
            limites: HashMap::from([("Aplicativos".into(), 8), ("Ajuda".into(), 50)]),
            esconder: vec![],
        }
    }
}

/// O que o jacundash gravou por último (pra não recarregar a própria gravação).
fn ultimo_salvo() -> &'static std::sync::Mutex<String> {
    static U: std::sync::OnceLock<std::sync::Mutex<String>> = std::sync::OnceLock::new();
    U.get_or_init(|| std::sync::Mutex::new(String::new()))
}

/// Copia os valores de `novo` pra `dest`, mexendo só no que mudou e mantendo os comentários.
fn mesclar(dest: &mut toml_edit::Table, novo: &toml_edit::Table) {
    use toml_edit::Item;
    for (chave, item) in novo.iter() {
        match (dest.get_mut(chave), item) {
            (Some(Item::Table(d)), Item::Table(n)) => mesclar(d, n),
            (Some(Item::Value(d)), Item::Value(n)) => {
                if d.to_string().trim() != n.to_string().trim() {
                    let enfeite = d.decor().clone(); // o comentário do lado do valor
                    *d = n.clone();
                    *d.decor_mut() = enfeite;
                }
            }
            (Some(d), n) => *d = n.clone(),
            (None, n) => {
                dest.insert(chave, n.clone());
            }
        }
    }
}

pub fn pasta_config() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
        .join("jacundash")
}

pub fn pasta_estado() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/state"))
        .join("jacundash")
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

impl Config {
    pub fn carregar() -> Config {
        let caminho = pasta_config().join("config.toml");
        let lido = std::fs::read_to_string(&caminho);
        // Guarda o que foi lido: a vigia compara com isso pra saber se o arquivo mudou por fora
        *ultimo_salvo().lock().unwrap() = lido.as_deref().unwrap_or_default().to_string();
        match lido {
            Ok(texto) => toml::from_str(&texto).unwrap_or_else(|e| {
                eprintln!("jacundash: {} inválido ({e}), usando o padrão", caminho.display());
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    /// Grava o config.toml (a aba de configurações chama isso a cada mudança). Só troca os
    /// valores que mudaram: comentários e a ordem do arquivo ficam como você deixou.
    pub fn salvar(&self) {
        let pasta = pasta_config();
        let caminho = pasta.join("config.toml");
        let novo: toml_edit::DocumentMut = match toml::to_string_pretty(self).map_err(|e| e.to_string()).and_then(|t| t.parse().map_err(|e: toml_edit::TomlError| e.to_string())) {
            Ok(d) => d,
            Err(e) => return eprintln!("jacundash: não deu pra salvar a config: {e}"),
        };
        let mut doc: toml_edit::DocumentMut = std::fs::read_to_string(&caminho).ok().and_then(|t| t.parse().ok()).unwrap_or_else(|| {
            "# Config do jacundash. Dá pra mudar pela aba de configurações (Ctrl+, ou busque \"config\")\n\
             # ou aqui mesmo: o jacundash percebe sozinho quando o arquivo muda.\n\n"
                .parse()
                .unwrap()
        });
        mesclar(doc.as_table_mut(), novo.as_table());
        let texto = doc.to_string();
        *ultimo_salvo().lock().unwrap() = texto.clone();
        if let Err(e) = std::fs::create_dir_all(&pasta).and_then(|_| std::fs::write(&caminho, texto)) {
            eprintln!("jacundash: não deu pra salvar a config: {e}");
        }
    }

    /// true se o arquivo tem algo que o próprio jacundash não escreveu (editado na mão).
    pub fn mudou_por_fora() -> bool {
        let atual = std::fs::read_to_string(pasta_config().join("config.toml")).unwrap_or_default();
        *ultimo_salvo().lock().unwrap() != atual
    }

    pub fn limite_de(&self, categoria: &str) -> usize {
        self.categorias.limites.get(categoria).copied().unwrap_or(self.categorias.limite)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mesclar_mantem_comentarios() {
        let mut doc: toml_edit::DocumentMut = "# meu comentário\n[busca]\nweb = \"https://x/{}\"  # meu buscador\n".parse().unwrap();
        let novo: toml_edit::DocumentMut = "[busca]\nweb = \"https://y/{}\"\nterminal = \"kitty\"\n".parse().unwrap();
        super::mesclar(doc.as_table_mut(), novo.as_table());
        let t = doc.to_string();
        assert!(t.contains("# meu comentário"), "{t}");
        assert!(t.contains("web = \"https://y/{}\"  # meu buscador"), "{t}");
        assert!(t.contains("terminal = \"kitty\""), "{t}");
    }
}

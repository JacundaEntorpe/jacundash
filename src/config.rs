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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Busca {
    /// Texto solto: "Pesquisar na web" usa isso ({} vira o texto).
    pub web: String,
    pub fechar_ao_perder_foco: bool,
    /// Terminal dos comandos com terminal = true ("konsole", "kitty"...). Vazio = acha sozinho.
    pub terminal: String,
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
        }
    }
}

impl Default for Aparencia {
    fn default() -> Self {
        Aparencia { largura: 640, altura_max: 460, topo: 0, icone: 22 }
    }
}

impl Default for Busca {
    fn default() -> Self {
        Busca {
            web: "https://www.google.com/search?q={}".into(),
            fechar_ao_perder_foco: true,
            terminal: String::new(),
        }
    }
}

impl Default for Categorias {
    fn default() -> Self {
        Categorias {
            ordem: ["Calculadora", "Aplicativos", "Janelas", "Repositórios", "Git", "Docker", "Serviços", "Tailscale", "IA", "Hermes", "Jacundash", "Pastas", "Documentos", "Imagens", "Arquivos", "Web"].map(String::from).to_vec(),
            limite: 6,
            limites: HashMap::from([("Aplicativos".into(), 8)]),
            esconder: vec![],
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
        match std::fs::read_to_string(&caminho) {
            Ok(texto) => toml::from_str(&texto).unwrap_or_else(|e| {
                eprintln!("jacundash: {} inválido ({e}), usando o padrão", caminho.display());
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    /// Grava o config.toml (a aba de configurações chama isso a cada mudança).
    pub fn salvar(&self) {
        let pasta = pasta_config();
        let texto = match toml::to_string_pretty(self) {
            Ok(t) => t,
            Err(e) => return eprintln!("jacundash: não deu pra salvar a config: {e}"),
        };
        let cabecalho = "# Feito pela aba de configurações do jacundash (Ctrl+, ou busque \"config\").\n\
                         # Dá pra editar na mão também; depois rode: jacundash --sair\n\n";
        if let Err(e) = std::fs::create_dir_all(&pasta).and_then(|_| std::fs::write(pasta.join("config.toml"), cabecalho.to_string() + &texto)) {
            eprintln!("jacundash: não deu pra salvar a config: {e}");
        }
    }

    pub fn limite_de(&self, categoria: &str) -> usize {
        self.categorias.limites.get(categoria).copied().unwrap_or(self.categorias.limite)
    }
}

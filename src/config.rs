//! ~/.config/jacundash/config.toml. Tudo tem padrão: o arquivo só precisa do que mudar.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub aparencia: Aparencia,
    pub busca: Busca,
    pub categorias: Categorias,
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
        Config { aparencia: Aparencia::default(), busca: Busca::default(), categorias: Categorias::default() }
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
            ordem: ["Calculadora", "Aplicativos", "Janelas", "Jacundash", "Pastas", "Documentos", "Imagens", "Arquivos", "Web"].map(String::from).to_vec(),
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

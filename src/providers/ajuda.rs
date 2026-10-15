//! "help", "ajuda" ou "?": tudo o que dá pra digitar no jacundash. Enter já começa a busca
//! com aquilo (o "vs " pra entrar nos repositórios, por exemplo).

use crate::engine::{Provider, Query};
use crate::model::{Effect, Item};

pub struct Ajuda {
    /// (o que digitar, o que faz, ícone, texto que o Enter põe na busca)
    itens: Vec<(String, String, String, String)>,
}

const GATILHOS: &[&str] = &["help", "ajuda", "?", "comandos", "atalhos", "prefixos"];

/// Os que não são palavra-chave de provedor: prefixos e coisas que funcionam sozinhas.
const FIXOS: &[(&str, &str, &str, &str)] = &[
    ("=2^10*3", "Calculadora (o = é opcional: 15% de 80, sqrt(2)...)", "accessories-calculator", "="),
    ("10 usd em brl", "Conversor de moedas e unidades (5 km em milhas, 100 f em c)", "accessories-calculator", ""),
    (">comando", "Roda no terminal; também mostra o histórico de comandos", "utilities-terminal", ">"),
    (":nome", "Emoji pelo nome (:fogo)", "face-smile", ":"),
    ("traduz texto  /  explica comando", "A IA local traduz ou explica (com ou sem o \"ia\" na frente)", "dialog-messages", "traduz "),
    ("timer 10m chá  /  lembrete 18:30 reunião", "Timer e lembrete com notificação e alarme; \"timer\" lista os ativos", "chronometer", "timer "),
    ("kill nome", "Encerra programas pelo nome (Shift+Enter força)", "process-stop", "kill "),
    ("gh:texto  ou  gh texto", "Atalhos de busca da web (yt, gh, aur, protondb...)", "internet-web-browser", ""),
    ("Tab / Shift+Tab", "Pula pra próxima categoria / volta", "input-keyboard", ""),
    ("Shift+Enter  ou  clique direito", "Todas as ações do resultado (fixar, esconder...)", "input-keyboard", ""),
    ("Ctrl+,", "Configurações", "preferences-system", "config"),
];

impl Ajuda {
    /// `palavras`: (palavra-chave, categoria) de cada provedor, plugins incluídos.
    pub fn new(palavras: Vec<(String, String)>) -> Ajuda {
        let mut itens: Vec<(String, String, String, String)> = palavras
            .into_iter()
            .filter(|(p, _)| !p.is_empty())
            .map(|(p, c)| (format!("{p} …"), format!("Só {c}"), "system-search".to_string(), format!("{p} ")))
            .collect();
        itens.extend(FIXOS.iter().map(|(a, b, c, d)| (a.to_string(), b.to_string(), c.to_string(), d.to_string())));
        Ajuda { itens }
    }
}

impl Provider for Ajuda {
    fn category(&self) -> &str {
        "Ajuda"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let t = q.norm.trim();
        if t.is_empty() || !GATILHOS.iter().any(|g| *g == t || (t.len() >= 3 && g.starts_with(t))) {
            return vec![];
        }
        self.itens
            .iter()
            .enumerate()
            .map(|(i, (digitar, faz, icone, busca))| {
                let efeito = if busca.is_empty() { Effect::None } else { Effect::Query { text: busca.clone() } };
                // Na ordem da lista; todas juntas (ajuda é pra ver tudo)
                Item::new(digitar, efeito).subtitle(faz).icon(icone).score(1.5 - i as f32 * 0.001)
            })
            .collect()
    }
}

//! ">htop": roda um comando no terminal (que fica aberto no fim pra ver a saída).

use crate::engine::{Provider, Query};
use crate::model::{Effect, Item};

pub struct Comando;

impl Provider for Comando {
    fn category(&self) -> &str {
        "Comando"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let Some(cmd) = q.text.trim().strip_prefix('>').map(str::trim).filter(|c| !c.is_empty()) else {
            return vec![];
        };
        vec![
            Item::new(format!("Rodar: {cmd}"), Effect::Exec { cmd: vec!["sh".into(), "-c".into(), cmd.into()], terminal: true, cwd: None, hold: true, tab: None })
                .subtitle("No terminal, que fica aberto no fim  ·  Ctrl+Enter: em segundo plano")
                .icon("utilities-terminal")
                .score(1.0)
                .action("Rodar em segundo plano (avisa quando terminar)", "system-run", Effect::Task { cmd: vec!["sh".into(), "-c".into(), cmd.into()], cwd: None, label: cmd.into() })
                .action("Copiar o comando", "edit-copy", Effect::Copy { text: cmd.into() }),
        ]
    }
}

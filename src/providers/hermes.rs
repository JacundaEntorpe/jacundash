//! "hm <tarefa>": manda uma tarefa pro Hermes, o agente que mexe no PC. Enter abre no
//! terminal (dá pra acompanhar e aprovar); ou roda em segundo plano com a resposta numa notificação.

use crate::config::Compartilhada;
use crate::engine::{self, Provider, Query};
use crate::model::{Effect, Item};
use crate::sistema;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Hermes {
    pub cfg: Compartilhada,
    respostas: Arc<Mutex<HashMap<String, String>>>,
    rodando: Arc<Mutex<HashSet<String>>>,
}

impl Hermes {
    pub fn new(cfg: Compartilhada) -> Hermes {
        Hermes { cfg, respostas: Arc::default(), rodando: Arc::default() }
    }
}

impl Provider for Hermes {
    fn category(&self) -> &str {
        "Hermes"
    }
    fn keyword(&self) -> &str {
        "hm"
    }
    fn needs_keyword(&self) -> bool {
        true
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let comando = self.cfg.read().unwrap().hermes.comando.clone();
        if !sistema::tem(&comando) {
            return vec![Item::new("O Hermes não está instalado", Effect::None).subtitle(format!("Não achei o comando \"{comando}\" (dá pra mudar nas configurações)")).icon("dialog-warning").score(1.0)];
        }
        let tarefa = q.text.trim().to_string();
        if tarefa.is_empty() {
            return vec![Item::new("Mande uma tarefa pro Hermes", Effect::None).subtitle("hm <tarefa>  ·  ex.: hm organiza minha pasta Downloads por tipo").icon("dialog-scripts").score(1.0)];
        }
        if let Some(r) = self.respostas.lock().unwrap().get(&tarefa) {
            let mut item = Item::new(r.clone(), Effect::Copy { text: r.clone() }).subtitle("Resposta do Hermes  ·  Enter copia").icon("dialog-scripts").score(1.0);
            item.multiline = true;
            return vec![item];
        }
        if self.rodando.lock().unwrap().contains(&tarefa) {
            return vec![Item::new("O Hermes está trabalhando nisso...", Effect::None).subtitle("A resposta chega numa notificação").icon("chronometer").score(1.0)];
        }
        vec![
            Item::new(format!("Hermes: {tarefa}"), Effect::terminal(&[&comando, "chat", "-q", &tarefa], None))
                .subtitle("Enter abre no terminal (dá pra acompanhar e aprovar)  ·  Ctrl+Enter roda em segundo plano")
                .icon("dialog-scripts")
                .score(1.0)
                .action("Rodar em segundo plano (resposta numa notificação)", "system-run", Effect::Callback { data: format!("fundo:{tarefa}") }),
        ]
    }
    fn callback(&self, data: &str) -> Vec<Effect> {
        let Some(tarefa) = data.strip_prefix("fundo:").map(String::from) else { return vec![] };
        let comando = self.cfg.read().unwrap().hermes.comando.clone();
        let (respostas, rodando) = (self.respostas.clone(), self.rodando.clone());
        rodando.lock().unwrap().insert(tarefa.clone());
        std::thread::spawn(move || {
            sistema::avisar("Hermes trabalhando...", &tarefa, "chronometer");
            let (ok, resposta) = match sistema::rodar(&[&comando, "-z", &tarefa], None, Duration::from_secs(900)) {
                Some(o) => {
                    let saida = String::from_utf8_lossy(if o.stdout.is_empty() { &o.stderr } else { &o.stdout }).trim().to_string();
                    let fim: String = saida.chars().rev().take(2000).collect::<Vec<_>>().into_iter().rev().collect();
                    (o.status.success(), fim)
                }
                None => (false, "Passou de 15 minutos, desisti de esperar.".to_string()),
            };
            rodando.lock().unwrap().remove(&tarefa);
            respostas.lock().unwrap().insert(tarefa, resposta.clone());
            engine::pedir_atualizacao();
            let resumo: String = resposta.chars().rev().take(600).collect::<Vec<_>>().into_iter().rev().collect();
            sistema::avisar(if ok { "Hermes terminou" } else { "Hermes deu erro" }, &resumo, if ok { "dialog-scripts" } else { "dialog-error" });
        });
        vec![]
    }
}

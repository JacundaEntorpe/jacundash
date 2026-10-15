//! "ia <pergunta>": pergunta pra IA local (Ollama). A resposta aparece ali mesmo, enquanto o
//! modelo escreve. "ia" sozinho mostra quanto de memória ela está ocupando e deixa liberar.

use crate::config::Compartilhada;
use crate::engine::{self, Provider, Query};
use crate::model::{Effect, Item};
use crate::sistema;
use serde_json::json;
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
struct Conversa {
    resposta: String,
    pronta: bool,
    erro: Option<String>,
    modelo: String,
}

pub struct Ia {
    pub cfg: Compartilhada,
    conversas: Arc<Mutex<HashMap<String, Conversa>>>,
}

const LIBERAR: &[&str] = &["liberar", "descarregar", "parar", "stop"];

/// Atalhos: "traduz ..." e "explica ..." trocam as instruções do modelo (com ou sem o "ia").
/// Devolve (o que aparece no item, as instruções).
fn modo(pergunta: &str, padrao: &str) -> (&'static str, String) {
    let p = crate::matcher::norm(pergunta);
    if ["traduz ", "traduza ", "traduzir ", "translate "].iter().any(|x| p.starts_with(x)) {
        return ("Traduzir", "Traduza o texto depois de \"traduz\" pro português do Brasil; se ele já estiver em português, traduza pro inglês. Responda só com a tradução, sem comentários.".into());
    }
    if ["explica ", "explique ", "explain "].iter().any(|x| p.starts_with(x)) {
        return ("Explicar", "Explique em português do Brasil, curto e direto, o que esse comando (ou trecho) faz, parte por parte, numa lista. Se tiver algo perigoso, avise.".into());
    }
    ("Perguntar", padrao.to_string())
}

fn agente(segundos: u64) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(segundos))).build().into()
}

fn base(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

/// [(modelo, GB)] do que está na memória agora.
fn carregados(url: &str) -> Vec<(String, f64)> {
    let Ok(mut r) = agente(1).get(format!("{}/api/ps", base(url))).call() else { return vec![] };
    let v: serde_json::Value = r.body_mut().read_json().unwrap_or_default();
    v.get("models")
        .and_then(|m| m.as_array())
        .into_iter()
        .flatten()
        .map(|m| (m["name"].as_str().unwrap_or("?").to_string(), m["size"].as_f64().unwrap_or(0.0) / 1073741824.0))
        .collect()
}

/// O modelo da config, ou o primeiro instalado.
fn modelo(cfg: &crate::config::Ia) -> Option<String> {
    if !cfg.modelo.is_empty() {
        return Some(cfg.modelo.clone());
    }
    let mut r = agente(2).get(format!("{}/api/tags", base(&cfg.url))).call().ok()?;
    let v: serde_json::Value = r.body_mut().read_json().ok()?;
    v.pointer("/models/0/name").and_then(|n| n.as_str()).map(String::from)
}

impl Ia {
    pub fn new(cfg: Compartilhada) -> Ia {
        Ia { cfg, conversas: Arc::default() }
    }

    fn perguntar(&self, pergunta: String) {
        let cfg = self.cfg.read().unwrap().ia.clone();
        let conversas = self.conversas.clone();
        conversas.lock().unwrap().insert(pergunta.clone(), Conversa::default());
        std::thread::spawn(move || {
            let atualizar = |f: &dyn Fn(&mut Conversa)| {
                if let Some(c) = conversas.lock().unwrap().get_mut(&pergunta) {
                    f(c);
                }
                engine::pedir_atualizacao();
            };
            let Some(nome) = modelo(&cfg) else {
                return atualizar(&|c| c.erro = Some(format!("Não achei o Ollama em {} ou ele não tem nenhum modelo. Está rodando? (sv ollama)", cfg.url)));
            };
            atualizar(&|c| c.modelo = nome.clone());
            let corpo = json!({
                "model": nome, "stream": true, "think": false, "keep_alive": cfg.manter_carregado,
                "messages": [{"role": "system", "content": modo(&pergunta, &cfg.instrucoes).1}, {"role": "user", "content": pergunta}],
            });
            let resposta = match agente(600).post(format!("{}/api/chat", base(&cfg.url))).send_json(&corpo) {
                Ok(r) => r,
                Err(e) => return atualizar(&|c| c.erro = Some(format!("A IA local não respondeu ({e}). O Ollama está rodando? (sv ollama)"))),
            };
            // Cada linha é um pedaço da resposta; redesenha no máximo a cada 60 ms
            let mut ultimo = Instant::now();
            for linha in std::io::BufReader::new(resposta.into_body().into_reader()).lines() {
                let Ok(linha) = linha else { break };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&linha) else { continue };
                if let Some(e) = v.get("error").and_then(|e| e.as_str()) {
                    return atualizar(&|c| c.erro = Some(e.to_string()));
                }
                let pedaco = v.pointer("/message/content").and_then(|t| t.as_str()).unwrap_or_default().to_string();
                if let Some(c) = conversas.lock().unwrap().get_mut(&pergunta) {
                    c.resposta.push_str(&pedaco);
                }
                if v.get("done").and_then(|d| d.as_bool()) == Some(true) {
                    break;
                }
                if ultimo.elapsed() > Duration::from_millis(60) {
                    engine::pedir_atualizacao();
                    ultimo = Instant::now();
                }
            }
            let texto = {
                let mut cs = conversas.lock().unwrap();
                let Some(c) = cs.get_mut(&pergunta) else { return };
                c.resposta = c.resposta.trim().to_string();
                c.pronta = true;
                c.resposta.clone()
            };
            engine::pedir_atualizacao();
            if cfg.copiar_resposta {
                engine::copiar(texto.clone());
            }
            // Fechou a janela enquanto esperava: a resposta vai numa notificação
            if !engine::VISIVEL.load(Ordering::Relaxed) {
                sistema::avisar(&pergunta, &texto, "dialog-messages");
            }
        });
    }

    fn item_liberar(&self, url: &str) -> Item {
        let modelos = carregados(url);
        if modelos.is_empty() {
            return Item::new("A IA não está ocupando memória", Effect::None).subtitle("Nenhum modelo carregado no Ollama").icon("edit-clear").score(0.9);
        }
        let lista: Vec<String> = modelos.iter().map(|(n, gb)| format!("{n} ({gb:.0} GB)")).collect();
        Item::new("Liberar a memória da IA", Effect::Callback { data: "liberar".into() })
            .subtitle(format!("{}  ·  tira da memória agora (bom antes de jogar)", lista.join("  ·  ")))
            .icon("edit-clear")
            .score(0.9)
    }
}

fn liberar(url: String) {
    let modelos = carregados(&url);
    for (nome, _) in &modelos {
        let corpo = json!({"model": nome, "keep_alive": 0});
        if let Err(e) = agente(30).post(format!("{}/api/generate", base(&url))).send_json(&corpo) {
            return sistema::avisar("Não deu pra liberar a IA", &e.to_string(), "dialog-error");
        }
    }
    let livre: f64 = modelos.iter().map(|(_, gb)| gb).sum();
    let nomes: Vec<&str> = modelos.iter().map(|(n, _)| n.as_str()).collect();
    let texto = if modelos.is_empty() { "Nada estava carregado".to_string() } else { format!("{}  ·  {livre:.0} GB livres", nomes.join(", ")) };
    sistema::avisar("Memória da IA liberada", &texto, "edit-clear");
}

impl Provider for Ia {
    fn category(&self) -> &str {
        "IA"
    }
    fn keyword(&self) -> &str {
        "ia"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let cfg = self.cfg.read().unwrap().ia.clone();
        let nome = if cfg.modelo.is_empty() { "modelo local".to_string() } else { cfg.modelo.clone() };
        let pergunta = q.text.trim().to_string();
        // Sem o "ia", só os atalhos "traduz ..." e "explica ..."
        if !q.with_keyword && (modo(&pergunta, "").0 == "Perguntar" || pergunta.split_whitespace().count() < 2) {
            return vec![];
        }
        if pergunta.is_empty() {
            return vec![
                Item::new(format!("Pergunte algo pro {nome}"), Effect::None)
                    .subtitle("ia <pergunta>  ·  Enter manda, a resposta aparece aqui")
                    .icon("dialog-messages")
                    .score(1.0),
                self.item_liberar(&cfg.url),
            ];
        }
        if q.norm.chars().count() >= 3 && LIBERAR.iter().any(|p| p.starts_with(q.norm.as_str())) {
            return vec![self.item_liberar(&cfg.url)];
        }
        let conversa = self.conversas.lock().unwrap().get(&pergunta).cloned();
        let Some(c) = conversa else {
            return vec![
                Item::new(format!("{}: {pergunta}", modo(&pergunta, "").0), Effect::Callback { data: format!("perguntar:{pergunta}") })
                    .subtitle(format!("{nome}  ·  Enter manda, a resposta aparece aqui"))
                    .icon("dialog-messages")
                    .score(1.0),
            ];
        };
        if let Some(erro) = c.erro {
            return vec![Item::new(erro, Effect::None).subtitle("Enter de novo tenta outra vez").icon("dialog-error").score(1.0)];
        }
        let modelo = if c.modelo.is_empty() { nome } else { c.modelo };
        let (titulo, sub) = match (c.pronta, c.resposta.is_empty()) {
            (true, _) => (c.resposta.clone(), format!("{modelo}  ·  {}", if cfg.copiar_resposta { "já copiada  ·  Enter copia de novo" } else { "Enter copia" })),
            (false, true) => ("Pensando...".to_string(), format!("{modelo} respondendo")),
            (false, false) => (format!("{}▍", c.resposta), format!("{modelo} respondendo")),
        };
        let mut item = Item::new(titulo, if c.pronta { Effect::Copy { text: c.resposta.clone() } } else { Effect::None })
            .subtitle(sub)
            .icon("dialog-messages")
            .score(1.0);
        item.multiline = true;
        if !cfg.webui.is_empty() {
            let url = format!("{}/?q={}", cfg.webui.trim_end_matches('/'), crate::providers::web::codificar(&pergunta));
            item = item.action("Continuar no Open WebUI", "internet-web-browser", Effect::Open { uri: url });
        }
        vec![item.action("Perguntar de novo", "view-refresh", Effect::Callback { data: format!("perguntar:{pergunta}") })]
    }
    fn callback(&self, data: &str) -> Vec<Effect> {
        if let Some(p) = data.strip_prefix("perguntar:") {
            self.perguntar(p.to_string());
            return vec![Effect::None]; // a janela fica aberta pra mostrar a resposta
        }
        if data == "liberar" {
            let url = self.cfg.read().unwrap().ia.url.clone();
            std::thread::spawn(move || liberar(url));
        }
        vec![]
    }
}

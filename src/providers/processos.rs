//! "kill firefox": encerra programas pelo nome (só os seus). Os processos com o mesmo nome
//! aparecem juntos, com quanta memória usam. Enter encerra; Ctrl+Enter força (kill -9).

use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};
use crate::sistema;
use std::collections::BTreeMap;
use std::time::Duration;

pub struct Processos;

struct Proc {
    pid: u32,
    nome: String,
    cmd: String,
    memoria: u64,
}

fn meu_uid() -> Option<String> {
    std::fs::read_to_string("/proc/self/status").ok()?.lines().find_map(|l| l.strip_prefix("Uid:").map(|r| r.split_whitespace().next().unwrap_or("").to_string()))
}

fn listar() -> Vec<Proc> {
    let Some(uid) = meu_uid() else { return vec![] };
    let eu = std::process::id();
    let pagina = 4096u64;
    let mut v = Vec::new();
    for e in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else { continue };
        if pid == eu {
            continue;
        }
        let dir = e.path();
        let Ok(status) = std::fs::read_to_string(dir.join("status")) else { continue };
        let dono = status.lines().find_map(|l| l.strip_prefix("Uid:").map(|r| r.split_whitespace().next().unwrap_or("").to_string()));
        if dono.as_deref() != Some(uid.as_str()) {
            continue;
        }
        let nome = std::fs::read_to_string(dir.join("comm")).unwrap_or_default().trim().to_string();
        let cmd = std::fs::read(dir.join("cmdline")).map(|b| String::from_utf8_lossy(&b).replace('\0', " ").trim().to_string()).unwrap_or_default();
        if cmd.is_empty() {
            continue; // thread do kernel ou zumbi
        }
        let memoria = std::fs::read_to_string(dir.join("statm")).ok().and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok()).unwrap_or(0) * pagina;
        v.push(Proc { pid, nome, cmd, memoria });
    }
    v
}

fn tamanho(b: u64) -> String {
    if b >= 1 << 30 { format!("{:.1} GB", b as f64 / (1u64 << 30) as f64) } else { format!("{} MB", b >> 20) }
}

impl Provider for Processos {
    fn category(&self) -> &str {
        "Processos"
    }
    fn keyword(&self) -> &str {
        "kill"
    }
    fn needs_keyword(&self) -> bool {
        true
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        // Agrupa pelo nome: "firefox" com 12 processos vira uma linha só
        let mut grupos: BTreeMap<String, Vec<Proc>> = BTreeMap::new();
        for p in listar() {
            grupos.entry(p.nome.clone()).or_default().push(p);
        }
        let mut itens = Vec::new();
        for (nome, ps) in grupos {
            let nota = if q.norm.is_empty() { 0.5 } else { matcher::score(&q.norm, &nome, &ps.iter().map(|p| p.cmd.as_str()).collect::<Vec<_>>().join(" ")) };
            if nota == 0.0 {
                continue;
            }
            let memoria: u64 = ps.iter().map(|p| p.memoria).sum();
            let pids: Vec<String> = ps.iter().map(|p| p.pid.to_string()).collect();
            let sub = if ps.len() == 1 { format!("pid {}  ·  {}  ·  {}", pids[0], tamanho(memoria), ps[0].cmd) } else { format!("{} processos  ·  {}", ps.len(), tamanho(memoria)) };
            // Com a busca vazia ("kill"), os que mais gastam memória primeiro
            let nota = if q.norm.is_empty() { (memoria as f32 / (1u64 << 34) as f32).min(0.9) } else { nota };
            itens.push(
                Item::new(format!("Encerrar {nome}"), Effect::Callback { data: format!("TERM:{nome}:{}", pids.join(",")) })
                    .subtitle(sub)
                    .icon("process-stop")
                    .score(nota)
                    .action("Forçar (kill -9)", "process-stop", Effect::Callback { data: format!("KILL:{nome}:{}", pids.join(",")) }),
            );
        }
        itens
    }
    fn callback(&self, data: &str) -> Vec<Effect> {
        let mut partes = data.splitn(3, ':');
        let (Some(sinal), Some(nome), Some(pids)) = (partes.next(), partes.next(), partes.next()) else { return vec![] };
        let mut cmd = vec!["kill".to_string(), format!("-{sinal}")];
        cmd.extend(pids.split(',').map(String::from));
        let args: Vec<&str> = cmd.iter().map(String::as_str).collect();
        match sistema::rodar(&args, None, Duration::from_secs(3)) {
            Some(o) if o.status.success() => sistema::avisar(&format!("{nome} encerrado"), if sinal == "KILL" { "à força (kill -9)" } else { "" }, "process-stop"),
            Some(o) => sistema::avisar(&format!("Não deu pra encerrar {nome}"), String::from_utf8_lossy(&o.stderr).trim(), "dialog-error"),
            None => sistema::avisar(&format!("Não deu pra encerrar {nome}"), "o kill não respondeu", "dialog-error"),
        }
        vec![]
    }
}

//! "timer 10m chá", "timer 1h30 pizza", "timer 25" (minutos), "lembrete 18:30 reunião".
//! Quando acaba: notificação urgente e o som de alarme do sistema. "timer" sozinho lista os
//! ativos (Ctrl+Enter cancela). Ficam salvos: sobrevivem ao jacundash reiniciar.

use crate::config::pasta_estado;
use crate::engine::{self, Provider, Query};
use crate::model::{Effect, Item};
use crate::sistema;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize)]
struct Timer {
    id: u64,
    /// Quando toca (segundos desde 1970).
    fim: u64,
    texto: String,
}

pub struct Timers {
    lista: Arc<Mutex<Vec<Timer>>>,
}

fn agora() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn arquivo() -> std::path::PathBuf {
    pasta_estado().join("timers.json")
}

fn salvar(lista: &[Timer]) {
    let _ = std::fs::create_dir_all(pasta_estado());
    let _ = std::fs::write(arquivo(), serde_json::to_string(lista).unwrap_or_default());
}

/// "10m", "1h30", "90s", "2h", "25" (minutos), "1h30m20s" -> segundos.
fn duracao(t: &str) -> Option<u64> {
    let t = t.trim().to_lowercase();
    if let Ok(min) = t.parse::<f64>() {
        return (min > 0.0).then_some((min * 60.0) as u64);
    }
    let mut total = 0u64;
    let mut numero = String::new();
    let mut ultima = 'm';
    for c in t.chars() {
        if c.is_ascii_digit() {
            numero.push(c);
            continue;
        }
        let n: u64 = numero.parse().ok()?;
        numero.clear();
        ultima = c;
        total += n * match c {
            'h' => 3600,
            'm' => 60,
            's' => 1,
            _ => return None,
        };
    }
    // "1h30": o número solto no fim vale a unidade de baixo
    if !numero.is_empty() {
        let n: u64 = numero.parse().ok()?;
        total += n * if ultima == 'h' { 60 } else { 1 };
    }
    (total > 0).then_some(total)
}

/// "18:30" -> segundos até lá (amanhã, se já passou).
fn ate_hora(t: &str) -> Option<u64> {
    let (h, m) = t.trim().split_once(':')?;
    let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return None;
    }
    let agora = gtk::glib::DateTime::now_local().ok()?;
    let mut alvo = gtk::glib::DateTime::from_local(agora.year(), agora.month(), agora.day_of_month(), h, m, 0.0).ok()?;
    if alvo.to_unix() <= agora.to_unix() {
        alvo = alvo.add_days(1).ok()?;
    }
    Some((alvo.to_unix() - agora.to_unix()) as u64)
}

fn hora_de(unix: u64) -> String {
    gtk::glib::DateTime::from_unix_local(unix as i64).ok().and_then(|d| d.format("%H:%M").ok()).map(|s| s.to_string()).unwrap_or_default()
}

/// Quanto falta, do jeito que se fala: "10 min", "1h30", "45 s", "2 min 30 s".
fn falta(s: u64) -> String {
    match (s / 3600, s / 60 % 60, s % 60) {
        (0, 0, seg) => format!("{seg} s"),
        (0, min, 0) => format!("{min} min"),
        (0, min, seg) => format!("{min} min {seg} s"),
        (h, 0, _) => format!("{h}h"),
        (h, min, _) => format!("{h}h{min:02}"),
    }
}

fn tocar(t: &Timer) {
    let _ = std::process::Command::new("notify-send").args(["-a", "Jacundash", "-u", "critical", "-i", "alarm-symbolic", &format!("⏰ {}", if t.texto.is_empty() { "Tempo!" } else { &t.texto }), "O timer do jacundash acabou"]).spawn();
    // O som de alarme do tema de sons, com o que tiver instalado
    let som = "/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga";
    for cmd in [vec!["canberra-gtk-play", "-i", "alarm-clock-elapsed"], vec!["pw-play", som], vec!["paplay", som]] {
        if sistema::tem(cmd[0]) && std::process::Command::new(cmd[0]).args(&cmd[1..]).spawn().is_ok() {
            break;
        }
    }
}

impl Timers {
    pub fn new() -> Timers {
        let lista: Vec<Timer> = std::fs::read_to_string(arquivo()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let lista = Arc::new(Mutex::new(lista));
        // Um relógio só, que olha os timers a cada segundo
        let l = lista.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            let agora = agora();
            let mut v = l.lock().unwrap();
            let (prontos, resto): (Vec<Timer>, Vec<Timer>) = v.drain(..).partition(|t| t.fim <= agora);
            *v = resto;
            if !prontos.is_empty() {
                salvar(&v);
                drop(v);
                prontos.iter().for_each(tocar);
                engine::pedir_atualizacao();
            }
        });
        Timers { lista }
    }
}

impl Provider for Timers {
    fn category(&self) -> &str {
        "Timer"
    }
    fn keyword(&self) -> &str {
        "timer"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        // "lembrete 18:30 x" também vale, sem ser a palavra-chave
        let texto = if q.with_keyword {
            q.text.trim().to_string()
        } else if let Some(r) = q.text.trim().strip_prefix("lembrete ") {
            r.trim().to_string()
        } else {
            return vec![];
        };
        if texto.is_empty() {
            let agora = agora();
            let ativos = self.lista.lock().unwrap().clone();
            if ativos.is_empty() {
                return vec![Item::new("Nenhum timer ativo", Effect::None).subtitle("timer 10m chá  ·  timer 1h30 pizza  ·  lembrete 18:30 reunião").icon("chronometer").score(1.0)];
            }
            return ativos
                .into_iter()
                .map(|t| {
                    Item::new(format!("{}  —  faltam {}", if t.texto.is_empty() { "Timer" } else { &t.texto }, falta(t.fim.saturating_sub(agora))), Effect::None)
                        .subtitle(format!("toca às {}  ·  Ctrl+Enter cancela", hora_de(t.fim)))
                        .icon("chronometer")
                        .score(1.0)
                        .action("Cancelar", "edit-delete", Effect::Callback { data: format!("cancelar:{}", t.id) })
                })
                .collect();
        }
        let (quando, resto) = texto.split_once(' ').unwrap_or((&texto, ""));
        let Some(segundos) = ate_hora(quando).or_else(|| duracao(quando)) else {
            return vec![Item::new("Não entendi o tempo", Effect::None).subtitle("Ex.: timer 10m chá  ·  timer 1h30  ·  timer 90s  ·  lembrete 18:30 reunião").icon("dialog-warning").score(1.0)];
        };
        let fim = agora() + segundos;
        let nome = if resto.trim().is_empty() { String::new() } else { format!(": {}", resto.trim()) };
        vec![
            Item::new(format!("Timer de {}{nome}", falta(segundos)), Effect::Callback { data: format!("criar:{segundos}:{}", resto.trim()) })
                .subtitle(format!("Toca às {}  ·  Enter liga", hora_de(fim)))
                .icon("chronometer")
                .score(1.0),
        ]
    }
    fn callback(&self, data: &str) -> Vec<Effect> {
        let mut l = self.lista.lock().unwrap();
        if let Some(r) = data.strip_prefix("criar:") {
            let (s, texto) = r.split_once(':').unwrap_or((r, ""));
            let Ok(s) = s.parse::<u64>() else { return vec![] };
            let id = l.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            let t = Timer { id, fim: agora() + s, texto: texto.to_string() };
            sistema::avisar("Timer ligado", &format!("{} toca às {}", if t.texto.is_empty() { "O timer" } else { &t.texto }, hora_de(t.fim)), "chronometer");
            l.push(t);
        } else if let Some(id) = data.strip_prefix("cancelar:").and_then(|i| i.parse::<u64>().ok()) {
            l.retain(|t| t.id != id);
            drop(l);
            engine::pedir_atualizacao();
            salvar(&self.lista.lock().unwrap());
            return vec![Effect::None]; // fica aberto, mostrando os que sobraram
        }
        salvar(&l);
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::duracao;

    #[test]
    fn tempos() {
        assert_eq!(duracao("10m"), Some(600));
        assert_eq!(duracao("1h30"), Some(5400));
        assert_eq!(duracao("90s"), Some(90));
        assert_eq!(duracao("25"), Some(1500));
        assert_eq!(duracao("1h30m20s"), Some(5420));
        assert_eq!(duracao("abc"), None);
    }
}

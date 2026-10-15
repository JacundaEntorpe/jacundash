//! Bloquear, suspender, reiniciar, desligar, sair da sessão (opção das configurações,
//! desligada de fábrica). No KDE, reiniciar/desligar/sair abrem a confirmação do próprio KDE.

use crate::config::Compartilhada;
use crate::engine::{Provider, Query};
use crate::krunner;
use crate::matcher;
use crate::model::{Effect, Item};
use gtk::{gio, glib};

pub struct Sistema {
    pub cfg: Compartilhada,
}

// (id, nome, explicação, ícone, palavras pra achar)
const COMANDOS: &[(&str, &str, &str, &str, &str)] = &[
    ("bloquear", "Bloquear a tela", "Pede a senha pra voltar", "system-lock-screen", "bloquear travar lock tela"),
    ("suspender", "Suspender", "Dorme e volta de onde parou", "system-suspend", "suspender dormir sleep suspend"),
    ("hibernar", "Hibernar", "Salva tudo no disco e desliga", "system-suspend-hibernate", "hibernar hibernate"),
    ("reiniciar", "Reiniciar", "Pede confirmação antes", "system-reboot", "reiniciar reboot restart"),
    ("desligar", "Desligar", "Pede confirmação antes", "system-shutdown", "desligar shutdown poweroff"),
    ("sair", "Sair da sessão", "Pede confirmação antes", "system-log-out", "sair logout encerrar sessao deslogar"),
];

fn exec(cmd: &[&str]) -> Effect {
    Effect::exec(cmd)
}

/// A confirmação do KDE (org.kde.LogoutPrompt). false se não tem.
fn confirmar_no_kde(metodo: &str) -> bool {
    if !krunner::existe("org.kde.LogoutPrompt") {
        return false;
    }
    gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).is_ok_and(|b| {
        b.call_sync(Some("org.kde.LogoutPrompt"), "/LogoutPrompt", "org.kde.LogoutPrompt", metodo, None, None::<&glib::VariantTy>, gio::DBusCallFlags::NONE, 2000, gio::Cancellable::NONE)
            .is_ok()
    })
}

impl Provider for Sistema {
    fn category(&self) -> &str {
        "Sistema"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        if q.norm.chars().count() < 3 || !self.cfg.read().unwrap().extras.sistema {
            return vec![];
        }
        COMANDOS
            .iter()
            .filter_map(|(id, nome, sub, icone, palavras)| {
                let nota = matcher::score(&q.norm, nome, palavras);
                (nota > 0.0).then(|| {
                    let efeito = match *id {
                        "bloquear" => exec(&["loginctl", "lock-session"]),
                        "suspender" => exec(&["systemctl", "suspend"]),
                        "hibernar" => exec(&["systemctl", "hibernate"]),
                        _ => Effect::Callback { data: id.to_string() },
                    };
                    Item::new(*nome, efeito).subtitle(*sub).icon(*icone).score(nota.max(0.85))
                })
            })
            .collect()
    }
    fn callback(&self, id: &str) -> Vec<Effect> {
        let (metodo, cmd): (&str, &[&str]) = match id {
            "reiniciar" => ("promptReboot", &["systemctl", "reboot"]),
            "desligar" => ("promptShutDown", &["systemctl", "poweroff"]),
            "sair" => ("promptLogout", &["loginctl", "terminate-session", ""]),
            _ => return vec![],
        };
        if confirmar_no_kde(metodo) {
            return vec![];
        }
        // Fora do KDE vai direto (é o que o Enter pediu)
        let mut cmd: Vec<String> = cmd.iter().map(|s| s.to_string()).collect();
        if cmd.last().is_some_and(|s| s.is_empty()) {
            cmd.pop();
            cmd.push(std::env::var("XDG_SESSION_ID").unwrap_or_default());
        }
        vec![Effect::Exec { cmd, terminal: false, cwd: None, hold: false, tab: None }]
    }
}

//! Ferramentas pra rodar coisas no sistema: comando com tempo limite, notificação, terminal.

use crate::config::home;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Roda e espera no máximo `limite`. None se não deu pra rodar ou demorou demais.
pub fn rodar(cmd: &[&str], cwd: Option<&str>, limite: Duration) -> Option<Output> {
    let mut c = Command::new(cmd.first()?);
    c.args(&cmd[1..]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    c.env("GIT_TERMINAL_PROMPT", "0"); // git nunca fica esperando senha
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    let mut filho = c.spawn().ok()?;
    let fim = Instant::now() + limite;
    loop {
        match filho.try_wait() {
            Ok(Some(_)) => return filho.wait_with_output().ok(),
            Ok(None) if Instant::now() < fim => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = filho.kill();
                let _ = filho.wait();
                return None;
            }
        }
    }
}

/// Saída de um comando como texto (vazio se falhou).
pub fn texto(cmd: &[&str], limite: Duration) -> String {
    rodar(cmd, None, limite).map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

pub fn tem(programa: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(programa).is_file()))
}

/// Notificação na área de notificações do desktop (via notify-send).
pub fn avisar(titulo: &str, texto: &str, icone: &str) {
    if !tem("notify-send") {
        eprintln!("jacundash: {titulo}: {texto}");
        return;
    }
    let _ = Command::new("notify-send").args(["-a", "Jacundash", "-i", icone, titulo, texto]).stdin(Stdio::null()).spawn();
}

/// Roda em segundo plano e avisa no fim: "git pull ok" ou "git pull falhou" com o erro.
pub fn tarefa(cmd: Vec<String>, cwd: Option<String>, label: String) {
    std::thread::spawn(move || {
        let args: Vec<&str> = cmd.iter().map(String::as_str).collect();
        match rodar(&args, cwd.as_deref(), Duration::from_secs(300)) {
            Some(o) if o.status.success() => {
                let saida = String::from_utf8_lossy(&o.stdout);
                avisar(&format!("{label}: ok"), saida.trim().lines().last().unwrap_or(""), "dialog-ok");
            }
            Some(o) => {
                let erro = String::from_utf8_lossy(&o.stderr);
                let erro: String = erro.trim().chars().rev().take(400).collect::<Vec<_>>().into_iter().rev().collect();
                avisar(&format!("{label}: falhou"), &erro, "dialog-error");
            }
            None => avisar(&format!("{label}: falhou"), "não rodou ou demorou demais", "dialog-error"),
        }
    });
}

/// "~/Projetos" -> "/home/aland/Projetos"
pub fn expandir(p: &str) -> PathBuf {
    match p.strip_prefix("~") {
        Some(resto) => home().join(resto.trim_start_matches('/')),
        None => PathBuf::from(p),
    }
}

/// "/home/aland/x" -> "~/x"
pub fn til(p: &str) -> String {
    let h = home();
    let h = h.to_string_lossy();
    match p.strip_prefix(h.as_ref()) {
        Some(r) if r.is_empty() || r.starts_with('/') => format!("~{r}"),
        _ => p.to_string(),
    }
}

pub fn achar_terminal() -> String {
    if let Ok(t) = std::env::var("TERMINAL") {
        return t;
    }
    for t in ["konsole", "kitty", "foot", "alacritty", "wezterm", "ghostty", "gnome-terminal", "ptyxis", "kgx", "xfce4-terminal", "xterm"] {
        if tem(t) {
            return t.into();
        }
    }
    "xterm".into()
}

fn nome_do_terminal(terminal: &str) -> String {
    let t = if terminal.trim().is_empty() { achar_terminal() } else { terminal.to_string() };
    let prog = t.split_whitespace().next().unwrap_or("").to_string();
    std::path::Path::new(&prog).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(prog)
}

/// Terminais que conseguem abrir uma aba na janela que já está aberta. Nos outros (foot,
/// alacritty, xterm...) a ação "aba nova" nem aparece.
pub fn tem_aba(terminal: &str) -> bool {
    matches!(nome_do_terminal(terminal).as_str(), "konsole" | "gnome-terminal" | "xfce4-terminal" | "ptyxis" | "kgx" | "wezterm" | "kitty")
}

/// Abre uma aba na pasta, na janela do terminal que já está aberta. false se não deu (aí
/// quem chamou abre uma janela nova).
pub fn aba_nova(terminal: &str, pasta: &str) -> bool {
    match nome_do_terminal(terminal).as_str() {
        // Cada janela do Konsole tem um nome próprio no D-Bus: o --new-tab não acha a aberta
        "konsole" => crate::konsole::nova_aba(pasta),
        // Só com o WezTerm aberto; senão o cli falha e vai janela nova
        "wezterm" => rodar(&["wezterm", "cli", "spawn", "--cwd", pasta], None, Duration::from_secs(3)).is_some_and(|o| o.status.success()),
        // Só com "allow_remote_control yes" na config do kitty
        "kitty" => rodar(&["kitty", "@", "launch", "--type=tab", &format!("--cwd={pasta}")], None, Duration::from_secs(3)).is_some_and(|o| o.status.success()),
        // gnome-terminal, xfce4-terminal, ptyxis e kgx: o no_terminal já põe a opção de aba
        _ => false,
    }
}

/// O comando completo pra abrir `args` num terminal. Cada terminal recebe pasta, "segurar"
/// (não fechar quando o comando termina) e aba nova de um jeito.
pub fn no_terminal(terminal: &str, args: &[String], cwd: Option<&str>, segurar: bool, aba: bool) -> Vec<String> {
    let terminal = if terminal.trim().is_empty() { achar_terminal() } else { terminal.to_string() };
    let mut t: Vec<String> = terminal.split_whitespace().map(String::from).collect();
    let nome = std::path::Path::new(&t[0]).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut args = args.to_vec();
    let mut segurar = segurar;
    if aba && matches!(nome.as_str(), "gnome-terminal" | "xfce4-terminal" | "ptyxis" | "kgx") {
        t.push("--tab".into());
    }
    // Quem não tem opção de segurar: o próprio comando espera um Enter no fim
    if segurar && !args.is_empty() && !matches!(nome.as_str(), "konsole" | "kitty" | "foot" | "alacritty" | "xterm") {
        let junto = args.iter().map(|a| shell_escape(a)).collect::<Vec<_>>().join(" ");
        args = vec!["sh".into(), "-c".into(), format!("{junto}; printf '\\n[Enter fecha]'; read _")];
        segurar = false;
    }
    let pasta = |flag: &str| cwd.map(|c| vec![flag.to_string(), c.to_string()]).unwrap_or_default();
    let pasta_igual = |flag: &str| cwd.map(|c| vec![format!("{flag}={c}")]).unwrap_or_default();
    let e = |args: &[String]| if args.is_empty() { vec![] } else { [vec!["-e".to_string()], args.to_vec()].concat() };
    let fim = |args: &[String]| if args.is_empty() { vec![] } else { [vec!["--".to_string()], args.to_vec()].concat() };
    let hold = |flag: &str| if segurar { vec![flag.to_string()] } else { vec![] };
    let resto: Vec<String> = match nome.as_str() {
        "konsole" => [pasta("--workdir"), hold("--hold"), e(&args)].concat(),
        "kitty" => [pasta("--directory"), hold("--hold"), args.clone()].concat(),
        "foot" => [pasta_igual("--working-directory"), hold("--hold"), args.clone()].concat(),
        "alacritty" => [pasta("--working-directory"), hold("--hold"), e(&args)].concat(),
        "wezterm" => [vec!["start".into()], pasta("--cwd"), fim(&args)].concat(),
        "ghostty" => [pasta_igual("--working-directory"), e(&args)].concat(),
        "ptyxis" => [pasta("-d"), fim(&args)].concat(),
        "gnome-terminal" | "kgx" | "xfce4-terminal" => [pasta_igual("--working-directory"), fim(&args)].concat(),
        "xterm" => [hold("-hold"), e(&args)].concat(),
        _ => e(&args),
    };
    t.extend(resto);
    t
}

fn shell_escape(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || "-_./=:@%+,".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminais() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(no_terminal("konsole", &[], Some("/x"), false, false), a(&["konsole", "--workdir", "/x"]));
        assert_eq!(no_terminal("ptyxis", &[], Some("/x"), false, true), a(&["ptyxis", "--tab", "-d", "/x"]));
        assert!(tem_aba("kitty") && tem_aba("/usr/bin/konsole") && !tem_aba("foot") && !tem_aba("alacritty"));
        assert_eq!(no_terminal("kitty", &a(&["ping", "1.1"]), None, true, false), a(&["kitty", "--hold", "ping", "1.1"]));
        assert_eq!(
            no_terminal("gnome-terminal", &a(&["ping", "a b"]), None, true, false),
            a(&["gnome-terminal", "--", "sh", "-c", "ping 'a b'; printf '\\n[Enter fecha]'; read _"])
        );
        assert_eq!(til(&format!("{}/Projetos", home().display())), "~/Projetos");
    }
}

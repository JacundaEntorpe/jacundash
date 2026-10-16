#!/bin/bash
# Instala o jacundash pro seu usuário (nada vai pra fora da sua home).
#
#   ./instalar.sh                        compila, instala, inicia com a sessão e liga a primeira
#                                        tecla livre (Alt+Space, Meta+Space, Ctrl+Space...)
#   ./instalar.sh --atalho Meta+Space    já liga essa tecla, sem perguntar
#   ./instalar.sh --atalho Alt+Space --assumir   liga mesmo se a tecla for de outro atalho
#                                        (o outro perde só essa tecla; desfaz com jacundash --atalho --desfazer)
#   ./instalar.sh --sem-tecla            não mexe na tecla de atalho
set -euo pipefail
AQUI="$(cd "$(dirname "$0")" && pwd)"
DADOS="${XDG_DATA_HOME:-$HOME/.local/share}"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}"
BIN="$HOME/.local/bin/jacundash"
TECLA=""; PERGUNTAR=1; ASSUMIR=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --atalho) TECLA="${2:?--atalho precisa da tecla, ex.: --atalho Meta+Space}"; shift ;;
    --sem-tecla) PERGUNTAR=0 ;;
    --assumir) ASSUMIR=1 ;;
    *) echo "Opção desconhecida: $1"; exit 1 ;;
  esac
  shift
done

# Nome de cada dependência no gerenciador de pacotes da distro:
# Rust, compilador C, pkg-config, GTK4 e gtk4-layer-shell
tem_pacote() { false; }
if command -v pacman >/dev/null; then
  INSTALAR="sudo pacman -S --needed"; PKGS=(rust gcc pkgconf gtk4 gtk4-layer-shell)
  tem_pacote() { pacman -Si "$1" >/dev/null 2>&1; }
elif command -v apt-get >/dev/null; then
  INSTALAR="sudo apt install"; PKGS=(cargo build-essential pkg-config libgtk-4-dev libgtk4-layer-shell0)
  tem_pacote() { apt-cache policy "$1" 2>/dev/null | grep -q 'Candidate: [^(]'; }
elif command -v dnf >/dev/null; then
  INSTALAR="sudo dnf install"; PKGS=(cargo gcc pkgconf-pkg-config gtk4-devel gtk4-layer-shell)
  tem_pacote() { dnf -q info "$1" >/dev/null 2>&1; }
elif command -v zypper >/dev/null; then
  INSTALAR="sudo zypper install"; PKGS=(cargo gcc pkg-config gtk4-devel libgtk4-layer-shell0)
  tem_pacote() { zypper -q se -x "$1" >/dev/null 2>&1; }
elif command -v xbps-install >/dev/null; then
  INSTALAR="sudo xbps-install"; PKGS=(cargo gcc pkg-config gtk4-devel gtk4-layer-shell)
  tem_pacote() { xbps-query -R "$1" >/dev/null 2>&1; }
elif command -v apk >/dev/null; then
  INSTALAR="sudo apk add"; PKGS=(cargo build-base pkgconf gtk4.0-dev gtk4-layer-shell)
  tem_pacote() { [[ -n "$(apk search -qe "$1" 2>/dev/null)" ]]; }
else
  INSTALAR=""; PKGS=(rust "um compilador C" pkg-config "gtk4 (pacote de desenvolvimento)" gtk4-layer-shell)
fi

# Junta tudo que falta antes de parar, pra dar um comando só que instala tudo
FALTA=(); PACOTES=()
command -v cargo >/dev/null      || { FALTA+=(Rust); PACOTES+=("${PKGS[0]}"); }
command -v cc >/dev/null         || { FALTA+=("compilador C"); PACOTES+=("${PKGS[1]}"); }
command -v pkg-config >/dev/null || { FALTA+=(pkg-config); PACOTES+=("${PKGS[2]}"); }
pkg-config --exists gtk4 2>/dev/null || { FALTA+=(GTK4); PACOTES+=("${PKGS[3]}"); }

# Sem o gtk4-layer-shell ainda funciona, só que como janela comum (não fica no topo da tela).
# Ele é carregado na hora de abrir, então dá pra instalar depois sem recompilar.
# Só sugere instalar se a distro tiver o pacote.
tem_camada() {
  { ldconfig -p 2>/dev/null || /sbin/ldconfig -p 2>/dev/null; } | grep -q 'libgtk4-layer-shell\.so\.0' ||
    compgen -G '/usr/lib*/libgtk4-layer-shell.so.0' >/dev/null || compgen -G '/usr/lib/*/libgtk4-layer-shell.so.0' >/dev/null
}
CAMADA=""
SEM_CAMADA=0
if ! tem_camada; then
  SEM_CAMADA=1
  tem_pacote "${PKGS[4]}" && CAMADA="${PKGS[4]}"
fi

if [[ ${#FALTA[@]} -gt 0 ]]; then
  echo "Falta: $(IFS=,; echo "${FALTA[*]}" | sed 's/,/, /g')."
  [[ -n "$CAMADA" ]] && PACOTES+=("$CAMADA")
  if [[ -n "$INSTALAR" ]]; then
    echo "Instale com:  $INSTALAR ${PACOTES[*]}"
  else
    echo "Instale pelo gerenciador de pacotes da sua distro: $(IFS=,; echo "${PACOTES[*]}" | sed 's/,/, /g')"
  fi
  [[ -n "$CAMADA" ]] && echo "  ($CAMADA é o que deixa a janela no topo da tela; dá pra ficar sem)"
  [[ " ${FALTA[*]} " == *" Rust "* ]] && echo "  (se o Rust da distro for velho demais pra compilar: https://rustup.rs)"
  exit 1
fi

if [[ $SEM_CAMADA == 1 ]]; then
  if [[ -n "$CAMADA" ]]; then
    echo "Aviso: sem o gtk4-layer-shell a janela não fica no topo da tela. Pra ter: $INSTALAR $CAMADA"
    echo "  (vale no próximo início da sessão, ou na hora com: jacundash --sair && jacundash --fundo)"
  else
    echo "Aviso: sem o gtk4-layer-shell a janela não fica no topo da tela (a distro não tem o pacote, vai como janela comum)."
  fi
fi
echo "Compilando..."
cargo build --release --quiet --manifest-path "$AQUI/Cargo.toml"

# Se já tinha um rodando, sai pra o novo entrar no lugar
"$BIN" --sair 2>/dev/null || true
install -Dm755 "$AQUI/target/release/jacundash" "$BIN"

# Aparece no menu de apps; o KDE também usa esse arquivo pra abrir pela tecla de atalho
mkdir -p "$DADOS/applications"
cat > "$DADOS/applications/jacundash.desktop" <<FIM
[Desktop Entry]
Type=Application
Name=Jacundash
Comment=Lançador com categorias e plugins
Exec=$BIN
Icon=system-search
Terminal=false
Categories=Utility;
StartupNotify=false
X-KDE-StartupNotify=false
FIM

# Inicia escondido junto com a sessão, pra abrir na hora (KDE, GNOME, Xfce... leem esta pasta)
mkdir -p "$CONF/autostart"
cat > "$CONF/autostart/jacundash.desktop" <<FIM
[Desktop Entry]
Type=Application
Name=Jacundash
Exec=$BIN --fundo
Icon=system-search
Terminal=false
NoDisplay=true
X-GNOME-Autostart-enabled=true
FIM
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true

# Konsole: perfil que roda o comando nas abas que o jacundash abre (o Konsole não deixa outro
# programa digitar numa aba). Vale nos Konsoles abertos depois daqui.
if command -v konsole >/dev/null; then
  install -Dm755 "$AQUI/konsole/aba.sh" "$DADOS/jacundash/aba.sh"
  mkdir -p "$DADOS/konsole"
  printf '[General]\nName=Jacundash\nParent=FALLBACK/\nCommand=%s\n' "$DADOS/jacundash/aba.sh" > "$DADOS/konsole/Jacundash.profile"
fi

# Atalhos de busca próprios (aur:, bp:, ghme:...), no formato dos do KDE
mkdir -p "$CONF/jacundash/atalhos-web"
usuario="$(git config --global user.name 2>/dev/null || echo "$USER")"
for f in "$AQUI"/atalhos-web/*.desktop; do
  sed "s|@GITHUB_USER@|$usuario|" "$f" > "$CONF/jacundash/atalhos-web/$(basename "$f")"
done

setsid "$BIN" --fundo >/dev/null 2>&1 < /dev/null &

# Tecla de atalho. Automático: a primeira livre de Alt+Space, Meta+Space, Ctrl+Space...
# Nunca tira a tecla de outro atalho (o KRunner, por exemplo); se estiver ocupada, passa pra próxima.
INTERATIVO=0; [[ -t 0 ]] && INTERATIVO=1

# Se a tecla for de outro atalho: com --assumir passa pro jacundash; no terminal, pergunta
tentar() {
  local erro codigo=0
  erro="$("$BIN" --atalho "$1" 2>&1)" || codigo=$?
  [[ $codigo == 0 ]] && { echo "$erro"; return 0; }
  if [[ $codigo == 2 && $ASSUMIR == 1 ]]; then
    "$BIN" --atalho "$1" --assumir; return
  fi
  echo "${erro%% Pra passar*}"
  if [[ $codigo == 2 && $INTERATIVO == 1 ]]; then
    read -rp "Passar $1 pro jacundash? O outro atalho perde só essa tecla, e dá pra desfazer depois. [s/N] " r
    [[ $r == [sS]* ]] && { "$BIN" --atalho "$1" --assumir; return; }
  fi
  return 1
}

automatico() {
  for t in Alt+Space Meta+Space Ctrl+Space Meta+J Ctrl+Alt+Space Meta+F1; do
    "$BIN" --atalho "$t" 2>/dev/null && return 0
  done
  echo "Nenhuma das teclas de sempre está livre. Ligue uma com: jacundash --atalho <tecla>"
}

if [[ -n "$TECLA" ]]; then
  tentar "$TECLA" || echo "Não liguei $TECLA."
elif [[ $PERGUNTAR == 1 ]] && ! "$BIN" --atalho >/dev/null 2>&1; then
  if [[ $INTERATIVO == 1 ]]; then
    read -rp "Tecla pro jacundash (Enter = a primeira livre: Alt+Space, Meta+Space...; n = nenhuma): " TECLA
    case "$TECLA" in
      [nN]) ;;
      "") automatico ;;
      *) tentar "$TECLA" || automatico ;;
    esac
  else
    automatico
  fi
fi

echo
if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
  echo "No Hyprland, coloque no hyprland.conf:  exec-once = $BIN --fundo"
elif [[ -n "${SWAYSOCK:-}" ]]; then
  echo "No Sway, coloque na config:  exec $BIN --fundo"
fi
echo "Pronto. Tecla: $("$BIN" --atalho 2>/dev/null || echo "nenhuma (jacundash --atalho Meta+Space)")"
echo "Configurações: abra o jacundash e aperte Ctrl+, (ou busque \"config\")."
case ":$PATH:" in *":$HOME/.local/bin:"*) ;; *) echo "Aviso: ~/.local/bin não está no PATH." ;; esac

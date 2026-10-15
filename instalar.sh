#!/bin/bash
# Instala o jacundash pro seu usuário (nada vai pra fora da sua home).
#
#   ./instalar.sh                        compila, instala, inicia com a sessão e liga a primeira
#                                        tecla livre (Alt+Space, Meta+Space, Ctrl+Space...)
#   ./instalar.sh --atalho Meta+Space    já liga essa tecla, sem perguntar
#   ./instalar.sh --sem-tecla            não mexe na tecla de atalho
set -euo pipefail
AQUI="$(cd "$(dirname "$0")" && pwd)"
DADOS="${XDG_DATA_HOME:-$HOME/.local/share}"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}"
BIN="$HOME/.local/bin/jacundash"
TECLA=""; PERGUNTAR=1
while [[ $# -gt 0 ]]; do
  case "$1" in
    --atalho) TECLA="${2:?--atalho precisa da tecla, ex.: --atalho Meta+Space}"; shift ;;
    --sem-tecla) PERGUNTAR=0 ;;
    *) echo "Opção desconhecida: $1"; exit 1 ;;
  esac
  shift
done

command -v cargo >/dev/null || { echo "Falta o Rust: sudo pacman -S rust (ou https://rustup.rs)"; exit 1; }
pkg-config --exists gtk4 || { echo "Falta o GTK4: sudo pacman -S gtk4"; exit 1; }

# Sem o gtk4-layer-shell ainda funciona, só que como janela comum (não fica no topo da tela)
RECURSOS=()
if ! pkg-config --exists gtk4-layer-shell-0; then
  echo "Aviso: sem o gtk4-layer-shell a janela não fica no topo da tela (sudo pacman -S gtk4-layer-shell)."
  RECURSOS=(--no-default-features)
fi
echo "Compilando..."
cargo build --release --quiet --manifest-path "$AQUI/Cargo.toml" "${RECURSOS[@]}"

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

tentar() {
  "$BIN" --atalho "$1"
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

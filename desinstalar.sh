#!/bin/bash
# Tira o jacundash. A config (~/.config/jacundash) fica, a não ser com --tudo.
set -uo pipefail
DADOS="${XDG_DATA_HOME:-$HOME/.local/share}"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}"
BIN="$HOME/.local/bin/jacundash"

"$BIN" --sair 2>/dev/null
# Solta a tecla no KDE
if command -v busctl >/dev/null && busctl --user status org.kde.kglobalaccel >/dev/null 2>&1; then
  busctl --user call org.kde.kglobalaccel /kglobalaccel org.kde.KGlobalAccel unregister ss jacundash.desktop _launch >/dev/null 2>&1
fi
rm -f "$BIN" "$DADOS/applications/jacundash.desktop" "$CONF/autostart/jacundash.desktop"
if [[ "${1:-}" == "--tudo" ]]; then
  rm -rf "$CONF/jacundash" "${XDG_STATE_HOME:-$HOME/.local/state}/jacundash"
  echo "jacundash removido, junto com a config."
else
  echo "jacundash removido. A config ficou em $CONF/jacundash (./desinstalar.sh --tudo apaga)."
fi

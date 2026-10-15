#!/bin/sh
# Início das abas que o jacundash abre no Konsole: pega o comando mais antigo da fila (um
# arquivo por comando), roda e depois continua no shell normal (a aba fica aberta, como se você
# tivesse digitado o comando).
fila="${XDG_RUNTIME_DIR:-/tmp}/jacundash-abas"
for f in $(ls "$fila" 2>/dev/null | sort); do
  # mv é atômico: se duas abas abrirem juntas, cada uma fica com um comando
  if mv "$fila/$f" "$fila/.rodando-$$" 2>/dev/null; then
    cmd=$(cat "$fila/.rodando-$$")
    rm -f "$fila/.rodando-$$"
    sh -c "$cmd"
    break
  fi
done
exec "${SHELL:-/bin/sh}"

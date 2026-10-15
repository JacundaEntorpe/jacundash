#!/usr/bin/env python3
"""Plugin de exemplo do jacundash: lista os Host do ~/.ssh/config.

O jacundash abre este programa uma vez e manda uma mensagem JSON por linha:
  {"id": 1, "type": "query", "text": "vps", "keyword": false}
e espera a resposta com o mesmo id:
  {"id": 1, "items": [{"title": "...", "effect": {...}}]}
"""
import json
import sys
from pathlib import Path


def hosts():
    try:
        linhas = (Path.home() / ".ssh/config").read_text().splitlines()
    except OSError:
        return []
    achados = []
    for linha in linhas:
        partes = linha.split()
        if len(partes) >= 2 and partes[0].lower() == "host":
            achados += [h for h in partes[1:] if "*" not in h and "?" not in h]
    return achados


def buscar(texto, com_palavra):
    texto = texto.lower().strip()
    itens = []
    for h in hosts():
        if texto and texto not in h.lower():
            continue
        itens.append({
            "title": h,
            "subtitle": f"ssh {h}",
            "icon": "network-server",
            "score": 0.9 if h.lower().startswith(texto) else 0.6,
            "effect": {"type": "exec", "cmd": ["ssh", h], "terminal": True},
            "actions": [
                {"label": "Copiar o comando", "icon": "edit-copy",
                 "effect": {"type": "copy", "text": f"ssh {h}"}},
            ],
        })
    return itens


for linha in sys.stdin:
    msg = json.loads(linha)
    if msg["type"] == "query":
        resposta = {"id": msg["id"], "items": buscar(msg["text"], msg["keyword"])}
    else:  # "callback": este plugin não usa
        resposta = {"id": msg["id"], "effects": []}
    print(json.dumps(resposta), flush=True)

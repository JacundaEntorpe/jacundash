# Plugins do jacundash

Um plugin é um programa qualquer (Python, shell, Rust, Node...) que o jacundash abre uma vez e mantém aberto. A cada letra digitada, ele recebe a busca e devolve resultados. A conversa é por JSON, uma mensagem por linha, pela entrada e saída padrão.

## Instalar

Cada plugin é uma pasta em `~/.config/jacundash/plugins/<nome>/` com um `plugin.toml`:

```toml
nome = "SSH"          # categoria onde os resultados aparecem (a coluna da esquerda)
exec = "./ssh.py"     # comando; "./" é relativo à pasta do plugin, senão procura no PATH
args = []             # argumentos extras (opcional)
palavra = "ssh"       # "ssh vps" mostra só este plugin, sem limite de resultados (opcional)
so_com_palavra = false  # true: só responde quando a busca começa com a palavra
minimo = 2            # sem a palavra, só pergunta a partir de tantas letras
espera_ms = 1500      # quanto esperar a resposta; depois disso a busca segue sem ele
```

Depois de instalar ou mudar um plugin: `jacundash --sair` (o próximo `jacundash` já carrega). Tem um exemplo completo em [`exemplos/plugins/ssh`](exemplos/plugins/ssh).

## Mensagens

O jacundash manda, e o plugin responde **com o mesmo `id`**. As respostas podem vir fora de ordem. Se a resposta chegar depois de `espera_ms`, o jacundash ignora.

### Busca

```json
{"id": 7, "type": "query", "text": "vps", "keyword": false}
```

`keyword` é `true` quando a busca começou com a palavra do plugin. Aí `text` vem sem ela (`ssh vps` → `"vps"`) e pode vir vazio: é a hora de listar tudo.

Resposta:

```json
{"id": 7, "items": [
  {
    "title": "vps-oracle",
    "subtitle": "ssh vps-oracle",
    "icon": "network-server",
    "score": 0.9,
    "effect": {"type": "exec", "cmd": ["ssh", "vps-oracle"], "terminal": true},
    "actions": [
      {"label": "Copiar o comando", "icon": "edit-copy", "effect": {"type": "copy", "text": "ssh vps-oracle"}}
    ]
  }
]}
```

| Campo | |
|---|---|
| `title` | obrigatório |
| `subtitle` | texto apagado do lado do título |
| `icon` | nome de ícone do tema (`folder`, `network-server`), caminho de imagem ou GIcon |
| `score` | de 0 a 1, quanto bate com a busca. Ordena dentro da categoria, e a categoria inteira sobe se tiver o melhor resultado. Use 1.0 pra "é exatamente isso" e uns 0.5 pra "talvez" |
| `effect` | o que o Enter faz (obrigatório) |
| `actions` | as opções do Shift+Enter, cada uma com `label`, `icon` e `effect` |

### Efeitos

| `type` | Campos | O que faz |
|---|---|---|
| `launch` | `app` | abre um app pelo id do .desktop (`"org.kde.kate.desktop"`) |
| `open` | `uri` | abre arquivo, pasta ou URL no programa padrão (`"file:///home/..."`, `"https://..."`) |
| `copy` | `text` | copia pra área de transferência |
| `exec` | `cmd`, `terminal`, `cwd`, `hold`, `tab` | roda um comando; com `terminal: true`, dentro do terminal da config (`cmd` vazio só abre o terminal na pasta `cwd`). `hold: true` deixa o terminal aberto quando o comando termina; `tab: true` força uma aba nova na janela do terminal que já está aberta, `tab: false` força janela nova; sem `tab`, vale a configuração (de fábrica: aba primeiro) |
| `task` | `cmd`, `cwd`, `label` | roda em segundo plano e avisa numa notificação quando terminar: "`label`: ok" ou "`label`: falhou" com o erro |
| `query` | `text` | troca o texto da busca e deixa a janela aberta (pra entrar num submenu, por exemplo) |
| `callback` | `data` | devolve pro plugin decidir (veja abaixo) |
| `none` | | não faz nada (linha só de informação) |

Todos menos `query` e `none` fecham a janela.

### Callback

Quando o que fazer depende do plugin (rodar um `git pull` com aviso no fim, por exemplo), use `{"type": "callback", "data": "pull:/home/x/repo"}`. Quando a pessoa escolher, o plugin recebe:

```json
{"id": 9, "type": "callback", "data": "pull:/home/x/repo"}
```

e responde com os efeitos que quer que o jacundash faça. Uma lista vazia fecha a janela (o plugin fez tudo sozinho); pra deixar aberta, responda `[{"type": "none"}]`:

```json
{"id": 9, "effects": [{"type": "copy", "text": "feito"}]}
```

Responda logo e faça o trabalho demorado em segundo plano: a resposta também tem o limite de `espera_ms`.

## Dicas

- Dê `flush` depois de cada resposta (em Python: `print(..., flush=True)`), senão ela fica presa no buffer.
- O que o plugin escreve no `stderr` aparece no log do jacundash: é o lugar pra mensagens de erro.
- O plugin fica aberto: guarde em memória o que for caro de calcular (lista de repositórios, por exemplo) e atualize de tempos em tempos, em vez de recalcular a cada letra.
- Se o plugin morrer, o jacundash abre ele de novo na próxima busca.

# jacundash

Lançador rápido pra qualquer Linux, com resultados separados por categoria (igual o KRunner) e plugins em qualquer linguagem. Feito em Rust com GTK4.

Fica rodando escondido com a janela pronta, então abre na hora. A busca roda a cada letra, em paralelo, e cada categoria aparece assim que fica pronta.

```
┌──────────────────────────────────────────────────┐
│ 🔍 k                                             │
│  Aplicativos  Kate      Editor de textos         │
│               Krita     Programa de desenho      │
│               Konsole   Terminal                 │
│ ──────────────────────────────────────────────── │
│          Web  Pesquisar "k" na web               │
└──────────────────────────────────────────────────┘
```

## O que já faz

| Digite | O que aparece |
|---|---|
| `help`, `ajuda`, `?` | Tudo o que dá pra digitar no jacundash (palavras-chave, prefixos, teclas). Enter já começa a busca com aquilo |
| `kat` | Apps instalados (os mesmos do menu do desktop). Os que você mais abre sobem |
| `2^10*3`, `15% de 80`, `sqrt(2)`, `=1/3` | Calculadora. Enter copia o resultado |
| `10 usd em brl`, `5 km em milhas`, `100 f em c`, `2 gb em mb` | Conversor de moedas (cotação do dia) e unidades: comprimento, peso, dados, tempo, volume, velocidade, área, temperatura. Enter copia |
| `>htop`, `>git pu` | Roda o comando no terminal (que fica aberto no fim); embaixo, os comandos parecidos que você já rodou (pelo [Atuin](https://atuin.sh) se tiver, senão pelo histórico do bash, zsh ou fish). Shift+Enter: copiar ou rodar em segundo plano |
| `:fogo`, `emoji fogo` | Emojis pelo nome em português. Enter copia |
| `cb`, `cb senha` | O que você copiou antes (Klipper no KDE, cliphist no Hyprland/Sway). Enter copia de novo |
| `desligar`, `bloquear`... | Comandos do sistema: bloquear, suspender, hibernar, reiniciar, desligar, sair. Reiniciar, desligar e sair pedem confirmação no KDE. **Desligado de fábrica** (Configurações → Extras) |
| `github` | Abas abertas e histórico do navegador (KDE, com a extensão Plasma Integration). Dá pra desligar em Configurações → Extras |
| `disc` | Janelas abertas (Enter vai pra janela; Shift+Enter: fechar, minimizar, maximizar, mandar pra próxima área de trabalho). `ja` lista todas |
| `relatorio` (3+ letras) | Arquivos e pastas com a busca no nome, separados em Pastas, Imagens, Documentos... `ar texto` mostra só arquivos e procura no conteúdo também |
| `poke`, `vs poke` | Repositórios git (e as pastas abertas no VS Code), com a branch. Enter abre no editor; Shift+Enter: terminal (janela ou aba nova), Claude Code, gerenciador de arquivos, remote no navegador, `git pull`, copiar o caminho |
| `sprites`, `pa sprites` | Pastas comuns (sem git) das mesmas pastas, com a pasta de cima do lado |
| `gs` | Repositórios com alteração, commit sem push ou atrás do remote; `git pull`/`git push` no Shift+Enter e "Buscar atualizações de todos" (git fetch) no fim |
| `dk`, `ani-mal` | Containers Docker. Enter: logs (ou inicia, se estiver parado); Shift+Enter: shell, reiniciar, parar |
| `sv`, `sv docker` | Serviços do systemd (sistema e usuário). `sv` sozinho mostra os favoritos e os que falharam. Enter: status; Shift+Enter: logs ao vivo, reiniciar, parar, iniciar |
| `ts`, `vps` | Aparelhos do Tailscale, online primeiro. Enter copia o IP; Shift+Enter: SSH, copiar o nome, navegador, ping |
| `ia qual comando lista portas abertas?` | Pergunta pra IA local (Ollama). Enter manda e a resposta aparece ali mesmo, enquanto o modelo escreve; quando termina, já vai pra área de transferência. `ia` sozinho mostra quanto de memória ela ocupa e deixa liberar (bom antes de jogar) |
| `hm organiza meus Downloads` | Manda a tarefa pro Hermes (o agente que mexe no PC). Enter abre no terminal pra acompanhar e aprovar; Shift+Enter roda em segundo plano, com a resposta numa notificação |
| `traduz bom dia`, `explica tar -xzvf` | A IA local traduz (pro português, ou pro inglês se já estiver em português) ou explica o comando parte por parte |
| `kill firefox` | Encerra o programa (todos os processos com esse nome, só os seus). Shift+Enter força (kill -9). `kill` sozinho lista os que mais gastam memória |
| `timer 10m chá`, `timer 1h30`, `lembrete 18:30 reunião` | Timer e lembrete: notificação e o som de alarme quando acaba. `timer` sozinho lista os ativos (Shift+Enter cancela). Sobrevivem ao jacundash reiniciar |
| `email`, `tx` | Textos prontos (Configurações → Textos prontos): o nome acha, o Enter copia |
| `config` | Configurações do Jacundash (ou **Ctrl+,**) |
| `gh:react`, `yt lofi`, `protondb elden ring` | Atalhos de busca da web (os do KDE e os seus) |
| qualquer texto | "Pesquisar na web" no fim da lista |
| `app`, `vs`, `ssh`... + espaço | Só aquela categoria, sem limite de resultados. A palavra vira uma etiqueta do lado do campo (Backspace com o campo vazio tira) |

Reiniciar, parar, `git pull` e afins rodam em segundo plano e avisam numa notificação quando terminam.

**Teclas:** setas ou Tab andam, Enter abre, **Shift+Enter** mostra as outras ações do item, Esc fecha (ou fecha as ações). Clicar fora ou trocar de janela também fecha.

**Mouse:** passando o mouse numa linha (ou na selecionada) aparecem botões com as ações principais e um "⋮" com todas; clique direito também abre as ações.

**Prévia:** ao lado da lista, a imagem, o começo do texto ou o README (ou o conteúdo) da pasta do resultado selecionado. Dá pra desligar em Configurações → Extras.

**Recentes:** com a busca vazia, o que você abriu por último. **Desligado de fábrica** (Configurações → Extras).

**Receita de projeto:** um `.jacundash.toml` na raiz do repositório diz o que abrir pra trabalhar nele; aí `vs projeto` → Shift+Enter → **Começar a trabalhar** abre tudo de uma vez. Sem receita, a mesma lista tem "Criar uma receita", que gera o arquivo comentado:

```toml
editor = true                         # abre no editor
terminais = ["", "npm run dev"]       # uma aba por item ("" = só o terminal na pasta)
comandos = ["docker compose up -d"]   # rodam por trás antes (avisam se falharem)
abrir = ["http://localhost:3000"]     # links e arquivos
```

**Aba nova do terminal:** todo comando de terminal (logs, SSH, `>comando`, status de serviço...) abre primeiro numa aba nova da janela do terminal que já está aberta; se o terminal não tiver abas ou não estiver aberto, abre janela. Dá pra desligar em Configurações → Geral, e "Abrir no terminal (janela nova)" sempre abre janela. No Konsole, o comando vai por um perfil "Jacundash" que o instalador cria (o Konsole não deixa outro programa digitar numa aba); ele só vale nos Konsoles abertos depois da instalação, e até lá o comando abre numa janela nova. Funciona no Konsole (pelo D-Bus, no Konsole que já está aberto), GNOME Terminal, Xfce Terminal, Ptyxis, GNOME Console, WezTerm (aberto) e kitty (com `allow_remote_control yes`). Nos terminais sem abas (foot, alacritty, xterm) a ação nem aparece.

**Ações personalizadas:** em Configurações → Ações dá pra esconder qualquer ação (o `git pull` dos repositórios, por exemplo) e escolher quais viram botão na linha, por categoria.

**Fixar e esconder:** todo resultado tem, no Shift+Enter, "Fixar no topo" (ele e a categoria dele sobem) e "Esconder este resultado". Pra desfazer: aba de configurações → Categorias.

## Instalar

Precisa de Rust, `gtk4` e, pra ficar no topo da tela no Wayland, `gtk4-layer-shell` (no Arch: `sudo pacman -S rust gtk4 gtk4-layer-shell`).

```bash
git clone <este repositório> ~/Projetos/jacundash
cd ~/Projetos/jacundash && ./instalar.sh
```

O instalador compila, coloca o `jacundash` em `~/.local/bin`, põe no menu de apps, deixa ele iniciar escondido junto com a sessão e liga a tecla de atalho: a que tu digitar, ou (só Enter) a primeira livre entre Alt+Space, Meta+Space, Ctrl+Space... Nunca tira a tecla de outro atalho, como o KRunner. Pra atualizar: `git pull` e `./instalar.sh` de novo. Pra tirar: `./desinstalar.sh` (`--tudo` apaga a config também).

Sem o `gtk4-layer-shell` (ou no X11) ele compila sem isso sozinho, e abre como uma janela comum, sem borda.

No Hyprland e no Sway, coloque na config `exec-once = jacundash --fundo` (Hyprland) ou `exec jacundash --fundo` (Sway), e a tecla com `jacundash --atalho Meta+Space`, que mostra a linha certa pra colar.

| Comando | |
|---|---|
| `jacundash` | abre, ou fecha se já estiver aberto |
| `jacundash "texto"` | abre já buscando o texto |
| `jacundash --config` | abre as configurações |
| `jacundash --atalho Meta+Space` | liga a tecla (KDE: na hora; Hyprland/Sway: mostra a linha da config). Sem a tecla, mostra a atual |
| `jacundash --enter` | aperta Enter na janela aberta (pra scripts: `jacundash "ia liberar" && sleep 1 && jacundash --enter`) |
| `jacundash --tecla Down` | aperta uma tecla na janela aberta (`Tab`, `shift+Return`, `Escape`...), pra scripts |
| `jacundash --fundo` | só deixa rodando em segundo plano |
| `jacundash --sair` | encerra (o próximo `jacundash` recarrega config, tema e plugins) |

## Onde funciona

| | KDE | Hyprland, Sway, Niri... | X11 | GNOME Wayland |
|---|---|---|---|---|
| Janela no topo, por cima de tudo | ✓ | ✓ | janela comum | janela comum (o GNOME não tem layer-shell) |
| Apps, calculadora, web, plugins | ✓ | ✓ | ✓ | ✓ |
| Janelas abertas | KWin | hyprctl, swaymsg, niri | `wmctrl` | — |
| Arquivos | Baloo (nome e conteúdo) | `plocate`/`locate`, ou `fd` | `plocate`/`locate`, ou `fd` | `plocate`/`locate`, ou `fd` |

## Configurar

A aba de configurações (**Ctrl+,** com o jacundash aberto, ou busque `config`) tem tudo: fechar ao perder o foco, busca da web, terminal, tamanho e posição, quais categorias aparecem, quantos resultados cada uma mostra e em que ordem, onde procurar repositórios, editor, serviços favoritos, usuário do SSH de cada aparelho do Tailscale e os plugins (ligar, desligar, abrir a pasta, recarregar). Cada mudança vale na hora.

Ela grava em `~/.config/jacundash/config.toml` mexendo só no que mudou (os seus comentários ficam). Dá pra editar na mão também: o jacundash percebe sozinho e aplica na hora, igual o `estilo.css`. Plugin ou atalho web novo faz ele reiniciar sozinho quando a janela fechar.

```toml
[aparencia]
largura = 640       # pixels
altura_max = 460    # a lista encolhe quando tem pouca coisa
topo = 0            # distância do topo da tela
icone = 22

[busca]
web = "https://duckduckgo.com/?q={}"
fechar_ao_perder_foco = true
terminal = "konsole"   # vazio = acha sozinho

[repos]
pastas = ["~/Projetos", "~/Documentos"]   # onde procurar repositórios (e as pastas do "pa")
profundidade = 4
editor = "code"
claude = "claude"

[servicos]
favoritos = ["docker", "tailscaled", "ollama"]   # o que aparece com "sv" sozinho

[tailscale.usuarios]
vps = "ubuntu"        # usuário do SSH por aparelho

[ia]
modelo = "gemma3:12b"      # vazio = o primeiro instalado no Ollama
webui = "http://localhost:8080"   # ação "Continuar no Open WebUI"
manter_carregado = "5m"

[categorias]
ordem = ["Calculadora", "Aplicativos", "SSH", "Web"]   # desempate entre categorias
limite = 6                                             # itens por categoria
limites = { Aplicativos = 8 }
esconder = []                                          # categorias desligadas
```

O instalador já põe uns atalhos de busca: `aur:`, `bp:`/`bulba:` (Bulbapedia), `sb:`/`serebii:`, `ot:`/`otland:`, `ghc:` (código no GitHub) e `ghme:` (seus repositórios). Os seus: arquivos `.desktop` no formato do KDE em `~/.config/jacundash/atalhos-web/` (`Keys=aur`, `Name=AUR`, `Query=https://aur.archlinux.org/packages?K=\{@}`). Os do KDE já são lidos sozinhos.

**Tema:** escuro (o padrão), claro ou do sistema (as cores do KDE, ou o claro/escuro e a cor de destaque do GNOME), em Configurações → Aparência. Pra ir além, `~/.config/jacundash/estilo.css` vale por cima de tudo. Só precisa do que mudar, por exemplo:

```css
@define-color jd_destaque #3daee9;
.caixa { font-size: 11pt; }
```

## Plugins

Qualquer programa vira plugin: ele recebe a busca em JSON e devolve os resultados. Veja o [PLUGINS.md](PLUGINS.md) e o exemplo em [`exemplos/plugins/ssh`](exemplos/plugins/ssh).

## Próximos passos

- Publicar (AUR, release com o binário pronto)

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
| `kat` | Apps instalados (os mesmos do menu do desktop). Os que você mais abre sobem |
| `2^10*3`, `15% de 80`, `sqrt(2)`, `=1/3` | Calculadora. Enter copia o resultado |
| `gh:react`, `yt lofi`, `protondb elden ring` | Atalhos de busca da web (os do KDE e os seus) |
| qualquer texto | "Pesquisar na web" no fim da lista |
| `app`, `ssh`... + espaço | Só aquela categoria, sem limite de resultados |

**Teclas:** setas ou Tab andam, Enter abre, **Shift+Enter** mostra as outras ações do item, Esc fecha (ou fecha as ações). Clicar fora ou trocar de janela também fecha.

## Instalar

Precisa de `gtk4` e, pra ficar no topo da tela no Wayland, `gtk4-layer-shell` (no Arch: `sudo pacman -S gtk4 gtk4-layer-shell`).

```bash
cargo build --release
install -Dm755 target/release/jacundash ~/.local/bin/jacundash
```

Sem o `gtk4-layer-shell` (ou no X11): `cargo build --release --no-default-features`. Aí ele abre como uma janela comum, sem borda.

Ligue o comando `jacundash` a uma tecla. Ele abre se estiver fechado e fecha se estiver aberto. Pra já ficar pronto quando a sessão começar, coloque `jacundash --fundo` pra iniciar junto (no KDE: Configurações do Sistema → Inicialização automática).

| Comando | |
|---|---|
| `jacundash` | abre, ou fecha se já estiver aberto |
| `jacundash "texto"` | abre já buscando o texto |
| `jacundash --fundo` | só deixa rodando em segundo plano |
| `jacundash --sair` | encerra (o próximo `jacundash` recarrega config, tema e plugins) |

## Onde funciona

| | KDE | Hyprland, Sway, Niri... | X11 | GNOME Wayland |
|---|---|---|---|---|
| Janela no topo, por cima de tudo | ✓ | ✓ | janela comum | janela comum (o GNOME não tem layer-shell) |
| Apps, calculadora, web, plugins | ✓ | ✓ | ✓ | ✓ |

## Configurar

`~/.config/jacundash/config.toml`, só com o que for mudar:

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

[categorias]
ordem = ["Calculadora", "Aplicativos", "SSH", "Web"]   # desempate entre categorias
limite = 6                                             # itens por categoria
limites = { Aplicativos = 8 }
esconder = []                                          # categorias desligadas
```

Atalhos de busca próprios: arquivos `.desktop` no formato do KDE em `~/.config/jacundash/atalhos-web/` (`Keys=aur`, `Name=AUR`, `Query=https://aur.archlinux.org/packages?K=\{@}`). Os do KDE já são lidos sozinhos.

**Tema:** `~/.config/jacundash/estilo.css` vale por cima do [tema padrão](src/estilo.css). Só precisa do que mudar, por exemplo:

```css
@define-color jd_destaque #3daee9;
.caixa { font-size: 11pt; }
```

## Plugins

Qualquer programa vira plugin: ele recebe a busca em JSON e devolve os resultados. Veja o [PLUGINS.md](PLUGINS.md) e o exemplo em [`exemplos/plugins/ssh`](exemplos/plugins/ssh).

## Próximos passos

- Aba de configurações dentro do próprio app (plugins, categorias, aparência, tecla)
- Janelas abertas e arquivos (com uma versão pra cada desktop)
- Repositórios, pastas, git, docker, serviços, Tailscale e IA (o que hoje está no krocket)
- Fixar e esconder itens, abrir com o mesmo atalho já configurado no desktop

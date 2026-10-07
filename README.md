# Sparkpad

**Sparkpad** é um bloco de notas nativo para macOS, inspirado na edição por blocos do Notion, com uma proposta mínima e leve. Escreva, organize páginas e subpáginas e conecte agentes pelo MCP. Seus dados ficam em SQLite local; a interface usa **Rust + GPUI** e a **empire-ui do Fennel Motion**, sem WebView.

O app aparece no Dock e na barra de menus. Fixar acima de outras janelas e ajustar a transparência são opções para quando você precisar de uma nota por perto.

## Rodar

Requisitos: macOS 12 ou superior, Rust com Cargo e as ferramentas de linha de comando do Xcode (`xcode-select --install`). O repositório inclui as dependências locais da interface; não é necessário baixar o projeto Fennel Motion.

```sh
cargo run --locked
```

Dependências incluídas no repositório e já configuradas em `Cargo.toml`:

- `vendor/empire-ui` — biblioteca de componentes do Fennel Motion, incluída com dependências explícitas para funcionar fora do workspace original.
- `vendor/gpui` — GPUI 0.2.2 com correção local na junção de trechos de fonte; preserva negrito/itálico nos inputs. Ver `vendor/gpui/PATCHES.md`.
- `vendor/gpui-component` — cópia do fork usado pelo Fennel para edição e seleção de texto, com correções locais de layout do Markdown em `ScrollArea`. A origem e os patches estão em `vendor/gpui-component/PATCHES.md`.

Para gerar o aplicativo:

```sh
./scripts/bundle.sh
open "dist/Sparkpad.app"
```

Use `./scripts/bundle.sh release` para uma build de distribuição otimizada. A assinatura local é ad hoc; o pacote não é notarizado para distribuição em outros Macs.

## Escrever e organizar

- Clique no ícone do Sparkpad na barra de menus para mostrar ou esconder.
- Arraste pela borda superior ou pelo texto **SparkPad** na sidebar para posicionar o painel; redimensione pelas bordas.
- Clique em um bloco para editar no lugar, mantendo títulos, negrito, itálico e o restante do documento renderizados. **⌘ Enter** esconde as ferramentas de edição.
- O menu **⋯** e o clique direito na nota oferecem as mesmas ações, incluindo **Excluir nota**, com confirmação que informa as subpáginas incluídas.
- Ao arrastar uma nota, as linhas mostram inserção acima/abaixo; o centro indica inserção como filha. Mova para a esquerda para sair de nível. Destinos fechados abrem após 500 ms, e a árvore rola automaticamente nas bordas.
- Arraste a borda direita da sidebar para ajustar a largura (180–420 px); ela é salva ao soltar.
- Sidebar e árvore expandem/recolhem com transições curtas e interrompíveis; popovers, toolbar e controles de hover aparecem suavemente. As animações pedem frames apenas enquanto estão em andamento.
- Abra ou recolha a sidebar pelo botão no canto superior esquerdo; **+** cria uma nota. O campo de título permite renomear.
- Ajuste a transparência pelo slider **Opacidade**. **A− / A+** ajustam o texto na leitura e na edição.
- **⌘ N** cria uma nota; **⌘ B** alterna a sidebar fora do texto, ou **⇧⌘ B** em qualquer lugar; **Esc** esconde o painel.
- No texto, **⌘ B / ⌘ I / ⌘ U / ⌘ E** formatam a seleção como negrito, itálico, sublinhado ou código. **⌘ Z / ⇧⌘ Z** desfazem/refazem alterações, inclusive operações entre blocos.
- Copiar e colar preserva títulos, listas, tarefas, citações, links, código e estilos inline por HTML no clipboard do macOS, para intercâmbio com editores como o Notion. Cópias entre páginas do Sparkpad também incluem Markdown para preservar o conteúdo. **⌘/Ctrl ⇧ V** cola apenas o texto, usando a formatação do destino.
- O botão de saída no rodapé encerra o app depois de salvar.

O conteúdo é salvo automaticamente após 350 ms sem digitar, e também ao trocar de nota, entrar em leitura, ocultar ou sair. Notas, seleção, opacidade, fonte, largura do conteúdo e posição da janela são persistidas. O seletor de fonte no footer oferece NV Legible Next (sans serif), Libron (serifada) e JetBrains Mono. A escolha é persistida e afeta apenas o título e o conteúdo da nota. As três famílias são embutidas com versões regular, negrito, itálico e negrito itálico. O botão de sol/lua no footer alterna entre os temas neutros claro e escuro, com preferência persistida. Os botões de ícone no footer abrem os sliders de opacidade e largura em popovers. O slider “Largura” ajusta a coluna de 40% a 100%, centralizando texto, título e ícone; a capa mantém a largura inteira. Markdown suporta títulos, negrito, itálico, listas, citações, tabelas e blocos de código pelo renderizador do `gpui-component`.

### Editor de blocos

O fim da nota oferece meia tela de espaço vazio rolável; clicar ali permite continuar escrevendo. O espaço acompanha a altura da janela.

**Enter** divide o bloco no cursor; **Shift Enter** insere uma quebra de linha. **Backspace** no início de um parágrafo junta com o bloco anterior; no início de título/lista/citação, converte em parágrafo; **Delete** no fim junta com o seguinte. Um Enter em um item vazio encerra a lista. As setas navegam entre blocos quando o cursor chega ao início/fim. Blocos de código mantêm Enter como nova linha.

Digite **/** no início de um parágrafo para abrir o dropdown de blocos. **↑ / ↓** navegam, **Enter** escolhe e **Esc** fecha sem apagar o texto. O menu tem ícones, atalhos, Scroll Area e mantém o foco no editor para filtrar pelo nome: texto, títulos 1–3, lista, lista numerada, checklist, citação, código ou divisor. O dropdown na toolbar mostra o tipo atual e permite converter um bloco existente. Os prefixos `# `, `## `, `### `, `- `, `1. `, `> ` e `[] ` convertem um parágrafo vazio ao digitar. Os controles de **adicionar** e **arrastar** aparecem apenas ao passar o mouse sobre o bloco, sobrepostos à esquerda e alinhados ao topo. **+** insere um novo bloco abaixo daquele bloco. Clique no ícone de **arrastar** para selecionar o bloco inteiro e abrir a toolbar de tipo, exclusão e desfazer/refazer; arraste por esse ícone para reordenar, com uma linha indicando o destino tanto acima quanto abaixo e nos espaços entre blocos. Clique no espaço vazio abaixo do último bloco para continuar escrevendo: um novo parágrafo é criado e recebe o cursor, ou o parágrafo final vazio é reutilizado. A seleção de texto é contínua e pode atravessar blocos ao clicar e arrastar, nos dois sentidos. **⌘A** ou **Ctrl+A** seleciona todo o conteúdo da nota. Copiar reúne os trechos selecionados; recortar, colar, digitar e apagar substituem o intervalo inteiro, preservando o texto e a formatação fora dele. As ferramentas de formatação aplicam o estilo a todos os trechos selecionados. Um novo clique inicia outra seleção; as setas recolhem a seleção para o início ou para o fim. Ao arrastar junto à borda da área de leitura, o documento rola para continuar a seleção.

A edição e a leitura usam o mesmo campo nativo por bloco: clicar só posiciona o cursor, sem trocar componentes, tamanho da fonte ou margens. O controle de arraste usa o SVG Grip Vertical de seis pontos fornecido pelo usuário. Os demais ícones da toolbar, do dropdown e dos controles são os SVGs oficiais do [Iconoir](https://iconoir.com/) já embutidos na empire-ui. A toolbar só aparece com texto selecionado (negrito, itálico, sublinhado, riscado e código) ou com o bloco selecionado pelo ícone de arraste (conversão de tipo). Com texto selecionado, fica centralizada acima do trecho na primeira linha visual selecionada, respeitando a largura do documento; com o bloco inteiro selecionado, fica acima do bloco. É ocultada durante o arraste; o menu `/` aparece abaixo dele, sem ocupar espaço no documento. Apenas posicionar o cursor não abre a toolbar. Sublinhado é persistido como HTML inline `<u>…</u>`, mantendo o documento em Markdown. Texto e títulos quebram linhas conforme a largura do painel. O título da nota tem o mesmo recuo lateral do texto dos parágrafos.

O modelo usa IDs estáveis, conteúdo de texto com intervalos de estilos e operações de divisão/junção. A edição usa o `InputState` nativo do fork do Fennel com estilos inline; não há WebView. O documento continua salvo como Markdown e mantém intactos os trechos que não foram modificados. Tabelas, imagens, listas aninhadas e outras construções avançadas são renderizadas normalmente e têm edição da fonte limitada ao respectivo bloco.

A lógica foi inspirada na consulta ao [gerenciador de blocos do Editor.js](https://github.com/codex-team/editor.js/blob/next/src/components/modules/blockManager.ts) (identidade, inserção, divisão e junção) e ao [menu de comandos do BlockNote](https://github.com/TypeCellOS/BlockNote/blob/main/packages/core/src/extensions/SuggestionMenu/getDefaultSlashMenuItems.ts) (conversão do bloco atual e seleção após inserir). A implementação Rust é própria.

Se um agente alterar a mesma nota durante uma edição local, a revisão impede a sobrescrita. O painel mantém seu texto e oferece **Preservar como nova nota**. Alterações externas aparecem no painel automaticamente quando não existe uma edição local pendente.

## MCP

O mesmo executável oferece um servidor **MCP via stdio**. O cliente inicia um processo separado, sem abrir a interface, que compartilha o SQLite com o painel. O app pode estar fechado para criar e editar notas; `show_panel` exige o app aberto.

Compile e registre o caminho absoluto do executável. Execute na raiz do checkout:

```sh
cargo build --locked
codex mcp add sparkpad -- "$PWD/target/debug/sparkpad" mcp
claude mcp add --scope user sparkpad -- "$PWD/target/debug/sparkpad" mcp
```

A conexão stdio do Codex é documentada em [MCP no Codex](https://developers.openai.com/codex/mcp). As sintaxes acima também foram verificadas nos comandos locais `codex mcp add --help` e `claude mcp add --help`.

Para outros clientes MCP, use:

```json
{
  "mcpServers": {
    "sparkpad": {
      "command": "/absolute/path/to/sparkpad/target/debug/sparkpad",
      "args": ["mcp"]
    }
  }
}
```

Também pode apontar para `dist/Sparkpad.app/Contents/MacOS/sparkpad`, que não depende da pasta `target` depois de empacotado.

| Ferramenta | Uso |
| --- | --- |
| `list_notes` | Lista notas com IDs, `parent_id`, texto e revisões |
| `list_sidebar_groups` | Lista os grupos da raiz e suas páginas |
| `create_sidebar_group` | Cria um agrupador com `title`, sem conteúdo ou subgrupos |
| `rename_sidebar_group` | Renomeia o agrupador por `id` e `title` |
| `delete_sidebar_group` | Exclui só o grupo; páginas e subpáginas são preservadas na raiz |
| `move_note_to_group` | Move a página e sua subárvore para `group_id`, ou remove do grupo com `null`; usa `expected_revision` |
| `get_note` | Lê título, Markdown e revisão |
| `create_note` | Cria nota com `title`, `markdown` e `parent_id` ou `group_id` opcional |
| `update_note` | Atualiza nota inteira com `expected_revision` |
| `patch_note` | Substitui um trecho único com `old_text`, `new_text` e `expected_revision` |
| `move_note` | Move uma nota e suas descendentes para `parent_id`; `null` devolve à raiz, com `expected_revision` |
| `delete_note` | Exclui com verificação de revisão; rejeita notas que ainda têm filhas |
| `select_note` | Seleciona a nota mostrada no painel |
| `show_panel` | Solicita a exibição do painel |
| `get_sidebar_tree` | Hierarquia completa e ordenada: raízes, grupos, páginas, IDs de filhas, ícones, capas e seleção |
| `reorder_sidebar` | Reordena todos os irmãos de uma raiz, página ou grupo; também reordena agrupadores |
| `get_note_presentation` | Lê ícone, capa e associação ao agrupador |
| `search_page_icons` | Pesquisa paginada no catálogo completo de emojis e Iconoir, com nomes em português/inglês |
| `set_note_icon` | Define emoji, ícone Iconoir ou imagem local; `value: null` remove |
| `list_cover_presets` | Lista IDs das cores e gradientes disponíveis |
| `set_note_cover` | Define capa de cor/gradiente ou imagem local; `value: null` remove |
| `get_note_blocks` | Lê blocos nativos com índice, tipo, texto, Markdown e revisão |
| `edit_note_block` | Insere, atualiza, remove ou move um bloco com `expected_revision` |
| `get_preferences` | Lê fonte, tema, opacidade, largura do conteúdo, tamanho do texto, sidebar e fixação |
| `update_preferences` | Atualiza as preferências fornecidas atomicamente e sincroniza com o app aberto |

O servidor se identifica como `sparkpad`, título **Sparkpad**, versão **0.2.0**, com **25 ferramentas**. Para instalações globais no Codex ou Kimi, configure o caminho absoluto de `dist/Sparkpad.app/Contents/MacOS/sparkpad`, com o argumento `mcp`. Depois de atualizar o executável, reconecte o MCP no cliente para recarregar a lista de ferramentas.

A árvore retorna páginas em formato plano (`pages`), com `parent_id`, `group_id` e `child_ids` ordenados. `root_ids` ordena as páginas sem agrupador e cada grupo contém `page_ids`. Isso preserva árvores profundas sem serialização recursiva. Para mudar de ramo, use `move_note`; para colocar numa seção, use `move_note_to_group`. Para mudar a posição, `reorder_sidebar` exige todos os IDs atuais daquele conjunto de irmãos, uma vez cada. IDs ausentes, duplicados ou de outra seção causam erro sem escrita parcial. A ordem persiste e não muda ao editar o título ou Markdown.

Para ícones, use `set_note_icon` com `kind: "emoji"`, `"icon"` ou `"image"`; `search_page_icons` retorna os valores exatos. Para capas, use `set_note_cover` com `kind: "preset"` e o ID retornado por `list_cover_presets`, ou `kind: "image"`. Imagens devem ser caminhos locais absolutos: PNG, JPG, WebP, GIF ou SVG, até 10 MB; o Sparkpad copia o arquivo para seu armazenamento e mantém a imagem se o original for removido. Links remotos devem ser baixados pelo agente antes. Ícone e capa não alteram o conteúdo nem a revisão da nota.

`edit_note_block` usa `action: "insert"`, `"update"`, `"delete"` ou `"move"` e índices base zero na revisão lida. Inserir/atualizar exige Markdown de um único bloco; mover usa `target_index` como posição final. Negrito, itálico, sublinhado (`<u>`), riscado, links, títulos, listas, tarefas, citações e código continuam sendo definidos em Markdown; os demais blocos preservam sua formatação.

`update_preferences` aceita apenas os campos enviados: `theme` (`dark`/`light`), `content_font` (`sans`/`serif`/`mono`), `opacity` (25–100%), `content_width` (40–100%), `font_size` (14–40 px), `sidebar_width` (180–420 px), `sidebar` e `always_on_top` (booleanos). Fontes afetam só conteúdo e título. Alterações aparecem no app aberto no intervalo de sincronização, ou na próxima abertura.

Exemplo de pedido ao agente:

> Crie uma nota para organizar as ideias do meu novo projeto, com uma página principal e subpáginas para decisões e tarefas. Use Markdown, selecione a página principal e mostre o Sparkpad.

## Dados e verificação

Novas instalações usam `~/Library/Application Support/Sparkpad/notes.sqlite3`. Instalações anteriores continuam usando o banco existente em `~/Library/Application Support/Interview Companion/notes.sqlite3`, preservando notas, hierarquia e preferências. `sparkpad db-path` mostra o caminho efetivo. `SPARKPAD_DB` permite escolher outro banco; `INTERVIEW_COMPANION_DB` continua aceito por compatibilidade. Configure o mesmo caminho no app e nos clientes MCP se usar essa opção.

SQLite usa WAL, espera por locks e revisões otimistas para concorrência entre processos. O MCP não abre portas de rede. A execução de `mcp` reserva stdout exclusivamente ao protocolo JSON-RPC.

```sh
cargo test --locked
cargo test --locked --no-default-features
cargo check --locked
python3 scripts/smoke_mcp.py target/debug/sparkpad
```

O smoke test usa um banco temporário e testa o servidor real por stdio, incluindo handshake, criação, atualização, conflito, seleção, exclusão e erros de protocolo.


## Opções da página

Ao passar o mouse sobre o título, aparecem **Adicionar ícone** e **Adicionar capa**, acima dele, sem deslocar o conteúdo. O seletor de ícone oferece emojis e atualiza também a sidebar. A galeria de capas oferece cores sólidas e gradientes desenhados pelo GPUI. A aba **Carregar** permite usar uma imagem local (PNG, JPG, WebP ou GIF). É possível trocar a capa ou removê-la pelo seletor. O ícone e a referência da capa também persistem ao reiniciar.

## Notas filhas e árvore da sidebar

O `+` no cabeçalho cria uma nota na raiz. O `+` que aparece ao passar o mouse sobre uma linha cria uma filha dessa nota. Cada filha pode ter outras filhas, sem limite de profundidade definido pelo app.

Passe o mouse sobre a linha para trocar o ícone da página pela seta de expandir/recolher e mostrar os botões de adicionar filha e abrir o menu. O menu permite renomear, adicionar uma filha ou mover para a raiz. As linhas têm 30 px de altura, intervalo de 1 px, texto de 14 px e ícone de 14 px; a seta tem 12 px e os controles usam alvos de 20 px. Expandir uma página não mantém o fundo de hover. Clique na seta para expandir/recolher um ramo. Arraste uma nota sobre outra para torná-la filha; a subárvore acompanha o movimento. Solte sobre **SparkPad** para devolver à raiz. Com a árvore em foco, ↑/↓ navegam, → expande/entra, ← recolhe/volta e Enter foca o editor. Ramos expandidos ficam salvos, e uma nota selecionada pelo MCP revela seus ancestrais automaticamente.

A estrutura segue a abordagem do [painel de projetos do Zed](https://github.com/zed-industries/zed/blob/main/crates/project_panel/src/project_panel.rs): índices por ID, uma lista plana das linhas visíveis e `gpui::uniform_list` para renderizar apenas a área visível. O modelo percorre a hierarquia de forma iterativa. A sidebar consulta metadados sem carregar o Markdown de todas as notas, e usa `PRAGMA data_version` para evitar reconstruções periódicas quando o banco não mudou.

A migração adiciona `parent_id` sem alterar títulos ou textos existentes. As notas anteriores continuam na raiz. O banco valida pais existentes, rejeita ciclos e protege movimentos com revisões e transações. Para excluir uma nota com filhas, mova ou exclua as filhas primeiro; não há exclusão em cascata implícita.

## Agrupadores da sidebar

O botão de pasta com `+`, sempre visível ao lado de **SparkPad**, cria um grupo na raiz e permite nomeá-lo inline. Grupos são organizadores manuais (por exemplo, Favoritos ou Trabalho), sem Markdown, capa ou página própria. Não podem conter outros grupos. O espaço entre seções acompanha as páginas visíveis: ao recolher um grupo, o respiro abaixo dele desaparece e os cabeçalhos ficam próximos. Páginas e grupos vazios podem ser expandidos: exibem uma linha discreta “Vazio”, que desaparece ao recolher ou ao criar o primeiro item interno. Clique no nome ou na seta para recolher/expandir; os controles `+` e menu aparecem durante o hover. O `+` cria uma página dentro do grupo; o menu renomeia ou exclui apenas o agrupador, devolvendo suas páginas à raiz.

Arraste uma página para o cabeçalho de um grupo para incluí-la; todas as descendentes acompanham. Se a página era filha, passa a ser raiz dentro do grupo. Arrastá-la sobre outra página a torna filha dessa página; soltar sobre **SparkPad** a devolve à raiz sem grupo. A árvore virtualizada inclui os cabeçalhos e só desenha as linhas visíveis. Grupos, associação das páginas e estado expandido são persistidos em SQLite. O MCP aceita `group_id` em `create_note` (não combine com `parent_id`) e oferece as cinco ferramentas de grupos descritas acima.

## Ícones das páginas

O seletor inspirado no Notion tem abas **Emoji**, **Ícones** e **Fazer upload**, busca, categorias, recentes, seleção aleatória e tons de pele. O catálogo Unicode 17 contém 3.944 sequências completas (incluindo variações de pele, gênero, famílias e bandeiras), com nomes e termos em português e inglês. O botão de tons oferece as variantes e **Todos**. A grade é virtualizada e o catálogo funciona offline. A aba Ícones usa os SVGs da Iconoir já embutidos na empire-ui. Emojis, ícones SVG e imagens próprias aparecem tanto no título quanto na sidebar.

O upload aceita PNG, JPG, WebP, GIF e SVG, até 10 MB. A imagem é copiada para a pasta `.assets` ao lado do SQLite, sem depender do arquivo original. Remover ou trocar o ícone não modifica o Markdown nem a revisão da nota. A seleção e os últimos 30 ícones são persistidos. Licenças e fontes do catálogo estão em `assets/emoji/SOURCES.md` e acompanham o bundle.

## Licença

Sparkpad e a biblioteca empire-ui usam **AGPL-3.0-or-later**; consulte [LICENSE](LICENSE). As dependências GPUI e gpui-component mantêm suas licenças e avisos nos respectivos diretórios em `vendor`. Ícones, fontes e dados de emoji mantêm suas licenças em `assets` e são incluídos no pacote do app.

A ferramenta `delete_note` aceita `include_children: true` para excluir a nota e todas as descendentes em uma única transação. Sem esse parâmetro, páginas com filhas continuam protegidas; `expected_revision` é obrigatório nos dois casos.

Blocos de código têm um seletor de linguagem com 30 linguagens e texto simples. O destaque usa Tree-sitter da biblioteca GPUI, atualiza durante a edição e acompanha o tema claro/escuro. A linguagem fica na cerca Markdown (por exemplo, `javascript`, `rust` ou `python`), inclusive ao editar notas pelo MCP. Linguagens desconhecidas mantêm seu identificador e são exibidas como texto simples.

Nos blocos de código, Tab insere dois espaços ou indenta as linhas selecionadas; Shift+Tab recua a indentação. O botão de copiar ao lado da linguagem copia somente o código, preservando espaços e quebras de linha. JavaScript/JSX, TypeScript e TSX (React + TypeScript) ficam no início do seletor.

Com o foco em um bloco de código, Ctrl+A/⌘A seleciona apenas seu conteúdo. Seleção de código não abre a toolbar de formatação e os atalhos de negrito/itálico/sublinhado/código inline são ignorados. Copiar/recortar usa texto literal; apagar todo o código mantém o bloco e a linguagem. A formatação de uma seleção mista da página ignora os blocos de código.

O bloco de código tem padding de 30 px em todos os lados. Seletor e copiar ficam em um container absoluto no canto superior direito, visível no hover (ou enquanto o dropdown está aberto), sem alterar a geometria do texto.

## Landing page (Astro)

O monorepo contém o app nativo Rust na raiz e a landing page independente em `apps/website`.

```sh
npm install
npm run dev:web      # http://localhost:4173
npm run check:web
npm run build:web    # apps/website/dist
```

A página padrão (`/`) é em inglês. O seletor no header permite português brasileiro (`/pt-br/`), espanhol (`/es/`) e francês (`/fr/`). Conteúdo, metadados, acessibilidade e a prévia interativa são traduzidos; todas as rotas são geradas estaticamente. Veja [apps/website/README.md](apps/website/README.md).

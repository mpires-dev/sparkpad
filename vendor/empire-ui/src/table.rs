//! `Table` — a **tabela de dados** do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/table.tsx`
//!
//! # Uso
//!
//! ```ignore
//! Table::new("faturas")
//!     .variant(TableVariant::Card)
//!     .caption("Suas faturas recentes.")
//!     .column(TableColumn::new("Fatura").width(TableColumnWidth::Fixed(110.0)))
//!     .column(TableColumn::new("Situação"))
//!     .column(
//!         TableColumn::new("Valor")
//!             .width(TableColumnWidth::Fixed(110.0))
//!             .align(TableAlign::End),
//!     )
//!     .row(TableRow::new().cell("INV001").cell("Paga").cell("R$ 250,00"))
//!     .row(TableRow::new().selected(true).cell("INV002").cell("Pendente").cell("R$ 150,00"))
//!     .footer(TableRow::new().cell("Total").cell("").cell("R$ 400,00"))
//! ```
//!
//! # ⚠️ A decisão que muda o contrato: **as larguras são dado do componente**
//!
//! O GPUI não tem `<table>`. Não existe `border-collapse`, `border-separate`, `border-spacing`,
//! `caption-side`, `colgroup`, nem — e isto é o que importa — o **algoritmo de layout de tabela**,
//! que mede o conteúdo de TODAS as linhas pra decidir a largura de cada coluna.
//!
//! Aqui cada linha é uma **linha de flex** e cada célula um item dela. Flex resolve larguras
//! linha por linha, sem olhar as vizinhas: duas linhas com conteúdos diferentes sairiam com colunas
//! desalinhadas. A única forma de as colunas se alinharem é a largura **não** vir do conteúdo — e
//! por isso ela vem de fora, em [`TableColumn::width`].
//!
//! É uma mudança de contrato em relação a uma `<table>` de verdade, e ela é visível pra quem usa:
//!
//! | | `<table>` | aqui |
//! |---|---|---|
//! | largura da coluna | medida do conteúdo mais largo | declarada em [`TableColumnWidth`] |
//! | conteúdo maior que a coluna | a coluna cresce | o conteúdo é recortado (`whitespace-nowrap` + elipse) |
//! | colunas de tabelas diferentes | independentes | independentes |
//!
//! O default de uma coluna é [`TableColumnWidth::Flex`] (divide o espaço em partes iguais), que é o
//! que mais se parece com uma tabela simples. Colunas de valor — data, dinheiro, situação — pedem
//! [`TableColumnWidth::Fixed`], e é aí que a diferença aparece.
//!
//! # Oito fatias, três tipos
//!
//! O original tem oito componentes (`Table`, `TableHeader`, `TableBody`, `TableFooter`, `TableRow`,
//! `TableHead`, `TableCell`, `TableCaption`). Aqui são três: [`Table`], [`TableColumn`] e
//! [`TableRow`].
//!
//! O motivo é o mesmo do [`crate::card`], e é o mesmo motivo da seção anterior: quase toda regra de
//! estilo do original é uma regra de **vizinhança** (`first:`, `last:`, `[&_tr:last-child]`,
//! `in-data-[variant=card]:*:[tr]:*:[td]:first:…`) — coisas que o CSS lê de fora e que um dono em
//! Rust decide localmente. Como a [`Table`] já **tem** que conhecer as colunas pra alinhá-las, ela
//! também sabe qual célula é a primeira, qual linha é a última e em que fatia cada uma está. Todas
//! essas decisões viraram as funções puras [`cell_pad`], [`row_borders`], [`row_radius`] e
//! [`row_fill`], que são o que os testes travam.
//!
//! O mapa:
//!
//! | coss | aqui |
//! |---|---|
//! | `Table` | [`Table`] |
//! | `TableHeader` + `TableHead` | [`TableColumn::new`] (o rótulo da coluna) |
//! | `TableBody` + `TableRow` | [`Table::row`] + [`TableRow`] |
//! | `TableCell` | [`TableRow::cell`] |
//! | `TableFooter` | [`Table::footer`] |
//! | `TableCaption` | [`Table::caption`] |
//!
//! # As duas variantes
//!
//! - **[`TableVariant::Default`]** — sem superfície: fundo transparente, linhas separadas por
//!   `border-b` (menos a última), cabeçalho com filete embaixo, rodapé com filete em cima e um
//!   fundo levemente tingido.
//! - **[`TableVariant::Card`]** — o corpo vira um **card**: fundo `--card`, moldura de 1px em volta,
//!   raio de 14px, `shadow-xs/5` e o fio de bisel da casa. O cabeçalho e o rodapé ficam FORA do
//!   card (transparentes), como no original.
//!
//! # ⚠️ `overflow_hidden` não recorta pelo raio — e por isso o fundo é da LINHA
//!
//! No original, na variante `card`, o fundo (`bg-card`) e a moldura vivem nas **células**, e os
//! quatro cantos do card são raios aplicados às quatro células de canto
//! (`first:*:[td]:first:rounded-ss-xl` e as três irmãs). É o desenho natural em CSS porque um
//! `<tr>` com `border-collapse` não consegue carregar fundo nem borda confiáveis.
//!
//! Aqui a tentação seria copiar isso e pôr `overflow_hidden` no corpo pra o fundo das células de
//! canto não vazar por cima da curva. **Não funciona**: o `overflow_hidden` do GPUI instala um
//! [`gpui::ContentMask`], que é um recorte **retangular** — ele não conhece o raio. O fundo da
//! célula apareceria quadrado nos quatro cantos, com a curva do card desenhada por baixo dele.
//!
//! A saída é não ter o que recortar: **fundo, moldura e raio vivem na LINHA**, não nas células. As
//! células apenas ladrilham a linha (não há `border-spacing`), então a união dos fundos delas é
//! exatamente o fundo da linha — o resultado é pixel a pixel o mesmo, e o GPUI desenha o fundo e a
//! borda de um elemento **já respeitando** os raios de canto dele. Nenhuma máscara envolvida.
//!
//! De quebra é isso que faz o `hover` funcionar: um refinamento `.hover()` do GPUI muda o estilo do
//! próprio elemento, e não o dos filhos — com o fundo nas células, passar o mouse na linha não
//! conseguiria repintá-las.
//!
//! # Rolagem horizontal: `overflow_x_scroll` cru, e não a [`crate::ScrollArea`]
//!
//! O original é `overflow-x-auto`: barra nativa, sem fade e sem polegar desenhado. A
//! [`crate::ScrollArea`] entrega bem mais que isso — e dois dos "mais" atrapalham aqui:
//!
//! 1. **Ela aninha o conteúdo um nível abaixo do elemento que rastreia**, e no eixo horizontal esse
//!    conteúdo é `absolute` (é o que dá curso de rolagem sem o viewport crescer — ver o comentário
//!    grande no `render` dela). Um filho absoluto **não dá altura ao pai**: a área é `size_full`,
//!    então a altura da tabela deixaria de vir da tabela e passaria a ter que ser declarada de fora.
//!    Uma tabela que não sabe a própria altura não serve.
//! 2. O fade de borda dela vem **ligado por padrão** (desvio declarado naquele módulo) e é um
//!    gradiente da cor do fundo pintado por cima; sobre as linhas de uma tabela ele apagaria a
//!    primeira e a última coluna.
//!
//! Então aqui é `overflow_x_scroll` no embrulho, que é exatamente o `overflow-x-auto` do original.
//! Ele precisa de `id` no elemento (o deslocamento vive no `element_state`, que só existe com
//! `GlobalElementId` — ver `Interactivity::before_layout`), e é por isso que [`Table::new`] pede um.
//! Quem quiser a barra do coss embrulha a [`Table`] numa [`crate::ScrollArea`] por fora, onde a
//! altura já está resolvida.
//!
//! ⚠️ **E o embrulho é uma COLUNA DE FLEX, não um bloco** — é ele que dá largura à tabela, e não uma
//! porcentagem. Ver a entrada do `w-full` em "Não reproduzível", abaixo: é a peça que consertou o
//! defeito de a tabela sair do tamanho do conteúdo em vez da largura do contêiner.
//!
//! # Seleção: [`RenderOnce`], com o estado vindo de fora
//!
//! Como o [`crate::Toggle`], e ao contrário do [`crate::Tabs`].
//!
//! O `data-state=selected` do original é escrito por quem usa (no exemplo do coss, pelo
//! `row.getIsSelected()` do TanStack Table). Não é acidente: "quais linhas estão selecionadas" é
//! fato do **conjunto de dados**, não da tabela que o desenha. Ele é lido pela caixa de seleção da
//! coluna de checkbox, pelo "selecionar todos" do cabeçalho, pela barra de ações em massa e pela
//! requisição que vai ao servidor. Uma cópia dentro da tabela seria uma segunda fonte de verdade
//! sobre a mesma coisa — e a que dessincroniza é sempre a de dentro.
//!
//! O [`crate::Tabs`] é `Entity` porque lá o oposto é verdade: a aba ativa é fato **da vista**, não
//! existe dono natural fora dela, e por isso ele guarda o índice e emite `TabsEvent::Change`.
//!
//! O `hover` não precisa de estado nenhum: ele é refinamento de estilo do GPUI (`.hover()`), que
//! resolve na hora de pintar.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`border-collapse` / `border-separate` / `border-spacing-0`**: são propriedades do layout de
//!   tabela do CSS, que o GPUI não tem. Aqui as células ladrilham a linha sem vão, o que é o mesmo
//!   resultado que `border-separate border-spacing-0`, e as bordas são declaradas uma vez cada (na
//!   linha) — então não há o que colapsar.
//! - **`caption-side: bottom`**: sem `<caption>`, a legenda é só o último filho. O
//!   `caption-bottom` do original já a põe embaixo, então o resultado coincide; o que não existe é
//!   a possibilidade de mudar de lado.
//! - **`has-[[role=checkbox]]:w-px`**: num `<table>`, `width` é um **mínimo** — `w-px` significa
//!   "colapse até o conteúdo". Em flex, `w-px` seria 1px literal e a caixa de seleção vazaria pra
//!   fora. Aqui a intenção é dita direto, em [`TableColumnWidth::Checkbox`]: `flex_none` sem largura
//!   declarada, ou seja a largura do conteúdo. O par `first:pe-0`/`last:ps-0` da referência é
//!   reproduzido fielmente (ver [`cell_pad`]).
//! - **`bg-clip-padding` na célula**: existe pra o fundo da célula não invadir a borda dela. Aqui a
//!   célula não tem fundo (ele é da linha — ver a seção do `overflow_hidden`), então não há o que
//!   recortar.
//! - **`relative` em `TableRow`**: no original serve pra ancorar overlays que quem usa ponha dentro
//!   da linha. As linhas daqui são `relative` de todo jeito.
//! - **`w-full` no `<table>`**: a intenção ("ocupe a largura do embrulho") é reproduzida, o
//!   MECANISMO não. Uma porcentagem ali não sobrevive: o embrulho tem `overflow` de rolagem, e
//!   quando o taffy precisa medir a árvore sob espaço indefinido — o que acontece se algum ancestral
//!   tirar a largura do flex **e** alguma tabela irmã na mesma coluna for mais larga que ela — o
//!   `width: 100%` cai pra `width: auto` = tamanho do conteúdo, com o `min_w` das colunas fixas como
//!   único piso. Medido na página do storybook: embrulho **738**, tabela **126**, coluna `Flex` em
//!   **zero**. Um embrulho de bloco com a tabela em `auto` dá o mesmo número — não é o `w_full` que
//!   está errado, é o bloco que não estica ali. A intenção é dita então pelo **esticamento de eixo
//!   cruzado**: o embrulho é uma coluna de flex com um item só, e `align-items: stretch` (o default)
//!   resolve do tamanho já definido do pai, sem porcentagem no meio. Ver
//!   `tests_de_janela::a_tabela_ocupa_a_largura_com_uma_irma_mais_larga_na_pagina`, que traz as
//!   quatro peças necessárias pra o defeito aparecer.
//!
//! **Resolvido em número**
//!
//! - Todo `color-mix(in srgb, …)` da referência está resolvido em `0xRRGGBBAA`, com a conta no
//!   comentário de cada campo de [`TABLE_LIGHT`] / [`TABLE_DARK`].
//! - `rounded-xl` = `--radius-xl` = `calc(var(--radius) + 4px)` = 10 + 4 = **14px** ([`RADIUS`]).
//! - `calc(--spacing(2.5) - 1px)` = 10 − 1 = **9px** ([`CARD_EDGE_PAD`]).
//! - `shadow-xs/5` = preto a 5% (alfa 13), `0 1px 2px 0` — a mesma sombra de superfície em repouso
//!   do [`crate::card`], do [`crate::Button`] e do [`crate::Toggle`].
//! - O bisel `before:shadow-[0_1px_…]` é uma **borda de 1px num lado**, e não uma
//!   [`gpui::BoxShadow`]: o `Window::paint_shadows` do GPUI não recorta a sombra pra fora do
//!   elemento que a projeta, então uma sombra viraria uma lavagem de cor sobre a tabela inteira. O
//!   lado sai do **sinal** do deslocamento (`0 1px` desce → borda de baixo; `0 -1px` sobe → borda de
//!   topo). Ver [`bevel_overlay`].
//! - `leading-none` é entrelinha **igual ao corpo** — 14px, e não os 22,65px que a razão de ouro do
//!   GPUI daria. Ver [`LINE_HEIGHT`], que é a armadilha nº 1 da casa e a que mais mexe na altura
//!   aqui: 62% a mais em CADA linha da tabela.
//!
//! **Desvio consciente**
//!
//! - O fio de bisel do tema escuro é o **DOBRO** do original (branco a ~16% em vez de 8%), como no
//!   [`crate::card`], no [`crate::Button`], no [`crate::Input`], no [`crate::Toggle`] e no
//!   [`crate::empty`]: no nosso fundo escuro o filete da referência é imperceptível. Ver
//!   [`TABLE_DARK`] e o teste `bisel_escuro_e_o_dobro_da_referencia`.
//! - **O `hover` é só das linhas do CORPO.** Na referência ele está na classe base de `TableRow`, que
//!   é usada também nas linhas de cabeçalho e de rodapé — então lá o cabeçalho muda de cor sob o
//!   ponteiro. É quase certamente efeito colateral e não intenção: um realce de linha diz "esta
//!   linha é o alvo de uma ação", e cabeçalho e rodapé não são. Ver [`row_fill`] e o teste
//!   `hover_e_so_das_linhas_do_corpo`.
//! - **Selecionado vence o hover.** As duas regras da referência (`hover:bg-…2%` e
//!   `data-[state=selected]:bg-…4%`) moram na mesma camada, e qual ganha depende da ordem em que o
//!   Tailwind emite as variantes. A leitura adotada é a mesma do [`crate::Toggle`] (ver
//!   `hover_nao_apaga_o_estado_ligado` lá): o estado é informação, o hover é só afordância — passar
//!   o mouse não pode apagar a informação. Ver o teste `hover_nao_apaga_a_selecao`.
//! - **O desconto de 1px no respiro da ponta é só do CORPO.** A referência põe
//!   `first:ps-[calc(--spacing(2.5)-1px)]` no `TableCell`, que serve corpo **e** rodapé (e não no
//!   `TableHead`). O desconto existe pra compensar a moldura de 1px do card, e a moldura é só das
//!   linhas de corpo — o rodapé fica fora do card. Aplicá-lo lá deixaria o total 1px à esquerda da
//!   coluna dele. Ver [`cell_pad`] e o teste `respiro_otico_da_ponta_e_igual_em_todas_as_fatias`.
//! - **O cabeçalho não tem fundo.** Vale registrar porque é fácil de trocar: o
//!   `color-mix(in srgb, var(--card), black 2%)` da referência está no **`TableFooter`**, não no
//!   `TableHeader` — o cabeçalho tem só `[&_tr]:border-b`. Ver o teste
//!   `o_fundo_tingido_e_do_rodape_e_nao_do_cabecalho`.
//!
//! **Superset consciente**
//!
//! - **[`TableAlign`]** — alinhamento por coluna. No coss o alinhamento é `className="text-right"`
//!   posto célula a célula (é o que o exemplo do rodapé faz pra o "Total"). Aqui a coluna é dado, e
//!   repetir o alinhamento em toda célula seria convidar ao erro de esquecer uma; então ele é
//!   propriedade da coluna e vale no cabeçalho, no corpo e no rodapé de uma vez.
//! - **[`Table::track_scroll`]** — um [`gpui::ScrollHandle`] de fora, pra ler a posição horizontal ou
//!   rolar por código. A referência não tem equivalente (o `overflow-x-auto` dela é opaco). É opcional
//!   e não muda o desenho.
//! - **Elipse** (`text_ellipsis`) nas células. O original tem `whitespace-nowrap` e deixa a coluna
//!   crescer; como aqui a largura é declarada (ver a primeira seção), o texto que não cabe tem que
//!   ter um fim visível — sem isso ele sairia recortado no meio de uma letra.
//!
//! **Ausente**
//!
//! - **Mais de uma linha de rodapé.** [`Table::footer`] aceita uma só, que é o que o exemplo do coss
//!   usa. As regras `[&>tr]:last:border-b-0` da referência existem pra o caso de várias; com uma, o
//!   resultado é o mesmo.
//! - **Ordenação, redimensionamento de coluna e seleção por clique.** Não estão na referência (lá
//!   vêm do TanStack Table, por fora). A [`Table`] desenha; quem ordena é quem monta as linhas.
//! - **`pointer-coarse`**: não há ramo de ponteiro grosso; a referência da tabela também não tem.
//!
//! **Sem cobertura de teste — declarado**
//!
//! - **O `flex_none` da coluna [`TableColumnWidth::Checkbox`].** Removê-lo não faz nenhum teste
//!   falhar — foi verificado por mutação. Ele só é exercido sob APERTO, e o único jeito de haver
//!   aperto é o contêiner ser mais estreito que "conteúdo da coluna de checkbox + colunas fixas":
//!   [`TableColumnWidth::min_width`] devolve zero pra ela porque a largura dela é a do conteúdo, que
//!   não se conhece antes do layout, então esse pedaço não entra no `min_w` que protege as outras
//!   colunas. Nesse aperto **alguma** coluna encolhe — com o `flex_none` é a fixa, sem ele é a de
//!   checkbox —, e as duas coisas contrariam o contrato de larguras declaradas. Não há teste porque
//!   um teste ali travaria um dos dois encolhimentos como se fosse o certo. Fica declarado: uma
//!   tabela com coluna de checkbox mais estreita que o próprio conteúdo mínimo aperta as colunas
//!   fixas em vez de rolar.
//! - **Que o defeito de largura não tenha um QUINTO caminho.** O teste
//!   `tests_de_janela::a_tabela_ocupa_a_largura_com_uma_irma_mais_larga_na_pagina` reproduz o arranjo
//!   que foi medido na página real, e cada uma das quatro peças dele foi tirada e medida (sem
//!   qualquer uma o defeito desaparece). O que ele NÃO prova é que não exista outro arranjo em que o
//!   taffy volte a medir a árvore sob espaço indefinido. O esticamento do embrulho não tem o buraco
//!   da porcentagem, então a expectativa é que não haja — mas isso é expectativa, não medição.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, point, px, AnyElement, App, ElementId, Hsla, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement as _, Styled, Window,
};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Convenção da casa: TODO valor é `0xRRGGBBAA`, com o byte de alfa, SEMPRE (ver [`crate::color`]).
// Misturar com os tokens de 6 dígitos de [`crate::theme`] desloca os canais e produz outra cor sem
// erro de compilação — já custou três bugs visíveis nesta base.
//
// As contas de `color-mix(in srgb, A, B N%)` são todas do mesmo tipo: canal = A·(1−N) + B·N,
// arredondado ao inteiro mais próximo. Os tokens de partida, de `globals.css`:
//
// | token | claro | escuro |
// |---|---|---|
// | `--background` | `#ffffff` | mix(neutral-950 96%, branco) = `#141414` (20) |
// | `--card` | `#ffffff` | mix(--background 98%, branco) = `#191919` (25) |
// | `--muted-foreground` | mix(neutral-500 90%, preto) = `#686868` | mix(neutral-500 90%, branco) = `#818181` |
// | `--border` | preto 8% (alfa 20) | branco 6% (alfa 15) |

/// Tokens visuais da tabela, por tema.
#[derive(Clone, Copy, Debug)]
struct TablePalette {
    /// `--card` — o fundo do corpo na variante [`TableVariant::Card`].
    card: Rgba8,
    /// `--foreground` — o texto das células.
    text: Rgba8,
    /// `--muted-foreground` — o texto do cabeçalho e da legenda.
    text_muted: Rgba8,
    /// `--border` — os filetes entre linhas e a moldura do card.
    border: Rgba8,
    /// Linha sob o ponteiro na variante padrão: `color-mix(--background, tinta 2%)`.
    row_hover: Rgba8,
    /// Linha selecionada na variante padrão: `color-mix(--background, tinta 4%)`.
    row_selected: Rgba8,
    /// Linha sob o ponteiro na variante card: `color-mix(--card, tinta 2%)`.
    card_row_hover: Rgba8,
    /// Linha selecionada na variante card: `color-mix(--card, tinta 4%)`.
    card_row_selected: Rgba8,
    /// Fundo do rodapé na variante padrão: `color-mix(--card, tinta 2%)`.
    ///
    /// ⚠️ É o **rodapé**, não o cabeçalho — ver o doc do módulo.
    footer_bg: Rgba8,
    /// Sombra externa do card (`shadow-xs/5`).
    shadow: Rgba8,
    /// Fio de bisel de 1px do card. Desce no claro, sobe no escuro.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro), `-1` sobe (escuro). É o SINAL que escolhe o lado, e não
    /// um segundo campo que poderia divergir dele.
    bevel_dir: f32,
}

/// Tema **claro**. A tinta dos `color-mix` é **preto**.
///
/// No claro `--background` e `--card` são os dois `#ffffff`, então as três misturas a 2%
/// (hover da variante padrão, hover da card e fundo do rodapé) caem no MESMO `#fafafa`. É
/// coincidência de token, não simplificação: no escuro elas se separam.
const TABLE_LIGHT: TablePalette = TablePalette {
    card: Rgba8(0xffffffff),
    text: Rgba8(0x262626ff), // neutral-800
    // mix(neutral-500 90%, preto) = 115·0,9 = 103,5 → 104 = 0x68
    text_muted: Rgba8(0x686868ff),
    border: Rgba8(0x00000014), // preto 8% → alfa 20
    // mix(#ffffff, preto 2%) = 255·0,98 = 249,9 → 250 = 0xfa
    row_hover: Rgba8(0xfafafaff),
    // mix(#ffffff, preto 4%) = 255·0,96 = 244,8 → 245 = 0xf5
    row_selected: Rgba8(0xf5f5f5ff),
    // mix(--card #ffffff, preto 2%) — o mesmo #fafafa, porque --card == --background aqui
    card_row_hover: Rgba8(0xfafafaff),
    // mix(--card #ffffff, preto 4%)
    card_row_selected: Rgba8(0xf5f5f5ff),
    // mix(--card #ffffff, preto 2%)
    footer_bg: Rgba8(0xfafafaff),
    shadow: Rgba8(0x0000000d), // preto 5% → alfa 13
    bevel: Rgba8(0x0000000a),  // preto 4% → alfa 10
    bevel_dir: 1.0,
};

/// Tema **escuro**. A tinta dos `color-mix` é **branco**.
const TABLE_DARK: TablePalette = TablePalette {
    // `--card: color-mix(in srgb, var(--background) 98%, white)`, com --background = #141414 (20):
    // 20·0,98 + 255·0,02 = 19,6 + 5,1 = 24,7 → 25 = 0x19. É o mesmo número que o `crate::card` já
    // tem travado, e tem que continuar sendo: um card de tabela ao lado de um `crate::Card` que
    // desafinasse 1/255 de cinza seria visível como uma emenda.
    card: Rgba8(0x191919ff),
    text: Rgba8(0xf5f5f5ff), // neutral-100
    // mix(neutral-500 90%, branco) = 115·0,9 + 255·0,1 = 103,5 + 25,5 = 129 = 0x81
    text_muted: Rgba8(0x818181ff),
    border: Rgba8(0xffffff0f), // branco 6% → alfa 15
    // mix(--background #141414, branco 2%) = 20·0,98 + 255·0,02 = 19,6 + 5,1 = 24,7 → 25 = 0x19
    row_hover: Rgba8(0x191919ff),
    // mix(--background #141414, branco 4%) = 20·0,96 + 255·0,04 = 19,2 + 10,2 = 29,4 → 29 = 0x1d
    row_selected: Rgba8(0x1d1d1dff),
    // mix(--card #191919, branco 2%) = 25·0,98 + 255·0,02 = 24,5 + 5,1 = 29,6 → 30 = 0x1e
    card_row_hover: Rgba8(0x1e1e1eff),
    // mix(--card #191919, branco 4%) = 25·0,96 + 255·0,04 = 24,0 + 10,2 = 34,2 → 34 = 0x22
    card_row_selected: Rgba8(0x222222ff),
    // mix(--card #191919, branco 2%) — igual ao hover da variante card
    footer_bg: Rgba8(0x1e1e1eff),
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE, o MESMO já vigente no `Card`, no `Input`, no `Frame`, no `Button`, no
    // `Toggle` e no `Empty`: a referência usa branco a 8% (alfa 20) e aqui é o DOBRO — 16%, alfa 41
    // ≈ 0,161 — porque a 8% o filete é imperceptível no nosso fundo escuro. NÃO "corrija" pra 0x14
    // achando que é erro de porte; se a intenção mudar, mude junto o teste
    // `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff29),
    bevel_dir: -1.0,
};

/// A paleta da tabela no tema corrente.
fn palette() -> &'static TablePalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &TABLE_DARK,
        theme::ThemeMode::Light => &TABLE_LIGHT,
    }
}

// =================================================================================================
// Geometria
// =================================================================================================

/// Corpo do texto — o `text-sm` que a referência põe no `<table>` e que todas as fatias herdam.
const TEXT_SIZE: f32 = 14.0;

/// Entrelinha das células e do cabeçalho — `leading-none`, ou seja **igual ao corpo**.
///
/// ⚠️ **A armadilha nº 1 da casa, e em tabela ela é a mais cara.** Sem declarar, o GPUI usa
/// `relative(1.618_034)` e cada linha ganha 8,65px que a referência não tem. Numa tabela de dez
/// linhas isso são 86px de altura inventada — a tabela inteira muda de tamanho, não só uma caixa.
/// Travado no teste de janela `entrelinha_apertada_mantem_a_altura_da_linha`, que mede a altura
/// REAL: um teste de constante não pega o esquecimento de chamar `.line_height()`.
const LINE_HEIGHT: f32 = 14.0;

/// Entrelinha da **legenda** — o par do `text-sm` no Tailwind: 14 de corpo, **20** de linha.
///
/// A legenda é a única fatia que NÃO tem `leading-none` na referência (`TableCaption` é só
/// `text-sm`), então ela usa o par normal. A diferença é de propósito: a legenda é prosa, as células
/// são dado.
const CAPTION_LINE_HEIGHT: f32 = 20.0;

/// Altura TOTAL da linha de cabeçalho — `h-10`. Inclui o filete de baixo (box-sizing border-box).
const HEAD_HEIGHT: f32 = 40.0;

/// Respiro base das células — `px-2.5` no cabeçalho e `p-2.5` no corpo.
const PAD: f32 = 10.0;

/// Respiro vertical das células de **rodapé** — `in-data-[slot=table-footer]:py-3.5`.
const FOOTER_PAD_Y: f32 = 14.0;

/// Respiro das células da **ponta** na variante card — `calc(--spacing(2.5) - 1px)` = 10 − 1.
///
/// O `-1px` desconta a moldura do card: 9 de respiro + 1 de borda fecham os 10px ÓTICOS das outras
/// células. Sem ele o conteúdo da primeira e da última coluna sairia 1px mais pra dentro que o resto.
const CARD_EDGE_PAD: f32 = 9.0;

/// Raio do card — `rounded-xl` = `--radius-xl` = `calc(var(--radius) + 4px)` = 10 + 4.
const RADIUS: f32 = 14.0;

/// Espessura de todo filete: linhas, moldura do card e bisel — `border` (1px).
const BORDER: f32 = 1.0;

/// Respiro da legenda — `mt-4` na variante padrão, `my-4` na card.
const CAPTION_MARGIN: f32 = 16.0;

// =================================================================================================
// Variante, fatia, coluna
// =================================================================================================

/// Variante visual da tabela — o `data-variant` do contêiner na referência.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TableVariant {
    /// Sem superfície: linhas separadas por filete, fundo transparente. O padrão.
    #[default]
    Default,
    /// O corpo é um **card**: fundo `--card`, moldura, raio de 14px, sombra e bisel.
    Card,
}

/// A **fatia** a que uma linha pertence — o `data-slot` da referência.
///
/// É o eixo que decide respiro vertical, fundo e filetes; junto com a variante e a posição, é tudo
/// que as funções puras deste módulo precisam saber.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slot {
    /// `thead` — a linha de rótulos.
    Head,
    /// `tbody` — as linhas de dado.
    Body,
    /// `tfoot` — a linha de totais.
    Footer,
}

/// Alinhamento horizontal de uma coluna.
///
/// **Superset consciente** — ver o doc do módulo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TableAlign {
    /// Encostado no início (`text-left` da referência). O padrão.
    #[default]
    Start,
    /// Encostado no fim — colunas de valor.
    End,
}

/// Como a largura de uma coluna é decidida. Ver a primeira seção do doc do módulo: aqui a largura é
/// **dado**, e não medida do conteúdo.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum TableColumnWidth {
    /// Divide em partes iguais o que sobrar depois das colunas fixas. O padrão.
    #[default]
    Flex,
    /// Largura exata, em px lógicos. É o que uma coluna de valor pede.
    Fixed(f32),
    /// **Colapsa até o conteúdo**, e some com o respiro do lado de dentro — a coluna de caixa de
    /// seleção (`has-[[role=checkbox]]` na referência).
    Checkbox,
}

impl TableColumnWidth {
    /// Quanto esta coluna contribui pra a largura MÍNIMA da tabela — é a soma disso que dá curso de
    /// rolagem horizontal quando o contêiner é estreito.
    ///
    /// [`Self::Flex`] contribui zero porque ela encolhe de propósito. [`Self::Checkbox`] também: a
    /// largura dela é a do conteúdo, que não se conhece antes do layout — ela é `flex_none`, então o
    /// taffy a soma ao `content_size` por conta própria.
    fn min_width(self) -> f32 {
        match self {
            TableColumnWidth::Fixed(w) => w.max(0.0),
            TableColumnWidth::Flex | TableColumnWidth::Checkbox => 0.0,
        }
    }
}

/// Uma **coluna**: o rótulo do cabeçalho, a largura e o alinhamento.
///
/// A largura é o que alinha as linhas entre si — ver a primeira seção do doc do módulo.
pub struct TableColumn {
    header: Option<SharedString>,
    width: TableColumnWidth,
    align: TableAlign,
}

impl TableColumn {
    /// Uma coluna com **rótulo** — o `TableHead` da referência.
    pub fn new(header: impl Into<SharedString>) -> Self {
        Self {
            header: Some(header.into()),
            width: TableColumnWidth::default(),
            align: TableAlign::default(),
        }
    }

    /// Uma coluna **sem rótulo** — a de caixa de seleção, ou a de botões de ação no fim da linha.
    ///
    /// Se NENHUMA coluna tiver rótulo, o cabeçalho não é desenhado (em vez de sair uma faixa de 40px
    /// vazia).
    pub fn blank() -> Self {
        Self {
            header: None,
            ..Self::new("")
        }
    }

    pub fn width(mut self, width: TableColumnWidth) -> Self {
        self.width = width;
        self
    }

    /// Alinhamento — vale no cabeçalho, no corpo e no rodapé de uma vez.
    pub fn align(mut self, align: TableAlign) -> Self {
        self.align = align;
        self
    }
}

// =================================================================================================
// Linha
// =================================================================================================

/// Uma **linha** — de corpo (via [`Table::row`]) ou de rodapé (via [`Table::footer`]).
///
/// As células entram na ordem das colunas. Faltando célula, a coluna sai vazia; sobrando, a sobra é
/// descartada — as duas coisas mantêm o alinhamento das colunas, que é o contrato do componente.
#[derive(Default)]
pub struct TableRow {
    cells: Vec<AnyElement>,
    selected: bool,
}

impl TableRow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Uma célula de **texto**.
    pub fn cell(self, text: impl Into<SharedString>) -> Self {
        let text: SharedString = text.into();
        self.child(div().child(text))
    }

    /// Uma célula com um elemento qualquer (uma caixa de seleção, um badge, um botão).
    pub fn child(mut self, cell: impl IntoElement) -> Self {
        self.cells.push(cell.into_any_element());
        self
    }

    /// **Se a linha está selecionada** — o `data-state=selected` da referência. O estado vem de
    /// fora; ver a seção sobre seleção no doc do módulo.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

// =================================================================================================
// As regras de vizinhança, como funções puras
// =================================================================================================
//
// Tudo que na referência é `first:`, `last:`, `[&_tr:last-child]` ou `in-data-[variant=card]:…` mora
// aqui. São as funções que os testes travam, e é onde a tabela do coss está de fato reproduzida — o
// `render` abaixo só traduz o que elas devolvem em chamadas do GPUI.

/// Os quatro lados de uma caixa, em px. Serve pra respiro e pra espessura de filete.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
struct Edges {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

/// Os quatro cantos de uma caixa, em px de raio.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
struct Radii {
    tl: f32,
    tr: f32,
    bl: f32,
    br: f32,
}

/// As duas cores de fundo de uma linha: em repouso e sob o ponteiro.
///
/// São duas porque o GPUI precisa das duas de uma vez — o hover é um refinamento de estilo
/// (`.hover()`), não um segundo render.
#[derive(Clone, Copy, PartialEq, Debug)]
struct RowFill {
    /// `None` = transparente.
    rest: Option<Hsla>,
    /// `None` = a linha não reage ao ponteiro.
    hover: Option<Hsla>,
}

/// **O respiro de uma célula.**
///
/// As regras da referência que isto resolve, na ordem em que se aplicam:
///
/// 1. `TableHead`: `px-2.5` e nenhum respiro vertical — a altura vem do `h-10` da linha.
/// 2. `TableCell`: `p-2.5` nos quatro lados.
/// 3. `TableCell` no rodapé: `in-data-[slot=table-footer]:py-3.5` troca só o vertical.
/// 4. Variante card, **só no corpo**: `first:ps-[calc(--spacing(2.5)-1px)]` e `last:pe-[…]`
///    descontam 1px do respiro das pontas, porque ali existe a moldura do card.
/// 5. Coluna de caixa de seleção: `first:has-[[role=checkbox]]:pe-0` e
///    `last:has-[[role=checkbox]]:ps-0` — ela perde o respiro do lado de DENTRO, pra a caixa
///    encostar na coluna seguinte.
///
/// A ordem importa: a regra 5 vem depois da 4, então uma coluna de checkbox na primeira posição de
/// uma tabela card fica com 9 à esquerda e 0 à direita.
///
/// ⚠️ **Por que a regra 4 é só do corpo.** A referência a põe no `TableCell`, que serve corpo E
/// rodapé, e NÃO no `TableHead`. O desconto existe pra compensar a moldura de 1px: onde não há
/// moldura, descontar desalinha. E a moldura é só das linhas de corpo (ver [`row_borders`]) —
/// cabeçalho e rodapé ficam fora do card. Seguir a referência ao pé da letra no rodapé deixaria o
/// total 1px à esquerda da coluna dele; no cabeçalho a referência já concorda com a gente. O
/// invariante que isso protege está no teste `respiro_otico_da_ponta_e_igual_em_todas_as_fatias`.
fn cell_pad(
    slot: Slot,
    variant: TableVariant,
    first_col: bool,
    last_col: bool,
    checkbox: bool,
) -> Edges {
    let mut e = match slot {
        // O cabeçalho não tem respiro vertical: quem dá a altura é o `h-10` da linha, e o
        // `align-middle` centra o conteúdo dentro dela.
        Slot::Head => Edges {
            top: 0.0,
            right: PAD,
            bottom: 0.0,
            left: PAD,
        },
        Slot::Body => Edges {
            top: PAD,
            right: PAD,
            bottom: PAD,
            left: PAD,
        },
        Slot::Footer => Edges {
            top: FOOTER_PAD_Y,
            right: PAD,
            bottom: FOOTER_PAD_Y,
            left: PAD,
        },
    };

    // Só o CORPO tem moldura pra compensar — ver a nota no doc desta função.
    if variant == TableVariant::Card && slot == Slot::Body {
        if first_col {
            e.left = CARD_EDGE_PAD;
        }
        if last_col {
            e.right = CARD_EDGE_PAD;
        }
    }

    if checkbox {
        if first_col {
            e.right = 0.0;
        }
        if last_col {
            e.left = 0.0;
        }
    }

    e
}

/// **Os filetes de uma linha.**
///
/// Na referência as bordas da variante card vivem nas CÉLULAS (`*:[tr]:*:[td]:border-b`,
/// `:first:border-s`, …), porque um `<tr>` com `border-collapse` não carrega borda confiável. Aqui
/// elas vivem na linha — ver a seção do `overflow_hidden` no doc do módulo. O desenho é o mesmo: as
/// células ladrilham a linha, então a borda esquerda da primeira célula É a borda esquerda da linha.
///
/// As regras traduzidas:
///
/// - **Cabeçalho** (as duas variantes): `[&_tr]:border-b` no `thead` mais o `border-b` da classe
///   base — filete embaixo.
/// - **Corpo, padrão**: `border-b` em todas, menos na última (`[&_tr:last-child]:border-0`).
/// - **Corpo, card**: `[tr]:border-0` apaga o filete da linha e as células o recriam como moldura —
///   `border-b` em todas (inclusive a última: é a base do card), `border-t` só na primeira,
///   `border-s` na primeira coluna e `border-e` na última.
/// - **Rodapé, padrão**: `border-t` no `tfoot` (o filete que separa do corpo) e, com uma linha só,
///   `[&>tr]:last:border-b-0` mata o de baixo.
/// - **Rodapé, card**: `in-data-[variant=card]:border-none` — o rodapé fica fora do card, sem filete
///   nenhum.
fn row_borders(slot: Slot, variant: TableVariant, first_row: bool, last_row: bool) -> Edges {
    match (slot, variant) {
        (Slot::Head, _) => Edges {
            bottom: BORDER,
            ..Edges::default()
        },
        (Slot::Body, TableVariant::Default) => Edges {
            bottom: if last_row { 0.0 } else { BORDER },
            ..Edges::default()
        },
        (Slot::Body, TableVariant::Card) => Edges {
            top: if first_row { BORDER } else { 0.0 },
            right: BORDER,
            bottom: BORDER,
            left: BORDER,
        },
        (Slot::Footer, TableVariant::Default) => Edges {
            top: BORDER,
            ..Edges::default()
        },
        (Slot::Footer, TableVariant::Card) => Edges::default(),
    }
}

/// **Os raios de uma linha.**
///
/// Só a variante card arredonda, e só o corpo: as quatro regras
/// `first:*:[td]:first:rounded-ss-xl`, `first:*:[td]:last:rounded-se-xl`,
/// `last:*:[td]:first:rounded-es-xl` e `last:*:[td]:last:rounded-ee-xl` da referência dizem
/// exatamente "arredonde os quatro cantos do bloco de linhas". Na linha, isso é: cantos de cima na
/// primeira, cantos de baixo na última.
///
/// Uma linha que é a primeira **e** a última (tabela de uma linha) arredonda os quatro — o caso que
/// um `if/else` fácil de escrever erraria.
fn row_radius(slot: Slot, variant: TableVariant, first_row: bool, last_row: bool) -> Radii {
    if variant != TableVariant::Card || slot != Slot::Body {
        return Radii::default();
    }
    let cima = if first_row { RADIUS } else { 0.0 };
    let baixo = if last_row { RADIUS } else { 0.0 };
    Radii {
        tl: cima,
        tr: cima,
        bl: baixo,
        br: baixo,
    }
}

/// **O fundo de uma linha**, em repouso e sob o ponteiro.
///
/// - **Cabeçalho**: nenhum fundo, nas duas variantes. ⚠️ O `color-mix(--card, tinta 2%)` da
///   referência é do RODAPÉ — ver o doc do módulo.
/// - **Corpo, padrão**: transparente em repouso; `color-mix(--background, tinta 2%)` no hover e
///   `…4%` selecionado. A mistura é com `--background` porque a variante padrão não tem superfície:
///   o que está atrás da linha é o fundo da tela.
/// - **Corpo, card**: `bg-card` em repouso, e as mesmas duas misturas — mas agora sobre `--card`,
///   porque é ele que está atrás.
/// - **Rodapé, padrão**: `color-mix(--card, tinta 2%)`. Note que ele mistura com `--card` mesmo na
///   variante que não tem card; no tema claro isso empata com o hover (os dois tokens são brancos),
///   no escuro não.
/// - **Rodapé, card**: `bg-transparent`.
///
/// **Selecionado vence o hover** e **o hover é só do corpo** — os dois são desvios declarados no doc
/// do módulo.
fn row_fill(
    slot: Slot,
    variant: TableVariant,
    selected: bool,
    p: &TablePalette,
) -> RowFill {
    match (slot, variant) {
        (Slot::Head, _) => RowFill {
            rest: None,
            hover: None,
        },
        (Slot::Footer, TableVariant::Default) => RowFill {
            rest: Some(p.footer_bg.hsla()),
            hover: None,
        },
        (Slot::Footer, TableVariant::Card) => RowFill {
            rest: None,
            hover: None,
        },
        (Slot::Body, variante) => {
            let (repouso, hover, selecionado) = match variante {
                TableVariant::Default => (None, p.row_hover, p.row_selected),
                TableVariant::Card => (Some(p.card.hsla()), p.card_row_hover, p.card_row_selected),
            };
            if selected {
                // O selecionado NÃO muda sob o ponteiro: o estado é informação, o hover é só
                // afordância.
                RowFill {
                    rest: Some(selecionado.hsla()),
                    hover: Some(selecionado.hsla()),
                }
            } else {
                RowFill {
                    rest: repouso,
                    hover: Some(hover.hsla()),
                }
            }
        }
    }
}

/// O respiro `(topo, base)` da **legenda** — `mt-4` na variante padrão, `my-4` na card.
///
/// A card ganha o respiro de baixo porque ali a legenda fica entre duas superfícies (o card acima e o
/// que vier depois), e sem ele encostaria na de baixo.
fn caption_margin(variant: TableVariant) -> (f32, f32) {
    match variant {
        TableVariant::Default => (CAPTION_MARGIN, 0.0),
        TableVariant::Card => (CAPTION_MARGIN, CAPTION_MARGIN),
    }
}

/// A altura TOTAL de uma linha, filete incluído — o número que a janela tem que medir.
///
/// Existe só pros testes: no render nenhuma linha de corpo ou rodapé tem altura declarada (ela vem
/// do conteúdo, como numa tabela de verdade), então esta é a fórmula que o layout deve REPRODUZIR, e
/// não uma que ele consome. É ela que documenta o custo de [`LINE_HEIGHT`].
#[cfg(test)]
fn row_height(slot: Slot, variant: TableVariant, first_row: bool, last_row: bool) -> f32 {
    let b = row_borders(slot, variant, first_row, last_row);
    match slot {
        // O `h-10` é altura total (border-box): o filete de baixo cabe dentro dos 40.
        Slot::Head => HEAD_HEIGHT,
        _ => {
            let p = cell_pad(slot, variant, false, false, false);
            p.top + LINE_HEIGHT + p.bottom + b.top + b.bottom
        }
    }
}

// =================================================================================================
// O bisel
// =================================================================================================

/// O **fio de bisel** de 1px do card, sobreposto ao corpo.
///
/// ⚠️ As duas armadilhas da casa, as mesmas do [`crate::card`] e do [`crate::Toggle`]:
///
/// 1. **É borda, não sombra.** O `Window::paint_shadows` do GPUI não recorta a sombra pra fora do
///    elemento que a projeta (diferente do CSS), então o `before:box-shadow` da referência viraria
///    uma lavagem de cor sobre a tabela inteira.
/// 2. **Cobre a moldura, e é por isso que o raio não perde 1px.** Na referência o pseudo-elemento
///    está em `inset: 1px` (por DENTRO da moldura, daí o `calc(--radius-xl - 1px)`) e a sombra dele
///    sai 1px pra fora — ou seja ela cai EM CIMA da moldura. Um filete 1px mais pra dentro criaria
///    uma segunda linha ao lado da moldura, e a 4–16% de alfa isso é praticamente invisível mas
///    engorda a borda. Aqui o overlay fica em `inset: 0` (a moldura é das LINHAS, então o limite do
///    corpo já é ela) e o raio é o [`RADIUS`] cheio.
///
/// O lado sai do **sinal** de [`TablePalette::bevel_dir`], nunca de um segundo campo que poderia
/// divergir dele.
fn bevel_overlay(p: &TablePalette) -> gpui::Div {
    let base = div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .rounded(px(RADIUS))
        .border_color(p.bevel.hsla());
    if p.bevel_dir > 0.0 {
        base.border_b(px(BORDER))
    } else {
        base.border_t(px(BORDER))
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// A tabela de dados. Ver o doc do módulo.
///
/// É um **elemento de render** ([`RenderOnce`]): construa a cada frame, e passe a seleção por
/// [`TableRow::selected`].
#[derive(IntoElement)]
pub struct Table {
    /// Precisa ser estável entre frames: é a chave do `element_state` que guarda o deslocamento de
    /// rolagem horizontal. Um id que muda a cada frame faz a tabela voltar ao início.
    id: ElementId,
    variant: TableVariant,
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    footer: Option<TableRow>,
    caption: Option<SharedString>,
    /// Handle de rolagem horizontal de quem chama. Sem ele o deslocamento vive no `element_state` do
    /// embrulho (que é o suficiente pra rolar com a roda); com ele, quem usa consegue LER e MEXER na
    /// posição — e é o que permite medir o curso num teste.
    scroll: Option<gpui::ScrollHandle>,
}

impl Table {
    /// Uma tabela nova. Ver [`Self::id`] sobre a estabilidade do `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: TableVariant::default(),
            columns: Vec::new(),
            rows: Vec::new(),
            footer: None,
            caption: None,
            scroll: None,
        }
    }

    /// Liga a rolagem horizontal a um [`gpui::ScrollHandle`] de fora, pra ler a posição
    /// (`offset`/`max_offset`) ou rolar por código.
    ///
    /// Opcional: sem ele a roda do mouse já rola (o deslocamento fica no `element_state` do
    /// embrulho, que existe porque [`Self::new`] exige um `id`).
    pub fn track_scroll(mut self, handle: &gpui::ScrollHandle) -> Self {
        self.scroll = Some(handle.clone());
        self
    }

    pub fn variant(mut self, variant: TableVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Acrescenta uma coluna. A ordem de chamada é a ordem na tela.
    pub fn column(mut self, column: TableColumn) -> Self {
        self.columns.push(column);
        self
    }

    /// Acrescenta uma linha de corpo.
    pub fn row(mut self, row: TableRow) -> Self {
        self.rows.push(row);
        self
    }

    /// A linha de rodapé (uma só — ver "Ausente" no doc do módulo).
    pub fn footer(mut self, footer: TableRow) -> Self {
        self.footer = Some(footer);
        self
    }

    /// A legenda, embaixo da tabela (`caption-bottom`).
    pub fn caption(mut self, caption: impl Into<SharedString>) -> Self {
        self.caption = Some(caption.into());
        self
    }
}

/// Uma linha montada: a faixa de flex, com as células nas larguras das colunas.
///
/// `first_row`/`last_row` são a posição DENTRO da fatia — é o que os seletores `first:`/`last:`/
/// `[&_tr:last-child]` da referência leem de fora.
#[allow(clippy::too_many_arguments)]
fn row_element(
    slot: Slot,
    variant: TableVariant,
    columns: &[TableColumn],
    cells: Vec<AnyElement>,
    selected: bool,
    first_row: bool,
    last_row: bool,
    p: &'static TablePalette,
) -> gpui::Div {
    let b = row_borders(slot, variant, first_row, last_row);
    let r = row_radius(slot, variant, first_row, last_row);
    let fill = row_fill(slot, variant, selected, p);

    let mut linha = div()
        // `relative` porque o corpo pode receber overlays (o bisel é irmão, mas quem usa pode pôr
        // algo dentro da linha, que é o que o `relative` de `TableRow` na referência serve).
        .relative()
        .flex()
        // `align-middle`: com uma célula mais alta que as outras (um avatar, um badge), as baixas
        // ficam centradas nela em vez de subirem pro topo.
        .items_center()
        .when(slot == Slot::Head, |d| d.h(px(HEAD_HEIGHT)))
        .border_color(p.border.hsla())
        .border_t(px(b.top))
        .border_r(px(b.right))
        .border_b(px(b.bottom))
        .border_l(px(b.left))
        .rounded_tl(px(r.tl))
        .rounded_tr(px(r.tr))
        .rounded_bl(px(r.bl))
        .rounded_br(px(r.br));

    if slot != Slot::Body {
        // Cabeçalho e rodapé são `font-medium` na referência (`TableHead` e `TableFooter`), e o
        // cabeçalho é também o único com texto esmaecido.
        linha = linha.font_weight(gpui::FontWeight::MEDIUM);
    }
    if slot == Slot::Head {
        linha = linha.text_color(p.text_muted.hsla());
    }
    if let Some(bg) = fill.rest {
        linha = linha.bg(bg);
    }
    if let Some(hover) = fill.hover {
        linha = linha.hover(move |s| s.bg(hover));
    }

    let total = columns.len();
    let mut cells = cells.into_iter();
    for (i, col) in columns.iter().enumerate() {
        let pad = cell_pad(
            slot,
            variant,
            i == 0,
            i + 1 == total,
            col.width == TableColumnWidth::Checkbox,
        );
        let mut celula = div()
            .flex()
            .items_center()
            .when(col.align == TableAlign::End, |d| d.justify_end())
            .pt(px(pad.top))
            .pr(px(pad.right))
            .pb(px(pad.bottom))
            .pl(px(pad.left))
            // `whitespace-nowrap` da referência, mais a elipse (superset — ver o doc do módulo):
            // com a largura declarada, o texto que não cabe precisa de um fim visível.
            .whitespace_nowrap()
            .text_ellipsis()
            // ⚠️ Recorte RETANGULAR, e aqui é o que se quer: a célula não tem canto arredondado (o
            // raio é da linha), então não há a armadilha do `overflow_hidden` descrita no doc do
            // módulo. Ele carrega DUAS coisas: o texto longo vira elipse em vez de vazar pra a coluna
            // vizinha, e — ver o ramo `Flex` abaixo — o mínimo automático do item de flex vai a zero,
            // que é o que deixa a coluna encolher e as linhas ficarem alinhadas.
            .overflow_hidden();
        celula = match col.width {
            // ⚠️ Sem `flex_none`, e ele não falta. O `flex_shrink` do GPUI é 1.0 por default, mas a
            // linha nunca fica menor que a soma das colunas fixas — é o que o `min_w` da tabela
            // garante (ver o comentário do `min_total` no `render`) — então não existe aperto pra
            // impedir. Medido: pôr ou tirar o `flex_none` não muda o layout em nenhum dos testes de
            // janela, e código que não faz nada é código que mente sobre o que sustenta o desenho.
            TableColumnWidth::Fixed(w) => celula.w(px(w.max(0.0))),
            // Colapsa até o conteúdo: `flex_none` sem largura declarada.
            TableColumnWidth::Checkbox => celula.flex_none(),
            // `flex_basis(0)`: sem ele o tamanho-base do item é o do CONTEÚDO, e o mínimo de
            // conteúdo de um texto no GPUI é a FRASE INTEIRA numa linha (o
            // `request_measured_layout` do `TextElement` só quebra com largura definida). A coluna
            // então cresceria com o texto e as linhas sairiam desalinhadas entre si — a mesma
            // armadilha do `crate::card`.
            //
            // ⚠️ Não há `min_w(0)` aqui, e ele não falta: o `overflow_hidden` acima já zera o
            // **tamanho mínimo automático** do item de flex (é a regra do CSS que diz que um item
            // com `overflow` diferente de `visible` tem mínimo automático 0 em vez de min-content, e
            // o taffy a implementa). Um `min_w(0)` seria redundante — medido: removê-lo não muda o
            // layout, remover o `overflow_hidden` desalinha. Ver
            // `tests_de_janela::colunas_flex_ficam_alinhadas_entre_linhas`.
            TableColumnWidth::Flex => celula.flex_grow().flex_shrink().flex_basis(px(0.0)),
        };
        if let Some(c) = cells.next() {
            celula = celula.child(c);
        }
        linha = linha.child(celula);
    }

    linha
}

impl RenderOnce for Table {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = palette();
        let Table {
            id,
            variant,
            columns,
            rows,
            footer,
            caption,
            scroll,
        } = self;

        // **A largura mínima da tabela: a soma das colunas de largura fixa.**
        //
        // Ela carrega duas coisas, e é bom saber as duas antes de mexer:
        //
        // 1. **O curso de rolagem.** O `content_size` que o embrulho usa pra calcular
        //    `padded_content_size − bounds` vem do tamanho da CAIXA do filho, não do que transborda
        //    dentro dele. Esticada à largura do embrulho, a tabela mede o embrulho, o curso dá zero e
        //    nada rola — as colunas transbordariam pra fora, visíveis e inalcançáveis. Com o `min_w`
        //    a caixa da tabela passa a ser a largura real das colunas, e aí há curso.
        // 2. **As colunas fixas não encolhem.** O `flex_shrink` do GPUI é 1.0 por default, então sem
        //    o `min_w` uma tabela mais larga que a caixa apertaria as colunas — exatamente o que o
        //    contrato de larguras declaradas promete não acontecer.
        //
        // É por isso que as colunas fixas NÃO precisam de `flex_none`: com o `min_w` valendo, a soma
        // dos tamanhos-base dos itens nunca passa da largura da linha, então o `flex_shrink` de 1.0
        // que o GPUI dá por default nunca é exercido. Uma peça só sustenta as duas coisas.
        //
        // Ver `TableColumnWidth::min_width` e os testes `a_tabela_estreita_tem_curso_de_rolagem` e
        // `as_larguras_das_colunas_nao_sao_apertadas_pelo_container`.
        let min_total: f32 = columns.iter().map(|c| c.width.min_width()).sum();

        let mut tabela = div()
            .flex()
            .flex_col()
            // ⚠️ **Aqui NÃO há `w_full`, e a ausência é a decisão.** O `w-full` do `<table>` da
            // referência é entregue pelo **esticamento de eixo cruzado** do embrulho, que é uma
            // coluna de flex — ver o comentário do embrulho no fim deste `render`.
            //
            // Um `w_full` aqui é uma PORCENTAGEM, e porcentagem não sobrevive ao embrulho de
            // rolagem: quando o taffy mede a árvore sob espaço indefinido — o que acontece se
            // qualquer ancestral tirar a largura do flex e alguma tabela irmã for mais larga que a
            // coluna — ela cai pra `width: auto` = tamanho do conteúdo, com o `min_w` abaixo como
            // único piso. Foi o defeito: embrulho 738, tabela 126, coluna `Flex` em ZERO. O
            // esticamento resolve do tamanho definido do pai e não tem esse buraco. Medido: com o
            // embrulho em coluna de flex, pôr ou tirar o `w_full` daqui não muda NENHUM dos testes
            // de janela — e código que não faz nada é código que mente sobre o que sustenta o
            // desenho.
            .when(min_total > 0.0, |d| d.min_w(px(min_total)))
            .text_size(px(TEXT_SIZE))
            // ⚠️ `leading-none`. Ver `LINE_HEIGHT`: sem isto a tabela inteira cresce ~62% em altura.
            .line_height(px(LINE_HEIGHT))
            .text_color(p.text.hsla());

        // O cabeçalho só existe se ALGUMA coluna tem rótulo — senão sairia uma faixa de 40px vazia
        // com um filete embaixo.
        if columns.iter().any(|c| c.header.is_some()) {
            let rotulos: Vec<AnyElement> = columns
                .iter()
                .map(|c| match c.header.clone() {
                    Some(t) => div().child(t).into_any_element(),
                    None => div().into_any_element(),
                })
                .collect();
            tabela = tabela.child(row_element(
                Slot::Head,
                variant,
                &columns,
                rotulos,
                false,
                true,
                true,
                p,
            ));
        }

        if !rows.is_empty() {
            let total = rows.len();
            let mut corpo = div().relative().flex().flex_col();
            if variant == TableVariant::Card {
                // O raio e a sombra ficam no CORPO (e os raios de canto se repetem nas linhas das
                // pontas, que é quem pinta o fundo): a sombra do GPUI é desenhada com os raios do
                // elemento que a projeta, então sem o raio aqui ela sairia quadrada em volta de um
                // card arredondado.
                corpo = corpo.rounded(px(RADIUS)).shadow(vec![gpui::BoxShadow {
                    color: p.shadow.hsla(),
                    offset: point(px(0.0), px(1.0)),
                    blur_radius: px(2.0),
                    spread_radius: px(0.0),
                }]);
            }
            for (i, r) in rows.into_iter().enumerate() {
                corpo = corpo.child(row_element(
                    Slot::Body,
                    variant,
                    &columns,
                    r.cells,
                    r.selected,
                    i == 0,
                    i + 1 == total,
                    p,
                ));
            }
            if variant == TableVariant::Card {
                // Por último: é absoluto, então a ordem só decide quem pinta em cima.
                corpo = corpo.child(bevel_overlay(p));
            }
            tabela = tabela.child(corpo);
        }

        if let Some(f) = footer {
            tabela = tabela.child(row_element(
                Slot::Footer,
                variant,
                &columns,
                f.cells,
                false,
                true,
                true,
                p,
            ));
        }

        if let Some(c) = caption {
            let (mt, mb) = caption_margin(variant);
            tabela = tabela.child(
                div()
                    .mt(px(mt))
                    .mb(px(mb))
                    .text_size(px(TEXT_SIZE))
                    // A legenda é a única fatia SEM `leading-none` — ver `CAPTION_LINE_HEIGHT`.
                    .line_height(px(CAPTION_LINE_HEIGHT))
                    .text_color(p.text_muted.hsla())
                    .child(c),
            );
        }

        // O embrulho: `relative w-full overflow-x-auto`. O `id` é o que dá `element_state` ao
        // deslocamento de rolagem — ver a seção sobre rolagem no doc do módulo.
        div()
            .id(id)
            .relative()
            .w_full()
            // ⚠️ **`flex_col` é CARGA ESTRUTURAL, e não estilo — é ele que dá a largura à tabela.**
            //
            // Uma coluna de flex com um item só, e `align-items: stretch` (o default) esticando esse
            // item à largura da coluna: é assim que o `w-full` do `<table>` da referência chega na
            // tabela. Ver o comentário no topo da raiz, acima.
            //
            // Em BLOCO — que é o que este embrulho era — não funcionava, e nem com `w_full` na
            // tabela nem sem: a porcentagem não resolve e o `auto` do bloco também não estica dentro
            // de um `overflow` de rolagem. O que sai é o tamanho do CONTEÚDO, com o `min_w` das
            // colunas fixas como único piso. Foi o defeito relatado, e é por isso que ele só
            // aparecia em algumas tabelas: com 8 colunas fixas o piso era 1120 e ninguém notava; com
            // `Checkbox + Flex + Fixed(80)` o piso era 80 e a tabela saía com ~110px, aparando o
            // texto de toda célula.
            //
            // Travado em `tests_de_janela::a_tabela_ocupa_a_largura_com_uma_irma_mais_larga_na_pagina`,
            // que traz junto as quatro peças necessárias pra o defeito aparecer.
            .flex()
            .flex_col()
            .overflow_x_scroll()
            .when_some(scroll, |d, h| d.track_scroll(&h))
            .child(tabela)
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `card.rs`, `toggle.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::color::opaque;

    /// Atalho: a paleta clara e a escura, pra varrer as duas em todo teste de cor.
    const AMBAS: [(&str, &TablePalette); 2] =
        [("claro", &TABLE_LIGHT), ("escuro", &TABLE_DARK)];

    // --- Geometria --------------------------------------------------------------------------------

    /// As medidas da referência, na escala do Tailwind (1 unidade = 4px).
    #[test]
    fn medidas_sao_as_da_referencia() {
        assert_eq!(TEXT_SIZE, 14.0, "text-sm");
        assert_eq!(HEAD_HEIGHT, 40.0, "h-10");
        assert_eq!(PAD, 10.0, "px-2.5 / p-2.5 = spacing(2.5)");
        assert_eq!(FOOTER_PAD_Y, 14.0, "py-3.5 = spacing(3.5)");
        assert_eq!(CAPTION_MARGIN, 16.0, "mt-4 / my-4");
        assert_eq!(BORDER, 1.0, "border");
        // `rounded-xl` = --radius-xl = calc(--radius + 4px), com --radius = 0.625rem = 10px.
        assert_eq!(RADIUS, 14.0, "--radius-xl = 10 + 4");
        // O respiro das pontas do card desconta a moldura, e a soma tem que FECHAR o --spacing.
        assert_eq!(CARD_EDGE_PAD, 9.0, "calc(--spacing(2.5) - 1px)");
        assert_eq!(
            CARD_EDGE_PAD + BORDER,
            PAD,
            "respiro da ponta + moldura tem que fechar o respiro ótico das outras células"
        );
    }

    /// ⚠️ **`leading-none` é entrelinha IGUAL ao corpo**, e a legenda é a única exceção.
    ///
    /// Sem declarar, o GPUI usa a razão de ouro e cada linha da tabela engorda 8,65px. A altura
    /// medida de verdade está em `tests_de_janela::entrelinha_apertada_mantem_a_altura_da_linha`;
    /// aqui ficam os números.
    #[test]
    fn entrelinhas_sao_a_apertada_e_o_par_do_text_sm() {
        assert_eq!(LINE_HEIGHT, TEXT_SIZE, "leading-none = 1.0 × o corpo");
        assert_eq!(CAPTION_LINE_HEIGHT, 20.0, "o par do text-sm, na legenda");
        assert_ne!(
            LINE_HEIGHT, CAPTION_LINE_HEIGHT,
            "a legenda NÃO tem leading-none na referência — se as duas empatarem, uma está errada"
        );

        const RAZAO_DE_OURO: f32 = 1.618_034;
        let default_do_gpui = TEXT_SIZE * RAZAO_DE_OURO;
        assert!(
            default_do_gpui > LINE_HEIGHT + 8.0,
            "sem fixar, a linha sairia {default_do_gpui:.2}px — {:.0}% a mais",
            (default_do_gpui / LINE_HEIGHT - 1.0) * 100.0
        );
        // E o custo por linha de corpo: 2 × 10 de respiro + entrelinha.
        let apertada = 2.0 * PAD + LINE_HEIGHT;
        let solta = 2.0 * PAD + default_do_gpui;
        assert_eq!(apertada, 34.0, "linha de corpo: 10 + 14 + 10");
        assert!(
            solta - apertada > 8.0,
            "cada linha ganharia {:.2}px",
            solta - apertada
        );
    }

    /// A altura de cada fatia, do respiro mais a entrelinha mais os filetes.
    #[test]
    fn alturas_das_fatias_saem_do_respiro_e_da_entrelinha() {
        // Cabeçalho: `h-10` é altura TOTAL (border-box), o filete de baixo cabe dentro.
        assert_eq!(
            row_height(Slot::Head, TableVariant::Default, true, true),
            40.0
        );
        // Corpo, padrão: 10 + 14 + 10 = 34, mais o filete de baixo nas que não são a última.
        assert_eq!(
            row_height(Slot::Body, TableVariant::Default, true, false),
            35.0,
            "linha do meio: 34 + filete"
        );
        assert_eq!(
            row_height(Slot::Body, TableVariant::Default, false, true),
            34.0,
            "a última perde o filete ([&_tr:last-child]:border-0)"
        );
        // Corpo, card: a primeira tem filete em cima E embaixo (a moldura do card).
        assert_eq!(
            row_height(Slot::Body, TableVariant::Card, true, false),
            36.0,
            "primeira linha do card: 34 + moldura de cima + filete de baixo"
        );
        assert_eq!(
            row_height(Slot::Body, TableVariant::Card, false, true),
            35.0,
            "a última do card MANTÉM o filete: é a base do card"
        );
        // Rodapé: `py-3.5` engorda 8px em relação ao corpo.
        assert_eq!(
            row_height(Slot::Footer, TableVariant::Default, true, true),
            43.0,
            "14 + 14 + 14 + o filete de cima do tfoot"
        );
        assert_eq!(
            row_height(Slot::Footer, TableVariant::Card, true, true),
            42.0,
            "no card o rodapé não tem filete nenhum"
        );
    }

    // --- Respiro ----------------------------------------------------------------------------------

    /// O respiro de cada fatia, e o que o rodapé troca.
    #[test]
    fn respiro_muda_de_fatia_pra_fatia() {
        let v = TableVariant::Default;

        // Cabeçalho: só horizontal — a altura vem do `h-10`.
        let head = cell_pad(Slot::Head, v, true, false, false);
        assert_eq!((head.top, head.bottom), (0.0, 0.0), "px-2.5 e nada vertical");
        assert_eq!((head.left, head.right), (PAD, PAD));

        // Corpo: `p-2.5` nos quatro lados.
        let body = cell_pad(Slot::Body, v, false, false, false);
        assert_eq!(
            body,
            Edges {
                top: PAD,
                right: PAD,
                bottom: PAD,
                left: PAD
            },
            "p-2.5"
        );

        // Rodapé: `py-3.5` troca SÓ o vertical.
        let foot = cell_pad(Slot::Footer, v, false, false, false);
        assert_eq!((foot.top, foot.bottom), (FOOTER_PAD_Y, FOOTER_PAD_Y));
        assert_eq!(
            (foot.left, foot.right),
            (PAD, PAD),
            "o horizontal do rodapé continua px-2.5"
        );
        assert!(foot.top > body.top, "o rodapé respira mais que o corpo");
    }

    /// **Na variante card as células da ponta perdem 1px**, porque ali existe a moldura. Vale nas
    /// três fatias — o `in-data-[variant=card]` da referência olha o contêiner, não a fatia.
    #[test]
    fn variante_card_desconta_a_moldura_do_respiro_das_pontas() {
        // ⚠️ O valor esperado é LITERAL, e não `CARD_EDGE_PAD`. Com a constante dos dois lados o
        // teste seria uma tautologia: mudar `CARD_EDGE_PAD` pra 10 (ou seja, esquecer o `-1px` do
        // `calc`) passaria — foi exatamente o que a mutação-verificação pegou aqui.
        const ESPERADO: f32 = 9.0;
        assert_eq!(CARD_EDGE_PAD, ESPERADO, "calc(--spacing(2.5) - 1px) = 10 − 1");
        assert_eq!(
            CARD_EDGE_PAD,
            PAD - BORDER,
            "e o 9 tem que ser o respiro base MENOS a moldura, não um número solto"
        );

        // O desconto é do CORPO, que é a fatia que tem moldura.
        let primeira = cell_pad(Slot::Body, TableVariant::Card, true, false, false);
        let meio = cell_pad(Slot::Body, TableVariant::Card, false, false, false);
        let ultima = cell_pad(Slot::Body, TableVariant::Card, false, true, false);

        assert_eq!(primeira.left, ESPERADO, "first:ps-[calc(…)]");
        assert_eq!(primeira.right, PAD, "o lado de dentro não muda");
        assert_eq!(ultima.right, ESPERADO, "last:pe-[calc(…)]");
        assert_eq!(ultima.left, PAD);
        assert_eq!(meio.left, PAD, "as do meio não têm moldura ao lado");
        assert_eq!(meio.right, PAD);

        // ⚠️ Cabeçalho e rodapé NÃO descontam: eles ficam fora do card, e sem moldura o desconto
        // desalinharia o rótulo (ou o total) 1px à esquerda da coluna. O `TableHead` da referência
        // concorda; o `TableCell` dela aplicaria no rodapé, e é aí que está o desvio declarado.
        for slot in [Slot::Head, Slot::Footer] {
            let p = cell_pad(slot, TableVariant::Card, true, true, false);
            assert_eq!(
                (p.left, p.right),
                (PAD, PAD),
                "{slot:?} fica fora do card: respiro cheio, sem desconto"
            );
        }

        // E a variante padrão NÃO desconta em fatia nenhuma: lá não há moldura.
        for slot in [Slot::Head, Slot::Body, Slot::Footer] {
            let padrao = cell_pad(slot, TableVariant::Default, true, true, false);
            assert_eq!((padrao.left, padrao.right), (PAD, PAD), "{slot:?}: padrão");
        }
    }

    /// ⚠️ **O respiro ÓTICO da ponta é o mesmo em todas as fatias e nas duas variantes.**
    ///
    /// É o invariante que o desconto de 1px existe pra manter: o que o olho vê é
    /// `respiro + moldura`, e ele tem que dar [`PAD`] sempre. Se não der, o rótulo da coluna sai
    /// desalinhado do conteúdo dela — 1px, o bastante pra parecer torto numa tabela e não o bastante
    /// pra alguém achar a causa.
    ///
    /// Este teste é o que pegou o erro de aplicar o desconto no cabeçalho, que a referência não faz.
    #[test]
    fn respiro_otico_da_ponta_e_igual_em_todas_as_fatias() {
        for variant in [TableVariant::Default, TableVariant::Card] {
            for slot in [Slot::Head, Slot::Body, Slot::Footer] {
                // A linha de corpo do meio é a que tem moldura lateral nas duas pontas; pras outras
                // fatias a posição não muda o resultado.
                let b = row_borders(slot, variant, false, false);
                let p = cell_pad(slot, variant, true, false, false);
                assert_eq!(
                    p.left + b.left,
                    PAD,
                    "{variant:?}/{slot:?}: respiro ({}) + moldura ({}) tem que dar {PAD}",
                    p.left,
                    b.left
                );

                let p = cell_pad(slot, variant, false, true, false);
                assert_eq!(
                    p.right + b.right,
                    PAD,
                    "{variant:?}/{slot:?}: idem do lado direito"
                );
            }
        }
    }

    /// **A coluna de caixa de seleção perde o respiro do lado de DENTRO** —
    /// `first:has-[[role=checkbox]]:pe-0` e `last:has-[[role=checkbox]]:ps-0`. Trocar os dois de lado
    /// é o erro fácil (`ps` é o início, ou seja a ESQUERDA em LTR), e daria uma caixa de seleção
    /// descolada da borda.
    #[test]
    fn coluna_de_checkbox_perde_o_respiro_de_dentro() {
        let v = TableVariant::Default;

        let primeira = cell_pad(Slot::Body, v, true, false, true);
        assert_eq!(primeira.left, PAD, "encostada na borda esquerda? não: ps fica");
        assert_eq!(primeira.right, 0.0, "first:pe-0 — some o respiro de dentro");

        let ultima = cell_pad(Slot::Body, v, false, true, true);
        assert_eq!(ultima.left, 0.0, "last:ps-0");
        assert_eq!(ultima.right, PAD);

        // No meio, sem `first`/`last`, nenhuma das duas regras vale.
        let meio = cell_pad(Slot::Body, v, false, false, true);
        assert_eq!((meio.left, meio.right), (PAD, PAD));

        // A regra do checkbox vem DEPOIS da do card: primeira coluna de uma tabela card fica com 9
        // à esquerda (a moldura) e 0 à direita (a caixa de seleção).
        let card = cell_pad(Slot::Body, TableVariant::Card, true, false, true);
        assert_eq!((card.left, card.right), (CARD_EDGE_PAD, 0.0));
    }

    /// A largura mínima que cada tipo de coluna contribui — é a soma disso que dá curso de rolagem.
    #[test]
    fn so_coluna_fixa_contribui_pra_largura_minima() {
        assert_eq!(TableColumnWidth::Fixed(120.0).min_width(), 120.0);
        assert_eq!(TableColumnWidth::Flex.min_width(), 0.0, "flex encolhe");
        assert_eq!(
            TableColumnWidth::Checkbox.min_width(),
            0.0,
            "a do checkbox é a do conteúdo, que o taffy soma sozinho"
        );
        // Largura negativa não vira mínimo negativo (que encolheria a tabela).
        assert_eq!(TableColumnWidth::Fixed(-10.0).min_width(), 0.0);
        // E o default de uma coluna é Flex — o que mais se parece com uma tabela simples.
        assert_eq!(TableColumn::new("x").width, TableColumnWidth::Flex);
    }

    // --- Filetes ----------------------------------------------------------------------------------

    /// **Os filetes da variante padrão**: cabeçalho embaixo, linhas embaixo menos a última, rodapé em
    /// cima.
    #[test]
    fn filetes_da_variante_padrao() {
        let v = TableVariant::Default;

        let head = row_borders(Slot::Head, v, true, true);
        assert_eq!(
            head,
            Edges {
                bottom: BORDER,
                ..Edges::default()
            },
            "cabeçalho: [&_tr]:border-b e nada mais"
        );

        let meio = row_borders(Slot::Body, v, false, false);
        assert_eq!(meio.bottom, BORDER, "linha do meio: border-b");
        assert_eq!((meio.top, meio.left, meio.right), (0.0, 0.0, 0.0));

        let ultima = row_borders(Slot::Body, v, false, true);
        assert_eq!(
            ultima,
            Edges::default(),
            "a última perde tudo — [&_tr:last-child]:border-0"
        );

        let foot = row_borders(Slot::Footer, v, true, true);
        assert_eq!(foot.top, BORDER, "rodapé: border-t no tfoot");
        assert_eq!(foot.bottom, 0.0, "[&>tr]:last:border-b-0");
    }

    /// **Os filetes da variante card formam a moldura**: laterais em toda linha, topo só na primeira,
    /// base em TODAS — inclusive a última, que é a base do card.
    #[test]
    fn filetes_da_variante_card_formam_a_moldura() {
        let v = TableVariant::Card;

        let primeira = row_borders(Slot::Body, v, true, false);
        assert_eq!(
            primeira,
            Edges {
                top: BORDER,
                right: BORDER,
                bottom: BORDER,
                left: BORDER
            },
            "a primeira fecha o card por cima"
        );

        let meio = row_borders(Slot::Body, v, false, false);
        assert_eq!(meio.top, 0.0, "o meio NÃO repete o topo: daria 2px na junção");
        assert_eq!((meio.left, meio.right, meio.bottom), (BORDER, BORDER, BORDER));

        let ultima = row_borders(Slot::Body, v, false, true);
        assert_eq!(
            ultima.bottom, BORDER,
            "⚠️ a última MANTÉM a base: o [&_tr:last-child]:border-0 é do tr, e no card a borda é do td"
        );
        assert_eq!(ultima.top, 0.0);

        // Uma linha só: primeira E última, então a moldura fecha nos quatro lados.
        let sozinha = row_borders(Slot::Body, v, true, true);
        assert_eq!(
            sozinha,
            Edges {
                top: BORDER,
                right: BORDER,
                bottom: BORDER,
                left: BORDER
            }
        );

        // O rodapé fica FORA do card: `in-data-[variant=card]:border-none`.
        assert_eq!(
            row_borders(Slot::Footer, v, true, true),
            Edges::default(),
            "rodapé do card: sem filete nenhum"
        );
        // O cabeçalho mantém o filete nas duas variantes.
        assert_eq!(
            row_borders(Slot::Head, v, true, true).bottom,
            BORDER,
            "o cabeçalho não muda com a variante"
        );
    }

    // --- Raios ------------------------------------------------------------------------------------

    /// **Só o corpo da variante card arredonda**, e os cantos saem da posição da linha.
    #[test]
    fn raios_saem_da_posicao_da_linha_no_card() {
        let v = TableVariant::Card;

        let primeira = row_radius(Slot::Body, v, true, false);
        assert_eq!(
            primeira,
            Radii {
                tl: RADIUS,
                tr: RADIUS,
                bl: 0.0,
                br: 0.0
            },
            "rounded-ss-xl + rounded-se-xl"
        );

        let ultima = row_radius(Slot::Body, v, false, true);
        assert_eq!(
            ultima,
            Radii {
                tl: 0.0,
                tr: 0.0,
                bl: RADIUS,
                br: RADIUS
            },
            "rounded-es-xl + rounded-ee-xl"
        );

        assert_eq!(
            row_radius(Slot::Body, v, false, false),
            Radii::default(),
            "as do meio são retas"
        );

        // ⚠️ Uma linha só é primeira E última: os quatro cantos. É o caso que um if/else erraria.
        let sozinha = row_radius(Slot::Body, v, true, true);
        assert_eq!(
            (sozinha.tl, sozinha.tr, sozinha.bl, sozinha.br),
            (RADIUS, RADIUS, RADIUS, RADIUS),
            "tabela de uma linha: card arredondado nos quatro cantos"
        );

        // Cabeçalho e rodapé ficam fora do card, então não arredondam.
        for slot in [Slot::Head, Slot::Footer] {
            assert_eq!(
                row_radius(slot, v, true, true),
                Radii::default(),
                "{slot:?} fica FORA do card"
            );
        }
        // E a variante padrão não arredonda nada.
        for slot in [Slot::Head, Slot::Body, Slot::Footer] {
            assert_eq!(
                row_radius(slot, TableVariant::Default, true, true),
                Radii::default(),
                "{slot:?}: a variante padrão não tem superfície"
            );
        }
    }

    // --- Fundos -----------------------------------------------------------------------------------

    /// **O `color-mix(--card, tinta 2%)` é do RODAPÉ, não do cabeçalho.** É a troca fácil de fazer
    /// (um cabeçalho tingido é o visual mais comum de tabela), e a referência não a faz: o
    /// `TableHeader` dela tem só `[&_tr]:border-b`.
    #[test]
    fn o_fundo_tingido_e_do_rodape_e_nao_do_cabecalho() {
        for (nome, p) in AMBAS {
            let head = row_fill(Slot::Head, TableVariant::Default, false, p);
            assert!(head.rest.is_none(), "{nome}: cabeçalho SEM fundo");
            let foot = row_fill(Slot::Footer, TableVariant::Default, false, p);
            assert_eq!(
                foot.rest,
                Some(p.footer_bg.hsla()),
                "{nome}: o tingido é o do rodapé"
            );

            // E na variante card o rodapé também perde o fundo (`bg-transparent`).
            let foot_card = row_fill(Slot::Footer, TableVariant::Card, false, p);
            assert!(foot_card.rest.is_none(), "{nome}: rodapé do card é transparente");
            assert!(
                row_fill(Slot::Head, TableVariant::Card, false, p).rest.is_none(),
                "{nome}: cabeçalho do card também"
            );
        }
    }

    /// **A variante padrão é transparente em repouso; a card tem `bg-card`.** É a diferença que faz
    /// as duas serem duas.
    #[test]
    fn repouso_e_transparente_no_padrao_e_card_na_variante_card() {
        for (nome, p) in AMBAS {
            let padrao = row_fill(Slot::Body, TableVariant::Default, false, p);
            assert!(
                padrao.rest.is_none(),
                "{nome}: a variante padrão não tem superfície"
            );

            let card = row_fill(Slot::Body, TableVariant::Card, false, p);
            assert_eq!(card.rest, Some(p.card.hsla()), "{nome}: bg-card");
            // E o fundo do card é OPACO — é ele que esconde o que houver atrás.
            assert_eq!(p.card.alpha(), 1.0, "{nome}: --card é opaco");
        }
    }

    /// **O hover mistura com o token que está ATRÁS**: `--background` na variante padrão (não há
    /// superfície) e `--card` na card. Resolver as duas pro mesmo valor é o erro que o tema escuro
    /// denuncia.
    #[test]
    fn hover_mistura_com_o_token_que_esta_atras() {
        for (nome, p) in AMBAS {
            let padrao = row_fill(Slot::Body, TableVariant::Default, false, p);
            assert_eq!(padrao.hover, Some(p.row_hover.hsla()), "{nome}: --background");

            let card = row_fill(Slot::Body, TableVariant::Card, false, p);
            assert_eq!(card.hover, Some(p.card_row_hover.hsla()), "{nome}: --card");
        }

        // No CLARO os dois tokens são brancos, então os dois hovers empatam.
        assert_eq!(
            TABLE_LIGHT.row_hover, TABLE_LIGHT.card_row_hover,
            "no claro --background == --card == #ffffff"
        );
        // No ESCURO eles se separam, e é isso que prova que são dois tokens e não um.
        assert_ne!(
            TABLE_DARK.row_hover, TABLE_DARK.card_row_hover,
            "no escuro --background (#141414) e --card (#191919) diferem — se empatarem aqui, um \
             dos dois foi resolvido contra o token errado"
        );
    }

    /// **O hover é só das linhas do CORPO.** Desvio consciente: na referência ele está na classe base
    /// de `TableRow`, que serve também cabeçalho e rodapé. Ver o doc do módulo.
    #[test]
    fn hover_e_so_das_linhas_do_corpo() {
        for (nome, p) in AMBAS {
            for v in [TableVariant::Default, TableVariant::Card] {
                assert!(
                    row_fill(Slot::Head, v, false, p).hover.is_none(),
                    "{nome}/{v:?}: cabeçalho não reage ao ponteiro"
                );
                assert!(
                    row_fill(Slot::Footer, v, false, p).hover.is_none(),
                    "{nome}/{v:?}: rodapé não reage ao ponteiro"
                );
                assert!(
                    row_fill(Slot::Body, v, false, p).hover.is_some(),
                    "{nome}/{v:?}: a linha de dado reage"
                );
            }
        }
    }

    /// **Selecionado vence o hover.** A leitura adotada é a mesma do [`crate::Toggle`]: o estado é
    /// informação, o hover é só afordância. Se alguém inverter, uma linha selecionada clareia ao
    /// passar o mouse e a seleção fica ilegível.
    #[test]
    fn hover_nao_apaga_a_selecao() {
        for (nome, p) in AMBAS {
            for v in [TableVariant::Default, TableVariant::Card] {
                let sel = row_fill(Slot::Body, v, true, p);
                assert!(sel.rest.is_some(), "{nome}/{v:?}: selecionado sempre tem fundo");
                assert_eq!(
                    sel.hover, sel.rest,
                    "{nome}/{v:?}: o hover do selecionado é o próprio fundo de selecionado"
                );

                // E o selecionado é MAIS forte que o hover: 4% contra 2%.
                let solto = row_fill(Slot::Body, v, false, p);
                assert_ne!(
                    sel.rest, solto.hover,
                    "{nome}/{v:?}: selecionado (4%) e hover (2%) são cores diferentes"
                );
            }
        }
    }

    // --- Cores ------------------------------------------------------------------------------------

    /// **Os `color-mix` resolvidos, um por um.** É a conta do doc do módulo travada em número: canal
    /// = A·(1−N) + B·N, arredondado.
    #[test]
    fn os_color_mix_batem_com_a_conta() {
        /// O canal (0–255) de uma cor `0xRRGGBBAA` neutra.
        fn canal(c: Rgba8) -> u32 {
            (c.0 >> 24) & 0xff
        }
        /// `color-mix(in srgb, a, tinta n)` num canal, arredondado ao inteiro.
        fn mix(a: u32, tinta: u32, n: f32) -> u32 {
            (a as f32 * (1.0 - n) + tinta as f32 * n).round() as u32
        }

        const PRETO: u32 = 0;
        const BRANCO: u32 = 255;

        // --- claro: a tinta é PRETO, e --background == --card == #ffffff ---
        let bg_claro = 255;
        assert_eq!(canal(TABLE_LIGHT.card), 255, "--card claro = #ffffff");
        assert_eq!(canal(TABLE_LIGHT.row_hover), mix(bg_claro, PRETO, 0.02));
        assert_eq!(canal(TABLE_LIGHT.row_hover), 250, "#fafafa");
        assert_eq!(canal(TABLE_LIGHT.row_selected), mix(bg_claro, PRETO, 0.04));
        assert_eq!(canal(TABLE_LIGHT.row_selected), 245, "#f5f5f5");
        assert_eq!(canal(TABLE_LIGHT.card_row_hover), mix(255, PRETO, 0.02));
        assert_eq!(canal(TABLE_LIGHT.card_row_selected), mix(255, PRETO, 0.04));
        assert_eq!(canal(TABLE_LIGHT.footer_bg), mix(255, PRETO, 0.02));

        // --- escuro: a tinta é BRANCO ---
        // --background = mix(neutral-950 96%, branco) = 10·0,96 + 255·0,04 ≈ 20 = 0x14
        let bg_escuro = mix(10, BRANCO, 0.04);
        assert_eq!(bg_escuro, 20, "--background escuro = #141414");
        // --card = mix(--background 98%, branco) = 20·0,98 + 255·0,02 = 24,7 → 25
        let card_escuro = canal(TABLE_DARK.card);
        assert_eq!(card_escuro, mix(bg_escuro, BRANCO, 0.02), "--card sai de --background");
        assert_eq!(card_escuro, 25, "--card escuro = #191919, o mesmo do crate::card");

        assert_eq!(canal(TABLE_DARK.row_hover), mix(bg_escuro, BRANCO, 0.02));
        assert_eq!(canal(TABLE_DARK.row_hover), 25, "#191919");
        assert_eq!(canal(TABLE_DARK.row_selected), mix(bg_escuro, BRANCO, 0.04));
        assert_eq!(canal(TABLE_DARK.row_selected), 29, "#1d1d1d");
        assert_eq!(canal(TABLE_DARK.card_row_hover), mix(card_escuro, BRANCO, 0.02));
        assert_eq!(canal(TABLE_DARK.card_row_hover), 30, "#1e1e1e");
        assert_eq!(
            canal(TABLE_DARK.card_row_selected),
            mix(card_escuro, BRANCO, 0.04)
        );
        assert_eq!(canal(TABLE_DARK.card_row_selected), 34, "#222222");
        assert_eq!(canal(TABLE_DARK.footer_bg), mix(card_escuro, BRANCO, 0.02));

        // O `--muted-foreground` das duas pontas: mix(neutral-500 90%, tinta).
        assert_eq!(canal(TABLE_LIGHT.text_muted), mix(115, PRETO, 0.10));
        assert_eq!(canal(TABLE_LIGHT.text_muted), 104, "#686868");
        assert_eq!(canal(TABLE_DARK.text_muted), mix(115, BRANCO, 0.10));
        assert_eq!(canal(TABLE_DARK.text_muted), 129, "#818181");

        // A 4% é sempre MAIS forte que a 2% — se alguém trocar os dois de lugar, a seleção fica mais
        // fraca que o hover e a tabela mente sobre o estado.
        assert!(
            canal(TABLE_LIGHT.row_selected) < canal(TABLE_LIGHT.row_hover),
            "no claro a tinta é preta: 4% escurece MAIS"
        );
        assert!(
            canal(TABLE_DARK.row_selected) > canal(TABLE_DARK.row_hover),
            "no escuro a tinta é branca: 4% clareia MAIS"
        );
    }

    /// Os alfas dos tokens translúcidos, e o fato de as cores de fundo serem OPACAS.
    #[test]
    fn alfas_dos_tokens_batem_com_a_referencia() {
        // `--border`: preto 8% no claro (alfa 20), branco 6% no escuro (alfa 15).
        assert_eq!(TABLE_LIGHT.border, Rgba8(0x00000014), "preto 8%");
        assert_eq!(TABLE_DARK.border, Rgba8(0xffffff0f), "branco 6%");
        assert!((TABLE_LIGHT.border.alpha() - 0.08).abs() < 0.01);
        assert!((TABLE_DARK.border.alpha() - 0.06).abs() < 0.01);

        // `shadow-xs/5`: preto 5% (alfa 13), o mesmo de toda sombra da lib.
        for (nome, p) in AMBAS {
            assert!(
                (p.shadow.alpha() - 13.0 / 255.0).abs() < 1e-6,
                "{nome}: sombra a 5%"
            );
        }

        // Os fundos de linha são OPACOS: eles se sobrepõem uns aos outros, e um alfa parcial faria a
        // cor da linha selecionada depender do que houver atrás dela.
        for (nome, p) in AMBAS {
            for (campo, c) in [
                ("card", p.card),
                ("row_hover", p.row_hover),
                ("row_selected", p.row_selected),
                ("card_row_hover", p.card_row_hover),
                ("card_row_selected", p.card_row_selected),
                ("footer_bg", p.footer_bg),
            ] {
                assert_eq!(c.alpha(), 1.0, "{nome}/{campo}: fundo tem que ser opaco");
            }
            // Filete e bisel, ao contrário, são translúcidos — é isso que os faz funcionar sobre
            // qualquer fundo.
            assert!(p.border.alpha() < 1.0, "{nome}: filete translúcido");
            assert!(p.bevel.alpha() < 1.0, "{nome}: bisel translúcido");
        }
    }

    /// Os tokens de tema entram por [`opaque`], a única ponte entre os `0xRRGGBB` de
    /// [`crate::theme`] e os `0xRRGGBBAA` daqui. Se um valor de 6 dígitos entrasse direto, o canal
    /// vermelho viria zerado — a assinatura do bug histórico do campo de texto.
    #[test]
    fn tokens_do_tema_entram_por_opaque() {
        assert_eq!(TABLE_LIGHT.text, opaque(0x262626), "neutral-800");
        assert_eq!(TABLE_DARK.text, opaque(0xf5f5f5), "neutral-100");
        assert_eq!(TABLE_LIGHT.card, opaque(0xffffff));
        assert_eq!(TABLE_DARK.card, opaque(0x191919));
        assert_eq!(TABLE_LIGHT.text_muted, opaque(0x686868));
        assert_eq!(TABLE_DARK.text_muted, opaque(0x818181));

        // E todas as cores neutras decodificam NEUTRAS (r == g == b). Um valor deslocado sairia
        // colorido — foi assim que campos ciano apareceram três vezes nesta base.
        for (nome, p) in AMBAS {
            for (campo, c) in [
                ("text", p.text),
                ("text_muted", p.text_muted),
                ("card", p.card),
                ("row_hover", p.row_hover),
                ("row_selected", p.row_selected),
                ("footer_bg", p.footer_bg),
            ] {
                let c: gpui::Rgba = c.hsla().into();
                assert!(
                    (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                    "{nome}/{campo}: tinha que ser neutro — se r≠g≠b, o valor foi lido deslocado"
                );
            }
        }
        // O texto é escuro no claro e claro no escuro.
        let claro: gpui::Rgba = TABLE_LIGHT.text.hsla().into();
        let escuro: gpui::Rgba = TABLE_DARK.text.hsla().into();
        assert!(claro.r < 0.2 && escuro.r > 0.8);
    }

    // --- Bisel ------------------------------------------------------------------------------------

    /// ⚠️ O desvio consciente da casa: o filete escuro é o **DOBRO** da referência.
    ///
    /// A referência da tabela usa branco a **8%** no escuro (`dark:before:shadow-[0_-1px_white/8%]`),
    /// não os 6% dos outros módulos — então o dobro aqui é 16%, e não os ~11,8% do `Card`. Se a
    /// intenção mudar, é este teste que muda junto, e não o valor sozinho.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // O claro segue FIEL: preto a 4% (`before:shadow-[0_1px_black/4%]`).
        const REF_CLARO: f32 = 0.04;
        let claro = TABLE_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 0.01,
            "o tema claro tem que seguir fiel (preto 4%); veio {claro:.3}"
        );

        // O escuro é o dobro de 8%.
        const REF_ESCURO: f32 = 0.08;
        let escuro = TABLE_DARK.bevel.alpha();
        assert!(
            (escuro - 2.0 * REF_ESCURO).abs() < 0.01,
            "esperado ~{:.3} (2 × 8%), veio {escuro:.3}",
            2.0 * REF_ESCURO
        );
        assert!(escuro < 1.0, "ainda translúcido");
        assert!(
            escuro > claro,
            "o filete escuro é mais forte que o claro — é o que o desvio existe pra conseguir"
        );
    }

    /// **O bisel troca de LADO entre os temas, e o lado sai do SINAL do deslocamento** — `0 1px`
    /// desce → borda de baixo; `0 -1px` sobe → borda de topo. Um segundo campo dizendo o lado
    /// poderia divergir do sinal; por isso é o sinal que manda.
    #[test]
    fn bisel_troca_de_lado_entre_os_temas() {
        assert!(TABLE_LIGHT.bevel_dir > 0.0, "claro: 0 1px desce");
        assert!(TABLE_DARK.bevel_dir < 0.0, "escuro: 0 -1px sobe");
        // O RGB acompanha o sentido: preto descendo no claro, branco subindo no escuro.
        assert_eq!(TABLE_LIGHT.bevel.0 & 0xffffff00, 0x00000000, "preto");
        assert_eq!(TABLE_DARK.bevel.0 & 0xffffff00, 0xffffff00, "branco");
    }

    // --- Legenda ----------------------------------------------------------------------------------

    /// A legenda ganha respiro embaixo **só na variante card** (`my-4` contra `mt-4`), porque ali ela
    /// fica entre duas superfícies.
    #[test]
    fn legenda_respira_embaixo_so_na_variante_card() {
        assert_eq!(
            caption_margin(TableVariant::Default),
            (CAPTION_MARGIN, 0.0),
            "mt-4"
        );
        assert_eq!(
            caption_margin(TableVariant::Card),
            (CAPTION_MARGIN, CAPTION_MARGIN),
            "my-4"
        );
    }

    // --- Construtores -----------------------------------------------------------------------------

    /// Os defaults do componente: variante padrão, nada dentro, nenhuma linha selecionada.
    #[test]
    fn defaults_seguem_a_referencia() {
        let t = Table::new("t");
        assert_eq!(t.variant, TableVariant::Default, "variant = 'default'");
        assert!(t.columns.is_empty() && t.rows.is_empty());
        assert!(t.footer.is_none() && t.caption.is_none());

        let r = TableRow::new();
        assert!(!r.selected, "linha nasce não selecionada");
        assert!(r.cells.is_empty());
        assert!(TableRow::new().selected(true).selected);

        assert_eq!(TableColumn::new("x").align, TableAlign::Start, "text-left");
    }

    /// Uma coluna **sem rótulo** ([`TableColumn::blank`]) não deixa string vazia pra trás: é
    /// `header: None`, e é isso que faz uma tabela só de colunas em branco não desenhar uma faixa de
    /// 40px vazia com um filete embaixo.
    #[test]
    fn coluna_em_branco_nao_tem_rotulo() {
        assert!(TableColumn::blank().header.is_none());
        assert_eq!(TableColumn::new("Fatura").header, Some("Fatura".into()));

        // A condição que o render usa.
        let so_brancas = [TableColumn::blank(), TableColumn::blank()];
        assert!(!so_brancas.iter().any(|c| c.header.is_some()));
        let com_rotulo = [TableColumn::blank(), TableColumn::new("Valor")];
        assert!(com_rotulo.iter().any(|c| c.header.is_some()));
    }

    /// As células entram na ordem em que foram declaradas — o alinhamento das colunas depende disso.
    #[test]
    fn celulas_entram_na_ordem_declarada() {
        let r = TableRow::new().cell("a").cell("b").cell("c");
        assert_eq!(r.cells.len(), 3);
        // `child` aceita elemento qualquer e conta igual.
        let r = TableRow::new().cell("a").child(div());
        assert_eq!(r.cells.len(), 2);
    }
}

/// Testes com janela de verdade — os que provam o que aritmética nenhuma prova.
///
/// Os dois aqui existem por motivos diferentes:
///
/// - o da **entrelinha** porque um teste de constante não pega o esquecimento de CHAMAR
///   `.line_height()`, e é exatamente esse esquecimento que a armadilha nº 1 da casa descreve;
/// - o das **larguras** porque a mudança de contrato do módulo (largura é dado, não medida) só é
///   verificável medindo: um flex que resolvesse errado apertaria as colunas em silêncio.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{canvas, Bounds, Context, Pixels, Render, TestAppContext, VisualTestContext};
    use std::cell::Cell;
    use std::rc::Rc;

    /// Uma sonda: grava os bounds que o layout lhe deu.
    type Sonda = Rc<Cell<Option<Bounds<Pixels>>>>;

    fn sonda_de(s: &Sonda) -> gpui::Canvas<()> {
        let s = s.clone();
        canvas(move |bounds, _w, _cx| s.set(Some(bounds)), |_, _, _, _| {})
    }

    /// A largura do contêiner nos testes de largura — o análogo do card do storybook.
    const LARGURA: f32 = 400.0;

    /// O lado da caixa de seleção que a coluna [`TableColumnWidth::Checkbox`] envolve.
    ///
    /// A coluna colapsa até o conteúdo, então a largura dela é este número mais o respiro — e é por
    /// isso que ela é um quadrado de lado conhecido, e não um `div()` vazio: com conteúdo de largura
    /// zero não se distingue "a coluna de checkbox colapsou certo" de "a linha inteira colapsou".
    const CHECKBOX_LADO: f32 = 16.0;

    /// A coluna fixa que fecha a tabela nos testes de largura total.
    const FIM: f32 = 80.0;

    /// As três colunas do caso `Checkbox + Flex + Fixed`, com uma sonda em cada célula.
    ///
    /// É o arranjo em que o defeito de largura aparecia: a soma das larguras fixas (80) é muito
    /// menor que o contêiner, então o `min_w` da tabela não ajuda em nada e só o esticamento faz a
    /// tabela ocupá-lo.
    fn tabela_de_tres_colunas(
        id: &'static str,
        variante: TableVariant,
        checkbox: &Sonda,
        meio: &Sonda,
        fim: &Sonda,
    ) -> Table {
        Table::new(id)
            .variant(variante)
            .column(TableColumn::blank().width(TableColumnWidth::Checkbox))
            .column(TableColumn::new("Item"))
            .column(
                TableColumn::new("Valor")
                    .width(TableColumnWidth::Fixed(FIM))
                    .align(TableAlign::End),
            )
            .row(
                TableRow::new()
                    .child(
                        div()
                            .size(px(CHECKBOX_LADO))
                            .child(sonda_de(checkbox).size_full()),
                    )
                    .child(sonda_de(meio).w_full().h(px(1.0)))
                    .child(sonda_de(fim).w_full().h(px(1.0))),
            )
    }

    /// As três medidas que reconstroem a largura de uma linha de `Checkbox + Flex + Fixed`.
    struct TresColunas {
        /// Onde a linha termina — a borda direita da tabela.
        borda_direita: f32,
        /// O quadrado da caixa de seleção, que a coluna de checkbox tem que respeitar.
        checkbox: Bounds<Pixels>,
        /// A largura da coluna `Flex` — o número que ia a zero no defeito.
        flex: f32,
    }

    /// Confere que as três colunas somam exatamente `largura`, com a `Flex` ficando com a sobra.
    ///
    /// `esquerda` é onde a tabela começa, pra a conta valer em qualquer lugar da tela.
    fn confere_tres_colunas(nome: &str, m: &TresColunas, esquerda: f32, largura: f32, moldura: f32) {
        assert!(
            (m.borda_direita - (esquerda + largura)).abs() < 0.5,
            "{nome}: a tabela terminou em {} e o contêiner de {largura} termina em {} — ela não \
             ocupou a largura, e o texto de toda célula sai aparado",
            m.borda_direita,
            esquerda + largura
        );
        assert!(
            (f32::from(m.checkbox.origin.x) - (esquerda + PAD)).abs() < 0.5,
            "{nome}: a caixa de seleção começa em {} e o respiro da ponta pede {}",
            f32::from(m.checkbox.origin.x),
            esquerda + PAD
        );
        assert!(
            (f32::from(m.checkbox.size.width) - CHECKBOX_LADO).abs() < 0.5,
            "{nome}: a caixa de seleção mediu {} e não os {CHECKBOX_LADO} declarados — a coluna de \
             checkbox apertou o conteúdo dela",
            f32::from(m.checkbox.size.width)
        );
        // A `Flex` fica com TUDO que sobrou: o contêiner menos a coluna de checkbox (respiro da
        // ponta + conteúdo + zero do lado de dentro), menos a fixa, menos os dois respiros dela, e
        // menos a moldura do card quando ela existe.
        let sobra = largura - (PAD + CHECKBOX_LADO) - FIM - 2.0 * PAD - moldura;
        assert!(
            (m.flex - sobra).abs() < 0.5,
            "{nome}: a coluna Flex mediu {} e o que sobrava era {sobra}",
            m.flex
        );
    }

    /// Lê as três sondas e devolve as medidas derivadas.
    fn le_tres_colunas(checkbox: &Sonda, meio: &Sonda, fim: &Sonda) -> TresColunas {
        let f = fim.get().expect("mediu a coluna fixa");
        TresColunas {
            // A célula fixa é a última: o fim dela mais o respiro é a borda direita da linha.
            borda_direita: f32::from(f.origin.x) + f32::from(f.size.width) + PAD,
            checkbox: checkbox.get().expect("mediu a coluna de checkbox"),
            flex: f32::from(meio.get().expect("mediu a coluna Flex").size.width),
        }
    }

    /// A moldura de 1px que entra na conta da largura, por variante.
    ///
    /// Ela é das linhas de **corpo** (ver [`row_borders`]), e é a linha de corpo que as sondas
    /// medem. O respiro da ponta desconta a moldura na variante card (ver [`cell_pad`]), então a
    /// soma "respiro + moldura" é a mesma nas duas — o respiro ótico de 10px.
    fn moldura_de(variante: TableVariant) -> f32 {
        match variante {
            TableVariant::Default => 0.0,
            TableVariant::Card => BORDER,
        }
    }

    /// Mede a **altura da tabela** pela origem de um irmão colado logo abaixo dela.
    ///
    /// Medir a tabela por dentro não serve: as linhas são internas. A origem do irmão é a soma de
    /// tudo que a tabela ocupou, que é justamente o número que a entrelinha muda.
    struct MedidorDeAltura {
        linhas: usize,
        abaixo: Sonda,
    }

    impl Render for MedidorDeAltura {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            let mut t = Table::new("t")
                .column(TableColumn::new("A"))
                .column(TableColumn::new("B"));
            for i in 0..self.linhas {
                t = t.row(TableRow::new().cell(format!("a{i}")).cell(format!("b{i}")));
            }
            div()
                .flex()
                .flex_col()
                .w(px(400.0))
                .child(t)
                .child(sonda_de(&self.abaixo).h(px(1.0)))
        }
    }

    /// ⚠️ **A entrelinha apertada mantém a altura da linha.**
    ///
    /// `leading-none` é 1,0 e o default do GPUI é `relative(1.618034)`. Sem declarar, CADA linha
    /// ganha 8,65px — numa tabela isso não desloca um texto meio pixel, muda a altura do componente
    /// inteiro. Um `assert_eq!(LINE_HEIGHT, 14.0)` não pega isso: pega o valor, não a chamada.
    #[gpui::test]
    fn entrelinha_apertada_mantem_a_altura_da_linha(cx: &mut TestAppContext) {
        const LINHAS: usize = 4;

        let abaixo: Sonda = Rc::new(Cell::new(None));
        let s = abaixo.clone();
        let window = cx.add_window(move |_w, _cx| MedidorDeAltura {
            linhas: LINHAS,
            abaixo: s,
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let medida = f32::from(abaixo.get().expect("a sonda mediu").origin.y);

        // O esperado, das constantes: cabeçalho + (n−1) linhas com filete + a última sem.
        let esperado = row_height(Slot::Head, TableVariant::Default, true, true)
            + (LINHAS - 1) as f32 * row_height(Slot::Body, TableVariant::Default, false, false)
            + row_height(Slot::Body, TableVariant::Default, false, true);

        assert!(
            (medida - esperado).abs() < 0.5,
            "a tabela mediu {medida} e a referência pede {esperado}"
        );

        // E a prova de que o teste morde: com a entrelinha default do GPUI, a mesma tabela sairia
        // muito mais alta. Se o `.line_height()` do render desaparecer, é este número que a janela
        // devolve.
        const RAZAO_DE_OURO: f32 = 1.618_034;
        let sem_declarar = esperado + LINHAS as f32 * (TEXT_SIZE * RAZAO_DE_OURO - LINE_HEIGHT);
        assert!(
            sem_declarar - esperado > 30.0,
            "sem a entrelinha declarada a tabela iria a {sem_declarar:.1} (contra {esperado})"
        );
    }

    /// Mede a célula do **meio** de uma tabela mais larga que o contêiner.
    struct MedidorDeColuna {
        largura_da_caixa: f32,
        larguras: [f32; 3],
        meio: Sonda,
    }

    impl Render for MedidorDeColuna {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            let [a, b, c] = self.larguras;
            div().w(px(self.largura_da_caixa)).child(
                Table::new("t")
                    .column(TableColumn::new("A").width(TableColumnWidth::Fixed(a)))
                    .column(TableColumn::new("B").width(TableColumnWidth::Fixed(b)))
                    .column(TableColumn::new("C").width(TableColumnWidth::Fixed(c)))
                    .row(
                        TableRow::new()
                            // ⚠️ Textos MAIS LARGOS que qualquer das colunas, de propósito. Com
                            // conteúdo curto, trocar o `w` da coluna por um `min_w` daria o mesmo
                            // layout e o teste não veria diferença — foi o que a
                            // mutação-verificação mostrou. Com conteúdo que transborda, `w` recorta
                            // (o contrato declarado) e um `min_w` deixaria o conteúdo mandar.
                            .cell("um primeiro texto longo o bastante pra passar de qualquer coluna")
                            // ⚠️ A sonda vai na coluna do MEIO, e isso é essencial. Na última ela não
                            // discrimina: um `min_w` no lugar do `w` daria à última coluna o mesmo
                            // piso, prendendo-a na largura declarada, e as duas primeiras se
                            // ajustariam pra somar o mesmo total — a posição da última coincidia e a
                            // mutação passava. A do meio depende da largura REAL da primeira.
                            .child(sonda_de(&self.meio).w_full().h(px(1.0)))
                            .cell("um terceiro texto longo o bastante pra passar de qualquer coluna"),
                    ),
            )
        }
    }

    /// ⚠️ **As larguras das colunas são dado do componente, e não são apertadas pelo contêiner.**
    ///
    /// É a mudança de contrato do módulo, e ela só existe se o layout a honrar: três colunas de 200
    /// numa caixa de 300 têm que continuar 200 cada e transbordar (o embrulho rola), e não virar 100
    /// cada. Um flex resolvido errado apertaria em silêncio — e colunas apertadas não parecem um bug,
    /// parecem uma tabela estreita.
    #[gpui::test]
    fn as_larguras_das_colunas_nao_sao_apertadas_pelo_container(cx: &mut TestAppContext) {
        // ⚠️ Larguras DESIGUAIS. Com três colunas iguais o teste não discrimina: um `min_w` no lugar
        // do `w` deixaria as três encolherem até a mesma largura, que por simetria é exatamente a
        // declarada — a mutação passava. Desiguais, só a largura declarada produz estas posições.
        const LARGURAS: [f32; 3] = [100.0, 200.0, 300.0];
        const SOMA: f32 = 600.0;

        // ⚠️ E as DUAS pressões, apertada e folgada. Com só a caixa estreita a mutação `w` → `min_w`
        // também passava: os pisos das três colunas somam exatamente a largura da linha, então o
        // encolhimento é forçado até os pisos e o resultado coincide. É na caixa FOLGADA que os dois
        // se separam — lá o conteúdo longo faria as colunas CRESCEREM além da largura declarada.
        let medir = |cx: &mut TestAppContext, caixa: f32| {
            let meio: Sonda = Rc::new(Cell::new(None));
            let s = meio.clone();
            let window = cx.add_window(move |_w, _cx| MedidorDeColuna {
                largura_da_caixa: caixa,
                larguras: LARGURAS,
                meio: s,
            });
            let vcx = VisualTestContext::from_window(window.into(), cx);
            vcx.run_until_parked();
            let b = meio.get().expect("a sonda mediu a célula do meio");
            (f32::from(b.origin.x), f32::from(b.size.width))
        };

        assert_eq!(LARGURAS.iter().sum::<f32>(), SOMA);
        // A segunda célula começa depois da primeira coluna cheia, mais o respiro dela; e tem a
        // largura declarada menos os dois respiros.
        let esperado = (LARGURAS[0] + PAD, LARGURAS[1] - 2.0 * PAD);

        for (nome, caixa) in [("apertada", SOMA / 2.0), ("folgada", SOMA * 1.5)] {
            let (x, largura) = medir(cx, caixa);
            assert!(
                (x - esperado.0).abs() < 0.5,
                "caixa {nome} ({caixa}): a segunda coluna começa em {x} e a largura declarada pede \
                 {} — o contêiner mandou na largura em vez das colunas",
                esperado.0
            );
            assert!(
                (largura - esperado.1).abs() < 0.5,
                "caixa {nome} ({caixa}): a célula mediu {largura} e o esperado é {}",
                esperado.1
            );
        }
    }

    /// ⚠️ **Uma tabela mais larga que a caixa TEM curso de rolagem.**
    ///
    /// Sem curso, as colunas que não cabem ficam visíveis fora do embrulho e inalcançáveis — o pior
    /// dos dois mundos. O curso não sai de graça: o `content_size` do embrulho vem do tamanho da
    /// CAIXA da tabela, e uma tabela esticada à largura do embrulho mede o embrulho, o que dá curso
    /// zero. É o `min_w` da soma das colunas que faz a caixa da tabela virar a largura real.
    ///
    /// Este teste existe porque o de cima **não** pega isso: lá o que se mede são as POSIÇÕES das
    /// colunas, e elas continuam certas mesmo sem curso — a tabela simplesmente transborda o embrulho
    /// em silêncio. Ter a largura certa e não poder alcançá-la são dois defeitos diferentes, e cada um
    /// precisa do seu teste.
    #[gpui::test]
    fn a_tabela_estreita_tem_curso_de_rolagem(cx: &mut TestAppContext) {
        const CAIXA: f32 = 300.0;
        const COLUNA: f32 = 200.0;
        const COLUNAS: f32 = 3.0;

        struct Medidor {
            scroll: gpui::ScrollHandle,
        }
        impl Render for Medidor {
            fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                theme::set_theme(theme::ThemeMode::Dark);
                let w = TableColumnWidth::Fixed(COLUNA);
                div().w(px(CAIXA)).child(
                    Table::new("t")
                        .track_scroll(&self.scroll)
                        .column(TableColumn::new("A").width(w))
                        .column(TableColumn::new("B").width(w))
                        .column(TableColumn::new("C").width(w))
                        .row(TableRow::new().cell("a").cell("b").cell("c")),
                )
            }
        }

        let scroll = gpui::ScrollHandle::new();
        let s = scroll.clone();
        let window = cx.add_window(move |_w, _cx| Medidor { scroll: s });
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let curso = f32::from(scroll.max_offset().width);
        let esperado = COLUNAS * COLUNA - CAIXA;
        assert!(
            (curso - esperado).abs() < 0.5,
            "o curso mediu {curso} e o esperado é {esperado} ({COLUNAS} × {COLUNA} − {CAIXA}) — sem \
             curso as colunas de fora ficam inalcançáveis"
        );
        // E o embrulho continua do tamanho da caixa: se ele crescesse com o conteúdo, não haveria
        // rolagem nenhuma, só transbordo.
        let embrulho = f32::from(scroll.bounds().size.width);
        assert!(
            (embrulho - CAIXA).abs() < 0.5,
            "o embrulho mediu {embrulho} numa caixa de {CAIXA} — ele cresceu com o conteúdo"
        );

        // E a roda de fato ROLA. Ter curso e não rolar seria o mesmo defeito visto de outro lado, e é
        // o que uma medida de `max_offset` sozinha não distingue.
        assert_eq!(f32::from(scroll.offset().x), 0.0, "começa encostada à esquerda");
        vcx.simulate_event(gpui::ScrollWheelEvent {
            position: point(px(150.0), px(20.0)),
            // Delta VERTICAL de propósito: o GPUI mapeia a roda vertical no eixo X quando o elemento
            // só rola em X (o `restrict_scroll_to_axis` é `false` por default), e é assim que se rola
            // uma tabela larga com roda de mouse ou dois dedos.
            delta: gpui::ScrollDelta::Pixels(point(px(0.0), px(-90.0))),
            modifiers: gpui::Modifiers::default(),
            touch_phase: gpui::TouchPhase::Moved,
        });
        vcx.run_until_parked();
        assert!(
            f32::from(scroll.offset().x) < -1.0,
            "a roda tinha que mover a tabela na horizontal (deu {})",
            f32::from(scroll.offset().x)
        );
    }

    /// Mede onde a SEGUNDA coluna começa em duas linhas de conteúdos bem diferentes.
    struct MedidorDeAlinhamento {
        curta: Sonda,
        longa: Sonda,
    }

    impl Render for MedidorDeAlinhamento {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            div().w(px(400.0)).child(
                Table::new("t")
                    // As duas em Flex: é o caso em que o conteúdo poderia mandar na largura.
                    .column(TableColumn::new("A"))
                    .column(TableColumn::new("B"))
                    .row(
                        TableRow::new()
                            .cell("x")
                            .child(sonda_de(&self.curta).w_full().h(px(1.0))),
                    )
                    .row(
                        TableRow::new()
                            // Texto longo de propósito: o min-content de um texto no GPUI é a frase
                            // INTEIRA numa linha, então é ele que empurraria a coluna.
                            .cell("um texto bem mais longo que o da linha de cima, de propósito")
                            .child(sonda_de(&self.longa).w_full().h(px(1.0))),
                    ),
            )
        }
    }

    /// ⚠️ **As colunas `Flex` ficam alinhadas entre linhas de conteúdos diferentes.**
    ///
    /// É o contrato do componente: flex resolve larguras linha por linha, sem olhar as vizinhas, e o
    /// min-content de um texto no GPUI é a frase inteira numa linha (o `request_measured_layout` do
    /// `TextElement` só quebra com largura definida). Sem `flex_basis(0)` + `min_w(0)` na coluna, a
    /// linha do texto longo empurraria a segunda coluna pra direita e as duas linhas sairiam
    /// desalinhadas — uma tabela torta, e por um motivo que não aparece lendo o código.
    #[gpui::test]
    fn colunas_flex_ficam_alinhadas_entre_linhas(cx: &mut TestAppContext) {
        let curta: Sonda = Rc::new(Cell::new(None));
        let longa: Sonda = Rc::new(Cell::new(None));
        let (c, l) = (curta.clone(), longa.clone());
        let window = cx.add_window(move |_w, _cx| MedidorDeAlinhamento { curta: c, longa: l });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let a = curta.get().expect("mediu a linha curta");
        let b = longa.get().expect("mediu a linha longa");
        let (xa, xb) = (f32::from(a.origin.x), f32::from(b.origin.x));
        assert!(
            (xa - xb).abs() < 0.5,
            "a segunda coluna começa em {xa} na linha curta e em {xb} na longa — as colunas estão \
             desalinhadas, que é o defeito que a largura declarada existe pra impedir"
        );
        // E as duas têm a mesma largura, não só a mesma origem.
        let (wa, wb) = (f32::from(a.size.width), f32::from(b.size.width));
        assert!(
            (wa - wb).abs() < 0.5,
            "a segunda coluna mediu {wa} e {wb} — larguras diferentes entre linhas"
        );
        // A prova de que o teste testa algo: a coluna não colapsou nem tomou tudo.
        assert!(wa > 1.0 && xa > 1.0, "coluna de largura {wa} em x={xa}");
    }

    /// Mede `Checkbox + Flex + Fixed` num contêiner de largura declarada — o caso simples.
    struct MedidorDeLarguraTotal {
        variante: TableVariant,
        checkbox: Sonda,
        meio: Sonda,
        fim: Sonda,
    }

    impl Render for MedidorDeLarguraTotal {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            let t = tabela_de_tres_colunas(
                "t",
                self.variante,
                &self.checkbox,
                &self.meio,
                &self.fim,
            );
            // ⚠️ Contêiner de LINHA de flex, e não de bloco. É onde o `w_full` do embrulho é
            // carga: num bloco, `width: auto` já esticaria e o `w_full` não faria diferença
            // nenhuma (medido); numa linha de flex o tamanho-base do item é o do CONTEÚDO, e sem
            // o `w_full` a tabela sairia do tamanho das colunas em vez da largura do contêiner.
            div().flex().w(px(LARGURA)).child(t)
        }
    }

    /// ⚠️ **Uma tabela cujas colunas fixas não enchem o contêiner ainda ocupa a largura dele.**
    ///
    /// É o `w-full` do `<table>` da referência. Com `Checkbox + Flex + Fixed(80)` a soma das
    /// larguras fixas é 80 — o `min_w` da tabela não ajuda em nada aqui, e só o `w_full` faz a
    /// tabela chegar à borda do contêiner. A coluna `Flex` é quem recebe a sobra, e é a largura dela
    /// que diz se a tabela ocupou o espaço ou colapsou no conteúdo.
    ///
    /// Este é o caso SIMPLES — contêiner de largura declarada. O caso que de fato quebrava é o
    /// irmão logo abaixo, `a_tabela_ocupa_a_largura_com_uma_irma_mais_larga_na_pagina`.
    #[gpui::test]
    fn a_tabela_ocupa_a_largura_do_container(cx: &mut TestAppContext) {
        for (nome, variante) in [
            ("padrão", TableVariant::Default),
            ("card", TableVariant::Card),
        ] {
            let (checkbox, meio, fim): (Sonda, Sonda, Sonda) = (
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
            );
            let (c, m, f) = (checkbox.clone(), meio.clone(), fim.clone());
            let window = cx.add_window(move |_w, _cx| MedidorDeLarguraTotal {
                variante,
                checkbox: c,
                meio: m,
                fim: f,
            });
            let vcx = VisualTestContext::from_window(window.into(), cx);
            vcx.run_until_parked();

            let m = le_tres_colunas(&checkbox, &meio, &fim);
            confere_tres_colunas(nome, &m, 0.0, LARGURA, moldura_de(variante));
        }
    }

    /// A largura da coluna de leitura na página de diagnóstico abaixo.
    const COLUNA_DE_LEITURA: f32 = 400.0;

    /// A largura do painel que a envolve — MAIOR que a coluna, e maior que a tabela larga.
    const PAINEL: f32 = 800.0;

    /// As oito colunas fixas da tabela larga, e a soma delas.
    const LARGAS: [f32; 8] = [130.0, 180.0, 130.0, 150.0, 130.0, 140.0, 120.0, 140.0];

    /// As colunas da tabela de faturas — `Fixed + Fixed + Flex + Fixed`, com a `Flex` na 3ª posição.
    const FATURAS: [f32; 3] = [110.0, 110.0, 120.0];

    /// ⚠️ **A página que reproduz o defeito: uma tabela LARGA e uma ESTREITA na mesma coluna.**
    ///
    /// As quatro peças abaixo são todas necessárias — cada uma foi tirada e medida, e sem qualquer
    /// uma o defeito desaparece:
    ///
    /// 1. **O painel tira a largura do FLEX** (`flex_1` numa linha), e não de um `w()` declarado.
    ///    Com largura declarada o defeito não aparece: medido 600 (certo) contra 126 (errado).
    /// 2. **Um viewport de rolagem** (`overflow_y_scroll`) entre o painel e o conteúdo. Sem ele,
    ///    também não aparece.
    /// 3. **Uma tabela irmã mais larga que a coluna de leitura.** É ela que força o taffy a medir o
    ///    conteúdo, e é nessa medição que o `w_full` da tabela vizinha se perde. Só com a estreita,
    ///    o layout sai certo — o que é exatamente por que o defeito não aparecia em teste isolado.
    /// 4. **A coluna de leitura com `max_w`**, que é o que faz a largura final ser MENOR que o
    ///    conteúdo medido.
    ///
    /// É a cadeia do storybook, encurtada até o osso: sidebar → painel `flex_1` → área de rolagem →
    /// coluna de leitura com `max_w` → um card por história.
    struct PaginaComIrmaLarga {
        variante: TableVariant,
        checkbox: Sonda,
        meio: Sonda,
        fim: Sonda,
        /// A coluna `Flex` da tabela de faturas — a história com **rodapé e legenda**.
        faturas: Sonda,
        larga: gpui::ScrollHandle,
    }

    impl Render for PaginaComIrmaLarga {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);

            // A história 4: oito colunas fixas, mais largas que a coluna de leitura.
            let mut larga = Table::new("larga").track_scroll(&self.larga);
            for (i, w) in LARGAS.iter().enumerate() {
                larga = larga
                    .column(TableColumn::new(format!("c{i}")).width(TableColumnWidth::Fixed(*w)));
            }
            larga = larga.row(TableRow::new().cell("a").cell("b").cell("c"));

            let estreita = tabela_de_tres_colunas(
                "estreita",
                self.variante,
                &self.checkbox,
                &self.meio,
                &self.fim,
            );

            // As histórias 1 e 2: `Fixed + Fixed + Flex + Fixed`, com RODAPÉ e LEGENDA. Elas entram
            // porque o rodapé e a legenda são filhos EXTRA da raiz da tabela — e a raiz é uma coluna
            // de flex, então cada filho a mais é um item a mais que poderia resolver a largura de
            // outro jeito. Medidas quebradas antes do conserto: raiz 360 numa coluna de 738.
            let faturas = Table::new("faturas")
                .variant(self.variante)
                .column(TableColumn::new("Fatura").width(TableColumnWidth::Fixed(FATURAS[0])))
                .column(TableColumn::new("Situação").width(TableColumnWidth::Fixed(FATURAS[1])))
                .column(TableColumn::new("Método"))
                .column(
                    TableColumn::new("Total")
                        .width(TableColumnWidth::Fixed(FATURAS[2]))
                        .align(TableAlign::End),
                )
                .row(
                    TableRow::new()
                        .cell("INV001")
                        .cell("Paga")
                        .child(sonda_de(&self.faturas).w_full().h(px(1.0)))
                        .cell("R$ 250,00"),
                )
                .footer(
                    TableRow::new()
                        .cell("Total")
                        .cell("")
                        .cell("")
                        .cell("R$ 1.200,00"),
                )
                .caption("Faturas recentes.");

            // A coluna de leitura: `w_full` apertado por um `max_w`, como no storybook.
            let leitura = div()
                .flex()
                .flex_col()
                .w_full()
                .max_w(px(COLUNA_DE_LEITURA))
                .gap(px(16.0))
                .child(div().flex().flex_col().child(faturas))
                .child(div().flex().flex_col().child(larga))
                .child(div().flex().flex_col().child(estreita));

            // O painel: a largura dele vem do FLEX, não de um `w()`.
            let painel = div().flex().flex_col().flex_1().min_h_0().child(
                div()
                    .id("viewport")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(leitura),
            );

            div().flex().w(px(PAINEL)).h(px(PAINEL)).child(painel)
        }
    }

    /// ⚠️ **ESTE é o defeito: a tabela perdia o `w_full` por causa de uma IRMÃ mais larga.**
    ///
    /// Medido antes do conserto, na página acima: o embrulho da tabela media a largura certa (738 no
    /// storybook) e a tabela lá dentro media **126** — a soma do conteúdo dela, com a coluna `Flex`
    /// em ZERO. A causa é o `w_full` (uma porcentagem) não resolver dentro de um embrulho de BLOCO
    /// com `overflow` de rolagem quando o taffy precisa medir a árvore sob espaço indefinido: a
    /// porcentagem cai pra `width: auto` = tamanho do conteúdo, e o `min_w` das colunas fixas fica
    /// sendo o único piso. Por isso a história de 8 colunas fixas (piso 1120) parecia certa e a de
    /// `Checkbox + Flex + Fixed(80)` (piso 80) saía com ~110px.
    ///
    /// O conserto é o embrulho virar uma COLUNA de flex — ver o comentário no `render`.
    ///
    /// E o teste confere as TRÊS formas na mesma página, porque consertar uma e quebrar a outra é o
    /// risco real: a de `Checkbox + Flex + Fixed` e a de faturas (com rodapé e legenda) têm que
    /// ocupar a coluna de leitura, e a larga tem que continuar do tamanho declarado, transbordando
    /// com curso de rolagem. São as quatro histórias do storybook, nas duas variantes.
    #[gpui::test]
    fn a_tabela_ocupa_a_largura_com_uma_irma_mais_larga_na_pagina(cx: &mut TestAppContext) {
        for (nome, variante) in [
            ("padrão", TableVariant::Default),
            ("card", TableVariant::Card),
        ] {
            let (checkbox, meio, fim): (Sonda, Sonda, Sonda) = (
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
            );
            let faturas: Sonda = Rc::new(Cell::new(None));
            let (c, m, f, fa) = (checkbox.clone(), meio.clone(), fim.clone(), faturas.clone());
            let larga = gpui::ScrollHandle::new();
            let l = larga.clone();
            let window = cx.add_window(move |_w, _cx| PaginaComIrmaLarga {
                variante,
                checkbox: c,
                meio: m,
                fim: f,
                faturas: fa,
                larga: l,
            });
            let vcx = VisualTestContext::from_window(window.into(), cx);
            vcx.run_until_parked();

            let medidas = le_tres_colunas(&checkbox, &meio, &fim);
            confere_tres_colunas(nome, &medidas, 0.0, COLUNA_DE_LEITURA, moldura_de(variante));

            // A tabela de faturas: a `Flex` dela também fica com a sobra da coluna de leitura, o que
            // só é verdade se a tabela ocupou a largura. Ela media 20 antes do conserto (só os dois
            // respiros, conteúdo em zero).
            //
            // ⚠️ Aqui a moldura do card conta DUAS vezes, e em `confere_tres_colunas` conta UMA. Não
            // é inconsistência: lá a primeira coluna é a de checkbox, que colapsa até o conteúdo e
            // portanto ENCOLHE 1px junto com o respiro da ponta (`CARD_EDGE_PAD`), compensando uma
            // das duas bordas. Aqui a primeira coluna é `Fixed`, e a largura declarada dela não muda
            // com o respiro — então as duas bordas saem inteiras da sobra da `Flex`.
            let caixa_da_linha = COLUNA_DE_LEITURA - 2.0 * moldura_de(variante);
            let sobra_faturas = caixa_da_linha - FATURAS.iter().sum::<f32>() - 2.0 * PAD;
            let medida_faturas =
                f32::from(faturas.get().expect("mediu a Flex das faturas").size.width);
            assert!(
                (medida_faturas - sobra_faturas).abs() < 0.5,
                "variante {nome}: a coluna Flex das faturas mediu {medida_faturas} e a sobra da \
                 coluna de leitura é {sobra_faturas} — a tabela com rodapé e legenda não ocupou a \
                 largura"
            );

            // E a irmã larga NÃO regrediu: ela continua com a soma das larguras declaradas, e o
            // curso de rolagem é a sobra sobre a coluna de leitura.
            let soma: f32 = LARGAS.iter().sum();
            assert_eq!(soma, 1120.0, "as oito colunas da história de rolagem");
            let curso = f32::from(larga.max_offset().width);
            let esperado = soma - COLUNA_DE_LEITURA;
            assert!(
                (curso - esperado).abs() < 0.5,
                "variante {nome}: a tabela larga tem curso {curso} e o esperado é {esperado} \
                 ({soma} − {COLUNA_DE_LEITURA}) — consertar a estreita não pode tirar o curso da \
                 larga"
            );
            let embrulho = f32::from(larga.bounds().size.width);
            assert!(
                (embrulho - COLUNA_DE_LEITURA).abs() < 0.5,
                "variante {nome}: o embrulho da tabela larga mediu {embrulho} numa coluna de \
                 {COLUNA_DE_LEITURA} — ele cresceu com o conteúdo em vez de rolar"
            );
        }
    }
}

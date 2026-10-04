//! `Command` — o **paladar de comandos** (⌘K): campo de busca, lista filtrada em grupos, item
//! destacado, navegação por teclado e rodapé de dicas. Porte do `command.tsx` do [coss][1], que é uma
//! casca sobre o `autocomplete.tsx` do próprio coss, que por sua vez é uma casca sobre o
//! `Autocomplete` do [Base UI][2].
//!
//! [1]: https://github.com/cosscom/coss — `packages/ui/src/components/command.tsx`
//! [2]: https://base-ui.com/react/components/autocomplete
//!
//! ```ignore
//! // uma vez, no `new` da sua view:
//! let busca = cx.new(|cx| empire_ui::input::single_line(window, cx).placeholder("Buscar comando…"));
//! let dialog = cx.new(empire_ui::dialog::Dialog::new);
//! let paladar = cx.new(|cx| {
//!     Command::new(&busca, cx)
//!         .rows(vec![
//!             CommandRow::group_label("Sugestões"),
//!             CommandRow::item("Linear").shortcut("⌘L"),
//!             CommandRow::item("Figma").shortcut("⌘F"),
//!             CommandRow::separator(),
//!             CommandRow::group_label("Comandos"),
//!             CommandRow::item("Criar snippet").shortcut("⌘N"),
//!         ])
//!         .empty("Nenhum resultado.")
//! });
//! cx.subscribe(&paladar, |this, paladar, ev: &CommandEvent, cx| match ev {
//!     CommandEvent::Select(i) => { /* rode o comando `i` e feche o dialog */ }
//!     CommandEvent::Query(q) => println!("busca: {q}"),
//! })
//! .detach();
//!
//! // no `render`, como ÚLTIMO filho de uma raiz `relative().size_full()`:
//! .child(command_dialog(&self.dialog, &self.paladar))
//! // e ao abrir: dialog.update(.., |d, cx| d.open(window, cx)); paladar.read(cx).focus_input(window, cx);
//! ```
//!
//! # O que este componente É, na fonte
//!
//! O `command.tsx` tem **zero** linha de comportamento: ele é um `<Autocomplete>` com quatro props
//! fixadas e um punhado de classes trocadas.
//!
//! ```text
//! export function Command({ autoHighlight = "always", keepHighlight = true, ...props }) {
//!   return <Autocomplete autoHighlight={autoHighlight} inline keepHighlight={keepHighlight} open {...props} />;
//! }
//! ```
//!
//! As quatro props são o componente inteiro, e cada uma vira uma linha de código aqui:
//!
//! | prop | o que é, na doc do Base UI | onde está aqui |
//! |---|---|---|
//! | `open` (sem controlar) | a lista está **sempre** aberta | não há estado de aberto/fechado |
//! | `inline` | "the list is rendered inline without using the component's own popup" | não há [`gpui::deferred`], nem `anchored`, nem ancoragem |
//! | `autoHighlight="always"` | "always highlight the first item" | [`highlight_after_change`] e [`crate::menu::step_index`] |
//! | `keepHighlight` | "the highlighted item should be preserved when the pointer leaves the list" | o `on_hover` do item **não** apaga ao sair |
//!
//! E é por isso que **nada de filtragem foi escrito neste arquivo**: o casador, a poda de grupo órfão
//! e a de separador órfão são os do [`crate::autocomplete`], importados
//! ([`crate::autocomplete::visible_rows`], [`crate::autocomplete::navigable_rows`]). As duas listas têm
//! tipos de linha diferentes (a daqui carrega o slot do atalho), e é pra isso que existe o
//! [`crate::autocomplete::FilterRow`] — a regra é uma, os tipos são dois.
//!
//! # Filtragem e pontuação: **não existe pontuação**
//!
//! Este é o ponto em que um paladar de comandos costuma ser portado errado, porque o `cmdk` (a
//! primitiva da concorrência, que quase todo mundo já viu) **pontua e reordena** por fuzzy match. O
//! Base UI **não**. O filtro default é `collator.contains`, e ele é isto, literal
//! (`packages/react/src/internals/filter.ts`):
//!
//! ```text
//! const collator = new Intl.Collator(options.locale, {
//!   usage: 'search', sensitivity: 'base', ignorePunctuation: true, ...options,
//! });
//! contains(item, query, itemToString) {
//!   if (!query) { return true; }
//!   const itemString = stringifyAsLabel(item, itemToString);
//!   for (let i = 0; i <= itemString.length - query.length; i += 1) {
//!     if (collator.compare(itemString.slice(i, i + query.length), query) === 0) { return true; }
//!   }
//!   return false;
//! }
//! ```
//!
//! Ou seja: **`bool`, não número**. Janela deslizante do tamanho da busca, comparada por um collator
//! insensível a caixa, a diacrítico e a pontuação. Sem `startsWith` privilegiado, sem bônus de início
//! de palavra, sem penalidade de distância — e, consequentemente, **a ordem da lista é a que o autor
//! escreveu**: grupos na ordem declarada, itens na ordem declarada. Um item que casa no meio da palavra
//! aparece no lugar dele, não no fim.
//!
//! Está travado em `o_filtro_nao_pontua_nem_reordena`. Se um dia a intenção mudar (pontuar de verdade),
//! muda-se ali — mas não por acidente, e não porque "paladar de comandos é fuzzy".
//!
//! # O destaque quando a lista muda debaixo dele
//!
//! É o bug clássico deste componente, e a resposta do Base UI é explícita. Com `autoHighlight="always"`
//! o `activeIndex` é um índice na lista **filtrada** e é reancorado por três caminhos, todos em
//! `combobox/root/AriaCombobox.tsx`:
//!
//! ```text
//! // a busca mudou (e tem texto):
//! if (pendingHighlight.hasQuery) { if (autoHighlightMode && listIsNavigable) { store.set('activeIndex', 0); } }
//! // a busca foi apagada:
//! if (autoHighlightMode === 'always' && !clearedBySelection && selectionMode === 'none') { store.set('activeIndex', 0); }
//! // e a rede de segurança, a cada passada:
//! if (storeActiveIndex == null) { if (autoHighlightMode === 'always' && candidateItems.length > 0) { store.set('activeIndex', 0); } … }
//! if (storeActiveIndex >= candidateItems.length) { emitHighlight(undefined, -1, REASONS.none); store.set('activeIndex', null); }
//! // e, quando o filtro não deixou nada:
//! if (hasItems && autoHighlightMode && flatFilteredItems.length === 0) { setIndices({ activeIndex: null }); }
//! ```
//!
//! Traduzido em uma função pura ([`highlight_after_change`]): **mudou a busca → volta pro primeiro
//! item; a lista mudou por outro motivo → fica onde está se a linha ainda existe e é alcançável, senão
//! volta pro primeiro; nada alcançável → nenhum**. O invariante é que o destaque **nunca** aponta pra
//! uma linha que o filtro escondeu ou que está desabilitada — e ele é reafirmado no começo de todo
//! `render`, não só nos caminhos que mexem na lista, exatamente como o efeito de rede acima.
//!
//! ⚠️ Note o que isso **não** é: não é "seguir o item destacado pelo texto dele". Digitar reancora no
//! primeiro, e é o que faz `Enter` logo depois de digitar escolher o primeiro resultado — o gesto que
//! define um paladar de comandos.
//!
//! # O que veio de onde
//!
//! | pedaço | de onde veio |
//! |---|---|
//! | filtro, poda de grupo/separador órfãos | [`crate::autocomplete`] — [`crate::autocomplete::visible_rows`] |
//! | ciclo do destaque pelas setas | [`crate::menu::step_index`] (circula, **sem** passar por "nenhum") |
//! | primeiro alcançável (a âncora do `autoHighlight`) | [`crate::menu::edge_index`] |
//! | cores do popup, do item, do rótulo, do atalho | [`crate::menu::palette`] — **não** há paleta neste módulo |
//! | geometria do item, do rótulo e do separador | as constantes do [`crate::menu`] |
//! | campo de busca | [`crate::input::Input`] com [`crate::input::Input::unstyled`] |
//! | superfície modal (backdrop, `Escape`, foco, fade, colocação) | [`crate::dialog`] — [`crate::dialog::DialogPopup::bare`] |
//! | lavagem `--muted/72` e o raio de quem encosta na curva | [`crate::dialog`] (promovidos a `pub(crate)`) |
//!
//! **Não há paleta neste módulo, e isso é de propósito.** Os nove tokens que o popup usa
//! (`popover_bg`, `border`, `text`, `muted`, `accent`, `accent_text`, `shortcut`, `bevel`, `shadow`)
//! são byte a byte os do [`crate::menu`] — foi o que o [`crate::autocomplete`] descobriu tarde, depois
//! de ter escrito uma cópia idêntica dos nove nos dois temas. Uma segunda cópia aqui divergiria no dia
//! em que alguém corrigisse uma das duas.
//!
//! # O atalho, e onde o `Kbd` entra
//!
//! O `CommandShortcut` da referência é um `<kbd>` com quatro classes de TEXTO
//! (`ms-auto font-medium text-muted-foreground/72 text-xs tracking-widest`) — não é uma pastilha
//! desenhada. Nas duas demos do coss (`p-command-1`, `p-command-2`) ele recebe texto puro (`"⌘L"`), e o
//! componente `Kbd` (a pastilha) aparece **só no rodapé**, ao lado de "Navigate"/"Open"/"Close".
//!
//! Então há três portas, e nenhuma delas desenha um `Kbd` por conta própria:
//!
//! - [`CommandRow::shortcut`] — texto, que é o caso da referência. Sai com o token de atalho do
//!   [`crate::menu`], o mesmo do `MenuShortcut`;
//! - [`CommandRow::shortcut_slot`] — **qualquer elemento** à direita da linha. É por aqui que um
//!   `Kbd`/`KbdGroup` entra numa linha, quando o módulo existir;
//! - [`Command::footer_start`] / [`Command::footer_end`] — **qualquer elemento** nas duas pontas do
//!   rodapé. É onde a referência põe os `Kbd`, e é a composição que as demos fazem.
//!
//! O `kbd.rs` está sendo portado em paralelo e **não** é dependência deste arquivo: os três slots são
//! `impl IntoElement`, e a costura é do call site.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`backdrop-blur-sm` do backdrop** e as **transições** de escala/opacidade do popup: já declarados
//!   no [`crate::dialog`], que é quem desenha a camada modal. O GPUI não tem blur de fundo nem
//!   `transform` em `div`.
//! - **`z-50`** do `Portal`/`Viewport`: não há `z-index`; a ordem é a de pintura, e o
//!   [`crate::dialog::dialog_layer`] resolve isso sendo o último filho da raiz. ⚠️ **Este componente
//!   NÃO usa [`gpui::deferred`]** — e não é esquecimento: a lista aqui é `inline`, ou seja está DENTRO
//!   do conteúdo, e `deferred` escapa da `ContentMask`. Usá-lo faria a lista vazar pra fora do recorte
//!   da rolagem.
//! - **`-translate-y-[calc(1.25rem*var(--nested-dialogs))]`** e a escala/opacidade de dialog aninhado:
//!   sem `transform`, e sem contador de aninhamento nesta base.
//! - **`tracking-widest`** (0,1em de entreletra) do `CommandShortcut`: o GPUI não expõe
//!   letter-spacing. Mesma nota do `MenuShortcut` no [`crate::menu`].
//! - **`scroll-py-2`** do `CommandList` (respiro ao trazer um item pra dentro da vista): o
//!   [`gpui::ScrollHandle::scroll_to_item`] rola o mínimo pra encostar o item na borda e não aceita
//!   folga. O item destacado encosta na aresta em vez de sobrar 8px.
//! - **`**:data-[slot=scroll-area-scrollbar]:mt-2`** do `CommandPanel` (empurrar a barra 8px pra baixo):
//!   a barra aqui é a do `overflow_y_scroll` do GPUI, que não é um filho estilizável.
//! - **`not-dark:bg-clip-padding`** no popup e no painel: `background-clip` não existe no GPUI, e com
//!   `--popover` opaco não há vazamento por baixo da borda pra recortar.
//! - **`before:` do `CommandPanel`** (`absolute inset-0 rounded-t-[calc(var(--radius-xl)-1px)]`, mais
//!   o `pointer-events-none`): **não declara cor nem sombra**, então não pinta nada. É ramo morto na
//!   referência, não omissão aqui.
//! - **`rounded-b-[calc(var(--radius-2xl)-1px)]` do `CommandFooter`**: a faixa de rodapé **não tem fundo
//!   próprio** (o `--muted/72` é do popup inteiro), então não há retângulo a arredondar. Ramo morto —
//!   diferente do rodapé do [`crate::dialog`], que tem `bg-muted/72` e por isso precisa do raio.
//!
//! **Resolvido em número**
//!
//! - O `-mx-px` + `[clip-path:inset(0_1px)]` do `CommandPanel` **se anulam**: o painel cresce 1px pra
//!   cada lado (por cima da borda do popup) e é recortado exatamente naquele 1px. O resultado visível é
//!   um painel da largura da padding box com as bordas laterais invisíveis — que aqui é o painel de
//!   largura cheia com `border-t` e nada nos lados. Sem margem negativa e sem `clip-path`.
//! - `max-h-105` → [`POPUP_MAX_HEIGHT`] = **420px** (105 × 4px), clampado contra a janela menos os dois
//!   [`crate::dialog::VIEWPORT_PAD`]. `max-w-xl` → [`POPUP_MAX_WIDTH`] = **576px** (`36rem`).
//! - `py-[max(--spacing(4),4vh)] sm:py-[10vh]` do viewport → a colocação vertical é a do
//!   [`crate::dialog`] (o `grid-rows-[1fr_auto_3fr]` dele). Ver "Desvio consciente".
//! - O breakpoint `sm:` vale SEMPRE (janela de desktop ≥ 640px): item de 28px de piso e texto de 14px.
//!   Mesma decisão do [`crate::menu`] e do [`crate::autocomplete`].
//! - **Entrelinha** em toda linha de texto: 14/20 e 12/16, do Tailwind — o default do GPUI é a razão de
//!   ouro. Ver [`crate::menu::TEXT_LINE_HEIGHT`] e o teste
//!   `a_linha_do_item_e_o_par_do_tailwind_mais_o_py_15`.
//! - A altura de cada faixa é uma soma declarada: [`input_row_height`] = 48, [`item_height`] = 32,
//!   [`footer_height`] = 41, e o teto da lista é [`list_max_height`].
//!
//! **Desvio consciente**
//!
//! - **A colocação vertical é a do [`crate::dialog`]** (1fr acima / 3fr abaixo do espaço livre), e não
//!   o `py-[10vh]` do `CommandDialogViewport`. Numa janela de 900px com o paladar em 420, o original o
//!   põe a 90px do topo e o nosso a 128px. Trocar isso exigiria um segundo viewport modal nesta base
//!   (ou um parâmetro no [`crate::dialog`] que só este componente usaria); a diferença é de 38px numa
//!   caixa que já está no terço superior da tela.
//! - **A borda transparente do campo virou respiro.** A referência mantém a borda do `Input` e só a
//!   pinta de transparente (`border-transparent!`), então ela continua ocupando 1px de cada lado. O
//!   [`crate::input::Input::unstyled`] **remove** a borda, então o 1px é devolvido ao respiro do
//!   embrulho: `py-1.5` + 1 = [`INPUT_PAD_Y`] = 7px. A soma — [`input_row_height`] = 48 — é a mesma, e o
//!   texto continua centrado na mesma linha.
//! - **A escala de ícone do prefixo é a do CAMPO** ([`crate::input::InputSize::icon`], 17px no `Lg`) e
//!   não o `sm:size-4` (16px) fixo da referência; e o `[&_svg]:-mx-0.5` do addon (margem negativa, que
//!   esta base não usa) não se aplica porque o prefixo aqui é um filho **em fluxo** do
//!   [`crate::input::Input`], não um bloco absoluto. É a mesma decisão (e a mesma justificativa) já
//!   vigente no [`crate::autocomplete`], que tem o MESMO `startAddon` no MESMO lugar — divergir aqui
//!   colocaria dois campos de busca com dois tamanhos de lupa no mesmo app. Consequência: o rótulo do
//!   item começa ~3px à direita do original.
//! - **Rótulo de grupo e separador órfãos somem** — o superset que o [`crate::autocomplete`] já
//!   declarou, herdado junto com o filtro. Na referência quem filtra é o app (via `items`/`Collection`)
//!   e um grupo esvaziado sobra na tela; o `last:hidden` do `CommandSeparator` é o remendo parcial
//!   disso. É especialmente visível aqui, porque a demo do coss põe um separador depois de **todo**
//!   grupo.
//! - **O rodapé são dois slots** ([`Command::footer_start`] e [`Command::footer_end`]) e não
//!   `children` livres. O `CommandFooter` é `justify-between` e as duas demos passam exatamente dois
//!   filhos; dois slots nomeados dizem isso no tipo, e um `Vec<AnyElement>` num componente que
//!   re-renderiza não se sustenta (um `AnyElement` é de uso único).
//! - **`Escape` é do [`crate::dialog`]**, e não deste componente. Na referência o `Escape` do
//!   Autocomplete tem dois estágios (fecha a lista, depois apaga o texto) — mas aqui a lista é `open`
//!   fixo, então o primeiro estágio não existe, e o `useDismiss` do Dialog é quem responde. É o que a
//!   demo anuncia no rodapé: "Esc — Close".
//!
//! **Superset consciente**
//!
//! - [`CommandRow::shortcut_slot`]: a referência só põe texto no `CommandShortcut`. O slot de elemento
//!   existe pro `Kbd` entrar numa linha sem este arquivo depender dele.
//! - [`Command::max_height`]: a referência fixa `max-h-105` no popup do dialog. Aqui o teto é um
//!   parâmetro (com aquele default) porque o mesmo componente serve embutido fora de um dialog.
//!
//! **Ausente**
//!
//! - **`CommandCollection`** e a função-filho de `CommandList`/`CommandGroup`: são as peças de
//!   composição em render-prop do React (`{(item) => …}`). Aqui a composição é a lista plana de
//!   [`CommandRow`], como no [`crate::menu`] e no [`crate::autocomplete`].
//! - **`CommandCreateHandle`/`CommandDialogTrigger`**: o gatilho é qualquer coisa que chame
//!   [`crate::dialog::Dialog::open`] — o [`crate::dialog`] não tem (e não precisa de) um componente de
//!   gatilho, e o `createHandle` é o mecanismo de handles imperativos do Base UI, que aqui é o próprio
//!   `Entity<Dialog>`.
//! - **`autoFocus` do `CommandInput`**: não há "montar já focado" no GPUI (foco pede uma `&mut Window`,
//!   que o construtor não tem). Quem abre o dialog chama [`Command::focus_input`] na mesma atualização
//!   — e isso é **load-bearing**, não cosmético: o [`crate::dialog::Dialog::open`] foca a superfície, e
//!   com a superfície focada as teclas nem passam pela árvore deste componente.
//! - **`filter`/`items`/`limit`/`mode`/`grid`/`virtualized`** e o `onItemHighlighted`: mesmos ausentes
//!   já declarados no [`crate::autocomplete`]. O `keywords` que a segunda demo usa vem de um `filter`
//!   customizado, que é justamente o que não existe aqui.
//! - **RTL**: `ms-auto`/`ps-*`/`pe-*` estão resolvidos pra LTR, como em todo o resto da lib.
//!
//! # Sem cobertura de teste — declarado
//!
//! - **A pintura.** Nenhum teste compara pixels: cor, sombra e raio são travados por valor
//!   (`Rgba8`/`f32`) e por estrutura (`style()`), não por captura de tela.
//! - **O `deferred` que não existe.** A afirmação "este componente não usa `deferred`" é uma decisão de
//!   código, não um comportamento observável — não há teste que a prove.
//! - **A colocação vertical do popup** (o 1fr/3fr) e o **fade de 200ms**: são do [`crate::dialog`] e
//!   estão cobertos lá.
//! - **`CommandRow::shortcut_slot` e os dois slots de rodapé**: todo teste de janela **monta** os três
//!   (é a cobertura de fumaça deles: o caminho de desenho roda), mas nada afirma sobre o que o elemento
//!   devolvido desenha — ele é do call site. O que está travado é a CAIXA do atalho
//!   (`o_atalho_usa_o_token_do_menu`), que é a parte deste módulo.
//! - **A rolagem até o item destacado** ([`gpui::ScrollHandle::scroll_to_item`]): o `ScrollHandle` não
//!   expõe posição em teste sem uma passada de layout com transbordo real; o que está travado é o
//!   ÍNDICE que se passa pra ele (`o_indice_da_rolagem_e_a_posicao_visivel`), que é onde estava o erro
//!   provável (mandar o índice da linha em vez da posição visível).

use crate::autocomplete::{navigable_rows, visible_rows, ADDON_OPACITY, FilterRow};
use crate::dialog::{dialog_layer, Dialog, DialogPopup};
use crate::input::{Input, InputSize};
use crate::menu::{
    edge_index, palette, step_index, BORDER, DISABLED_OPACITY, ITEM_MIN_HEIGHT, ITEM_PAD_X,
    ITEM_RADIUS, LABEL_PAD_X, LABEL_PAD_Y, SEPARATOR_HEIGHT, SEPARATOR_MARGIN_X, SMALL_LINE_HEIGHT,
    SMALL_TEXT_SIZE, TEXT_LINE_HEIGHT, TEXT_SIZE,
};
use gpui::{
    div, prelude::FluentBuilder as _, px, AnyElement, App, Context, CursorStyle, Div, Entity,
    EventEmitter, Focusable as _, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Enter, InputEvent, InputState, MoveDown, MoveUp};

// =================================================================================================
// Eventos
// =================================================================================================

/// O que o [`Command`] emite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEvent {
    /// Um comando foi escolhido (clique ou `Enter`); carrega o índice dele em [`Command::rows`].
    ///
    /// É aqui que o call site **roda o comando e fecha o dialog** — o componente não fecha nada, como
    /// na referência (`handleItemClick` da demo faz `setOpen(false)`).
    Select(usize),
    /// O texto da busca mudou — por digitação ou porque um item foi escolhido (o
    /// `fillInputOnItemPress` que o `AutocompleteRoot` fixa; ver [`Command::choose`]).
    Query(SharedString),
}

// =================================================================================================
// Geometria
// =================================================================================================
//
// 1 unidade Tailwind (`--spacing`) = 4px; `--radius` = 10px, e daí `--radius-xl` = `--radius + 4` = 14
// e `--radius-2xl` = o default do Tailwind, 16 (o coss não redefine este último — ver
// `crate::dialog::FOOTER_RADIUS`). O que este módulo NÃO declara está importado do `crate::menu`: as
// classes do item, do rótulo de grupo e do separador são as mesmas, e três dos números mudam (estão
// abaixo).

/// Largura máxima do popup — `max-w-xl` = `36rem` = **576px**.
///
/// Maior que o [`crate::dialog`] usa por default (`max-w-lg`, 512): um paladar de comandos mostra
/// rótulo **e** atalho na mesma linha.
const POPUP_MAX_WIDTH: f32 = 576.0;

/// Altura máxima do popup — `max-h-105` = 105 × 4px = **420px**.
///
/// É o teto da caixa INTEIRA (border box), como todo `max-h-*` do Tailwind: o interior útil é
/// `420 − 2 × BORDER`.
const POPUP_MAX_HEIGHT: f32 = 420.0;

/// Respiro **horizontal** do embrulho do campo de busca — `px-2.5`.
const INPUT_PAD_X: f32 = 10.0;

/// Respiro **vertical** do embrulho do campo de busca — o `py-1.5` (6px) **mais** o 1px da borda que a
/// referência mantém transparente e o [`crate::input::Input::unstyled`] remove.
///
/// Ver "Desvio consciente" no doc do módulo: sem esta soma a faixa do campo mediria 46px em vez de 48.
const INPUT_PAD_Y: f32 = 6.0 + BORDER;

/// Tamanho do campo de busca — o `size="lg"` do `CommandInput`.
const INPUT_SIZE: InputSize = InputSize::Lg;

/// Ícone-prefixo do campo — o `SearchIcon` do lucide.
const SEARCH_ICON: &str = "icons/search.svg";

/// Raio das quinas de CIMA do painel de resultados — `rounded-t-xl` = `--radius-xl` =
/// `calc(var(--radius) + 4px)` = **14px**.
const PANEL_RADIUS_TOP: f32 = 14.0;

/// As três camadas… não: a **única** camada do `shadow-xs/5` do painel, em `(dy, blur, spread)`.
///
/// O `--shadow-xs` do Tailwind v4 é `0 1px 2px 0 rgb(0 0 0 / 0.05)`, e o modificador `/5` troca a cor
/// inteira por preto 5% (que é o que ela já era). A cor não está aqui porque é a `shadow` da paleta do
/// [`crate::menu`] — a mesma do `shadow-lg/5` do popup, porque o `/5` é o mesmo.
const PANEL_SHADOW: (f32, f32, f32) = (1.0, 2.0, 0.0);

/// Respiro da lista — `not-empty:p-2`.
///
/// ⚠️ **É o DOBRO do `p-1` do [`crate::menu`] e do [`crate::autocomplete`]**, e é uma das três
/// medidas que o `command.tsx` de fato troca. Não importe o `LIST_PAD` de lá.
const LIST_PAD: f32 = 8.0;

/// Respiro **vertical** de um item — `py-1.5`.
///
/// ⚠️ A segunda medida trocada: o `AutocompleteItem` é `py-1` (4px) e o `CommandItem` o sobrepõe com
/// `py-1.5`. Com a entrelinha de 20px isso dá um item de **32px** ([`item_height`]), acima do piso de
/// `sm:min-h-7`.
const ITEM_PAD_Y: f32 = 6.0;

/// Margem **vertical** do separador — `my-2`.
///
/// ⚠️ A terceira medida trocada: `my-1` (4px) no `AutocompleteSeparator`, `my-2` aqui.
const SEPARATOR_MARGIN_Y: f32 = 8.0;

/// Folga acima de um rótulo de grupo que não abre a lista — o `[[role=group]+&]:mt-1.5` que o
/// `AutocompleteGroup` traz.
const GROUP_GAP: f32 = 6.0;

/// Respiro **horizontal** da linha de "nenhum resultado" — o `not-empty:p-2` do `AutocompleteEmpty`.
const EMPTY_PAD_X: f32 = 8.0;

/// Respiro **vertical** da linha de "nenhum resultado" — o `not-empty:py-6` que o `CommandEmpty`
/// sobrepõe.
const EMPTY_PAD_Y: f32 = 24.0;

/// Respiro **horizontal** do rodapé — `px-5`.
const FOOTER_PAD_X: f32 = 20.0;

/// Respiro **vertical** do rodapé — `py-3`.
const FOOTER_PAD_Y: f32 = 12.0;

/// Vão entre as duas pontas do rodapé — `gap-2`.
const FOOTER_GAP: f32 = 8.0;

/// Piso do teto da lista, pra uma janela baixa: **uma linha inteira** mais o respiro da lista nos dois
/// lados.
///
/// A referência não tem piso — ela tem `4vh` de respiro mínimo no viewport e deixa o popup encolher.
/// Sem piso, uma janela de 200px daria uma lista de altura negativa (clampada em zero) e o paladar
/// ficaria inclicável; é o mesmo raciocínio do piso de 96px do [`crate::menu`], com o número saindo da
/// geometria daqui em vez de ser escolhido.
fn list_min_height() -> f32 {
    item_height() + 2.0 * LIST_PAD
}

// =================================================================================================
// As linhas da lista
// =================================================================================================

/// A assinatura de um slot montado pelo call site. Fica num alias porque inline ela é ilegível.
type SlotRender = dyn Fn(&mut Window, &mut App) -> AnyElement;

/// O atalho à direita de uma linha — o `CommandShortcut`.
enum Shortcut {
    /// Texto, que é o que a referência põe lá dentro (`"⌘L"`).
    Text(SharedString),
    /// Qualquer elemento — é por aqui que um `Kbd` entra (ver o doc do módulo).
    Slot(Box<SlotRender>),
}

/// Que tipo de linha é.
enum CommandRowKind {
    /// Um comando clicável — o `CommandItem`.
    Item {
        /// Rótulo, e também o texto que o filtro compara e que vai pro campo ao escolher.
        label: SharedString,
        /// `data-disabled`: não é clicável, o teclado a pula, e ela esmaece.
        disabled: bool,
    },
    /// O `CommandGroupLabel` — o título de um grupo.
    GroupLabel(SharedString),
    /// O `CommandSeparator` — o filete entre grupos.
    Separator,
}

/// Uma linha do paladar.
///
/// A lista é **plana** de propósito, como a do [`crate::menu`] e a do [`crate::autocomplete`]: na
/// referência o grupo é um elemento que embrulha os itens, mas o que ele produz é um rótulo seguido dos
/// itens dele — e um `Vec` plano é o que permite filtrar, navegar e rolar por um índice só.
///
/// | referência | aqui |
/// |---|---|
/// | `<CommandItem>` | [`CommandRow::item`] |
/// | `<CommandGroupLabel>` | [`CommandRow::group_label`] |
/// | `<CommandSeparator>` | [`CommandRow::separator`] |
/// | `<CommandShortcut>` | [`CommandRow::shortcut`] / [`CommandRow::shortcut_slot`] |
/// | `<CommandGroup>`, `<CommandCollection>` | — (o rótulo e o separador expressam o grupo) |
///
/// Os decoradores que não se aplicam a um tipo são **ignorados** em vez de proibidos (um `.shortcut()`
/// num separador não faz nada) — a mesma escolha do [`crate::menu::MenuItem`].
pub struct CommandRow {
    kind: CommandRowKind,
    shortcut: Option<Shortcut>,
}

impl CommandRow {
    /// Base comum dos construtores.
    fn of(kind: CommandRowKind) -> Self {
        Self {
            kind,
            shortcut: None,
        }
    }

    /// Um comando clicável (`CommandItem`).
    pub fn item(label: impl Into<SharedString>) -> Self {
        Self::of(CommandRowKind::Item {
            label: label.into(),
            disabled: false,
        })
    }

    /// Um rótulo de grupo (`CommandGroupLabel`).
    pub fn group_label(text: impl Into<SharedString>) -> Self {
        Self::of(CommandRowKind::GroupLabel(text.into()))
    }

    /// Um separador (`CommandSeparator`).
    pub fn separator() -> Self {
        Self::of(CommandRowKind::Separator)
    }

    /// Desabilita o comando: não responde a clique, o teclado o pula, e ele esmaece pra 64%. Em rótulo
    /// e separador não faz nada.
    pub fn disabled(mut self, disabled: bool) -> Self {
        if let CommandRowKind::Item { disabled: d, .. } = &mut self.kind {
            *d = disabled;
        }
        self
    }

    /// O atalho como **texto** — o caso da referência (`<CommandShortcut>⌘L</CommandShortcut>`).
    pub fn shortcut(mut self, text: impl Into<SharedString>) -> Self {
        self.shortcut = Some(Shortcut::Text(text.into()));
        self
    }

    /// O atalho como **elemento**: é por aqui que um `Kbd` entra numa linha (ver o doc do módulo).
    ///
    /// O elemento é montado a cada frame — é o mesmo contrato do gatilho do [`crate::menu::Menu`],
    /// porque um [`gpui::AnyElement`] é de uso único e não pode ficar guardado na entidade.
    pub fn shortcut_slot(
        mut self,
        slot: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.shortcut = Some(Shortcut::Slot(Box::new(slot)));
        self
    }

    /// O rótulo, quando é um comando.
    pub fn label(&self) -> Option<&SharedString> {
        match &self.kind {
            CommandRowKind::Item { label, .. } => Some(label),
            _ => None,
        }
    }
}

impl FilterRow for CommandRow {
    fn match_text(&self) -> Option<&str> {
        match &self.kind {
            CommandRowKind::Item { label, .. } => Some(label.as_ref()),
            _ => None,
        }
    }

    fn is_label(&self) -> bool {
        matches!(self.kind, CommandRowKind::GroupLabel(_))
    }

    fn is_divider(&self) -> bool {
        matches!(self.kind, CommandRowKind::Separator)
    }

    fn keyboard_reachable(&self) -> bool {
        matches!(self.kind, CommandRowKind::Item { disabled: false, .. })
    }
}

// =================================================================================================
// Comportamento (as quatro props fixadas, resolvidas)
// =================================================================================================

/// A máscara que o [`crate::menu::step_index`] e o [`crate::menu::edge_index`] consomem, a partir dos
/// índices que o [`crate::autocomplete::navigable_rows`] devolveu.
///
/// É só a troca de representação (lista de índices → `Vec<bool>` do tamanho da lista): quem decide
/// **quem** é alcançável continua sendo a regra de lá, e não uma segunda cópia dela aqui.
fn navigable_mask(len: usize, nav: &[usize]) -> Vec<bool> {
    let mut mask = vec![false; len];
    for &i in nav {
        mask[i] = true;
    }
    mask
}

/// Onde o destaque vai parar depois de a lista mudar — o `autoHighlight="always"` do Base UI.
///
/// Três casos, e os três estão citados no doc do módulo:
///
/// 1. **nada alcançável** → `None` (o `if (… flatFilteredItems.length === 0) setIndices({ activeIndex:
///    null })`);
/// 2. **a busca mudou** (`query_changed`) → o **primeiro** alcançável (o `store.set('activeIndex', 0)`
///    dos dois caminhos de `pendingQueryHighlight`). É o que faz `Enter` depois de digitar escolher o
///    primeiro resultado;
/// 3. **a lista mudou por outro motivo** (itens trocados de fora) → fica onde está **se a linha ainda
///    existe e é alcançável**; senão, o primeiro (o par
///    `if (storeActiveIndex >= candidateItems.length) → null` + `if (storeActiveIndex == null) → 0`).
///
/// O invariante que sai daí: o destaque **nunca** aponta pra uma linha escondida pelo filtro nem pra
/// uma desabilitada. É por isso que ele é reafirmado no começo de todo `render` e não só nos caminhos
/// que mexem na lista.
fn highlight_after_change(
    current: Option<usize>,
    mask: &[bool],
    query_changed: bool,
) -> Option<usize> {
    let primeiro = edge_index(mask, false);
    if primeiro.is_none() || query_changed {
        return primeiro;
    }
    match current {
        Some(i) if mask.get(i) == Some(&true) => Some(i),
        _ => primeiro,
    }
}

/// Altura da faixa do campo de busca: o embrulho `py-1.5` (com o 1px da borda transparente absorvido,
/// ver [`INPUT_PAD_Y`]) em volta de um campo `size="lg"`.
fn input_row_height() -> f32 {
    2.0 * INPUT_PAD_Y + INPUT_SIZE.content_height()
}

/// Altura de uma linha de comando: a entrelinha do `sm:text-sm` mais o `py-1.5`, com o piso de
/// `sm:min-h-7`.
fn item_height() -> f32 {
    (TEXT_LINE_HEIGHT + 2.0 * ITEM_PAD_Y).max(ITEM_MIN_HEIGHT)
}

/// Altura do rodapé: o `border-t`, o `py-3` e a entrelinha do `text-xs`.
fn footer_height() -> f32 {
    BORDER + 2.0 * FOOTER_PAD_Y + SMALL_LINE_HEIGHT
}

/// O interior útil do popup: o `max-h-105`, clampado contra o que a janela oferece, menos as duas
/// bordas da superfície.
///
/// O clamp existe porque a superfície do [`crate::dialog`] é `max-h-full` e **não** recorta os filhos
/// (o `overflow_hidden` comeria o bisel dela): um conteúdo mais alto que a janela pintaria pra fora
/// dela em vez de rolar.
fn popup_inner_height(viewport_height: f32, max_height: f32) -> f32 {
    (viewport_height - 2.0 * crate::dialog::VIEWPORT_PAD).min(max_height) - 2.0 * BORDER
}

/// O teto da lista rolável: o interior do popup menos a faixa do campo, a borda de topo do painel e o
/// rodapé (quando existe).
///
/// É esta subtração que faz a lista ser a única coisa que rola — o campo e o rodapé são `flex_none` e
/// ficam. O piso é [`list_min_height`].
fn list_max_height(inner_height: f32, has_footer: bool) -> f32 {
    let rodape = if has_footer { footer_height() } else { 0.0 };
    (inner_height - input_row_height() - BORDER - rodape).max(list_min_height())
}

// =================================================================================================
// O componente
// =================================================================================================

/// O paladar de comandos: campo de busca + lista filtrada, sempre aberta.
///
/// É uma `Entity` (view) própria: ela guarda o item destacado e a rolagem da lista, e **emite**
/// [`CommandEvent`]. O texto da busca mora no [`InputState`] de quem chama — o mesmo contrato do
/// [`crate::input::Input`] e do [`crate::autocomplete::Autocomplete`].
///
/// ⚠️ **Não implementa [`gpui::Focusable`], de propósito.** O que precisa de foco aqui é o CAMPO, e um
/// `focus_handle` da raiz convidaria a focar o nó errado — com a raiz focada, o `InputState` não recebe
/// tecla e o paladar fica mudo. A porta é [`Command::focus_input`].
pub struct Command {
    /// O estado de edição do campo de busca, de quem chama.
    state: Entity<InputState>,
    /// As linhas, na ordem em que aparecem.
    rows: Vec<CommandRow>,
    /// A linha **destacada** (`data-highlighted`), por hover OU por teclado — é um estado só, o que dá
    /// a semântica do Base UI: nunca há duas acesas. Índice em [`Self::rows`].
    ///
    /// Com `autoHighlight="always"` ela é `None` **só** quando não há nenhuma linha alcançável (ver
    /// [`highlight_after_change`]).
    highlighted: Option<usize>,
    /// Rolagem da lista. Persiste entre renders: sem ela, cada `cx.notify()` (um hover, por exemplo)
    /// devolveria a lista pro topo no meio da rolagem.
    list_scroll: ScrollHandle,
    /// Id estável desta instância (pra `div().id(..)` único na árvore).
    id: u64,
    /// Texto do `CommandEmpty`. `None` = não monta a linha.
    empty: Option<SharedString>,
    /// A ponta esquerda do `CommandFooter` (`justify-between`).
    footer_start: Option<Box<SlotRender>>,
    /// A ponta direita do `CommandFooter`.
    footer_end: Option<Box<SlotRender>>,
    /// Teto de altura da caixa inteira — o `max-h-105`. Ver [`Command::max_height`].
    max_height: f32,
}

impl Command {
    /// Cria um paladar sobre o [`InputState`] dado, sem comandos.
    ///
    /// O construtor **assina** o estado de texto: é a assinatura que faz digitar filtrar e reancorar o
    /// destaque, e sem ela a lista nunca reagiria à busca.
    pub fn new(state: &Entity<InputState>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(state, |this, state, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let texto = state.read(cx).value();
            this.on_text_changed(texto, cx);
        })
        .detach();

        Self {
            state: state.clone(),
            rows: Vec::new(),
            highlighted: None,
            list_scroll: ScrollHandle::new(),
            id: cx.entity_id().as_u64(),
            empty: None,
            footer_start: None,
            footer_end: None,
            max_height: POPUP_MAX_HEIGHT,
        }
    }

    /// As linhas da lista (comandos, rótulos de grupo e separadores).
    pub fn rows(mut self, rows: Vec<CommandRow>) -> Self {
        self.rows = rows;
        self
    }

    /// Atalho pra uma lista só de comandos, sem grupo nem separador.
    pub fn items<S: Into<SharedString>>(self, labels: impl IntoIterator<Item = S>) -> Self {
        self.rows(labels.into_iter().map(CommandRow::item).collect())
    }

    /// Texto do `CommandEmpty` — a linha "nenhum resultado", montada quando o filtro não deixou nenhum
    /// comando.
    pub fn empty(mut self, text: impl Into<SharedString>) -> Self {
        self.empty = Some(text.into());
        self
    }

    /// A ponta **esquerda** do rodapé. É um dos lugares onde um `Kbd` entra por composição (ver o doc do
    /// módulo); montado a cada frame.
    pub fn footer_start(
        mut self,
        slot: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.footer_start = Some(Box::new(slot));
        self
    }

    /// A ponta **direita** do rodapé (`justify-between`).
    pub fn footer_end(mut self, slot: impl Fn(&mut Window, &mut App) -> AnyElement + 'static) -> Self {
        self.footer_end = Some(Box::new(slot));
        self
    }

    /// Teto de altura da caixa inteira. Default [`POPUP_MAX_HEIGHT`] (o `max-h-105` do
    /// `CommandDialogPopup`); passe outro só se estiver embutindo o paladar fora de um dialog.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }

    /// Troca as linhas de fora. Re-filtra e **reancora o destaque** pela regra do
    /// [`highlight_after_change`] com `query_changed = false`: quem estava aceso continua, se a linha
    /// ainda existir e for alcançável.
    pub fn set_rows(&mut self, rows: Vec<CommandRow>, cx: &mut Context<Self>) {
        self.rows = rows;
        self.settle_highlight(false, cx);
        cx.notify();
    }

    /// O texto da busca.
    pub fn query(&self, cx: &App) -> SharedString {
        self.state.read(cx).value()
    }

    /// As linhas visíveis com a busca atual — a lista depois do filtro, na ordem original (o filtro do
    /// Base UI não reordena; ver o doc do módulo).
    pub fn visible(&self, cx: &App) -> Vec<usize> {
        visible_rows(&self.rows, &self.query(cx))
    }

    /// A linha destacada (índice em [`Self::rows`]), se alguma.
    ///
    /// ⚠️ Antes do primeiro `render` ela é `None`: a âncora do `autoHighlight` depende da busca, que
    /// mora no [`InputState`], e o construtor não a lê.
    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    /// Foca o campo de busca — o `autoFocus` do `CommandInput`.
    ///
    /// ⚠️ **Chame isto ao abrir o dialog.** O [`crate::dialog::Dialog::open`] foca a superfície do
    /// modal, e no GPUI tecla só chega a quem tem foco: com a superfície focada, as ações de
    /// `↑ ↓ Enter` nem passam pela árvore deste componente.
    pub fn focus_input(&self, window: &mut Window, cx: &App) {
        self.state.read(cx).focus_handle(cx).focus(window);
    }

    /// Escolhe um comando: emite [`CommandEvent::Select`] e põe o rótulo no campo.
    ///
    /// Pôr o rótulo no campo é o `fillInputOnItemPress` que o `AutocompleteRoot` fixa
    /// (`autocomplete/root/AutocompleteRoot.tsx:127`) — vale pro paladar porque ele **é** um
    /// Autocomplete. Na prática o call site fecha o dialog no [`CommandEvent::Select`] e ninguém vê o
    /// texto trocar; se ele não fechar, a lista re-filtra pelo rótulo escolhido, que é o que a referência
    /// faz.
    ///
    /// ⚠️ A escrita no [`InputState`] dispara o `InputEvent::Change` do núcleo, que volta em
    /// [`Self::on_text_changed`] **depois** desta função (o `cx.emit` do GPUI é diferido) e é ele que
    /// emite o [`CommandEvent::Query`] — daí a ordem observável ser `Select` e depois `Query`.
    ///
    /// Aqui **não há** o `Echo` do [`crate::autocomplete`], e não é esquecimento: aquele existe pra
    /// preservar a decisão de aberto/fechado que o eco desfaria, e este componente não tem aberto/fechado
    /// (`open` é fixo).
    pub fn choose(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(label) = self.rows.get(row).and_then(CommandRow::label).cloned() else {
            return;
        };
        cx.emit(CommandEvent::Select(row));
        self.state.update(cx, |estado, cx| {
            estado.set_value(label, window, cx);
        });
        cx.notify();
    }

    /// Reação a uma mudança do texto de busca — o `InputEvent::Change` do núcleo, de qualquer origem.
    fn on_text_changed(&mut self, texto: SharedString, cx: &mut Context<Self>) {
        self.settle_highlight(true, cx);
        cx.emit(CommandEvent::Query(texto));
        cx.notify();
    }

    /// Reafirma o invariante do destaque (ver [`highlight_after_change`]).
    fn settle_highlight(&mut self, query_changed: bool, cx: &App) {
        let visible = self.visible(cx);
        let nav = navigable_rows(&self.rows, &visible);
        let mask = navigable_mask(self.rows.len(), &nav);
        self.highlighted = highlight_after_change(self.highlighted, &mask, query_changed);
    }

    /// Destaca uma linha, sem repetir `notify` à toa.
    fn highlight(&mut self, row: Option<usize>, cx: &mut Context<Self>) {
        if self.highlighted == row {
            return;
        }
        self.highlighted = row;
        cx.notify();
    }

    /// Anda com as setas. Devolve `true` se consumiu a tecla.
    ///
    /// O ciclo é o do [`crate::menu::step_index`]: circula do fim pro começo **sem** passar por "nenhum
    /// destacado". É consequência direta do `autoHighlight="always"` —
    /// `allowEscape: loopFocus && !autoHighlightMode` é `false` aqui, enquanto no
    /// [`crate::autocomplete`] (que fica no default `autoHighlight: false`) é `true`.
    fn arrow(&mut self, dir: isize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        crate::focus_ring::keyboard_used(window);
        let visible = self.visible(cx);
        let nav = navigable_rows(&self.rows, &visible);
        let mask = navigable_mask(self.rows.len(), &nav);
        let Some(row) = step_index(self.highlighted, &mask, dir) else {
            // Nada alcançável: a tecla não foi usada, então não é consumida.
            return false;
        };
        // A rolagem conta os FILHOS DIRETOS da lista, que são as linhas VISÍVEIS — então o índice que
        // interessa é a posição dentro de `visible`, e não o índice da linha.
        if let Some(pos) = scroll_position(&visible, row) {
            self.list_scroll.scroll_to_item(pos);
        }
        self.highlight(Some(row), cx);
        true
    }

    /// A faixa do campo de busca — o `CommandInput` com o embrulho `px-2.5 py-1.5` dele.
    fn render_input(&self) -> impl IntoElement {
        let p = palette();
        div()
            .flex_none()
            .w_full()
            .px(px(INPUT_PAD_X))
            .py(px(INPUT_PAD_Y))
            .child(
                Input::new(&self.state)
                    .size(INPUT_SIZE)
                    // `border-transparent! bg-transparent! shadow-none before:hidden
                    //  has-focus-visible:ring-0` — as cinco classes da referência, que é exatamente o
                    // conjunto que o `unstyled` desliga (moldura, fundo, sombra, bisel, anel).
                    .unstyled()
                    .prefix(
                        gpui::svg()
                            .path(SEARCH_ICON)
                            .size(px(INPUT_SIZE.icon()))
                            .flex_none()
                            // `opacity-80` do addon, no ALFA da cor — não num passe de opacidade só
                            // pra um glifo.
                            .text_color(p.text.scaled(ADDON_OPACITY)),
                    ),
            )
    }

    /// O `CommandPanel` com o `CommandEmpty` e o `CommandList` dentro.
    fn render_panel(
        &self,
        visible: &[usize],
        list_max: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = palette();
        let tem_item = visible.iter().any(|&i| self.rows[i].is_row_item());
        let linhas = visible
            .iter()
            .enumerate()
            .map(|(pos, &row)| self.render_row(pos, row, window, cx))
            .collect::<Vec<_>>();

        // `not-empty:p-2`: o respiro só existe quando há conteúdo.
        let lista = div()
            .id(("command-list", self.id))
            .flex()
            .flex_col()
            .w_full()
            .when(!linhas.is_empty(), |d| d.p(px(LIST_PAD)))
            .max_h(px(list_max))
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            .children(linhas);

        let sem_rodape = self.footer_start.is_none() && self.footer_end.is_none();
        let mut painel = div()
            .flex()
            .flex_col()
            .flex_none()
            .w_full()
            .min_h(px(0.0))
            .bg(p.popover_bg.hsla())
            // `border border-b-0` + o par `-mx-px`/`clip-path` que se anula (ver o doc do módulo): o
            // que sobra visível é a borda de CIMA.
            .border_t_1()
            .border_color(p.border.hsla())
            .rounded_tl(px(PANEL_RADIUS_TOP))
            .rounded_tr(px(PANEL_RADIUS_TOP))
            // `not-has-[+footer]:rounded-b-2xl`, com o raio de quem vive na padding box.
            .when(sem_rodape, |d| {
                d.rounded_bl(px(crate::dialog::FOOTER_RADIUS))
                    .rounded_br(px(crate::dialog::FOOTER_RADIUS))
            })
            // Sombra EXTERNA atrás de um fundo OPACO: é o único arranjo em que a armadilha do
            // `paint_shadows` (que não recorta a sombra pra fora de quem a projeta) não morde.
            .shadow(panel_shadow());

        // A ordem é a da referência: o `CommandEmpty` vem ANTES do `CommandList`. Só um dos dois tem
        // conteúdo em qualquer momento, então a ordem é fidelidade, não desenho.
        if !tem_item {
            if let Some(texto) = self.empty.clone() {
                painel = painel.child(empty_row(texto));
            }
        }
        painel.child(lista)
    }

    /// Uma linha da lista. `pos` é a posição entre as linhas VISÍVEIS (o índice que a rolagem entende);
    /// `row` é o índice em [`Self::rows`].
    fn render_row(
        &self,
        pos: usize,
        row: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = palette();
        match &self.rows[row].kind {
            CommandRowKind::Separator => div()
                .flex_none()
                .h(px(SEPARATOR_HEIGHT))
                .mx(px(SEPARATOR_MARGIN_X))
                .my(px(SEPARATOR_MARGIN_Y))
                .bg(p.border.hsla())
                .into_any_element(),

            CommandRowKind::GroupLabel(texto) => div()
                .flex_none()
                .px(px(LABEL_PAD_X))
                .py(px(LABEL_PAD_Y))
                // `[[role=group]+&]:mt-1.5`: a folga é ENTRE grupos, então o primeiro não a leva.
                .when(pos > 0, |d| d.mt(px(GROUP_GAP)))
                .text_size(px(SMALL_TEXT_SIZE))
                .line_height(px(SMALL_LINE_HEIGHT))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted.hsla())
                .child(texto.clone())
                .into_any_element(),

            CommandRowKind::Item { label, disabled } => {
                let disabled = *disabled;
                // Uma linha desabilitada nunca acende: `data-disabled` e `data-highlighted` não
                // convivem no Base UI, e o teclado já a pula.
                let aceso = !disabled && self.highlighted == Some(row);
                let mut el = div()
                    .id(("command-item", row))
                    .flex()
                    .items_center()
                    .w_full()
                    .min_h(px(ITEM_MIN_HEIGHT))
                    .py(px(ITEM_PAD_Y))
                    .px(px(ITEM_PAD_X))
                    .rounded(px(ITEM_RADIUS))
                    .text_size(px(TEXT_SIZE))
                    .line_height(px(TEXT_LINE_HEIGHT))
                    .text_color(if aceso { p.accent_text } else { p.text }.hsla())
                    .when(aceso, |d| d.bg(p.accent.hsla()))
                    .when(disabled, |d| d.opacity(DISABLED_OPACITY))
                    // `cursor-default`: um comando não é um link.
                    .when(!disabled, |d| d.cursor(CursorStyle::Arrow))
                    // O `<span className="flex-1">` da demo: o rótulo toma o espaço e empurra o atalho
                    // pra direita (o `ms-auto` dele, que o GPUI não tem — não há `margin: auto`).
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(label.clone()),
                    );

                if let Some(atalho) = &self.rows[row].shortcut {
                    el = el.child(shortcut_row(atalho, window, cx));
                }

                if !disabled {
                    el = el
                        // `highlightItemOnHover` (default `true`) acende ao entrar. E **sair não
                        // apaga**: é o `keepHighlight` que o `Command` fixa em `true`
                        // (`resetOnPointerLeave: !keepHighlight`). É a diferença observável em relação
                        // ao [`crate::menu`] e ao [`crate::autocomplete`], que apagam.
                        .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                            if *hovered {
                                this.highlight(Some(row), cx);
                            }
                        }))
                        .on_click(cx.listener(move |this, _e, window, cx| {
                            crate::focus_ring::pointer_used(window);
                            this.choose(row, window, cx);
                        }));
                }
                el.into_any_element()
            }
        }
    }

    /// O `CommandFooter`, se algum dos dois slots existir.
    fn render_footer(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<Div> {
        if self.footer_start.is_none() && self.footer_end.is_none() {
            return None;
        }
        let p = palette();
        let mut rodape = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .gap(px(FOOTER_GAP))
            .w_full()
            .px(px(FOOTER_PAD_X))
            .py(px(FOOTER_PAD_Y))
            .border_t_1()
            .border_color(p.border.hsla())
            .text_size(px(SMALL_TEXT_SIZE))
            .line_height(px(SMALL_LINE_HEIGHT))
            .text_color(p.muted.hsla());
        if let Some(slot) = &self.footer_start {
            rodape = rodape.child(slot(window, cx));
        }
        if let Some(slot) = &self.footer_end {
            rodape = rodape.child(slot(window, cx));
        }
        Some(rodape)
    }
}

impl EventEmitter<CommandEvent> for Command {}

impl Render for Command {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // A rede de segurança do `autoHighlight="always"`: o efeito do Base UI roda a cada passada, e
        // não só quando a lista muda. É o que garante o destaque no PRIMEIRO frame (o construtor não lê
        // a busca) e depois de qualquer mudança que não tenha passado por `set_rows`.
        self.settle_highlight(false, cx);

        let visible = self.visible(cx);
        let inner = popup_inner_height(
            f32::from(window.viewport_size().height),
            self.max_height,
        );
        let rodape = self.render_footer(window, cx);
        let list_max = list_max_height(inner, rodape.is_some());

        // ⚠️ **Por que AÇÕES, e por que na fase de CAPTURA.** É o mesmo raciocínio (e as mesmas
        // referências ao núcleo) do `render` do [`crate::autocomplete::Autocomplete`]: o `InputState`
        // empilha o contexto de teclas `Input`, o `gpui_component::input::init` vincula
        // `up`→`MoveUp`/`down`→`MoveDown`/`enter`→`Enter` nele, e as ações são despachadas ANTES dos
        // ouvintes de tecla. Na fase de bolha o campo decidiria primeiro (e o `InputState::enter` emite
        // `InputEvent::PressEnter`); na de captura, quem manda com a lista aberta é este componente — e
        // aqui ela está sempre aberta.
        //
        // `Escape` **não** está aqui: ele é do [`crate::dialog`] (ver o doc do módulo).
        div()
            .flex()
            .flex_col()
            .w_full()
            .min_h(px(0.0))
            .capture_action(cx.listener(|this, _: &MoveDown, window, cx| {
                if this.arrow(1, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveUp, window, cx| {
                if this.arrow(-1, window, cx) {
                    cx.stop_propagation();
                }
            }))
            // Com `open` fixo em `true`, o `Enter` do `ComboboxInput`
            // (`if (activeIndex === null) { setOpen(false); return; } … clickHighlightedItem(…)`)
            // degenera no próprio destaque: com item aceso escolhe e consome; sem nenhum (lista vazia)
            // a tecla segue seu caminho, que é o que deixa um formulário hospedeiro submeter.
            .capture_action(cx.listener(|this, _: &Enter, window, cx| {
                if let Some(row) = this.highlighted {
                    crate::focus_ring::keyboard_used(window);
                    this.choose(row, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(self.render_input())
            .child(self.render_panel(&visible, list_max, window, cx))
            .children(rodape)
    }
}

// =================================================================================================
// Pedaços de desenho (funções livres — testáveis sem construir um `Command`)
// =================================================================================================

/// A posição de uma linha entre as VISÍVEIS — o índice que o
/// [`gpui::ScrollHandle::scroll_to_item`] entende, porque ele conta os filhos diretos do elemento
/// rastreado e os filhos da lista são exatamente as linhas visíveis.
///
/// Existe como função pura porque o erro provável aqui é silencioso: passar o índice da LINHA rolaria
/// pra outro lugar (ou pra nenhum) assim que um filtro escondesse qualquer linha anterior.
fn scroll_position(visible: &[usize], row: usize) -> Option<usize> {
    visible.iter().position(|&v| v == row)
}

/// A linha do `CommandEmpty`: `p-2 py-6 text-center text-sm text-muted-foreground`.
fn empty_row(texto: SharedString) -> Div {
    let p = palette();
    div()
        .flex_none()
        .w_full()
        .px(px(EMPTY_PAD_X))
        .py(px(EMPTY_PAD_Y))
        .text_size(px(TEXT_SIZE))
        .line_height(px(TEXT_LINE_HEIGHT))
        .text_center()
        .text_color(p.muted.hsla())
        .child(texto)
}

/// O `CommandShortcut`: `font-medium text-muted-foreground/72 text-xs` (sem o `tracking-widest`, que o
/// GPUI não tem), à direita da linha.
///
/// A cor é o token de atalho do [`crate::menu`] — a mesma classe, no mesmo papel, do `MenuShortcut`.
fn shortcut_row(atalho: &Shortcut, window: &mut Window, cx: &mut App) -> Div {
    match atalho {
        Shortcut::Text(t) => shortcut_box().child(t.clone()),
        // O slot é do call site: além do que a caixa herda, nada de estilo é imposto ao que ele
        // devolve — um `Kbd` desenha a própria pastilha.
        Shortcut::Slot(f) => shortcut_box().child(f(window, cx)),
    }
}

/// A **caixa** do atalho, sem conteúdo.
///
/// Está separada de [`shortcut_row`] pra ser testável: montar a variante de slot exige uma
/// [`gpui::Window`], e um teste que remontasse estas cinco linhas pra si estaria comparando o desenho
/// com uma cópia dele.
fn shortcut_box() -> Div {
    let p = palette();
    div()
        .flex_none()
        .text_size(px(SMALL_TEXT_SIZE))
        .line_height(px(SMALL_LINE_HEIGHT))
        .font_weight(FontWeight::MEDIUM)
        .text_color(p.shortcut.hsla())
}

/// A camada do `shadow-xs/5` do painel.
fn panel_shadow() -> Vec<gpui::BoxShadow> {
    let (dy, blur, spread) = PANEL_SHADOW;
    vec![gpui::BoxShadow {
        color: palette().shadow.hsla(),
        offset: gpui::point(px(0.0), px(dy)),
        blur_radius: px(blur),
        spread_radius: px(spread),
    }]
}

// =================================================================================================
// O paladar dentro do dialog
// =================================================================================================

/// O `CommandDialogPopup`: o [`Command`] dentro da superfície modal do [`crate::dialog`].
///
/// **Monte-o como último filho de uma raiz `relative().size_full()`** — é o contrato do
/// [`crate::dialog::dialog_layer`], e é ele que substitui o `Portal` da referência.
///
/// O que vem de graça do [`crate::dialog`] (e é por isso que este módulo não desenha camada modal
/// nenhuma): o backdrop `bg-black/32` que `occlude` o resto da tela, o clique fora que fecha, o
/// `Escape`, o salvamento e a restauração do foco, o fade de 200ms, a colocação vertical, o
/// `rounded-2xl`, a borda, o `bg-popover`, o `shadow-lg/5` e o bisel.
///
/// O que este envelope acrescenta, e que é do `command.tsx`:
///
/// - **`max-w-xl`** ([`POPUP_MAX_WIDTH`]), maior que o default do dialog;
/// - **a lavagem `before:bg-muted/72`**, que tinge o corpo do popup e é o que faz o painel de
///   resultados (`bg-popover`) "levantar" sobre a faixa da busca e a do rodapé. É o
///   [`crate::dialog::muted_wash`] — o mesmo token do rodapé do dialog, não uma segunda conta;
/// - **o raio de quem vive na padding box** ([`crate::dialog::FOOTER_RADIUS`]), pro retângulo da
///   lavagem não aparecer pra fora da curva da superfície.
pub fn command_dialog(dialog: &Entity<Dialog>, command: &Entity<Command>) -> impl IntoElement {
    dialog_layer(
        dialog,
        DialogPopup::bare(
            div()
                .flex()
                .flex_col()
                .w_full()
                .min_h(px(0.0))
                .bg(crate::dialog::muted_wash().hsla())
                .rounded(px(crate::dialog::FOOTER_RADIUS))
                .child(command.clone()),
        )
        .max_width(POPUP_MAX_WIDTH),
    )
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::theme;

    /// Uma lista com dois grupos, um separador e um comando desabilitado — cobre os três tipos de
    /// linha e os dois casos de poda.
    fn lista() -> Vec<CommandRow> {
        vec![
            CommandRow::group_label("Sugestões"),                 // 0
            CommandRow::item("Linear").shortcut("⌘L"),            // 1
            CommandRow::item("Figma").shortcut("⌘F"),             // 2
            CommandRow::separator(),                              // 3
            CommandRow::group_label("Comandos"),                  // 4
            CommandRow::item("Criar snippet").shortcut("⌘N"),     // 5
            CommandRow::item("Gerenciar janelas").disabled(true), // 6
        ]
    }

    /// A máscara alcançável da lista inteira, sem busca — o atalho que os testes de navegação usam.
    fn mask_de(rows: &[CommandRow], query: &str) -> Vec<bool> {
        let visible = visible_rows(rows, query);
        navigable_mask(rows.len(), &navigable_rows(rows, &visible))
    }

    // --- Filtragem ------------------------------------------------------------------------------

    /// **O filtro é o do [`crate::autocomplete`], reusado pelo [`FilterRow`]** — não uma segunda
    /// implementação. Este teste é o que prova que a ponte está ligada: se `match_text`/`is_label`/
    /// `is_divider` estivessem trocados, a poda sairia errada aqui e continuaria certa lá.
    #[test]
    fn o_filtro_e_o_do_autocomplete_reusado() {
        let rows = lista();

        // Busca vazia casa com tudo (`if (!query) return true`).
        assert_eq!(visible_rows(&rows, ""), vec![0, 1, 2, 3, 4, 5, 6]);

        // Substring em QUALQUER posição, insensível a caixa e a diacrítico.
        assert_eq!(visible_rows(&rows, "gma"), vec![0, 2], "casa no MEIO de \"Figma\"");
        assert_eq!(visible_rows(&rows, "LINEAR"), vec![0, 1], "caixa não conta");
        assert_eq!(
            visible_rows(&rows, "sugestoes").len(),
            0,
            "o rótulo de grupo NÃO é filtrado por si — nenhum comando casa com ele"
        );

        // Poda: o grupo que perdeu todos os itens some, e o separador sem lado some.
        assert_eq!(
            visible_rows(&rows, "snip"),
            vec![4, 5],
            "sobra o 2º grupo; o 1º morre com os itens dele e o separador perde o lado de antes"
        );
        assert_eq!(
            visible_rows(&rows, "ar"),
            vec![0, 1, 3, 4, 5, 6],
            "\"Linear\", \"Criar\" e \"Gerenciar\" casam nos DOIS grupos, então o separador fica"
        );
        assert!(visible_rows(&rows, "zzz").is_empty(), "nada casa: nem rótulo, nem filete");

        // O desabilitado é VISÍVEL (conta pro Empty) mas o teclado não o alcança.
        assert_eq!(visible_rows(&rows, "janelas"), vec![4, 6]);
        assert!(
            navigable_rows(&rows, &visible_rows(&rows, "janelas")).is_empty(),
            "o único resultado é desabilitado, então não há nada alcançável"
        );
    }

    /// **O filtro do Base UI não pontua e não reordena.**
    ///
    /// É a diferença que separa esta primitiva do `cmdk` (que quase todo mundo já viu), e o erro mais
    /// caro possível neste porte: `collator.contains` devolve `bool`, e a lista sai na ordem em que o
    /// autor escreveu — grupos inclusive. Um item que casa no início **não** sobe.
    #[test]
    fn o_filtro_nao_pontua_nem_reordena() {
        let rows = vec![
            CommandRow::item("Importar extensão"), // casa "ex" na posição 9
            CommandRow::item("Extensões"),         // casa "ex" na posição 0
        ];
        assert_eq!(
            visible_rows(&rows, "ex"),
            vec![0, 1],
            "o que casa no INÍCIO não sobe: sem `startsWith` privilegiado, sem pontuação"
        );

        // E a saída é sempre crescente — se algum dia alguém "melhorar" isto com um score, esta
        // afirmação é a que cai.
        let ordenado = visible_rows(&lista(), "");
        assert!(
            ordenado.windows(2).all(|p| p[0] < p[1]),
            "a ordem das linhas visíveis é a ordem declarada"
        );
    }

    // --- Navegação ------------------------------------------------------------------------------

    /// **O ciclo do destaque circula, e NÃO passa por "nenhum"** — a consequência direta do
    /// `autoHighlight="always"`, que zera o `allowEscape` do `useListNavigation`
    /// (`allowEscape: loopFocus && !autoHighlightMode`).
    ///
    /// É aqui que este componente se separa do [`crate::autocomplete`], onde o ciclo tem `len + 1`
    /// estados e o fim da lista devolve o destaque pro campo. Copiar aquele `step_highlight` pra cá
    /// daria um paladar de comandos em que a seta "apaga" a seleção no meio do caminho.
    #[test]
    fn a_navegacao_circula_sem_passar_por_nenhum() {
        let rows = lista();
        let mask = mask_de(&rows, "");
        // Alcançáveis: 1, 2 e 5 (o 6 é desabilitado; 0, 3 e 4 não são itens).
        assert_eq!(
            mask,
            vec![false, true, true, false, false, true, false],
            "a máscara é só dos comandos habilitados"
        );

        assert_eq!(step_index(Some(1), &mask, 1), Some(2));
        assert_eq!(step_index(Some(2), &mask, 1), Some(5), "pula rótulo e separador");
        assert_eq!(
            step_index(Some(5), &mask, 1),
            Some(1),
            "do último volta pro PRIMEIRO — sem parada em `None`"
        );
        assert_eq!(
            step_index(Some(1), &mask, -1),
            Some(5),
            "e do primeiro pra trás vai pro último"
        );
        // Sem nada aceso, a seta entra pela ponta certa.
        assert_eq!(step_index(None, &mask, 1), Some(1));
        assert_eq!(step_index(None, &mask, -1), Some(5));
        // Lista sem nada alcançável: não há pra onde ir.
        assert_eq!(step_index(Some(1), &[false, false], 1), None);
    }

    /// A máscara é **só troca de representação** dos índices que o [`crate::autocomplete`] devolveu —
    /// nenhuma regra nova. Se alguém puser um predicado aqui, a regra passa a existir em dois lugares.
    #[test]
    fn a_mascara_e_so_troca_de_representacao() {
        assert_eq!(navigable_mask(4, &[1, 3]), vec![false, true, false, true]);
        assert_eq!(navigable_mask(3, &[]), vec![false, false, false]);
        assert_eq!(navigable_mask(0, &[]), Vec::<bool>::new());

        // E ela concorda com a fonte, item por item.
        let rows = lista();
        let visible = visible_rows(&rows, "");
        let nav = navigable_rows(&rows, &visible);
        let mask = navigable_mask(rows.len(), &nav);
        assert_eq!(mask.len(), rows.len(), "uma casa por linha, inclusive as inalcançáveis");
        for (i, alcancavel) in mask.iter().enumerate() {
            assert_eq!(*alcancavel, nav.contains(&i), "linha {i}");
        }
    }

    /// **O destaque quando a lista muda debaixo dele** — o bug clássico deste componente, nos cinco
    /// casos do Base UI.
    #[test]
    fn o_destaque_volta_pro_primeiro_quando_a_busca_muda() {
        let mask = vec![false, true, true, false, true];

        // 1. A busca mudou: volta pro PRIMEIRO alcançável, sempre — é o que faz `Enter` depois de
        //    digitar escolher o primeiro resultado.
        assert_eq!(highlight_after_change(Some(4), &mask, true), Some(1));
        assert_eq!(highlight_after_change(None, &mask, true), Some(1));

        // 2. A lista mudou por outro motivo e a linha acesa continua alcançável: fica onde está.
        assert_eq!(highlight_after_change(Some(4), &mask, false), Some(4));

        // 3. A linha acesa deixou de ser alcançável (escondida pelo filtro, ou desabilitada): volta
        //    pro primeiro, e NÃO fica apontando pra linha errada.
        assert_eq!(highlight_after_change(Some(3), &mask, false), Some(1));

        // 4. Índice velho, fora da lista de agora: idem, sem estourar.
        assert_eq!(highlight_after_change(Some(99), &mask, false), Some(1));

        // 5. Nada alcançável: NENHUM. É o `setIndices({ activeIndex: null })` do efeito de rede — sem
        //    ele o `Enter` escolheria uma linha invisível.
        assert_eq!(highlight_after_change(Some(1), &[false, false], false), None);
        assert_eq!(highlight_after_change(Some(1), &[], true), None);

        // ⚠️ O invariante que sai de tudo isso: o resultado é sempre uma linha alcançável, ou nada.
        for atual in [None, Some(0), Some(3), Some(4), Some(99)] {
            for mudou in [true, false] {
                if let Some(r) = highlight_after_change(atual, &mask, mudou) {
                    assert!(mask[r], "destaque em linha não alcançável: {r}");
                }
            }
        }
    }

    /// O índice que vai pro [`gpui::ScrollHandle::scroll_to_item`] é a **posição visível**, não o
    /// índice da linha — e os dois só coincidem enquanto nada está filtrado.
    #[test]
    fn o_indice_da_rolagem_e_a_posicao_visivel() {
        // Sem filtro os dois coincidem, e é por isso que o erro passa despercebido.
        assert_eq!(scroll_position(&[0, 1, 2, 3], 2), Some(2));
        // Com as duas primeiras linhas escondidas, a linha 5 é o SEGUNDO filho da lista.
        assert_eq!(scroll_position(&[4, 5], 5), Some(1));
        assert_eq!(scroll_position(&[4, 5], 4), Some(0));
        // Uma linha que o filtro escondeu não tem posição nenhuma.
        assert_eq!(scroll_position(&[4, 5], 1), None);
    }

    // --- Geometria ------------------------------------------------------------------------------

    /// **As três medidas que o `command.tsx` de fato troca** em relação ao `autocomplete.tsx`.
    ///
    /// Todo o resto das classes é herdado, e é por isso que este módulo importa a geometria do
    /// [`crate::menu`]. Estas três são as que **não** se pode importar — e são exatamente as que se
    /// esquece de trocar, porque as duas listas são visualmente parecidas.
    #[test]
    fn as_tres_medidas_que_o_command_troca() {
        assert_eq!(LIST_PAD, 8.0, "not-empty:p-2");
        assert_eq!(ITEM_PAD_Y, 6.0, "py-1.5");
        assert_eq!(SEPARATOR_MARGIN_Y, 8.0, "my-2");

        // Cada uma é um passo de Tailwind acima da do menu/autocomplete, que é a herdada: `p-1`→`p-2`
        // e `my-1`→`my-2` dobram; `py-1`→`py-1.5` é uma vez e meia.
        assert_eq!(LIST_PAD, 2.0 * crate::menu::LIST_PAD, "p-1 → p-2");
        assert_eq!(ITEM_PAD_Y, 1.5 * crate::menu::ITEM_PAD_Y, "py-1 → py-1.5");
        assert_eq!(
            SEPARATOR_MARGIN_Y,
            2.0 * crate::menu::SEPARATOR_MARGIN_Y,
            "my-1 → my-2"
        );

        // O que é herdado continua herdado — se alguém copiar estas pra cá, o teste fica redundante e
        // a divergência começa.
        assert_eq!(ITEM_PAD_X, 8.0, "px-2, simétrico (importado do menu)");
        assert_eq!(ITEM_RADIUS, 6.0, "rounded-sm (importado)");
        assert_eq!(ITEM_MIN_HEIGHT, 28.0, "sm:min-h-7 (importado)");
    }

    /// **A altura do item é a entrelinha do Tailwind mais o `py-1.5`** — e o `min-h-7` deixa de mandar.
    ///
    /// O GPUI não usa a entrelinha do CSS: o default dele é `relative(phi())`, a razão de ouro. Sem
    /// fixar 14/20, a linha sairia com 22,65px e o item com 34,65 em vez de 32 — 8% mais alto, sem nada
    /// no código parecendo errado, e com o teto da lista mentindo por 2,65px por linha.
    #[test]
    fn a_linha_do_item_e_o_par_do_tailwind_mais_o_py_15() {
        const PHI: f32 = 1.618_034;

        assert_eq!(TEXT_SIZE, 14.0, "sm:text-sm");
        assert_eq!(TEXT_LINE_HEIGHT, 20.0, "o par do Tailwind pro text-sm");
        assert_eq!(item_height(), 32.0, "20 de linha + 2 × 6 de py-1.5");
        assert!(
            item_height() > ITEM_MIN_HEIGHT,
            "o py-1.5 passa do piso de min-h-7, então quem manda é a soma"
        );

        let sem_fixar = TEXT_SIZE * PHI;
        assert!(
            (sem_fixar - 22.652_476).abs() < 1e-3,
            "o default do GPUI pro corpo de 14px é 22,65px; veio {sem_fixar}"
        );
        assert!(
            (sem_fixar + 2.0 * ITEM_PAD_Y - 34.652_476).abs() < 1e-3,
            "sem fixar, o item mediria 34,65px em vez de 32"
        );
    }

    /// **A faixa do campo de busca fecha em 48px, como no original** — e a conta é diferente da dele.
    ///
    /// Lá: `py-1.5` + um campo `size="lg"` de 36px de border box (com a borda pintada de transparente).
    /// Aqui o [`crate::input::Input::unstyled`] **remove** a borda, então o campo mede 34 e o 1px de
    /// cada lado é devolvido ao respiro do embrulho. A soma é a mesma; o texto fica na mesma linha.
    #[test]
    fn a_faixa_do_campo_soma_48() {
        assert_eq!(INPUT_PAD_X, 10.0, "px-2.5");
        assert_eq!(INPUT_PAD_Y, 7.0, "py-1.5 (6) + a borda transparente (1)");
        assert_eq!(INPUT_SIZE.content_height(), 34.0, "o Lg sem borda");
        assert_eq!(INPUT_SIZE.height(), 36.0, "e COM borda, que é o h-9 da referência");
        assert_eq!(input_row_height(), 48.0);
        // A conta da referência, escrita por extenso: 6 + 36 + 6.
        assert_eq!(input_row_height(), 6.0 + 36.0 + 6.0, "a mesma soma do original");
    }

    /// **O teto da lista é o que sobra do popup** — e é a lista que rola, não a caixa.
    #[test]
    fn o_teto_da_lista_e_o_que_sobra() {
        assert_eq!(POPUP_MAX_HEIGHT, 420.0, "max-h-105 = 105 × 4px");
        assert_eq!(POPUP_MAX_WIDTH, 576.0, "max-w-xl = 36rem");
        assert_eq!(footer_height(), 41.0, "border-t + 2 × py-3 + a linha do text-xs");

        // Janela alta: quem manda é o `max-h-105`, e o interior é ele menos as duas bordas.
        let interior = popup_inner_height(1000.0, POPUP_MAX_HEIGHT);
        assert_eq!(interior, 418.0);
        assert_eq!(
            list_max_height(interior, false),
            418.0 - 48.0 - 1.0,
            "sem rodapé: interior − faixa do campo − borda de topo do painel"
        );
        assert_eq!(
            list_max_height(interior, true),
            418.0 - 48.0 - 1.0 - 41.0,
            "com rodapé, ele também sai da conta"
        );
        assert!(
            list_max_height(interior, true) < list_max_height(interior, false),
            "o rodapé come altura da lista, e não da janela"
        );

        // Janela baixa: quem manda é a janela menos os dois respiros do viewport do dialog.
        let apertado = popup_inner_height(300.0, POPUP_MAX_HEIGHT);
        assert_eq!(apertado, 300.0 - 2.0 * 16.0 - 2.0, "a janela menos p-4 e as bordas");
        assert!(apertado < interior, "num espaço apertado o interior encolhe");

        // Janela minúscula: o piso segura, e é uma linha inteira com o respiro da lista.
        assert_eq!(list_min_height(), 48.0, "uma linha de 32 + 2 × p-2");
        assert_eq!(list_max_height(popup_inner_height(60.0, POPUP_MAX_HEIGHT), true), 48.0);
    }

    /// Os raios: o painel arredonda por cima com `--radius-xl` e, quando não há rodapé, por baixo com o
    /// raio de quem vive na padding box do popup.
    #[test]
    fn os_raios_do_painel() {
        assert_eq!(PANEL_RADIUS_TOP, 14.0, "rounded-t-xl = --radius + 4");
        assert_eq!(
            crate::dialog::FOOTER_RADIUS,
            15.0,
            "calc(var(--radius-2xl) - 1px), com --radius-2xl = 16 (o default do Tailwind)"
        );
        assert!(
            PANEL_RADIUS_TOP < crate::dialog::FOOTER_RADIUS,
            "a quina de cima do painel é mais fechada que a curva do popup: ela não encosta nela"
        );
        assert_eq!(ITEM_RADIUS, 6.0, "e o item arredonda menos que os dois");
    }

    // --- Desenho --------------------------------------------------------------------------------

    /// A sombra do painel é o `shadow-xs/5`: **uma** camada, sem spread — e não as duas do
    /// `shadow-lg/5` do popup, que é o erro fácil por estarem na mesma superfície.
    #[test]
    fn a_sombra_do_painel_e_o_shadow_xs() {
        theme::set_theme(theme::ThemeMode::Dark);
        let camadas = panel_shadow();
        assert_eq!(camadas.len(), 1, "--shadow-xs tem UMA camada; a do popup tem duas");
        assert_eq!(camadas[0].offset.y, px(1.0));
        assert_eq!(camadas[0].offset.x, px(0.0), "só desce, não desloca de lado");
        assert_eq!(camadas[0].blur_radius, px(2.0));
        assert_eq!(camadas[0].spread_radius, px(0.0), "o xs não tem spread negativo");
        // O `/5` do Tailwind troca a cor inteira por preto 5%.
        assert!((camadas[0].color.a - 13.0 / 255.0).abs() < 1e-3, "preto 5%");
    }

    /// A linha de "nenhum resultado" tem o `py-6` que o `CommandEmpty` acrescenta ao `p-2` da base — e
    /// o par corpo/entrelinha do Tailwind, nos dois temas.
    #[test]
    fn a_linha_de_nenhum_resultado_tem_py_6() {
        let px_len = |v: f32| Some(gpui::AbsoluteLength::Pixels(px(v)));
        let alt = |v: f32| Some(gpui::DefiniteLength::Absolute(gpui::AbsoluteLength::Pixels(px(v))));

        assert_eq!(EMPTY_PAD_Y, 24.0, "not-empty:py-6");
        assert_eq!(EMPTY_PAD_X, 8.0, "e o p-2 horizontal da base");
        assert!(EMPTY_PAD_Y > EMPTY_PAD_X, "o py-6 sobrepõe o py-2 da base");

        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let mut linha = empty_row("Nenhum resultado.".into());
            let e = linha.style();
            let texto = e.text.clone().expect("o Empty declara estilo de texto");
            assert_eq!(texto.font_size, px_len(14.0), "{modo:?}: sm:text-sm");
            assert_eq!(texto.line_height, alt(20.0), "{modo:?}: 14/20, não a razão de ouro");
            assert_eq!(texto.color, Some(palette().muted.hsla()), "{modo:?}: muted-foreground");
            assert_eq!(e.padding.top, alt(EMPTY_PAD_Y), "{modo:?}: py-6");
            assert_eq!(e.padding.left, alt(EMPTY_PAD_X), "{modo:?}: px-2");
        }
    }

    /// O `CommandShortcut` usa o **token de atalho do [`crate::menu`]** — o mesmo
    /// `text-muted-foreground/72` do `MenuShortcut`, e não o `--muted-foreground` cheio.
    #[test]
    fn o_atalho_usa_o_token_do_menu() {
        let px_len = |v: f32| Some(gpui::AbsoluteLength::Pixels(px(v)));
        let alt = |v: f32| Some(gpui::DefiniteLength::Absolute(gpui::AbsoluteLength::Pixels(px(v))));

        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let p = palette();
            // O `/72` MULTIPLICA o alfa do `--muted-foreground` (Tailwind v4).
            let cheio = p.muted.alpha();
            let atalho = p.shortcut.alpha();
            assert!(
                (atalho - cheio * 0.72).abs() < 2e-3,
                "{modo:?}: o atalho é 72% do muted; veio {atalho}"
            );
            assert!(atalho < cheio, "{modo:?}: e é mais apagado que o rótulo do grupo");

            let mut caixa = shortcut_box();
            let s = caixa.style();
            let texto = s.text.clone().expect("o atalho declara estilo de texto");
            assert_eq!(texto.font_size, px_len(12.0), "{modo:?}: text-xs");
            assert_eq!(texto.line_height, alt(16.0), "{modo:?}: 12/16");
            assert_eq!(texto.font_weight, Some(FontWeight::MEDIUM), "{modo:?}: font-medium");
            assert_eq!(texto.color, Some(p.shortcut.hsla()), "{modo:?}: muted-foreground/72");
        }
    }

    // --- As linhas ------------------------------------------------------------------------------

    /// O construtor diz o que a linha é, e os decoradores que não se aplicam são ignorados em vez de
    /// virarem outra coisa em silêncio.
    #[test]
    fn a_linha_sabe_o_que_e() {
        let item = CommandRow::item("Linear").shortcut("⌘L");
        assert_eq!(item.match_text(), Some("Linear"));
        assert_eq!(item.label().map(SharedString::to_string), Some("Linear".into()));
        assert!(item.is_row_item() && item.keyboard_reachable());
        assert!(!item.is_label() && !item.is_divider());

        let desab = CommandRow::item("Gerenciar janelas").disabled(true);
        assert!(desab.is_row_item(), "desabilitado continua sendo item (conta pro Empty)");
        assert!(!desab.keyboard_reachable(), "mas o teclado não o alcança");

        let rotulo = CommandRow::group_label("Sugestões").disabled(true);
        assert!(rotulo.is_label() && !rotulo.is_row_item() && !rotulo.keyboard_reachable());
        assert_eq!(rotulo.match_text(), None, "rótulo não é filtrado por si");
        assert_eq!(rotulo.label(), None, "e não é rótulo de comando");

        let sep = CommandRow::separator().shortcut("⌘X");
        assert!(sep.is_divider() && !sep.is_row_item() && !sep.keyboard_reachable());

        // O slot de elemento é o outro caminho do atalho — quem desenha é o call site.
        let com_slot = CommandRow::item("Figma").shortcut_slot(|_w, _cx| div().into_any_element());
        assert!(com_slot.shortcut.is_some());
    }
}

/// **O que só a janela mede.**
///
/// Os testes de unidade acima trancam as *regras* ([`highlight_after_change`],
/// [`crate::menu::step_index`], [`crate::autocomplete::visible_rows`]). Estes trancam três coisas que
/// nenhuma função pura vê:
///
/// 1. **O destaque no PRIMEIRO frame.** O `autoHighlight="always"` depende da busca, que mora no
///    [`InputState`] — o construtor não a lê, então a âncora só existe depois de um `render`. É o
///    invariante mais visível do componente e o mais fácil de perder num refactor.
/// 2. **A tecla chega?** `↑ ↓ Enter` estão vinculadas a ações no contexto de teclas do [`InputState`],
///    e o campo tem ouvinte próprio pra elas. Só um teste com um campo de verdade **focado** responde
///    se o roteamento funciona — e, dentro do dialog, se ele funciona com a superfície do modal no
///    caminho.
/// 3. **`keepHighlight`.** "Sair com o mouse não apaga" é a AUSÊNCIA de um ramo no `on_hover`; só um
///    evento de mouse de verdade prova que a ausência é a certa.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use crate::dialog::Dialog;
    use gpui::{point, AppContext as _, Modifiers, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// O que os eventos capturados guardam.
    type Eventos = Rc<RefCell<Vec<CommandEvent>>>;

    /// Abre uma janela com um paladar de quatro linhas (a última desabilitada) e **o campo já focado**.
    ///
    /// O [`gpui_component::Root`] na raiz não é enfeite: o campo de texto do núcleo chama `Root::read`
    /// no caminho de teclado, e sem ele qualquer tecla derruba o teste com "the window root view should
    /// be of type `ui::Root`".
    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
    ) -> (
        Entity<Command>,
        Entity<InputState>,
        Eventos,
        VisualTestContext,
    ) {
        let (cmd, estado, eventos, vcx) = montar(cx, |cmd, _dialog| cmd.clone().into_any_element());
        (cmd, estado, eventos, vcx)
    }

    /// A montagem comum: cria o `InputState`, o [`Command`], um [`Dialog`] e deixa o call site decidir
    /// o que vai na árvore (o paladar cru, ou ele dentro do [`command_dialog`]).
    #[allow(clippy::type_complexity)]
    fn montar(
        cx: &mut TestAppContext,
        arvore: impl Fn(&Entity<Command>, &Entity<Dialog>) -> AnyElement + 'static,
    ) -> (
        Entity<Command>,
        Entity<InputState>,
        Eventos,
        VisualTestContext,
    ) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let eventos: Eventos = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();
        let mut saida = None;

        let window = cx.add_window(|window, cx| {
            let estado = cx.new(|cx| crate::input::single_line(window, cx));
            // Os dois slots de rodapé e o `shortcut_slot` entram aqui pra que TODO teste de janela
            // também monte esses caminhos de desenho — é a cobertura de fumaça deles (nada afirma
            // sobre o que eles desenham; ver "Sem cobertura de teste" no doc do módulo).
            let cmd = cx.new(|cx| {
                Command::new(&estado, cx)
                    .rows(vec![
                        CommandRow::item("Linear").shortcut("⌘L"),
                        CommandRow::item("Limão")
                            .shortcut_slot(|_w, _cx| div().child("⌘M").into_any_element()),
                        CommandRow::item("Figma").shortcut("⌘F"),
                        CommandRow::item("Lichia").disabled(true),
                    ])
                    .empty("Nenhum resultado.")
                    .footer_start(|_w, _cx| div().child("Navegar").into_any_element())
                    .footer_end(|_w, _cx| div().child("Esc").into_any_element())
            });
            cx.subscribe(&cmd, move |_this, _e, ev: &CommandEvent, _cx| {
                capturados.borrow_mut().push(ev.clone());
            })
            .detach();
            let dialog = cx.new(Dialog::new);
            // O `autoFocus` do `CommandInput`, que aqui é explícito: sem o campo focado as ações do
            // núcleo nem entram no caminho de despacho e nenhuma tecla mediria nada (é o que o teste
            // `dentro_do_dialog_as_teclas_chegam_e_o_escape_fecha` mede de propósito).
            cmd.read(cx).focus_input(window, cx);
            saida = Some((cmd.clone(), estado, dialog.clone()));
            let hospede = cx.new(|_| Hospede::new(cmd, dialog, arvore));
            gpui_component::Root::new(hospede, window, cx)
        });

        let (cmd, estado, _dialog) = saida.expect("paladar montado");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        (cmd, estado, eventos, vcx)
    }

    /// A view de teste que hospeda a árvore. Uma raiz `relative().size_full()`, que é o contrato do
    /// [`crate::dialog::dialog_layer`].
    struct Hospede {
        cmd: Entity<Command>,
        dialog: Entity<Dialog>,
        #[allow(clippy::type_complexity)]
        arvore: Box<dyn Fn(&Entity<Command>, &Entity<Dialog>) -> AnyElement>,
    }

    impl Hospede {
        fn new(
            cmd: Entity<Command>,
            dialog: Entity<Dialog>,
            arvore: impl Fn(&Entity<Command>, &Entity<Dialog>) -> AnyElement + 'static,
        ) -> Self {
            Self {
                cmd,
                dialog,
                arvore: Box::new(arvore),
            }
        }
    }

    impl Render for Hospede {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .relative()
                .size_full()
                .child((self.arvore)(&self.cmd, &self.dialog))
        }
    }

    /// **O primeiro comando nasce destacado** — o `autoHighlight="always"`.
    ///
    /// ⚠️ É o oposto do [`crate::autocomplete`], que nasce sem nada aceso (`autoHighlight` no default
    /// `false`) e precisa de duas setas pra acender o primeiro item. Aqui o `Enter` funciona no primeiro
    /// frame, sem nenhuma seta — é o gesto que define um paladar de comandos.
    #[gpui::test]
    fn o_primeiro_comando_nasce_destacado(cx: &mut TestAppContext) {
        let (cmd, _estado, _eventos, vcx) = abrir(cx);
        assert_eq!(
            vcx.read(|cx| cmd.read(cx).highlighted()),
            Some(0),
            "sem digitar nada e sem seta nenhuma, o primeiro comando já está aceso"
        );
    }

    /// **Digitar filtra e reancora no primeiro resultado**, e o `Enter` logo depois escolhe justamente
    /// ele.
    #[gpui::test]
    fn digitar_filtra_e_reancora_no_primeiro(cx: &mut TestAppContext) {
        let (cmd, estado, eventos, mut vcx) = abrir(cx);

        // Anda até o TERCEIRO item antes de digitar: é isso que faz o teste medir a reancoragem, e não
        // o estado inicial.
        vcx.simulate_keystrokes("down down");
        assert_eq!(vcx.read(|cx| cmd.read(cx).highlighted()), Some(2));

        // ⚠️ Uma busca que mantém a linha acesa VISÍVEL — é o que separa "reancorou" de "a linha velha
        // sumiu e sobrou o primeiro". Todas as quatro linhas têm um "i".
        vcx.simulate_keystrokes("i");
        assert_eq!(
            vcx.read(|cx| cmd.read(cx).visible(cx)),
            vec![0, 1, 2, 3],
            "\"i\" casa com as quatro"
        );
        assert_eq!(
            vcx.read(|cx| cmd.read(cx).highlighted()),
            Some(0),
            "o destaque VOLTOU pro primeiro, mesmo com a linha velha ainda na lista"
        );

        vcx.simulate_keystrokes("g");
        assert_eq!(vcx.read(|cx| estado.read(cx).value()), "ig");
        assert_eq!(
            vcx.read(|cx| cmd.read(cx).visible(cx)),
            vec![2],
            "só \"Figma\" casa com \"ig\""
        );
        assert_eq!(
            vcx.read(|cx| cmd.read(cx).highlighted()),
            Some(2),
            "e agora o primeiro resultado é ele"
        );

        vcx.simulate_keystrokes("enter");
        assert_eq!(
            vcx.read(|cx| estado.read(cx).value()),
            "Figma",
            "o Enter escolheu o único resultado e completou o campo (fillInputOnItemPress)"
        );

        // A ordem observável: o `Select` sai de `choose`, e o `Query` do eco do `InputState`.
        let evs = eventos.borrow().clone();
        let sel = evs
            .iter()
            .position(|e| *e == CommandEvent::Select(2))
            .expect("emitiu Select com o índice da LINHA");
        assert_eq!(
            evs[sel + 1],
            CommandEvent::Query("Figma".into()),
            "e o Query do rótulo escolhido vem depois dele"
        );
    }

    /// **A busca sem resultado apaga o destaque** — e aí o `Enter` não tem o que escolher.
    #[gpui::test]
    fn sem_resultado_nao_ha_destaque_nem_escolha(cx: &mut TestAppContext) {
        let (cmd, estado, eventos, mut vcx) = abrir(cx);

        vcx.simulate_keystrokes("z z");
        assert!(vcx.read(|cx| cmd.read(cx).visible(cx)).is_empty());
        assert_eq!(
            vcx.read(|cx| cmd.read(cx).highlighted()),
            None,
            "sem nada alcançável, o destaque sai — senão o Enter escolheria uma linha invisível"
        );

        // ⚠️ A ação, e não `simulate_keystrokes("enter")`: sem destaque o `Enter` não é consumido, e o
        // `Keystroke::with_simulated_ime` do GPUI dá a "enter" um `key_char` de `"\n"`, que o
        // `dispatch_keystroke` insere no campo quando ninguém consumiu — num `InputState` de UMA linha
        // isso derruba o `text_system`. É artefato do simulador, e vale pra qualquer campo de uma linha
        // desta lib (mesma nota do `crate::autocomplete`).
        vcx.dispatch_action(Enter { secondary: false });
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| estado.read(cx).value()), "zz", "nada foi escolhido");
        assert!(
            !eventos
                .borrow()
                .iter()
                .any(|e| matches!(e, CommandEvent::Select(_))),
            "e nenhum Select foi emitido"
        );
    }

    /// **As setas circulam sem passar por "nenhum"**, pulam o desabilitado, e o `Enter` escolhe.
    #[gpui::test]
    fn as_setas_circulam_e_o_enter_escolhe(cx: &mut TestAppContext) {
        let (cmd, estado, _eventos, mut vcx) = abrir(cx);
        let aceso = |vcx: &VisualTestContext| vcx.read(|cx| cmd.read(cx).highlighted());

        assert_eq!(aceso(&vcx), Some(0));
        vcx.simulate_keystrokes("down");
        assert_eq!(aceso(&vcx), Some(1));
        vcx.simulate_keystrokes("down");
        assert_eq!(aceso(&vcx), Some(2), "o 3º item");
        vcx.simulate_keystrokes("down");
        assert_eq!(
            aceso(&vcx),
            Some(0),
            "\"Lichia\" é desabilitada, então o fim da lista volta direto pro primeiro — sem parada \
             em `None`, que é o que o autocomplete faria"
        );
        vcx.simulate_keystrokes("up");
        assert_eq!(aceso(&vcx), Some(2), "e pra trás dá a volta pelo último alcançável");

        vcx.simulate_keystrokes("enter");
        assert_eq!(vcx.read(|cx| estado.read(cx).value()), "Figma");
    }

    /// **`keepHighlight`: sair com o mouse NÃO apaga o destaque.**
    ///
    /// É a única das quatro props fixadas cujo efeito é a ausência de um ramo de código — o
    /// `resetOnPointerLeave: !keepHighlight` do Base UI. No [`crate::menu`] e no
    /// [`crate::autocomplete`] o `on_hover(false)` apaga; aqui não pode.
    ///
    /// A aritmética das coordenadas é a geometria declarada do componente: a faixa do campo tem
    /// [`input_row_height`], depois vem a borda de topo do painel e o `p-2` da lista, e cada linha mede
    /// [`item_height`].
    #[gpui::test]
    fn sair_com_o_mouse_nao_apaga_o_destaque(cx: &mut TestAppContext) {
        let (cmd, _estado, _eventos, mut vcx) = abrir(cx);
        let aceso = |vcx: &VisualTestContext| vcx.read(|cx| cmd.read(cx).highlighted());

        let topo_da_lista = input_row_height() + BORDER + LIST_PAD;
        let centro_da_linha = |i: f32| px(topo_da_lista + item_height() * (i + 0.5));

        // Passar o mouse na 2ª linha acende ela (`highlightItemOnHover`, default `true`).
        vcx.simulate_mouse_move(point(px(120.0), centro_da_linha(1.0)), None, Modifiers::none());
        vcx.run_until_parked();
        assert_eq!(aceso(&vcx), Some(1), "o hover acendeu a linha 1");

        // Sair da lista NÃO apaga.
        vcx.simulate_mouse_move(point(px(120.0), px(2000.0)), None, Modifiers::none());
        vcx.run_until_parked();
        assert_eq!(
            aceso(&vcx),
            Some(1),
            "keepHighlight: o destaque fica onde o ponteiro deixou"
        );
    }

    /// **Dentro do dialog: as teclas só chegam depois do [`Command::focus_input`], e o `Escape` fecha.**
    ///
    /// Este é o teste que prova a composição inteira do [`command_dialog`] — e que o `focus_input` é
    /// load-bearing, não cosmético: o [`crate::dialog::Dialog::open`] foca a SUPERFÍCIE do modal, e com
    /// ela focada a árvore deste componente não está no caminho de despacho da ação.
    #[gpui::test]
    fn dentro_do_dialog_as_teclas_chegam_e_o_escape_fecha(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let mut saida = None;
        let window = cx.add_window(|window, cx| {
            let estado = cx.new(|cx| crate::input::single_line(window, cx));
            let cmd = cx.new(|cx| {
                Command::new(&estado, cx).rows(vec![
                    CommandRow::item("Linear"),
                    CommandRow::item("Figma"),
                ])
            });
            let dialog = cx.new(Dialog::new);
            saida = Some((cmd.clone(), dialog.clone()));
            let hospede = cx.new(|_| {
                Hospede::new(cmd, dialog, |cmd, dialog| {
                    command_dialog(dialog, cmd).into_any_element()
                })
            });
            gpui_component::Root::new(hospede, window, cx)
        });
        let (cmd, dialog) = saida.expect("paladar montado");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        // Fechado, o paladar nem é montado: nada foi renderizado, então nada está aceso.
        assert!(!vcx.read(|cx| dialog.read(cx).is_open()));
        assert_eq!(vcx.read(|cx| cmd.read(cx).highlighted()), None);

        vcx.update(|window, cx| {
            dialog.update(cx, |d, cx| d.open(window, cx));
        });
        vcx.run_until_parked();
        assert!(vcx.read(|cx| dialog.read(cx).is_open()), "o dialog abriu");
        assert_eq!(
            vcx.read(|cx| cmd.read(cx).highlighted()),
            Some(0),
            "e o primeiro frame já ancorou o destaque"
        );

        // Sem focar o campo, a seta não chega: quem tem o foco é a superfície do modal.
        vcx.simulate_keystrokes("down");
        assert_eq!(
            vcx.read(|cx| cmd.read(cx).highlighted()),
            Some(0),
            "com a superfície focada a ação nem passa pela árvore do paladar"
        );

        vcx.update(|window, cx| {
            cmd.read(cx).focus_input(window, cx);
        });
        vcx.run_until_parked();
        vcx.simulate_keystrokes("down");
        assert_eq!(
            vcx.read(|cx| cmd.read(cx).highlighted()),
            Some(1),
            "com o campo focado, a seta chega — é o que o `focus_input` compra"
        );

        // O `Escape` é do dialog: ele atravessa o campo focado e fecha o modal.
        vcx.simulate_keystrokes("escape");
        vcx.run_until_parked();
        assert!(
            !vcx.read(|cx| dialog.read(cx).is_open()),
            "o Escape fechou o dialog — este componente não trata a tecla"
        );
    }
}

//! `Combobox` — o **campo de texto que filtra uma lista** do `empire-ui`, com o visual do design
//! system [coss][1] e o comportamento do [Base UI][2].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/combobox.tsx`
//! [2]: https://github.com/mui/base-ui — `packages/react/src/combobox/`
//!
//! # O que ele é (e o que ele NÃO é)
//!
//! É o [`crate::select::Select`] com um campo de texto no lugar do gatilho: em vez de escolher numa
//! lista fechada, você **digita** e a lista se reduz ao que casa. Duas coisas o distinguem dos
//! vizinhos desta lib:
//!
//! | componente | escolha | campo | seleção múltipla |
//! |---|---|---|---|
//! | [`crate::select::Select`] | 1-de-N | não (gatilho com o valor) | não |
//! | [`crate::menu::Menu`] | comando | não | — |
//! | **`Combobox`** | 1-de-N ou N-de-N | **sim, e filtra** | **sim, em pastilhas** |
//!
//! O irmão dele na referência é o `Autocomplete`, que no Base UI é literalmente o **mesmo**
//! componente com `selectionMode="none"` (`autocomplete/root/AutocompleteRoot.tsx:126`): o texto
//! digitado é o valor (sugestão livre), não há seleção pra marcar, então não há coluna de check nem
//! pastilhas. Este módulo é o Combobox; o Autocomplete é outro porte.
//!
//! # Anatomia
//!
//! ```text
//! seleção ÚNICA                          seleção MÚLTIPLA (pastilhas)
//! ┌──────────────────────────────┐       ┌──────────────────────────────┐
//! │ mult|                     ⇕ │       │ [Normal ✕] [Screen ✕] mu|   │
//! └──────────────────────────────┘       └──────────────────────────────┘
//!   ┌────────────────────────────┐
//!   │ Escuros                    │  ← rótulo de grupo (text-xs, muted)
//!   │ ✓  Multiply                │  ← item: coluna do check (16px) + rótulo
//!   │    Multiply Alpha          │
//!   │ ────────────────────────── │  ← separador (mx-2 my-1 h-px)
//!   │ Nada encontrado            │  ← ComboboxEmpty
//!   │ carregando…                │  ← ComboboxStatus (rodapé, fora da lista rolável)
//!   └────────────────────────────┘
//! ```
//!
//! # Uso
//!
//! ```ignore
//! let cb = cx.new(|cx| {
//!     Combobox::new(
//!         vec![
//!             ComboboxItem::group_label("Modos"),
//!             ComboboxItem::new("Normal"),
//!             ComboboxItem::new("Multiply"),
//!             ComboboxItem::new("Screen").disabled(true),
//!         ],
//!         window,
//!         cx,
//!     )
//!     .multiple()
//! });
//!
//! cx.subscribe(&cb, |_this, _cb, ev: &ComboboxEvent, _cx| match ev {
//!     ComboboxEvent::Change(idx) => println!("selecionado: {idx:?}"),
//!     ComboboxEvent::Query(q) => println!("filtro: {q}"),
//!     ComboboxEvent::OpenChange(open) => println!("aberto: {open}"),
//! })
//! .detach();
//! ```
//!
//! # As três reutilizações que definem este porte
//!
//! O `.tsx` da referência é uma **casca** sobre o `Combobox` do Base UI que compõe o `Input` e a
//! `ScrollArea` do próprio coss. Traduzido, isso quer dizer que quase nada aqui é desenho novo:
//!
//! 1. **O popup e o item são os do [`crate::select::Select`]/[`crate::menu::Menu`].** As classes
//!    casam uma a uma com as constantes que já existiam — `grid-cols-[1rem_1fr]` é o
//!    `select::CHECK_SLOT`, `ps-2`/`pe-4` são `select::ITEM_PAD_START`/`ITEM_PAD_END`, `gap-2` é o
//!    `menu::ITEM_GAP`, `sm:min-h-7` é o `menu::ITEM_MIN_HEIGHT`, `rounded-sm` é o
//!    `menu::ITEM_RADIUS`, `p-1` é o `menu::LIST_PAD`, e a superfície (`rounded-lg border bg-popover
//!    shadow-lg/5` + o bisel `before:`) é **string por string** a do `MenuPopup`. Então nada disso
//!    foi reescrito: os módulos vizinhos tiveram os itens promovidos a `pub(crate)` e este consome.
//!    Uma terceira superfície de popup (ou uma terceira geometria de item) é como um design system
//!    começa a divergir de si mesmo.
//! 2. **O campo é o [`crate::input::Input`].** A referência renderiza o `ComboboxInput` com
//!    `render={<Input nativeInput size={…} />}`, ou seja o campo de texto do coss inteiro — moldura,
//!    borda por estado, bisel, anel de foco de 3px, tamanhos. Aqui é literalmente o nosso `Input`,
//!    apontando pro `InputState` do `gpui-component` (que já resolve IME, clusters de grafema,
//!    seleção e undo). O `size` também é o dele: [`InputSize`], e não um quarto enum de tamanho.
//! 3. **O caminhador da lista e as cores vêm de lá.** `menu::edge_index` acha a ponta,
//!    `select::border_color_for`/`shadow_stack_for`/`bevel_for`/`ring_overlay` dão os estados da
//!    moldura das pastilhas, e as duas paletas (`menu::palette` pro popup, `select::palette` pro
//!    campo) são as mesmas que o menu e o dropdown já pintam. **Este módulo não tem paleta própria**
//!    — de propósito.
//!
//! O que **não** existia e por isso é desenhado aqui: as **pastilhas** (o `ComboboxChips`, que é uma
//! moldura de campo com `flex-wrap`), o **botão de afordância** dentro do campo (gatilho/limpar) e o
//! **filtro**.
//!
//! # O comportamento vem do Base UI, não do `.tsx`
//!
//! O arquivo da referência tem 435 linhas de classe e **zero** linha de comportamento. Filtrar,
//! destacar, abrir, fechar, o que o Enter faz: tudo isso é do `@base-ui/react/combobox`, e quase
//! nada disso está publicado na documentação — o site só publica a tabela de props. Então as
//! citações abaixo (e as espalhadas pelo código) apontam pro **código-fonte**, lido no `master` do
//! `mui/base-ui` em 2026-08-04. O contrato inteiro vive num arquivo só,
//! `combobox/root/AriaCombobox.tsx`, compartilhado com o Autocomplete via `selectionMode`.
//!
//! O que foi confirmado e portado:
//!
//! - **filtro**: substring (não prefixo), sem caixa, sem acento, ignorando pontuação —
//!   `internals/filter.ts:6-21` monta `new Intl.Collator(locale, { usage: 'search', sensitivity:
//!   'base', ignorePunctuation: true })` e faz `contains` por janela deslizante. Filtro vazio casa
//!   com tudo (`filter.ts:25`), e a consulta é **aparada** (`AriaCombobox.tsx:239`).
//! - **destaque ao abrir**: o item **SELECIONADO**, não o primeiro nem nenhum — `AriaCombobox.tsx:
//!   1180-1181` + `useListNavigation.ts:372-379`, e no modo múltiplo é o **último** da seleção
//!   (`findSelectionIndex(..., multiple)`). Sem seleção, abrir por clique não destaca nada
//!   (`useListNavigation.ts:417-421`).
//! - **as setas circulam PASSANDO PELO CAMPO**: `último item → nenhum destaque → primeiro item`,
//!   porque `loopFocus` é `true` e `allowEscape = loopFocus && !autoHighlight`
//!   (`AriaCombobox.tsx:113,1179`). O JSDoc é explícito: *"The input is always included in the focus
//!   loop per ARIA Authoring Practices."*
//! - **digitar apaga o destaque** (com `autoHighlight` no default `false`) e devolve a lista pro topo
//!   — `ComboboxInput.tsx:334-336`, `AriaCombobox.tsx:568-584`.
//! - **`Home`/`End` movem o CURSOR DO TEXTO**, não o destaque — `ComboboxInput.tsx:352-366` chama
//!   `stopEvent` e `setSelectionRange`.
//! - **abrir**: clique (mousedown) no campo, com `openOnInputClick` no default **`true`** pro
//!   Combobox e `false` pro Autocomplete (`AriaCombobox.tsx:109` vs
//!   `AutocompleteRoot.tsx:36`) — e `toggle: false`, ou seja clicar de novo **não** fecha
//!   (`AriaCombobox.tsx:1139-1147`); digitar (`ComboboxInput.tsx:271-281`, só com texto não vazio);
//!   `↓`/`↑`; e o clique no gatilho, que ESSE alterna (`ComboboxTrigger.tsx:121-124`). **Foco sozinho
//!   não abre** — não há `useFocus` no root.
//! - **`Escape` tem DOIS comportamentos**: aberto, fecha e **descarta** a consulta; fechado, limpa o
//!   texto **e** a seleção (`ComboboxInput.tsx:368-384`).
//! - **`Enter`**: com destaque, é idêntico ao clique (`clickHighlightedItem` sintetiza um clique de
//!   verdade, `combobox/utils/parts.ts:47-59`); **sem** destaque, só fecha o popup e deixa o evento
//!   passar (*"Allow form submission when no item is highlighted"*, `ComboboxInput.tsx:436-438`).
//! - **escolher no modo único** fecha sempre e escreve o rótulo no campo
//!   (`AriaCombobox.tsx:702-711,758-766`); no **múltiplo** alterna, e só fecha se o usuário estava
//!   filtrando (`AriaCombobox.tsx:731-757`).
//! - **ao fechar sem escolher**, o texto é reconciliado: no único volta pro rótulo do selecionado (ou
//!   vazio), no múltiplo é limpo — `handleUnmount`, `AriaCombobox.tsx:777-818`.
//! - **esvaziar o campo limpa a seleção única** — `ComboboxInput.tsx:316-327`.
//! - **`Backspace` no campo vazio remove a última pastilha** — `ComboboxInput.tsx:386-409`.
//! - **`Clear` limpa o texto E a seleção**, e devolve o foco pro campo
//!   (`combobox/clear/ComboboxClear.tsx:101-126`); e ele **desaparece** quando não há valor
//!   (`:55-62`, `keepMounted` default `false`).
//! - **`Empty` aparece quando a lista filtrada está vazia, com ou sem consulta** —
//!   `combobox/empty/ComboboxEmpty.tsx:33`, e os testes `ComboboxEmpty.test.tsx:28,53,77,101`.
//! - **`Status` não tem lógica nenhuma**: é a região viva pra estado assíncrono
//!   (`combobox/status/ComboboxStatus.tsx:7-17`).
//!
//! # Contrato
//!
//! Mesmo contrato dos outros controles: é um `Entity` (view) próprio que guarda o seu estado e
//! **emite** [`ComboboxEvent`]. Quem usa assina (`cx.subscribe`) e grava onde quiser. Os `set_*`
//! sincronizam de fora **sem** emitir — evita loop de feedback (a mesma regra do
//! `Select::set_selected`).
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! ## Não reproduzível
//!
//! - **`transition-[scale,opacity]` do popup e `transition-opacity` do botão de afordância.** O GPUI
//!   0.2.2 não tem transição declarativa de estilo. O popup entra e sai instantâneo. (O
//!   [`crate::input::Input`] anima o anel de foco à mão, por frame; um popup inteiro exigiria o mesmo
//!   truque num elemento `deferred`, e não vale o custo.) Junto com isso cai o `closeQuery` do Base
//!   UI, que congela a consulta durante a animação de saída pra a lista não piscar sem filtro
//!   (`AriaCombobox.tsx:644-670`): sem animação de saída não há o que congelar.
//! - **`origin-(--transform-origin)`.** Não há `transform` em `div` no GPUI, então não há origem de
//!   transformação — e sem escala não há o que originar.
//! - **`pointer-coarse:after:min-h-11`** (alvo de toque de 44px no botão de afordância). É uma
//!   media query de ponteiro grosso; num app de desktop ela nunca casa.
//! - **`z-50` do positioner.** Não existe `z-index` no GPUI; a ordem é a de pintura, e o popup vai
//!   em [`gpui::deferred`] com prioridade — que é o que `z-50` queria dizer.
//! - **`--anchor-width` no primeiro frame.** A largura mínima do popup é a do campo MEDIDO, e antes
//!   do primeiro `prepaint` não há medida: nesse frame o popup sai com a largura do conteúdo. No
//!   navegador a variável já existe no primeiro layout.
//! - **`not-dark:bg-clip-padding`.** É o recorte do fundo na padding box, que evita a borda
//!   translúcida clarear por cima do fundo. O GPUI não tem `background-clip`.
//! - **`Intl.Collator` de verdade.** Ver *Resolvido em número*.
//! - **`aria-activedescendant` / `role="combobox"` / `role="listbox"` e a região viva do `Empty` e do
//!   `Status`.** O GPUI não expõe árvore de acessibilidade nesta versão, então o destaque virtual
//!   existe como estado visual mas não é anunciado.
//!
//! ## Resolvido em número
//!
//! - **`max-h-[min(var(--available-height),23rem)]`** → [`POPUP_MAX_HEIGHT`] = 368px, tomado como
//!   `min` com o `menu::popup_max_height` (que é o `--available-height`).
//! - **`p-[calc(--spacing(1)-1px)]` das pastilhas** → [`CHIPS_PAD`] = 3px (4 − 1 da borda).
//! - **`rounded-[calc(var(--radius-md)-1px)]` da pastilha** → [`CHIP_RADIUS`] = 7px
//!   (`--radius-md` = `--radius` − 2 = 8).
//! - **`min-h-9 sm:min-h-8` + `has-data-[size=…]`** das pastilhas → [`InputSize::height`]
//!   (28/32/36), que é a mesma altura externa do campo de texto. E `*:min-h-6` etc. da pastilha →
//!   `altura − 8`, ver [`chip_min_height`].
//! - **`sm:size-7` / `rounded-md` do botão de afordância** → [`AFFORDANCE_SIZE`] = 28px e
//!   [`AFFORDANCE_RADIUS`] = 8px.
//! - **`sm:text-xs/(--text-xs--line-height)`** da pastilha → `menu::SMALL_TEXT_SIZE`/
//!   `menu::SMALL_LINE_HEIGHT` = 12/16. **Todo `text-*` declara o par do Tailwind**: o default do
//!   GPUI é `relative(1.618…)`, e sem o par a linha sai 2,6px mais alta (ver o teste
//!   `entrelinhas_seguem_o_par_do_tailwind`).
//! - **`ChevronsUpDownIcon`/`XIcon` (lucide)** → `iconoir/regular/arrow-separate-vertical.svg` e
//!   `iconoir/regular/xmark.svg`, os equivalentes já embutidos (o segundo é o mesmo mapeamento que o
//!   [`crate::dialog`] fez).
//! - **`[[role=group]+&]:mt-1.5`** → [`GROUP_GAP`] = 6px, aplicado ao rótulo de grupo que não é a
//!   primeira linha visível (a lista aqui é achatada, ver [`ComboboxItem`]).
//! - **`Intl.Collator` com `sensitivity: 'base'` e `ignorePunctuation: true`** → a dobra de
//!   [`fold`]/[`normalize`], por tabela própria: minúscula, sem acento na faixa latina, e sem
//!   pontuação. É uma aproximação declarada, não o Collator (ver *Desvio consciente*).
//!
//! ## Desvio consciente
//!
//! - **O bisel escuro é o DOBRO da referência** (branco a ~11,8% em vez de 6%). Não é decisão deste
//!   módulo: vem de `menu::bevel_overlay` e de `select::bevel_for`, e é o desvio já vigente em
//!   `input.rs`, `card.rs`, `select.rs`, `menu.rs` e nos outros. Manter os seis iguais é o ponto — o
//!   popup do combobox aparece lado a lado com o do menu. Travado em
//!   [`tests::bisel_escuro_e_o_dobro_da_referencia`].
//! - **O botão de afordância (gatilho/limpar) fica no FLUXO do campo, como sufixo**, e não
//!   `absolute end-0.5`. É o idioma da casa pra botão dentro de campo (o olho do campo de senha é
//!   assim, ver [`crate::input::Input::icon_button`]), e é o único jeito de reusar o `Input` inteiro
//!   em vez de remontar a moldura. Consequência medida: o centro do ícone fica a
//!   `pad_x + 28/2` = 25px da borda direita em vez de `2 + 28/2` = 16px. O desenho do botão em si
//!   (28px, raio 8, `opacity-80`→`100` no hover, sem mudança de fundo) é o da referência, e **não** o
//!   `Input::icon_button` — aquele tem 21px, raio 4 e realce de FUNDO no hover, que é outra
//!   linguagem.
//! - **`aria-invalid` na seleção única é pintado POR CIMA da borda neutra do `Input`.** O `Input` só
//!   expressa inválido junto de uma mensagem ([`crate::input::Validity::Error`]), e a linha de
//!   mensagem entraria na caixa que ancora o popup, deslocando-o. Então a borda vermelha é um
//!   overlay irmão de 1px sobre a border box — a mesma técnica do bisel. Como `--destructive/36` é
//!   translúcido, ela mistura com a borda neutra embaixo e lê um vermelho um pouco mais "sujo" que o
//!   da referência. Nas **pastilhas** não há desvio: ali a moldura é desenhada aqui e a borda
//!   inválida SUBSTITUI a neutra.
//! - **As setas PULAM os itens desabilitados.** No Base UI elas **não** pulam: o
//!   `disabledIndices: EMPTY_ARRAY` do `AriaCombobox.tsx:1186` faz a checagem de `aria-disabled` do
//!   `floating-ui-react/utils/composite.ts:501-503` ser ignorada, e só o destaque INICIAL evita o
//!   desabilitado (`useListNavigation.ts:436-439`). Ou seja: dá pra destacar um item que o
//!   `ComboboxItem.tsx:173-179` depois recusa a escolher. Aqui as setas pulam, como no
//!   [`crate::menu::Menu`] — um destaque que o `Enter` ignora é um beco sem saída, e o item
//!   desabilitado continua **visível** (só não é alcançável).
//! - **O separador some também quando não há linha visível ANTES dele**, e não só quando é o último
//!   (`last:hidden`). Filtrando, um separador entre dois grupos que ficaram vazios flutuaria sozinho
//!   no meio do popup. Superset do `last:hidden`, que continua valendo.
//! - **O filtro dobra acento por tabela própria** (Latin-1 Supplement + Latin Extended-A), não por
//!   `Intl.Collator`: o crate não pode ganhar dependência nova (ver o invariante no `Cargo.toml`) e a
//!   `std` do Rust não normaliza Unicode. Fora dessa faixa o filtro cai no `to_lowercase` simples —
//!   ou seja, "São" casa com "sao", mas um script com marcas combinantes próprias não. E não há
//!   `locale`: a colação é a mesma em qualquer idioma. Ver [`fold`].
//! - **O `Empty` e o `Status` são criados só quando têm conteúdo**, em vez de ficarem sempre montados.
//!   O Base UI exige que eles NUNCA desmontem, porque são regiões vivas (`aria-live`) e os leitores
//!   de tela precisam do elemento estável pra anunciar a mudança
//!   (`combobox/empty/ComboboxEmpty.tsx:11-21`). Como o GPUI não tem árvore de acessibilidade nesta
//!   versão, o elemento estável não anunciaria nada — e um `div` vazio permanente na lista custaria
//!   uma linha de altura (o `not-empty:p-2`/`empty:p-0` da referência existe justamente pra
//!   neutralizar isso no CSS). No dia em que o GPUI expor acessibilidade, isto volta.
//!
//! ## Superset consciente
//!
//! - **[`ComboboxEvent::Query`] e [`ComboboxEvent::OpenChange`]**: o `.tsx` não tem callbacks, mas
//!   eles existem no Base UI (`onInputValueChange`, `AriaCombobox.tsx:1579-1594`; `onOpenChange`) e
//!   sem eles um host não consegue alimentar uma lista assíncrona.
//! - **[`ComboboxItem::disabled`]**: existe no Base UI (`data-disabled` está nas classes do item do
//!   `.tsx`), mas o `.tsx` não expõe construtor. Aqui é um builder.
//!
//! ## Ausente
//!
//! - **`ComboboxRow`** (e o modo `grid`). É o layout em GRADE do Base UI: com `grid`, a `List` vira
//!   `role="grid"`, cada `Item` vira `role="gridcell"` e as setas navegam nos quatro sentidos
//!   (`combobox/row/ComboboxRow.tsx:7-26`, `ComboboxList.tsx:79`, `AriaCombobox.tsx:1184-1187`). A
//!   referência não dá estilo nenhum a ele (`className={className}`), então não há visual a portar —
//!   e a navegação 2D é um componente diferente, não uma variação deste.
//! - **`ComboboxValue`** e **`ComboboxCollection`**: reexports crus do Base UI, sem estilo (o
//!   `Collection` não renderiza elemento nenhum, só mapeia os itens filtrados —
//!   `combobox/collection/ComboboxCollection.tsx:14-23`). Aqui o valor selecionado sai de
//!   [`Combobox::selected`]/[`Combobox::selected_labels`].
//! - **`useComboboxFilter`**: o hook do Base UI. O filtro aqui é interno ([`matches_query`]); um
//!   filtro custom viria como um `Box<dyn Fn>` no dia em que alguém precisar, e não antes. Junto dele
//!   fica de fora o `limit` (que apara a lista DEPOIS de filtrar, `AriaCombobox.tsx:265-341`).
//! - **`autoHighlight`** (`false | true | 'always'`, default `false`) e **`keepHighlight`**
//!   (`AriaCombobox.tsx:1553-1565`): aqui vale sempre o default dos dois — nada é destacado
//!   automaticamente ao digitar, e o destaque apaga quando o ponteiro sai do item.
//! - **`ComboboxGroup` como elemento**: a lista é achatada (o mesmo desenho do
//!   [`crate::menu::MenuItem`]), então o grupo é expresso pelo par rótulo + separador. O único
//!   estilo que o `ComboboxGroup` tem é o `mt-1.5` entre grupos, e esse está aplicado no rótulo.
//! - **Navegação por setas ENTRE as pastilhas.** O contrato do Base UI é conhecido e completo:
//!   `Combobox.Chips` é um `role="toolbar"` com `CompositeList`, as pastilhas têm `tabIndex={-1}` e
//!   **foco de DOM real** (`ComboboxChip.tsx:103,112-122`), `←` do campo só entra nas pastilhas se o
//!   cursor do texto está na posição 0 (`ComboboxInput.tsx:180-188`), `Backspace`/`Delete` numa
//!   pastilha focada removem e reposicionam o foco por `getIndexAfterChipRemoval`
//!   (`combobox/utils/parts.ts:38-41`), e qualquer caractere imprimível devolve o foco pro campo
//!   (`ComboboxChip.tsx:72-90`). Não está implementado porque exige um [`gpui::FocusHandle`] por
//!   pastilha (foco de verdade, não um índice destacado) **e** ler a posição do cursor de dentro do
//!   `InputState` pra saber se `←` deve sair do texto. Aqui remove-se pelo ✕ de cada pastilha e pelo
//!   `Backspace` no campo vazio; o resto fica declarado em vez de simulado pela metade.
//! - **`ScrollArea` com `scrollFade`/`scrollbarGutter` na lista.** A lista rola com
//!   `overflow_y_scroll` + `ScrollHandle`, como no `select.rs` e no `menu.rs`, e **não** com a nossa
//!   [`crate::scroll_area::ScrollArea`] — que é o que a referência usa. Motivo: a `ScrollArea` é um
//!   elemento `RenderOnce` de altura `size_full`, e dentro de um popup `deferred` cuja altura é um
//!   `max_h` ela precisaria de um piso de altura pra não colapsar; o `in-data-has-overflow-y:pe-3` da
//!   referência (o respiro pro gutter) também não tem como ser observado daqui. Fica como dívida
//!   declarada, não como simulação: o comportamento de rolagem é o mesmo, o que falta é o fade nas
//!   bordas e a barra que se esconde.
//! - **`submitOnItemClick`, `inline`, `readOnly`, `openOnArrowKeyDown`, `limit`, `itemToStringLabel`,
//!   `inputValue` controlado.** Props do Base UI sem equivalente na casca do coss; entram quando
//!   houver call site.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    anchored, canvas, deferred, div, point, px, svg, App, AppContext as _, Bounds, Context,
    CursorStyle, ElementId,
    EventEmitter, FocusHandle, Focusable, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window,
};
use gpui_component::input::{Input as CoreInput, InputEvent, InputState};

use crate::input::{Input, InputSize};
use crate::menu::{
    align_center_wrap, anchor_point, bevel_overlay, edge_index, palette as popup_palette,
    popup_max_height, popup_shadow, rect_of, MenuAlign, MenuSide, BORDER, DISABLED_OPACITY,
    ICON_SIZE, ITEM_GAP, ITEM_MIN_HEIGHT, ITEM_PAD_Y, ITEM_RADIUS, LABEL_PAD_X, LABEL_PAD_Y,
    LIST_PAD, RADIUS, SEPARATOR_HEIGHT, SEPARATOR_MARGIN_X, SEPARATOR_MARGIN_Y, SIDE_OFFSET,
    SMALL_LINE_HEIGHT, SMALL_TEXT_SIZE, TEXT_LINE_HEIGHT, TEXT_SIZE, WINDOW_MARGIN,
};
use crate::select::{
    bevel_for, border_color_for, palette as field_palette, ring_overlay, ring_visible,
    shadow_stack_for, CHECK_SLOT, ITEM_PAD_END, ITEM_PAD_START,
};

// =================================================================================================
// Eventos
// =================================================================================================

/// O que o [`Combobox`] emite.
///
/// Os índices são sempre posições na lista **achatada** de [`ComboboxItem`] que você passou pro
/// [`Combobox::new`] — a mesma que [`Combobox::items`] indexa, incluindo rótulos de grupo e
/// separadores. Filtrar não renumera nada: o filtro decide o que APARECE, não o que as coisas são.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComboboxEvent {
    /// A seleção mudou. Em ordem crescente de índice; no modo único tem 0 ou 1 elemento.
    Change(Vec<usize>),
    /// O texto do campo mudou porque o usuário DIGITOU — o `onInputValueChange` do Base UI. As
    /// reescritas que o próprio componente faz (escrever o rótulo escolhido, limpar ao fechar) não
    /// emitem: senão quem assina não conseguiria distinguir uma intenção do usuário de um eco.
    Query(SharedString),
    /// O popup abriu (`true`) ou fechou (`false`) — o `onOpenChange`.
    OpenChange(bool),
}

// =================================================================================================
// Modo de seleção
// =================================================================================================

/// Quantos itens dá pra escolher.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ComboboxMode {
    /// **Um** item. Escolher escreve o rótulo no campo e fecha o popup — é o `multiple={false}` da
    /// referência (o default dela também).
    #[default]
    Single,
    /// **Vários**, mostrados como pastilhas antes do campo (o `ComboboxChips`). Escolher alterna.
    Multiple,
}

// =================================================================================================
// Linhas da lista
// =================================================================================================

/// O que uma linha da lista É. Privado: o que o call site precisa saber sai dos construtores e dos
/// predicados públicos.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RowKind {
    /// `ComboboxItem` — escolhível, com a coluna do check.
    Item,
    /// `ComboboxGroupLabel` — só texto.
    GroupLabel,
    /// `ComboboxSeparator` — só um fio.
    Separator,
}

/// Uma linha da lista do popup.
///
/// A referência compõe por JSX (`<ComboboxItem>`, `<ComboboxGroupLabel>`, `<ComboboxSeparator>`);
/// aqui a lista é **achatada** num `Vec<ComboboxItem>` e cada componente virou um **construtor** —
/// exatamente o desenho do [`crate::menu::MenuItem`], pra os dois popups desta lib se escreverem do
/// mesmo jeito.
///
/// | referência | aqui |
/// |---|---|
/// | `<ComboboxItem>` | [`ComboboxItem::new`] |
/// | `<ComboboxGroupLabel>` | [`ComboboxItem::group_label`] |
/// | `<ComboboxSeparator>` | [`ComboboxItem::separator`] |
/// | `<ComboboxGroup>` | — (o único estilo dele é o vão entre grupos, aplicado no rótulo) |
#[derive(Clone, Debug)]
pub struct ComboboxItem {
    label: SharedString,
    kind: RowKind,
    disabled: bool,
}

impl ComboboxItem {
    /// Um item **escolhível**.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            kind: RowKind::Item,
            disabled: false,
        }
    }

    /// Um **rótulo de grupo** (`text-xs`, `--muted-foreground`): não é escolhível e some quando o
    /// filtro esvazia o grupo dele.
    pub fn group_label(text: impl Into<SharedString>) -> Self {
        Self {
            label: text.into(),
            kind: RowKind::GroupLabel,
            disabled: false,
        }
    }

    /// Um **separador** (`mx-2 my-1 h-px`).
    pub fn separator() -> Self {
        Self {
            label: SharedString::default(),
            kind: RowKind::Separator,
            disabled: false,
        }
    }

    /// Desabilita o item: não é escolhível, não acende e esmaece (`data-disabled:opacity-64`).
    /// Ignorado em rótulo e separador — manter a lista homogênea vale mais que um erro de tipo.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// O rótulo (vazio num separador).
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// Se é um item escolhível (e não rótulo/separador).
    pub fn is_item(&self) -> bool {
        self.kind == RowKind::Item
    }

    /// Se está desabilitado.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Se o teclado pode parar nesta linha: item, e habilitado.
    pub fn is_activatable(&self) -> bool {
        self.kind == RowKind::Item && !self.disabled
    }
}

// =================================================================================================
// Geometria própria
// =================================================================================================
//
// Só o que NÃO existe nos módulos vizinhos. Tudo o que o popup e o item têm em comum com o
// `select.rs`/`menu.rs` vem importado de lá (ver o doc do módulo); duplicar um `4.0` aqui daria duas
// verdades pro mesmo pixel.
//
// 1 unidade Tailwind = 4px; `--radius` = 10px, `--radius-md` = 8px. O breakpoint `sm:` (≥640px) vale
// SEMPRE numa janela de desktop, então as variantes `sm:` são as efetivas.

/// Teto absoluto da altura do popup — o `23rem` do `max-h-[min(var(--available-height),23rem)]`.
///
/// O outro termo do `min` é o espaço livre em volta do campo, que é o `menu::popup_max_height`.
const POPUP_MAX_HEIGHT: f32 = 368.0;

/// Respiro do [`Combobox::empty_text`] — `not-empty:p-2`.
const EMPTY_PAD: f32 = 8.0;

/// Respiro horizontal da linha de status — `px-3`.
const STATUS_PAD_X: f32 = 12.0;

/// Respiro vertical da linha de status — `py-2`.
const STATUS_PAD_Y: f32 = 8.0;

/// Vão entre um grupo e o anterior — `[[role=group]+&]:mt-1.5`.
const GROUP_GAP: f32 = 6.0;

/// Respiro da moldura das pastilhas — `p-[calc(--spacing(1)-1px)]`: 4px menos a borda de 1px.
const CHIPS_PAD: f32 = 3.0;

/// Vão entre pastilhas — `gap-1`.
const CHIPS_GAP: f32 = 4.0;

/// Raio de uma pastilha — `rounded-[calc(var(--radius-md)-1px)]` = 8 − 1.
const CHIP_RADIUS: f32 = 7.0;

/// Respiro no início de uma pastilha (antes do rótulo) — `ps-2`. O fim não tem: quem fecha a
/// pastilha é o ✕, que traz o respiro dele.
const CHIP_PAD_START: f32 = 8.0;

/// Respiro horizontal do ✕ da pastilha — `px-1.5`.
const CHIP_REMOVE_PAD_X: f32 = 6.0;

/// Lado do ✕ da pastilha — `sm:size-3.5`. Menor que o [`ICON_SIZE`] dos ícones de item: a pastilha
/// toda é `text-xs`.
const CHIP_ICON_SIZE: f32 = 14.0;

/// Largura mínima do campo entre as pastilhas — `min-w-12`. É o que garante que sempre sobra onde
/// digitar, mesmo com a linha cheia de pastilhas.
const CHIPS_INPUT_MIN_WIDTH: f32 = 48.0;

/// Respiro no início do campo das pastilhas quando ele vem logo depois de uma pastilha —
/// `[[data-slot=combobox-chip]+&]:ps-0.5`. Sem pastilha nenhuma vale o `ps-2`/`ps-1.5`, ver
/// [`chips_input_pad_start`].
const CHIPS_INPUT_PAD_AFTER_CHIP: f32 = 2.0;

/// Lado do botão de afordância dentro do campo (gatilho ⇕ e limpar ✕) — `sm:size-7`.
const AFFORDANCE_SIZE: f32 = 28.0;

/// Raio do botão de afordância — `rounded-md` = `--radius-md` = 10 − 2.
const AFFORDANCE_RADIUS: f32 = 8.0;

/// Opacidade em repouso do botão de afordância — `opacity-80` (o hover leva a 1).
const AFFORDANCE_OPACITY: f32 = 0.8;

/// O ⇕ do gatilho — o `ChevronsUpDownIcon` do lucide. O `arrow-separate-vertical` do Iconoir é o
/// mesmo desenho (dois chevrons opostos), e já vem embutido.
const TRIGGER_ICON: &str = "iconoir/regular/arrow-separate-vertical.svg";

/// O ✕ do limpar e das pastilhas — o `XIcon` do lucide. Mesmo mapeamento que o [`crate::dialog`].
const CLOSE_ICON: &str = "iconoir/regular/xmark.svg";

/// O check da coluna 1 do item. O MESMO arquivo que o [`crate::select::Select`] e o
/// [`crate::menu::Menu`] usam — três checks de pesos diferentes em três popups do mesmo design
/// system é pior que qualquer aproximação de traço.
const CHECK_ICON: &str = "icons/check.svg";

/// Id do botão de limpar. Comparado por igualdade dentro do handler compartilhado, então é uma
/// constante e não um literal repetido.
const ID_CLEAR: &str = "combobox-clear";

/// Id do botão de gatilho.
const ID_TRIGGER: &str = "combobox-trigger";

/// Altura mínima da moldura das pastilhas — é a **mesma** do campo de texto.
///
/// Na referência são declarações separadas (`min-h-9 sm:min-h-8`, `has-data-[size=lg]:…`), e elas
/// batem número por número com o `sm:h-*` do `Input` + as duas bordas: 28/32/36. Derivar em vez de
/// copiar é o que garante que uma pastilha e um campo lado a lado não fiquem 1px diferentes.
fn chips_min_height(size: InputSize) -> f32 {
    size.height()
}

/// Altura mínima de uma pastilha — o `*:min-h-6` / `sm:has-data-[size=sm]:*:min-h-5` da referência.
///
/// É a altura da moldura menos o que sobra dela: as duas bordas de 1px e os dois [`CHIPS_PAD`]. Dá
/// 20/24/28 pros três tamanhos, que é exatamente o que a referência declara — mais uma coincidência
/// que só é coincidência se você copiar os números em vez de derivá-los.
fn chip_min_height(size: InputSize) -> f32 {
    chips_min_height(size) - 2.0 * (1.0 + CHIPS_PAD)
}

/// Respiro no início do campo que fica entre as pastilhas.
///
/// Três casos na referência, nesta ordem de especificidade: depois de uma pastilha é `ps-0.5`;
/// senão `ps-1.5` no tamanho `sm` e `ps-2` nos outros.
fn chips_input_pad_start(size: InputSize, after_chip: bool) -> f32 {
    if after_chip {
        CHIPS_INPUT_PAD_AFTER_CHIP
    } else if size == InputSize::Sm {
        CHIP_PAD_START - 2.0
    } else {
        CHIP_PAD_START
    }
}

// =================================================================================================
// O filtro (lógica pura)
// =================================================================================================

/// Dobra um caractere pra comparação: minúscula **e sem acento**.
///
/// ⚠️ **Não é o `Intl.Collator`.** O Base UI filtra com `new Intl.Collator(locale, { usage:
/// 'search', sensitivity: 'base', ignorePunctuation: true })` (`internals/filter.ts:6-21`), que sabe
/// as regras de colação de qualquer locale; a `std` do Rust não normaliza Unicode e este crate não
/// pode ganhar dependência nova (ver o invariante no `Cargo.toml`). Então a tabela cobre o que um app
/// em português/espanhol/francês encontra de verdade — Latin-1 Supplement e Latin Extended-A — e o
/// resto cai no `to_lowercase` da `std`, que já resolve maiúscula/minúscula em qualquer script.
///
/// Devolve `&'static str` (e não `char`) porque uma dobra pode CRESCER: `ß` → `ss`, `æ` → `ae`.
/// String vazia = "não está na tabela".
fn fold(c: char) -> &'static str {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'ā' | 'Ā' | 'ă'
        | 'Ă' | 'ą' | 'Ą' => "a",
        'æ' | 'Æ' => "ae",
        'ç' | 'Ç' | 'ć' | 'Ć' | 'ĉ' | 'Ĉ' | 'ċ' | 'Ċ' | 'č' | 'Č' => "c",
        'ď' | 'Ď' | 'đ' | 'Đ' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' | 'ē' | 'Ē' | 'ĕ' | 'Ĕ' | 'ė' | 'Ė' | 'ę'
        | 'Ę' | 'ě' | 'Ě' => "e",
        'ĝ' | 'Ĝ' | 'ğ' | 'Ğ' | 'ġ' | 'Ġ' | 'ģ' | 'Ģ' => "g",
        'ĥ' | 'Ĥ' | 'ħ' | 'Ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' | 'ĩ' | 'Ĩ' | 'ī' | 'Ī' | 'ĭ' | 'Ĭ' | 'į'
        | 'Į' | 'ı' | 'İ' => "i",
        'ĵ' | 'Ĵ' => "j",
        'ķ' | 'Ķ' => "k",
        'ĺ' | 'Ĺ' | 'ļ' | 'Ļ' | 'ľ' | 'Ľ' | 'ł' | 'Ł' => "l",
        'ñ' | 'Ñ' | 'ń' | 'Ń' | 'ņ' | 'Ņ' | 'ň' | 'Ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'ō' | 'Ō' | 'ŏ'
        | 'Ŏ' | 'ő' | 'Ő' => "o",
        'œ' | 'Œ' => "oe",
        'ŕ' | 'Ŕ' | 'ŗ' | 'Ŗ' | 'ř' | 'Ř' => "r",
        'ś' | 'Ś' | 'ŝ' | 'Ŝ' | 'ş' | 'Ş' | 'š' | 'Š' => "s",
        'ß' => "ss",
        'ţ' | 'Ţ' | 'ť' | 'Ť' | 'ŧ' | 'Ŧ' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' | 'ũ' | 'Ũ' | 'ū' | 'Ū' | 'ŭ' | 'Ŭ' | 'ů'
        | 'Ů' | 'ű' | 'Ű' | 'ų' | 'Ų' => "u",
        'ŵ' | 'Ŵ' => "w",
        'ý' | 'ÿ' | 'Ý' | 'Ÿ' | 'ŷ' | 'Ŷ' => "y",
        'ź' | 'Ź' | 'ż' | 'Ż' | 'ž' | 'Ž' => "z",
        _ => "",
    }
}

/// A forma de comparação de um texto: minúscula, sem acento (ver [`fold`]) e **sem pontuação** — o
/// `ignorePunctuation: true` do Collator do Base UI. Espaço é preservado (o Collator também o
/// preserva); pontuação é qualquer coisa que não seja letra, número nem espaço.
fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let dobrado = fold(c);
        if !dobrado.is_empty() {
            out.push_str(dobrado);
        } else if c.is_alphanumeric() || c.is_whitespace() {
            // Fora da tabela: minúscula da `std` (que trata os scripts que ela conhece).
            out.extend(c.to_lowercase());
        }
        // O resto (pontuação) é descartado.
    }
    out
}

/// Se um rótulo casa com o que foi digitado.
///
/// É **substring** (o `contains` do Base UI, `internals/filter.ts:25-37`), não prefixo: buscar "mult"
/// numa lista de modos tem que achar "Linear Multiply" também. Ignora caixa, acento e pontuação (ver
/// [`normalize`]). A consulta é **aparada** antes (`AriaCombobox.tsx:239` faz
/// `String(inputValue).trim()`), e consulta vazia casa com tudo (`filter.ts:25`) — é o que faz o
/// popup abrir com a lista inteira.
fn matches_query(label: &str, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    normalize(label).contains(&normalize(query))
}

/// Quais linhas o popup mostra, como máscara sobre a lista achatada.
///
/// Três regras, nesta ordem:
///
/// 1. um **item** fica se casa com o filtro ([`matches_query`]);
/// 2. um **rótulo de grupo** fica se sobrou algum item no grupo dele — ou seja entre ele e o próximo
///    rótulo. Um rótulo sozinho sobre nada é ruído;
/// 3. um **separador** fica se há linha visível antes E depois dele. O `last:hidden` da referência é
///    a metade "depois" desta regra; a metade "antes" é o superset declarado no doc do módulo.
fn visible_mask(items: &[ComboboxItem], query: &str) -> Vec<bool> {
    let mut keep: Vec<bool> = items
        .iter()
        .map(|it| it.kind == RowKind::Item && matches_query(&it.label, query))
        .collect();

    // (2) rótulos de grupo.
    for i in 0..items.len() {
        if items[i].kind != RowKind::GroupLabel {
            continue;
        }
        let mut j = i + 1;
        while j < items.len() && items[j].kind != RowKind::GroupLabel {
            if keep[j] {
                keep[i] = true;
                break;
            }
            j += 1;
        }
    }

    // (3) separadores. Avaliado depois dos rótulos, pra um separador entre dois grupos contar o
    // rótulo do grupo seguinte como "linha visível depois".
    for i in 0..items.len() {
        if items[i].kind != RowKind::Separator {
            continue;
        }
        let antes = keep[..i].iter().any(|k| *k);
        let depois = keep[i + 1..].iter().any(|k| *k);
        keep[i] = antes && depois;
    }

    keep
}

/// A máscara que a navegação por teclado consulta: visível **e** alcançável.
///
/// O item desabilitado sai daqui — ver o desvio declarado no doc do módulo (no Base UI ele
/// continuaria navegável, e o `Enter` recusaria).
fn activatable_mask(items: &[ComboboxItem], visible: &[bool]) -> Vec<bool> {
    items
        .iter()
        .zip(visible)
        .map(|(it, v)| *v && it.is_activatable())
        .collect()
}

/// Qual item nasce destacado quando o popup abre: o **SELECIONADO**.
///
/// `AriaCombobox.tsx:1180-1181` + `useListNavigation.ts:372-379`: o índice inicial é o
/// `selectedIndex`, e ele é trazido pra vista (`block: 'nearest'`). No modo **múltiplo** é o
/// **último** da seleção — o `findSelectionIndex(..., multiple)` usa
/// `selectedValue[selectedValue.length - 1]`, comprovado pelo teste
/// `ComboboxRoot.test.tsx:2523-2552`. Sem seleção, **nada** é destacado
/// (`useListNavigation.ts:417-421` + `ComboboxRoot.test.tsx:7766-7797`).
///
/// Um selecionado que o filtro escondeu (ou que está desabilitado) não pode ser destaque — daí a
/// consulta à máscara.
fn highlight_on_open(selected: &[usize], mask: &[bool]) -> Option<usize> {
    selected
        .last()
        .copied()
        .filter(|i| mask.get(*i).copied().unwrap_or(false))
}

/// O próximo índice alcançável a partir de `from`, andando `dir` e **sem circular**. `None` quando
/// passou da ponta.
fn step_no_wrap(from: usize, mask: &[bool], dir: isize) -> Option<usize> {
    let mut i = from as isize;
    loop {
        i += dir;
        if i < 0 {
            return None;
        }
        let i = usize::try_from(i).ok()?;
        if i >= mask.len() {
            return None;
        }
        if mask[i] {
            return Some(i);
        }
    }
}

/// Onde o destaque vai com `↓`/`↑`.
///
/// ⚠️ **O ciclo passa PELO CAMPO**: do último item a seta seguinte NÃO volta pro primeiro — ela
/// devolve `None`, ou seja o destaque apaga e a "vez" é do campo de texto; só a seta depois dessa
/// entra pelo primeiro item. É o `loopFocus`/`allowEscape` do Base UI (`AriaCombobox.tsx:113,1179`),
/// cujo JSDoc diz *"The input is always included in the focus loop per ARIA Authoring Practices"*, e
/// está comprovado pelo teste `ComboboxRoot.test.tsx:10718-10765`, que afirma a sequência de três
/// passos (`aria-activedescendant` presente → ausente → presente).
///
/// É por isso que este caminhador **não** é o `menu::step_index`: aquele circula direto do último pro
/// primeiro, o que é o certo num menu (que não tem campo pra onde voltar). A ponta continua sendo do
/// `menu::edge_index` — a entrada pela ponta é a mesma coisa nos dois.
fn step_highlight(from: Option<usize>, mask: &[bool], dir: isize) -> Option<usize> {
    match from {
        None => edge_index(mask, dir < 0),
        Some(i) => step_no_wrap(i, mask, dir),
    }
}

// =================================================================================================
// A seleção (lógica pura)
// =================================================================================================

/// O que escolher o item `idx` faz com a seleção.
///
/// Devolve `(nova_seleção, fecha_o_popup, mudou)`:
///
/// - **único** (`AriaCombobox.tsx:758-766`): substitui a seleção e fecha **incondicionalmente**.
///   Re-escolher o já selecionado fecha sem emitir — não há caminho de desmarcar no modo único, e um
///   evento redundante é ruído pra quem assina (a mesma regra do [`crate::select::Select`]).
/// - **múltiplo** (`AriaCombobox.tsx:731-757`): **alterna** (escolher o já marcado desmarca). E só
///   fecha se o usuário estava **filtrando**: sem filtro o popup fica aberto pra marcar vários
///   seguidos; com filtro, o texto já cumpriu o papel dele e o popup se despede. A seleção volta
///   sempre ordenada, pra o [`ComboboxEvent::Change`] não depender da ordem dos cliques.
///
/// Um `idx` fora do range é ignorado (nada muda, e no modo único ainda fecha — quem clicou clicou).
fn apply_pick(
    mode: ComboboxMode,
    filtering: bool,
    selected: &[usize],
    len: usize,
    idx: usize,
) -> (Vec<usize>, bool, bool) {
    if idx >= len {
        return (selected.to_vec(), mode == ComboboxMode::Single, false);
    }
    match mode {
        ComboboxMode::Single => {
            if selected == [idx] {
                (selected.to_vec(), true, false)
            } else {
                (vec![idx], true, true)
            }
        }
        ComboboxMode::Multiple => {
            let mut novo: Vec<usize> = selected.iter().copied().filter(|i| *i != idx).collect();
            if novo.len() == selected.len() {
                novo.push(idx);
                novo.sort_unstable();
            }
            (novo, filtering, true)
        }
    }
}

/// O texto que o campo passa a ter quando o popup fecha — o `handleUnmount` do Base UI
/// (`AriaCombobox.tsx:777-818`).
///
/// - **único**: volta pro rótulo do selecionado, ou vazio se não há seleção. É o que faz o `Escape`
///   com uma consulta pela metade devolver o campo pro valor de antes, em vez de deixar o texto
///   órfão (`ComboboxInput.test.tsx:486-491`).
/// - **múltiplo**: vazio. A seleção está nas pastilhas; o texto era só o filtro
///   (`ComboboxRoot.test.tsx:5970`).
fn text_after_close(mode: ComboboxMode, selected_label: Option<&SharedString>) -> SharedString {
    match mode {
        ComboboxMode::Single => selected_label.cloned().unwrap_or_default(),
        ComboboxMode::Multiple => SharedString::default(),
    }
}

/// O que o `Escape` faz — e ele faz **duas** coisas diferentes (`ComboboxInput.tsx:368-384`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EscapeAction {
    /// Com o popup **aberto**: fecha, descartando a consulta digitada (quem reconcilia o texto é o
    /// [`text_after_close`]). A seleção fica.
    Close,
    /// Com o popup **fechado** e havendo valor: limpa o texto **e** a seleção.
    ClearAll,
    /// Fechado e sem nada pra limpar: o `Escape` não é consumido (no Base UI ele até volta a
    /// propagar, `:378-383`).
    Nothing,
}

/// A decisão do `Escape`, em função do estado.
fn escape_action(open: bool, selected: &[usize], query: &str) -> EscapeAction {
    if open {
        EscapeAction::Close
    } else if clear_is_useful(selected, query) {
        EscapeAction::ClearAll
    } else {
        EscapeAction::Nothing
    }
}

/// Se o botão de **limpar** tem o que fazer — e, por consequência, se ele é desenhado.
///
/// No Base UI o `Clear` **desmonta** quando não há valor (`ComboboxClear.tsx:55-62`, com
/// `keepMounted` no default `false`), e o critério é por modo: texto não vazio, seleção não nula, ou
/// pastilhas. Aqui as três condições viram uma: há seleção **ou** há texto. E ele limpa as DUAS
/// coisas (`:101-126`) — um ✕ que apagasse só uma deixaria o campo num estado que ninguém pediu.
fn clear_is_useful(selected: &[usize], query: &str) -> bool {
    !selected.is_empty() || !query.is_empty()
}

/// Qual pastilha o `Backspace` num campo vazio remove: a **última**
/// (`ComboboxInput.tsx:386-409`, que exige `input.value === ''`).
fn backspace_target(selected: &[usize], query: &str) -> Option<usize> {
    if query.is_empty() {
        selected.last().copied()
    } else {
        None
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// Campo de texto que filtra uma lista, com seleção única ou múltipla.
pub struct Combobox {
    /// As linhas, na ordem dos índices.
    items: Vec<ComboboxItem>,
    /// Modo de seleção.
    mode: ComboboxMode,
    /// Índices selecionados, **ordenados**. No modo único tem 0 ou 1 elemento.
    selected: Vec<usize>,
    /// O texto do campo. Vive num `InputState` do `gpui-component` (que resolve IME, grafemas,
    /// seleção e undo); este módulo só o lê e o reescreve.
    input: gpui::Entity<InputState>,
    /// Se o texto do campo está sendo usado como FILTRO.
    ///
    /// Só depois de digitar. Abrir pelo gatilho, ou escolher no modo único (que escreve o rótulo no
    /// campo), volta pra `false` — senão o popup reabriria já filtrado pelo próprio rótulo
    /// selecionado, mostrando um item só. É o `shouldBypassFiltering` do Base UI: *"in single mode,
    /// if the query has not changed since open and equals the selected label, `filterQuery` becomes
    /// `''`"* (`AriaCombobox.tsx:243-250`).
    filtering: bool,
    /// Marca que a próxima `InputEvent::Change` é ECO de uma reescrita nossa, não digitação.
    ///
    /// Necessário porque o evento do núcleo é assíncrono (ele passa pela fila de efeitos do GPUI),
    /// então não dá pra "desfazer" o efeito dele logo depois do `set_value` — quando ele chega, o
    /// código que escreveu já terminou. Sem esta guarda, escrever o rótulo escolhido no campo
    /// religaria o filtro e reabriria o popup, e limpar o campo no modo único apagaria a seleção que
    /// acabou de ser feita. Só é armada quando o texto REALMENTE vai mudar (ver [`Self::set_text`]),
    /// pra não ficar pendurada e engolir a próxima tecla de verdade.
    echo: bool,
    /// Se o popup está aberto.
    open: bool,
    /// Índice (na lista achatada) do item **destacado** — por hover OU por teclado. É um estado só,
    /// como o `data-highlighted` do Base UI: dois itens acesos ao mesmo tempo é defeito.
    highlighted: Option<usize>,
    /// Bounds do campo (px), capturados pelo `canvas` overlay no prepaint. É o `--anchor-width` e a
    /// posição de ancoragem do popup.
    bounds: Bounds<Pixels>,
    /// Id estável desta instância (pra `div().id(..)` único na árvore).
    id: u64,
    focus_handle: FocusHandle,
    /// Rolagem da lista. Persiste entre renders: sem ela, cada `notify` (um hover) devolveria a
    /// lista pro topo no meio da rolagem.
    list_scroll: ScrollHandle,
    /// Tamanho do campo — o mesmo enum do [`crate::input::Input`], porque é o mesmo campo.
    size: InputSize,
    /// `aria-invalid`.
    invalid: bool,
    /// Desabilitado: não abre, não edita, e o conjunto esmaece.
    disabled: bool,
    /// Mostra o gatilho ⇕ no fim do campo — o `showTrigger = true` da referência.
    show_trigger: bool,
    /// Mostra o ✕ de limpar no fim do campo — o `showClear = false` da referência. Quando os dois
    /// estão ligados, o limpar VENCE (é o `has-[+[data-slot=combobox-clear]]:hidden` do gatilho).
    show_clear: bool,
    /// Texto do `ComboboxEmpty`: o que aparece quando o filtro não deixa nenhum item.
    empty_text: SharedString,
    /// Linha de status no rodapé do popup (`ComboboxStatus`) — pra "carregando…", contagem, etc.
    status: Option<SharedString>,
    /// De que lado o popup abre. `bottom` na referência.
    side: MenuSide,
    /// Como o popup se alinha ao campo. `start` na referência.
    align: MenuAlign,
    /// A assinatura do `InputState`, que mantém o filtro em sincronia com o que foi digitado.
    /// Guardada porque um `Subscription` solto se cancela ao ser descartado.
    _input_sub: Subscription,
}

impl Combobox {
    /// Cria um combobox com as linhas dadas, sem nada selecionado e fechado.
    ///
    /// Pede `window` porque o `InputState` do campo pede — é ele que resolve a edição de texto.
    pub fn new(items: Vec<ComboboxItem>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| crate::input::single_line(window, cx));
        // Digitar é o que liga o filtro e abre o popup. `subscribe_in` (e não `subscribe`) porque
        // reagir ao texto pode exigir reescrevê-lo, e escrever num `InputState` exige um `Window`.
        let sub = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, state, ev: &InputEvent, window, cx| {
                if !matches!(ev, InputEvent::Change) {
                    return;
                }
                let texto = state.read(cx).value();
                this.on_typed(texto, window, cx);
            },
        );
        Self {
            items,
            mode: ComboboxMode::default(),
            selected: Vec::new(),
            input,
            filtering: false,
            echo: false,
            open: false,
            highlighted: None,
            bounds: Bounds::default(),
            id: cx.entity_id().as_u64(),
            focus_handle: cx.focus_handle(),
            list_scroll: ScrollHandle::new(),
            size: InputSize::default(),
            invalid: false,
            disabled: false,
            show_trigger: true,
            show_clear: false,
            empty_text: SharedString::new_static("Nada encontrado"),
            status: None,
            side: MenuSide::Bottom,
            align: MenuAlign::Start,
            _input_sub: sub,
        }
    }

    /// Liga a seleção **múltipla** (pastilhas).
    pub fn multiple(mut self) -> Self {
        self.mode = ComboboxMode::Multiple;
        self
    }

    /// Define o modo de seleção direto (útil quando ele vem de um `match`).
    pub fn mode(mut self, mode: ComboboxMode) -> Self {
        self.mode = mode;
        self
    }

    /// Tamanho do campo.
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Marca o campo como **inválido** (`aria-invalid`). Só sinaliza — quem valida é quem usa.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Desabilita: não abre o popup, não edita, e o conjunto esmaece (`opacity-64`).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Esconde o gatilho ⇕ do fim do campo (o `showTrigger={false}`).
    pub fn without_trigger(mut self) -> Self {
        self.show_trigger = false;
        self
    }

    /// Mostra o ✕ de limpar (o `showClear` da referência, que vem desligado). Quando há o que
    /// limpar, ele OCUPA O LUGAR do gatilho — é o `has-[+[data-slot=combobox-clear]]:hidden`.
    pub fn with_clear(mut self) -> Self {
        self.show_clear = true;
        self
    }

    /// Texto do campo vazio (o placeholder do `InputState`).
    pub fn placeholder(
        self,
        placeholder: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        self.input.update(cx, |st, cx| {
            st.set_placeholder(placeholder, window, cx);
        });
        self
    }

    /// Texto do `ComboboxEmpty` — o que aparece quando o filtro não deixa nenhum item.
    pub fn empty_text(mut self, text: impl Into<SharedString>) -> Self {
        self.empty_text = text.into();
        self
    }

    /// Linha de **status** no rodapé do popup (`ComboboxStatus`): "carregando…", "3 de 40", etc.
    /// `None` (o default) não desenha nada.
    pub fn status(mut self, status: impl Into<SharedString>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Tira a linha de status.
    pub fn without_status(mut self) -> Self {
        self.status = None;
        self
    }

    /// De que lado o popup abre (o default é `bottom`, como na referência).
    pub fn side(mut self, side: MenuSide) -> Self {
        self.side = side;
        self
    }

    /// Como o popup se alinha ao campo (o default é `start`, como na referência).
    pub fn align(mut self, align: MenuAlign) -> Self {
        self.align = align;
        self
    }

    /// As linhas, na ordem dos índices.
    pub fn items(&self) -> &[ComboboxItem] {
        &self.items
    }

    /// Os índices selecionados, ordenados.
    pub fn selected(&self) -> &[usize] {
        &self.selected
    }

    /// Os rótulos selecionados, na ordem dos índices.
    pub fn selected_labels(&self) -> Vec<SharedString> {
        self.selected
            .iter()
            .filter_map(|i| self.items.get(*i))
            .map(|it| it.label.clone())
            .collect()
    }

    /// Se o popup está aberto.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// O item destacado (`data-highlighted`), se houver.
    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    /// O texto do campo.
    pub fn query(&self, cx: &App) -> SharedString {
        self.input.read(cx).value()
    }

    /// Define a seleção de fora. **Não** emite `Change` — evita loop de feedback com quem assina
    /// (a mesma regra do `Select::set_selected`). Índices que não são item, ou repetidos, são
    /// descartados; no modo único sobra só o primeiro.
    pub fn set_selected(&mut self, selected: &[usize], window: &mut Window, cx: &mut Context<Self>) {
        let mut novo: Vec<usize> = selected
            .iter()
            .copied()
            .filter(|i| self.items.get(*i).is_some_and(ComboboxItem::is_item))
            .collect();
        novo.sort_unstable();
        novo.dedup();
        if self.mode == ComboboxMode::Single {
            novo.truncate(1);
        }
        if novo == self.selected {
            return;
        }
        self.selected = novo;
        self.sync_single_text(window, cx);
        cx.notify();
    }

    /// Troca a lista de linhas (e descarta a seleção que não existe mais). **Não** emite.
    pub fn set_items(&mut self, items: Vec<ComboboxItem>, cx: &mut Context<Self>) {
        self.selected
            .retain(|i| items.get(*i).is_some_and(ComboboxItem::is_item));
        self.items = items;
        self.highlighted = None;
        cx.notify();
    }

    // --- Estado interno ------------------------------------------------------------------------

    /// A máscara do que está visível agora (o filtro só conta quando [`Self::filtering`]).
    fn visible(&self, cx: &App) -> Vec<bool> {
        let q = if self.filtering {
            self.query(cx)
        } else {
            SharedString::default()
        };
        visible_mask(&self.items, &q)
    }

    /// O rótulo do item selecionado (o primeiro, no modo único).
    fn first_selected_label(&self) -> Option<SharedString> {
        self.selected
            .first()
            .and_then(|i| self.items.get(*i))
            .map(|it| it.label.clone())
    }

    /// Digitou. Liga o filtro, apaga o destaque, devolve a lista pro topo e abre o popup.
    fn on_typed(&mut self, texto: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        // Eco de uma reescrita nossa: não é digitação, não mexe em nada.
        if self.echo {
            self.echo = false;
            return;
        }
        self.filtering = true;
        // `ComboboxInput.tsx:334-336`: digitar LIMPA o destaque (o `autoHighlight` está no default
        // `false`). Um destaque que sobrevive ao filtro faria o `Enter` seguinte escolher um item que
        // o usuário não viu aceso.
        self.highlighted = None;
        // `AriaCombobox.tsx:568-584`: e devolve a lista pro topo.
        self.list_scroll.set_offset(point(px(0.0), px(0.0)));

        if texto.trim().is_empty() {
            // `ComboboxInput.tsx:316-327`: esvaziar o campo LIMPA a seleção única.
            if self.mode == ComboboxMode::Single && !self.selected.is_empty() {
                self.selected.clear();
                cx.emit(ComboboxEvent::Change(Vec::new()));
            }
        } else {
            // `ComboboxInput.tsx:271-281`: só abre com texto não vazio.
            self.set_open(true, window, cx);
        }
        cx.emit(ComboboxEvent::Query(texto));
        cx.notify();
    }

    /// Abre/fecha o popup. Desabilitado não abre.
    fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled && open {
            return;
        }
        if self.open == open {
            return;
        }
        self.open = open;
        if open {
            // Abrir mostra a lista INTEIRA quando o texto é o rótulo do que já foi escolhido, e não
            // um filtro recém-digitado (o `shouldBypassFiltering`). Quem acabou de digitar chega aqui
            // já com `filtering = true` e não é afetado.
            if !self.filtering {
                let mask = activatable_mask(&self.items, &self.visible(cx));
                self.highlighted = highlight_on_open(&self.selected, &mask);
                // O selecionado nasce destacado E na vista (`useListNavigation.ts:372-379`).
                if let Some(i) = self.highlighted {
                    self.scroll_to(i, cx);
                }
            }
        } else {
            self.highlighted = None;
            // `handleUnmount`: o texto é reconciliado com a seleção.
            let texto = text_after_close(self.mode, self.first_selected_label().as_ref());
            self.set_text(texto, window, cx);
        }
        cx.emit(ComboboxEvent::OpenChange(open));
        cx.notify();
    }

    /// Traz o item `idx` pra dentro da vista.
    ///
    /// O índice do filho na lista rolável é a POSIÇÃO entre os visíveis, não o índice na lista
    /// achatada — quem o filtro escondeu não foi renderizado.
    fn scroll_to(&self, idx: usize, cx: &App) {
        let visible = self.visible(cx);
        let posicao = visible[..idx.min(visible.len())]
            .iter()
            .filter(|v| **v)
            .count();
        self.list_scroll.scroll_to_item(posicao);
    }

    /// Escolhe o item `idx` (clique ou `Enter`).
    fn pick(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.items.get(idx).is_none_or(|it| !it.is_activatable()) {
            return;
        }
        let (novo, fecha, mudou) =
            apply_pick(self.mode, self.filtering, &self.selected, self.items.len(), idx);
        self.selected = novo;
        if mudou {
            cx.emit(ComboboxEvent::Change(self.selected.clone()));
        }
        if fecha {
            // Fechar já reconcilia o texto (`text_after_close`): no único ele passa a ser o rótulo
            // escolhido, no múltiplo é limpo. Uma escrita só, num lugar só.
            self.set_open(false, window, cx);
        } else {
            // Múltiplo sem filtro: o popup fica, e o texto do campo não é tocado
            // (`AriaCombobox.tsx:731-757` retorna antes de mexer no input).
            self.highlight(Some(idx), cx);
        }
        cx.notify();
    }

    /// Escreve o rótulo do item selecionado no campo (modo único), ou o limpa se não há seleção.
    fn sync_single_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.mode != ComboboxMode::Single {
            return;
        }
        let texto = self.first_selected_label().unwrap_or_default();
        self.set_text(texto, window, cx);
    }

    /// Reescreve o texto do campo **sem** deixar isso virar filtro.
    ///
    /// Arma o [`Self::echo`] só quando o texto realmente vai mudar: o `set_value` do núcleo não emite
    /// `Change` se o valor é o mesmo, e uma guarda armada sem evento pra engolir engoliria a próxima
    /// tecla de verdade.
    fn set_text(&mut self, texto: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        self.filtering = false;
        if self.input.read(cx).value() == texto {
            return;
        }
        self.echo = true;
        self.input.update(cx, |st, cx| {
            st.set_value(texto, window, cx);
        });
    }

    /// O ✕ de limpar: apaga a seleção **e** o texto, e devolve o foco pro campo
    /// (`ComboboxClear.tsx:101-126`).
    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !clear_is_useful(&self.selected, &self.query(cx)) {
            return;
        }
        self.selected.clear();
        self.set_text(SharedString::default(), window, cx);
        self.highlighted = None;
        self.input.read(cx).focus_handle(cx).focus(window);
        cx.emit(ComboboxEvent::Change(Vec::new()));
        cx.notify();
    }

    /// Remove uma pastilha (o ✕ dela, ou o `Backspace` no campo vazio).
    fn remove_chip(&mut self, idx: usize, cx: &mut Context<Self>) {
        let antes = self.selected.len();
        self.selected.retain(|i| *i != idx);
        if self.selected.len() == antes {
            return;
        }
        cx.emit(ComboboxEvent::Change(self.selected.clone()));
        cx.notify();
    }

    /// Destaca um item, sem `notify` à toa.
    fn highlight(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.highlighted == index {
            return;
        }
        self.highlighted = index;
        cx.notify();
    }

    /// O teclado. Ouvido pela RAIZ: o evento sobe do campo focado pelos ancestrais, então um
    /// listener cobre o campo, as pastilhas e o popup.
    ///
    /// - **fechado**: `↓` abre com o primeiro item destacado, `↑` com o último; `Escape` limpa tudo
    ///   se há o que limpar. Digitar também abre (por [`Self::on_typed`]).
    /// - **aberto**: `↑`/`↓` andam pelos itens VISÍVEIS, passando pelo campo ao chegar na ponta (ver
    ///   [`step_highlight`]); `Enter` escolhe o destacado, ou fecha se não há destaque; `Escape`
    ///   fecha e descarta a consulta.
    /// - **pastilhas**: `Backspace` com o campo vazio remove a última.
    ///
    /// `Home`/`End` **não** aparecem aqui de propósito: na referência eles movem o CURSOR DO TEXTO
    /// (`ComboboxInput.tsx:352-366` chama `stopEvent` e `setSelectionRange`), e o
    /// `useListNavigation` só os trata quando o gatilho **não** é digitável. Roubá-los pro destaque
    /// tiraria do usuário o "ir pro começo do que eu digitei".
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();

        if !self.open {
            match key {
                // `Escape` com o popup FECHADO limpa texto e seleção — e só quando há o que
                // limpar (`ComboboxInput.tsx:368-384`, que só consome a tecla nesse caso).
                "escape"
                    if escape_action(false, &self.selected, &self.query(cx))
                        == EscapeAction::ClearAll =>
                {
                    crate::focus_ring::keyboard_used(window);
                    self.clear(window, cx);
                }
                "down" | "up" => {
                    crate::focus_ring::keyboard_used(window);
                    self.set_open(true, window, cx);
                    // Depois do `set_open`, que pode ter desligado o filtro (e portanto mudado a
                    // máscara) e destacado o item selecionado. Sem seleção, a seta escolhe a ponta.
                    if self.highlighted.is_none() {
                        let mask = activatable_mask(&self.items, &self.visible(cx));
                        self.highlighted = edge_index(&mask, key == "up");
                        cx.notify();
                    }
                }
                _ => {}
            }
            return;
        }

        let visible = self.visible(cx);
        let mask = activatable_mask(&self.items, &visible);
        let destino = match key {
            "escape" => {
                crate::focus_ring::keyboard_used(window);
                self.set_open(false, window, cx);
                return;
            }
            "enter" => {
                crate::focus_ring::keyboard_used(window);
                match self.highlighted {
                    Some(i) => self.pick(i, window, cx),
                    // Sem destaque, o Base UI só FECHA e deixa o evento seguir (é o que permite o
                    // Enter submeter o formulário em volta) — `ComboboxInput.tsx:436-438`.
                    None => self.set_open(false, window, cx),
                }
                return;
            }
            "backspace" => {
                // Só nas pastilhas, e só com o campo vazio — senão é o `Backspace` do texto.
                if self.mode == ComboboxMode::Multiple {
                    if let Some(alvo) = backspace_target(&self.selected, &self.query(cx)) {
                        self.remove_chip(alvo, cx);
                    }
                }
                return;
            }
            "down" => step_highlight(self.highlighted, &mask, 1),
            "up" => step_highlight(self.highlighted, &mask, -1),
            _ => return,
        };

        crate::focus_ring::keyboard_used(window);
        // `destino: None` é o passo pelo CAMPO no ciclo (ver `step_highlight`), não "não faz nada".
        if let Some(destino) = destino {
            self.scroll_to(destino, cx);
        }
        self.highlight(destino, cx);
    }

    // --- Render --------------------------------------------------------------------------------

    /// O botão de afordância dentro do campo: o gatilho ⇕ ou o ✕ de limpar.
    ///
    /// Fica no FLUXO (como sufixo do campo) e não `absolute end-0.5` — ver o desvio declarado no doc
    /// do módulo. O desenho é o da referência: quadrado de 28px, raio 8, ícone a 80% que vai a 100%
    /// no hover, **sem** mudança de fundo.
    ///
    /// ⚠️ O `.occlude()` não é enfeite: ele é o que impede o clique no botão de também acertar o
    /// `on_mouse_down` do campo (que abre o popup). Sem ele, o gatilho abriria no mouse-down e
    /// fecharia no clique, e o popup nunca abriria por ele. O [`gpui::Hitbox::is_hovered`] devolve
    /// `false` pra quem está ATRÁS de um hitbox que bloqueia o mouse — está no doc dele.
    fn affordance(
        &self,
        which: &'static str,
        icon: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = field_palette();
        div()
            .id(ElementId::NamedInteger(which.into(), self.id))
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .flex_none()
            .size(px(AFFORDANCE_SIZE))
            .rounded(px(AFFORDANCE_RADIUS))
            .text_color(p.text.scaled(AFFORDANCE_OPACITY))
            .child(
                svg()
                    .path(icon)
                    .size(px(ICON_SIZE))
                    .flex_none()
                    .text_color(p.text.scaled(AFFORDANCE_OPACITY)),
            )
            .when(!self.disabled, |d| {
                d.cursor(CursorStyle::PointingHand)
                    .on_click(cx.listener(move |this, _e, window, cx| {
                        crate::focus_ring::pointer_used(window);
                        if which == ID_CLEAR {
                            this.clear(window, cx);
                        } else {
                            // O gatilho ALTERNA (`ComboboxTrigger.tsx:121-124`, `useClick` com o
                            // `toggle` default) — diferente do clique no campo, que só abre.
                            let abrir = !this.open;
                            this.set_open(abrir, window, cx);
                        }
                    }))
            })
    }

    /// Qual afordância desenhar, se alguma. O **limpar vence o gatilho** quando há o que limpar — é
    /// o `has-[+[data-slot=combobox-clear]]:hidden` do gatilho da referência, e o desmonte do
    /// `Clear` sem valor (`ComboboxClear.tsx:55-62`).
    fn affordance_element(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if self.show_clear && clear_is_useful(&self.selected, &self.query(cx)) {
            Some(self.affordance(ID_CLEAR, CLOSE_ICON, cx).into_any_element())
        } else if self.show_trigger {
            Some(
                self.affordance(ID_TRIGGER, TRIGGER_ICON, cx)
                    .into_any_element(),
            )
        } else {
            None
        }
    }

    /// O campo de **seleção única**: o [`crate::input::Input`] inteiro + a afordância no sufixo.
    fn render_single_field(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut campo = Input::new(&self.input)
            .size(self.size)
            .disabled(self.disabled);
        if let Some(a) = self.affordance_element(cx) {
            campo = campo.suffix(a);
        }
        campo
    }

    /// Uma pastilha (`ComboboxChip` + `ComboboxChipRemove`).
    fn render_chip(
        &self,
        idx: usize,
        label: SharedString,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = popup_palette();
        div()
            .flex()
            .items_center()
            .flex_none()
            .min_h(px(chip_min_height(self.size)))
            .rounded(px(CHIP_RADIUS))
            .bg(p.accent.hsla())
            .text_color(p.accent_text.hsla())
            .text_size(px(SMALL_TEXT_SIZE))
            .line_height(px(SMALL_LINE_HEIGHT))
            .font_weight(FontWeight::MEDIUM)
            .child(div().pl(px(CHIP_PAD_START)).child(label))
            .child(
                div()
                    .id(("combobox-chip-remove", idx))
                    .occlude()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_none()
                    .h_full()
                    .px(px(CHIP_REMOVE_PAD_X))
                    .when(!self.disabled, |d| {
                        d.cursor(CursorStyle::PointingHand)
                            .on_click(cx.listener(move |this, _e, window, cx| {
                                crate::focus_ring::pointer_used(window);
                                this.remove_chip(idx, cx);
                            }))
                    })
                    .child(
                        svg()
                            .path(CLOSE_ICON)
                            .size(px(CHIP_ICON_SIZE))
                            .flex_none()
                            .text_color(p.accent_text.scaled(AFFORDANCE_OPACITY)),
                    ),
            )
    }

    /// O campo de **seleção múltipla**: a moldura `ComboboxChips` com as pastilhas e o campo nu.
    ///
    /// A moldura é a do campo de texto, desenhada aqui porque nenhum componente da lib embrulha
    /// conteúdo arbitrário nela: as CORES e a precedência de estados vêm de
    /// `select::border_color_for`/`shadow_stack_for`, o bisel de `select::bevel_for` e o anel de
    /// `select::ring_overlay` — os mesmos do gatilho do dropdown.
    fn render_chips_field(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = field_palette();
        let s = self.size;
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let tem_pastilha = !self.selected.is_empty();

        let mut moldura = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(CHIPS_GAP))
            .relative()
            .w_full()
            .min_h(px(chips_min_height(s)))
            .p(px(CHIPS_PAD))
            .rounded(px(RADIUS))
            .border_1()
            .border_color(border_color_for(self.invalid, focused).hsla())
            .bg(p.bg.hsla())
            .shadow(shadow_stack_for(self.disabled, self.invalid, focused));

        for idx in self.selected.clone() {
            let Some(label) = self.items.get(idx).map(|it| it.label.clone()) else {
                continue;
            };
            moldura = moldura.child(self.render_chip(idx, label, cx));
        }

        // O campo NU (o `Combobox.Input` dentro do `Combobox.Chips`): o núcleo do `gpui-component`
        // sem aparência nenhuma — a moldura é a de fora. É o mesmo `.appearance(false)` que o
        // `crate::input::Input` faz por dentro.
        moldura = moldura.child(
            div()
                .flex_1()
                .min_w(px(CHIPS_INPUT_MIN_WIDTH))
                .pl(px(chips_input_pad_start(s, tem_pastilha)))
                .child(
                    CoreInput::new(&self.input)
                        .appearance(false)
                        .disabled(self.disabled)
                        .bare_metrics()
                        .line_height(px(chip_min_height(s)))
                        .pl(px(0.0))
                        .pr(px(0.0))
                        .py(px(0.0))
                        .text_size(px(s.text_size())),
                ),
        );

        if let Some(a) = self.affordance_element(cx) {
            moldura = moldura.child(a);
        }

        // O bisel e o anel são IRMÃOS da moldura, não filhos: um filho que se estende pra fora dela
        // seria recortado, e o `paint_shadows` do GPUI não recorta sombra — é a armadilha que o
        // `input.rs` e o `select.rs` já pagaram (está documentada nos dois).
        let mut wrap = div().relative().w_full().child(moldura);
        if let Some(b) = bevel_for(
            self.disabled,
            focused,
            self.invalid,
            RADIUS,
            crate::group::Join::NONE,
        ) {
            wrap = wrap.child(b);
        }
        if ring_visible(self.disabled, focused) && crate::focus_ring::visible() {
            wrap = wrap.child(ring_overlay(self.invalid, RADIUS, crate::group::Join::NONE));
        }
        wrap
    }

    /// Um item do popup: a coluna do check (16px, sempre presente) + o rótulo.
    ///
    /// A coluna existe mesmo vazia — é a 1ª coluna do grid `[1rem_1fr]` da referência, e é o que
    /// mantém todos os rótulos alinhados em vez de deslocar o do item marcado.
    fn render_item(&self, idx: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let p = popup_palette();
        let item = &self.items[idx];
        let marcado = self.selected.contains(&idx);
        let disabled = item.disabled;
        // Um item desabilitado nunca acende: `data-disabled` e `data-highlighted` não convivem.
        let destacado = !disabled && self.highlighted == Some(idx);

        div()
            .id(("combobox-item", idx))
            .flex()
            .items_center()
            .gap(px(ITEM_GAP))
            .w_full()
            .min_h(px(ITEM_MIN_HEIGHT))
            .py(px(ITEM_PAD_Y))
            .pl(px(ITEM_PAD_START))
            .pr(px(ITEM_PAD_END))
            .rounded(px(ITEM_RADIUS))
            .text_size(px(TEXT_SIZE))
            .line_height(px(TEXT_LINE_HEIGHT))
            .text_color(if destacado { p.accent_text } else { p.text }.hsla())
            .when(destacado, |d| d.bg(p.accent.hsla()))
            .when(disabled, |d| d.opacity(DISABLED_OPACITY))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_none()
                    .size(px(CHECK_SLOT))
                    .when(marcado, |d| {
                        d.child(
                            svg()
                                .path(CHECK_ICON)
                                .size(px(CHECK_SLOT))
                                .flex_none()
                                .text_color(p.text.hsla()),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(item.label.clone()),
            )
            .when(!disabled, |d| {
                d.cursor(CursorStyle::PointingHand)
                    // Hover e teclado são o MESMO estado (o `data-highlighted`), e o destaque apaga
                    // quando o ponteiro sai — é o `keepHighlight` no default `false`
                    // (`AriaCombobox.tsx:1561-1565` → `resetOnPointerLeave: !keepHighlight`).
                    .on_hover(cx.listener(move |this, dentro: &bool, _w, cx| {
                        if *dentro {
                            this.highlight(Some(idx), cx);
                        } else if this.highlighted == Some(idx) {
                            this.highlight(None, cx);
                        }
                    }))
                    .on_click(cx.listener(move |this, _e, window, cx| {
                        crate::focus_ring::pointer_used(window);
                        this.pick(idx, window, cx);
                    }))
            })
    }

    /// O popup: superfície do [`crate::menu::Menu`] + lista rolável + status.
    fn render_popup(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = popup_palette();
        let anchor = rect_of(self.bounds);
        let viewport = window.viewport_size();
        // As duas metades do `max-h-[min(var(--available-height),23rem)]`.
        let max_h = popup_max_height(self.side, anchor, f32::from(viewport.height), SIDE_OFFSET)
            .min(POPUP_MAX_HEIGHT);
        // O `max-w-(--available-width)`. O popup abre num lado VERTICAL por default, onde ele desliza
        // livre na horizontal — o disponível é a janela menos as duas margens. O piso é a largura do
        // próprio campo: um popup mais estreito que a âncora seria pior que um que estoura.
        let max_w = (f32::from(viewport.width) - 2.0 * WINDOW_MARGIN).max(anchor.w);
        let (x, y, corner) = anchor_point(anchor, self.side, self.align, SIDE_OFFSET, 0.0);

        let visible = self.visible(cx);
        let tem_item = self
            .items
            .iter()
            .zip(&visible)
            .any(|(it, v)| *v && it.is_item());

        // As linhas visíveis. Um rótulo de grupo que não é a primeira linha ganha o vão entre grupos.
        let mut primeira = true;
        let mut linhas: Vec<gpui::AnyElement> = Vec::new();
        for (idx, mostra) in visible.iter().enumerate() {
            if !mostra {
                continue;
            }
            let linha: gpui::AnyElement = match self.items[idx].kind {
                RowKind::Item => self.render_item(idx, cx).into_any_element(),
                RowKind::GroupLabel => div()
                    .px(px(LABEL_PAD_X))
                    .py(px(LABEL_PAD_Y))
                    .when(!primeira, |d| d.mt(px(GROUP_GAP)))
                    .text_size(px(SMALL_TEXT_SIZE))
                    .line_height(px(SMALL_LINE_HEIGHT))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted.hsla())
                    .child(self.items[idx].label.clone())
                    .into_any_element(),
                RowKind::Separator => div()
                    .mx(px(SEPARATOR_MARGIN_X))
                    .my(px(SEPARATOR_MARGIN_Y))
                    .h(px(SEPARATOR_HEIGHT))
                    .bg(p.border.hsla())
                    .into_any_element(),
            };
            linhas.push(linha);
            primeira = false;
        }

        // `ComboboxEmpty`: quando a lista filtrada não tem item nenhum — com ou sem consulta
        // (`ComboboxEmpty.tsx:33`).
        if !tem_item {
            linhas.push(
                div()
                    .w_full()
                    .p(px(EMPTY_PAD))
                    .text_size(px(TEXT_SIZE))
                    .line_height(px(TEXT_LINE_HEIGHT))
                    .text_color(p.muted.hsla())
                    .text_center()
                    .child(self.empty_text.clone())
                    .into_any_element(),
            );
        }

        let lista = div()
            .id(("combobox-list", self.id))
            .flex()
            .flex_col()
            .w_full()
            .p(px(LIST_PAD))
            .max_h(px(max_h))
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            .children(linhas);

        // A superfície. ⚠️ NÃO tem `overflow_hidden`: recortaria o bisel, que é um filho absoluto
        // sobre a borda. Quem rola é a lista.
        let mut surface = div()
            .relative()
            .occlude()
            .flex()
            .flex_col()
            .flex_none()
            .max_w(px(max_w))
            .bg(p.popover_bg.hsla())
            .border(px(BORDER))
            .border_color(p.border.hsla())
            .rounded(px(RADIUS))
            .shadow(popup_shadow())
            .child(lista);
        // `min-w-(--anchor-width)`: a largura do campo medido. No primeiro frame ela é zero e o
        // popup sai com a largura do conteúdo (ver *Não reproduzível* no doc do módulo).
        if anchor.w > 0.0 {
            surface = surface.min_w(px(anchor.w));
        }
        if let Some(status) = self.status.clone() {
            surface = surface.child(
                div()
                    .flex_none()
                    .px(px(STATUS_PAD_X))
                    .py(px(STATUS_PAD_Y))
                    .text_size(px(SMALL_TEXT_SIZE))
                    .line_height(px(SMALL_LINE_HEIGHT))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted.hsla())
                    .child(status),
            );
        }
        let surface = surface.child(bevel_overlay()).on_mouse_down_out(cx.listener(
            |this, e: &MouseDownEvent, window, cx| {
                // Um clique no próprio campo é "fora do popup". Fechar aqui (fase de CAPTURA) e
                // deixar o gatilho alternar na de BOLHA reabriria o popup — então o clique no campo
                // é do campo. Mesma armadilha documentada no `select.rs` e no `menu.rs`.
                if this.bounds.contains(&e.position) {
                    return;
                }
                this.set_open(false, window, cx);
            },
        ));

        let child: gpui::AnyElement = if self.align == MenuAlign::Center {
            align_center_wrap(self.side, anchor, surface.into_any_element()).into_any_element()
        } else {
            surface.into_any_element()
        };

        deferred(
            anchored()
                .position(point(px(x), px(y)))
                .anchor(corner)
                .snap_to_window_with_margin(px(WINDOW_MARGIN))
                .child(child),
        )
        .with_priority(1)
    }
}

impl EventEmitter<ComboboxEvent> for Combobox {}

impl Focusable for Combobox {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Combobox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.open;
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);

        let campo: gpui::AnyElement = match self.mode {
            ComboboxMode::Single => self.render_single_field(cx).into_any_element(),
            ComboboxMode::Multiple => self.render_chips_field(window, cx).into_any_element(),
        };

        // O wrap mede a âncora do popup (`--anchor-width` e a posição): a caixa dele coincide com a
        // do campo, porque o campo é o único filho em fluxo. Mesmo truque de `canvas` overlay do
        // `select.rs`.
        let mut wrap = div()
            .id(("combobox-field", self.id))
            .relative()
            .w_full()
            .child(campo)
            .child(
                canvas(
                    {
                        let view = cx.entity();
                        move |bounds, _window, cx| {
                            view.update(cx, |this, _| this.bounds = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            // `openOnInputClick` no default `true`, com `toggle: false`: pressionar o campo ABRE, e
            // pressionar de novo não fecha (`AriaCombobox.tsx:1139-1147`). O botão de afordância está
            // fora disto porque ele `occlude()` — ver [`Self::affordance`].
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _e: &MouseDownEvent, window, cx| {
                    crate::focus_ring::pointer_used(window);
                    this.set_open(true, window, cx);
                }),
            );

        // `aria-invalid` na seleção única: overlay de 1px sobre a border box do campo — ver o desvio
        // declarado no doc do módulo. Nas pastilhas a borda inválida já é a da moldura.
        if self.invalid && self.mode == ComboboxMode::Single {
            wrap = wrap.child(
                div()
                    .absolute()
                    .inset(px(0.0))
                    .rounded(px(RADIUS))
                    .border_1()
                    .border_color(border_color_for(true, focused).hsla()),
            );
        }

        div()
            .id(("combobox", self.id))
            .relative()
            .w_full()
            // `has-disabled:opacity-64`: o coss esmaece o conjunto em vez de trocar cor por cor.
            .when(self.disabled, |d| d.opacity(DISABLED_OPACITY))
            .on_key_down(cx.listener(Self::on_key))
            .child(wrap)
            .when(open, |d| d.child(self.render_popup(window, cx)))
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `select.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use gpui::{Modifiers, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Uma lista com os três tipos de linha, dois grupos e um item desabilitado — a forma que os
    /// testes de filtro e de navegação usam.
    fn lista() -> Vec<ComboboxItem> {
        vec![
            ComboboxItem::group_label("Claros"),          // 0
            ComboboxItem::new("Normal"),                  // 1
            ComboboxItem::new("Screen"),                  // 2
            ComboboxItem::separator(),                    // 3
            ComboboxItem::group_label("Escuros"),         // 4
            ComboboxItem::new("Multiply"),                // 5
            ComboboxItem::new("Overlay").disabled(true),  // 6
        ]
    }

    // --- Filtro ---------------------------------------------------------------------------------

    /// O filtro é **substring**, ignora caixa, acento e pontuação. Prefixo puro não serviria: buscar
    /// "mult" numa lista de modos tem que achar "Linear Multiply".
    #[test]
    fn filtro_casa_substring_sem_caixa_sem_acento_e_sem_pontuacao() {
        assert!(matches_query("Linear Multiply", "mult"), "substring, não prefixo");
        assert!(matches_query("Multiply", "MULT"), "ignora caixa");
        assert!(matches_query("São Paulo", "sao"), "ignora acento no rótulo");
        assert!(matches_query("Sao Paulo", "são"), "ignora acento no filtro");
        assert!(matches_query("Ação", "AÇÃ"), "as duas coisas juntas");
        // `ignorePunctuation: true` do Collator: o hífen não conta.
        assert!(matches_query("e-mail", "email"), "ignora pontuação no rótulo");
        assert!(matches_query("email", "e-mail"), "ignora pontuação no filtro");
        assert!(!matches_query("Normal", "xyz"));
        // Consulta vazia (ou só espaço, porque ela é APARADA) casa com tudo.
        assert!(matches_query("Normal", ""));
        assert!(matches_query("Normal", "   "));
        // O espaço INTERNO não é pontuação: ele conta.
        assert!(!matches_query("Multiply", "mult iply"));
    }

    /// A dobra de acento cobre a faixa latina e pode CRESCER (`ß` → `ss`). Fora da tabela, o
    /// `to_lowercase` da `std` ainda resolve a caixa; a pontuação cai.
    #[test]
    fn dobra_normaliza_a_faixa_latina_e_derruba_pontuacao() {
        assert_eq!(normalize("Ãé Ç"), "ae c");
        assert_eq!(normalize("Straße"), "strasse", "ß cresce pra ss");
        assert_eq!(normalize("ŒUF"), "oeuf");
        assert_eq!(normalize("a-b.c!"), "abc", "pontuação some");
        assert_eq!(normalize("a b"), "a b", "espaço FICA");
        // Fora da tabela: minúscula, sem dobra inventada.
        assert_eq!(normalize("ПРИВЕТ"), "привет");
        // A dobra é idempotente: normalizar duas vezes dá o mesmo.
        assert_eq!(normalize(&normalize("Ação")), normalize("Ação"));
    }

    /// **Rótulo de grupo e separador acompanham o filtro.** Um rótulo sobre nada e um fio entre dois
    /// grupos vazios são o ruído que faz um popup filtrado parecer quebrado.
    #[test]
    fn filtro_esconde_rotulo_orfao_e_separador_solto() {
        let items = lista();

        // Sem filtro: tudo aparece.
        assert_eq!(visible_mask(&items, ""), vec![true; 7]);

        // "mult" só existe no 2º grupo: o rótulo do 1º e o separador somem com ele.
        assert_eq!(
            visible_mask(&items, "mult"),
            vec![false, false, false, false, true, true, false],
            "sobra o rótulo do grupo que tem o item, e mais nada"
        );

        // "e" está nos dois grupos: aí o separador tem linha visível dos dois lados e fica.
        let m = visible_mask(&items, "e");
        assert!(m[3], "separador entre dois grupos POVOADOS fica");
        assert!(m[0] && m[4], "os dois rótulos ficam");

        // Nada casa: nem rótulo, nem separador, nem item.
        assert_eq!(visible_mask(&items, "zzz"), vec![false; 7]);
    }

    /// O `last:hidden` da referência, e a metade a mais que declaramos: um separador no FIM ou no
    /// COMEÇO da lista visível não aparece.
    #[test]
    fn separador_no_fim_e_no_comeco_nao_aparece() {
        // Separador como última linha: `last:hidden`.
        let fim = vec![ComboboxItem::new("Normal"), ComboboxItem::separator()];
        assert_eq!(visible_mask(&fim, ""), vec![true, false]);

        // Separador como primeira: o superset declarado (a referência não cobre este caso).
        let comeco = vec![ComboboxItem::separator(), ComboboxItem::new("Normal")];
        assert_eq!(visible_mask(&comeco, ""), vec![false, true]);

        // Dois separadores seguidos no meio: os dois têm item dos dois lados, então ficam —
        // esconder o duplicado é problema de quem montou a lista, não do filtro.
        let meio = vec![
            ComboboxItem::new("A"),
            ComboboxItem::separator(),
            ComboboxItem::separator(),
            ComboboxItem::new("B"),
        ];
        assert_eq!(visible_mask(&meio, ""), vec![true, true, true, true]);
    }

    /// A máscara de navegação: só item VISÍVEL e HABILITADO. O desabilitado aparece na lista mas o
    /// teclado passa por cima dele (desvio declarado em relação ao Base UI, que o mantém navegável).
    #[test]
    fn mascara_de_navegacao_pula_rotulo_separador_e_desabilitado() {
        let items = lista();
        let vis = visible_mask(&items, "");
        assert_eq!(
            activatable_mask(&items, &vis),
            vec![false, true, true, false, false, true, false],
            "1, 2 e 5 são os alcançáveis; 6 está desabilitado"
        );

        // Com filtro, o que não está visível também sai da máscara.
        let vis = visible_mask(&items, "mult");
        assert_eq!(
            activatable_mask(&items, &vis),
            vec![false, false, false, false, false, true, false]
        );
    }

    // --- Destaque -------------------------------------------------------------------------------

    /// **Ao abrir, quem nasce destacado é o item SELECIONADO** — não o primeiro, não nenhum. E no
    /// modo múltiplo é o ÚLTIMO da seleção.
    #[test]
    fn destaque_ao_abrir_e_o_item_selecionado() {
        let items = lista();
        let mask = activatable_mask(&items, &visible_mask(&items, ""));

        assert_eq!(highlight_on_open(&[], &mask), None, "sem seleção, nada acende");
        assert_eq!(highlight_on_open(&[5], &mask), Some(5), "o selecionado");
        assert_eq!(
            highlight_on_open(&[1, 5], &mask),
            Some(5),
            "múltiplo: o ÚLTIMO da seleção"
        );
        assert_eq!(
            highlight_on_open(&[2, 5], &mask),
            Some(5),
            "e é o último por ÍNDICE, que é como a seleção é guardada"
        );

        // Um selecionado que o filtro escondeu não pode acender.
        let mask_filtrada = activatable_mask(&items, &visible_mask(&items, "mult"));
        assert_eq!(highlight_on_open(&[1], &mask_filtrada), None);
        // Nem um desabilitado.
        assert_eq!(highlight_on_open(&[6], &mask), None);
    }

    /// **As setas circulam PASSANDO PELO CAMPO.** Do último item, `↓` não volta pro primeiro: apaga
    /// o destaque (a vez é do campo de texto), e só a seta seguinte entra pelo primeiro. É o
    /// `loopFocus`/`allowEscape` do Base UI, e é o que diferencia este caminhador do
    /// `menu::step_index` — que circula direto, porque um menu não tem campo pra onde voltar.
    #[test]
    fn setas_circulam_passando_pelo_campo() {
        let items = lista();
        let mask = activatable_mask(&items, &visible_mask(&items, ""));

        // Sem destaque: `↓` entra pelo primeiro, `↑` pelo último (pulando rótulo e desabilitado).
        assert_eq!(step_highlight(None, &mask, 1), Some(1));
        assert_eq!(step_highlight(None, &mask, -1), Some(5));

        // Andando pra frente: 1 → 2 → 5 (pula separador e rótulo) → NENHUM → 1.
        assert_eq!(step_highlight(Some(1), &mask, 1), Some(2));
        assert_eq!(step_highlight(Some(2), &mask, 1), Some(5));
        assert_eq!(
            step_highlight(Some(5), &mask, 1),
            None,
            "do último a seta apaga o destaque — o ciclo passa pelo CAMPO"
        );
        assert_eq!(step_highlight(None, &mask, 1), Some(1), "e só então volta pro 1º");

        // Pra trás, simétrico.
        assert_eq!(step_highlight(Some(2), &mask, -1), Some(1));
        assert_eq!(step_highlight(Some(1), &mask, -1), None, "passa pelo campo");
        assert_eq!(step_highlight(None, &mask, -1), Some(5));

        // Lista sem nenhum item alcançável: não há pra onde ir.
        assert_eq!(step_highlight(None, &[false, false], 1), None);
    }

    // --- Seleção --------------------------------------------------------------------------------

    /// **Seleção única**: substitui, fecha, e re-escolher o mesmo não emite (evento redundante é
    /// ruído pra quem assina — a mesma regra do `Select::choose`).
    #[test]
    fn selecao_unica_substitui_fecha_e_nao_reemite() {
        let m = ComboboxMode::Single;

        assert_eq!(apply_pick(m, false, &[], 7, 5), (vec![5], true, true));
        assert_eq!(apply_pick(m, false, &[1], 7, 5), (vec![5], true, true), "substitui");
        assert_eq!(
            apply_pick(m, false, &[5], 7, 5),
            (vec![5], true, false),
            "re-escolher fecha, sem emitir"
        );
        // Índice fora do range: ignorado, mas o popup fecha (quem clicou clicou).
        assert_eq!(apply_pick(m, false, &[1], 7, 99), (vec![1], true, false));
        // E o único FECHA independentemente de estar filtrando.
        assert!(apply_pick(m, true, &[], 7, 5).1);
    }

    /// **Seleção múltipla**: alterna, ordena, e só fecha se o usuário estava FILTRANDO.
    #[test]
    fn selecao_multipla_alterna_e_so_fecha_filtrando() {
        let m = ComboboxMode::Multiple;

        assert_eq!(apply_pick(m, false, &[], 7, 5), (vec![5], false, true));
        // Marcar um segundo, em ordem inversa de clique: sai ordenado.
        assert_eq!(apply_pick(m, false, &[5], 7, 1), (vec![1, 5], false, true));
        // Re-clicar DESMARCA (é o que "alterna" quer dizer) — o oposto do modo único.
        assert_eq!(apply_pick(m, false, &[1, 5], 7, 5), (vec![1], false, true));
        assert_eq!(apply_pick(m, false, &[5], 7, 5), (vec![], false, true));

        // Filtrando, o popup FECHA depois de escolher: o texto já cumpriu o papel dele.
        assert!(
            apply_pick(m, true, &[], 7, 5).1,
            "múltiplo COM filtro fecha"
        );
        assert!(
            !apply_pick(m, false, &[], 7, 5).1,
            "múltiplo SEM filtro fica aberto"
        );

        // Fora do range: nada muda e continua aberto.
        assert_eq!(apply_pick(m, false, &[1], 7, 99), (vec![1], false, false));
    }

    /// Os dois modos discordam em TUDO o que importa: o único fecha e substitui, o múltiplo fica
    /// aberto e acumula. Este teste existe pra a diferença não virar um `if` esquecido.
    #[test]
    fn os_dois_modos_discordam_no_fechar_e_no_acumular() {
        let (subst, fecha_unico, _) = apply_pick(ComboboxMode::Single, false, &[1], 7, 5);
        let (acc, fecha_multi, _) = apply_pick(ComboboxMode::Multiple, false, &[1], 7, 5);
        assert!(fecha_unico, "único FECHA");
        assert!(!fecha_multi, "múltiplo fica ABERTO");
        assert_eq!(subst.len(), 1, "único SUBSTITUI");
        assert_eq!(acc.len(), 2, "múltiplo ACUMULA");
    }

    /// **O `Escape` faz duas coisas diferentes**, e qual delas depende de o popup estar aberto.
    /// Trocá-las é o defeito que apaga a seleção do usuário quando ele só queria fechar a lista.
    #[test]
    fn escape_fecha_aberto_e_limpa_fechado() {
        // Aberto: fecha, e só. A seleção fica (quem reconcilia o texto é o `text_after_close`).
        assert_eq!(escape_action(true, &[], ""), EscapeAction::Close);
        assert_eq!(escape_action(true, &[5], "mult"), EscapeAction::Close);

        // Fechado, com valor: limpa texto E seleção.
        assert_eq!(escape_action(false, &[5], ""), EscapeAction::ClearAll);
        assert_eq!(escape_action(false, &[], "mult"), EscapeAction::ClearAll);

        // Fechado e vazio: o Escape não é nosso.
        assert_eq!(escape_action(false, &[], ""), EscapeAction::Nothing);
    }

    /// **Ao fechar, o texto é reconciliado com a seleção** — no único ele volta pro rótulo escolhido
    /// (é o que devolve o campo ao valor de antes quando o `Escape` descarta uma consulta pela
    /// metade), no múltiplo ele é limpo (a seleção está nas pastilhas).
    #[test]
    fn texto_apos_fechar_reverte_no_unico_e_limpa_no_multiplo() {
        let rotulo = SharedString::new_static("Multiply");

        assert_eq!(
            text_after_close(ComboboxMode::Single, Some(&rotulo)),
            rotulo,
            "único com seleção: volta pro rótulo"
        );
        assert_eq!(
            text_after_close(ComboboxMode::Single, None),
            SharedString::default(),
            "único sem seleção: esvazia"
        );
        assert_eq!(
            text_after_close(ComboboxMode::Multiple, Some(&rotulo)),
            SharedString::default(),
            "múltiplo: SEMPRE esvazia, mesmo com seleção"
        );
    }

    /// O ✕ de limpar só tem o que fazer se há seleção **ou** texto — e é este mesmo predicado que
    /// decide se ele aparece no lugar do gatilho (no Base UI o `Clear` desmonta sem valor).
    #[test]
    fn limpar_so_serve_com_selecao_ou_texto() {
        assert!(!clear_is_useful(&[], ""));
        assert!(clear_is_useful(&[1], ""), "só seleção");
        assert!(clear_is_useful(&[], "mult"), "só texto");
        assert!(clear_is_useful(&[1], "mult"));
    }

    /// `Backspace` remove a ÚLTIMA pastilha — e só com o campo vazio, senão ele é o backspace do
    /// texto.
    #[test]
    fn backspace_remove_a_ultima_pastilha_so_com_campo_vazio() {
        assert_eq!(backspace_target(&[1, 5], ""), Some(5), "a última");
        assert_eq!(backspace_target(&[1, 5], "m"), None, "com texto, é do texto");
        assert_eq!(backspace_target(&[], ""), None, "sem pastilha, nada");
    }

    // --- Geometria ------------------------------------------------------------------------------

    /// **A geometria do item é a do `select.rs`/`menu.rs`, não uma cópia.** Se alguém trocar um
    /// número lá, este teste é o que denuncia que os dois popups do design system saíram do lugar
    /// juntos — ou que alguém reintroduziu uma constante local aqui.
    #[test]
    fn geometria_do_item_vem_da_lib_e_tem_a_coluna_do_check() {
        assert_eq!(CHECK_SLOT, 16.0, "grid-cols-[1rem_1fr]: a 1ª coluna");
        assert_eq!(ITEM_PAD_START, 8.0, "ps-2");
        assert_eq!(ITEM_PAD_END, 16.0, "pe-4");
        assert!(
            ITEM_PAD_END > ITEM_PAD_START,
            "pe-4 > ps-2: sobra espaço à direita do rótulo"
        );
        assert_eq!(ITEM_GAP, 8.0, "gap-2");
        assert_eq!(ITEM_MIN_HEIGHT, 28.0, "sm:min-h-7");
        assert_eq!(ITEM_PAD_Y, 4.0, "py-1");
        assert_eq!(ITEM_RADIUS, 6.0, "rounded-sm");
        assert_eq!(LIST_PAD, 4.0, "p-1 da lista");
        assert_eq!(RADIUS, 10.0, "rounded-lg do popup");
        assert_eq!(BORDER, 1.0, "border do popup");
        assert!(ITEM_RADIUS < RADIUS, "o item arredonda MENOS que o popup");
        // A largura ocupada pelo par check+vão, que é o recuo do rótulo a partir da borda do item.
        assert_eq!(ITEM_PAD_START + CHECK_SLOT + ITEM_GAP, 32.0);
    }

    /// **O teto de altura do popup são as DUAS metades do `min()`.** Só o espaço livre deixaria um
    /// popup de 900px numa janela alta; só o 23rem ignoraria uma janela baixa.
    #[test]
    fn altura_do_popup_e_o_menor_entre_o_livre_e_23rem() {
        assert_eq!(POPUP_MAX_HEIGHT, 368.0, "23rem");

        let anchor = crate::menu::Rect {
            x: 0.0,
            y: 40.0,
            w: 200.0,
            h: 32.0,
        };
        // Janela alta: o 23rem é que vale.
        let livre = popup_max_height(MenuSide::Bottom, anchor, 2000.0, SIDE_OFFSET);
        assert!(livre > POPUP_MAX_HEIGHT, "numa janela alta sobra mais que 23rem");
        assert_eq!(livre.min(POPUP_MAX_HEIGHT), POPUP_MAX_HEIGHT);

        // Janela baixa: o espaço livre é que vale.
        let livre = popup_max_height(MenuSide::Bottom, anchor, 300.0, SIDE_OFFSET);
        assert!(livre < POPUP_MAX_HEIGHT);
        assert_eq!(livre.min(POPUP_MAX_HEIGHT), livre);
    }

    /// **A moldura das pastilhas tem a MESMA altura do campo de texto**, e a pastilha é a moldura
    /// menos o respiro e as bordas. Os números (28/32/36 e 20/24/28) são os da referência, mas saem
    /// DERIVADOS — copiá-los deixaria os dois campos 1px diferentes no dia em que um mudasse.
    #[test]
    fn alturas_das_pastilhas_derivam_do_campo() {
        for (s, moldura, pastilha) in [
            (InputSize::Sm, 28.0, 20.0),
            (InputSize::Md, 32.0, 24.0),
            (InputSize::Lg, 36.0, 28.0),
        ] {
            assert_eq!(chips_min_height(s), moldura, "min-h da moldura em {s:?}");
            assert_eq!(chip_min_height(s), pastilha, "min-h da pastilha em {s:?}");
            assert_eq!(
                chips_min_height(s),
                s.height(),
                "a moldura é a MESMA altura do campo de texto"
            );
            // A conta: as duas bordas de 1px e os dois respiros de 3px.
            assert_eq!(moldura - pastilha, 8.0);
        }
        assert_eq!(CHIPS_PAD, 3.0, "p-[calc(--spacing(1)-1px)]");
        assert_eq!(CHIPS_GAP, 4.0, "gap-1");
        assert_eq!(CHIP_RADIUS, 7.0, "rounded-[calc(--radius-md - 1px)]");
        assert!(CHIP_RADIUS < RADIUS, "a pastilha arredonda menos que a moldura");
    }

    /// O respiro do campo entre as pastilhas tem três casos, e o "depois de uma pastilha" é o mais
    /// específico — sem ele o cursor nasce longe da última pastilha.
    #[test]
    fn respiro_do_campo_das_pastilhas_por_caso() {
        assert_eq!(
            chips_input_pad_start(InputSize::Md, true),
            CHIPS_INPUT_PAD_AFTER_CHIP,
            "depois de uma pastilha: ps-0.5"
        );
        assert_eq!(chips_input_pad_start(InputSize::Md, false), 8.0, "ps-2");
        assert_eq!(chips_input_pad_start(InputSize::Sm, false), 6.0, "ps-1.5 no sm");
        // O caso "depois de pastilha" vence o tamanho, nos dois tamanhos.
        assert_eq!(
            chips_input_pad_start(InputSize::Sm, true),
            chips_input_pad_start(InputSize::Lg, true)
        );
        assert_eq!(CHIPS_INPUT_MIN_WIDTH, 48.0, "min-w-12: sempre sobra onde digitar");
    }

    /// O botão de afordância: 28px, raio 8, ícone de 16 a 80%. O raio TEM que ser menor que o do
    /// campo — um botão dentro de um campo com o mesmo raio parece transbordar a quina.
    #[test]
    fn botao_de_afordancia_tem_as_medidas_da_referencia() {
        assert_eq!(AFFORDANCE_SIZE, 28.0, "sm:size-7");
        assert_eq!(AFFORDANCE_RADIUS, 8.0, "rounded-md = --radius-md");
        assert!(AFFORDANCE_RADIUS < RADIUS, "rounded-md < rounded-lg");
        assert_eq!(ICON_SIZE, 16.0, "sm:size-4");
        assert_eq!(AFFORDANCE_OPACITY, 0.8, "opacity-80");
        assert!(
            AFFORDANCE_SIZE <= chips_min_height(InputSize::Md),
            "o botão cabe no campo do tamanho padrão"
        );
        // Os ícones existem de verdade — um path errado desaparece SILENCIOSAMENTE na tela.
        assert!(crate::iconoir::has(TRIGGER_ICON), "o ⇕ do gatilho");
        assert!(crate::iconoir::has(CLOSE_ICON), "o ✕");
        assert!(
            crate::assets::ICONS.iter().any(|(p, _)| *p == CHECK_ICON),
            "o check da coluna 1"
        );
    }

    /// A pastilha e as linhas pequenas são `text-xs`; o item e o vazio são `text-sm`.
    #[test]
    fn corpos_de_texto_por_papel() {
        assert_eq!(TEXT_SIZE, 14.0, "text-sm do item e do vazio");
        assert_eq!(SMALL_TEXT_SIZE, 12.0, "text-xs do rótulo, do status e da pastilha");
        assert!(SMALL_TEXT_SIZE < TEXT_SIZE);
        assert_eq!(CHIP_ICON_SIZE, 14.0, "sm:size-3.5 do ✕ da pastilha");
        assert!(
            CHIP_ICON_SIZE < ICON_SIZE,
            "o ✕ da pastilha é menor que os ícones de item — a pastilha toda é text-xs"
        );
        assert_eq!(EMPTY_PAD, 8.0, "not-empty:p-2");
        assert_eq!((STATUS_PAD_X, STATUS_PAD_Y), (12.0, 8.0), "px-3 py-2");
        assert_eq!(GROUP_GAP, 6.0, "[[role=group]+&]:mt-1.5");
        assert_eq!(CHIP_PAD_START, 8.0, "ps-2 da pastilha");
        assert_eq!(CHIP_REMOVE_PAD_X, 6.0, "px-1.5 do ✕");
        assert_eq!(SEPARATOR_MARGIN_X, 8.0, "mx-2");
        assert_eq!(SEPARATOR_MARGIN_Y, 4.0, "my-1");
        assert_eq!(SEPARATOR_HEIGHT, 1.0, "h-px");
        assert_eq!((LABEL_PAD_X, LABEL_PAD_Y), (8.0, 6.0), "px-2 py-1.5 do rótulo");
        assert_eq!(DISABLED_OPACITY, 0.64, "data-disabled:opacity-64");
    }

    /// **A armadilha da entrelinha.** O default do GPUI é `relative(1.618…)`: um `text-sm` sem par
    /// declarado sai com 22,6px de linha em vez de 20, e um item de `min-h-7` (28) estoura pra 30,6.
    /// Todo `text-*` deste módulo declara o par do Tailwind, e é isto que trava.
    #[test]
    fn entrelinhas_seguem_o_par_do_tailwind() {
        assert_eq!((TEXT_SIZE, TEXT_LINE_HEIGHT), (14.0, 20.0), "text-sm = 14/20");
        assert_eq!(
            (SMALL_TEXT_SIZE, SMALL_LINE_HEIGHT),
            (12.0, 16.0),
            "text-xs = 12/16"
        );
        // O que o default do GPUI daria, e por que ele não serve.
        const PHI: f32 = 1.618_034;
        assert!(
            TEXT_SIZE * PHI > TEXT_LINE_HEIGHT,
            "o default do GPUI é MAIOR que o par do Tailwind — é a fonte do bug"
        );
        assert!(
            TEXT_SIZE * PHI + 2.0 * ITEM_PAD_Y > ITEM_MIN_HEIGHT,
            "sem o par declarado, o item estouraria o min-h-7"
        );
        assert!(
            TEXT_LINE_HEIGHT + 2.0 * ITEM_PAD_Y <= ITEM_MIN_HEIGHT,
            "com o par declarado, ele cabe"
        );
        // A pastilha também: 16 de linha dentro de 20px de altura mínima no menor tamanho.
        assert!(SMALL_LINE_HEIGHT <= chip_min_height(InputSize::Sm));
    }

    /// **Desvio consciente, travado aqui.** O bisel escuro desta lib é o DOBRO do da referência
    /// (branco ~11,8% em vez de 6%), porque a 6% o filete é imperceptível no nosso fundo. Este
    /// módulo não escolhe isso — ele CONSOME o bisel do `menu.rs` e do `select.rs`, e o teste existe
    /// pra que o popup do combobox nunca fique com um relevo diferente do popup do menu ao lado.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        crate::theme::set_theme(crate::theme::ThemeMode::Dark);
        // Alfa de referência em 8 bits: 6% -> 15.
        const REF_ESCURO: f32 = 15.0 / 255.0;

        // O bisel do POPUP: uma borda de 1px num lado só, com a cor da paleta do menu.
        let cor = bevel_overlay()
            .style()
            .border_color
            .expect("o bisel do popup tem cor");
        assert!(
            (cor.a - REF_ESCURO * 2.0).abs() < 1e-4,
            "o bisel escuro é o DOBRO da referência; veio {}",
            cor.a
        );
        assert!(cor.a < 1.0, "ainda translúcido");

        // O bisel do CAMPO (pastilhas) vem do `select.rs`, e tem que ser o MESMO alfa: os dois
        // aparecem na mesma tela, um dentro do outro.
        let cor_campo = bevel_for(false, false, false, RADIUS, crate::group::Join::NONE)
            .expect("em repouso o bisel do campo existe")
            .style()
            .border_color
            .expect("o bisel do campo tem cor");
        assert!(
            (cor_campo.a - cor.a).abs() < 1e-4,
            "o bisel do campo e o do popup têm que ter o mesmo alfa"
        );

        // No tema CLARO o desvio não existe: preto a 4% (alfa 10), fiel.
        crate::theme::set_theme(crate::theme::ThemeMode::Light);
        let claro = bevel_overlay()
            .style()
            .border_color
            .expect("o bisel tem cor");
        assert!(
            (claro.a - 10.0 / 255.0).abs() < 1e-4,
            "o tema claro segue fiel (preto 4%)"
        );
    }

    /// As cores do popup e do campo vêm das paletas dos módulos vizinhos — este módulo não tem
    /// paleta própria, de propósito. O teste é que elas continuam sendo o que o combobox precisa:
    /// popup OPACO (é o que segura a sombra), destaque translúcido (é um realce, não uma moldura).
    #[test]
    fn cores_vem_das_paletas_vizinhas_e_continuam_servindo() {
        for modo in [crate::theme::ThemeMode::Light, crate::theme::ThemeMode::Dark] {
            crate::theme::set_theme(modo);
            let p = popup_palette();

            let bg: gpui::Rgba = p.popover_bg.hsla().into();
            assert_eq!(bg.a, 1.0, "{modo:?}: o fundo do popup é OPACO");

            let accent: gpui::Rgba = p.accent.hsla().into();
            assert!(
                accent.a < 1.0,
                "{modo:?}: o destaque é translúcido — realce de fundo, não moldura"
            );
            let borda: gpui::Rgba = p.border.hsla().into();
            assert!(borda.a < 1.0, "{modo:?}: a borda do popup é translúcida");

            // O texto do rótulo de grupo é diferente do do item (é `--muted-foreground`).
            let texto: gpui::Rgba = p.text.hsla().into();
            let muted: gpui::Rgba = p.muted.hsla().into();
            assert_ne!(
                (texto.r, texto.g, texto.b),
                (muted.r, muted.g, muted.b),
                "{modo:?}: rótulo de grupo e item não podem ter a MESMA cor"
            );

            // O fundo do campo das pastilhas vem do `select.rs`, e é o `--background`/`bg-input/32`.
            let campo: gpui::Rgba = field_palette().bg.hsla().into();
            assert!(campo.a > 0.0, "{modo:?}: o campo tem fundo");
        }
    }

    /// A precedência de estados da moldura das pastilhas é a do gatilho do dropdown, porque é a
    /// MESMA função. Inválido vence foco; sombra e anel se excluem.
    #[test]
    fn estados_da_moldura_das_pastilhas_seguem_o_dropdown() {
        crate::theme::set_theme(crate::theme::ThemeMode::Dark);

        // Inválido+foco é o mais específico, e é diferente de inválido sem foco.
        assert_ne!(border_color_for(true, true), border_color_for(true, false));
        // Inválido vence o foco.
        assert_ne!(border_color_for(true, true), border_color_for(false, true));

        // Repouso: sombra `shadow-xs`, sem anel.
        assert_eq!(shadow_stack_for(false, false, false).len(), 1);
        assert!(!ring_visible(false, false));
        // Focado: anel, sem sombra.
        assert!(shadow_stack_for(false, false, true).is_empty());
        assert!(ring_visible(false, true));
        // Inválido apaga a sombra mesmo sem foco.
        assert!(shadow_stack_for(false, true, false).is_empty());
        // Desabilitado: sem sombra e sem anel.
        assert!(shadow_stack_for(true, false, false).is_empty());
        assert!(!ring_visible(true, true));

        // O bisel só em repouso.
        let j = crate::group::Join::NONE;
        assert!(bevel_for(false, false, false, RADIUS, j).is_some());
        assert!(bevel_for(false, true, false, RADIUS, j).is_none(), "focado: some");
        assert!(bevel_for(false, false, true, RADIUS, j).is_none(), "inválido: some");
        assert!(bevel_for(true, false, false, RADIUS, j).is_none(), "desabilitado: some");
    }

    // --- Construtores e predicados --------------------------------------------------------------

    /// Os três tipos de linha, e quais são alcançáveis.
    #[test]
    fn tipos_de_linha_e_alcancabilidade() {
        let item = ComboboxItem::new("Normal");
        assert!(item.is_item() && item.is_activatable() && !item.is_disabled());
        assert_eq!(item.label().as_ref(), "Normal");

        let desabilitado = ComboboxItem::new("Normal").disabled(true);
        assert!(desabilitado.is_item(), "continua item");
        assert!(desabilitado.is_disabled());
        assert!(!desabilitado.is_activatable(), "mas o teclado não para nele");

        let rotulo = ComboboxItem::group_label("Modos");
        assert!(!rotulo.is_item() && !rotulo.is_activatable());
        assert_eq!(rotulo.label().as_ref(), "Modos");

        let sep = ComboboxItem::separator();
        assert!(!sep.is_item() && !sep.is_activatable());
        assert!(sep.label().is_empty(), "separador não tem rótulo");

        assert_eq!(ComboboxMode::default(), ComboboxMode::Single);
    }

    // --- Testes de janela (o componente montado de verdade) --------------------------------------
    //
    // O que os testes puros acima não podem ver: se a assinatura do `InputState` está ligada, se a
    // guarda de eco funciona atravessando a fila de efeitos do GPUI, e se o componente RENDERIZA.
    // Um método escrito e nunca chamado passa em todo teste puro deste arquivo.

    /// Uma view raiz mínima que hospeda o combobox.
    struct Harness {
        cb: gpui::Entity<Combobox>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(self.cb.clone())
        }
    }

    #[allow(clippy::type_complexity)]
    fn montar(
        cx: &mut TestAppContext,
        mode: ComboboxMode,
    ) -> (
        gpui::Entity<Combobox>,
        Rc<RefCell<Vec<ComboboxEvent>>>,
        VisualTestContext,
    ) {
        crate::theme::set_theme(crate::theme::ThemeMode::Dark);
        // O campo é o núcleo do `gpui-component`, e ele lê o tema DELE de um global. Sem estes dois
        // `init` o `InputState` explode no primeiro render (é o mesmo preâmbulo do `color_picker`).
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let eventos: Rc<RefCell<Vec<ComboboxEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |window, cx| {
            let cb = cx.new(|cx| Combobox::new(lista(), window, cx).mode(mode));
            cx.subscribe(&cb, move |_this, _c, ev: &ComboboxEvent, _cx| {
                capturados.borrow_mut().push(ev.clone());
            })
            .detach();
            Harness { cb }
        });

        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let cb = vcx.read(|cx| harness.read(cx).cb.clone());
        (cb, eventos, vcx)
    }

    /// Escreve no campo como se fosse digitação (passa pela assinatura, sem a guarda de eco).
    fn digitar(cb: &gpui::Entity<Combobox>, texto: &str, vcx: &mut VisualTestContext) {
        let estado = vcx.read(|cx| cb.read(cx).input.clone());
        vcx.update(|window, cx| {
            estado.update(cx, |st, cx| st.set_value(texto.to_string(), window, cx));
        });
        vcx.run_until_parked();
    }

    /// **Digitar abre o popup, filtra a lista e apaga o destaque.**
    ///
    /// É o caminho que atravessa a fila de efeitos do GPUI: `set_value` → `InputEvent::Change` →
    /// assinatura → `on_typed`. Nenhum teste puro cobre isso, e se a assinatura não estivesse ligada
    /// (ou o `Subscription` fosse descartado) o campo aceitaria texto e a lista nunca filtraria.
    #[gpui::test]
    fn digitar_abre_filtra_e_apaga_o_destaque(cx: &mut TestAppContext) {
        let (cb, eventos, mut vcx) = montar(cx, ComboboxMode::Single);
        assert!(!vcx.read(|cx| cb.read(cx).is_open()), "nasce fechado");

        // Abre pelo teclado e destaca o primeiro item, pra haver destaque pra apagar.
        vcx.update(|window, cx| {
            cb.update(cx, |this, cx| {
                this.on_key(&tecla("down"), window, cx);
            });
        });
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| cb.read(cx).highlighted()), Some(1));

        digitar(&cb, "mult", &mut vcx);

        assert!(vcx.read(|cx| cb.read(cx).is_open()), "digitar mantém aberto");
        assert_eq!(
            vcx.read(|cx| cb.read(cx).highlighted()),
            None,
            "digitar APAGA o destaque"
        );
        // A lista filtrou de verdade: só o item 5 e o rótulo do grupo dele sobraram.
        let visiveis = vcx.read(|cx| cb.read(cx).visible(cx));
        assert_eq!(visiveis, vec![false, false, false, false, true, true, false]);
        assert!(
            eventos
                .borrow()
                .iter()
                .any(|e| matches!(e, ComboboxEvent::Query(q) if q.as_ref() == "mult")),
            "o texto digitado é emitido como Query"
        );
    }

    /// **Escolher no modo único escreve o rótulo no campo, fecha, e a reescrita NÃO vira filtro.**
    ///
    /// É o teste da guarda de eco: sem ela, escrever "Multiply" no campo dispararia um `Change` do
    /// núcleo que religaria o filtro e reabriria o popup — e o combobox nunca conseguiria fechar
    /// depois de uma escolha.
    #[gpui::test]
    fn escolher_no_unico_escreve_o_rotulo_e_nao_religa_o_filtro(cx: &mut TestAppContext) {
        let (cb, eventos, mut vcx) = montar(cx, ComboboxMode::Single);

        digitar(&cb, "mult", &mut vcx);
        assert!(vcx.read(|cx| cb.read(cx).is_open()));

        // Enter com o item 5 destacado (a seta entra pela ponta da lista filtrada).
        vcx.update(|window, cx| {
            cb.update(cx, |this, cx| {
                this.on_key(&tecla("down"), window, cx);
                this.on_key(&tecla("enter"), window, cx);
            });
        });
        vcx.run_until_parked();

        assert_eq!(vcx.read(|cx| cb.read(cx).selected().to_vec()), vec![5]);
        assert_eq!(
            vcx.read(|cx| cb.read(cx).query(cx)).as_ref(),
            "Multiply",
            "o rótulo escolhido virou o texto do campo"
        );
        assert!(
            !vcx.read(|cx| cb.read(cx).is_open()),
            "o modo único FECHA — e a reescrita do campo não pode reabrir"
        );
        assert!(
            !vcx.read(|cx| cb.read(cx).filtering),
            "e a reescrita não religa o filtro"
        );
        // Reabrir mostra a lista INTEIRA, apesar de o campo conter "Multiply".
        vcx.update(|window, cx| {
            cb.update(cx, |this, cx| this.set_open(true, window, cx));
        });
        vcx.run_until_parked();
        assert_eq!(
            vcx.read(|cx| cb.read(cx).visible(cx)),
            vec![true; 7],
            "reabrir NÃO filtra pelo rótulo já escolhido (o shouldBypassFiltering)"
        );
        assert_eq!(
            vcx.read(|cx| cb.read(cx).highlighted()),
            Some(5),
            "e o item selecionado nasce destacado"
        );

        // O `Change` foi emitido uma vez só, com a seleção nova.
        let mudancas: Vec<_> = eventos
            .borrow()
            .iter()
            .filter_map(|e| match e {
                ComboboxEvent::Change(v) => Some(v.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(mudancas, vec![vec![5]]);
    }

    /// **No modo múltiplo, escolher sem filtro mantém o popup aberto e acumula pastilhas**; o ✕ de
    /// uma pastilha remove só ela.
    #[gpui::test]
    fn multiplo_acumula_pastilhas_e_o_x_remove_uma(cx: &mut TestAppContext) {
        let (cb, _ev, mut vcx) = montar(cx, ComboboxMode::Multiple);

        vcx.update(|window, cx| {
            cb.update(cx, |this, cx| {
                this.set_open(true, window, cx);
                this.pick(1, window, cx);
                this.pick(5, window, cx);
            });
        });
        vcx.run_until_parked();

        assert_eq!(vcx.read(|cx| cb.read(cx).selected().to_vec()), vec![1, 5]);
        assert!(
            vcx.read(|cx| cb.read(cx).is_open()),
            "sem filtro, o múltiplo fica ABERTO pra marcar vários"
        );
        assert_eq!(
            vcx.read(|cx| cb.read(cx).selected_labels()),
            vec![
                SharedString::new_static("Normal"),
                SharedString::new_static("Multiply")
            ]
        );

        vcx.update(|_window, cx| {
            cb.update(cx, |this, cx| this.remove_chip(1, cx));
        });
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| cb.read(cx).selected().to_vec()), vec![5], "o ✕ remove só a sua");
    }

    /// Um `KeyDownEvent` pra a tecla dada — os testes acima chamam o handler direto, porque o foco
    /// do campo vive no `InputState` e simular tecla exigiria focá-lo primeiro.
    fn tecla(key: &str) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: gpui::Keystroke {
                modifiers: Modifiers::default(),
                key: key.to_string(),
                key_char: None,
            },
            is_held: false,
        }
    }
}

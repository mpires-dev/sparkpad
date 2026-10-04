//! `Autocomplete` — o **campo de texto livre com sugestões**. Porte do `autocomplete.tsx` do
//! [coss][1], sobre o `Autocomplete` do [Base UI][2].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/autocomplete.tsx`
//! [2]: https://base-ui.com/react/components/autocomplete
//!
//! ```ignore
//! // uma vez, no `new` da sua view:
//! let estado = cx.new(|cx| empire_ui::input::single_line(window, cx).placeholder("Buscar fruta"));
//! let ac = cx.new(|cx| {
//!     Autocomplete::new(&estado, cx)
//!         .rows(vec![
//!             AutocompleteRow::group_label("Cítricas"),
//!             AutocompleteRow::item("Laranja"),
//!             AutocompleteRow::item("Limão"),
//!             AutocompleteRow::separator(),
//!             AutocompleteRow::item("Maçã"),
//!         ])
//!         .show_clear(true)
//! });
//! cx.subscribe(&ac, |_, _, ev: &AutocompleteEvent, _| match ev {
//!     // o VALOR do componente é o texto — inclusive texto que não está na lista
//!     AutocompleteEvent::Change(texto) => println!("valor: {texto}"),
//!     AutocompleteEvent::Select(i) => println!("escolheu a sugestão {i}"),
//! });
//! ```
//!
//! # Autocomplete × Combobox: a diferença é de INTENÇÃO, e não de classes
//!
//! Os dois `.tsx` do coss são quase o mesmo arquivo: os **12 slots** que aparecem nos dois (`Input`,
//! `Popup`, `List`, `Item`, `Group`, `GroupLabel`, `Separator`, `Empty`, `Status`, `Row`, `Trigger`,
//! `Clear`) têm classes **idênticas**, mudando só o nome do marcador (`autocomplete-empty` vs
//! `combobox-empty`). As duas diferenças de desenho são o `Item` (aqui `flex … px-2`, **sem** a
//! coluna do check; lá `grid grid-cols-[1rem_1fr] gap-2 ps-2 pe-4` **com** `ItemIndicator`) e os
//! chips de seleção múltipla (`Chips`/`Chip`/`ChipRemove`/`ChipsInput`), que só o Combobox tem.
//!
//! Mas o que decide qual usar não está no CSS — está na doc do Base UI, e é **de quem é o valor**:
//!
//! > **Avoid when selection state is needed**: Use Combobox instead of Autocomplete if the selection
//! > should be remembered and the input value cannot be custom. Unlike Combobox, Autocomplete's
//! > input can contain free-form text, as its suggestions only *optionally* autocomplete the text.
//! > — <https://base-ui.com/react/components/autocomplete>
//!
//! > **Combobox is a filterable Select**: Use Combobox when the input is restricted to a set of
//! > predefined selectable items. […] **Avoid for simple search widgets**: Combobox does not allow
//! > free-form text input. For search widgets, consider using Autocomplete instead.
//! > — <https://base-ui.com/react/components/combobox>
//!
//! Na fonte isso é literal: `autocomplete/root/AutocompleteRoot.tsx` renderiza o **mesmo** motor
//! (`<AriaCombobox>`) do Combobox com `selectionMode="none"` — o Autocomplete **não tem estado de
//! seleção**, e `selectedValue`/`onSelectedValueChange` são `Omit`-ados do tipo de props dele. O
//! valor que ele expõe (`value`/`onValueChange`) é a **string do input**. É daí que sai tudo o
//! resto: sem seleção não há check pra desenhar (a 1ª diferença), não há o que virar chip (a 2ª), e
//! nada fica destacado ao abrir (ver [`step_highlight`]).
//!
//! A consequência prática pra quem escolhe: **o valor daqui é [`SharedString`]**
//! ([`AutocompleteEvent::Change`]), e as sugestões são um atalho de digitação. Um campo cujo valor
//! *tem* que ser um item da lista é um Combobox, não isto — e usar isto ali significa validar o
//! texto na mão em todo lugar que o lê.
//!
//! # Onde cada pedaço foi buscado
//!
//! | pedaço | de onde veio |
//! |---|---|
//! | classes de todos os 12 slots | do `autocomplete.tsx`, direto |
//! | aritmética de posição do popup | de [`crate::menu`] — [`anchor_point`], [`SIDE_OFFSET`], [`WINDOW_MARGIN`], [`crate::menu::popup_max_height`] |
//! | o campo (moldura, tamanhos, sufixo) | de [`crate::input::Input`] |
//! | filtragem, teclado, abrir/fechar, limpar | da FONTE do `mui/base-ui` (citada função por função abaixo) |
//!
//! # O popup: é a MESMA superfície do [`crate::menu`], não uma terceira
//!
//! As classes do `AutocompletePopup` e as do `MenuPopup` do coss coincidem inteiras —
//! `rounded-lg border bg-popover shadow-lg/5` + o `before:shadow-[0_1px_…]` do bisel — e as do
//! `AutocompleteItem` coincidem com as do item de **ação** do menu, incluindo o respiro
//! **simétrico**: `px-2`, ou seja [`ITEM_PAD_X`] nos dois lados.
//!
//! ⚠️ **Não é a geometria de item do [`crate::select`].** Lá o item é o grid `ps-2 pe-4` — 8px no
//! início e **16** no fim — porque ele tem a coluna do check; é a geometria do *Combobox*, não a
//! desta referência. Copiá-la daqui deslocaria todo rótulo 8px pra esquerda do centro da linha. O
//! que o `select.rs` de fato compartilha com este módulo são os *números* que os dois herdam do
//! mesmo `--radius`/`--spacing` do coss, e esses estão declarados um por um nas constantes abaixo.
//!
//! Cores e medidas ficam num par (`crate::menu::MenuPalette`, as constantes) **deste** módulo, como em
//! [`crate::menu`], [`crate::tooltip`], [`crate::input`] e [`crate::card`]: é a convenção da casa
//! para tokens de UM componente (ver [`crate::color`]). O que é compartilhado de verdade — a
//! aritmética de `side`/`align`/offset e o teto de altura — é **importado**, não recopiado.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`-translate-y-1/2`** do `AutocompleteTrigger`/`AutocompleteClear`: o GPUI 0.2 não tem
//!   `transform` em `div`. Na referência o botão é `absolute top-1/2` e a metade da própria altura é
//!   descontada pelo transform; aqui ele entra no [`crate::input::Input::suffix`], que é uma linha
//!   flex `items_center` — a centralização vertical sai por **layout**, e o resultado é o mesmo
//!   pixel. Ver também "Desvio consciente" para o eixo horizontal, que *não* sai igual.
//! - **`transition-colors` / `transition-[…,opacity]`** dos dois botões e o
//!   `transition-[scale,opacity]` do popup: o GPUI não interpola estilo. Hover e abertura são
//!   instantâneos. Mesma limitação já declarada em [`crate::menu`] e [`crate::tooltip`].
//! - **`z-50`** do `Positioner`: não há `z-index`; a ordem é a de pintura. O popup vai em
//!   [`gpui::deferred`] com prioridade [`POPUP_PRIORITY`] — a mesma de [`crate::menu`],
//!   [`crate::select`] e [`crate::tooltip`], porque na referência os quatro escrevem o mesmo `z-50`.
//! - **`origin-(--transform-origin)`** (origem da animação de escala): sem transform, sem origem.
//! - **`pointer-coarse:after:min-h-11 min-w-11`** (alvo de 44px pra dedo) nos dois botões: consulta
//!   de MÍDIA que nunca vale numa janela de desktop com mouse. Ramo morto, não omissão.
//! - **`in-data-has-overflow-y:pe-3`** na lista (reservar 12px pra barra quando ela transborda): o
//!   GPUI não tem seletor de "quando este container transborda". O popup do [`crate::menu`] também
//!   não reserva nada, e os dois aparecem lado a lado.
//! - **`not-dark:bg-clip-padding`** no popup: `background-clip` não existe no GPUI. Ele só importa
//!   quando a borda é translúcida E o fundo vaza por baixo dela; com `--popover` opaco (o
//!   `popover_bg` da paleta deste módulo) não há vazamento pra recortar.
//! - **`select-none`** no `Positioner`: não há seleção de texto a suprimir no popup — os itens são
//!   `div`s pintadas, não texto selecionável.
//!
//! **Resolvido em número**
//!
//! - O breakpoint `sm:` do Tailwind é ≥640px e uma janela de desktop está sempre acima disso: valem
//!   SEMPRE as variantes `sm:`. Item de **28px** (`sm:min-h-7`, não `min-h-8`) e texto de **14px**
//!   (`sm:text-sm`, não `text-base`). Mesma decisão de [`crate::menu`] e [`crate::select`].
//! - `max-h-[min(var(--available-height),23rem)]` → o teto é o **menor** entre o espaço livre em
//!   volta do campo e [`POPUP_MAX_HEIGHT`] = **368px** (`23rem`). O espaço livre vem de
//!   [`crate::menu::popup_max_height`], que é a mesma conta do menu (com o mesmo piso de 96px).
//! - `min-w-(--anchor-width)` → o popup tem no mínimo a largura medida do campo, com o piso de
//!   [`MIN_WIDTH`] pro primeiro frame (antes do prepaint os bounds são zero).
//! - `max-w-(--available-width)` → ver [`max_width`]: a janela menos as duas margens. Sem o teto o
//!   `snap_to_window_with_margin` deslizaria o popup pra dentro sem encolhê-lo, e um rótulo comprido
//!   ficaria cortado fora da tela.
//! - `[[role=group]+&]:mt-1.5` → [`GROUP_GAP`] = **6px** de folga acima de um rótulo de grupo que
//!   não é a primeira linha visível.
//! - **Entrelinha**, em toda linha de texto: ver [`TEXT_LINE_HEIGHT`] e [`SMALL_LINE_HEIGHT`]. O
//!   default do GPUI é a razão de ouro, não o par do Tailwind — a armadilha está explicada lá.
//!
//! **Desvio consciente**
//!
//! - **`end-0.5`** dos botões de gatilho/limpar (2px da borda direita do campo, sobrepondo o respiro
//!   dele) → aqui o botão é um **sufixo em fluxo** do [`crate::input::Input`], então ele respeita o
//!   respiro do campo ([`crate::input::InputSize::pad_x`], 9–11px). Encostá-lo nos 2px exigiria
//!   margem negativa, que esta base não usa (e o `pe-7` que a referência põe no texto pra abrir
//!   espaço deixa de ser necessário, porque em fluxo o texto simplesmente não passa por baixo do
//!   botão). Consequência visível: o ícone fica ~5px mais pra dentro do que no original.
//! - **O botão é o [`crate::input::Input::icon_button`]** — o mesmo alvo do olho do campo de senha,
//!   quadrado de `ícone + 6` — e não o `size-7`/`size-8` da referência. É a peça que já existe pra
//!   este lugar exato; um segundo desenho de botão-dentro-de-campo divergiria do primeiro na
//!   próxima mudança de tema.
//! - ~~**Ícone do gatilho**~~: **resolvido na integração.** Este porte nasceu com
//!   `icons/chevron_down.svg` (chevron simples), justificado por consistência com o
//!   [`crate::select`]. Mas o [`crate::combobox`] — o IRMÃO deste componente, portado na mesma
//!   rodada — escolheu o chevron DUPLO, fiel ao `ChevronsUpDownIcon` que a referência importa pros
//!   dois. As duas justificativas se sustentavam sozinhas e se contradiziam juntas: os dois
//!   componentes convivem no mesmo formulário e a referência lhes dá o MESMO ícone. Ficou o duplo
//!   ([`TRIGGER_ICON`]), e o desvio do chevron simples sobrou só no [`crate::select`], que agora é o
//!   ímpar da família — candidato a acertar numa próxima passada, com verificação própria.
//! - **Tamanho dos ícones**: o `startAddon`, o gatilho e o limpar seguem a escala de ícone do CAMPO
//!   ([`crate::input::InputSize::icon`], 13/15/17px), e não o `sm:size-4` fixo (16px) da referência.
//!   É a escala que todo prefixo/sufixo desta lib usa — inclusive o olho do campo de senha — e um
//!   ícone de 16 ao lado de um de 13 no mesmo formulário salta mais que a diferença com o original.
//! - **`[&_svg]:-mx-0.5`** no `startAddon` (o glifo avança 2px pros lados pra compensar a caixa do
//!   ícone do lucide): é **margem negativa**, que esta base não usa (ver o `ICON_PULL` do
//!   [`crate::menu`], que existe justamente pra resolver o mesmo `-mx-0.5` por conta positiva). Aqui
//!   ele simplesmente não se aplica: o prefixo do [`crate::input::Input`] já é um filho em fluxo com
//!   o respiro do campo, não um bloco absoluto a compensar.
//! - **A lista NÃO vai dentro de um [`crate::scroll_area::ScrollArea`]**, e a referência põe
//!   (`<ScrollArea overscrollContain scrollbarGutter scrollFade>`). O nosso `ScrollArea` aninha o
//!   conteúdo um nível abaixo do elemento que ele rastreia, e
//!   [`gpui::ScrollHandle::scroll_to_item`] conta os **filhos diretos** do rastreado — dentro dele o
//!   índice do item não existe, e andar com as setas pararia de trazer o item pra dentro da vista.
//!   Perder isso é pior que perder o fade e a barra que se esconde. Então a rolagem é a mesma do
//!   popup do [`crate::menu`]: `overflow_y_scroll` + `track_scroll`.
//! - **`empty:m-0 empty:p-0`** do `Status` e **`not-empty:p-2`** do `Empty`: em vez de zerar o
//!   respiro de um elemento vazio, a linha simplesmente **não é montada** quando não há texto —
//!   mesmo resultado, e sem um `div` de tamanho zero na árvore.
//! - **Rótulo de grupo e separador órfãos somem.** Ver [`visible_rows`]: um rótulo cujo grupo perdeu
//!   todos os itens no filtro, e um separador sem linha viva antes **e** depois, não são montados. No
//!   Base UI a estrutura não é filtrada (quem filtra é o app, via `filteredItems`/`Collection`), e um
//!   grupo vazio sobra na tela; o `last:hidden` que o `.tsx` põe no separador é justamente o remendo
//!   parcial dessa sobra. A regra daqui **contém** aquele `last:hidden` e cobre os outros três casos.
//! - **Dobra de acentos própria** em vez do `Intl.Collator`: ver [`fold`]. Cobre Latin-1 + Latin
//!   Extended-A (todo o português, espanhol, francês, alemão); fora daí a comparação continua
//!   correta, só não é insensível a diacrítico.
//! - **O [`Echo`]** não tem equivalente na referência, e não é enfeite: no React o `setInputValue` do
//!   `ComboboxClear`/`clickHighlightedItem` é síncrono no mesmo reducer, e aqui o `InputEvent::Change`
//!   do [`InputState`] volta um flush de efeitos DEPOIS. Sem ele, escolher uma sugestão reabria a
//!   lista e limpar a fechava — os dois ao contrário do original. Leia o doc de [`Echo`].
//!
//! **Superset consciente**
//!
//! - **`Home`/`End` no popup aberto continuam movendo o CURSOR do texto**, e é o que a referência
//!   faz (`ComboboxInput.tsx` intercepta as duas e chama `setSelectionRange`, e o
//!   `useListNavigation` pula o salto pra primeiro/último item quando a referência é digitável). Não
//!   há superset aqui — está nesta lista porque é a única tecla de menu que **não** foi ligada, e
//!   procurá-la é o reflexo de quem conhece o [`crate::menu`].
//! - O piso de altura do popup ([`crate::menu::popup_max_height`] usa 96px): a referência não tem
//!   piso nenhum. Sem ele um campo colado no rodapé abriria um popup de altura ~0 e inclicável.
//!
//! **Ausente**
//!
//! - **`mode`** (`'list' | 'both' | 'inline' | 'none'`, default `'list'`). Só o default está
//!   implementado: **a lista filtra e o texto do input NÃO acompanha o item destacado** — ele muda
//!   quando um item é de fato escolhido. Os outros três valores pedem *inline completion* (um texto
//!   de sobreposição temporário, que o `InputState` não tem como representar sem reescrever a
//!   camada de edição) e/ou itens estáticos.
//! - **`Chips`/`Chip`/`ChipRemove`/`ChipsInput`**: não existem nesta referência — são do Combobox
//!   (seleção múltipla). Ver o topo deste doc.
//! - **`autoHighlight`** (default `false`), **`limit`** (default `-1`), **`filter`/`filteredItems`**
//!   (trocar o casador), **`openOnInputClick`** (default `false`), **`keepHighlight`** (default
//!   `false`), **`highlightItemOnHover`** (default `true`), **`loopFocus`** (default `true`),
//!   **`submitOnItemClick`** (default `false`), **`readOnly`**: só os **defaults** estão
//!   implementados, sem setter. Cada default está citado no ponto do código que o realiza.
//! - **`Autocomplete.Value`**, **`Autocomplete.Collection`**, **`Autocomplete.Row`** e
//!   `useAutocompleteFilter`: peças de composição do React (render-prop, grade de itens, hook de
//!   filtro exposto). Aqui a composição é a lista de [`AutocompleteRow`] e o filtro é [`fold`].
//! - **`anchor` do `Positioner`** (ancorar em outro elemento que não o campo): ausente, como em
//!   [`crate::menu`] e [`crate::tooltip`].
//! - **RTL**: [`MenuSide::Left`]/[`MenuSide::Right`] são o `inline-start`/`inline-end` resolvidos pra
//!   LTR, e `end-*`/`ps-*`/`pe-*` também. Mesma nota do [`crate::menu`].

use crate::input::{Input, InputSize};
use crate::menu::{
    align_center_wrap, anchor_point, popup_max_height, rect_of, MenuAlign, MenuSide, SIDE_OFFSET,
    WINDOW_MARGIN,
};
use gpui::{
    anchored, canvas, deferred, div, point, prelude::FluentBuilder as _, px, AnyElement, App,
    Bounds, Context, CursorStyle, Div, Entity, EventEmitter, FocusHandle, Focusable, FontWeight,
    InteractiveElement, IntoElement, MouseDownEvent, ParentElement, Pixels, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Enter, Escape, InputEvent, InputState, MoveDown, MoveUp};

// =================================================================================================
// Eventos
// =================================================================================================

/// O que o [`Autocomplete`] emite.
///
/// ⚠️ **O valor deste componente é o TEXTO** — é o que o separa de um combobox (ver o topo do doc do
/// módulo). Quem grava o valor assina [`AutocompleteEvent::Change`]; [`AutocompleteEvent::Select`]
/// existe para quem precisa saber que a mudança veio de uma sugestão e não da digitação (registrar
/// uma métrica, buscar o objeto inteiro por índice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutocompleteEvent {
    /// O texto mudou — por digitação, por escolha de uma sugestão ou pelo botão de limpar.
    Change(SharedString),
    /// Uma sugestão foi escolhida; carrega o índice dela em [`Autocomplete::rows`]. Vem **sempre**
    /// seguido de um [`AutocompleteEvent::Change`] com o rótulo dela.
    Select(usize),
}

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Os utilitários Tailwind do `autocomplete.tsx` resolvidos em número. As cores vêm das custom
// properties do tema do coss com a paleta `neutral` do Tailwind resolvida (`neutral-100 #f5f5f5`,
// `neutral-500 #737373`, `neutral-800 #262626`); onde o coss usa `color-mix`, o resultado já está
// calculado aqui.
//
// Fica num struct próprio (e não na `crate::theme::Palette`) porque são tokens de UM componente — é
// o mesmo padrão de `crate::menu`, `crate::tooltip`, `crate::input` e `crate::card`.

/// Tokens visuais do popup, por tema.
///
/// ⚠️ **TODAS as cores aqui são `0xRRGGBBAA`** — com o byte de alfa, sempre, mesmo quando opacas
/// (`…ff`). É a convenção das paletas de componente do crate (ver [`crate::color`]); misturar com os
/// tokens de 6 dígitos da [`crate::theme::Palette`] **desloca os canais** e dá uma cor completamente
/// diferente, **sem erro de compilação** (`rgba(0xffffff)` é lido como `0x00FFFFFF`, ou seja ciano).
/// Já custou três bugs visíveis nesta base. A única ponte é [`crate::color::opaque`].
/// A paleta do popup é **a do [`crate::menu`]**, importada — não uma cópia.
///
/// Isto começou como uma `crate::menu::MenuPalette` própria, seguindo o precedente do
/// [`crate::tooltip`]. Na integração a comparação campo a campo mostrou o que aquele precedente
/// esconde: os **nove** campos, nos **dois** temas, eram byte a byte iguais aos do menu — 18 valores
/// de cor com duas fontes de verdade. E o [`crate::combobox`], o irmão deste componente, já importava
/// `menu::palette()`. Duas cópias divergem no dia em que alguém corrige uma; e "corrigir a cor do
/// popup" é exatamente o tipo de mudança que se faz num lugar só.
///
/// Se um dia o popup do autocomplete precisar de uma cor que o do menu não tem, o caminho é um campo
/// novo lá — não uma paleta nova aqui.
use crate::menu::palette;

// =================================================================================================
// Geometria
// =================================================================================================
//
// 1 unidade Tailwind = 4px; `--radius` = 10px. Onde há variante `sm:`, é ela que vale (ver
// "Resolvido em número", no doc do módulo).

/// Raio da superfície do popup — `rounded-lg` = `--radius-lg` = **10px**.
const RADIUS: f32 = 10.0;

/// Espessura da borda do popup — `border`.
const BORDER: f32 = 1.0;

/// Raio de um item — `rounded-sm` = `--radius-sm` = `calc(var(--radius) - 4px)` = **6px**.
const ITEM_RADIUS: f32 = 6.0;

/// Respiro da lista — `p-1`.
const LIST_PAD: f32 = 4.0;

/// Altura mínima de um item — `sm:min-h-7`.
const ITEM_MIN_HEIGHT: f32 = 28.0;

/// Respiro **vertical** de um item — `py-1`.
const ITEM_PAD_Y: f32 = 4.0;

/// Respiro **horizontal** de um item — `px-2`, ou seja o MESMO nos dois lados.
///
/// ⚠️ É aqui que esta referência se separa da do [`crate::select`], onde o item é `ps-2 pe-4` (8 e
/// **16**) porque tem a coluna do check. Ver o doc do módulo.
const ITEM_PAD_X: f32 = 8.0;

/// Corpo do texto de um item, do `Empty` e do valor — `sm:text-sm`.
const TEXT_SIZE: f32 = 14.0;

/// **Entrelinha** do texto de 14px — o `text-sm` do Tailwind é `14px/20px`.
///
/// ⚠️ **Isto NÃO é decoração.** O GPUI não usa a entrelinha do CSS; o default dele é
/// `relative(phi())`, a razão de ouro (**1,618034** × o corpo, em `gpui/src/geometry.rs`). Sem
/// fixar, a linha sairia com 22,65px em vez de 20 — 13% mais alta — e um item com uma linha de texto
/// mediria `22,65 + 2×4 = 30,65px` em vez dos **28** que o `sm:min-h-7` manda, sem nada no código
/// parecendo errado. Travado em `a_entrelinha_e_a_do_tailwind_e_nao_a_razao_de_ouro`, que mostra os
/// dois números.
const TEXT_LINE_HEIGHT: f32 = 20.0;

/// Corpo do texto do rótulo de grupo e do `Status` — `text-xs`.
const SMALL_TEXT_SIZE: f32 = 12.0;

/// **Entrelinha** do texto de 12px — o `text-xs` do Tailwind é `12px/16px`. Mesma armadilha de
/// [`TEXT_LINE_HEIGHT`]: sem fixar sairia 19,42px.
const SMALL_LINE_HEIGHT: f32 = 16.0;

/// Respiro **horizontal** do rótulo de grupo — `px-2`.
const LABEL_PAD_X: f32 = 8.0;

/// Respiro **vertical** do rótulo de grupo — `py-1.5`.
const LABEL_PAD_Y: f32 = 6.0;

/// Folga acima de um rótulo de grupo que não abre a lista — o `[[role=group]+&]:mt-1.5`.
const GROUP_GAP: f32 = 6.0;

/// Margem horizontal do separador — `mx-2`.
const SEPARATOR_MARGIN_X: f32 = 8.0;

/// Margem vertical do separador — `my-1`.
const SEPARATOR_MARGIN_Y: f32 = 4.0;

/// Espessura do separador — `h-px`.
const SEPARATOR_HEIGHT: f32 = 1.0;

/// Respiro da linha de `Empty` — `not-empty:p-2`, nos quatro lados.
const EMPTY_PAD: f32 = 8.0;

/// Respiro **horizontal** da linha de `Status` — `px-3`.
const STATUS_PAD_X: f32 = 12.0;

/// Respiro **vertical** da linha de `Status` — `py-2`.
const STATUS_PAD_Y: f32 = 8.0;

/// Opacidade de um item desabilitado — `data-disabled:opacity-64`. Esmaece o item inteiro, em vez de
/// exigir um par "apagado" de cada token.
const DISABLED_OPACITY: f32 = 0.64;

/// Opacidade do ícone-prefixo do campo — o `opacity-80` do `autocomplete-start-addon`.
///
/// É o mesmo valor (e o mesmo papel) do `ICON_OPACITY` do [`crate::menu`] e do [`crate::select`]: o
/// glifo acompanha o texto sem competir com ele.
///
/// `pub(crate)` porque o `CommandInput` do [`crate::command`] **é** este `AutocompleteInput` — o
/// `command.tsx` o renderiza com `startAddon={<SearchIcon />}` e só troca a moldura. É o mesmo nó do
/// DOM com a mesma classe; um segundo `0.8` seria a segunda verdade pro mesmo glifo.
pub(crate) const ADDON_OPACITY: f32 = 0.8;

/// Largura mínima do popup, usada **só** enquanto o campo não foi medido.
///
/// A referência diz `min-w-(--anchor-width)`, ou seja "a largura do campo": no primeiro frame o
/// prepaint ainda não mediu nada e os bounds são zero, e um popup de largura zero é invisível. O
/// número é o `min-w-36` que o coss usa como piso de campo em outros lugares.
const MIN_WIDTH: f32 = 144.0;

/// Teto **absoluto** da altura do popup — o `23rem` do `max-h-[min(var(--available-height),23rem)]`.
///
/// O outro termo do `min()` (o espaço livre em volta do campo) vem de
/// [`crate::menu::popup_max_height`], que é a mesma conta do popup do menu.
const POPUP_MAX_HEIGHT: f32 = 368.0;

/// As duas camadas do `--shadow-lg` do Tailwind v4, em `(dy, blur, spread)`.
///
/// O coss não redefine `--shadow-lg` no `globals.css`, então vale o default:
/// `0 10px 15px -3px …, 0 4px 6px -4px …`. A COR não está aqui porque o `/5` do `shadow-lg/5` a
/// substitui inteira por preto 5% — ela é o `shadow` da `crate::menu::MenuPalette`.
const SHADOW_LAYERS: [(f32, f32, f32); 2] = [(10.0, 15.0, -3.0), (4.0, 6.0, -4.0)];

/// Prioridade do [`gpui::deferred`] do popup — **a mesma** do [`crate::menu`], do [`crate::select`] e
/// do [`crate::tooltip`], porque na referência os quatro escrevem o mesmo `z-50`. Empatados na
/// prioridade, quem pinta por cima é quem vem depois na árvore, que é a regra do `z-index` igual.
const POPUP_PRIORITY: usize = 1;

/// Lado default — o `side="bottom"` da referência. Coincide com [`MenuSide::default()`] e é escrito
/// por extenso pelo mesmo motivo do `DEFAULT_SIDE` do [`crate::tooltip`]: um default herdado em
/// silêncio é o que pega o valor errado quando o outro módulo muda.
const DEFAULT_SIDE: MenuSide = MenuSide::Bottom;

/// Alinhamento default — o `align="start"` da referência.
///
/// ⚠️ **NÃO é o [`MenuAlign::default()`]**, que é `Center` (o default do *menu*). O popup de um
/// autocomplete encosta na borda inicial do campo, como o de um `<select>`.
const DEFAULT_ALIGN: MenuAlign = MenuAlign::Start;

/// Ícone do botão de gatilho. Ver "Desvio consciente" no doc do módulo: a referência usa o chevron
/// DUPLO do lucide, e aqui é o simples — o mesmo do [`crate::select`], com quem este campo divide a
/// tela.
const TRIGGER_ICON: &str = "iconoir/regular/arrow-separate-vertical.svg";

/// Ícone do botão de limpar — o `XIcon` do lucide.
const CLEAR_ICON: &str = "icons/x.svg";

// =================================================================================================
// As linhas da lista
// =================================================================================================

/// Que tipo de linha é.
#[derive(Clone, Debug, PartialEq, Eq)]
enum RowKind {
    /// Uma sugestão clicável — o `AutocompleteItem`.
    Item {
        /// Rótulo, e também o texto que vai pro campo ao escolher (`fillInputOnItemPress`).
        label: SharedString,
        /// `data-disabled`: não é clicável, o teclado a pula, e ela esmaece.
        disabled: bool,
    },
    /// O `AutocompleteGroupLabel` — o título de um grupo. Não é clicável nem filtrada por si.
    GroupLabel(SharedString),
    /// O `AutocompleteSeparator` — o filete entre blocos.
    Separator,
}

/// Uma linha da lista de sugestões.
///
/// A lista é **plana** de propósito: no `.tsx` o grupo é um elemento que embrulha os itens, mas o que
/// ele produz visualmente é um rótulo seguido dos itens dele — e um `Vec` plano é o que permite
/// filtrar, navegar e rolar por um índice só (é a mesma decisão do [`crate::menu::MenuItem`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutocompleteRow {
    kind: RowKind,
}

impl AutocompleteRow {
    /// Uma sugestão clicável.
    pub fn item(label: impl Into<SharedString>) -> Self {
        Self {
            kind: RowKind::Item {
                label: label.into(),
                disabled: false,
            },
        }
    }

    /// Um rótulo de grupo (`AutocompleteGroupLabel`).
    pub fn group_label(text: impl Into<SharedString>) -> Self {
        Self {
            kind: RowKind::GroupLabel(text.into()),
        }
    }

    /// Um separador (`AutocompleteSeparator`).
    pub fn separator() -> Self {
        Self {
            kind: RowKind::Separator,
        }
    }

    /// Marca a sugestão como desabilitada. Em rótulo e separador não faz nada — eles já não são
    /// clicáveis.
    pub fn disabled(mut self, disabled: bool) -> Self {
        if let RowKind::Item { disabled: d, .. } = &mut self.kind {
            *d = disabled;
        }
        self
    }

    /// O rótulo, quando é uma sugestão.
    pub fn label(&self) -> Option<&SharedString> {
        match &self.kind {
            RowKind::Item { label, .. } => Some(label),
            _ => None,
        }
    }

    /// Se o teclado alcança esta linha: sugestão, e não desabilitada.
    fn is_navigable(&self) -> bool {
        matches!(self.kind, RowKind::Item { disabled: false, .. })
    }

    /// Se é uma sugestão (habilitada ou não) — é o que decide se a lista está "vazia" pro `Empty`.
    fn is_item(&self) -> bool {
        matches!(self.kind, RowKind::Item { .. })
    }
}

// =================================================================================================
// Filtragem (o `filter` default do Base UI, resolvido)
// =================================================================================================

/// Dobra um caractere pra comparação: minúscula e **sem diacrítico**.
///
/// É o `sensitivity: 'base'` do `Intl.Collator` que o Base UI usa
/// (`packages/react/src/combobox/internals/filter.ts`: `usage: 'search', sensitivity: 'base',
/// ignorePunctuation: true`), reduzido ao que dá pra fazer sem uma tabela Unicode inteira: Latin-1
/// Supplement e Latin Extended-A, que é todo o português, espanhol, francês, alemão, polonês e
/// tcheco. Fora dessas faixas a dobra é só a de caixa — a comparação continua correta, só não é
/// insensível a diacrítico (declarado no doc do módulo).
///
/// A dobra é **1 para 1** de propósito: `ß`→`ss` e `æ`→`ae` mudariam o comprimento e são
/// dependentes de locale no collator de verdade.
fn fold_char(c: char) -> char {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => 'c',
        'ď' | 'đ' => 'd',
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => 'e',
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => 'g',
        'ĥ' | 'ħ' => 'h',
        'í' | 'ì' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => 'i',
        'ĵ' => 'j',
        'ķ' => 'k',
        'ĺ' | 'ļ' | 'ľ' | 'ł' => 'l',
        'ñ' | 'ń' | 'ņ' | 'ň' => 'n',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => 'o',
        'ŕ' | 'ŗ' | 'ř' => 'r',
        'ś' | 'ŝ' | 'ş' | 'š' => 's',
        'ţ' | 'ť' | 'ŧ' => 't',
        'ú' | 'ù' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => 'u',
        'ŵ' => 'w',
        'ý' | 'ÿ' | 'ŷ' => 'y',
        'ź' | 'ż' | 'ž' => 'z',
        outro => outro,
    }
}

/// Dobra um texto inteiro pra comparação: cada caractere por [`fold_char`], e a **pontuação
/// descartada** (o `ignorePunctuation: true` do collator).
///
/// "Pontuação" aqui é tudo que não é alfanumérico nem espaço em branco — o Unicode do `std` não
/// expõe a categoria `P`, e esta aproximação também come símbolos (`+`, `%`). Em rótulo de UI o
/// efeito é o pretendido: `"E-mail"` e `"Email"` casam, `"(novo)"` casa com `"novo"`.
fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .map(fold_char)
        .collect()
}

/// Se um rótulo casa com a busca — o `contains` do Base UI: **substring em qualquer posição**,
/// insensível a caixa e a diacrítico.
///
/// Busca vazia casa com tudo (`if (!query) { return true; }`, em `internals/filter.ts`). Uma busca
/// que só tem pontuação dobra pra vazia e também casa com tudo: é uma consequência declarada da
/// dobra, e o collator de verdade responderia `false` ali.
fn matches_query(label: &str, query: &str) -> bool {
    let q = fold(query);
    if q.is_empty() {
        return true;
    }
    fold(label).contains(&q)
}

/// O que [`visible_rows`] e [`navigable_rows`] precisam saber sobre uma linha.
///
/// Existe porque **o filtro é o mesmo em dois componentes**: este e o [`crate::command`] (o
/// `command.tsx` do coss é literalmente um `<Autocomplete autoHighlight="always" inline keepHighlight
/// open>`). As duas listas têm os mesmos três tipos de linha e as mesmas três regras de poda, mas
/// **tipos de linha diferentes** — a do paladar de comandos carrega o slot de atalho, que um
/// `AutocompleteRow` não tem. Duas cópias de [`visible_rows`] divergiriam na primeira correção da
/// regra do separador; um `trait` de quatro perguntas não.
///
/// Só as perguntas que o filtro faz entram aqui. Rótulo, cor, atalho e ícone são desenho, e desenho
/// não é assunto do filtro.
pub(crate) trait FilterRow {
    /// O texto que o filtro compara. `Some` só nas linhas que são **item** — rótulo de grupo e
    /// separador não são filtrados por si (ver [`visible_rows`]).
    fn match_text(&self) -> Option<&str>;

    /// Se é um **rótulo de grupo**.
    fn is_label(&self) -> bool;

    /// Se é um **separador**.
    fn is_divider(&self) -> bool;

    /// Se o **teclado** alcança a linha: item, e não desabilitado.
    fn keyboard_reachable(&self) -> bool;

    /// Se é um item (habilitado ou não) — é o que decide se a lista está "vazia" pro `Empty`, e onde
    /// termina o grupo de um rótulo.
    fn is_row_item(&self) -> bool {
        self.match_text().is_some()
    }
}

impl FilterRow for AutocompleteRow {
    fn match_text(&self) -> Option<&str> {
        match &self.kind {
            RowKind::Item { label, .. } => Some(label.as_ref()),
            _ => None,
        }
    }

    fn is_label(&self) -> bool {
        matches!(self.kind, RowKind::GroupLabel(_))
    }

    fn is_divider(&self) -> bool {
        matches!(self.kind, RowKind::Separator)
    }

    fn keyboard_reachable(&self) -> bool {
        self.is_navigable()
    }
}

/// Os índices das linhas que sobrevivem ao filtro, na ordem em que aparecem.
///
/// A regra por tipo de linha:
///
/// - **sugestão**: sobrevive se casa com a busca ([`matches_query`]);
/// - **rótulo de grupo**: sobrevive se alguma sugestão **do grupo dele** sobreviveu — o grupo vai do
///   rótulo até a próxima linha que não é sugestão;
/// - **separador**: sobrevive se há linha viva **antes** e **depois** dele.
///
/// As duas últimas são superset da referência (ver o doc do módulo). A do separador **contém** o
/// `last:hidden` que o `.tsx` põe nele — e cobre também o separador que virou o primeiro, e o par de
/// separadores que ficou colado.
pub(crate) fn visible_rows<R: FilterRow>(rows: &[R], query: &str) -> Vec<usize> {
    // 1ª passada: as sugestões que casam, e os rótulos com pelo menos uma sugestão viva no grupo.
    let mut keep = vec![false; rows.len()];
    for (i, row) in rows.iter().enumerate() {
        if let Some(label) = row.match_text() {
            keep[i] = matches_query(label, query);
        }
    }
    for (i, row) in rows.iter().enumerate() {
        if !row.is_label() {
            continue;
        }
        // O grupo termina na próxima linha que não é sugestão (outro rótulo, ou um separador).
        keep[i] = rows[i + 1..]
            .iter()
            .enumerate()
            .take_while(|(_, r)| r.is_row_item())
            .any(|(k, _)| keep[i + 1 + k]);
    }

    // 2ª passada: os separadores, que dependem de já se saber quem sobrou dos dois lados.
    let mut vivos: Vec<usize> = (0..rows.len()).filter(|&i| keep[i]).collect();
    for (i, row) in rows.iter().enumerate() {
        if !row.is_divider() {
            continue;
        }
        let antes = vivos.iter().any(|&v| v < i);
        let depois = vivos.iter().any(|&v| v > i);
        keep[i] = antes && depois;
    }
    vivos = (0..rows.len()).filter(|&i| keep[i]).collect();
    vivos
}

/// Os índices das linhas que o **teclado** alcança: as visíveis que são sugestões habilitadas.
pub(crate) fn navigable_rows<R: FilterRow>(rows: &[R], visible: &[usize]) -> Vec<usize> {
    visible
        .iter()
        .copied()
        .filter(|&i| rows[i].keyboard_reachable())
        .collect()
}

// =================================================================================================
// Comportamento (o `AriaCombobox` do Base UI, resolvido)
// =================================================================================================

/// O próximo item destacado ao andar com as setas — **incluindo o estado "nenhum" no ciclo**.
///
/// `from` e o retorno são índices em `rows`; `nav` é a saída de [`navigable_rows`].
///
/// ⚠️ **Isto NÃO é o `step_index` do [`crate::menu`]**, e a diferença é comportamento, não estilo. Lá
/// o ciclo tem `nav.len()` estados e o último item volta direto pro primeiro. Aqui o ciclo tem
/// **`nav.len() + 1`**: passando do fim, o destaque **volta pro campo** (nenhum item aceso) e só na
/// tecla seguinte reentra pela outra ponta. É a combinação de dois defaults do Base UI, os dois no
/// `useListNavigation` do `AriaCombobox`:
///
/// - `loopFocus` (default `true`): "Whether to loop keyboard focus back to the **input** when the end
///   of the list is reached […] The first item can then be reached by pressing ArrowDown again from
///   the input";
/// - `allowEscape: loopFocus && !autoHighlightMode`, que com `autoHighlight` no default `false` é
///   `true` — é ele que autoriza o estado sem destaque no meio do ciclo.
///
/// Num menu isso seria errado (não há campo pra voltar); aqui é o que deixa o usuário desfazer a
/// navegação e voltar a ver o que digitou.
///
/// `from` fora de `nav` (índice velho de antes de um filtro) é tratado como "nenhum" e entra pela
/// ponta — nunca trava.
fn step_highlight(from: Option<usize>, nav: &[usize], forward: bool) -> Option<usize> {
    if nav.is_empty() {
        return None;
    }
    let pos = from.and_then(|r| nav.iter().position(|&n| n == r));
    match (pos, forward) {
        (None, true) => Some(nav[0]),
        (None, false) => Some(nav[nav.len() - 1]),
        (Some(i), true) if i + 1 < nav.len() => Some(nav[i + 1]),
        (Some(_), true) => None,
        (Some(0), false) => None,
        (Some(i), false) => Some(nav[i - 1]),
    }
}

/// Se a lista fica aberta depois de o texto mudar por **digitação**.
///
/// Duas metades do Base UI, as duas no `ComboboxInput`:
///
/// - abre com texto: `maybeOpenOnInput(trimmed)` — `if (readOnly || disabled || !trimmed ||
///   !shouldOpenOnInput) return;` então `setOpen(true, …'input-change')`. É **o texto aparado** que
///   decide, logo espaço em branco puro não abre;
/// - **fecha ao esvaziar**: `if (empty && !inputInsidePopup) { … if (!store.state.openOnInputClick)
///   setOpen(false, …'input-clear'); }`. O Autocomplete fixa `openOnInputClick = false` (é o default
///   dele, e o do Combobox é `true`), então a condição vale sempre aqui.
///
/// ⚠️ Note o assimétrico com o botão de limpar, que **não** mexe no aberto/fechado (ver
/// [`Autocomplete::clear`]): esvaziar digitando fecha, esvaziar pelo botão não.
fn open_after_input(query: &str) -> bool {
    !query.trim().is_empty()
}

/// O que a tecla `Enter` faz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnterAction {
    /// Escolhe a sugestão destacada (índice em `rows`) e consome a tecla.
    Choose(usize),
    /// Fecha a lista **sem** consumir a tecla — é o que deixa o `Enter` chegar no formulário.
    Close,
    /// Não mexe em nada; a tecla segue seu caminho.
    Pass,
}

/// O `Enter` do `ComboboxInput`:
///
/// ```text
/// if (event.key === 'Enter' && open) {
///   if (activeIndex === null) { … setOpen(false, …); return; }  // deixa o form submeter
///   stopEvent(event); clickHighlightedItem(store, activeIndex, nativeEvent);
/// }
/// ```
///
/// Com a lista fechada o `Enter` não é tocado (o `&& open` do guard) — é o que preserva o submit
/// nativo.
fn enter_action(open: bool, highlighted: Option<usize>) -> EnterAction {
    if !open {
        return EnterAction::Pass;
    }
    match highlighted {
        Some(i) => EnterAction::Choose(i),
        None => EnterAction::Close,
    }
}

/// O que a tecla `Escape` faz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EscapeAction {
    /// Fecha a lista.
    Close,
    /// Apaga o texto.
    Clear,
    /// Não mexe em nada.
    Pass,
}

/// O `Escape` é de **dois estágios**, e os dois nunca acontecem na mesma tecla.
///
/// Com a lista montada quem trata é o `useDismiss`, que só fecha. O handler do próprio input é
/// explicitamente guardado por `!mounted`:
///
/// ```text
/// if (!mounted && event.key === 'Escape') { … store.state.setInputValue('', details); … }
/// ```
///
/// Ou seja: **1º Escape fecha a lista, 2º Escape apaga o texto**. Com a lista fechada e o campo já
/// vazio não há nada a fazer e a tecla segue (a referência chama `setInputValue('')` mesmo assim, o
/// que é um no-op observável).
fn escape_action(open: bool, empty: bool) -> EscapeAction {
    if open {
        EscapeAction::Close
    } else if !empty {
        EscapeAction::Clear
    } else {
        EscapeAction::Pass
    }
}

/// Se o botão de limpar está **montado**.
///
/// No `ComboboxClear`, com `selectionMode === 'none'` (que é o que o Autocomplete fixa) a
/// visibilidade é `inputValue !== ''`, e o `keepMounted` default é `false` — ou seja o botão é
/// **desmontado**, não apenas escondido, enquanto o campo está vazio.
fn clear_visible(query: &str) -> bool {
    !query.is_empty()
}

/// Qual botão ocupa o sufixo do campo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SuffixButton {
    /// Nenhum: o campo é só texto.
    None,
    /// O `AutocompleteTrigger` (chevron) — abre/fecha a lista.
    Trigger,
    /// O `AutocompleteClear` (✕) — apaga o texto.
    Clear,
}

/// Quem ganha o sufixo quando os dois botões estão ligados.
///
/// A referência resolve isso no CSS do gatilho: `has-[+[data-slot=autocomplete-clear]]:hidden` — o
/// gatilho **se esconde** quando é seguido por um botão de limpar. Como o de limpar é desmontado com
/// o campo vazio (ver [`clear_visible`]), o efeito combinado é: campo vazio mostra o chevron, campo
/// com texto mostra o ✕. Um só, no mesmo lugar.
fn suffix_button(show_trigger: bool, show_clear: bool, query: &str) -> SuffixButton {
    if show_clear && clear_visible(query) {
        SuffixButton::Clear
    } else if show_trigger {
        SuffixButton::Trigger
    } else {
        SuffixButton::None
    }
}

/// O teto de altura do popup: o **menor** entre o espaço livre em volta do campo e o
/// [`POPUP_MAX_HEIGHT`] — é o `max-h-[min(var(--available-height),23rem)]` da referência, literal.
///
/// O primeiro termo vem de [`crate::menu::popup_max_height`] em vez de ser recontado aqui: é a mesma
/// pergunta ("quanto sobra em volta deste retângulo, descontadas a margem da janela e a folga até o
/// gatilho?"), e duas cópias dela divergiriam na primeira correção.
fn max_height(side: MenuSide, anchor: crate::menu::Rect, viewport_height: f32) -> f32 {
    popup_max_height(side, anchor, viewport_height, SIDE_OFFSET).min(POPUP_MAX_HEIGHT)
}

/// A largura máxima do popup — o `max-w-(--available-width)` da referência.
///
/// Sem teto, um rótulo comprido faria o popup mais largo que a janela: o
/// `snap_to_window_with_margin` o **desliza** pra dentro, mas não o encolhe, e o excesso ficaria
/// cortado fora da tela. Com o teto, o `flex_1 min_w(0) overflow_hidden` do rótulo trunca dentro do
/// popup, que é o comportamento do original.
///
/// É a janela toda menos as duas margens, em **qualquer** lado — e não o espaço do lado pedido, como
/// faz o teto de ALTURA. Nos lados verticais (o caso normal) o popup desliza livremente na
/// horizontal, então o espaço é mesmo a janela inteira; nos lados horizontais isto é mais generoso
/// que o disponível, e o preço é o popup poder deslizar por cima do campo — aceitável num `side` que
/// um autocomplete praticamente não usa, e o oposto (apertar) daria um popup de 20px num campo colado
/// na borda. O piso é o mesmo [`MIN_WIDTH`], pelo mesmo motivo do piso do teto de altura.
fn max_width(viewport_width: f32) -> f32 {
    (viewport_width - 2.0 * WINDOW_MARGIN).max(MIN_WIDTH)
}

// =================================================================================================
// O componente
// =================================================================================================

/// Uma escrita de texto feita por **este** componente, cujo eco ainda não voltou.
///
/// ⚠️ **Existe porque o `InputEvent::Change` do núcleo chega DEPOIS, e não durante.** O `cx.emit` do
/// GPUI é *diferido*: quando [`Autocomplete::choose`] escreve o rótulo no [`InputState`], a nossa
/// assinatura só roda no flush de efeitos seguinte — ou seja, depois de `choose` já ter fechado a
/// lista. Sem esta marca, [`Autocomplete::on_text_changed`] aplicaria a regra de digitação sobre uma
/// escrita que não foi digitação, e os dois efeitos ficavam ao contrário:
///
/// - **escolher uma sugestão reabria a lista** (o rótulo não é vazio, então [`open_after_input`] dizia
///   "abre") — o item era escolhido e o popup voltava;
/// - **limpar fechava a lista** (o texto virou vazio), quando o `ComboboxClear` do Base UI não mexe no
///   aberto de propósito.
///
/// Os dois foram medidos pelos testes de janela, e é por isso que eles existem: nenhum teste de
/// função pura vê a ordem em que os efeitos do GPUI rodam.
///
/// O texto escrito é a **chave** do eco. Comparar (em vez de contar ecos pendentes) é o que torna
/// isto robusto quando a escrita é recusada: um `InputState` com `pattern`/`validate` pode rejeitar o
/// rótulo, e aí o `Change` nunca vem — o eco simplesmente não casa com a digitação seguinte e é
/// descartado, em vez de engolir a mudança de outra pessoa.
struct Echo {
    /// O texto que foi escrito.
    texto: SharedString,
    /// O aberto/fechado que a escrita decidiu, e que o eco tem de PRESERVAR.
    open: bool,
}

/// Campo de texto livre com lista de sugestões filtrada pelo que se digita.
///
/// É uma `Entity` (view) própria: ela guarda o aberto/fechado e o item destacado, e **emite**
/// [`AutocompleteEvent`]. O texto em si mora no [`InputState`] de quem chama — o mesmo contrato do
/// [`crate::input::Input`], que é o que permite ler/escrever o valor de fora sem passar por aqui.
pub struct Autocomplete {
    /// O estado de edição de texto, de quem chama.
    state: Entity<InputState>,
    /// As linhas da lista, na ordem em que aparecem.
    rows: Vec<AutocompleteRow>,
    /// Se o popup está aberto.
    open: bool,
    /// A linha **destacada** (`data-highlighted`), por hover OU por teclado — é um estado só, o que
    /// dá a semântica do Base UI: nunca há duas acesas. Índice em [`Self::rows`].
    highlighted: Option<usize>,
    /// Bounds do campo, medidos por `canvas` no prepaint — o "anchor rect" do `Positioner`.
    field_bounds: Bounds<Pixels>,
    /// Rolagem da lista. Persiste entre renders: sem ela, cada `cx.notify()` (um hover, por exemplo)
    /// devolveria a lista pro topo no meio da rolagem.
    list_scroll: ScrollHandle,
    /// Id estável desta instância (pra `div().id(..)` único na árvore).
    id: u64,
    /// Foco da RAIZ. O campo tem o dele (no [`InputState`]); este existe pra o popup e as teclas
    /// terem um nó de dispatch quando o campo não está focado.
    focus_handle: FocusHandle,
    side: MenuSide,
    align: MenuAlign,
    size: InputSize,
    disabled: bool,
    /// Mostrar o botão de chevron que abre/fecha (o `showTrigger` da referência, default `false`).
    show_trigger: bool,
    /// Mostrar o botão de limpar (o `showClear` da referência, default `false`).
    show_clear: bool,
    /// Ícone-prefixo dentro do campo — o `startAddon` da referência.
    start_addon: Option<SharedString>,
    /// Texto do `AutocompleteEmpty` (a linha "nada encontrado"). `None` = não monta a linha.
    empty: Option<SharedString>,
    /// Texto do `AutocompleteStatus` (a linha de estado acima da lista). `None` = não monta.
    status: Option<SharedString>,
    /// A escrita de texto que este componente fez e cujo eco ainda não voltou. Ver [`Echo`].
    echo: Option<Echo>,
}

impl Autocomplete {
    /// Cria um autocomplete sobre o [`InputState`] dado, sem sugestões.
    ///
    /// A assinatura da lista é [`Self::rows`]. O construtor **assina** o estado de texto: é a
    /// assinatura que faz digitar filtrar e abrir a lista, e sem ela o componente seria um campo
    /// comum com um popup que nunca reage.
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
            open: false,
            highlighted: None,
            field_bounds: Bounds::default(),
            list_scroll: ScrollHandle::new(),
            id: cx.entity_id().as_u64(),
            focus_handle: cx.focus_handle(),
            side: DEFAULT_SIDE,
            align: DEFAULT_ALIGN,
            size: InputSize::default(),
            disabled: false,
            show_trigger: false,
            show_clear: false,
            start_addon: None,
            empty: None,
            status: None,
            echo: None,
        }
    }

    /// As linhas da lista (sugestões, rótulos de grupo e separadores).
    pub fn rows(mut self, rows: Vec<AutocompleteRow>) -> Self {
        self.rows = rows;
        self
    }

    /// Atalho pra uma lista só de sugestões, sem grupo nem separador.
    pub fn suggestions<S: Into<SharedString>>(self, labels: impl IntoIterator<Item = S>) -> Self {
        self.rows(labels.into_iter().map(AutocompleteRow::item).collect())
    }

    /// De que lado do campo o popup abre. Default [`DEFAULT_SIDE`].
    pub fn side(mut self, side: MenuSide) -> Self {
        self.side = side;
        self
    }

    /// Como o popup se alinha ao campo. Default [`DEFAULT_ALIGN`].
    pub fn align(mut self, align: MenuAlign) -> Self {
        self.align = align;
        self
    }

    /// Tamanho do campo — o `size` da referência.
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Desabilita: o campo não edita, a lista não abre e o conjunto esmaece.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Liga o botão de chevron que abre/fecha a lista — o `showTrigger`.
    pub fn show_trigger(mut self, show: bool) -> Self {
        self.show_trigger = show;
        self
    }

    /// Liga o botão de limpar — o `showClear`. Ele só aparece com o campo preenchido (ver
    /// [`clear_visible`]).
    pub fn show_clear(mut self, show: bool) -> Self {
        self.show_clear = show;
        self
    }

    /// Ícone-prefixo dentro do campo (path svg) — o `startAddon`.
    pub fn start_addon(mut self, path: impl Into<SharedString>) -> Self {
        self.start_addon = Some(path.into());
        self
    }

    /// Texto do `AutocompleteEmpty`, montado quando o filtro não deixou nenhuma sugestão.
    pub fn empty(mut self, text: impl Into<SharedString>) -> Self {
        self.empty = Some(text.into());
        self
    }

    /// Texto do `AutocompleteStatus`, uma linha de estado acima da lista ("Buscando…", "12
    /// resultados").
    pub fn status(mut self, text: impl Into<SharedString>) -> Self {
        self.status = Some(text.into());
        self
    }

    /// O texto atual — **o valor do componente**.
    pub fn value(&self, cx: &App) -> SharedString {
        self.state.read(cx).value()
    }

    /// Se o popup está aberto.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// A linha destacada (índice em [`Self::rows`]), se alguma.
    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    /// As linhas visíveis com o texto atual — a lista depois do filtro.
    pub fn visible(&self, cx: &App) -> Vec<usize> {
        visible_rows(&self.rows, &self.value(cx))
    }

    /// Troca as linhas de fora. Re-filtra no render seguinte e **larga o destaque**, porque um índice
    /// de uma lista que não existe mais não aponta pra nada.
    pub fn set_rows(&mut self, rows: Vec<AutocompleteRow>, cx: &mut Context<Self>) {
        self.rows = rows;
        self.highlighted = None;
        cx.notify();
    }

    /// Abre ou fecha, por código. Desabilitado não abre.
    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        let open = open && !self.disabled;
        if self.open == open {
            return;
        }
        self.open = open;
        // Fechar larga o destaque: reabrir tem que começar sem nada aceso (ver [`step_highlight`]).
        if !open {
            self.highlighted = None;
        }
        cx.notify();
    }

    /// Reação a uma mudança de texto — o `InputEvent::Change` do núcleo, para **qualquer** origem.
    ///
    /// Em todo caminho ela **larga o destaque**: é o `if (!autoHighlightEnabled) { clearHighlight();
    /// }` do `maybeOpenOnInput`, e com `autoHighlight` no default `false` ele vale sempre. Largar aqui
    /// é também o que impede um índice velho de sobreviver a um filtro que encurtou a lista.
    ///
    /// Depois disso os dois caminhos se separam pelo [`Echo`]:
    ///
    /// - **eco de uma escrita nossa** ([`Self::choose`]/[`Self::clear`]): elas já decidiram o aberto e
    ///   já emitiram o `Change`. Aqui só se **preserva** aquela decisão, que a regra de digitação
    ///   desfaria — ver o doc de [`Echo`] pros dois defeitos que isso causava.
    /// - **digitação**: abre/fecha por [`open_after_input`] e emite o `Change`.
    fn on_text_changed(&mut self, texto: SharedString, cx: &mut Context<Self>) {
        self.highlighted = None;
        if let Some(eco) = self.echo.take().filter(|e| e.texto == texto) {
            self.open = eco.open && !self.disabled;
            cx.notify();
            return;
        }
        self.open = !self.disabled && open_after_input(&texto);
        cx.emit(AutocompleteEvent::Change(texto));
        cx.notify();
    }

    /// Destaca uma linha, sem repetir `notify` à toa.
    fn highlight(&mut self, row: Option<usize>, cx: &mut Context<Self>) {
        if self.highlighted == row {
            return;
        }
        self.highlighted = row;
        cx.notify();
    }

    /// Escolhe uma sugestão: o rótulo dela vai pro campo, a lista fecha, o destaque sai.
    ///
    /// Pôr o rótulo no campo é o `fillInputOnItemPress` que o `AutocompleteRoot` fixa — no Combobox
    /// isso é opcional, aqui é o que o componente É (a sugestão *completa a digitação*).
    ///
    /// ⚠️ Escrever no [`InputState`] dispara o `InputEvent::Change` do núcleo, que volta pra
    /// [`Self::on_text_changed`] **depois** desta função inteira (o `cx.emit` do GPUI é diferido) e
    /// reabriria a lista. É pra isso que serve o [`Echo`] — leia o doc dele antes de mexer nesta
    /// ordem.
    fn choose(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(label) = self.rows.get(row).and_then(AutocompleteRow::label).cloned() else {
            return;
        };
        self.open = false;
        self.highlighted = None;
        self.echo = Some(Echo {
            texto: label.clone(),
            open: false,
        });
        self.state.update(cx, |estado, cx| {
            estado.set_value(label.clone(), window, cx);
        });
        cx.emit(AutocompleteEvent::Select(row));
        cx.emit(AutocompleteEvent::Change(label));
        cx.notify();
    }

    /// O botão de limpar: apaga o texto, larga o destaque, devolve o foco ao campo — e **não** mexe
    /// no aberto/fechado.
    ///
    /// O `ComboboxClear` não chama `setOpen` em lugar nenhum; ele faz `setInputValue('')`,
    /// `setIndices({ activeIndex: null })` e `inputRef.current?.focus()`. Ou seja: com a lista aberta
    /// ela **continua** aberta, agora mostrando tudo (busca vazia casa com tudo). É o assimétrico com
    /// esvaziar digitando, que fecha — ver [`open_after_input`].
    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // O aberto NÃO muda; o [`Echo`] é o que impede a regra de digitação de fechar quando o eco do
        // texto vazio voltar (esvaziar DIGITANDO fecha, esvaziar por aqui não).
        self.echo = Some(Echo {
            texto: SharedString::default(),
            open: self.open,
        });
        self.highlighted = None;
        self.state.update(cx, |estado, cx| {
            estado.set_value("", window, cx);
        });
        self.state.read(cx).focus_handle(cx).focus(window);
        cx.emit(AutocompleteEvent::Change(SharedString::default()));
        cx.notify();
    }

    /// O `AutocompleteTrigger`: **alterna** o aberto.
    ///
    /// `useClick(…, { event: 'mousedown' })` com o `toggle` no default `true` — "a repeated click
    /// toggles the popup closed". É diferente do clique no próprio campo, que usa `toggle: false` e
    /// (com `openOnInputClick: false`, o default do Autocomplete) não faz nada.
    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let alvo = !self.open;
        self.set_open(alvo, cx);
        // O `onMouseDown` do gatilho também foca o input, pra o teclado continuar no texto.
        self.state.read(cx).focus_handle(cx).focus(window);
    }

    /// Anda com as setas. Devolve `true` se consumiu a tecla.
    ///
    /// Com a lista **fechada** a seta só abre (`openOnArrowKeyDown` do `useListNavigation`, default
    /// `true`) e **não** destaca nada: o `focusItemOnOpen` do `AriaCombobox` é
    /// `queryChangedAfterOpen || (selectionMode === 'none' && !autoHighlightMode) ? false : 'auto'`,
    /// e o Autocomplete fixa `selectionMode="none"`. Então a 1ª seta abre e a 2ª acende o 1º item.
    fn arrow(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.disabled {
            return false;
        }
        crate::focus_ring::keyboard_used(window);
        if !self.open {
            self.set_open(true, cx);
            return true;
        }
        let visible = self.visible(cx);
        let nav = navigable_rows(&self.rows, &visible);
        let destino = step_highlight(self.highlighted, &nav, forward);
        // A rolagem conta os FILHOS DIRETOS da lista, que são as linhas visíveis — então o índice
        // que interessa é a posição dentro de `visible`, não o índice da linha.
        if let Some(row) = destino {
            if let Some(pos) = visible.iter().position(|&v| v == row) {
                self.list_scroll.scroll_to_item(pos);
            }
        }
        self.highlight(destino, cx);
        true
    }

    /// O popup — em `deferred(anchored(..))` pra ficar por cima de tudo e não estourar a janela.
    fn render_popup(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette();
        let anchor = rect_of(self.field_bounds);
        let max_h = max_height(self.side, anchor, f32::from(window.viewport_size().height));
        let (x, y, corner) = anchor_point(anchor, self.side, self.align, SIDE_OFFSET, 0.0);

        let visible = self.visible(cx);
        let tem_item = visible.iter().any(|&i| self.rows[i].is_item());
        let linhas = visible
            .iter()
            .enumerate()
            .map(|(pos, &row)| self.render_row(pos, row, cx))
            .collect::<Vec<_>>();

        // A lista rolável. `p-1` só quando há conteúdo (`not-empty:p-1`).
        let lista = div()
            .id(("autocomplete-list", self.id))
            .flex()
            .flex_col()
            .w_full()
            .when(!linhas.is_empty(), |d| d.p(px(LIST_PAD)))
            .max_h(px(max_h))
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            .children(linhas);

        // A superfície. ⚠️ NÃO tem `overflow_hidden`: recortaria o bisel, que é um filho absoluto
        // sobre a borda. Quem rola é a lista de dentro.
        let mut superficie = div()
            .relative()
            // `occlude`: captura o mouse, não vaza clique pro conteúdo atrás.
            .occlude()
            .flex()
            .flex_col()
            .flex_none()
            // `min-w-(--anchor-width)`: a largura medida do campo, com piso pro 1º frame.
            .min_w(if self.field_bounds.size.width > px(0.0) {
                self.field_bounds.size.width
            } else {
                px(MIN_WIDTH)
            })
            // `max-w-(--available-width)`: sem teto, um rótulo comprido estouraria a janela.
            .max_w(px(max_width(f32::from(window.viewport_size().width))))
            .bg(p.popover_bg.hsla())
            .border(px(BORDER))
            .border_color(p.border.hsla())
            .rounded(px(RADIUS))
            // Sombra EXTERNA atrás de um fundo OPACO: aqui a armadilha do `paint_shadows` (que não
            // recorta a sombra pra fora do elemento que a projeta) não morde — o retângulo cheio
            // fica escondido pelo `--popover`.
            .shadow(popup_shadow());

        if let Some(texto) = self.status.clone() {
            superficie = superficie.child(status_row(texto));
        }
        superficie = superficie.child(lista);
        if !tem_item {
            if let Some(texto) = self.empty.clone() {
                superficie = superficie.child(empty_row(texto));
            }
        }

        let superficie = superficie.child(bevel_overlay()).on_mouse_down_out(cx.listener(
            |this, e: &MouseDownEvent, _window, cx| {
                // ⚠️ Um clique no PRÓPRIO campo também é "fora do popup". Fechar aqui, na fase de
                // captura, e deixar o gatilho alternar na de bolha REABRIRIA o popup — é o defeito
                // que o `select.rs` documenta. Quem trata o clique no campo é o campo.
                if this.field_bounds.contains(&e.position) {
                    return;
                }
                this.set_open(false, cx);
            },
        ));

        // O wrap existe pra o `flex_none` não brigar com o container de centralização, e é onde o
        // `align: center` entra — ele não é um canto, sai por layout (ver `crate::menu`).
        let wrap = div().relative().flex().flex_none().child(superficie);
        let child: AnyElement = if self.align == MenuAlign::Center {
            align_center_wrap(self.side, anchor, wrap.into_any_element()).into_any_element()
        } else {
            wrap.into_any_element()
        };

        deferred(
            anchored()
                .position(point(px(x), px(y)))
                // O canto do POPUP que encosta no ponto — é ele que expressa `align: start`/`end` e
                // `side: top`/`left` sem precisar do tamanho do popup.
                .anchor(corner)
                // O `collisionPadding` do `Positioner`: gruda na janela em vez de trocar de lado.
                .snap_to_window_with_margin(px(WINDOW_MARGIN))
                .child(child),
        )
        .with_priority(POPUP_PRIORITY)
    }

    /// Uma linha da lista. `pos` é a posição entre as linhas VISÍVEIS (o id do filho, e o índice que
    /// a rolagem entende); `row` é o índice em [`Self::rows`].
    fn render_row(&self, pos: usize, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let p = palette();
        match &self.rows[row].kind {
            RowKind::Separator => div()
                .flex_none()
                .h(px(SEPARATOR_HEIGHT))
                .mx(px(SEPARATOR_MARGIN_X))
                .my(px(SEPARATOR_MARGIN_Y))
                .bg(p.border.hsla())
                .into_any_element(),

            RowKind::GroupLabel(texto) => div()
                .flex_none()
                .px(px(LABEL_PAD_X))
                .py(px(LABEL_PAD_Y))
                // `[[role=group]+&]:mt-1.5`: a folga é entre grupos, então o primeiro não a leva.
                .when(pos > 0, |d| d.mt(px(GROUP_GAP)))
                .text_size(px(SMALL_TEXT_SIZE))
                .line_height(px(SMALL_LINE_HEIGHT))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted.hsla())
                .child(texto.clone())
                .into_any_element(),

            RowKind::Item { label, disabled } => {
                let disabled = *disabled;
                // Uma linha desabilitada nunca acende: `data-disabled` e `data-highlighted` não
                // convivem no Base UI, e o teclado já a pula.
                let aceso = !disabled && self.highlighted == Some(row);
                let mut el = div()
                    .id(("autocomplete-item", row))
                    .flex()
                    .items_center()
                    .w_full()
                    .min_h(px(ITEM_MIN_HEIGHT))
                    .py(px(ITEM_PAD_Y))
                    // `px-2`: simétrico. Ver [`ITEM_PAD_X`].
                    .px(px(ITEM_PAD_X))
                    .rounded(px(ITEM_RADIUS))
                    .text_size(px(TEXT_SIZE))
                    // Ver [`TEXT_LINE_HEIGHT`]: é ela que faz a linha fechar em 28px, não 30,65.
                    .line_height(px(TEXT_LINE_HEIGHT))
                    .text_color(if aceso { p.accent_text } else { p.text }.hsla())
                    .when(aceso, |d| d.bg(p.accent.hsla()))
                    // `data-disabled:opacity-64`: esmaece a linha inteira.
                    .when(disabled, |d| d.opacity(DISABLED_OPACITY))
                    // `cursor-default`: uma sugestão não é um link.
                    .when(!disabled, |d| d.cursor(CursorStyle::Arrow))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(label.clone()),
                    );

                if !disabled {
                    el = el
                        // `highlightItemOnHover` (default `true`) e `keepHighlight` (default
                        // `false`): hover acende, e sair apaga. O destaque de hover e o de teclado
                        // são o MESMO estado, então nunca há dois acesos. Ao sair, só apaga se a
                        // linha que saiu é a acesa: a ordem dos dois eventos ao passar de uma linha
                        // pra vizinha não é garantida, e sem essa guarda o par (entra 3, sai 2)
                        // apagaria o 3.
                        .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                            if *hovered {
                                this.highlight(Some(row), cx);
                            } else if this.highlighted == Some(row) {
                                this.highlight(None, cx);
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

    /// O campo, dentro do wrap que mede os bounds pra ancorar o popup.
    fn render_field(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette();
        let query = self.value(cx);
        let size = self.size;
        let disabled = self.disabled;

        let mut campo = Input::new(&self.state).size(size).disabled(disabled);
        if let Some(path) = self.start_addon.clone() {
            campo = campo.prefix(
                gpui::svg()
                    .path(path)
                    .size(px(size.icon()))
                    .flex_none()
                    // `opacity-80` do addon. Vai no ALFA da cor (via [`Rgba8::scaled`]) e não num
                    // `.opacity(..)`, que pediria um passe de pintura só pra um glifo.
                    .text_color(p.text.scaled(ADDON_OPACITY)),
            );
        }
        campo = match suffix_button(self.show_trigger, self.show_clear, &query) {
            SuffixButton::None => campo,
            SuffixButton::Trigger => campo.suffix(Input::icon_button(
                ("autocomplete-trigger", self.id),
                TRIGGER_ICON,
                size,
                disabled,
                cx.listener(|this, _e, window, cx| {
                    crate::focus_ring::pointer_used(window);
                    this.toggle(window, cx);
                }),
            )),
            SuffixButton::Clear => campo.suffix(Input::icon_button(
                ("autocomplete-clear", self.id),
                CLEAR_ICON,
                size,
                disabled,
                cx.listener(|this, _e, window, cx| {
                    crate::focus_ring::pointer_used(window);
                    this.clear(window, cx);
                }),
            )),
        };

        div()
            .relative()
            .w_full()
            .child(campo)
            .child(
                canvas(
                    {
                        let view = cx.entity();
                        move |bounds, _window, cx| {
                            view.update(cx, |this, _| this.field_bounds = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
    }
}

impl EventEmitter<AutocompleteEvent> for Autocomplete {}

impl Focusable for Autocomplete {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Autocomplete {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.open;

        // ⚠️ **Por que AÇÕES, e por que na fase de CAPTURA.**
        //
        // *Ações, e não `on_key_down`*: o `InputState` do núcleo empilha o contexto de teclas `Input`
        // e o `gpui_component::input::init` vincula `up`→`MoveUp`, `down`→`MoveDown`,
        // `enter`→`Enter`, `escape`→`Escape` nele. No GPUI as ações são despachadas **antes** dos
        // ouvintes de tecla (`Window::dispatch_key_event` roda o laço de `dispatch_action_on_node` e
        // só depois o `finish_dispatch_key_event`), então com um campo focado estas quatro teclas
        // chegam aqui como AÇÃO. Um `on_key_down` na raiz só as veria se nenhum ouvinte de ação as
        // consumisse — depender disso seria depender do que o núcleo faz por dentro.
        //
        // *Captura, e não bolha*: na fase de bolha a ordem é do nó FOCADO pra raiz e uma ação **para
        // a propagação por default** (o comentário é literal no `dispatch_action_on_node`: "Actions
        // stop propagation by default during the bubble phase") — ou seja o campo decidiria primeiro.
        // Pra `Enter` e `Escape` isso importa de verdade, porque o campo tem ouvinte pras duas SEMPRE
        // (`input/input.rs:305-306`) e os handlers dele agem **antes** do `cx.propagate()`:
        // `InputState::escape` limpa a completação inline (e a consome) e, com `clean_on_escape`,
        // **apaga o texto**; `InputState::enter` emite `InputEvent::PressEnter`. Com a lista aberta é
        // este componente que manda na tecla, e esperar a bolha significaria o campo já ter agido.
        //
        // ⚠️ Pras SETAS isto é defensivo, não load-bearing, e a diferença é declarada de propósito:
        // o campo só registra ouvinte de `MoveUp`/`MoveDown` em modo **multi-linha**
        // (`input/input.rs:325`, `.when(state.mode.is_multi_line(), …)`), então num `InputState` de
        // uma linha — o caso deste componente — ninguém as consome e a bolha funcionaria igual. As
        // setas usam `capture_action` por uniformidade com as outras duas e pra o componente
        // continuar certo se lhe passarem um estado multi-linha. **Não é observável** num teste com
        // estado de uma linha: a mutação `capture_action` → `on_action` nas setas SOBREVIVE, e é por
        // isso que ela não virou teste em vez de virar uma afirmação que o código não sustenta.
        //
        // Consumir só quando de fato se usa a tecla é o resto do contrato: com a lista fechada, ou
        // desabilitado, ou sem destaque no `Enter`, a ação segue pro campo — como deve.
        let mut root = div()
            .id(("autocomplete", self.id))
            .track_focus(&self.focus_handle)
            .relative()
            .w_full()
            .capture_action(cx.listener(|this, _: &MoveDown, window, cx| {
                if this.arrow(true, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveUp, window, cx| {
                if this.arrow(false, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &Enter, window, cx| {
                match enter_action(this.open, this.highlighted) {
                    EnterAction::Choose(row) => {
                        crate::focus_ring::keyboard_used(window);
                        this.choose(row, window, cx);
                        cx.stop_propagation();
                    }
                    // Fecha e **deixa passar**: é o que permite ao `Enter` submeter o formulário.
                    EnterAction::Close => this.set_open(false, cx),
                    EnterAction::Pass => {}
                }
            }))
            .capture_action(cx.listener(|this, _: &Escape, window, cx| {
                let vazio = this.value(cx).is_empty();
                match escape_action(this.open, vazio) {
                    EscapeAction::Close => {
                        crate::focus_ring::keyboard_used(window);
                        this.set_open(false, cx);
                        cx.stop_propagation();
                    }
                    EscapeAction::Clear => {
                        crate::focus_ring::keyboard_used(window);
                        this.clear(window, cx);
                        cx.stop_propagation();
                    }
                    EscapeAction::Pass => {}
                }
            }))
            .child(self.render_field(cx));

        if open {
            root = root.child(self.render_popup(window, cx));
        }
        root
    }
}

// =================================================================================================
// Pedaços de desenho (funções livres — testáveis sem construir um `Autocomplete`)
// =================================================================================================

/// A linha do `AutocompleteEmpty`: `not-empty:p-2 text-center text-sm text-muted-foreground`.
fn empty_row(texto: SharedString) -> Div {
    let p = palette();
    div()
        .flex_none()
        .w_full()
        .p(px(EMPTY_PAD))
        .text_size(px(TEXT_SIZE))
        .line_height(px(TEXT_LINE_HEIGHT))
        .text_center()
        .text_color(p.muted.hsla())
        .child(texto)
}

/// A linha do `AutocompleteStatus`: `px-3 py-2 font-medium text-xs text-muted-foreground`.
fn status_row(texto: SharedString) -> Div {
    let p = palette();
    div()
        .flex_none()
        .w_full()
        .px(px(STATUS_PAD_X))
        .py(px(STATUS_PAD_Y))
        .text_size(px(SMALL_TEXT_SIZE))
        .line_height(px(SMALL_LINE_HEIGHT))
        .font_weight(FontWeight::MEDIUM)
        .text_color(p.muted.hsla())
        .child(texto)
}

/// As duas camadas do `shadow-lg/5` do popup.
///
/// Sombra EXTERNA atrás de um fundo OPACO: é o único arranjo em que a armadilha do
/// [`gpui::Window::paint_shadows`] (que insere a sombra como um retângulo arredondado CHEIO, sem
/// recortar a área do próprio elemento) não aparece — o retângulo fica escondido pelo `--popover`.
fn popup_shadow() -> Vec<gpui::BoxShadow> {
    let cor = palette().shadow.hsla();
    SHADOW_LAYERS
        .iter()
        .map(|&(dy, blur, spread)| gpui::BoxShadow {
            color: cor,
            offset: point(px(0.0), px(dy)),
            blur_radius: px(blur),
            spread_radius: px(spread),
        })
        .collect()
}

/// O **fio de bisel** de 1px sobre a borda do popup.
///
/// ⚠️ **A técnica do coss não traduz pro GPUI.** Lá é um pseudo-elemento transparente com
/// `box-shadow: 0 ±1px <cor>`: só o filete que ESCAPA da caixa fica visível, porque o CSS nunca
/// pinta a sombra por baixo da border box de quem a projeta. O [`gpui::Window::paint_shadows`]
/// insere a sombra como um retângulo arredondado **completo** — num overlay transparente isso viraria
/// uma lavagem de cor sobre o popup INTEIRO. É o bug que o `input.rs` já pagou.
///
/// Então o filete é desenhado como o que ele é: uma **borda de 1px num único lado** de um overlay
/// absoluto, e o LADO sai do SINAL do deslocamento da sombra da referência (claro desce, escuro
/// sobe). O overlay cobre a **BORDER box** (daí o `inset: -1px` a partir da padding box), e por isso
/// o raio dele é o da superfície — **não** `raio − 1`, que é o que o `.tsx` diz porque lá o
/// pseudo-elemento fica por DENTRO da borda.
fn bevel_overlay() -> Div {
    let p = palette();
    let overlay = div()
        .absolute()
        .top(px(-BORDER))
        .left(px(-BORDER))
        .right(px(-BORDER))
        .bottom(px(-BORDER))
        .rounded(px(RADIUS))
        .border_color(p.bevel.hsla());
    if p.bevel_dir > 0.0 {
        overlay.border_b_1()
    } else {
        overlay.border_t_1()
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `menu.rs`, `select.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::color::Rgba8;
    use crate::theme;

    /// Uma lista com grupo, separador e item desabilitado — cobre os quatro tipos de linha.
    fn lista() -> Vec<AutocompleteRow> {
        vec![
            AutocompleteRow::group_label("Cítricas"),   // 0
            AutocompleteRow::item("Laranja"),           // 1
            AutocompleteRow::item("Limão"),             // 2
            AutocompleteRow::separator(),               // 3
            AutocompleteRow::group_label("Vermelhas"),  // 4
            AutocompleteRow::item("Morango"),           // 5
            AutocompleteRow::item("Framboesa").disabled(true), // 6
        ]
    }

    // --- Filtragem ------------------------------------------------------------------------------

    /// O casador é o `contains` do Base UI: substring em QUALQUER posição (não prefixo), insensível
    /// a caixa e a diacrítico, e busca vazia casa com tudo.
    #[test]
    fn o_filtro_e_contains_insensivel_a_caixa_e_acento() {
        // Substring no meio — se fosse `starts_with`, isto falharia.
        assert!(matches_query("Framboesa", "boes"));
        assert!(matches_query("Framboesa", "Fram"));
        assert!(!matches_query("Framboesa", "xyz"));

        // Caixa.
        assert!(matches_query("Laranja", "LARANJA"));
        assert!(matches_query("LARANJA", "laranja"));

        // Diacrítico, nos dois sentidos (`sensitivity: 'base'`).
        assert!(matches_query("Limão", "limao"));
        assert!(matches_query("Limao", "limão"));
        assert!(matches_query("Résumé", "resume"));
        assert!(matches_query("Ação", "acao"));

        // Busca vazia casa com tudo (`if (!query) return true`).
        assert!(matches_query("Laranja", ""));

        // `ignorePunctuation`: o hífen não conta de nenhum dos dois lados.
        assert!(matches_query("E-mail", "email"));
        assert!(matches_query("Email", "e-mail"));
    }

    /// A dobra é 1-para-1 e cobre Latin-1 + Latin Extended-A. Fora dessas faixas ela **não** tira
    /// diacrítico — está declarado, e este teste é o que impede alguém de afirmar o contrário.
    #[test]
    fn a_dobra_cobre_o_latim_e_para_ali() {
        for (c, esperado) in [
            ('Á', 'a'),
            ('ã', 'a'),
            ('Ç', 'c'),
            ('ê', 'e'),
            ('Ï', 'i'),
            ('ñ', 'n'),
            ('Õ', 'o'),
            ('ü', 'u'),
            ('ž', 'z'),
            ('ł', 'l'),
        ] {
            assert_eq!(fold_char(c), esperado, "dobra de {c}");
        }
        // Já dobrado: idempotente.
        assert_eq!(fold_char('a'), 'a');
        // Fora do latim: só a caixa muda (grego), e o diacrítico permanece.
        assert_eq!(fold_char('Δ'), 'δ');
        assert_ne!(fold_char('ή'), 'η');
        // A dobra de texto descarta pontuação e preserva o espaço.
        assert_eq!(fold("Olá, Mundo!"), "ola mundo");
    }

    /// **Rótulo de grupo e separador órfãos somem** — o superset declarado no doc do módulo. Sem
    /// isso, filtrar deixaria um título de grupo vazio e um filete solto na tela.
    #[test]
    fn o_filtro_poda_grupo_vazio_e_separador_orfao() {
        let rows = lista();

        // Sem busca: tudo aparece.
        assert_eq!(visible_rows(&rows, ""), vec![0, 1, 2, 3, 4, 5, 6]);

        // "mora" só casa com "Morango" (linha 5): sobra o rótulo dela (4) e mais nada — o grupo
        // "Cítricas" (0) morre com os itens dele, e o separador (3) perde o lado de antes.
        assert_eq!(visible_rows(&rows, "mora"), vec![4, 5]);

        // "lim" só casa com "Limão" (2): sobra o rótulo "Cítricas" (0). O separador (3) perde o lado
        // de DEPOIS e some — é o `last:hidden` do `.tsx` caindo fora da mesma regra.
        assert_eq!(visible_rows(&rows, "lim"), vec![0, 2]);

        // Uma busca que casa nos DOIS grupos mantém o separador, porque agora ele tem os dois lados.
        // "ran" casa com "Laranja" (1) e "Morango" (5), e com mais nada.
        assert_eq!(visible_rows(&rows, "ran"), vec![0, 1, 3, 4, 5]);

        // Uma busca que casa com tudo é igual a não ter busca.
        assert_eq!(visible_rows(&rows, "a"), visible_rows(&rows, ""));

        // Nada casa: nem rótulo, nem separador, nada.
        assert!(visible_rows(&rows, "zzz").is_empty());
    }

    /// O teclado alcança só sugestão HABILITADA: rótulo, separador e desabilitada ficam de fora.
    #[test]
    fn a_navegacao_pula_rotulo_separador_e_desabilitado() {
        let rows = lista();
        let visible = visible_rows(&rows, "");
        // Das 7 linhas visíveis, só 1, 2 e 5 são alcançáveis (6 é desabilitada).
        assert_eq!(navigable_rows(&rows, &visible), vec![1, 2, 5]);
    }

    // --- Navegação ------------------------------------------------------------------------------

    /// **O ciclo do destaque tem `len + 1` estados**, e o extra é "nenhum": passando do fim, o
    /// destaque volta pro CAMPO antes de reentrar pela outra ponta.
    ///
    /// É `loopFocus: true` + `allowEscape: true` (que é `loopFocus && !autoHighlightMode`, com
    /// `autoHighlight` no default `false`). Um `step_index` de menu — que vai do último direto pro
    /// primeiro — está errado aqui: tiraria do usuário a chance de voltar a ver o que digitou.
    #[test]
    fn o_ciclo_do_destaque_passa_por_nenhum() {
        let nav = [1usize, 2, 5];

        // Descendo: nenhum → 1 → 2 → 5 → NENHUM → 1 …
        assert_eq!(step_highlight(None, &nav, true), Some(1));
        assert_eq!(step_highlight(Some(1), &nav, true), Some(2));
        assert_eq!(step_highlight(Some(2), &nav, true), Some(5));
        assert_eq!(step_highlight(Some(5), &nav, true), None, "o fim volta pro campo");
        assert_eq!(step_highlight(None, &nav, true), Some(1), "e só então dá a volta");

        // Subindo: nenhum → 5 → 2 → 1 → NENHUM → 5 …
        assert_eq!(step_highlight(None, &nav, false), Some(5));
        assert_eq!(step_highlight(Some(5), &nav, false), Some(2));
        assert_eq!(step_highlight(Some(2), &nav, false), Some(1));
        assert_eq!(step_highlight(Some(1), &nav, false), None, "o começo volta pro campo");

        // Lista vazia: não há o que destacar, em nenhum sentido.
        assert_eq!(step_highlight(None, &[], true), None);
        assert_eq!(step_highlight(Some(3), &[], false), None);

        // Índice velho (de antes de um filtro) não trava: entra pela ponta.
        assert_eq!(step_highlight(Some(99), &nav, true), Some(1));
        assert_eq!(step_highlight(Some(99), &nav, false), Some(5));
    }

    // --- Abrir e fechar -------------------------------------------------------------------------

    /// Digitar abre; **esvaziar digitando FECHA**, porque o Autocomplete fixa
    /// `openOnInputClick: false`. Espaço em branco puro não abre — o Base UI decide pelo texto
    /// APARADO.
    #[test]
    fn digitar_abre_e_esvaziar_fecha() {
        assert!(open_after_input("l"));
        assert!(open_after_input("laranja"));
        assert!(!open_after_input(""), "vazio fecha (input-clear)");
        assert!(!open_after_input("   "), "só espaço não abre: o Base UI apara");
        assert!(open_after_input("  l  "), "texto de verdade com folga abre");
    }

    /// `Enter`: com destaque escolhe e consome; sem destaque só fecha (e **deixa passar**, pra o
    /// formulário poder submeter); com a lista fechada não é tocado.
    #[test]
    fn enter_escolhe_o_destacado_e_senao_deixa_passar() {
        assert_eq!(enter_action(true, Some(2)), EnterAction::Choose(2));
        assert_eq!(enter_action(true, None), EnterAction::Close);
        assert_eq!(enter_action(false, None), EnterAction::Pass);
        // Fechado com destaque velho também não faz nada: o guard do Base UI é `&& open`.
        assert_eq!(enter_action(false, Some(2)), EnterAction::Pass);
    }

    /// `Escape` é de **dois estágios**: a 1ª fecha a lista, a 2ª apaga o texto. Nunca as duas na
    /// mesma tecla — no Base UI o handler do input é guardado por `!mounted`.
    #[test]
    fn escape_fecha_primeiro_e_so_depois_apaga() {
        assert_eq!(escape_action(true, false), EscapeAction::Close);
        assert_eq!(escape_action(true, true), EscapeAction::Close, "aberto sempre fecha");
        assert_eq!(escape_action(false, false), EscapeAction::Clear);
        assert_eq!(escape_action(false, true), EscapeAction::Pass, "nada a fazer");
    }

    // --- Os dois botões do sufixo ---------------------------------------------------------------

    /// O botão de limpar é **desmontado** com o campo vazio (`visible = inputValue !== ''` e
    /// `keepMounted: false`), e o gatilho se esconde quando ele aparece
    /// (`has-[+[data-slot=autocomplete-clear]]:hidden`). O resultado é um botão só, no mesmo lugar:
    /// chevron enquanto vazio, ✕ com texto.
    #[test]
    fn o_limpar_substitui_o_gatilho_quando_ha_texto() {
        assert!(!clear_visible(""));
        assert!(clear_visible("l"));

        // Os dois ligados: o ✕ ganha, mas só quando existe.
        assert_eq!(suffix_button(true, true, ""), SuffixButton::Trigger);
        assert_eq!(suffix_button(true, true, "l"), SuffixButton::Clear);
        // Só o gatilho: ele fica nos dois casos.
        assert_eq!(suffix_button(true, false, "l"), SuffixButton::Trigger);
        // Só o limpar: some com o campo vazio, e aí não sobra sufixo nenhum.
        assert_eq!(suffix_button(false, true, ""), SuffixButton::None);
        assert_eq!(suffix_button(false, true, "l"), SuffixButton::Clear);
        // Nenhum dos dois (o default da referência: `showTrigger` e `showClear` são `false`).
        assert_eq!(suffix_button(false, false, "l"), SuffixButton::None);
    }

    // --- Geometria ------------------------------------------------------------------------------

    /// **A entrelinha, e o número que sairia sem fixá-la.**
    ///
    /// O default do GPUI é `relative(phi())` — a razão de ouro, 1,618034 × o corpo. O Tailwind dá
    /// `text-sm = 14/20` e `text-xs = 12/16`. Sem fixar, uma linha de 14px mediria 22,65px e um item
    /// com `py-1` fecharia em 30,65 em vez dos 28 do `sm:min-h-7` — 9,5% mais alto, sem nada no
    /// código parecendo errado.
    #[test]
    fn a_entrelinha_e_a_do_tailwind_e_nao_a_razao_de_ouro() {
        const PHI: f32 = 1.618_034;

        assert_eq!(TEXT_SIZE, 14.0, "sm:text-sm");
        assert_eq!(TEXT_LINE_HEIGHT, 20.0, "o par do Tailwind pro text-sm");
        assert_eq!(SMALL_TEXT_SIZE, 12.0, "text-xs");
        assert_eq!(SMALL_LINE_HEIGHT, 16.0, "o par do Tailwind pro text-xs");

        // O que o GPUI faria sozinho, e a diferença que isso dá na ALTURA do item.
        let sem_fixar = TEXT_SIZE * PHI;
        assert!(
            (sem_fixar - 22.652_476).abs() < 1e-3,
            "o default do GPUI pro corpo de 14px é 22,65px; veio {sem_fixar}"
        );
        assert!(sem_fixar > TEXT_LINE_HEIGHT, "a razão de ouro é MAIS alta que o par do Tailwind");
        let item_sem_fixar = sem_fixar + 2.0 * ITEM_PAD_Y;
        assert!(
            (item_sem_fixar - 30.652_476).abs() < 1e-3,
            "sem fixar, o item mediria 30,65px em vez de 28; veio {item_sem_fixar}"
        );
        assert_eq!(TEXT_LINE_HEIGHT + 2.0 * ITEM_PAD_Y, ITEM_MIN_HEIGHT, "fixada, fecha em 28");

        // O mesmo pro texto pequeno.
        let pequeno_sem_fixar = SMALL_TEXT_SIZE * PHI;
        assert!(
            (pequeno_sem_fixar - 19.416_408).abs() < 1e-3,
            "o default pro corpo de 12px é 19,42px; veio {pequeno_sem_fixar}"
        );
    }

    /// **A geometria do item é a do [`crate::menu`], não a do [`crate::select`].**
    ///
    /// O `AutocompleteItem` é `flex … px-2` — respiro SIMÉTRICO, sem coluna de check. O do Combobox
    /// (e o do nosso `Select`) é `grid grid-cols-[1rem_1fr] gap-2 ps-2 pe-4`, com 8 no início e 16 no
    /// fim. Copiar aquele pra cá deslocaria todo rótulo 8px pra esquerda do centro da linha, e é o
    /// erro fácil porque os dois `.tsx` são quase o mesmo arquivo.
    #[test]
    fn o_item_tem_respiro_simetrico_e_nenhuma_coluna_de_check() {
        assert_eq!(ITEM_PAD_X, 8.0, "px-2, os DOIS lados");
        assert_eq!(ITEM_PAD_Y, 4.0, "py-1");
        assert_eq!(ITEM_MIN_HEIGHT, 28.0, "sm:min-h-7");
        assert_eq!(ITEM_RADIUS, 6.0, "rounded-sm = --radius-sm = --radius - 4");
        assert_eq!(LIST_PAD, 4.0, "p-1 na lista");
        assert!(ITEM_RADIUS < RADIUS, "o item arredonda MENOS que o popup");
        // O que existiria se a coluna do check existisse — e não existe: nenhuma linha deste módulo
        // tem 16px de respiro no fim.
        assert_ne!(ITEM_PAD_X, 16.0, "pe-4 é do Combobox, não daqui");
    }

    /// Os números dos outros slots, todos direto do `.tsx`.
    #[test]
    fn geometria_dos_outros_slots() {
        assert_eq!(RADIUS, 10.0, "rounded-lg = --radius-lg = --radius");
        assert_eq!(BORDER, 1.0, "border");
        assert_eq!((LABEL_PAD_X, LABEL_PAD_Y), (8.0, 6.0), "px-2 py-1.5 no rótulo de grupo");
        assert_eq!(GROUP_GAP, 6.0, "[[role=group]+&]:mt-1.5");
        assert_eq!(
            (SEPARATOR_MARGIN_X, SEPARATOR_MARGIN_Y, SEPARATOR_HEIGHT),
            (8.0, 4.0, 1.0),
            "mx-2 my-1 h-px"
        );
        assert_eq!(EMPTY_PAD, 8.0, "not-empty:p-2");
        assert_eq!((STATUS_PAD_X, STATUS_PAD_Y), (12.0, 8.0), "px-3 py-2");
        assert_eq!(DISABLED_OPACITY, 0.64, "data-disabled:opacity-64");
        // **O `px-3` do Status não é um número solto**: ele é irmão da lista (fora do `p-1` dela), e
        // 12 = 4 do `p-1` + 8 do `px-2` do item. Ou seja o texto de estado nasce na MESMA coluna que
        // o rótulo das sugestões. Se alguém "arredondar" o Status pra `px-2`, isto falha e diz por quê.
        assert_eq!(
            STATUS_PAD_X,
            LIST_PAD + ITEM_PAD_X,
            "o Status alinha com o texto dos itens"
        );
    }

    /// **`side`/`align` default são os da referência**, e o de `align` NÃO é o do
    /// [`MenuAlign::default()`] — que é `Center`, o default do *menu*. Um popup de autocomplete
    /// centralizado no campo é errado: ele encosta na borda inicial, como um `<select>`.
    #[test]
    fn defaults_de_lado_e_alinhamento() {
        assert_eq!(DEFAULT_SIDE, MenuSide::Bottom, "side=\"bottom\"");
        assert_eq!(DEFAULT_ALIGN, MenuAlign::Start, "align=\"start\"");
        assert_ne!(
            DEFAULT_ALIGN,
            MenuAlign::default(),
            "o default do MENU é Center; herdá-lo aqui pegaria o valor errado em silêncio"
        );
        assert_eq!(SIDE_OFFSET, 4.0, "o sideOffset da referência é 4");
    }

    /// **O teto do popup é `min(espaço livre, 23rem)`** — os dois termos, e o `min` entre eles.
    #[test]
    fn o_teto_do_popup_e_o_menor_entre_o_espaco_livre_e_23rem() {
        assert_eq!(POPUP_MAX_HEIGHT, 368.0, "23rem");

        // Janela alta com o campo no topo: sobra muito mais que 368, então o teto é o 23rem.
        let campo = crate::menu::Rect {
            x: 0.0,
            y: 20.0,
            w: 200.0,
            h: 32.0,
        };
        assert_eq!(max_height(MenuSide::Bottom, campo, 1200.0), POPUP_MAX_HEIGHT);

        // Janela curta: agora quem manda é o espaço livre, e ele é MENOR que o 23rem.
        let curto = max_height(MenuSide::Bottom, campo, 300.0);
        assert!(curto < POPUP_MAX_HEIGHT, "num espaço apertado o teto encolhe; veio {curto}");
        // E é exatamente o mesmo número que o popup do menu usaria — a conta é importada, não
        // recopiada.
        assert_eq!(
            curto,
            popup_max_height(MenuSide::Bottom, campo, 300.0, SIDE_OFFSET)
        );

        // Janela minúscula: o piso do `crate::menu` (96px) segura, e o `min` não o afunda.
        let piso = max_height(MenuSide::Bottom, campo, 40.0);
        assert!(piso >= 96.0, "o piso de altura do menu vale aqui também; veio {piso}");
    }

    /// O piso de largura existe só pro PRIMEIRO frame — antes do prepaint os bounds do campo são
    /// zero, e um popup de largura zero é invisível.
    #[test]
    fn o_popup_tem_piso_de_largura_pro_primeiro_frame() {
        assert_eq!(MIN_WIDTH, 144.0, "min-w-36");
        // Tem que caber um rótulo de verdade com o respiro dos dois lados da lista e do item.
        assert!(MIN_WIDTH > 2.0 * (BORDER + LIST_PAD + ITEM_PAD_X));
    }

    /// **A largura máxima é a janela menos as duas margens** — o `max-w-(--available-width)`. Sem ela
    /// um rótulo comprido faria o popup mais largo que a tela: o `snap_to_window_with_margin` desliza
    /// o popup pra dentro, mas **não** o encolhe, e o excesso ficaria fora da vista.
    #[test]
    fn a_largura_maxima_e_a_janela_menos_as_margens() {
        assert_eq!(max_width(1000.0), 1000.0 - 2.0 * WINDOW_MARGIN);
        // Janela estreita: o piso segura, mesmo que estoure a margem — um popup de 20px é ilegível.
        assert_eq!(max_width(60.0), MIN_WIDTH);
        // E o teto é sempre MENOR que a janela: se fosse ≥, ele não seria teto de nada.
        assert!(max_width(1000.0) < 1000.0);
    }

    /// O ícone-prefixo do campo entra a **80%** (`opacity-80` do `autocomplete-start-addon`), e pelo
    /// ALFA da cor — não por um passe de opacidade só pra um glifo.
    #[test]
    fn o_icone_prefixo_entra_a_oitenta_por_cento() {
        assert_eq!(ADDON_OPACITY, 0.8, "opacity-80");
        theme::set_theme(theme::ThemeMode::Dark);
        let cheio = palette().text.hsla().a;
        let apagado = palette().text.scaled(ADDON_OPACITY).a;
        assert!(
            (apagado - cheio * ADDON_OPACITY).abs() < 1e-6,
            "o /80 MULTIPLICA o alfa que já existe (Tailwind v4); veio {apagado}"
        );
        assert!(apagado < cheio, "e o resultado é mais apagado que o texto");
    }

    // --- Paleta ---------------------------------------------------------------------------------

    /// **A convenção de cor da paleta, decodificada de verdade.**
    ///
    /// Todo valor de `crate::menu::MenuPalette` é `0xRRGGBBAA` e é consumido por `rgba`; um valor de 6
    /// dígitos esquecido ali vira uma cor completamente diferente, **sem erro de compilação** —
    /// `rgba(0xffffff)` é lido como `0x00FFFFFF`, ou seja ciano. Já aconteceu três vezes nesta base.
    /// Em vez de comparar números com números (que não pegaria nada), este teste decodifica e afirma
    /// o que a cor DEVE ser perceptualmente.
    #[test]
    fn a_paleta_decodifica_pras_cores_pretendidas() {
        // O fundo do popup é OPACO nos dois temas — é o que torna a sombra externa segura.
        for (nome, c) in [
            ("popup claro", crate::menu::MENU_LIGHT.popover_bg),
            ("popup escuro", crate::menu::MENU_DARK.popover_bg),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome} tem que ser opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome} é NEUTRO: se r≠g≠b, o valor foi lido deslocado"
            );
        }
        let claro: gpui::Rgba = crate::menu::MENU_LIGHT.popover_bg.hsla().into();
        let escuro: gpui::Rgba = crate::menu::MENU_DARK.popover_bg.hsla().into();
        assert!(claro.r > 0.9, "o popup do tema claro é claro");
        assert!(escuro.r < 0.2, "o popup do tema escuro é escuro");
        // `--background` do tema escuro é #141414 (20/255): o popup (#1d1d1d) LEVANTA sobre ele. Se
        // o `--popover` fosse copiado do `--background`, o popup sumiria contra o painel.
        assert!(
            escuro.r > 20.0 / 255.0,
            "o popup escuro é mais claro que o --background (#141414)"
        );

        // Texto e texto apagado: neutros, opacos, e o apagado mais perto do fundo que o normal.
        for (nome, c) in [
            ("texto claro", crate::menu::MENU_LIGHT.text),
            ("texto escuro", crate::menu::MENU_DARK.text),
            ("muted claro", crate::menu::MENU_LIGHT.muted),
            ("muted escuro", crate::menu::MENU_DARK.muted),
            ("accent-fg claro", crate::menu::MENU_LIGHT.accent_text),
            ("accent-fg escuro", crate::menu::MENU_DARK.accent_text),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome} é NEUTRO: se r≠g≠b, o valor foi lido deslocado"
            );
        }
        let t_claro: gpui::Rgba = crate::menu::MENU_LIGHT.text.hsla().into();
        let m_claro: gpui::Rgba = crate::menu::MENU_LIGHT.muted.hsla().into();
        assert!(t_claro.r < 0.2, "o texto do claro é quase preto");
        assert!(m_claro.r > t_claro.r, "no claro o muted é MAIS claro que o texto");
        let t_escuro: gpui::Rgba = crate::menu::MENU_DARK.text.hsla().into();
        let m_escuro: gpui::Rgba = crate::menu::MENU_DARK.muted.hsla().into();
        assert!(t_escuro.r > 0.8, "o texto do escuro é quase branco");
        assert!(m_escuro.r < t_escuro.r, "no escuro o muted é MAIS escuro que o texto");

        // Os tokens que DEVEM ser translúcidos continuam translúcidos — é isso que os faz funcionar
        // sobre qualquer fundo.
        for (nome, c) in [
            ("borda claro", crate::menu::MENU_LIGHT.border),
            ("borda escuro", crate::menu::MENU_DARK.border),
            ("accent claro", crate::menu::MENU_LIGHT.accent),
            ("accent escuro", crate::menu::MENU_DARK.accent),
            ("bisel claro", crate::menu::MENU_LIGHT.bevel),
            ("bisel escuro", crate::menu::MENU_DARK.bevel),
            ("sombra claro", crate::menu::MENU_LIGHT.shadow),
            ("sombra escuro", crate::menu::MENU_DARK.shadow),
        ] {
            assert!(
                c.alpha() < 1.0,
                "{nome} tem que ser translúcido, veio com alfa {}",
                c.alpha()
            );
        }

        // O destaque de linha é MAIS sutil que a borda do popup (`--accent` 4% vs `--border` 8%/6%):
        // ele é um realce de fundo, não uma moldura.
        assert!(crate::menu::MENU_LIGHT.accent.alpha() < crate::menu::MENU_LIGHT.border.alpha());
        assert!(crate::menu::MENU_DARK.accent.alpha() < crate::menu::MENU_DARK.border.alpha());

        // O bisel troca de SENTIDO entre os temas: claro desce, escuro sobe.
        assert!(crate::menu::MENU_LIGHT.bevel_dir > 0.0);
        assert!(crate::menu::MENU_DARK.bevel_dir < 0.0);
    }

    /// **A borda do popup é `--border`, e NÃO `--input`.**
    ///
    /// A referência escreve `border` seco no popup, que no Tailwind do coss resolve pro `--border`
    /// (preto 8% no claro, branco 6% no escuro). O gatilho do [`crate::select`] usa `--input` (10% e
    /// 8%) porque lá a classe é outra — pegar a de lá deixaria o popup 2 pontos mais marcado que o do
    /// [`crate::menu`], que aparece ao lado dele.
    #[test]
    fn a_borda_do_popup_e_border_e_nao_input() {
        assert_eq!(crate::menu::MENU_LIGHT.border, Rgba8(0x00000014), "--border claro: preto 8%");
        assert_eq!(crate::menu::MENU_DARK.border, Rgba8(0xffffff0f), "--border escuro: branco 6%");
        // `--input` seria 0x1a (10%) e 0x14 (8%) — os valores do `select.rs`.
        assert_ne!(crate::menu::MENU_LIGHT.border, Rgba8(0x0000001a), "isto é --input, não --border");
        assert_ne!(crate::menu::MENU_DARK.border, Rgba8(0xffffff14), "isto é --input, não --border");
    }

    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, porque a 6% o filete é
    /// imperceptível no nosso fundo. É o mesmo desvio já vigente no `input.rs`, no `card.rs`, no
    /// `select.rs` e no `menu.rs` — o valor tem que ser o MESMO nos cinco, senão o popup do
    /// autocomplete e o do menu ficam lado a lado com relevos diferentes. Se alguém "corrigir" pra
    /// 6% achando que é erro de porte, este teste falha e aponta pra cá.
    ///
    /// O tema CLARO segue fiel (preto 4%) — o desvio é só no escuro, onde o problema existia.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = crate::menu::MENU_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%); veio {claro}"
        );

        let escuro = crate::menu::MENU_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    /// A sombra é o `shadow-lg/5` do Tailwind v4: duas camadas com spread NEGATIVO, e a cor
    /// substituída por preto 5% (o `/5`). O spread negativo é o que a faz nascer menor que a caixa;
    /// positivo, ela apareceria como uma auréola em volta do popup.
    #[test]
    fn a_sombra_e_o_shadow_lg_com_5_por_cento() {
        theme::set_theme(theme::ThemeMode::Dark);
        let camadas = popup_shadow();
        assert_eq!(camadas.len(), 2, "--shadow-lg tem DUAS camadas");
        assert_eq!(camadas[0].offset.y, px(10.0));
        assert_eq!(camadas[0].blur_radius, px(15.0));
        assert_eq!(camadas[1].offset.y, px(4.0));
        assert_eq!(camadas[1].blur_radius, px(6.0));
        for c in &camadas {
            assert!(c.spread_radius < px(0.0), "o spread do shadow-lg é NEGATIVO");
            assert_eq!(c.offset.x, px(0.0), "a sombra só desce, não desloca de lado");
            // O `/5` do Tailwind substitui a cor inteira por preto 5%.
            assert!((c.color.a - 13.0 / 255.0).abs() < 1e-3, "preto 5%");
        }
    }

    /// **O fio de bisel é DESENHADO no lado do sinal** — não só declarado nele.
    ///
    /// O teste do `bevel_dir` (acima) tranca a intenção; este tranca a tradução dela em borda, que é
    /// onde o erro custa pixel: trocar o `border_b_1()` pelo `border_t_1()` no [`bevel_overlay`] não
    /// mexe em constante nenhuma e põe o filete na aresta errada nos DOIS temas de uma vez.
    ///
    /// De passagem tranca o **raio** do overlay: ele cobre a BORDER box (`inset: -1px`), então o raio
    /// dele é o da superfície — e **não** o `calc(var(--radius-lg) - 1px)` = 9 que o `.tsx` escreve
    /// (lá o pseudo-elemento fica por DENTRO da borda). Ver o doc de [`bevel_overlay`].
    #[test]
    fn o_fio_de_bisel_e_desenhado_no_lado_do_sinal() {
        let px_len = |v: f32| Some(gpui::AbsoluteLength::Pixels(px(v)));

        for (modo, na_base) in [
            (theme::ThemeMode::Light, crate::menu::MENU_LIGHT.bevel_dir > 0.0),
            (theme::ThemeMode::Dark, crate::menu::MENU_DARK.bevel_dir > 0.0),
        ] {
            theme::set_theme(modo);
            let mut overlay = bevel_overlay();
            let estilo = overlay.style();
            let bordas = estilo.border_widths.clone();
            let raios = estilo.corner_radii.clone();

            if na_base {
                assert!(bordas.bottom.is_some(), "{modo:?}: `0 1px` desce — filete na BASE");
                assert!(bordas.top.is_none(), "{modo:?}: e nada no topo");
            } else {
                assert!(bordas.top.is_some(), "{modo:?}: `0 -1px` sobe — filete no TOPO");
                assert!(bordas.bottom.is_none(), "{modo:?}: e nada na base");
            }
            // Nunca nos lados: o filete é horizontal, não um contorno.
            assert!(
                bordas.left.is_none() && bordas.right.is_none(),
                "{modo:?}: o bisel é um LADO, não uma moldura"
            );
            assert_eq!(raios.top_left, px_len(RADIUS), "{modo:?}: o raio é o da SUPERFÍCIE, não -1");
            // O overlay cobre a border box: sai 1px pra fora da padding box em cada lado.
            let inset = estilo.inset.clone();
            assert_eq!(
                inset.top,
                Some(gpui::Length::Definite(gpui::DefiniteLength::Absolute(
                    gpui::AbsoluteLength::Pixels(px(-BORDER))
                ))),
                "{modo:?}: inset -1px, pra cobrir a BORDER box"
            );
        }
    }

    /// As duas linhas de texto apagado (`Empty` e `Status`) declaram o par corpo/entrelinha do
    /// Tailwind e a cor `--muted-foreground`. São as únicas linhas do popup que não são item, e por
    /// isso as mais fáceis de esquecer de fixar a entrelinha.
    #[test]
    fn as_linhas_de_apoio_declaram_corpo_entrelinha_e_cor() {
        let px_len = |v: f32| Some(gpui::AbsoluteLength::Pixels(px(v)));
        let alt = |v: f32| Some(gpui::DefiniteLength::Absolute(gpui::AbsoluteLength::Pixels(px(v))));

        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let muted = palette().muted.hsla();

            let mut vazio = empty_row("Nada encontrado".into());
            let e = vazio.style();
            let texto = e.text.clone().expect("o Empty declara estilo de texto");
            assert_eq!(texto.font_size, px_len(TEXT_SIZE), "{modo:?}: Empty é sm:text-sm = 14");
            assert_eq!(texto.line_height, alt(TEXT_LINE_HEIGHT), "{modo:?}: 14/20, não a razão de ouro");
            assert_eq!(texto.color, Some(muted), "{modo:?}: text-muted-foreground");
            assert_eq!(e.padding.left, alt(EMPTY_PAD), "{modo:?}: not-empty:p-2");

            let mut estado = status_row("Buscando…".into());
            let s = estado.style();
            let texto = s.text.clone().expect("o Status declara estilo de texto");
            assert_eq!(texto.font_size, px_len(SMALL_TEXT_SIZE), "{modo:?}: Status é text-xs = 12");
            assert_eq!(texto.line_height, alt(SMALL_LINE_HEIGHT), "{modo:?}: 12/16");
            assert_eq!(texto.color, Some(muted), "{modo:?}: text-muted-foreground");
            assert_eq!(texto.font_weight, Some(FontWeight::MEDIUM), "{modo:?}: font-medium");
            assert_eq!(s.padding.left, alt(STATUS_PAD_X), "{modo:?}: px-3");
            assert_eq!(s.padding.top, alt(STATUS_PAD_Y), "{modo:?}: py-2");
        }
    }

    // --- As linhas ------------------------------------------------------------------------------

    /// O construtor de linha diz o que a linha é, e `disabled` só morde numa sugestão — um rótulo ou
    /// um separador "desabilitado" não existe, e silenciosamente virar um seria pior que ignorar.
    #[test]
    fn a_linha_sabe_o_que_e() {
        let item = AutocompleteRow::item("Laranja");
        assert_eq!(item.label().map(SharedString::to_string), Some("Laranja".into()));
        assert!(item.is_item() && item.is_navigable());

        let desab = AutocompleteRow::item("Laranja").disabled(true);
        assert!(desab.is_item(), "desabilitada continua sendo sugestão (conta pro Empty)");
        assert!(!desab.is_navigable(), "mas o teclado não a alcança");

        let rotulo = AutocompleteRow::group_label("Cítricas").disabled(true);
        assert!(!rotulo.is_item() && !rotulo.is_navigable());
        assert_eq!(rotulo.label(), None, "rótulo de grupo não é rótulo de sugestão");
        assert_eq!(rotulo, AutocompleteRow::group_label("Cítricas"), "o disabled foi ignorado");

        let sep = AutocompleteRow::separator();
        assert!(!sep.is_item() && !sep.is_navigable());
    }
}

/// **O que só a janela mede.**
///
/// Os testes de unidade acima trancam as *regras* ([`enter_action`], [`escape_action`],
/// [`step_highlight`], [`visible_rows`]). Estes trancam duas coisas que nenhuma função pura vê:
///
/// 1. **A tecla chega?** As quatro teclas deste componente (`↑ ↓ Enter Escape`) estão vinculadas a
///    ações no contexto de teclas do [`InputState`], e o campo tem ouvinte próprio pra elas. Só um
///    teste com um campo de verdade **focado** responde se o roteamento funciona — ver o comentário
///    longo no `render` do [`Autocomplete`] pra por que ele é `capture_action`, e pra o que ali é
///    defensivo em vez de load-bearing.
/// 2. **A ORDEM dos efeitos do GPUI.** O `cx.emit` é diferido: o `InputEvent::Change` de uma escrita
///    nossa volta um flush DEPOIS de [`Autocomplete::choose`]/[`Autocomplete::clear`] terem
///    terminado. Foram estes testes que pegaram os dois defeitos que o [`Echo`] conserta — escolher
///    reabria a lista, limpar a fechava — e são eles que impedem a volta dos dois (mutação-verificado
///    tirando o `Echo` de cada uma das duas).
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{AppContext as _, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Abre uma janela com um autocomplete de quatro linhas (a última desabilitada) e **o campo já
    /// focado** — sem o foco no campo, as ações do núcleo nem entram no caminho de dispatch e o teste
    /// não mediria nada.
    ///
    /// O [`gpui_component::Root`] na raiz não é enfeite: o campo de texto do núcleo chama `Root::read`
    /// no caminho de teclado, e sem ele qualquer tecla derruba o teste com "the window root view
    /// should be of type `ui::Root`". Mesma armadilha (e mesmo remédio) do
    /// `color_picker::tests_de_janela`.
    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
    ) -> (
        Entity<Autocomplete>,
        Entity<InputState>,
        Rc<RefCell<Vec<AutocompleteEvent>>>,
        VisualTestContext,
    ) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let eventos: Rc<RefCell<Vec<AutocompleteEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();
        let mut saida = None;

        let window = cx.add_window(|window, cx| {
            let estado = cx.new(|cx| crate::input::single_line(window, cx));
            let ac = cx.new(|cx| {
                Autocomplete::new(&estado, cx).rows(vec![
                    AutocompleteRow::group_label("Cítricas"),
                    AutocompleteRow::item("Laranja"),
                    AutocompleteRow::item("Limão"),
                    AutocompleteRow::item("Lichia").disabled(true),
                ])
            });
            cx.subscribe(&ac, move |_this, _e, ev: &AutocompleteEvent, _cx| {
                capturados.borrow_mut().push(ev.clone());
            })
            .detach();
            estado.read(cx).focus_handle(cx).focus(window);
            saida = Some((ac.clone(), estado));
            gpui_component::Root::new(ac, window, cx)
        });

        let (ac, estado) = saida.expect("autocomplete montado");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        (ac, estado, eventos, vcx)
    }

    /// **Digitar filtra e abre; as setas andam; `Enter` completa o campo.** O ciclo inteiro, pelas
    /// teclas de verdade.
    #[gpui::test]
    fn digitar_filtra_e_as_setas_com_enter_completam_o_campo(cx: &mut TestAppContext) {
        let (ac, estado, eventos, mut vcx) = abrir(cx);

        // Fechado em repouso: `openOnInputClick` é `false` no Autocomplete.
        assert!(!vcx.read(|cx| ac.read(cx).is_open()), "nasce fechado");

        vcx.simulate_keystrokes("l i");
        assert_eq!(vcx.read(|cx| estado.read(cx).value()), "li");
        assert!(vcx.read(|cx| ac.read(cx).is_open()), "digitar abriu a lista");
        // "li" casa com Limão e Lichia, mas não com Laranja — e o rótulo do grupo sobrevive.
        assert_eq!(vcx.read(|cx| ac.read(cx).visible(cx)), vec![0, 2, 3]);
        assert_eq!(
            vcx.read(|cx| ac.read(cx).highlighted()),
            None,
            "digitar NÃO destaca nada (autoHighlight é false)"
        );

        // ⚠️ A tecla que prova o `capture_action`: sem ele, o `MoveDown` morreria no `InputState`.
        vcx.simulate_keystrokes("down");
        assert_eq!(
            vcx.read(|cx| ac.read(cx).highlighted()),
            Some(2),
            "a 1ª seta acende o 1º item ALCANÇÁVEL (Lichia é desabilitada e fica de fora)"
        );
        // Só há um item alcançável em "li": a seguinte volta pro campo (o ciclo len+1).
        vcx.simulate_keystrokes("down");
        assert_eq!(vcx.read(|cx| ac.read(cx).highlighted()), None, "o fim volta pro campo");
        vcx.simulate_keystrokes("up");
        assert_eq!(vcx.read(|cx| ac.read(cx).highlighted()), Some(2), "e a seta pra cima reentra");

        vcx.simulate_keystrokes("enter");
        assert_eq!(
            vcx.read(|cx| estado.read(cx).value()),
            "Limão",
            "o Enter completou o campo com o rótulo (fillInputOnItemPress)"
        );
        assert!(!vcx.read(|cx| ac.read(cx).is_open()), "e fechou a lista");
        assert_eq!(vcx.read(|cx| ac.read(cx).highlighted()), None);

        // Os eventos: o par Select+Change da escolha, nessa ordem, no fim.
        let evs = eventos.borrow().clone();
        assert_eq!(
            evs.last().cloned(),
            Some(AutocompleteEvent::Change("Limão".into())),
            "o último evento é o valor novo"
        );
        assert_eq!(
            evs[evs.len() - 2],
            AutocompleteEvent::Select(2),
            "e antes dele o Select com o índice da LINHA (2), não o da posição visível (1)"
        );
    }

    /// **A seta com a lista fechada só ABRE, sem destacar** — é o `focusItemOnOpen: false` que o
    /// `selectionMode: "none"` do Autocomplete produz. É contra-intuitivo (num menu a 1ª seta já
    /// acende) e por isso fica travado.
    #[gpui::test]
    fn a_seta_com_a_lista_fechada_so_abre(cx: &mut TestAppContext) {
        let (ac, _estado, _eventos, mut vcx) = abrir(cx);

        vcx.simulate_keystrokes("down");
        assert!(vcx.read(|cx| ac.read(cx).is_open()), "a seta abriu");
        assert_eq!(
            vcx.read(|cx| ac.read(cx).highlighted()),
            None,
            "e NÃO destacou nada — a 2ª seta é que acende"
        );

        vcx.simulate_keystrokes("down");
        assert_eq!(vcx.read(|cx| ac.read(cx).highlighted()), Some(1), "agora sim, Laranja");
    }

    /// **`Escape` é de dois estágios**: o 1º fecha a lista, o 2º apaga o texto.
    #[gpui::test]
    fn escape_fecha_e_so_o_segundo_apaga(cx: &mut TestAppContext) {
        let (ac, estado, _eventos, mut vcx) = abrir(cx);

        vcx.simulate_keystrokes("l a");
        assert!(vcx.read(|cx| ac.read(cx).is_open()));

        vcx.simulate_keystrokes("escape");
        assert!(!vcx.read(|cx| ac.read(cx).is_open()), "o 1º escape fechou");
        assert_eq!(
            vcx.read(|cx| estado.read(cx).value()),
            "la",
            "e NÃO apagou o texto — as duas coisas nunca acontecem na mesma tecla"
        );

        vcx.simulate_keystrokes("escape");
        assert_eq!(vcx.read(|cx| estado.read(cx).value()), "", "o 2º escape apagou");
        assert!(!vcx.read(|cx| ac.read(cx).is_open()));
    }

    /// **Apagar o texto até esvaziar FECHA a lista** (é o `input-clear`, porque
    /// `openOnInputClick: false`), e o `Enter` sem destaque só fecha, sem completar nada.
    #[gpui::test]
    fn esvaziar_digitando_fecha_e_enter_sem_destaque_so_fecha(cx: &mut TestAppContext) {
        let (ac, estado, _eventos, mut vcx) = abrir(cx);

        vcx.simulate_keystrokes("l");
        assert!(vcx.read(|cx| ac.read(cx).is_open()));
        vcx.simulate_keystrokes("backspace");
        assert_eq!(vcx.read(|cx| estado.read(cx).value()), "");
        assert!(!vcx.read(|cx| ac.read(cx).is_open()), "esvaziar digitando fecha");

        // Reabre pela seta e aperta Enter sem nada destacado.
        vcx.simulate_keystrokes("down");
        assert!(vcx.read(|cx| ac.read(cx).is_open()));

        // ⚠️ **A ação, e não `simulate_keystrokes("enter")`.** O `Enter` sem destaque NÃO consome a
        // tecla (é o que deixa um `Enter` chegar no `InputEvent::PressEnter` de quem hospeda, o
        // análogo do submit de formulário da referência) — e o
        // `Keystroke::with_simulated_ime` do GPUI dá a "enter" um `key_char` de `"\n"`, que o
        // `dispatch_keystroke` insere no campo quando ninguém consumiu. Num `InputState` de UMA linha
        // isso derruba o `text_system` com "text argument should not contain newlines". É artefato do
        // simulador (no macOS de verdade o Enter chega como comando `insertNewline:`, não como texto)
        // e vale pra qualquer campo de uma linha desta lib, não só pra este componente. Despachar a
        // ação exercita o mesmo caminho de `capture_action` sem passar pelo inseridor de texto.
        vcx.dispatch_action(Enter { secondary: false });
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| ac.read(cx).is_open()), "o Enter sem destaque só fecha");
        assert_eq!(
            vcx.read(|cx| estado.read(cx).value()),
            "",
            "e não completa nada"
        );
    }

    /// **O botão de limpar NÃO mexe no aberto/fechado** — o assimétrico com esvaziar digitando, e o
    /// ponto mais fácil de errar do componente (o `ComboboxClear` não chama `setOpen` em lugar
    /// nenhum). Com a lista aberta, limpar a deixa aberta mostrando TUDO.
    #[gpui::test]
    fn limpar_apaga_o_texto_e_deixa_a_lista_como_estava(cx: &mut TestAppContext) {
        let (ac, estado, _eventos, mut vcx) = abrir(cx);

        vcx.simulate_keystrokes("l i m");
        assert!(vcx.read(|cx| ac.read(cx).is_open()));
        assert_eq!(vcx.read(|cx| ac.read(cx).visible(cx)), vec![0, 2]);

        vcx.update(|window, cx| {
            ac.update(cx, |this, cx| this.clear(window, cx));
        });
        vcx.run_until_parked();

        assert_eq!(vcx.read(|cx| estado.read(cx).value()), "", "apagou o texto");
        assert!(
            vcx.read(|cx| ac.read(cx).is_open()),
            "e a lista CONTINUA aberta — limpar não fecha"
        );
        assert_eq!(
            vcx.read(|cx| ac.read(cx).visible(cx)),
            vec![0, 1, 2, 3],
            "agora mostrando tudo, porque busca vazia casa com tudo"
        );
        assert_eq!(vcx.read(|cx| ac.read(cx).highlighted()), None, "e sem destaque");
    }

    /// **Desabilitado não abre por nada: nem por texto, nem pela seta.**
    ///
    /// ⚠️ As duas guardas são chamadas **direto**, e não por `simulate_keystrokes`, porque pelas
    /// teclas este teste era VAZIO — descoberto na mutação-verificação: um campo desabilitado deixa de
    /// receber tecla, então nem [`Autocomplete::arrow`] nem [`Autocomplete::on_text_changed`] rodavam,
    /// e apagar as duas guardas não fazia o teste cair. Chamando direto, cada guarda é medida de fato
    /// (as mutações que as removem agora falham).
    ///
    /// Elas não são redundantes com "o campo não recebe tecla": [`Autocomplete::set_open`] é **pública**
    /// e [`Autocomplete::toggle`] sai de um clique no botão de gatilho, que não passa pelo teclado.
    #[gpui::test]
    fn desabilitado_nao_abre(cx: &mut TestAppContext) {
        let (ac, _estado, _eventos, mut vcx) = abrir(cx);

        vcx.update(|window, cx| {
            ac.update(cx, |this, cx| {
                this.disabled = true;

                // A seta: a guarda devolve `false` (não consumiu a tecla) e não abre.
                assert!(!this.arrow(true, window, cx), "a seta não consome a tecla");
                assert!(!this.open, "e não abre a lista");

                // O texto: mesmo com texto que abriria (`open_after_input("l")` é `true`).
                this.on_text_changed("l".into(), cx);
                assert!(open_after_input("l"), "o texto ABRIRIA, se não estivesse desabilitado");
                assert!(!this.open, "mas desabilitado não abre");

                // E o caminho público também respeita.
                this.set_open(true, cx);
                assert!(!this.open, "set_open(true) desabilitado não abre");
            });
        });

        // De passagem: pelas teclas também não abre — porque o campo desabilitado nem as recebe. É a
        // segunda camada, e é a razão de as guardas acima serem chamadas direto.
        vcx.simulate_keystrokes("l");
        assert!(!vcx.read(|cx| ac.read(cx).is_open()));
        vcx.simulate_keystrokes("down");
        assert!(!vcx.read(|cx| ac.read(cx).is_open()));
    }
}

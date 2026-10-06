//! `Menu` — o **menu suspenso** (dropdown menu / menu de contexto) do `empire-ui`, com o visual do
//! design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/menu.tsx`
//!
//! Diferente do [`crate::select::Select`], que edita **um valor** (escolha 1-de-N e mostra a
//! escolhida no gatilho), o `Menu` dispara **ações**: cada item é um comando, e o gatilho continua
//! sendo o que era (um botão) depois do clique. É o menu de "…" de um painel, o `File`/`Edit` de uma
//! barra, o menu de contexto de um clip da timeline.
//!
//! # Anatomia
//!
//! ```text
//! ┌──────────┐
//! │  Editar ⌄│                       ← o GATILHO (`MenuTrigger`): um `Button` ou o que você quiser
//! └──────────┘
//!  ┌────────────────────────────────┐
//!  │ Arquivo                        │ ← `MenuGroupLabel`  (12px, medium, --muted-foreground)
//!  │ ⧉  Duplicar              ⌘D    │ ← `MenuItem` + ícone + `MenuShortcut`
//!  │ ─────────────────────────────  │ ← `MenuSeparator`
//!  │ ✓  Mostrar grade               │ ← `MenuCheckboxItem`
//!  │ ✓  Pequeno                     │ ← `MenuRadioItem` (o coss marca radio com CHECK, não bolinha)
//!  │    Apagar                      │ ← `MenuItem` destrutivo (texto --destructive-foreground)
//!  └────────────────────────────────┘
//! ```
//!
//! # Não há portal no GPUI — o caminho é o do `Select`
//!
//! A referência monta `MenuPortal` → `MenuPositioner` → `MenuPopup`: o popup **sai** da árvore e se
//! planta na janela, e o `Positioner` (Base UI) resolve `side`/`align`/offsets contra o retângulo do
//! gatilho. Aqui não existe portal. O caminho **já trilhado nesta base** é o do
//! [`crate::select::Select`], e é o que este módulo segue:
//!
//! 1. os `bounds` do gatilho são capturados por um [`gpui::canvas`] overlay (lê os bounds no
//!    prepaint) — é o "anchor rect" do `Positioner`;
//! 2. o popup é renderizado dentro de [`gpui::deferred`]`(`[`gpui::anchored`]`(..))`, que o pinta
//!    **por cima** de todo o resto e o gruda na janela quando ele estouraria a borda;
//! 3. o popup é `.occlude()` (não vaza clique pra trás) e fecha no `on_mouse_down_out`.
//!
//! O storybook já monta o `Root` do `gpui-component`, que provê as camadas de popover — o mesmo
//! ambiente em que o `Select` funciona.
//!
//! ## `side`/`align` são conta nossa — e saem no PRIMEIRO frame
//!
//! O [`gpui::anchored`] ancora um **canto** do filho num ponto. Isso cobre 8 das 12 combinações de
//! `side` × `align` de graça: `align: start`/`end` são exatamente "este canto do popup neste canto do
//! gatilho" (ver [`anchor_point`]) — e o `anchored` já conhece o tamanho do popup quando resolve o
//! canto, então **não é preciso medir nada**.
//!
//! Sobra o `align: center` (o **default** da referência), que não é um canto. Esse sai por **layout**:
//! o popup vai dentro de um container do tamanho do gatilho **no eixo transversal**, com
//! `justify-center`. Como o popup é `flex-none` e em geral mais largo que o gatilho, o espaço livre é
//! negativo e ele transborda **igualmente pros dois lados** — que é a definição de centralizado.
//!
//! ⚠️ **Uma versão anterior media o popup por `canvas` e calculava a posição.** Não use esse caminho:
//! a medida só existe DEPOIS do prepaint, então o primeiro frame saía deslocado (meia largura à
//! direita), e o frame de correção **não vinha** — nem por `request_animation_frame` nem por `notify`
//! de dentro do prepaint. O popup ficava pintado no lugar errado, e os cliques nos itens caíam FORA
//! dele (o `on_mouse_down_out` fechava o menu em vez de o item receber o clique). Quatro testes de
//! janela pegaram isso; ver `tests_de_janela::o_clique_cai_no_item_certo_e_so_a_acao_fecha`.
//!
//! O preço do `center` é que o `snap_to_window_with_margin` passa a medir o container (do tamanho do
//! gatilho) em vez do popup, então um menu centralizado colado na borda da janela pode transbordar.
//! Com `align: start`/`end` o encaixe na janela é exato.
//!
//! ## O mesmo desenho serve o menu de CONTEXTO
//!
//! O [`crate::context_menu::ContextMenu`] (clique direito) **é** este módulo: popup, itens, teclado,
//! rolagem, fechamento e paleta, tudo daqui. As strings de classe do `context-menu.tsx` do coss são
//! byte-a-byte as do `menu.tsx` no popup, no item, no separador, no checkbox, no radio, no rótulo de
//! grupo e no atalho — duplicar o desenho lá seria criar duas verdades pra um pixel só.
//!
//! A diferença real é **como abre**, e ela cabe num enum: ver [`MenuAnchoring`]. Ancorado no
//! ponteiro, o `Menu` não renderiza gatilho nenhum (a raiz fica 0×0 e quem embrulha os filhos é a
//! região do outro módulo), o "anchor rect" é um retângulo de tamanho ZERO no ponto do clique
//! ([`pointer_rect`]), e o popup **vira** pro outro lado em vez de deslizar ([`snaps_to_window`]).
//!
//! # Armadilhas do GPUI que este módulo paga
//!
//! - **A sombra do `Window::paint_shadows` não é recortada** pra fora do elemento (o CSS recorta).
//!   O filete de bisel (`before:shadow-[0_1px_…]` da referência) portanto **não** é sombra: é uma
//!   **borda de 1px num único lado** de um overlay absoluto que cobre a BORDER box do popup — ver
//!   [`bevel_overlay`]. A sombra externa (`shadow-lg/5`) é segura porque o `--popover` é **opaco**
//!   nos dois temas e esconde o retângulo cheio que o GPUI pinta atrás.
//! - **O overlay do bisel cobre a border box** (`inset: -1px` a partir da padding box), e por isso o
//!   raio dele é o da SUPERFÍCIE (10), não `10 − 1`.
//! - **A superfície do popup NÃO tem `overflow_hidden`** — se tivesse, recortaria o bisel. Quem rola
//!   é a lista interna, e o bisel é irmão dela.
//! - **Margem negativa colapsa o layout no GPUI**, então os `-mx-0.5`/`-ms-0.5` que a referência põe
//!   nos ícones viram **geometria equivalente**: o respiro e o vão daquele lado perdem os 2px. Ver
//!   [`action_pad_start`] e [`INDICATOR_GAP`].
//!
//! # Contrato
//!
//! Mesmo contrato dos outros controles: é um `Entity` (view) que mantém o seu estado (aberto,
//! item destacado, marcações de checkbox/radio) e **emite** [`MenuEvent`]. Quem usa assina
//! (`cx.subscribe`) e age. Os `set_*` sincronizam de fora **sem** emitir (evita loop de feedback —
//! mesma regra do `Switch::set_on`).
//!
//! ```ignore
//! let menu = cx.new(|cx| {
//!     Menu::new(
//!         vec![
//!             MenuItem::group_label("Arquivo"),
//!             MenuItem::new("Duplicar").icon("icons/copy.svg").shortcut("⌘D"),
//!             MenuItem::separator(),
//!             MenuItem::checkbox("Mostrar grade", true),
//!             MenuItem::new("Apagar").destructive(),
//!         ],
//!         cx,
//!     )
//!     .trigger_button("Editar")
//! });
//! cx.subscribe(&menu, |_this, _menu, ev: &MenuEvent, _cx| println!("{ev:?}")).detach();
//! ```
//!
//! # O que NÃO está aqui (declarado, não esquecido)
//!
//! - **Submenu** (`MenuSub`/`MenuSubTrigger`/`MenuSubPopup`): ausente. Precisa de um segundo nível
//!   de ancoragem (anchor = o próprio item) e de atraso de abertura no hover; entregar isso trêmulo
//!   é pior que não entregar. O ícone já está mapeado pra quem for fazer: `icons/chevron_right.svg`
//!   tem o path EXATO do `ChevronRightIcon` do lucide usado na referência.
//! - **`MenuCheckboxItem` com `variant="switch"`**: ausente. É o [`crate::switch::Switch`] inteiro
//!   dentro do item (trilho, bolinha que desliza, `scale-x-110` na pressão) — outro componente.
//! - **`MenuLinkItem`**: na referência ele tem **exatamente** as mesmas classes do `MenuItem`; o que
//!   muda é ser um `<a href>`. Sem navegação nativa, use [`MenuItem`] e trate o
//!   [`MenuEvent::Select`] — é o mesmo pixel.
//! - **Animação de entrada/saída**: a referência (352 linhas) **não especifica nenhuma**. A única
//!   pista é `origin-(--transform-origin)`, que é `transform-origin` pra uma animação declarada
//!   FORA deste arquivo, que não me foi dada. Não inventei uma.
//! - **`anchor` do `Positioner`** (ancorar em outro elemento que não o gatilho) e o
//!   `in-data-[side=none]:min-w-[calc(var(--anchor-width)+1.25rem)]` dos itens de checkbox/radio (o
//!   modo `side=none`, em que o popup cobre o gatilho): ausentes.
//! - **RTL**: [`MenuSide::Left`]/[`MenuSide::Right`] são o `inline-start`/`inline-end` da referência
//!   resolvidos pra LTR. Não há inversão por direção de escrita.
//! - **`tracking-widest` do `MenuShortcut`** (0,1em de entreletra): o GPUI não expõe letter-spacing.
//! - **`not-dark:bg-clip-padding` do popup**: no tema claro o coss recorta o fundo na padding box,
//!   pra a borda translúcida (preto 8%) mostrar o que há ATRÁS do menu em vez do branco do próprio
//!   popup. O GPUI pinta o fundo sob a borda e não tem `background-clip`, então no tema claro a borda
//!   sai ~8% mais escura que o fundo do popup em vez de compor com o que está atrás. Diferença de
//!   uma borda de 1px, e sem contorno no GPUI pra reproduzir.
//!
//! Uma nota de fidelidade que **não** é omissão: os itens usam o cursor de seta, não a mãozinha. É o
//! `cursor-default` da referência. O [`crate::select::Select`] desvia disso no popup dele; aqui o
//! arquivo ganha.

use crate::color::Rgba8;
use crate::theme;
use crate::{Button, ButtonVariant};
use gpui::{
    anchored, canvas, deferred, div, point, prelude::FluentBuilder as _, px, svg, AnyElement, App,
    Bounds, Context, Corner, Div, EventEmitter, FocusHandle, Focusable, FontWeight,
    InteractiveElement, IntoElement, KeyDownEvent, MouseDownEvent, ParentElement, Pixels, Point,
    Render, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window,
};

// =================================================================================================
// Eventos
// =================================================================================================

/// O que o [`Menu`] emite. Todos os índices são posições na lista **achatada** de
/// [`MenuItem`] (a mesma que você passou pro [`Menu::new`]), incluindo rótulos e separadores — é o
/// índice que [`Menu::items`] indexa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEvent {
    /// Um item de ação foi escolhido (clique ou `Enter`).
    Select(usize),
    /// Um [`MenuItem::checkbox`] alternou. `checked` já é o valor NOVO.
    CheckedChange {
        /// Índice do item.
        index: usize,
        /// Estado novo.
        checked: bool,
    },
    /// Um [`MenuItem::radio`] foi escolhido. Só emite quando a escolha **muda** (re-clicar o já
    /// marcado não emite — mesma regra do [`crate::select::SelectEvent`]).
    RadioChange {
        /// Índice do item recém-marcado.
        index: usize,
        /// Grupo ao qual ele pertence.
        group: usize,
    },
    /// O popup abriu (`true`) ou fechou (`false`) — o `onOpenChange` da referência.
    OpenChange(bool),
}

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================

/// Tokens visuais do menu, por tema.
///
/// ⚠️ **TODAS as cores aqui são `0xRRGGBBAA`** — com o byte de alfa, sempre, mesmo quando opacas
/// (`…ff`). É a convenção das paletas de componente do crate (ver [`crate::color`]); misturar com os
/// tokens de 6 dígitos da [`crate::theme::Palette`] **desloca os canais** e dá uma cor
/// completamente diferente, **sem erro de compilação** (`rgba(0xffffff)` = ciano).
#[derive(Clone, Copy, Debug)]
pub(crate) struct MenuPalette {
    /// Fundo da superfície do popup (`bg-popover`). **Opaco** nos dois temas — é o que torna a
    /// sombra externa segura.
    pub(crate) popover_bg: Rgba8,
    /// Borda do popup e cor do [`MenuItem::separator`] (`--border`).
    ///
    /// Note que é `--border`, e **não** `--input`: a referência escreve `border` seco no popup, que
    /// no Tailwind do coss resolve pro `--border` (preto 8% no claro, branco 6% no escuro). O
    /// gatilho do [`crate::select::Select`] usa `--input` porque lá a classe é outra.
    pub(crate) border: Rgba8,
    /// Texto dos itens (`text-foreground`).
    pub(crate) text: Rgba8,
    /// Texto do [`MenuItem::group_label`] (`--muted-foreground`).
    pub(crate) muted: Rgba8,
    /// Texto do atalho (`text-muted-foreground/72`).
    ///
    /// `pub(crate)` porque o `CommandShortcut` do [`crate::command`] é a MESMA classe
    /// (`font-medium text-muted-foreground/72 text-xs tracking-widest`) no mesmo papel — o atalho à
    /// direita de uma linha de lista. Um segundo `0x…b8` na base seria a segunda verdade pro mesmo
    /// pixel.
    pub(crate) shortcut: Rgba8,
    /// Fundo do item destacado — por hover OU por teclado (`data-highlighted:bg-accent`).
    pub(crate) accent: Rgba8,
    /// Texto do item destacado (`--accent-foreground`).
    ///
    /// Nos dois temas do coss ele é IGUAL ao `--foreground`: o destaque muda só o fundo. Fica como
    /// token próprio porque é assim no original, e um tema derivado pode divergir.
    pub(crate) accent_text: Rgba8,
    /// Texto do item destrutivo (`--destructive-foreground`) — red-700 no claro, red-400 no escuro.
    destructive_text: Rgba8,
    /// Fio de bisel de 1px sobre a borda do popup. No claro é escuro e desce 1px; no escuro é claro
    /// e sobe 1px — é o `before:shadow-[0_1px_black/4%]` / `[0_-1px_white/6%]` do coss.
    pub(crate) bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (fio na BASE), `-1` sobe (fio no TOPO). O sinal sai do
    /// deslocamento da sombra da referência, e errar isso põe o filete no lado errado.
    pub(crate) bevel_dir: f32,
    /// Cor das duas camadas da sombra externa (`shadow-lg/5` = preto 5%).
    pub(crate) shadow: Rgba8,
}

/// Tema **claro**.
pub(crate) const MENU_LIGHT: MenuPalette = MenuPalette {
    popover_bg: Rgba8(0xffffffff),
    border: Rgba8(0x00000014), // --border: preto 8%
    text: Rgba8(0x262626ff),   // neutral-800
    // --muted-foreground = mix(neutral-500 90%, black) = #686868
    muted: Rgba8(0x686868ff),
    shortcut: Rgba8(0x686868b8), // o mesmo a 72%
    accent: Rgba8(0x0000000a),   // --accent: preto 4%
    accent_text: Rgba8(0x262626ff),
    destructive_text: Rgba8(0xb91c1cff), // red-700
    bevel: Rgba8(0x0000000a),            // preto 4% — fiel à referência
    bevel_dir: 1.0,
    shadow: Rgba8(0x0000000d), // preto 5%
};

/// Tema **escuro**.
pub(crate) const MENU_DARK: MenuPalette = MenuPalette {
    // --popover escuro = mix(background 96%, white) = #1d1d1d (o --background é #141414): o popup
    // LEVANTA sobre o painel em vez de sumir nele.
    popover_bg: Rgba8(0x1d1d1dff),
    border: Rgba8(0xffffff0f), // --border: branco 6%
    text: Rgba8(0xf5f5f5ff),   // neutral-100
    // --muted-foreground = mix(neutral-500 90%, white) = #818181
    muted: Rgba8(0x818181ff),
    shortcut: Rgba8(0x818181b8), // o mesmo a 72%
    accent: Rgba8(0xffffff0a),   // --accent: branco 4%
    accent_text: Rgba8(0xf5f5f5ff),
    destructive_text: Rgba8(0xf87171ff), // red-400
    // ⚠️ DESVIO CONSCIENTE do coss: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO —
    // alfa 30 ≈ 11,8% — por decisão de design: a 6% o filete é imperceptível no nosso fundo escuro.
    // É o mesmo desvio já vigente no `input.rs`, no `card.rs` e no `select.rs`; manter os quatro
    // iguais é o ponto — o popup do menu e o do dropdown aparecem lado a lado. NÃO "corrija" isto
    // pra 0x0f achando que é erro de porte; se a intenção mudar, mude junto o teste
    // `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
    shadow: Rgba8(0x0000000d), // preto 5% — igual no claro (o `/5` não muda com o tema)
};

/// A paleta do menu no tema corrente.
pub(crate) fn palette() -> &'static MenuPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &MENU_DARK,
        theme::ThemeMode::Light => &MENU_LIGHT,
    }
}

// =================================================================================================
// Geometria
// =================================================================================================
//
// Os utilitários Tailwind do `menu.tsx` resolvidos em número. O breakpoint `sm:` do Tailwind é
// ≥640px e uma janela de desktop está sempre acima disso, então valem SEMPRE as variantes `sm:`
// (item de 28px, texto de 14px, ícone de 16px). 1 unidade Tailwind = 4px; `--radius` = 10px.

/// Raio da superfície do popup — `rounded-lg` = `--radius-lg` = **10px**.
pub(crate) const RADIUS: f32 = 10.0;

/// Espessura da borda do popup — `border`.
pub(crate) const BORDER: f32 = 1.0;

/// Raio de um item — `rounded-sm` = `--radius-sm` = **6px**.
pub(crate) const ITEM_RADIUS: f32 = 6.0;

/// Largura mínima do popup — `min-w-32` (só vale quando não há largura explícita, o
/// `not-[class*='w-']:` da referência; ver [`Menu::width`]).
const MIN_WIDTH: f32 = 128.0;

/// Respiro da lista dentro do popup — `p-1`.
pub(crate) const LIST_PAD: f32 = 4.0;

/// Altura mínima de um item — `sm:min-h-7`.
pub(crate) const ITEM_MIN_HEIGHT: f32 = 28.0;

/// Respiro vertical de um item — `py-1`.
pub(crate) const ITEM_PAD_Y: f32 = 4.0;

/// Respiro horizontal de um item de ação — `px-2`.
///
/// `pub(crate)` porque a linha do [`crate::command`] é o mesmo `px-2` **simétrico** — e é justamente o
/// número que se erra ali, porque o item do [`crate::select`] (e o do Combobox) é `ps-2 pe-4`, com 16
/// no fim, por causa da coluna do check. Um item de paladar de comandos com `pe-4` deslocaria todo
/// rótulo 8px pra esquerda do centro da linha.
pub(crate) const ITEM_PAD_X: f32 = 8.0;

/// Vão entre ícone, rótulo e atalho — `gap-2`.
pub(crate) const ITEM_GAP: f32 = 8.0;

/// Respiro no início de um item **inset** — `data-inset:ps-8`. O `inset` alinha o rótulo de um item
/// SEM ícone com o rótulo dos itens que têm um, pra a lista não ficar dentada.
const INSET_PAD_START: f32 = 32.0;

/// Lado de um ícone — `sm:size-4`.
pub(crate) const ICON_SIZE: f32 = 16.0;

/// Opacidade dos ícones (`[&>svg]:opacity-80`). O ícone herda a cor do texto do item e a esmaece.
const ICON_OPACITY: f32 = 0.8;

/// Quanto um ícone "puxa" pra fora com o `-mx-0.5`/`-ms-0.5` da referência.
///
/// ⚠️ **Margem negativa colapsa o layout no GPUI** (três abas caíram uma sobre a outra no `tabs`),
/// então o valor não é aplicado como margem: ele é DESCONTADO do respiro e do vão daquele lado —
/// ver [`action_pad_start`], [`ICON_GAP`] e [`INDICATOR_GAP`]. A geometria resultante é a mesma.
const ICON_PULL: f32 = 2.0;

/// Vão entre o ícone e o rótulo de um item de ação: o `gap-2` menos o que o `-mx-0.5` do ícone puxa.
const ICON_GAP: f32 = ITEM_GAP - ICON_PULL;

/// Corpo do texto de um item — `sm:text-sm`.
pub(crate) const TEXT_SIZE: f32 = 14.0;

/// **Entrelinha** do texto de um item — o `text-sm` do Tailwind é `14px/20px`.
///
/// ⚠️ **Isto NÃO é decoração: sem ele o item fica 2,6px mais alto que o do coss.** O GPUI não usa a
/// entrelinha do CSS; o default dele é `phi()`, a razão de ouro (**1,618** × o corpo). Um item de
/// 14px sairia com 22,6px de linha + `py-1` = **30,6px**, estourando o `min-h-7` de 28 e inflando o
/// popup inteiro (141,5px em vez de 131 na lista de 5 linhas dos testes de janela). Foi um teste de
/// janela que pegou isso — nenhuma leitura do `.tsx` pegaria.
pub(crate) const TEXT_LINE_HEIGHT: f32 = 20.0;

/// Largura da coluna do indicador nos itens de checkbox/radio — a 1ª coluna do
/// `grid-cols-[.75rem_1fr]`, ou seja `0.75rem`.
const INDICATOR_COL: f32 = 12.0;

/// Respiro no início de um item de checkbox/radio, já com o `-ms-0.5` do indicador descontado:
/// `ps-2` − 2px. É o x em que o ícone de 16px começa a ser desenhado.
const INDICATOR_PAD_START: f32 = ITEM_PAD_X - ICON_PULL;

/// Vão entre o indicador e o rótulo de um item de checkbox/radio.
///
/// No original o rótulo é a **2ª coluna** de um grid, então ele começa em `ps-2 + 0.75rem + gap-2`
/// = 28px — independente de o ícone de 16px transbordar a coluna de 12px. Reproduzido em flex: o
/// ícone termina em [`INDICATOR_PAD_START`] + [`ICON_SIZE`] = 22px, então sobram 6px de vão.
const INDICATOR_GAP: f32 = ITEM_PAD_X + INDICATOR_COL + ITEM_GAP - (INDICATOR_PAD_START + ICON_SIZE);

/// Respiro no FIM de um item de checkbox/radio — `pe-4`. Maior que o do item de ação (`px-2`) de
/// propósito: é o original.
const INDICATOR_PAD_END: f32 = 16.0;

/// Corpo do texto do rótulo de grupo e do atalho — `text-xs`.
pub(crate) const SMALL_TEXT_SIZE: f32 = 12.0;

/// Entrelinha do `text-xs` — `12px/16px` no Tailwind. Mesmo motivo do [`TEXT_LINE_HEIGHT`].
pub(crate) const SMALL_LINE_HEIGHT: f32 = 16.0;

/// Respiro horizontal do rótulo de grupo — `px-2`.
pub(crate) const LABEL_PAD_X: f32 = 8.0;

/// Respiro vertical do rótulo de grupo — `py-1.5`.
pub(crate) const LABEL_PAD_Y: f32 = 6.0;

/// Margem horizontal do separador — `mx-2`.
pub(crate) const SEPARATOR_MARGIN_X: f32 = 8.0;

/// Margem vertical do separador — `my-1`.
pub(crate) const SEPARATOR_MARGIN_Y: f32 = 4.0;

/// Espessura do separador — `h-px`.
pub(crate) const SEPARATOR_HEIGHT: f32 = 1.0;

/// Opacidade de um item desabilitado — `data-disabled:opacity-64`. O coss esmaece o item inteiro em
/// vez de trocar cor por cor; assim não é preciso um par "apagado" de cada token.
pub(crate) const DISABLED_OPACITY: f32 = 0.64;

/// Distância default entre o gatilho e o popup — o `sideOffset = 4` da referência.
///
/// `pub(crate)` porque o `sideOffset` é o MESMO 4 em todo popup ancorado do coss — o menu, o menu de
/// contexto e o [`crate::tooltip::Tooltip`] escrevem `sideOffset={4}` no `Positioner`. Dois `4.0`
/// independentes seriam duas verdades pro mesmo pixel.
pub(crate) const SIDE_OFFSET: f32 = 4.0;

/// Margem que o popup guarda das bordas da janela (vai também pro `snap_to_window_with_margin`).
///
/// `pub(crate)` pelo mesmo motivo do [`SIDE_OFFSET`]: é o `collisionPadding` de todos os popups
/// ancorados desta lib, e quem gruda na janela por conta própria tem que grudar com a mesma folga.
pub(crate) const WINDOW_MARGIN: f32 = 8.0;

/// Piso da altura máxima do popup. Sem ele, um gatilho colado no rodapé abriria um menu de altura
/// ~0 — melhor estourar um pouco a margem e ser rolável do que ficar inclicável.
const MIN_MAX_HEIGHT: f32 = 96.0;

/// O check dos indicadores de checkbox **e de radio**.
///
/// ⚠️ **A referência marca o radio com um CHECK, não com uma bolinha** — o
/// `MenuRadioItemIndicator` usa o mesmo `<path d="M5.252 12.7 10.2 18.63 18.748 5.37">` do
/// checkbox. Não troque por `icons/circle_dot.svg` achando que é esquecimento.
///
/// Este arquivo (`assets/icons/check.svg`) é o MESMO que o [`crate::select::Select`] usa no popup
/// dele. É um desvio pequeno e consciente: o path da referência tem `stroke-width` 2 (≈1,33px
/// renderizado a 16px) e este tem 3 (≈2px) — o `iconoir/regular/check.svg` seria mais próximo (1,5),
/// mas dois checks de pesos diferentes em dois popups irmãos do mesmo design system é pior que 0,7px
/// de traço. Trocar é mudar esta constante.
const CHECK_ICON: &str = "icons/check.svg";

// =================================================================================================
// Posicionamento (o `Positioner` da referência, reduzido a aritmética)
// =================================================================================================

/// Um retângulo em pixels de janela — o "anchor rect" do `Positioner`, e também o tamanho medido do
/// popup. Existe pra a matemática de posição ser uma função pura testável sem GPU.
///
/// `pub(crate)` porque é o tipo de entrada de [`anchor_point`], que o [`crate::tooltip::Tooltip`]
/// também usa: a aritmética de `side`/`align`/offset é UMA nesta lib, e quem a chama precisa poder
/// falar a língua dela.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl Rect {
    /// Se o ponto está dentro (bordas inclusive no início, exclusive no fim — a regra do
    /// [`gpui::Bounds::contains`]).
    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// De que lado do gatilho o popup abre — o `side` do `Positioner`.
///
/// [`MenuSide::Left`]/[`MenuSide::Right`] são o `inline-start`/`inline-end` da referência resolvidos
/// pra LTR (ver a nota de RTL no doc do módulo).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MenuSide {
    /// Acima do gatilho.
    Top,
    /// Abaixo do gatilho — o default da referência.
    #[default]
    Bottom,
    /// À esquerda (`inline-start`).
    Left,
    /// À direita (`inline-end`) — é o lado do `MenuSubPopup`.
    Right,
}

impl MenuSide {
    /// Se o popup abre no eixo VERTICAL (acima/abaixo), caso em que o alinhamento é horizontal.
    ///
    /// `pub(crate)` porque o [`crate::tooltip::Tooltip`] decide o eixo do `max-w-(--available-width)`
    /// e o eixo do container de centralização pela mesma pergunta.
    pub(crate) fn is_vertical(self) -> bool {
        matches!(self, MenuSide::Top | MenuSide::Bottom)
    }
}

/// Como o popup se alinha ao gatilho no eixo transversal — o `align` do `Positioner`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MenuAlign {
    /// Encosta o início (esquerda, ou topo nos lados horizontais).
    Start,
    /// Centraliza — o default da referência.
    #[default]
    Center,
    /// Encosta o fim.
    End,
}

/// **Onde o popup se ancora** — a única diferença de verdade entre o [`Menu`] e o
/// [`crate::context_menu::ContextMenu`].
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(crate) enum MenuAnchoring {
    /// Nos **bounds do gatilho**: o popup nasce colado no botão que o abriu e **desliza** pra dentro
    /// da janela quando estouraria a borda. É o [`Menu`].
    #[default]
    Trigger,
    /// Na **posição do ponteiro**: o clique direito virou um retângulo de tamanho ZERO (ver
    /// [`pointer_rect`]), não há gatilho na árvore, e o popup **vira** pro outro lado em vez de
    /// deslizar (ver [`snaps_to_window`]). É o menu de contexto.
    Pointer(Point<Pixels>),
}

/// O "anchor rect" de um menu ancorado no ponteiro: um retângulo de **tamanho zero** no ponto do
/// clique.
///
/// É o que faz toda a máquina de [`anchor_point`] servir sem uma conta nova: com largura e altura
/// zero, "a quina inferior-esquerda do gatilho" **é** o ponto do ponteiro, e o canto devolvido
/// continua sendo o do POPUP — ou seja, quem resolve a posição continua sendo o [`gpui::anchored`],
/// que é o único que conhece o tamanho do popup na hora certa (ver o aviso sobre medir, no doc do
/// módulo).
fn pointer_rect(position: Point<Pixels>) -> Rect {
    Rect {
        x: f32::from(position.x),
        y: f32::from(position.y),
        w: 0.0,
        h: 0.0,
    }
}

/// Se o popup **desliza** pra dentro da janela (`true`) ou **vira pro outro lado** (`false`).
///
/// ⚠️ **No [`gpui::anchored`] os dois são EXCLUSIVOS**: `fit_mode` é um campo só, e o
/// `snap_to_window_with_margin` (deslizar, com margem) substitui o `AnchoredFitMode::SwitchAnchor`
/// default (virar o canto de ancoragem, sem margem). Não existe "vira e, se não couber de nenhum
/// lado, desliza com 8px" — é um ou outro.
///
/// - **gatilho** → desliza. Ali o `side` é uma decisão do call site ("este menu abre PRA CIMA") e
///   virar sozinho contrariaria o pedido; é o que este módulo sempre fez.
/// - **ponteiro** → vira. Um menu de contexto aberto a 20px do rodapé, se deslizasse, seria pintado
///   POR CIMA do cursor: o item sob o ponteiro nasceria destacado sem o usuário ter mexido, e o
///   próximo clique escolheria ele. Todo menu de contexto nativo cresce pro lado que tem espaço.
///
/// O `SwitchAnchor` só vira **se o resultado couber**; quando não cabe de lado nenhum ele ainda
/// gruda o popup na janela, só que com margem 0 — é o preço declarado desta escolha, e ele só
/// aparece num popup mais largo (ou mais alto) que os dois lados do ponteiro.
fn snaps_to_window(anchoring: MenuAnchoring) -> bool {
    matches!(anchoring, MenuAnchoring::Trigger)
}

/// O ponto de ancoragem do popup e **qual canto dele** encosta nesse ponto.
///
/// É o `Positioner` da referência reduzido ao que o [`gpui::anchored`] sabe fazer: `side` escolhe a
/// aresta do gatilho (deslocada por `side_offset`), e `align` escolhe a ponta dessa aresta
/// (deslocada por `align_offset`). O canto devolvido é o do POPUP que vai nesse ponto — é o que
/// dispensa conhecer o tamanho do popup, porque o `anchored` já o conhece quando resolve o canto.
///
/// `align: center` **não é um canto**: ele devolve o mesmo ponto/canto do `start`, e a
/// centralização é feita por layout (ver [`align_center_wrap`]). Grudar na janela também não está
/// aqui — disso cuida o `snap_to_window_with_margin`, que faz o papel do `collisionPadding`.
///
/// ⚠️ **É a ÚNICA aritmética de posição da lib.** O [`crate::tooltip::Tooltip`] chama esta função (é
/// o motivo do `pub(crate)`); reimplementá-la lá daria dois lugares pra corrigir quando um canto
/// estivesse trocado, e a cópia que ninguém lembrou de corrigir é a que fica errada pra sempre.
///
/// O sinal do `align_offset` é o do Base UI: **positivo empurra pro FIM** do eixo de alinhamento. É
/// o que faz o `alignOffset: -5` do `MenuSubPopup` subir o submenu 5px (o bastante pra o topo dele
/// coincidir com o topo do popup pai: 4px de `p-1` + 1px de borda).
pub(crate) fn anchor_point(
    anchor: Rect,
    side: MenuSide,
    align: MenuAlign,
    side_offset: f32,
    align_offset: f32,
) -> (f32, f32, Corner) {
    // No eixo de alinhamento, `end` parte da aresta final; `start` e `center` da inicial.
    let fim = align == MenuAlign::End;
    match side {
        MenuSide::Bottom => (
            if fim { anchor.x + anchor.w } else { anchor.x } + align_offset,
            anchor.y + anchor.h + side_offset,
            if fim { Corner::TopRight } else { Corner::TopLeft },
        ),
        MenuSide::Top => (
            if fim { anchor.x + anchor.w } else { anchor.x } + align_offset,
            anchor.y - side_offset,
            if fim { Corner::BottomRight } else { Corner::BottomLeft },
        ),
        MenuSide::Right => (
            anchor.x + anchor.w + side_offset,
            if fim { anchor.y + anchor.h } else { anchor.y } + align_offset,
            if fim { Corner::BottomLeft } else { Corner::TopLeft },
        ),
        MenuSide::Left => (
            anchor.x - side_offset,
            if fim { anchor.y + anchor.h } else { anchor.y } + align_offset,
            if fim { Corner::BottomRight } else { Corner::TopRight },
        ),
    }
}

/// Altura máxima do popup — o `max-h-(--available-height)` da referência.
///
/// Nos lados verticais é o MAIOR dos dois espaços livres (acima/abaixo), não o do lado pedido: o
/// `snap_to_window_with_margin` desliza o popup pra dentro da janela quando ele não cabe no lado
/// escolhido, e limitá-lo ao espaço do lado pedido daria um menu de 20px de altura num gatilho
/// colado no rodapé. Nos lados horizontais o popup pode usar a janela toda, menos as margens.
pub(crate) fn popup_max_height(
    side: MenuSide,
    anchor: Rect,
    viewport_height: f32,
    side_offset: f32,
) -> f32 {
    let livre = if side.is_vertical() {
        let abaixo = viewport_height - (anchor.y + anchor.h) - side_offset - WINDOW_MARGIN;
        let acima = anchor.y - side_offset - WINDOW_MARGIN;
        abaixo.max(acima)
    } else {
        viewport_height - 2.0 * WINDOW_MARGIN
    };
    livre.max(MIN_MAX_HEIGHT)
}

// =================================================================================================
// Itens
// =================================================================================================

/// Variante de cor de um item — o `variant` do `MenuItem` da referência.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MenuItemVariant {
    /// Texto normal (`text-foreground`).
    #[default]
    Default,
    /// Ação destrutiva: texto `--destructive-foreground` (red-700 no claro, red-400 no escuro).
    Destructive,
}

/// O que um [`MenuItem`] É. Privado de propósito: o que o call site precisa saber sai dos
/// construtores e dos predicados públicos (`is_activatable`, `is_checked`, `group`, …).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MenuItemKind {
    /// `MenuItem` — dispara e fecha.
    Action,
    /// `MenuCheckboxItem` — alterna e (por default) NÃO fecha.
    Checkbox { checked: bool },
    /// `MenuRadioItem` — escolhe um do `group` e (por default) NÃO fecha.
    Radio { group: usize, checked: bool },
    /// `MenuGroupLabel` — só texto, não é alcançável.
    Label,
    /// `MenuSeparator` — só um fio.
    Separator,
}

/// Uma linha do menu.
///
/// A referência compõe por JSX (`<MenuItem>`, `<MenuCheckboxItem>`, `<MenuSeparator>`, …). Aqui a
/// lista é **achatada** num `Vec<MenuItem>`, no mesmo espírito do [`crate::tabs::TabsTab`]: cada
/// componente da referência virou um **construtor**.
///
/// | referência | aqui |
/// |---|---|
/// | `<MenuItem>` | [`MenuItem::new`] |
/// | `<MenuCheckboxItem>` | [`MenuItem::checkbox`] |
/// | `<MenuRadioItem>` (dentro de `<MenuRadioGroup>`) | [`MenuItem::radio`] (com a tag de grupo) |
/// | `<MenuGroupLabel>` | [`MenuItem::group_label`] |
/// | `<MenuSeparator>` | [`MenuItem::separator`] |
/// | `<MenuShortcut>` | [`MenuItem::shortcut`] |
/// | `<MenuLinkItem>` | [`MenuItem::new`] (mesmas classes na referência) |
/// | `<MenuGroup>` | — (não tem estilo nenhum na referência; rótulo + separador expressam o grupo) |
///
/// Os construtores de decoração que não se aplicam a um tipo são **ignorados** em vez de proibidos
/// (um `.shortcut(..)` num separador não faz nada): manter a lista homogênea vale mais que um erro
/// de tipo que ninguém cometeria duas vezes.
#[derive(Clone, Debug)]
pub struct MenuItem {
    label: SharedString,
    kind: MenuItemKind,
    icon: Option<SharedString>,
    shortcut: Option<SharedString>,
    variant: MenuItemVariant,
    inset: bool,
    disabled: bool,
    close_on_click: Option<bool>,
}

impl MenuItem {
    /// Base comum dos construtores.
    fn of(kind: MenuItemKind, label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            kind,
            icon: None,
            shortcut: None,
            variant: MenuItemVariant::default(),
            inset: false,
            disabled: false,
            close_on_click: None,
        }
    }

    /// Um item de **ação** (`MenuItem`): clicar emite [`MenuEvent::Select`] e fecha o menu.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self::of(MenuItemKind::Action, label)
    }

    /// Um item de **checkbox** (`MenuCheckboxItem`): clicar alterna, emite
    /// [`MenuEvent::CheckedChange`] e **não** fecha o menu (dá pra marcar vários seguidos).
    pub fn checkbox(label: impl Into<SharedString>, checked: bool) -> Self {
        Self::of(MenuItemKind::Checkbox { checked }, label)
    }

    /// Um item de **radio** (`MenuRadioItem`): clicar desmarca os irmãos do mesmo `group`, emite
    /// [`MenuEvent::RadioChange`] e **não** fecha o menu.
    ///
    /// O `group` é a tag que substitui o `<MenuRadioGroup>` da referência: itens com o mesmo número
    /// formam um grupo, mesmo separados por rótulos e separadores.
    pub fn radio(group: usize, label: impl Into<SharedString>, checked: bool) -> Self {
        Self::of(MenuItemKind::Radio { group, checked }, label)
    }

    /// Um **rótulo de grupo** (`MenuGroupLabel`): 12px, `medium`, `--muted-foreground`. Não é
    /// clicável nem alcançável pelo teclado.
    pub fn group_label(text: impl Into<SharedString>) -> Self {
        Self::of(MenuItemKind::Label, text)
    }

    /// Um **separador** (`MenuSeparator`): um fio de 1px com `mx-2 my-1`.
    pub fn separator() -> Self {
        Self::of(MenuItemKind::Separator, SharedString::default())
    }

    /// Ícone à esquerda do rótulo (ex.: `"icons/copy.svg"`, `"iconoir/regular/trash.svg"`).
    ///
    /// Servido pela [`crate::assets::Assets`] — sem essa `AssetSource` registrada no bootstrap, o
    /// ícone some SILENCIOSAMENTE. Só vale em itens de ação (checkbox/radio já usam a coluna do
    /// indicador).
    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }

    /// O **atalho** mostrado à direita (`MenuShortcut`) — ex.: `"⌘D"`. Só texto: o menu não registra
    /// keybinding nenhum.
    pub fn shortcut(mut self, text: impl Into<SharedString>) -> Self {
        self.shortcut = Some(text.into());
        self
    }

    /// Variante de cor.
    pub fn variant(mut self, variant: MenuItemVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Açúcar pra `.variant(MenuItemVariant::Destructive)`.
    pub fn destructive(self) -> Self {
        self.variant(MenuItemVariant::Destructive)
    }

    /// **Inset**: alinha o rótulo com o dos itens que têm ícone (`data-inset:ps-8`). Serve pra uma
    /// lista mista não ficar dentada.
    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    /// Desabilita: o item não responde a clique, o teclado o pula, e ele esmaece pra 64%.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Força o comportamento de fechar (ou não) ao escolher, sobrepondo o default do tipo — o
    /// `closeOnClick` da referência. Default: ação fecha, checkbox/radio não.
    pub fn close_on_click(mut self, close: bool) -> Self {
        self.close_on_click = Some(close);
        self
    }

    /// O texto do item (ou do rótulo de grupo). Vazio num separador.
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// O caminho do ícone, se houver.
    pub fn icon_path(&self) -> Option<&SharedString> {
        self.icon.as_ref()
    }

    /// O texto do atalho, se houver.
    pub fn shortcut_text(&self) -> Option<&SharedString> {
        self.shortcut.as_ref()
    }

    /// A variante de cor.
    pub fn item_variant(&self) -> MenuItemVariant {
        self.variant
    }

    /// Se está desabilitado.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Se é `inset`.
    pub fn is_inset(&self) -> bool {
        self.inset
    }

    /// Se é um separador.
    pub fn is_separator(&self) -> bool {
        matches!(self.kind, MenuItemKind::Separator)
    }

    /// Se é um rótulo de grupo.
    pub fn is_group_label(&self) -> bool {
        matches!(self.kind, MenuItemKind::Label)
    }

    /// O estado de marcação, pros itens que têm um (checkbox e radio). `None` nos outros.
    pub fn is_checked(&self) -> Option<bool> {
        match self.kind {
            MenuItemKind::Checkbox { checked } | MenuItemKind::Radio { checked, .. } => Some(checked),
            _ => None,
        }
    }

    /// O grupo, se for um item de radio.
    pub fn group(&self) -> Option<usize> {
        match self.kind {
            MenuItemKind::Radio { group, .. } => Some(group),
            _ => None,
        }
    }

    /// Se o item é **alcançável**: clicável e navegável pelo teclado. Rótulo, separador e item
    /// desabilitado não são.
    pub fn is_activatable(&self) -> bool {
        !self.disabled
            && matches!(
                self.kind,
                MenuItemKind::Action | MenuItemKind::Checkbox { .. } | MenuItemKind::Radio { .. }
            )
    }
}

/// A máscara de itens alcançáveis — o que a navegação por teclado consulta.
fn activatable_mask(items: &[MenuItem]) -> Vec<bool> {
    items.iter().map(MenuItem::is_activatable).collect()
}

/// O próximo índice alcançável a partir de `from`, andando `dir` e **circulando**.
///
/// `from: None` (nada destacado ainda, o caso de acabar de abrir com o mouse) entra pela ponta
/// certa: o primeiro item indo pra frente, o último indo pra trás. É o que faz a primeira seta
/// depois de abrir cair no lugar óbvio.
///
/// `pub(crate)` porque o [`crate::command`] anda a lista dele com **esta** regra, e não com a do
/// [`crate::autocomplete`]: o paladar de comandos fixa `autoHighlight="always"`, e no Base UI isso
/// zera o `allowEscape` (`allowEscape: loopFocus && !autoHighlightMode`, em
/// `combobox/root/AriaCombobox.tsx`) — ou seja o ciclo dele **não** passa pelo estado "nenhum
/// destacado", exatamente como o de um menu. O `step_highlight` do [`crate::autocomplete`] é o
/// contraste: lá o ciclo tem `len + 1` estados.
pub(crate) fn step_index(from: Option<usize>, mask: &[bool], dir: isize) -> Option<usize> {
    let len = mask.len();
    if len == 0 {
        return None;
    }
    let Some(from) = from else {
        return edge_index(mask, dir < 0);
    };
    let mut i = from.min(len - 1);
    for _ in 0..len {
        i = (i as isize + dir).rem_euclid(len as isize) as usize;
        if mask[i] {
            return Some(i);
        }
    }
    None
}

/// O primeiro (ou o último) índice alcançável — o `Home`/`End`.
pub(crate) fn edge_index(mask: &[bool], last: bool) -> Option<usize> {
    if last {
        mask.iter().rposition(|m| *m)
    } else {
        mask.iter().position(|m| *m)
    }
}

/// Se escolher este item fecha o menu.
///
/// Default por tipo, e é o do Base UI (não está nas 352 linhas — a referência só torna o
/// `closeOnClick` explícito no `MenuLinkItem`): **ação fecha**, porque a ação já aconteceu;
/// **checkbox e radio não**, pra dar pra marcar vários sem reabrir. [`MenuItem::close_on_click`]
/// sobrepõe.
fn closes_on_click(kind: MenuItemKind, explicit: Option<bool>) -> bool {
    match explicit {
        Some(v) => v,
        None => !matches!(
            kind,
            MenuItemKind::Checkbox { .. } | MenuItemKind::Radio { .. }
        ),
    }
}

/// Marca o radio `idx` e desmarca os irmãos do MESMO grupo. Devolve o grupo se a escolha **mudou**
/// (`None` se `idx` não é radio, ou se já era o marcado).
///
/// Re-clicar o já marcado não muda nada — e é por isso que não emite (mesma regra do
/// `Select::choose`, que não emite ao re-escolher o índice atual).
fn apply_radio(items: &mut [MenuItem], idx: usize) -> Option<usize> {
    let group = items.get(idx)?.group()?;
    if items[idx].is_checked() == Some(true) {
        return None;
    }
    for item in items.iter_mut() {
        if let MenuItemKind::Radio { group: g, checked } = &mut item.kind {
            if *g == group {
                *checked = false;
            }
        }
    }
    if let MenuItemKind::Radio { checked, .. } = &mut items[idx].kind {
        *checked = true;
    }
    Some(group)
}

/// A cor do texto de um item.
///
/// ⚠️ **Desvio consciente, e é o único ponto de ambiguidade real da referência.** As classes do
/// `MenuItem` trazem `data-[variant=destructive]:text-destructive-foreground` **e**
/// `data-highlighted:text-accent-foreground`, com a mesma especificidade; quem ganha depende da
/// ordem no CSS gerado, que o `.tsx` não determina. Aqui o **destrutivo ganha**, por dois motivos:
///
/// 1. `--accent-foreground` é IGUAL a `--foreground` nos dois temas do coss, então
///    `data-highlighted:text-accent-foreground` não pinta nada de novo — o único efeito observável
///    dele é matar o vermelho do destrutivo;
/// 2. um "Apagar" que deixa de ser vermelho justamente quando o cursor está nele é o oposto do que
///    a variante existe pra comunicar.
///
/// Se a captura de tela do coss mostrar o contrário, é aqui que se inverte (e o teste
/// `destrutivo_vence_o_destaque` aponta pra cá).
fn item_text_color(variant: MenuItemVariant, highlighted: bool) -> Rgba8 {
    let p = palette();
    match variant {
        MenuItemVariant::Destructive => p.destructive_text,
        MenuItemVariant::Default if highlighted => p.accent_text,
        MenuItemVariant::Default => p.text,
    }
}

/// Respiro no início de um item de ação, já com o `-mx-0.5` do ícone descontado (ver [`ICON_PULL`]).
fn action_pad_start(inset: bool, has_icon: bool) -> f32 {
    let base = if inset { INSET_PAD_START } else { ITEM_PAD_X };
    if has_icon {
        base - ICON_PULL
    } else {
        base
    }
}

/// Se um mouse-down FORA do popup deve fechá-lo.
///
/// O clique no gatilho **não** fecha aqui: ele é fora do popup, então o `on_mouse_down_out`
/// dispararia, fecharia o menu, e em seguida o `on_click` do gatilho o abriria de novo — o menu
/// nunca fecharia clicando no próprio botão que o abriu. Quem trata esse clique é o gatilho, com
/// [`Menu::toggle`].
///
/// `pub(crate)` porque o [`crate::popover::Popover`] tem o MESMO gatilho de clique e a MESMA
/// armadilha: reimplementar a guarda lá daria dois lugares pra corrigir, e a cópia esquecida é a que
/// fica errada pra sempre.
pub(crate) fn outside_click_closes(trigger: Rect, x: f32, y: f32) -> bool {
    !trigger.contains(x, y)
}

// =================================================================================================
// O componente
// =================================================================================================

/// A assinatura de um gatilho montado pelo call site (ver [`Menu::trigger`]). Fica num alias porque
/// inline ela é complexa demais até pro clippy.
type TriggerRender = dyn Fn(bool, &mut Window, &mut Context<Menu>) -> AnyElement;

/// O que renderiza o gatilho.
enum MenuTrigger {
    /// Um [`Button`] com rótulo (o embutido).
    Button(SharedString),
    /// Um [`Button`] só de ícone, quadrado — o caso do menu de "…".
    IconButton(SharedString),
    /// Qualquer elemento, montado pelo call site. Recebe `open` (pra o gatilho poder reagir a estar
    /// aberto, o `data-popup-open` da referência).
    Custom(Box<TriggerRender>),
}

/// O menu suspenso (gatilho + popup de itens).
pub struct Menu {
    scroll_area: bool,
    smooth:bool,
    fade:crate::motion::Tween,
    /// As linhas, na ordem em que aparecem.
    items: Vec<MenuItem>,
    /// Se o popup está aberto.
    open: bool,
    /// O item **destacado** (`data-highlighted`), por hover OU por teclado — é um estado só, o que
    /// dá a semântica do Base UI: nunca há dois itens destacados.
    highlighted: Option<usize>,
    /// O gatilho.
    trigger: MenuTrigger,
    side: MenuSide,
    align: MenuAlign,
    side_offset: f32,
    align_offset: f32,
    /// Largura fixa do popup. `None` = largura de conteúdo com piso de [`MIN_WIDTH`] (o
    /// `not-[class*='w-']:min-w-32` da referência).
    width: Option<f32>,
    /// Onde o popup se ancora: nos bounds do gatilho, ou no ponteiro (ver [`MenuAnchoring`]).
    anchoring: MenuAnchoring,
    /// Bounds do gatilho, medidos por `canvas` no prepaint — o "anchor rect".
    ///
    /// Com [`MenuAnchoring::Pointer`] não há gatilho, e este retângulo passa a ser a **região** que
    /// responde ao clique direito (escrita de fora por [`Self::set_anchor_region`]). Ele continua
    /// tendo o mesmo papel nos dois casos: é o retângulo em que um mouse-down **não** conta como
    /// "clique fora", porque quem trata aquele clique é o gatilho/a região — ver
    /// [`outside_click_closes`].
    trigger_bounds: Bounds<Pixels>,
    /// A caixa do popup, medida por `canvas` no prepaint — a BORDER box, em pixels de janela.
    ///
    /// **Não** participa do posicionamento (ver o doc do módulo: posicionar pela medida punha o
    /// popup no lugar errado no primeiro frame). É observabilidade: quem embute o menu pode querer
    /// saber onde ele abriu, e é contra ela que o teste de janela confere a geometria declarada.
    popup_bounds: Option<Bounds<Pixels>>,
    /// Id estável desta entidade (pra `div().id(..)` único na árvore).
    id: u64,
    focus_handle: FocusHandle,
    /// Rolagem da lista. Persiste entre renders: sem ela, cada `cx.notify()` (um hover, por
    /// exemplo) devolveria a lista pro topo no meio da rolagem.
    list_scroll: ScrollHandle,
}

impl Menu {
    /// Cria um menu com `items`, fechado, com o gatilho embutido (um [`Button`] rotulado `Menu` —
    /// troque com [`Self::trigger_button`] ou [`Self::trigger`]).
    pub fn new(items: Vec<MenuItem>, cx: &mut Context<Self>) -> Self {
        Self {
            scroll_area: false,
            items,
            open: false,
            smooth:false,fade:crate::motion::Tween::new(0.),
            highlighted: None,
            trigger: MenuTrigger::Button(SharedString::from("Menu")),
            side: MenuSide::default(),
            align: MenuAlign::default(),
            side_offset: SIDE_OFFSET,
            align_offset: 0.0,
            width: None,
            anchoring: MenuAnchoring::default(),
            trigger_bounds: Bounds::default(),
            popup_bounds: None,
            id: cx.entity_id().as_u64(),
            focus_handle: cx.focus_handle(),
            list_scroll: ScrollHandle::new(),
        }
    }

    /// Gatilho = um [`Button`] `Outline` com este rótulo.
    pub fn trigger_button(mut self, label: impl Into<SharedString>) -> Self {
        self.trigger = MenuTrigger::Button(label.into());
        self
    }

    /// Gatilho = um [`Button`] quadrado só com este ícone (ex.: `"iconoir/regular/more-horiz.svg"`).
    pub fn trigger_icon_button(mut self, path: impl Into<SharedString>) -> Self {
        self.trigger = MenuTrigger::IconButton(path.into());
        self
    }

    /// Gatilho = o que você montar. Recebe `open` e devolve o elemento.
    ///
    /// O clique é tratado pelo **wrap** que o `Menu` põe em volta, não pelo seu elemento: não
    /// registre `on_click` pra abrir, ou o menu abre e fecha no mesmo clique.
    pub fn trigger(
        mut self,
        render: impl Fn(bool, &mut Window, &mut Context<Self>) -> AnyElement + 'static,
    ) -> Self {
        self.trigger = MenuTrigger::Custom(Box::new(render));
        self
    }

    /// De que lado do gatilho o popup abre (default [`MenuSide::Bottom`]).
    pub fn side(mut self, side: MenuSide) -> Self {
        self.side = side;
        self
    }

    /// Como o popup se alinha ao gatilho (default [`MenuAlign::Center`]).
    pub fn align(mut self, align: MenuAlign) -> Self {
        self.align = align;
        self
    }

    /// Distância entre o gatilho e o popup (default 4px).
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = offset;
        self
    }

    /// Deslocamento no eixo de **alinhamento** (default 0). Positivo empurra pro fim do eixo.
    pub fn align_offset(mut self, offset: f32) -> Self {
        self.align_offset = offset;
        self
    }

    /// Largura fixa do popup, em px. Sem isto ele tem a largura do conteúdo, com piso de 128px.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// As linhas do menu.
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    /// Se o popup está aberto.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// O item destacado, se algum.
    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    /// O índice do item marcado de um grupo de radio, se houver.
    pub fn radio_selected(&self, group: usize) -> Option<usize> {
        self.items
            .iter()
            .position(|i| i.group() == Some(group) && i.is_checked() == Some(true))
    }

    /// Close immediately when another popup takes its place.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.set_open(false,cx);self.fade=crate::motion::Tween::new(0.);cx.notify();
    }
    /// Enable interruptible entrance and exit fades. Off by default.
    pub fn motion(mut self, enabled:bool)->Self {self.smooth=enabled;self}

    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        self.open = open;
        self.fade.set(if open {1.} else {0.},160);
        if !open {
            // Fechar limpa o destaque: reabrir tem que começar do zero, senão o item que o cursor
            // tocou por último volta aceso sem o cursor estar nele.
            self.highlighted = None;
        }
        cx.emit(MenuEvent::OpenChange(open));
        cx.notify();
    }

    /// Alterna o popup (o clique no gatilho).
    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.set_open(!self.open, cx);
    }

    // --- Ancoragem no ponteiro (o que o `crate::context_menu` usa) -------------------------------
    //
    // Tudo aqui é `pub(crate)`: é a costura entre os dois módulos, não API de quem usa o crate. Quem
    // usa monta um menu de contexto por `crate::context_menu::ContextMenu::menu`.

    /// Torna este menu um menu de **contexto**: sem gatilho, ancorado no ponteiro.
    pub(crate) fn pointer_anchored(mut self) -> Self {
        self.anchoring = MenuAnchoring::Pointer(Point::default());
        self
    }

    /// Se este menu é ancorado no ponteiro.
    fn is_pointer_anchored(&self) -> bool {
        matches!(self.anchoring, MenuAnchoring::Pointer(_))
    }

    /// Abre (ou **reposiciona**) o popup no ponto dado, em pixels de janela.
    ///
    /// Já aberto, reposicionar **não** emite [`MenuEvent::OpenChange`] — o menu não fechou nem
    /// abriu, ele se mudou. É o que faz o segundo clique direito, em outro ponto da mesma região,
    /// mover o popup em vez de sair um par fecha/abre no log de quem assina.
    ///
    /// O destaque é limpo: reabrir noutro lugar começa do zero, senão o item que o cursor tocou por
    /// último volta aceso longe do cursor.
    pub(crate) fn open_at(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        self.anchoring = MenuAnchoring::Pointer(position);
        self.highlighted = None;
        if self.open {
            cx.notify();
        } else {
            self.set_open(true, cx);
        }
    }

    /// A **região** que responde ao clique direito, medida por quem a desenha.
    ///
    /// Ver [`Self::trigger_bounds`]: é o retângulo em que um mouse-down não conta como clique fora.
    /// Sem isto, o clique direito de reposicionar cairia como "fora", fecharia o popup, e só depois
    /// a região o reabriria — com um par de eventos espúrio.
    pub(crate) fn set_anchor_region(&mut self, bounds: Bounds<Pixels>) {
        self.trigger_bounds = bounds;
    }

    /// Troca a lista de itens. Sincronização externa: **não** emite.
    pub fn set_items(&mut self, items: Vec<MenuItem>, cx: &mut Context<Self>) {
        self.items = items;
        self.highlighted = None;
        cx.notify();
    }

    /// Draw the design system scrollbar for long command menus.
    pub fn scroll_area(mut self, enabled: bool) -> Self { self.scroll_area = enabled; self }

    /// Navigate a menu from an associated text input without moving its focus.
    pub fn navigate(&mut self, direction: isize, cx: &mut Context<Self>) {
        if !self.open { return; }
        let target = step_index(self.highlighted, &activatable_mask(&self.items), direction);
        if let Some(index) = target { self.scroll_highlight(index); }
        self.highlight(target, cx);
    }

    fn scroll_highlight(&self, index: usize) {
        if !self.scroll_area { self.list_scroll.scroll_to_item(index); return; }
        let row_height = |item: &MenuItem| match item.kind {
            MenuItemKind::Label => SMALL_LINE_HEIGHT + LABEL_PAD_Y * 2.,
            MenuItemKind::Separator => SEPARATOR_HEIGHT + SEPARATOR_MARGIN_Y * 2.,
            _ => ITEM_MIN_HEIGHT,
        };
        let top: f32 = self.items[..index].iter().map(row_height).sum();
        let bottom = top + row_height(&self.items[index]);
        let offset = self.list_scroll.offset();
        let visible_top = -f32::from(offset.y);
        let height = f32::from(self.list_scroll.bounds().size.height);
        let target = if top < visible_top { top } else if bottom > visible_top + height { bottom - height } else { visible_top };
        self.list_scroll.set_offset(point(offset.x, px(-target.max(0.))));
    }

    /// Activate the highlighted item (or the first enabled result).
    pub fn choose_highlighted(&mut self, cx: &mut Context<Self>) {
        if !self.open { return; }
        if let Some(index) = self.highlighted.or_else(|| edge_index(&activatable_mask(&self.items), false)) {
            self.activate(index, cx);
        }
    }

    /// Onde o popup foi medido no último frame em que esteve aberto (BORDER box, em pixels de
    /// janela). `None` enquanto ele nunca abriu.
    pub fn popup_bounds(&self) -> Option<Bounds<Pixels>> {
        self.popup_bounds
    }

    /// Marca/desmarca um item de checkbox de fora. **Não** emite (evita loop de feedback com quem
    /// assina — mesma regra do `Switch::set_on`). Ignora índices que não são checkbox.
    pub fn set_checked(&mut self, index: usize, checked: bool, cx: &mut Context<Self>) {
        if let Some(item) = self.items.get_mut(index) {
            if let MenuItemKind::Checkbox { checked: c } = &mut item.kind {
                if *c == checked {
                    return;
                }
                *c = checked;
                cx.notify();
            }
        }
    }

    /// Escolhe um item de radio de fora (desmarcando os irmãos do grupo). **Não** emite.
    pub fn select_radio(&mut self, index: usize, cx: &mut Context<Self>) {
        if apply_radio(&mut self.items, index).is_some() {
            cx.notify();
        }
    }

    /// Escolhe um item por interação: dispara o efeito do tipo dele, emite, e fecha se for o caso.
    fn activate(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(item) = self.items.get(index) else {
            return;
        };
        if !item.is_activatable() {
            return;
        }
        let (kind, explicit) = (item.kind, item.close_on_click);

        match kind {
            MenuItemKind::Action => cx.emit(MenuEvent::Select(index)),
            MenuItemKind::Checkbox { checked } => {
                let novo = !checked;
                self.items[index].kind = MenuItemKind::Checkbox { checked: novo };
                cx.emit(MenuEvent::CheckedChange {
                    index,
                    checked: novo,
                });
            }
            MenuItemKind::Radio { .. } => {
                if let Some(group) = apply_radio(&mut self.items, index) {
                    cx.emit(MenuEvent::RadioChange { index, group });
                }
            }
            MenuItemKind::Label | MenuItemKind::Separator => return,
        }

        if closes_on_click(kind, explicit) {
            // Return focus immediately after an action; never leave a fading menu over its result.
            if self.smooth {self.dismiss(cx);} else {self.set_open(false,cx);}
        }
        cx.notify();
    }

    /// Destaca um item (hover ou teclado), sem repetir `notify` à toa.
    fn highlight(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.highlighted == index {
            return;
        }
        self.highlighted = index;
        cx.notify();
    }

    /// O teclado. Ouvido pela RAIZ (o evento sobe do elemento focado pelos ancestrais), então um
    /// listener cobre o gatilho e o popup.
    ///
    /// - **fechado**: `Enter`/`Space`/`↓` abrem com o **primeiro** item destacado, `↑` com o último
    ///   — é o que o Base UI faz, e é o que permite operar o menu sem tocar no mouse.
    /// - **aberto**: `↑`/`↓` andam (circulando e pulando rótulo, separador e desabilitado),
    ///   `Home`/`End` vão pras pontas, `Enter`/`Space` escolhem o destacado, `Escape` fecha.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let mask = activatable_mask(&self.items);

        if !self.open {
            // Um menu de CONTEXTO não abre por tecla: ele não tem gatilho, e o único ponto de
            // ancoragem que ele conhece é o do último clique direito. Abrir ali seria ressuscitar o
            // menu num lugar que o usuário já dispensou (e, se nunca houve clique, na origem da
            // janela). Quem abre um menu de contexto é o ponteiro.
            if self.is_pointer_anchored() {
                return;
            }
            let alvo = match key {
                "enter" | "space" | "down" => edge_index(&mask, false),
                "up" => edge_index(&mask, true),
                _ => return,
            };
            crate::focus_ring::keyboard_used(window);
            self.highlighted = alvo;
            self.set_open(true, cx);
            return;
        }

        // Chegou aqui com o menu aberto: as teclas abaixo são as que o menu trata, ou seja o usuário
        // está navegando pelo teclado. O `focus_ring::init` já marcaria isso; marcar aqui faz as
        // setas funcionarem mesmo num app que esqueceu de inicializar o crate.
        let destino = match key {
            "escape" => {
                crate::focus_ring::keyboard_used(window);
                self.set_open(false, cx);
                return;
            }
            "enter" | "space" => {
                crate::focus_ring::keyboard_used(window);
                if let Some(i) = self.highlighted {
                    self.activate(i, cx);
                }
                return;
            }
            "down" => step_index(self.highlighted, &mask, 1),
            "up" => step_index(self.highlighted, &mask, -1),
            "home" => edge_index(&mask, false),
            "end" => edge_index(&mask, true),
            _ => return,
        };

        let Some(destino) = destino else { return };
        crate::focus_ring::keyboard_used(window);
        // Num menu mais alto que o espaço livre, andar com as setas tem que trazer o item pra
        // dentro da vista — o índice do filho na lista rolável é o próprio índice do item.
        self.scroll_highlight(destino);
        self.highlight(Some(destino), cx);
    }

    /// O gatilho, dentro do wrap que mede os bounds e trata o clique.
    fn render_trigger(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match &self.trigger {
            MenuTrigger::Button(label) => {
                // O `focus_handle` vai pro Button de propósito: é ELE que fica na ordem de
                // tabulação e desenha o próprio anel `focus-visible` (ver `crate::focus_ring`),
                // então o menu não precisa reinventar anel nenhum — e a referência não dá estilo
                // nenhum ao `MenuTrigger`, o visual dele é do botão.
                Button::new(("menu-trigger", self.id), label.clone())
                    .variant(ButtonVariant::Outline)
                    .focus(&self.focus_handle)
                    .into_any_element()
            }
            MenuTrigger::IconButton(path) => Button::icon(("menu-trigger", self.id), path.clone())
                .variant(ButtonVariant::Outline)
                .focus(&self.focus_handle)
                .into_any_element(),
            MenuTrigger::Custom(render) => render(self.open, window, cx),
        };

        div()
            .id(("menu-trigger-wrap", self.id))
            // `relative` porque o canvas de medida é filho ABSOLUTO; `flex_none` pra o wrap ter a
            // largura do gatilho e não a da linha inteira — a largura errada aqui desalinharia
            // `align: center`, que mede o gatilho.
            .relative()
            .flex()
            .flex_none()
            .child(content)
            .child(
                canvas(
                    {
                        let view = cx.entity();
                        move |bounds, _window, cx| {
                            view.update(cx, |this, _| this.trigger_bounds = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_click(cx.listener(|this, _e, window, cx| {
                // Abrir com o mouse não deve acender anel (ver `crate::focus_ring`).
                crate::focus_ring::pointer_used(window);
                this.toggle(cx);
            }))
    }

    /// Uma linha do menu.
    fn render_item(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let p = palette();
        let item = &self.items[index];

        match item.kind {
            // --- Separador: `mx-2 my-1 h-px bg-border` ------------------------------------------
            MenuItemKind::Separator => div()
                .h(px(SEPARATOR_HEIGHT))
                .mx(px(SEPARATOR_MARGIN_X))
                .my(px(SEPARATOR_MARGIN_Y))
                .bg(p.border.hsla())
                .into_any_element(),

            // --- Rótulo de grupo: `px-2 py-1.5 font-medium text-muted-foreground text-xs` -------
            MenuItemKind::Label => div()
                .pl(px(if item.inset {
                    INSET_PAD_START
                } else {
                    LABEL_PAD_X
                }))
                .pr(px(LABEL_PAD_X))
                .py(px(LABEL_PAD_Y))
                .text_size(px(SMALL_TEXT_SIZE))
                .line_height(px(SMALL_LINE_HEIGHT))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted.hsla())
                .child(item.label.clone())
                .into_any_element(),

            MenuItemKind::Action => self.render_row(index, RowSlot::Icon, cx),
            MenuItemKind::Checkbox { checked } | MenuItemKind::Radio { checked, .. } => {
                self.render_row(index, RowSlot::Indicator(checked), cx)
            }
        }
    }

    /// O corpo comum de um item clicável: o `MenuItem` (com ícone opcional) e os de
    /// checkbox/radio (com a coluna do indicador) só diferem no respiro e no primeiro filho.
    fn render_row(&self, index: usize, slot: RowSlot, cx: &mut Context<Self>) -> AnyElement {
        let p = palette();
        let item = &self.items[index];
        let disabled = item.disabled;
        // Um item desabilitado nunca acende: `data-disabled` e `data-highlighted` não convivem no
        // Base UI, e o teclado já o pula.
        let highlighted = !disabled && self.highlighted == Some(index);
        let text = item_text_color(item.variant, highlighted);

        let (pad_start, pad_end, gap) = match slot {
            RowSlot::Icon => (
                action_pad_start(item.inset, item.icon.is_some()),
                ITEM_PAD_X,
                if item.icon.is_some() {
                    ICON_GAP
                } else {
                    ITEM_GAP
                },
            ),
            RowSlot::Indicator(_) => (INDICATOR_PAD_START, INDICATOR_PAD_END, INDICATOR_GAP),
        };

        let mut el = div()
            .id(("menu-item", index))
            .flex()
            .items_center()
            .w_full()
            .min_h(px(ITEM_MIN_HEIGHT))
            .py(px(ITEM_PAD_Y))
            .pl(px(pad_start))
            .pr(px(pad_end))
            .gap(px(gap))
            .rounded(px(ITEM_RADIUS))
            .text_size(px(TEXT_SIZE))
            // Ver [`TEXT_LINE_HEIGHT`]: é ela que faz o item fechar em 28px em vez de 30,6.
            .line_height(px(TEXT_LINE_HEIGHT))
            .text_color(text.hsla())
            // `data-highlighted:bg-accent`. O item MARCADO não muda de fundo — quem o marca é o
            // check, como no original.
            .when(highlighted, |d| d.bg(p.accent.hsla()))
            // `data-disabled:opacity-64`: esmaece o item inteiro, sem par de tokens "apagados".
            .when(disabled, |d| d.opacity(DISABLED_OPACITY));

        // --- Primeiro filho: ícone (item de ação) ou a coluna do indicador -----------------------
        match slot {
            RowSlot::Icon => {
                if let Some(path) = item.icon.clone() {
                    el = el.child(
                        svg()
                            .path(path)
                            .size(px(ICON_SIZE))
                            .flex_none()
                            .text_color(text.scaled(ICON_OPACITY)),
                    );
                }
            }
            RowSlot::Indicator(checked) => {
                // A coluna existe SEMPRE, vazia quando o item não está marcado — é o que mantém
                // todos os rótulos alinhados em vez de deslocar o do item marcado.
                el = el.child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(ICON_SIZE))
                        .when(checked, |d| {
                            d.child(
                                svg()
                                    .path(CHECK_ICON)
                                    .size(px(ICON_SIZE))
                                    .flex_none()
                                    .text_color(text.hsla()),
                            )
                        }),
                );
            }
        }

        // --- Rótulo -----------------------------------------------------------------------------
        //
        // `flex_grow` (e não `flex_1`) porque o `flex-basis: 0` do `flex_1` embaralha a largura de
        // conteúdo do popup: aqui o rótulo tem que contribuir com o tamanho dele pra o popup nascer
        // com a largura do item mais largo, e só então crescer pro que sobrar. É o que empurra o
        // atalho pra direita — o `ms-auto` do `MenuShortcut`, que o GPUI não tem (não há
        // `margin: auto`).
        el = el.child(div().flex_grow().child(item.label.clone()));

        // --- Atalho: `ms-auto font-medium text-xs text-muted-foreground/72` ---------------------
        //
        // Duas notas de fidelidade:
        //
        // - sem o `tracking-widest` (0.1em) da referência: o GPUI não expõe letter-spacing;
        // - **só no item de ação**. No coss os itens de checkbox/radio são um grid de DUAS colunas
        //   (indicador + rótulo), sem lugar pra um atalho; um `.shortcut(..)` neles é ignorado, e não
        //   inventado numa terceira coluna que a referência não tem.
        if let Some(shortcut) = item.shortcut.clone() {
            if matches!(slot, RowSlot::Icon) {
                el = el.child(
                    div()
                        .flex_none()
                        .text_size(px(SMALL_TEXT_SIZE))
                        .line_height(px(SMALL_LINE_HEIGHT))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.shortcut.hsla())
                        .child(shortcut),
                );
            }
        }

        if !disabled {
            el = el
                // O destaque de hover e o de teclado são o MESMO estado, então nunca há dois itens
                // acesos (a semântica do Base UI). Ao sair, só apaga se o item que saiu é o que
                // está aceso: a ordem dos dois eventos ao passar de um item pro vizinho não é
                // garantida, e sem essa guarda o par (entra 3, sai 2) apagaria o 3.
                .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                    if *hovered {
                        this.highlight(Some(index), cx);
                    } else if this.highlighted == Some(index) {
                        this.highlight(None, cx);
                    }
                }))
                .on_click(cx.listener(move |this, _e, window, cx| {
                    crate::focus_ring::pointer_used(window);
                    this.activate(index, cx);
                }));
        }

        el.into_any_element()
    }

    /// O popup — em `deferred(anchored(..))` pra ficar por cima de tudo e não estourar a janela.
    fn render_popup(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette();
        // O "anchor rect" do `Positioner`: os bounds do gatilho, ou um retângulo de tamanho zero na
        // posição do ponteiro. Daqui pra frente o resto do método não sabe a diferença — é isso que
        // faz o menu de contexto reusar este desenho inteiro.
        let anchor = match self.anchoring {
            MenuAnchoring::Trigger => rect_of(self.trigger_bounds),
            MenuAnchoring::Pointer(position) => pointer_rect(position),
        };
        let max_h = popup_max_height(
            self.side,
            anchor,
            f32::from(window.viewport_size().height),
            self.side_offset,
        );

        let (x, y, corner) = anchor_point(
            anchor,
            self.side,
            self.align,
            self.side_offset,
            self.align_offset,
        );

        let items = (0..self.items.len())
            .map(|i| self.render_item(i, cx))
            .collect::<Vec<_>>();

        // A superfície. ⚠️ NÃO tem `overflow_hidden`: recortaria o bisel (que é filho absoluto
        // sobre a borda). Quem rola é a lista de dentro.
        let mut surface = div()
            .opacity(if self.smooth {self.fade.value()} else {1.})
            .when(!self.open,|d|d.capture_any_mouse_down(cx.listener(|this,event:&MouseDownEvent,_,cx| {if this.popup_bounds.is_some_and(|b|b.contains(&event.position)) {cx.stop_propagation();}})))
            .relative()
            // `occlude`: captura o mouse, não vaza clique pro conteúdo atrás.
            .occlude()
            .flex()
            .flex_col()
            .flex_none()
            .bg(p.popover_bg.hsla())
            .border(px(BORDER))
            .border_color(p.border.hsla())
            .rounded(px(RADIUS))
            // Sombra EXTERNA atrás de um fundo OPACO: aqui a armadilha do `paint_shadows` (que não
            // recorta a sombra pra fora do elemento) não morde — o retângulo cheio fica escondido
            // pelo `--popover`.
            .shadow(popup_shadow());
        surface = match self.width {
            Some(w) => surface.w(px(w)),
            None => surface.min_w(px(MIN_WIDTH)),
        };

        let list: AnyElement = if self.scroll_area {
            crate::scroll_area::ScrollArea::new(("menu-scroll", self.id), &self.list_scroll)
                .fade(false).radius(RADIUS)
                .w_full().h(px((self.items.len() as f32 * 28. + LIST_PAD * 2.).min(max_h.min(360.))))
                .children(items).into_any_element()
        } else {
            div().id(("menu-list", self.id)).flex().flex_col().w_full()
                .p(px(LIST_PAD)).max_h(px(max_h)).overflow_y_scroll()
                .track_scroll(&self.list_scroll).children(items).into_any_element()
        };
        let surface = surface.child(list)
            .child(bevel_overlay())
            .on_mouse_down_out(cx.listener(|this, e: &MouseDownEvent, _window, cx| {
                let pos = e.position;
                if outside_click_closes(
                    rect_of(this.trigger_bounds),
                    f32::from(pos.x),
                    f32::from(pos.y),
                ) {
                    this.set_open(false, cx);
                }
            }));

        // O wrap existe pra MEDIR: ele não tem borda nem respiro, então a caixa dele coincide com a
        // BORDER box do popup — medir por dentro da superfície daria a padding box, 2px menor.
        // A medida não posiciona nada (ver o doc do módulo); ela é observabilidade, e é contra ela
        // que o teste de janela confere a geometria declarada nas constantes.
        let wrap = div()
            .relative()
            .flex()
            .flex_none()
            .child(surface)
            .child(
                canvas(
                    {
                        let view = cx.entity();
                        move |bounds, _window, cx| {
                            view.update(cx, |this, _| this.popup_bounds = Some(bounds));
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );

        // `align: center` não é um canto: sai por LAYOUT, no container compartilhado com o
        // [`crate::tooltip::Tooltip`] — ver [`align_center_wrap`].
        let child: AnyElement = if self.align == MenuAlign::Center {
            align_center_wrap(self.side, anchor, wrap.into_any_element()).into_any_element()
        } else {
            wrap.into_any_element()
        };

        let mut ancora = anchored()
            .position(point(px(x), px(y)))
            // O canto do POPUP que encosta no ponto — é ele que expressa `align: start`/`end` e
            // `side: top`/`left` sem precisar do tamanho do popup.
            .anchor(corner);
        // Deslizar ou virar: ver [`snaps_to_window`], que explica por que os dois não convivem. O
        // default do `anchored` (sem esta chamada) é `SwitchAnchor`, que VIRA o canto.
        if snaps_to_window(self.anchoring) {
            // O `collisionPadding` do `Positioner`: gruda na janela em vez de trocar de lado —
            // `side` aqui é uma decisão do call site, não uma preferência.
            ancora = ancora.snap_to_window_with_margin(px(WINDOW_MARGIN));
        }

        deferred(ancora.child(child)).with_priority(1)
    }
}

/// O que ocupa a primeira coluna de um item clicável.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RowSlot {
    /// Item de ação: ícone opcional, respiro `px-2`.
    Icon,
    /// Item de checkbox/radio: a coluna do indicador (com o check quando marcado), respiro
    /// `ps-2 pe-4`.
    Indicator(bool),
}

impl EventEmitter<MenuEvent> for Menu {}

impl Focusable for Menu {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Menu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.smooth && self.fade.moving() {window.request_animation_frame();}
        let open = self.open;
        // Ancorado no ponteiro (menu de contexto) NÃO há gatilho: quem abre é a região do
        // `crate::context_menu`, que embrulha os filhos de quem chama. Sem gatilho esta raiz fica
        // 0×0 (e a região a hospeda fora do fluxo), então ela não perturba o layout de ninguém.
        let sem_gatilho = self.is_pointer_anchored();
        // Com o gatilho embutido, o `focus_handle` está no `Button` (que o rastreia e desenha o
        // anel). Com gatilho customizado — ou sem gatilho — ninguém o rastreia, então a RAIZ
        // rastreia: senão o menu sairia da ordem de tabulação e o teclado não chegaria nele.
        let custom_trigger = matches!(self.trigger, MenuTrigger::Custom(_));
        let trigger = (!sem_gatilho).then(|| self.render_trigger(window, cx));

        let mut root = div()
            .id(("menu", self.id))
            .relative()
            .flex()
            // `items_start` pra o wrap do gatilho ter a ALTURA dele (o `stretch` do flex esticaria
            // o wrap, e a medida do anchor rect sairia alta).
            .items_start()
            .when(custom_trigger || sem_gatilho, |d| {
                d.track_focus(&self.focus_handle)
            })
            // O teclado é ouvido pela RAIZ: o evento sobe do elemento focado pelos ancestrais,
            // então um listener cobre o gatilho (fechado) e a lista (aberto).
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                this.on_key(e, window, cx);
            }))
            .children(trigger);

        if open || (self.smooth && self.fade.moving()) {
            root = root.child(self.render_popup(window, cx));
        }

        root
    }
}

// =================================================================================================
// Peças visuais (funções livres, pra serem testáveis sem construir um `Menu`)
// =================================================================================================

/// Os `bounds` do GPUI como [`Rect`] de f32 — a fronteira entre o layout e a aritmética testável.
pub(crate) fn rect_of(bounds: Bounds<Pixels>) -> Rect {
    Rect {
        x: f32::from(bounds.origin.x),
        y: f32::from(bounds.origin.y),
        w: f32::from(bounds.size.width),
        h: f32::from(bounds.size.height),
    }
}

/// O container que expressa **`align: center`** — o único dos três alinhamentos que não é um canto.
///
/// Ver o doc do módulo: `center` sai por LAYOUT, não por conta. O popup vai dentro de um container do
/// tamanho do gatilho **no eixo transversal**, com `justify-center`; como o popup é `flex-none` e em
/// geral maior que o gatilho nesse eixo, o espaço livre é negativo e ele transborda **igualmente pros
/// dois lados** — que é a definição de centralizado. Não entra nenhuma medida do popup, e é
/// justamente isso que faz o primeiro frame já sair no lugar.
///
/// O `max(1)` cobre o frame anterior à medida do gatilho: um container de 0px centralizaria na quina.
///
/// `pub(crate)` porque o [`crate::tooltip::Tooltip`] centraliza do mesmo jeito. É layout, e não
/// aritmética, mas é layout que **substitui** aritmética — duas cópias divergiriam no dia em que
/// alguém trocasse o `flex_none` por `flex_1` numa delas.
pub(crate) fn align_center_wrap(side: MenuSide, anchor: Rect, child: AnyElement) -> Div {
    let centro = div().flex().flex_none().justify_center();
    if side.is_vertical() {
        centro.w(px(anchor.w.max(1.0))).child(child)
    } else {
        centro.flex_col().h(px(anchor.h.max(1.0))).child(child)
    }
}

/// A sombra externa do popup — `shadow-lg/5`, as duas camadas do `shadow-lg` do Tailwind com a cor
/// trocada por preto a 5% (é o que o `/5` faz).
pub(crate) fn popup_shadow() -> Vec<gpui::BoxShadow> {
    let cor = palette().shadow.hsla();
    vec![
        gpui::BoxShadow {
            color: cor,
            offset: point(px(0.0), px(10.0)),
            blur_radius: px(15.0),
            spread_radius: px(-3.0),
        },
        gpui::BoxShadow {
            color: cor,
            offset: point(px(0.0), px(4.0)),
            blur_radius: px(6.0),
            spread_radius: px(-4.0),
        },
    ]
}

/// O **fio de bisel** de 1px sobre a borda do popup.
///
/// ⚠️ **A técnica do coss não traduz pro GPUI.** Lá é um pseudo-elemento transparente com
/// `box-shadow: 0 ±1px <cor>`: só o filete que ESCAPA da caixa fica visível, porque o CSS nunca
/// pinta a sombra por baixo da border box de quem a projeta. O [`gpui::Window::paint_shadows`]
/// insere a sombra como um retângulo arredondado **completo**, sem recortar a área do próprio
/// elemento — num overlay transparente isso viraria uma lavagem de cor sobre o popup INTEIRO.
///
/// Então o filete é desenhado como o que ele é: uma **borda de 1px num único lado** de um overlay
/// absoluto. Duas consequências que já custaram defeito nesta base:
///
/// - o overlay cobre a **BORDER box** (daí o `inset: -1px` a partir da padding box), e por isso o
///   raio dele é o da SUPERFÍCIE — **não** `raio − 1`, que é o que o `.tsx` diz porque lá o
///   pseudo-elemento fica por DENTRO da borda;
/// - o **lado** sai do SINAL do deslocamento da sombra: `0 1px` desce, então o fio é na BASE
///   (`border_b`); `0 -1px` sobe, então é no TOPO.
pub(crate) fn bevel_overlay() -> Div {
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
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `select.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Um retângulo de gatilho qualquer, pros testes de posição: 100×32 em (200, 300).
    const GATILHO: Rect = Rect {
        x: 200.0,
        y: 300.0,
        w: 100.0,
        h: 32.0,
    };

    // --- Paleta ---------------------------------------------------------------------------------

    /// **A convenção de cor da paleta, decodificada de verdade.**
    ///
    /// Todo valor de [`MenuPalette`] é `0xRRGGBBAA` e é consumido por `rgba`; um valor de 6 dígitos
    /// esquecido ali vira uma cor completamente diferente, **sem erro de compilação** —
    /// `rgba(0xffffff)` é lido como `0x00FFFFFF`, ou seja ciano. Em vez de comparar números com
    /// números (que não pegaria nada), este teste decodifica e afirma o que a cor DEVE ser
    /// perceptualmente.
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        // O fundo do popup é OPACO nos dois temas — é o que torna a sombra externa segura.
        for (nome, c) in [
            ("popup claro", MENU_LIGHT.popover_bg),
            ("popup escuro", MENU_DARK.popover_bg),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome} tem que ser opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome} é NEUTRO: se r≠g≠b, o valor foi lido deslocado"
            );
        }
        let claro: gpui::Rgba = MENU_LIGHT.popover_bg.hsla().into();
        let escuro: gpui::Rgba = MENU_DARK.popover_bg.hsla().into();
        assert!(claro.r > 0.9, "o popup do tema claro é claro");
        assert!(escuro.r < 0.2, "o popup do tema escuro é escuro");
        // `--background` do tema escuro é #141414 (20/255): o popup (#1d1d1d) LEVANTA sobre ele. Se
        // o `--popover` fosse copiado do `--background`, o menu sumiria contra o painel.
        assert!(
            escuro.r > 20.0 / 255.0,
            "o popup escuro é mais claro que o --background (#141414)"
        );

        // Textos: neutros e opacos; o rótulo de grupo mais apagado que o item.
        for (nome, c) in [
            ("texto claro", MENU_LIGHT.text),
            ("texto escuro", MENU_DARK.text),
            ("muted claro", MENU_LIGHT.muted),
            ("muted escuro", MENU_DARK.muted),
            ("accent-fg claro", MENU_LIGHT.accent_text),
            ("accent-fg escuro", MENU_DARK.accent_text),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome} é NEUTRO: se r≠g≠b, o valor foi lido deslocado"
            );
        }
        let txt_claro: gpui::Rgba = MENU_LIGHT.text.hsla().into();
        let muted_claro: gpui::Rgba = MENU_LIGHT.muted.hsla().into();
        assert!(txt_claro.r < 0.2, "texto do claro é quase preto");
        assert!(
            muted_claro.r > txt_claro.r,
            "no claro o rótulo de grupo é MAIS claro que o item"
        );
        let txt_escuro: gpui::Rgba = MENU_DARK.text.hsla().into();
        let muted_escuro: gpui::Rgba = MENU_DARK.muted.hsla().into();
        assert!(txt_escuro.r > 0.8, "texto do escuro é quase branco");
        assert!(
            muted_escuro.r < txt_escuro.r,
            "no escuro o rótulo de grupo é MAIS escuro que o item"
        );

        // O destrutivo é VERMELHO de verdade nos dois temas (r bem acima de g e b), e o do escuro é
        // mais CLARO que o do claro (red-400 contra red-700) — se estivessem trocados, o texto
        // ficaria ilegível em cima do popup.
        for (nome, c) in [
            ("destrutivo claro", MENU_LIGHT.destructive_text),
            ("destrutivo escuro", MENU_DARK.destructive_text),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: opaco");
            assert!(
                c.r > 0.6 && c.r > c.g + 0.3 && c.r > c.b + 0.3,
                "{nome} é VERMELHO"
            );
        }
        let d_claro: gpui::Rgba = MENU_LIGHT.destructive_text.hsla().into();
        let d_escuro: gpui::Rgba = MENU_DARK.destructive_text.hsla().into();
        assert!(
            d_escuro.r > d_claro.r,
            "red-400 (escuro) é mais claro que red-700 (claro)"
        );

        // O atalho é o muted a 72%: mesma cor, alfa menor. Se alguém trocar por um cinza próprio,
        // este teste falha.
        for (nome, muted, atalho) in [
            ("claro", MENU_LIGHT.muted, MENU_LIGHT.shortcut),
            ("escuro", MENU_DARK.muted, MENU_DARK.shortcut),
        ] {
            let (m, a): (gpui::Rgba, gpui::Rgba) = (muted.hsla().into(), atalho.hsla().into());
            assert!(
                (m.r - a.r).abs() < 0.01 && (m.g - a.g).abs() < 0.01 && (m.b - a.b).abs() < 0.01,
                "{nome}: o atalho é o MESMO cinza do rótulo de grupo"
            );
            assert!(
                (a.a - 0.72).abs() < 0.01,
                "{nome}: o atalho é o muted a 72%, veio {}",
                a.a
            );
        }

        // Os tokens que DEVEM ser translúcidos continuam translúcidos — é isso que os faz funcionar
        // sobre qualquer fundo.
        for (nome, c) in [
            ("borda claro", MENU_LIGHT.border),
            ("borda escuro", MENU_DARK.border),
            ("accent claro", MENU_LIGHT.accent),
            ("accent escuro", MENU_DARK.accent),
            ("bisel claro", MENU_LIGHT.bevel),
            ("bisel escuro", MENU_DARK.bevel),
            ("sombra claro", MENU_LIGHT.shadow),
            ("sombra escuro", MENU_DARK.shadow),
        ] {
            assert!(
                c.alpha() < 1.0,
                "{nome} tem que ser translúcido, veio com alfa {}",
                c.alpha()
            );
        }

        // O destaque de item é MAIS sutil que a borda do popup (`--accent` 4% vs `--border` 8%/6%):
        // ele é um realce de fundo, não uma moldura.
        assert!(MENU_LIGHT.accent.alpha() < MENU_LIGHT.border.alpha());
        assert!(MENU_DARK.accent.alpha() < MENU_DARK.border.alpha());

        // O bisel troca de SENTIDO entre os temas (fio escuro embaixo no claro, claro em cima no
        // escuro). Trocar o sinal põe o filete no lado errado, e é um erro que só aparece na tela.
        assert!(MENU_LIGHT.bevel_dir > 0.0, "no claro o fio desce (base)");
        assert!(MENU_DARK.bevel_dir < 0.0, "no escuro o fio sobe (topo)");
    }

    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, porque a 6% o filete é
    /// imperceptível no nosso fundo. É o mesmo desvio já vigente no `input.rs`, no `card.rs` e no
    /// `select.rs` — o valor tem que ser o MESMO nos quatro, senão o popup do menu e o do dropdown
    /// ficam lado a lado com relevos diferentes. Se alguém "corrigir" pra 6% achando que é erro de
    /// porte, este teste falha e aponta pra cá.
    ///
    /// O tema CLARO segue fiel (preto 4%) — o desvio é só no escuro, onde o problema existia.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = MENU_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%)"
        );
        let escuro = MENU_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    /// A sombra do popup é o `shadow-lg` do Tailwind (duas camadas, a de baixo mais espalhada) com a
    /// cor do `/5`. Uma camada só, ou o `spread` positivo, e o relevo muda de caráter.
    #[test]
    fn sombra_do_popup_e_o_shadow_lg_a_5_por_cento() {
        theme::set_theme(theme::ThemeMode::Dark);
        let s = popup_shadow();
        assert_eq!(s.len(), 2, "shadow-lg tem DUAS camadas");
        assert_eq!(s[0].offset.y, px(10.0));
        assert_eq!(s[0].blur_radius, px(15.0));
        assert_eq!(s[0].spread_radius, px(-3.0));
        assert_eq!(s[1].offset.y, px(4.0));
        assert_eq!(s[1].blur_radius, px(6.0));
        assert_eq!(s[1].spread_radius, px(-4.0));
        for camada in &s {
            // 5% na tabela de bytes desta base é `0x0d` = 13/255 ≈ 0,051 (e NÃO 5/255, que seria
            // 2%: o alfa é uma FRAÇÃO, não um byte de porcentagem).
            assert!(
                (camada.color.a - 13.0 / 255.0).abs() < 1e-3,
                "o `/5` põe as duas camadas a 5%, veio {}",
                camada.color.a
            );
            assert!(camada.spread_radius < px(0.0), "o shadow-lg CONTRAI o espalhamento");
        }
    }

    // --- Geometria ------------------------------------------------------------------------------

    /// A geometria da superfície. O item arredonda MENOS que o popup (6 contra 10) — o contrário
    /// deixaria os cantos do item estourando o canto do popup.
    #[test]
    fn geometria_do_popup() {
        assert_eq!(RADIUS, 10.0, "rounded-lg = --radius-lg");
        assert_eq!(ITEM_RADIUS, 6.0, "rounded-sm = --radius-sm");
        assert!(ITEM_RADIUS < RADIUS);
        assert_eq!(MIN_WIDTH, 128.0, "min-w-32");
        assert_eq!(LIST_PAD, 4.0, "p-1");
        assert_eq!(BORDER, 1.0);
        // O respiro da lista + a borda é o que o `alignOffset: -5` do submenu da referência desce:
        // se estes dois mudarem, aquele -5 deixa de alinhar os topos.
        assert_eq!(LIST_PAD + BORDER, 5.0);
    }

    /// A geometria de um item de ação, incluindo o que substitui o `-mx-0.5` do ícone.
    ///
    /// ⚠️ Margem negativa colapsa o layout no GPUI, então o pull é descontado do respiro e do vão —
    /// mas a geometria RESULTANTE tem que ser a da referência: com ícone, o conjunto começa 2px mais
    /// perto da borda e o vão ícone→rótulo é 2px menor.
    #[test]
    fn geometria_do_item_de_acao() {
        assert_eq!(ITEM_MIN_HEIGHT, 28.0, "sm:min-h-7");
        assert_eq!(ITEM_PAD_Y, 4.0, "py-1");
        assert_eq!(ITEM_PAD_X, 8.0, "px-2");
        assert_eq!(ITEM_GAP, 8.0, "gap-2");
        assert_eq!(TEXT_SIZE, 14.0, "sm:text-sm");
        assert_eq!(SMALL_TEXT_SIZE, 12.0, "text-xs");
        assert_eq!(ICON_SIZE, 16.0, "sm:size-4");
        assert_eq!(INSET_PAD_START, 32.0, "data-inset:ps-8");

        // Sem ícone: respiro cheio. Com ícone: 2px a menos, dos dois lados que o `-mx-0.5` puxa.
        assert_eq!(action_pad_start(false, false), ITEM_PAD_X);
        assert_eq!(action_pad_start(false, true), ITEM_PAD_X - ICON_PULL);
        assert_eq!(ICON_GAP, ITEM_GAP - ICON_PULL);
        assert!(ICON_GAP < ITEM_GAP, "o ícone aproxima o rótulo");

        // O inset serve pra alinhar com quem tem ícone, e ele TAMBÉM perde o pull se houver ícone.
        assert_eq!(action_pad_start(true, false), INSET_PAD_START);
        assert_eq!(action_pad_start(true, true), INSET_PAD_START - ICON_PULL);
        assert!(
            action_pad_start(true, false) > action_pad_start(false, false),
            "o inset empurra o rótulo pra direita — é pra isso que ele existe"
        );
    }

    /// **O respiro do item de checkbox/radio reproduz o GRID da referência.**
    ///
    /// Lá é `grid-cols-[.75rem_1fr] gap-2 ps-2 pe-4` com o indicador em `-ms-0.5`: o ícone de 16px
    /// começa em 6px (e transborda a coluna de 12px, o que num grid é normal), e o rótulo começa na
    /// 2ª coluna, em `8 + 12 + 8 = 28`. Em flex isso só fecha se o vão for 6px. Este teste é o que
    /// garante que a conta de [`INDICATOR_GAP`] continua fechando se alguém mexer nos termos.
    #[test]
    fn respiro_do_indicador_reproduz_o_grid() {
        assert_eq!(INDICATOR_COL, 12.0, "a 1ª coluna: .75rem");
        assert_eq!(INDICATOR_PAD_START, 6.0, "ps-2 menos o -ms-0.5");
        assert_eq!(INDICATOR_GAP, 6.0);
        // Onde o rótulo começa: tem que ser o começo da 2ª coluna do grid.
        let rotulo = INDICATOR_PAD_START + ICON_SIZE + INDICATOR_GAP;
        assert_eq!(rotulo, ITEM_PAD_X + INDICATOR_COL + ITEM_GAP);
        assert_eq!(rotulo, 28.0);
        // O `pe-4` é o dobro do `ps-2`: sobra espaço à direita do rótulo em vez de ele encostar na
        // borda do popup. É o original — o inverso pareceria erro de digitação.
        assert_eq!(INDICATOR_PAD_END, 16.0, "pe-4");
        assert!(INDICATOR_PAD_END > ITEM_PAD_X);
    }

    /// O rótulo de grupo e o separador.
    #[test]
    fn geometria_do_rotulo_e_do_separador() {
        assert_eq!(LABEL_PAD_X, 8.0, "px-2");
        assert_eq!(LABEL_PAD_Y, 6.0, "py-1.5");
        assert!(
            LABEL_PAD_Y > ITEM_PAD_Y,
            "o rótulo respira mais que o item: py-1.5 contra py-1"
        );
        assert_eq!(SEPARATOR_HEIGHT, 1.0, "h-px");
        assert_eq!(SEPARATOR_MARGIN_X, 8.0, "mx-2");
        assert_eq!(SEPARATOR_MARGIN_Y, 4.0, "my-1");
        // O separador NÃO encosta na borda do popup: a margem dele mais o respiro da lista deixam
        // 12px de cada lado.
        assert_eq!(SEPARATOR_MARGIN_X + LIST_PAD, 12.0);
    }

    /// Os dois valores de opacidade da referência.
    #[test]
    fn opacidades_da_referencia() {
        assert_eq!(DISABLED_OPACITY, 0.64, "data-disabled:opacity-64");
        assert_eq!(ICON_OPACITY, 0.8, "[&>svg]:opacity-80");
        assert!(DISABLED_OPACITY < ICON_OPACITY);
    }

    // --- Posicionamento -------------------------------------------------------------------------

    /// **O `Positioner`, lado por lado e alinhamento por alinhamento.**
    ///
    /// É a parte que mais fácil sai errada sem aparecer no código (um sinal trocado põe o menu do
    /// outro lado do gatilho). O que se afirma aqui é o par (ponto, canto) que vai pro
    /// [`gpui::anchored`]; que o popup REALMENTE apareça ali é o que
    /// `tests_de_janela::o_popup_centraliza_no_gatilho` confere, contra a caixa medida.
    ///
    /// A leitura de cada linha é "o canto X do popup encosta no ponto P do gatilho".
    #[test]
    fn ponto_e_canto_de_ancoragem_por_side_e_align() {
        let (dir, base) = (GATILHO.x + GATILHO.w, GATILHO.y + GATILHO.h); // 300, 332
        let ancora = |side, align, so, ao| anchor_point(GATILHO, side, align, so, ao);

        // --- Abaixo (o default): a aresta é a BASE do gatilho, e o canto é de cima --------------
        assert_eq!(
            ancora(MenuSide::Bottom, MenuAlign::Start, 4.0, 0.0),
            (200.0, base + 4.0, Corner::TopLeft),
            "start: o topo-esquerdo do popup na quina inferior-esquerda do gatilho"
        );
        assert_eq!(
            ancora(MenuSide::Bottom, MenuAlign::End, 4.0, 0.0),
            (dir, base + 4.0, Corner::TopRight),
            "end: o topo-DIREITO do popup na quina inferior-direita — é isso que junta as bordas \
             direitas sem precisar da largura do popup"
        );
        // `center` devolve o mesmo do `start`: quem centraliza é o container de layout (e é o
        // teste de janela que confere o resultado).
        assert_eq!(
            ancora(MenuSide::Bottom, MenuAlign::Center, 4.0, 0.0),
            ancora(MenuSide::Bottom, MenuAlign::Start, 4.0, 0.0)
        );

        // --- Acima: a aresta é o TOPO, e o canto passa a ser de baixo ---------------------------
        assert_eq!(
            ancora(MenuSide::Top, MenuAlign::Start, 4.0, 0.0),
            (200.0, GATILHO.y - 4.0, Corner::BottomLeft),
            "acima: é a BASE do popup que encosta 4px antes do topo do gatilho"
        );
        assert_eq!(
            ancora(MenuSide::Top, MenuAlign::End, 4.0, 0.0).2,
            Corner::BottomRight
        );

        // --- À direita e à esquerda: o alinhamento passa a ser VERTICAL -------------------------
        assert_eq!(
            ancora(MenuSide::Right, MenuAlign::Start, 0.0, 0.0),
            (dir, GATILHO.y, Corner::TopLeft),
            "encosta na borda direita do gatilho (offset 0), topos juntos"
        );
        assert_eq!(
            ancora(MenuSide::Right, MenuAlign::End, 0.0, 0.0),
            (dir, base, Corner::BottomLeft),
            "end à direita: BASES juntas"
        );
        assert_eq!(
            ancora(MenuSide::Left, MenuAlign::Start, 4.0, 0.0),
            (GATILHO.x - 4.0, GATILHO.y, Corner::TopRight),
            "à esquerda é a borda DIREITA do popup que encosta"
        );
        assert_eq!(
            ancora(MenuSide::Left, MenuAlign::End, 4.0, 0.0).2,
            Corner::BottomRight
        );

        // --- Os offsets -------------------------------------------------------------------------
        //
        // `sideOffset` afasta do gatilho, sempre PRA FORA (o sinal do eixo depende do lado);
        // `alignOffset` desloca no eixo do alinhamento, positivo pro FIM.
        assert_eq!(
            ancora(MenuSide::Bottom, MenuAlign::Start, 12.0, 0.0).1 - base,
            12.0,
            "abaixo: o offset empurra pra BAIXO"
        );
        assert_eq!(
            GATILHO.y - ancora(MenuSide::Top, MenuAlign::Start, 12.0, 0.0).1,
            12.0,
            "acima: o mesmo offset empurra pra CIMA"
        );
        assert_eq!(
            ancora(MenuSide::Bottom, MenuAlign::Start, 4.0, 16.0).0 - 200.0,
            16.0,
            "alignOffset positivo vai pro FIM do eixo"
        );
        assert_eq!(
            ancora(MenuSide::Right, MenuAlign::Start, 0.0, -5.0).1 - GATILHO.y,
            -5.0,
            "e negativo, pro início — o caso do submenu"
        );

        // --- O caso do `MenuSubPopup` da referência ---------------------------------------------
        //
        // `side="inline-end" sideOffset={0} align="start" alignOffset={-5}`: o submenu encosta na
        // direita do item e sobe 5px, o bastante pra o topo dele coincidir com o topo do popup pai
        // (4px de `p-1` + 1px de borda). O submenu não está implementado, mas a MATEMÁTICA dele
        // está — é o que sobra pra quem for fazer.
        let item = Rect { x: 200.0, y: 305.0, w: 232.0, h: 28.0 };
        assert_eq!(
            anchor_point(item, MenuSide::Right, MenuAlign::Start, 0.0, -(LIST_PAD + BORDER)),
            (432.0, 300.0, Corner::TopLeft),
            "cola na direita do item e sobe 5px: o topo do submenu = o topo do popup pai"
        );
    }

    /// A altura máxima segue o espaço livre, com piso — um gatilho colado no rodapé abriria um menu
    /// de altura ~0 e ficaria inclicável.
    #[test]
    fn altura_maxima_segue_o_espaco_livre() {
        // Gatilho no topo de uma janela de 600px: sobra quase tudo pra baixo.
        let alto = Rect { x: 0.0, y: 20.0, w: 100.0, h: 32.0 };
        assert!(
            (popup_max_height(MenuSide::Bottom, alto, 600.0, 4.0)
                - (600.0 - 52.0 - 4.0 - WINDOW_MARGIN))
                .abs()
                < 1e-4
        );

        // Gatilho no rodapé: o espaço de CIMA é que vale, mesmo com `side: bottom` — o
        // `snap_to_window` vai deslizar o popup pra dentro, e limitá-lo aos 20px de baixo daria um
        // menu inútil.
        let baixo = Rect { x: 0.0, y: 540.0, w: 100.0, h: 32.0 };
        assert!(
            (popup_max_height(MenuSide::Bottom, baixo, 600.0, 4.0) - (540.0 - 4.0 - WINDOW_MARGIN))
                .abs()
                < 1e-4
        );
        // O lado escolhido não muda a conta nos lados verticais (é sempre o maior espaço).
        assert_eq!(
            popup_max_height(MenuSide::Bottom, baixo, 600.0, 4.0),
            popup_max_height(MenuSide::Top, baixo, 600.0, 4.0)
        );

        // Nos lados horizontais o popup pode usar a janela toda menos as margens.
        assert_eq!(
            popup_max_height(MenuSide::Right, alto, 600.0, 4.0),
            600.0 - 2.0 * WINDOW_MARGIN
        );

        // Janela minúscula: o piso segura, mesmo que estoure a margem.
        assert_eq!(popup_max_height(MenuSide::Bottom, alto, 60.0, 4.0), MIN_MAX_HEIGHT);
        // Antes do 1º prepaint os bounds são zero — não pode virar altura negativa.
        let zero = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
        assert!(popup_max_height(MenuSide::Bottom, zero, 0.0, 4.0) >= MIN_MAX_HEIGHT);
    }

    /// Os defaults do popup são os da referência: `side="bottom"`, `align="center"`,
    /// `sideOffset={4}`. Um default diferente muda o lugar de TODO menu da app.
    #[test]
    fn defaults_de_posicao_sao_os_da_referencia() {
        assert_eq!(MenuSide::default(), MenuSide::Bottom);
        assert_eq!(MenuAlign::default(), MenuAlign::Center);
        assert_eq!(SIDE_OFFSET, 4.0);
        assert!(MenuSide::Bottom.is_vertical() && MenuSide::Top.is_vertical());
        assert!(!MenuSide::Left.is_vertical() && !MenuSide::Right.is_vertical());
    }

    /// **A ancoragem no ponteiro reusa a máquina de canto sem uma conta nova.**
    ///
    /// O retângulo de tamanho zero é o truque inteiro: com `w = h = 0`, "a quina inferior-esquerda
    /// do gatilho" É o ponto do clique, então `anchor_point` devolve exatamente aquele ponto com o
    /// canto `TopLeft` do popup — e continua sendo o `anchored` (que conhece o tamanho na hora
    /// certa) quem resolve a posição. É o que a armadilha da medida exige.
    #[test]
    fn o_ponteiro_e_um_gatilho_de_tamanho_zero() {
        let p = point(px(640.0), px(480.0));
        let r = pointer_rect(p);
        assert_eq!((r.x, r.y), (640.0, 480.0));
        assert_eq!((r.w, r.h), (0.0, 0.0), "tamanho ZERO: o ponto é as quatro quinas");

        // Com `side: bottom`, `align: start` e offset 0, o canto de cima-esquerda do popup vai
        // EXATAMENTE no ponteiro — que é onde todo menu de contexto nasce.
        assert_eq!(
            anchor_point(r, MenuSide::Bottom, MenuAlign::Start, 0.0, 0.0),
            (640.0, 480.0, Corner::TopLeft)
        );
        // E o `end` devolve o MESMO ponto com o canto oposto: num retângulo de largura zero as duas
        // arestas coincidem. Ou seja, `align` num menu de contexto só troca pra que lado ele cresce.
        assert_eq!(
            anchor_point(r, MenuSide::Bottom, MenuAlign::End, 0.0, 0.0),
            (640.0, 480.0, Corner::TopRight)
        );

        // A altura máxima passa a ser o maior espaço livre a partir do PONTEIRO (menos a margem):
        // num clique a 20px do rodapé de uma janela de 600, o que vale é o que sobra acima.
        assert!(
            (popup_max_height(MenuSide::Bottom, pointer_rect(point(px(0.0), px(580.0))), 600.0, 0.0)
                - (580.0 - WINDOW_MARGIN))
                .abs()
                < 1e-4
        );
    }

    /// **Deslizar e virar são exclusivos no `anchored`, e cada ancoragem escolhe um.**
    ///
    /// Ver o doc de [`snaps_to_window`]: o `fit_mode` do [`gpui::anchored`] é um campo só. O gatilho
    /// desliza (o `side` dele é uma decisão de quem chama); o ponteiro vira, porque um popup
    /// deslizado seria pintado por cima do cursor e nasceria com um item destacado.
    #[test]
    fn quem_desliza_e_quem_vira() {
        assert!(
            snaps_to_window(MenuAnchoring::Trigger),
            "o menu de gatilho DESLIZA (snap_to_window_with_margin)"
        );
        assert!(
            !snaps_to_window(MenuAnchoring::Pointer(Point::default())),
            "o menu de contexto VIRA (o SwitchAnchor default do anchored)"
        );
        assert_eq!(
            MenuAnchoring::default(),
            MenuAnchoring::Trigger,
            "o default é o gatilho: quem não pede nada continua sendo o Menu de sempre"
        );
    }

    // --- Itens e navegação ----------------------------------------------------------------------

    /// Uma lista com um de cada coisa, pros testes de navegação:
    /// `[rótulo, ação, ação desabilitada, separador, checkbox, radio A, radio A]`.
    fn lista() -> Vec<MenuItem> {
        vec![
            MenuItem::group_label("Grupo"),
            MenuItem::new("Ação"),
            MenuItem::new("Inerte").disabled(true),
            MenuItem::separator(),
            MenuItem::checkbox("Marcar", false),
            MenuItem::radio(0, "Um", true),
            MenuItem::radio(0, "Dois", false),
        ]
    }

    /// **Só item clicável é alcançável.** Rótulo, separador e desabilitado ficam fora da máscara —
    /// se entrassem, a seta pararia num fio de 1px e o `Enter` não faria nada.
    #[test]
    fn rotulo_separador_e_desabilitado_nao_sao_alcancaveis() {
        let itens = lista();
        assert_eq!(
            activatable_mask(&itens),
            vec![false, true, false, false, true, true, true]
        );
        assert!(itens[0].is_group_label() && !itens[0].is_activatable());
        assert!(itens[3].is_separator() && !itens[3].is_activatable());
        assert!(itens[2].is_disabled() && !itens[2].is_activatable());
        // O separador não tem texto — não é um item "sem rótulo", é outra coisa.
        assert!(itens[3].label().is_empty());
    }

    /// As setas circulam e pulam o que não é alcançável; `Home`/`End` vão pras pontas ALCANÇÁVEIS
    /// (e não pro índice 0, que aqui é um rótulo).
    #[test]
    fn navegacao_por_teclado_pula_o_que_nao_e_alcancavel() {
        let mask = activatable_mask(&lista());

        // Acabou de abrir (nada destacado): a primeira seta entra pela ponta certa.
        assert_eq!(step_index(None, &mask, 1), Some(1), "↓ começa no primeiro");
        assert_eq!(step_index(None, &mask, -1), Some(6), "↑ começa no último");

        // Andando pra frente: pula o desabilitado (2) e o separador (3).
        assert_eq!(step_index(Some(1), &mask, 1), Some(4));
        assert_eq!(step_index(Some(4), &mask, 1), Some(5));
        assert_eq!(step_index(Some(6), &mask, 1), Some(1), "circula pro primeiro");
        // E pra trás, pulando o rótulo (0).
        assert_eq!(step_index(Some(1), &mask, -1), Some(6), "circula pro último");
        assert_eq!(step_index(Some(5), &mask, -1), Some(4));

        // Home/End.
        assert_eq!(edge_index(&mask, false), Some(1));
        assert_eq!(edge_index(&mask, true), Some(6));

        // Lista sem nada alcançável (só rótulos e fios): o teclado não tem onde parar, e não pode
        // entrar em laço infinito procurando.
        let so_enfeite = vec![MenuItem::group_label("x"), MenuItem::separator()];
        let mask = activatable_mask(&so_enfeite);
        assert_eq!(step_index(None, &mask, 1), None);
        assert_eq!(step_index(Some(0), &mask, 1), None);
        assert_eq!(edge_index(&mask, false), None);
        // Lista vazia.
        assert_eq!(step_index(None, &[], 1), None);
        assert_eq!(edge_index(&[], true), None);
    }

    /// **O radio é exclusivo por GRUPO**, e re-escolher o já marcado não é mudança (logo não emite,
    /// mesma regra do `Select::choose`).
    #[test]
    fn radio_e_exclusivo_por_grupo() {
        let mut itens = vec![
            MenuItem::radio(0, "A1", true),
            MenuItem::radio(0, "A2", false),
            MenuItem::radio(1, "B1", true),
            MenuItem::new("Ação"),
        ];

        // Escolher A2 desmarca A1 — e NÃO toca no grupo 1.
        assert_eq!(apply_radio(&mut itens, 1), Some(0));
        assert_eq!(itens[0].is_checked(), Some(false));
        assert_eq!(itens[1].is_checked(), Some(true));
        assert_eq!(itens[2].is_checked(), Some(true), "o outro grupo é intocado");

        // Re-escolher o marcado: nada muda, e o `None` é o que barra o evento redundante.
        assert_eq!(apply_radio(&mut itens, 1), None);
        assert_eq!(itens[1].is_checked(), Some(true));

        // Índice que não é radio (ou nem existe): ignorado.
        assert_eq!(apply_radio(&mut itens, 3), None);
        assert_eq!(apply_radio(&mut itens, 99), None);
    }

    /// **Ação fecha o menu; checkbox e radio NÃO** — é o que permite marcar vários seguidos. O
    /// `closeOnClick` explícito sobrepõe os dois sentidos.
    #[test]
    fn o_que_fecha_o_menu_ao_escolher() {
        assert!(closes_on_click(MenuItemKind::Action, None));
        assert!(!closes_on_click(MenuItemKind::Checkbox { checked: false }, None));
        assert!(!closes_on_click(
            MenuItemKind::Radio { group: 0, checked: false },
            None
        ));

        assert!(!closes_on_click(MenuItemKind::Action, Some(false)));
        assert!(closes_on_click(
            MenuItemKind::Checkbox { checked: false },
            Some(true)
        ));
    }

    /// **O destrutivo vence o destaque.** Ver o doc de [`item_text_color`]: é o único ponto em que a
    /// referência é ambígua, e este teste é onde a decisão fica registrada. Se for pra inverter, é
    /// aqui que aparece.
    #[test]
    fn destrutivo_vence_o_destaque() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let p = palette();

            assert_eq!(item_text_color(MenuItemVariant::Default, false), p.text);
            assert_eq!(item_text_color(MenuItemVariant::Default, true), p.accent_text);
            assert_eq!(
                item_text_color(MenuItemVariant::Destructive, false),
                p.destructive_text
            );
            assert_eq!(
                item_text_color(MenuItemVariant::Destructive, true),
                p.destructive_text,
                "o destaque NÃO apaga o vermelho"
            );

            // E o motivo de o destaque poder ceder: nos dois temas do coss `--accent-foreground` é
            // igual a `--foreground`, então o destaque de texto não pinta nada de novo — o único
            // efeito observável dele seria matar o destrutivo.
            assert_eq!(p.accent_text, p.text);
        }
    }

    /// **O clique no gatilho não é "clique fora".** Sem esta guarda o `on_mouse_down_out` fecharia o
    /// menu e o `on_click` do gatilho o reabriria no mesmo clique — o botão nunca fecharia o próprio
    /// menu.
    #[test]
    fn clique_no_gatilho_nao_conta_como_clique_fora() {
        assert!(!outside_click_closes(GATILHO, 250.0, 310.0), "dentro do gatilho");
        assert!(!outside_click_closes(GATILHO, 200.0, 300.0), "a quina inicial é dentro");
        assert!(outside_click_closes(GATILHO, 300.0, 332.0), "a quina final é FORA");
        assert!(outside_click_closes(GATILHO, 150.0, 310.0), "à esquerda");
        assert!(outside_click_closes(GATILHO, 250.0, 400.0), "abaixo");
        // Antes do 1º prepaint o gatilho é um retângulo vazio: aí todo clique é "fora", que é o
        // fallback certo (fechar).
        let zero = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
        assert!(outside_click_closes(zero, 0.0, 0.0));
    }

    /// Os construtores e os builders: o que cada tipo de linha é, e o que ele ignora.
    #[test]
    fn construtores_e_builders_de_item() {
        let acao = MenuItem::new("Copiar")
            .icon("icons/copy.svg")
            .shortcut("⌘C")
            .inset(true);
        assert_eq!(acao.label().as_ref(), "Copiar");
        assert_eq!(acao.icon_path().map(SharedString::as_ref), Some("icons/copy.svg"));
        assert_eq!(acao.shortcut_text().map(SharedString::as_ref), Some("⌘C"));
        assert!(acao.is_inset() && acao.is_activatable());
        assert_eq!(acao.is_checked(), None, "ação não tem marcação");
        assert_eq!(acao.group(), None);
        assert_eq!(acao.item_variant(), MenuItemVariant::Default);

        assert_eq!(
            MenuItem::new("x").destructive().item_variant(),
            MenuItemVariant::Destructive
        );

        let cb = MenuItem::checkbox("Grade", true);
        assert_eq!(cb.is_checked(), Some(true));
        assert_eq!(cb.group(), None, "checkbox não tem grupo");

        let radio = MenuItem::radio(2, "Médio", false);
        assert_eq!(radio.is_checked(), Some(false));
        assert_eq!(radio.group(), Some(2));

        // Rótulo e separador não são alcançáveis, mesmo decorados — a decoração é ignorada, não
        // proibida (ver o doc de `MenuItem`).
        let rotulo = MenuItem::group_label("Grupo").icon("icons/copy.svg");
        assert!(rotulo.is_group_label() && !rotulo.is_activatable());
        assert!(!MenuItem::separator().is_activatable());
    }

    /// O ícone do check existe no `AssetSource` do crate, e é o mesmo do popup do `Select`.
    ///
    /// ⚠️ **A referência marca o RADIO com um check também** (o `RadioItemIndicator` usa o mesmo
    /// path do checkbox), e é por isso que há uma constante só. Se este caminho quebrar, os dois
    /// indicadores somem SILENCIOSAMENTE — a `AssetSource` não reclama de path inexistente.
    #[test]
    fn o_check_e_um_asset_que_existe() {
        assert_eq!(CHECK_ICON, "icons/check.svg");
        let caminho = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/check.svg");
        assert!(
            std::path::Path::new(caminho).exists(),
            "o check do indicador tem que existir em assets/: {caminho}"
        );
    }
}

#[cfg(test)]
mod tests_de_janela {
    //! Os testes que precisam de uma JANELA de verdade: é onde a geometria declarada nas constantes
    //! é conferida contra a **medida**, e onde o clique é conferido contra o que ele realmente
    //! acerta. Nenhum teste puro pega um popup ancorado 100px pra esquerda.

    use super::*;
    use gpui::{AppContext as _, Modifiers, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Onde o gatilho é plantado dentro da janela do harness. Longe da origem de propósito: um
    /// `anchor_point` que ignorasse a posição do gatilho passaria num harness em (0,0).
    const OFFSET_X: f32 = 200.0;
    /// Idem, no eixo vertical.
    const OFFSET_Y: f32 = 100.0;

    /// Um container que planta o menu num ponto conhecido da janela.
    struct Harness {
        menu: gpui::Entity<Menu>,
        offset: (f32, f32),
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .pt(px(self.offset.1))
                .pl(px(self.offset.0))
                .child(self.menu.clone())
        }
    }

    /// A lista de teste, com um de cada coisa. As linhas são, em ordem:
    /// `0` ação (com ícone e atalho) · `1` ação DESABILITADA · `2` separador · `3` checkbox ·
    /// `4` ação destrutiva.
    fn itens() -> Vec<MenuItem> {
        vec![
            MenuItem::new("Duplicar").icon("icons/copy.svg").shortcut("⌘D"),
            MenuItem::new("Inerte").disabled(true),
            MenuItem::separator(),
            MenuItem::checkbox("Grade", false),
            MenuItem::new("Apagar").destructive(),
        ]
    }

    /// Abre a janela e devolve o menu, os eventos capturados e o contexto visual — com o gatilho no
    /// default da referência (`side: bottom`, `align: center`).
    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
    ) -> (
        gpui::Entity<Menu>,
        Rc<RefCell<Vec<MenuEvent>>>,
        VisualTestContext,
    ) {
        abrir_com(cx, MenuSide::Bottom, MenuAlign::Center, (OFFSET_X, OFFSET_Y))
    }

    /// Idem, escolhendo lado, alinhamento e onde o gatilho é plantado.
    #[allow(clippy::type_complexity)]
    fn abrir_com(
        cx: &mut TestAppContext,
        side: MenuSide,
        align: MenuAlign,
        offset: (f32, f32),
    ) -> (
        gpui::Entity<Menu>,
        Rc<RefCell<Vec<MenuEvent>>>,
        VisualTestContext,
    ) {
        theme::set_theme(theme::ThemeMode::Dark);
        let eventos: Rc<RefCell<Vec<MenuEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_window, cx| {
            let menu = cx.new(|cx| {
                Menu::new(itens(), cx)
                    .trigger_button("Editar")
                    .side(side)
                    .align(align)
            });
            cx.subscribe(&menu, move |_this, _m, ev: &MenuEvent, _cx| {
                capturados.borrow_mut().push(ev.clone());
            })
            .detach();
            Harness { menu, offset }
        });

        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let menu = vcx.read(|cx| harness.read(cx).menu.clone());
        (menu, eventos, vcx)
    }

    /// O centro do gatilho, em coordenadas da janela.
    fn centro_do_gatilho(
        menu: &gpui::Entity<Menu>,
        vcx: &mut VisualTestContext,
    ) -> gpui::Point<Pixels> {
        vcx.read(|cx| {
            let b = menu.read(cx).trigger_bounds;
            point(
                b.origin.x + b.size.width / 2.0,
                b.origin.y + b.size.height / 2.0,
            )
        })
    }

    /// A altura de cada linha da lista de teste, DERIVADA das constantes (não copiada): as quatro
    /// linhas clicáveis medem [`ITEM_MIN_HEIGHT`], e o separador mede o fio mais as duas margens.
    fn altura_da_linha(i: usize) -> f32 {
        if i == 2 {
            SEPARATOR_HEIGHT + 2.0 * SEPARATOR_MARGIN_Y
        } else {
            ITEM_MIN_HEIGHT
        }
    }

    /// O topo da linha `i` a partir da borda de cima do popup.
    fn topo_da_linha(i: usize) -> f32 {
        (0..i).map(altura_da_linha).sum::<f32>() + BORDER + LIST_PAD
    }

    /// O ponto no centro da linha `i`, em coordenadas da JANELA.
    ///
    /// Derivado só do **gatilho** e das constantes de geometria: com `side: bottom` o topo do popup é
    /// `base do gatilho + sideOffset`, e com `align: center` o centro horizontal do popup é o do
    /// gatilho — nenhuma medida do popup entra na conta. É este ponto que os testes clicam: se o
    /// popup for pintado em outro lugar, o clique erra o item e o teste falha.
    fn centro_da_linha(
        menu: &gpui::Entity<Menu>,
        vcx: &mut VisualTestContext,
        i: usize,
    ) -> gpui::Point<Pixels> {
        vcx.read(|cx| {
            let g = rect_of(menu.read(cx).trigger_bounds);
            point(
                px(g.x + g.w / 2.0),
                px(g.y + g.h + SIDE_OFFSET + topo_da_linha(i) + altura_da_linha(i) / 2.0),
            )
        })
    }

    /// Abre o menu pelo clique no gatilho e deixa a janela parada.
    fn abrir_o_popup(menu: &gpui::Entity<Menu>, vcx: &mut VisualTestContext) {
        let g = centro_do_gatilho(menu, vcx);
        vcx.simulate_click(g, Modifiers::default());
        vcx.run_until_parked();
        assert!(vcx.read(|cx| menu.read(cx).is_open()), "o clique abre o menu");
    }

    /// **A geometria declarada bate com a MEDIDA, px a px.**
    ///
    /// A altura do popup é a soma de tudo que este módulo afirma nas constantes: as duas bordas, o
    /// respiro da lista em cima e embaixo, quatro linhas de 28px e um separador de 1px com `my-1`.
    /// Se o `min-h-7` virasse `min-h-8`, se o `p-1` virasse `p-2`, se o separador colapsasse as
    /// margens ou se a **entrelinha** voltasse pro default do GPUI (que é a razão de ouro, e já
    /// esticou este popup pra 141,5px), este número muda — e é o número que a captura de tela mede.
    #[gpui::test]
    fn o_popup_nasce_com_a_geometria_declarada(cx: &mut TestAppContext) {
        let (menu, _ev, mut vcx) = abrir(cx);
        abrir_o_popup(&menu, &mut vcx);

        vcx.read(|cx| {
            let m = menu.read(cx);

            // O gatilho foi medido onde o harness o plantou — é o "anchor rect" do Positioner.
            let g = rect_of(m.trigger_bounds);
            assert_eq!((g.x, g.y), (OFFSET_X, OFFSET_Y), "o anchor rect é o do gatilho");
            assert!(g.w > 0.0 && g.h > 0.0);

            let caixa = m.popup_bounds().expect("o popup se mede no primeiro prepaint dele");
            let esperada = 2.0 * (BORDER + LIST_PAD) + 4.0 * ITEM_MIN_HEIGHT + altura_da_linha(2);
            assert_eq!(
                f32::from(caixa.size.height),
                esperada,
                "a altura do popup é borda + p-1 + 4 linhas de 28 + separador de 9"
            );
            assert_eq!(f32::from(caixa.size.height), 131.0);
            // A linha de item fecha em 28px justamente porque a entrelinha do `text-sm` (20px) mais o
            // `py-1` dá exatamente o `min-h-7` — é isso que faz o `min-h` do coss não inflar nada.
            assert_eq!(TEXT_LINE_HEIGHT + 2.0 * ITEM_PAD_Y, ITEM_MIN_HEIGHT);
            assert_eq!(SMALL_LINE_HEIGHT + 2.0 * LABEL_PAD_Y, ITEM_MIN_HEIGHT);

            // Sem largura explícita vale o piso do `min-w-32`; o conteúdo desta lista é estreito.
            assert!(
                f32::from(caixa.size.width) >= MIN_WIDTH,
                "min-w-32 é um PISO, veio {}",
                f32::from(caixa.size.width)
            );
        });
    }

    /// **O popup é PINTADO centralizado no gatilho, 4px abaixo dele.**
    ///
    /// Aqui não há modelo nenhum: a caixa é a que o `canvas` mediu no prepaint, ou seja onde o popup
    /// realmente está. É o teste que prova que a centralização por layout (o container do tamanho do
    /// gatilho com `justify-center`) faz o que o `align: center` da referência pede — e o que pegou a
    /// versão que posicionava pela medida e errava o primeiro frame em meia largura.
    #[gpui::test]
    fn o_popup_centraliza_no_gatilho(cx: &mut TestAppContext) {
        let (menu, _ev, mut vcx) = abrir(cx);
        abrir_o_popup(&menu, &mut vcx);

        vcx.read(|cx| {
            let m = menu.read(cx);
            let g = rect_of(m.trigger_bounds);
            let caixa = m.popup_bounds().expect("medido");
            let (x, y) = (f32::from(caixa.origin.x), f32::from(caixa.origin.y));
            let w = f32::from(caixa.size.width);

            assert_eq!(
                x + w / 2.0,
                g.x + g.w / 2.0,
                "os centros horizontais coincidem — é a definição de align: center"
            );
            assert_eq!(y, g.y + g.h + SIDE_OFFSET, "4px abaixo da base do gatilho");
            // O popup é mais largo que este gatilho, então `center` o faz transbordar pra esquerda —
            // é o comportamento certo, e é por isso que o harness planta o gatilho longe da borda.
            assert!(
                x < g.x,
                "um popup mais largo que o gatilho transborda dos dois lados"
            );
            assert!(x > WINDOW_MARGIN, "sem precisar grudar na janela neste harness");
        });
    }

    /// **Os cantos de ancoragem pintam onde devem** — os 8 casos de `align: start`/`end`, que são os
    /// que dispensam medir o popup.
    ///
    /// É o que confere a semântica do canto contra a realidade: um `Corner` trocado põe o menu do
    /// outro lado do gatilho, e nenhum teste puro pega isso porque a conta de canto → caixa é do
    /// GPUI, não nossa.
    #[gpui::test]
    fn os_cantos_de_ancoragem_pintam_onde_devem(cx: &mut TestAppContext) {
        // `align: end` abaixo: as bordas DIREITAS se juntam.
        let (menu, _ev, mut vcx) = abrir_com(cx, MenuSide::Bottom, MenuAlign::End, (200.0, 100.0));
        abrir_o_popup(&menu, &mut vcx);
        vcx.read(|cx| {
            let m = menu.read(cx);
            let g = rect_of(m.trigger_bounds);
            let b = m.popup_bounds().expect("medido");
            assert_eq!(
                f32::from(b.origin.x) + f32::from(b.size.width),
                g.x + g.w,
                "align end: bordas direitas juntas"
            );
            assert_eq!(f32::from(b.origin.y), g.y + g.h + SIDE_OFFSET);
        });

        // `side: top`: é a BASE do popup que encosta acima do gatilho. O gatilho vai fundo na janela
        // pra o popup caber acima dele sem o `snap_to_window` interferir.
        let (menu, _ev, mut vcx) = abrir_com(cx, MenuSide::Top, MenuAlign::Start, (200.0, 400.0));
        abrir_o_popup(&menu, &mut vcx);
        vcx.read(|cx| {
            let m = menu.read(cx);
            let g = rect_of(m.trigger_bounds);
            let b = m.popup_bounds().expect("medido");
            assert_eq!(f32::from(b.origin.x), g.x, "align start: bordas esquerdas juntas");
            assert_eq!(
                f32::from(b.origin.y) + f32::from(b.size.height),
                g.y - SIDE_OFFSET,
                "side top: a base do popup fica 4px acima do topo do gatilho"
            );
        });

        // `side: right` (o `inline-end` do submenu): encosta na direita, topos juntos.
        let (menu, _ev, mut vcx) = abrir_com(cx, MenuSide::Right, MenuAlign::Start, (200.0, 100.0));
        abrir_o_popup(&menu, &mut vcx);
        vcx.read(|cx| {
            let m = menu.read(cx);
            let g = rect_of(m.trigger_bounds);
            let b = m.popup_bounds().expect("medido");
            assert_eq!(f32::from(b.origin.x), g.x + g.w + SIDE_OFFSET);
            assert_eq!(f32::from(b.origin.y), g.y, "align start à direita: topos juntos");
        });

        // `side: left`: é a borda DIREITA do popup que encosta antes do gatilho.
        let (menu, _ev, mut vcx) = abrir_com(cx, MenuSide::Left, MenuAlign::End, (400.0, 100.0));
        abrir_o_popup(&menu, &mut vcx);
        vcx.read(|cx| {
            let m = menu.read(cx);
            let g = rect_of(m.trigger_bounds);
            let b = m.popup_bounds().expect("medido");
            assert_eq!(
                f32::from(b.origin.x) + f32::from(b.size.width),
                g.x - SIDE_OFFSET,
                "side left: a direita do popup encosta 4px antes do gatilho"
            );
            assert_eq!(
                f32::from(b.origin.y) + f32::from(b.size.height),
                g.y + g.h,
                "align end à esquerda: BASES juntas"
            );
        });
    }

    /// **O clique cai no item certo, e só a AÇÃO fecha o menu.**
    ///
    /// Este é o teste que amarra tudo: o ponto clicado é derivado das constantes de geometria e do
    /// [`anchor_point`], então se o popup for pintado em outro lugar (ou se as linhas tiverem outra
    /// altura), o clique acerta o item errado — ou nenhum — e o evento não confere.
    #[gpui::test]
    fn o_clique_cai_no_item_certo_e_so_a_acao_fecha(cx: &mut TestAppContext) {
        let (menu, eventos, mut vcx) = abrir(cx);
        abrir_o_popup(&menu, &mut vcx);

        // Linha 3: o checkbox. Marca, EMITE, e o menu FICA ABERTO (dá pra marcar vários).
        let p = centro_da_linha(&menu, &mut vcx, 3);
        vcx.simulate_click(p, Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            &*eventos.borrow(),
            &[
                MenuEvent::OpenChange(true),
                MenuEvent::CheckedChange { index: 3, checked: true }
            ]
        );
        assert!(
            vcx.read(|cx| menu.read(cx).is_open()),
            "marcar um checkbox NÃO fecha o menu"
        );
        assert_eq!(
            vcx.read(|cx| menu.read(cx).items()[3].is_checked()),
            Some(true)
        );

        // Linha 4: a ação destrutiva. Emite Select(4) e FECHA.
        let p = centro_da_linha(&menu, &mut vcx, 4);
        vcx.simulate_click(p, Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            &eventos.borrow()[2..],
            &[MenuEvent::Select(4), MenuEvent::OpenChange(false)]
        );
        assert!(!vcx.read(|cx| menu.read(cx).is_open()), "a ação fecha o menu");
    }

    /// **O desabilitado e o separador não têm hitbox.** Clicar neles não emite nada e não fecha o
    /// menu — se o separador fosse clicável, o menu fecharia ao clicar num fio de 1px.
    #[gpui::test]
    fn desabilitado_e_separador_nao_reagem_ao_clique(cx: &mut TestAppContext) {
        let (menu, eventos, mut vcx) = abrir(cx);
        abrir_o_popup(&menu, &mut vcx);

        for linha in [1, 2] {
            let p = centro_da_linha(&menu, &mut vcx, linha);
            vcx.simulate_click(p, Modifiers::default());
            vcx.run_until_parked();
            assert!(
                vcx.read(|cx| menu.read(cx).is_open()),
                "a linha {linha} não pode fechar o menu"
            );
        }
        assert_eq!(
            &*eventos.borrow(),
            &[MenuEvent::OpenChange(true)],
            "nem o desabilitado nem o separador emitem"
        );
    }

    /// **Um item destacado de cada vez.** O destaque de hover e o de teclado são o MESMO estado, o
    /// que reproduz o `data-highlighted` do Base UI; dois itens acesos ao mesmo tempo é o defeito que
    /// o `.hover()` do GPUI produziria se convivesse com o índice do teclado.
    #[gpui::test]
    fn o_hover_destaca_um_item_de_cada_vez(cx: &mut TestAppContext) {
        let (menu, _ev, mut vcx) = abrir(cx);
        abrir_o_popup(&menu, &mut vcx);

        let sobre = |i, vcx: &mut VisualTestContext| {
            let p = centro_da_linha(&menu, vcx, i);
            vcx.simulate_mouse_move(p, None, Modifiers::default());
            vcx.run_until_parked();
        };

        sobre(0, &mut vcx);
        assert_eq!(vcx.read(|cx| menu.read(cx).highlighted()), Some(0));
        sobre(3, &mut vcx);
        assert_eq!(
            vcx.read(|cx| menu.read(cx).highlighted()),
            Some(3),
            "o destaque MUDA de item, não acumula"
        );
        // O desabilitado não destaca (ele não tem listener nenhum).
        sobre(1, &mut vcx);
        assert_ne!(
            vcx.read(|cx| menu.read(cx).highlighted()),
            Some(1),
            "item desabilitado não destaca"
        );

        // Saindo do popup, o destaque apaga.
        vcx.simulate_mouse_move(point(px(5.0), px(5.0)), None, Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| menu.read(cx).highlighted()), None);
    }

    /// **O clique no gatilho com o menu aberto FECHA** (e não fecha-e-reabre), e o clique fora
    /// também fecha.
    ///
    /// O primeiro caso é a armadilha: o clique no gatilho é, do ponto de vista do popup, um clique
    /// FORA — então o `on_mouse_down_out` fecharia, e o `on_click` do gatilho reabriria no mesmo
    /// clique. Quem impede é a guarda de [`outside_click_closes`].
    #[gpui::test]
    fn o_gatilho_fecha_o_proprio_menu_e_o_clique_fora_tambem(cx: &mut TestAppContext) {
        let (menu, eventos, mut vcx) = abrir(cx);

        abrir_o_popup(&menu, &mut vcx);
        let g = centro_do_gatilho(&menu, &mut vcx);
        vcx.simulate_click(g, Modifiers::default());
        vcx.run_until_parked();
        assert!(
            !vcx.read(|cx| menu.read(cx).is_open()),
            "o segundo clique no gatilho FECHA — se reabrisse, o botão nunca fecharia o menu"
        );
        assert_eq!(
            &*eventos.borrow(),
            &[MenuEvent::OpenChange(true), MenuEvent::OpenChange(false)],
            "abriu uma vez e fechou uma vez"
        );

        // Clique bem longe: fecha.
        abrir_o_popup(&menu, &mut vcx);
        vcx.simulate_click(point(px(5.0), px(5.0)), Modifiers::default());
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| menu.read(cx).is_open()), "clique fora fecha");
    }

    /// **O teclado abre, anda, escolhe e fecha — e é ele que acende o anel de foco.**
    ///
    /// O `focus-visible` da referência: o clique no gatilho não pode acender anel (defeito já
    /// relatado nesta base), e a primeira tecla acende.
    #[gpui::test]
    fn o_teclado_abre_anda_escolhe_e_acende_o_anel(cx: &mut TestAppContext) {
        let (menu, eventos, mut vcx) = abrir(cx);

        // Foca o gatilho por CLIQUE (e fecha o menu que o clique abriu): o anel não acende.
        abrir_o_popup(&menu, &mut vcx);
        let g = centro_do_gatilho(&menu, &mut vcx);
        vcx.simulate_click(g, Modifiers::default());
        vcx.run_until_parked();
        assert!(
            !crate::focus_ring::visible(),
            "clicar não acende o anel (a referência é focus-visible, não focus)"
        );

        // `↓` com o menu fechado ABRE, já com o primeiro item alcançável destacado.
        vcx.simulate_keystrokes("down");
        vcx.run_until_parked();
        assert!(vcx.read(|cx| menu.read(cx).is_open()), "a seta abre o menu");
        assert_eq!(
            vcx.read(|cx| menu.read(cx).highlighted()),
            Some(0),
            "abre destacando o primeiro"
        );
        assert!(crate::focus_ring::visible(), "o teclado acende o anel");

        // `↓` pula a linha 1 (desabilitada) e a 2 (separador).
        vcx.simulate_keystrokes("down");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| menu.read(cx).highlighted()), Some(3));

        // `End` vai pro último alcançável; `Home` volta pro primeiro.
        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| menu.read(cx).highlighted()), Some(4));
        vcx.simulate_keystrokes("home");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| menu.read(cx).highlighted()), Some(0));

        // `Enter` escolhe o destacado: emite e fecha.
        vcx.simulate_keystrokes("enter");
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| menu.read(cx).is_open()));
        assert_eq!(
            &eventos.borrow()[2..],
            &[
                MenuEvent::OpenChange(true),
                MenuEvent::Select(0),
                MenuEvent::OpenChange(false)
            ]
        );

        // `Escape` fecha sem escolher, e limpa o destaque.
        vcx.simulate_keystrokes("down");
        vcx.run_until_parked();
        vcx.simulate_keystrokes("escape");
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| menu.read(cx).is_open()), "escape fecha");
        assert_eq!(
            vcx.read(|cx| menu.read(cx).highlighted()),
            None,
            "fechar limpa o destaque: reabrir começa do zero"
        );
    }
}

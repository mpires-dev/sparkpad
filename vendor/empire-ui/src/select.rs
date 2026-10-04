//! `Select` — o **dropdown** (escolha 1-de-N) do `empire-ui`, com o visual do design system
//! [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/select.tsx`
//!
//! Complementa os outros controles de param do inspector: enquanto o
//! [`crate::scrub_input::ScrubInput`] edita *float* e o [`crate::switch::Switch`]/
//! [`crate::checkbox::Checkbox`] editam *bool*, o `Select` edita uma **escolha enumerada**
//! (um índice de uma lista de opções). É o controle que o `pve-core`
//! `ParamKind::Enum` mapeia — usado pelos modos do DoF v2 e pelo picker "add effect" por-clip.
//!
//! # Anatomia
//!
//! ```text
//! ┌──────────────────────────────────────┐
//! │ ⌘  Multiply                       ⌄ │  ← o GATILHO: .with_icon(..) · valor · chevron
//! └──────────────────────────────────────┘
//!   ┌────────────────────────────────────┐
//!   │ ✓  Normal                          │  ← o POPUP: um item por opção, check na coluna 1
//!   │    Multiply                         │
//!   └────────────────────────────────────┘
//! ```
//!
//! # O gatilho é o mesmo campo do [`crate::input::Input`]
//!
//! No coss o `SelectTrigger` e o `Input` compartilham a geometria e os estados: mesma altura,
//! mesmo raio, mesma borda, mesmo bisel de 1px, mesmo anel de foco de 3px, mesma precedência
//! de inválido sobre foco. Este módulo repete essas decisões de propósito — inclusive as duas
//! **armadilhas do GPUI** que o campo de texto já pagou (ver [`bevel_for`] e [`ring_overlay`]):
//!
//! 1. `Window::paint_shadows` **não** recorta a sombra pra fora do elemento que a projeta
//!    (diferente do CSS), então bisel e anel **não** podem ser sombra — são bordas de overlay.
//! 2. Um overlay que cobre a BORDA do gatilho é **irmão** dele, não filho: o gatilho tem
//!    `overflow_hidden` e recortaria o overlay.
//!
//! # Popover (qual API do gpui)
//!
//! O menu segue o **idioma de popover do gpui-component**: a posição/largura do gatilho é
//! capturada por um [`gpui::canvas`] overlay (lê os `bounds` no prepaint); quando aberto, o menu
//! é renderizado dentro de
//! [`gpui::deferred`]`(`[`gpui::anchored`]`().snap_to_window_with_margin(..))` — `deferred`
//! pinta o menu **por cima** de tudo (depois dos ancestrais), e `anchored` o reposiciona pra
//! não estourar a janela. O `div` do menu é `.occlude()` (captura o mouse, não vaza cliques
//! pra trás) e usa `on_mouse_down_out` pra fechar ao clicar fora.
//!
//! # Contrato
//!
//! Mesmo contrato desacoplado dos demais controles: é um `Entity` (view) próprio que mantém
//! o seu estado (`selected`) e **emite** [`SelectEvent::Change`] com o novo índice ao escolher.
//! Quem usa só assina o evento (`cx.subscribe`) e grava onde quiser. `set_selected` sincroniza
//! de fora **sem** emitir (evita loop de feedback — mesma regra do `Switch::set_on`).

use crate::color::Rgba8;
use crate::theme;
use gpui::{
    anchored, canvas, deferred, div, prelude::FluentBuilder as _, px, svg, App, Bounds, Context,
    CursorStyle, Div, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Pixels, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

/// Evento emitido pelo [`Select`] quando o usuário escolhe uma opção. Carrega o **índice**
/// recém-selecionado (no espírito do [`crate::switch::SwitchEvent`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectEvent {
    /// Novo índice selecionado após o clique numa opção.
    Change(usize),
}

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Os utilitários Tailwind do `select.tsx` resolvidos em número, para um app nativo:
//
// - O breakpoint `sm:` do Tailwind é ≥640px, e uma janela de desktop está sempre acima disso —
//   então valem as variantes `sm:` (gatilho de 32px, item de 28px).
// - As cores vêm das custom properties do tema do coss, com a paleta `neutral`/`red` do Tailwind
//   resolvida: `neutral-400 #a3a3a3`, `neutral-500 #737373`, `neutral-800 #262626`,
//   `neutral-100 #f5f5f5`, `red-500 #ef4444`. Onde o coss usa `color-mix`, o resultado já está
//   calculado aqui.
//
// Fica num struct próprio (e não na [`crate::theme::Palette`]) porque são tokens de UM componente —
// é o mesmo padrão do [`crate::input`] e do [`crate::card`].

/// Tokens visuais do dropdown, por tema.
///
/// ⚠️ **TODAS as cores aqui são `0xRRGGBBAA`** — com o byte de alfa, sempre, mesmo quando opacas
/// (`…ff`). É a convenção das paletas de componente do crate (ver [`crate::color`]); misturar com
/// os tokens de 6 dígitos da [`crate::theme::Palette`] **desloca os canais** e dá uma cor
/// completamente diferente, sem erro de compilação. Já custou três bugs visíveis no campo de texto
/// (campos ciano no claro, anel de foco teal no escuro). A ponte é [`crate::color::opaque`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct SelectPalette {
    /// Fundo do gatilho (`--background`). No escuro é translúcido (`dark:bg-input/32`), então
    /// "levanta" sobre o painel.
    pub(crate) bg: Rgba8,
    /// Cor do valor selecionado e do texto dos itens (`--foreground`).
    pub(crate) text: Rgba8,
    /// Cor do valor quando é placeholder (`--muted-foreground`).
    placeholder: Rgba8,
    /// Borda do gatilho em repouso, e também a do popup (`--input`).
    border: Rgba8,
    /// Borda quando focado/aberto (`--ring`, sólida).
    ring: Rgba8,
    /// Anel de foco de 3px (`ring-ring/24`).
    ring_glow: Rgba8,
    /// Borda quando inválido e SEM foco (`--destructive` a 36%).
    danger_border: Rgba8,
    /// Borda quando inválido e COM foco (`--destructive` a 64%).
    danger_border_focus: Rgba8,
    /// Anel de foco quando inválido (16% no claro, 24% no escuro).
    danger_glow: Rgba8,
    /// Sombra externa do gatilho em repouso (`shadow-xs/5`).
    shadow: Rgba8,
    /// Fio de bisel de 1px em repouso. No claro é escuro e desce 1px; no escuro é claro e sobe 1px
    /// — é o `before:shadow-[0_1px_black/4%]` / `[0_-1px_white/6%]` do coss.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro), `-1` sobe (escuro).
    bevel_dir: f32,
    /// Fundo do popup (`--popover`). **Opaco** nos dois temas — é o que permite a sombra externa.
    popover_bg: Rgba8,
    /// Fundo do item destacado, por hover ou teclado (`--accent`).
    accent: Rgba8,
    /// Texto do item destacado (`--accent-foreground`).
    ///
    /// Nos dois temas do coss ele é IGUAL ao `--foreground` — o destaque muda só o fundo. Fica
    /// como token próprio de propósito: é assim no original, e um tema derivado pode divergir.
    accent_text: Rgba8,
}

/// Tema **claro**.
const SELECT_LIGHT: SelectPalette = SelectPalette {
    bg: Rgba8(0xffffffff),
    text: Rgba8(0x262626ff), // neutral-800
    // muted-foreground = mix(neutral-500 90%, black) = #686868
    placeholder: Rgba8(0x686868ff),
    border: Rgba8(0x0000001a),   // --input: black 10%
    ring: Rgba8(0xa3a3a3ff),     // neutral-400
    ring_glow: Rgba8(0xa3a3a33d), // 24%
    danger_border: Rgba8(0xef44445c), // 36%
    danger_border_focus: Rgba8(0xef4444a3), // 64%
    danger_glow: Rgba8(0xef444429), // 16%
    shadow: Rgba8(0x0000000d),   // black 5%
    bevel: Rgba8(0x0000000a),    // black 4%
    bevel_dir: 1.0,
    popover_bg: Rgba8(0xffffffff),
    accent: Rgba8(0x0000000a), // black 4%
    accent_text: Rgba8(0x262626ff),
};

/// Tema **escuro**.
const SELECT_DARK: SelectPalette = SelectPalette {
    // `dark:bg-input/32`: --input escuro é branco a 8%, e o /32 do Tailwind o multiplica → ~2,5%.
    bg: Rgba8(0xffffff07),
    text: Rgba8(0xf5f5f5ff), // neutral-100
    // muted-foreground = mix(neutral-500 90%, white) = #818181
    placeholder: Rgba8(0x818181ff),
    border: Rgba8(0xffffff14),    // --input: white 8%
    ring: Rgba8(0x737373ff),      // neutral-500
    ring_glow: Rgba8(0x7373733d), // 24%
    // destructive escuro = mix(red-500 90%, white) = #f15757.
    danger_border: Rgba8(0xf157575c),
    danger_border_focus: Rgba8(0xf15757a3),
    danger_glow: Rgba8(0xf157573d), // 24% no escuro
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE do coss: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO —
    // alfa 30 ≈ 11,8% — por decisão de design: a 6% o filete é imperceptível no nosso fundo escuro.
    // É o mesmo desvio já vigente no `input.rs` e no `card.rs`; manter os três iguais é o ponto.
    // NÃO "corrija" isto pra 0x0f achando que é erro de porte; se a intenção mudar, mude junto o
    // teste `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
    // --popover escuro = mix(background 96%, white) = #1d1d1d (background = #141414).
    popover_bg: Rgba8(0x1d1d1dff),
    accent: Rgba8(0xffffff0a), // white 4%
    accent_text: Rgba8(0xf5f5f5ff),
};

/// A paleta do dropdown no tema corrente.
pub(crate) fn palette() -> &'static SelectPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &SELECT_DARK,
        theme::ThemeMode::Light => &SELECT_LIGHT,
    }
}

// =================================================================================================
// Geometria
// =================================================================================================

/// Raio do gatilho e do popup — `rounded-lg` = `--radius-lg` = **10px**.
const RADIUS: f32 = 10.0;

/// Raio dos itens do popup — `rounded-sm` = `--radius-sm` = **6px**.
const ITEM_RADIUS: f32 = 6.0;

/// Espessura do anel de foco — `focus-visible:ring-[3px]`.
const RING_WIDTH: f32 = 3.0;

/// Largura mínima do gatilho — `min-w-36`.
const MIN_WIDTH: f32 = 144.0;

/// Corpo do texto do valor e dos itens — `text-sm`. **Igual nos três tamanhos**: no coss as
/// variantes de tamanho mudam altura e respiro, não o corpo (mesma decisão do [`crate::input`]).
const TEXT_SIZE: f32 = 14.0;

/// Lado dos ícones do gatilho (o prefixo e o chevron) — `[&_svg]:size-4`.
const ICON_SIZE: f32 = 16.0;

/// Opacidade dos ícones do gatilho — `[&_svg]:opacity-80`.
const ICON_OPACITY: f32 = 0.8;

/// Opacidade do conjunto quando desabilitado — `disabled:opacity-64`. O coss esmaece o conjunto
/// inteiro em vez de trocar cor por cor; assim não é preciso um par "apagado" de cada token.
const DISABLED_OPACITY: f32 = 0.64;

/// Respiro da lista dentro do popup — `p-1`.
const LIST_PAD: f32 = 4.0;

/// Altura mínima de um item — `sm:min-h-7`.
const ITEM_MIN_HEIGHT: f32 = 28.0;

/// Vão entre a coluna do check e o rótulo — `gap-2`.
const ITEM_GAP: f32 = 8.0;

/// Largura da coluna do check — a 1ª coluna do grid `[16px_1fr]`.
pub(crate) const CHECK_SLOT: f32 = 16.0;

/// Respiro vertical de um item — `py-1`.
const ITEM_PAD_Y: f32 = 4.0;

/// Respiro no INÍCIO do item — `ps-2`.
pub(crate) const ITEM_PAD_START: f32 = 8.0;

/// Respiro no FIM do item — `pe-4`. Maior que o do início de propósito (é o original): sobra
/// espaço à direita do rótulo em vez de ele encostar na borda do popup.
pub(crate) const ITEM_PAD_END: f32 = 16.0;

/// Distância entre o gatilho e o popup — o `sideOffset` do original.
const MENU_GAP: f32 = 4.0;

/// Margem que o popup guarda das bordas da janela (o mesmo valor vai pro
/// `snap_to_window_with_margin`).
const MENU_MARGIN: f32 = 8.0;

/// Piso da altura máxima do popup. Sem ele, um gatilho colado no rodapé da janela abriria um menu
/// de altura ~0 — melhor estourar um pouco a margem e ser rolável do que ficar inclicável.
const MENU_MIN_MAX_HEIGHT: f32 = 96.0;

// =================================================================================================
// Tamanho
// =================================================================================================

/// Tamanho do dropdown. Controla altura, respiro horizontal e vão de uma vez — pra não existir
/// dropdown "quase md" com números escolhidos a olho no call site.
///
/// Os números são os do gatilho do coss no breakpoint `sm:`. Diferente do [`crate::input::InputSize`],
/// aqui a altura é `min-h-*` **na própria caixa com borda** (no coss o `h-*` do campo de texto está
/// no `<input>`, por dentro da moldura) — então 32px já é a caixa visível inteira, não o miolo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SelectSize {
    /// Compacto — inspector, barras de ferramentas, tabelas densas (`sm:min-h-7`).
    Sm,
    /// Padrão (`sm:min-h-8`).
    #[default]
    Md,
    /// Confortável — formulários de destaque (`sm:min-h-9`).
    Lg,
}

impl SelectSize {
    /// Altura mínima do gatilho, **já incluindo as duas bordas de 1px**.
    pub fn min_height(self) -> f32 {
        match self {
            SelectSize::Sm => 28.0,
            SelectSize::Md => 32.0,
            SelectSize::Lg => 36.0,
        }
    }

    /// Respiro horizontal — `px-[calc(--spacing(2.5)-1px)]` no `Sm` e
    /// `px-[calc(--spacing(3)-1px)]` nos outros. O `-1px` desconta a borda, pra o recuo TOTAL a
    /// partir da borda externa fechar em 10/12px.
    pub fn pad_x(self) -> f32 {
        match self {
            SelectSize::Sm => 9.0,
            SelectSize::Md | SelectSize::Lg => 11.0,
        }
    }

    /// Vão entre ícone, valor e chevron — `gap-1.5` no `Sm`, `gap-2` nos outros.
    pub fn gap(self) -> f32 {
        match self {
            SelectSize::Sm => 6.0,
            SelectSize::Md | SelectSize::Lg => 8.0,
        }
    }

    /// Corpo do texto do valor — 14px nos três tamanhos (ver [`TEXT_SIZE`]).
    pub fn text_size(self) -> f32 {
        let _ = self;
        TEXT_SIZE
    }

    /// Raio do gatilho — `rounded-lg`, igual nos três tamanhos.
    pub fn radius(self) -> f32 {
        let _ = self;
        RADIUS
    }
}

/// O tamanho efetivo do gatilho: o que foi pedido explicitamente por [`Select::size`] ou, na
/// falta dele, o default da variante.
///
/// O **borderless** cai em [`SelectSize::Sm`] porque é a variante dos blocos densos do inspector,
/// onde ele conviveu (e precisa continuar convivendo) com fields compactos — um `Md` de 32px ali
/// abre a linha e desalinha os vizinhos.
fn effective_size(explicit: Option<SelectSize>, borderless: bool) -> SelectSize {
    match explicit {
        Some(s) => s,
        None if borderless => SelectSize::Sm,
        None => SelectSize::Md,
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// Dropdown padrão (gatilho clicável + popup de opções).
pub struct Select {
    /// Rótulos das opções, na ordem dos índices.
    options: Vec<SharedString>,
    /// Índice atualmente selecionado (`< options.len()`, salvo lista vazia).
    selected: usize,
    /// Se o menu está aberto.
    open: bool,
    /// Bounds do gatilho (px), capturados pelo `canvas` overlay no prepaint — usados para
    /// dimensionar o menu na mesma largura do gatilho e limitar a altura dele ao espaço livre.
    bounds: Bounds<Pixels>,
    /// Id estável deste select (pra `div().id(...)` único na árvore).
    id: u64,
    focus_handle: FocusHandle,
    /// Rolagem da lista do popup. Persiste entre renders: sem ela, cada `cx.notify()` (um hover,
    /// por exemplo) devolveria a lista pro topo no meio da rolagem.
    menu_scroll: ScrollHandle,
    /// Ícone-prefixo opcional DENTRO do gatilho (path svg, ex.: `"icons/blend.svg"`). `None` = sem.
    icon: Option<SharedString>,
    /// Estilo **borderless**: sem borda visível em repouso (e sem sombra nem bisel), com o anel de
    /// foco normal ao abrir. Usa o gatilho compacto (ver [`effective_size`]).
    borderless: bool,
    /// Tamanho pedido explicitamente. `None` = o default da variante (ver [`effective_size`]).
    size: Option<SelectSize>,
    /// Marca o gatilho como **inválido** (`aria-invalid`): borda vermelha, sem sombra e sem bisel.
    invalid: bool,
    /// Desabilitado: não abre e o conjunto esmaece.
    disabled: bool,
    /// Texto mostrado quando não há opção selecionada (lista vazia). Pintado com
    /// `--muted-foreground`.
    placeholder: SharedString,
    /// Gatilho do **tamanho do conteúdo** em vez da largura disponível — o `w-fit min-w-none` que a
    /// referência põe no `SelectTrigger` quando o select entra num grupo.
    fit: bool,
    /// Como o gatilho encosta nos vizinhos, quando o select está num [`crate::group::Group`].
    ///
    /// Aqui a costura é **estado**, e não um campo de builder como no [`crate::Button`]: o `Select` é
    /// uma `Entity` que se renderiza sozinha, então o grupo não tem como embrulhar o render dele —
    /// ele grava a costura na entidade antes de pedir o elemento. Ver a implementação de
    /// `crate::group::GroupChild` pra `Entity<Select>`.
    pub(crate) join: crate::group::Join,
}

impl Select {
    /// Cria um `Select` com `options` e o índice inicial `selected`. Se `selected` estourar
    /// `options`, é clampado ao último índice válido (ou 0 numa lista vazia).
    pub fn new(options: Vec<SharedString>, selected: usize, cx: &mut Context<Self>) -> Self {
        let selected = clamp_idx(selected, options.len());
        Self {
            options,
            selected,
            open: false,
            bounds: Bounds::default(),
            id: cx.entity_id().as_u64(),
            focus_handle: cx.focus_handle(),
            menu_scroll: ScrollHandle::new(),
            icon: None,
            borderless: false,
            size: None,
            invalid: false,
            disabled: false,
            placeholder: SharedString::from("—"),
            fit: false,
            join: crate::group::Join::NONE,
        }
    }

    /// O gatilho passa a ter a largura do **conteúdo**, em vez de ocupar o container e respeitar o
    /// piso de [`MIN_WIDTH`].
    ///
    /// É o `className="w-fit min-w-none"` que os exemplos do coss põem no gatilho sempre que o select
    /// entra num [`crate::group::Group`] — ali a largura é do CONJUNTO, e um select que insiste em
    /// 144px empurra o vizinho (foi o que estourou o layout do `ColorPicker`).
    ///
    /// O popup continua no piso de `MIN_WIDTH`: a lista precisa caber os rótulos inteiros, mesmo que
    /// o gatilho só mostre `$`.
    pub fn fit(mut self) -> Self {
        self.fit = true;
        self
    }

    /// Define um **ícone-prefixo** desenhado DENTRO do gatilho, à esquerda do valor (ex.: o glifo
    /// de blend). Pintado pela `text_color` do gatilho, a 80% de opacidade.
    pub fn with_icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }

    /// Ativa o estilo **borderless**: sem borda, sombra nem bisel em repouso; anel de foco normal
    /// ao abrir. Usa o gatilho compacto ([`SelectSize::Sm`]) salvo se [`Self::size`] disser outro.
    pub fn borderless(mut self) -> Self {
        self.borderless = true;
        self
    }

    /// Tamanho do gatilho.
    pub fn size(mut self, size: SelectSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Marca o gatilho como **inválido**. Só sinaliza — não valida nada; quem valida é quem usa.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Desabilita: o gatilho não abre o menu e o conjunto esmaece (`opacity-64`).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Texto mostrado quando não há opção selecionada (o default é `—`).
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Índice atualmente selecionado.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// As opções (rótulos), na ordem dos índices.
    pub fn options(&self) -> &[SharedString] {
        &self.options
    }

    /// Rótulo da opção selecionada, se houver (lista pode estar vazia).
    pub fn selected_label(&self) -> Option<&SharedString> {
        self.options.get(self.selected)
    }

    /// Se o menu está aberto.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Define o índice de fora (sincronização externa). **Não** emite `Change` — evita loops
    /// de feedback com quem assina (mesma regra do `Switch::set_on`/`Checkbox::set_checked`).
    /// O índice é clampado ao range válido.
    pub fn set_selected(&mut self, selected: usize, cx: &mut Context<Self>) {
        let selected = clamp_idx(selected, self.options.len());
        if self.selected == selected {
            return;
        }
        self.selected = selected;
        cx.notify();
    }

    /// Troca a lista de opções (e re-clampa a seleção). Sincronização externa, **não** emite.
    pub fn set_options(&mut self, options: Vec<SharedString>, cx: &mut Context<Self>) {
        self.selected = clamp_idx(self.selected, options.len());
        self.options = options;
        cx.notify();
    }

    /// Abre/fecha o menu (interação do usuário). Desabilitado não abre.
    fn toggle_open(&mut self, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.open = !self.open;
        cx.notify();
    }

    /// Fecha o menu sem alterar a seleção (Escape / clique fora).
    fn close(&mut self, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        cx.notify();
    }

    /// Escolhe uma opção por interação: fecha o menu e **emite** `Change(idx)` se mudou.
    fn choose(&mut self, idx: usize, cx: &mut Context<Self>) {
        let (selected, open, changed) = apply_choice(self.selected, self.options.len(), idx);
        self.selected = selected;
        self.open = open;
        if changed {
            cx.emit(SelectEvent::Change(idx));
        }
        cx.notify();
    }

    /// O popup (lista de opções) — renderizado em `deferred(anchored(..))` p/ ficar por cima de
    /// tudo e não estourar a janela.
    fn render_menu(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette();
        let width = self.bounds.size.width;
        let selected = self.selected;
        // A altura é limitada ao espaço livre em volta do gatilho; o excedente rola.
        let max_h = menu_max_height(
            f32::from(self.bounds.origin.y),
            f32::from(self.bounds.origin.y + self.bounds.size.height),
            f32::from(window.viewport_size().height),
        );

        let items = self
            .options
            .iter()
            .enumerate()
            .map(|(idx, label)| {
                // A coluna do check existe SEMPRE (é a 1ª coluna do grid `[16px_1fr]` do coss),
                // vazia quando a opção não é a selecionada — é o que mantém todos os rótulos
                // alinhados em vez de deslocar o da opção marcada.
                let check = div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_none()
                    .size(px(CHECK_SLOT))
                    .when(idx == selected, |d| {
                        d.child(
                            svg()
                                .path("icons/check.svg")
                                .size(px(CHECK_SLOT))
                                .flex_none()
                                // `--accent-foreground` é igual a `--foreground` nos dois temas do
                                // coss, então uma cor só serve pro item destacado e pro normal.
                                .text_color(p.text.hsla()),
                        )
                    });

                div()
                    .id(("select-item", idx))
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
                    .text_color(p.text.hsla())
                    .cursor(CursorStyle::PointingHand)
                    // Destacado (hover): fundo `--accent`, texto `--accent-foreground`. O item
                    // SELECIONADO não muda de fundo — quem o marca é o check, como no original.
                    .hover(|s| s.bg(p.accent.hsla()).text_color(p.accent_text.hsla()))
                    .child(check)
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(label.clone()),
                    )
                    .on_click(cx.listener(move |this, _e, window, cx| {
                        crate::focus_ring::pointer_used(window);
                        this.choose(idx, cx);
                    }))
            })
            .collect::<Vec<_>>();

        deferred(
            anchored()
                .snap_to_window_with_margin(px(MENU_MARGIN))
                .child(
                    div()
                        .occlude()
                        // Largura mínima = a do gatilho; o piso de `MIN_WIDTH` cobre o 1º frame,
                        // antes do prepaint ter medido qualquer coisa.
                        .min_w(if width > px(0.0) {
                            width
                        } else {
                            px(MIN_WIDTH)
                        })
                        .mt(px(MENU_GAP))
                        .bg(p.popover_bg.hsla())
                        .text_color(p.text.hsla())
                        .border_1()
                        .border_color(p.border.hsla())
                        .rounded(px(RADIUS))
                        // Sombra EXTERNA atrás de um fundo opaco: aqui a armadilha do
                        // `paint_shadows` não morde (ver [`bevel_for`]) — o retângulo cheio da
                        // sombra fica escondido pelo `--popover`, que é opaco nos dois temas.
                        .shadow_lg()
                        .child(
                            div()
                                .id(("select-menu", self.id))
                                .flex()
                                .flex_col()
                                .p(px(LIST_PAD))
                                .max_h(px(max_h))
                                .overflow_y_scroll()
                                .track_scroll(&self.menu_scroll)
                                .children(items),
                        )
                        .on_mouse_down_out(cx.listener(
                            |this, e: &gpui::MouseDownEvent, window, cx| {
                                // ⚠️ Este handler dispara pra QUALQUER clique fora do popup —
                                // inclusive um no próprio gatilho, que é como se fecha o menu com o
                                // mouse. Sair cedo nesse caso conserta um defeito que existia antes
                                // deste bloco: fechar aqui, na fase de CAPTURA, e o `on_click` do
                                // gatilho alternar na de BOLHA REABRIA o menu — então clicar no
                                // gatilho pra fechar não fechava. Quem trata o clique no gatilho é o
                                // `on_click` dele, e só ele.
                                //
                                // Os bounds vêm da sonda que já existe pra ancorar o menu.
                                if this.bounds.contains(&e.position) {
                                    return;
                                }
                                this.close(cx);
                                // Fora de tudo: além de fechar, tira o foco — é o que o navegador
                                // faz, e sem isso o gatilho ficava com o anel aceso.
                                crate::input::blur_on_outside_click(this.focus_handle.clone())(
                                    e, window, cx,
                                );
                            },
                        )),
                ),
        )
        .with_priority(1)
    }
}

impl EventEmitter<SelectEvent> for Select {}

impl Render for Select {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette();
        let s = effective_size(self.size, self.borderless);
        let open = self.open;
        // O gatilho aberto conta como focado: é ele que carrega o `data-state=open` do coss, e o
        // anel é o que dá a ele o mesmo destaque que o campo de texto tem ao receber o cursor.
        let focused = self.open || self.focus_handle.is_focused(window);

        // O valor: o rótulo selecionado ou o placeholder (que vai em `--muted-foreground`).
        let (title, is_placeholder) = match self.selected_label() {
            Some(label) => (label.clone(), false),
            None => (self.placeholder.clone(), true),
        };

        // --- O gatilho ------------------------------------------------------------------------
        //
        // Praticamente o mesmo campo do `input.rs`: `overflow_hidden` (pra o valor truncar dentro
        // do raio) + borda por estado + fundo + sombra de repouso. O bisel e o anel NÃO entram
        // aqui — são irmãos, montados no wrap abaixo, senão o `overflow_hidden` os recorta.
        let mut trigger = div()
            .id(("select-field", self.id))
            .flex()
            .items_center()
            .gap(px(s.gap()))
            .relative()
            .overflow_hidden()
            .map(|d| {
                if self.fit {
                    d.flex_none()
                } else {
                    d.w_full().min_w(px(MIN_WIDTH))
                }
            })
            .min_h(px(s.min_height()))
            .px(px(s.pad_x()))
            .bg(p.bg.hsla())
            .border_color(trigger_border(self.borderless, self.invalid, focused).hsla())
            .when(!self.disabled, |d| d.cursor(CursorStyle::PointingHand))
            .shadow(shadow_stack_for(
                self.disabled || self.borderless,
                self.invalid,
                focused,
            ));
        // Raio e bordas vêm da COSTURA (ver `crate::group`): solto são os quatro cantos e lados; num
        // grupo, o lado da emenda perde os dois.
        trigger = self.join.rounded(trigger, s.radius());
        trigger = self.join.borders(trigger);

        if let Some(path) = self.icon.clone() {
            trigger = trigger.child(
                svg()
                    .path(path)
                    .size(px(ICON_SIZE))
                    .flex_none()
                    .text_color(p.text.scaled(ICON_OPACITY)),
            );
        }

        trigger = trigger
            // O valor ocupa o espaço e TRUNCA (`flex-1 truncate` no original): sem o `min_w(0)` o
            // texto longo empurraria o chevron pra fora em vez de ser cortado.
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(s.text_size()))
                    .text_color(if is_placeholder { p.placeholder } else { p.text }.hsla())
                    .child(title),
            )
            .child(
                svg()
                    .path("icons/chevron_down.svg")
                    .size(px(ICON_SIZE))
                    .flex_none()
                    .text_color(p.text.scaled(ICON_OPACITY)),
            )
            // Mede e guarda os bounds do gatilho (pra largura/posição/altura do menu) — mesmo
            // truque de `canvas` overlay do `Select` do gpui-component.
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
                .size_full(),
            )
            .on_click(cx.listener(|this, _e, window, cx| {
                // Abrir com o mouse não deve acender anel (ver `crate::focus_ring`).
                crate::focus_ring::pointer_used(window);
                this.toggle_open(cx);
            }));

        // O bisel e o anel são IRMÃOS do gatilho, num container `relative` só pra ancorá-los —
        // ver as duas armadilhas no doc do módulo.
        let mut wrap = div()
            .relative()
            .map(|d| if self.fit { d.flex_none() } else { d.w_full() })
            .child(trigger);
        if let Some(b) = bevel_for(
            self.disabled || self.borderless,
            focused,
            self.invalid,
            s.radius(),
            self.join,
        ) {
            wrap = wrap.child(b);
        }
        // O segundo termo é o `focus-visible`: abrir com o mouse não acende anel, abrir com o
        // teclado acende. Ver `crate::focus_ring`.
        if ring_visible(self.disabled, focused) && crate::focus_ring::visible() {
            wrap = wrap.child(ring_overlay(self.invalid, s.radius(), self.join));
        }

        div()
            .id(("select", self.id))
            .track_focus(&self.focus_handle)
            .relative()
            .w_full()
            // `disabled:opacity-64` — o coss esmaece o conjunto inteiro em vez de trocar cor por
            // cor. (O menu não pode estar aberto aqui: `toggle_open` barra o clique.)
            .when(self.disabled, |d| d.opacity(DISABLED_OPACITY))
            // Escape fecha o menu (quando aberto). Outras teclas são ignoradas.
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _window, cx| {
                if this.open && e.keystroke.key == "escape" {
                    this.close(cx);
                }
            }))
            // Clicar fora tira o foco, como no navegador. Só quando o menu está FECHADO: com ele
            // aberto, um clique numa opção também cai "fora" do gatilho, e aí quem decide é o
            // handler do popup (ver `render_menu`), que sabe distinguir os três casos.
            .when(!open, |d| {
                d.on_mouse_down_out(crate::input::blur_on_outside_click(
                    self.focus_handle.clone(),
                ))
            })
            .child(wrap)
            .when(open, |this| this.child(self.render_menu(window, cx)))
    }
}

impl Focusable for Select {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// =================================================================================================
// Estados visuais (funções livres, pra serem testáveis sem construir um `Select`)
// =================================================================================================

/// Cor da borda do gatilho conforme a precedência de estados do coss.
///
/// A ordem é intencional e é a mesma que os seletores `aria-invalid:`/`focus-visible:` dele impõem:
/// inválido+foco é o estado mais específico, depois inválido, depois foco, depois repouso.
///
/// `disabled` NÃO entra: no coss o gatilho desabilitado mantém as cores e o conjunto todo recebe
/// `opacity-64`. Ter um par "apagado" de cada token além disso só daria duas fontes de verdade.
pub(crate) fn border_color_for(invalid: bool, focused: bool) -> Rgba8 {
    let p = palette();
    match (invalid, focused) {
        (true, true) => p.danger_border_focus,
        (true, false) => p.danger_border,
        (false, true) => p.ring,
        (false, false) => p.border,
    }
}

/// A borda do gatilho, já com o desconto do **borderless**.
///
/// O borderless não existe no coss: é a variante dos blocos densos do inspector, onde o field só
/// se revela quando você interage com ele. Em repouso a borda vira a cor do FUNDO (e não
/// `transparent`, que deixaria o fundo do painel vazar 1px em volta); nos outros estados ela é a
/// mesma de qualquer gatilho — inclusive a de inválido, que é informação demais pra esconder.
fn trigger_border(borderless: bool, invalid: bool, focused: bool) -> Rgba8 {
    if borderless && !invalid && !focused {
        palette().bg
    } else {
        border_color_for(invalid, focused)
    }
}

/// A sombra externa do gatilho (`shadow-xs/5`).
///
/// Presente só em REPOUSO: o coss a apaga quando o gatilho está desabilitado, focado/aberto ou
/// inválido (`disabled:shadow-none`, `aria-invalid:shadow-none`, e o foco a substitui pelo anel).
/// `flat` cobre os casos em que o gatilho não tem relevo nenhum — desabilitado e borderless.
pub(crate) fn shadow_stack_for(flat: bool, invalid: bool, focused: bool) -> Vec<gpui::BoxShadow> {
    if flat || invalid || focused {
        return Vec::new();
    }
    // `shadow-xs/5` = 0 1px 2px rgba(0,0,0,.05)
    vec![gpui::BoxShadow {
        color: palette().shadow.hsla(),
        offset: gpui::point(px(0.0), px(1.0)),
        blur_radius: px(2.0),
        spread_radius: px(0.0),
    }]
}

/// Quando o anel de foco de 3px está na árvore: ao focar ou abrir, nunca desabilitado.
/// A modalidade do input NÃO entra aqui de propósito: este predicado é o estado do componente, e
/// quem combina com `crate::focus_ring::visible()` é o call site. Manter separado é o que deixa isto
/// testável sem estado global.
pub(crate) fn ring_visible(disabled: bool, focused: bool) -> bool {
    !disabled && focused
}

/// O overlay do anel de foco.
///
/// ⚠️ **`ring` não existe no GPUI, e sombra não serve.** É o `focus-visible:ring-[3px]` do coss, e
/// tentá-lo como `BoxShadow` erra por dois motivos, os dois vindos do `Window::paint_shadows`:
///
/// 1. A sombra é pintada como um retângulo arredondado CHEIO **atrás** do elemento. Como o fundo do
///    gatilho é translúcido no tema escuro (`bg-input/32` ≈ 2,5% de branco), o anel inteiro
///    atravessaria e tingiria o fundo do gatilho ao abrir.
/// 2. O `spread_radius` dilata os limites mas **mantém o raio**, então a curvatura sairia errada
///    nas quinas.
///
/// Um `ring` do Tailwind é geometricamente um anel: a forma do elemento dilatada em 3px, com o raio
/// externo crescendo junto. Então é isso que se desenha — um overlay 3px maior em cada lado, com
/// borda de 3px e raio `radius + 3`. Fica **fora** do gatilho (irmão, não filho), porque o gatilho
/// tem `overflow_hidden` e recortaria o anel.
pub(crate) fn ring_overlay(invalid: bool, radius: f32, join: crate::group::Join) -> Div {
    let p = palette();
    let cor = if invalid { p.danger_glow } else { p.ring_glow };
    // Num grupo o anel PARA na emenda; quem fecha o contorno ali é o separador, que vira a cor do
    // anel quando o campo vizinho está focado. Ver o doc de `crate::group`.
    join.ring_overlay(-RING_WIDTH, radius + RING_WIDTH)
        .border_3()
        .border_color(cor.hsla())
}

/// O **fio de bisel** de 1px sobreposto ao gatilho.
///
/// ⚠️ **A técnica do coss não traduz pro GPUI.** Lá é um pseudo-elemento transparente cobrindo a
/// padding box, com `box-shadow: 0 ±1px <cor>`: só o filete que ESCAPA fica visível, porque o CSS
/// nunca pinta a sombra embaixo da border box de quem a projeta. O `Window::paint_shadows` do GPUI
/// insere a sombra como um retângulo arredondado **completo**, sem recortar a área do próprio
/// elemento — num overlay transparente isso vira uma lavagem da cor sobre o gatilho INTEIRO. É o
/// bug que o `input.rs` já pagou; ver `crate::input::bevel_for`.
///
/// Então o filete é desenhado como o que ele é: uma **borda de 1px num único lado** de um overlay
/// que cobre a BORDER box do gatilho e é **irmão** dele (como filho, o `overflow_hidden` do gatilho
/// deixaria só um arco em cada quina).
///
/// Some quando o gatilho está desabilitado/borderless, focado/aberto ou inválido — é o
/// `not-disabled:not-focus-visible:not-aria-invalid:before:shadow-*` do original.
pub(crate) fn bevel_for(
    flat: bool,
    focused: bool,
    invalid: bool,
    radius: f32,
    join: crate::group::Join,
) -> Option<Div> {
    let p = palette();
    if flat || focused || invalid {
        return None;
    }
    // Na emenda o filete avança meio pixel pra dentro do vizinho, pra encontrar o dele.
    let overlay = join.bevel_overlay(0.0, radius).border_color(p.bevel.hsla());
    // Claro: filete escuro embaixo. Escuro: filete claro em cima.
    Some(if p.bevel_dir > 0.0 {
        overlay.border_b_1()
    } else {
        overlay.border_t_1()
    })
}

/// Altura máxima do popup: o maior espaço livre em volta do gatilho, com piso.
///
/// O `anchored().snap_to_window_with_margin` decide se o menu abre pra baixo ou pra cima, mas
/// **não** o encolhe — sem um teto ele simplesmente estoura a janela. Então o teto é o maior dos
/// dois espaços (que é o lado que o `anchored` vai escolher), descontadas a margem da janela e a
/// distância até o gatilho.
fn menu_max_height(field_top: f32, field_bottom: f32, viewport_height: f32) -> f32 {
    let abaixo = viewport_height - field_bottom - MENU_GAP - MENU_MARGIN;
    let acima = field_top - MENU_GAP - MENU_MARGIN;
    abaixo.max(acima).max(MENU_MIN_MAX_HEIGHT)
}

/// Clampa um índice ao range válido de uma lista de tamanho `len` (último índice se estourar,
/// 0 se vazia). Lógica pura — testável sem GPU.
fn clamp_idx(idx: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        idx.min(len - 1)
    }
}

/// Aplica a escolha de uma opção sobre o estado `(selected, open)`, devolvendo
/// `(novo_selected, novo_open, mudou)`. É o miolo puro do método [`Select::choose`]
/// (fecha o menu; troca o índice só se for válido e diferente) — extraído pra ser
/// testável sem `Context`/GPU. `mudou` indica se um `Change` seria emitido.
fn apply_choice(selected: usize, len: usize, idx: usize) -> (usize, bool, bool) {
    if idx < len && idx != selected {
        (idx, false, true)
    } else {
        (selected, false, false)
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `frame.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    // O estado/seleção do `Select` é lógica pura (sem GPU/janela): testamos os helpers
    // `clamp_idx`/`apply_choice` — o miolo de `new`/`set_selected`/`choose` — sem precisar
    // construir um `Select` real (que exige um `FocusHandle` do app). O mesmo vale pros estados
    // visuais, que são funções livres.

    #[test]
    fn clamp_index_respeita_o_range() {
        assert_eq!(clamp_idx(0, 3), 0);
        assert_eq!(clamp_idx(2, 3), 2);
        assert_eq!(clamp_idx(5, 3), 2); // estoura → último válido
        assert_eq!(clamp_idx(0, 0), 0); // lista vazia → 0
        assert_eq!(clamp_idx(9, 0), 0);
    }

    #[test]
    fn choose_troca_e_marca_mudanca() {
        // Escolher um índice válido e diferente: muda a seleção, fecha, sinaliza Change.
        assert_eq!(apply_choice(0, 3, 2), (2, false, true));
        // Re-escolher o atual: não muda, fecha, sem Change (evita evento redundante).
        assert_eq!(apply_choice(2, 3, 2), (2, false, false));
        // Índice fora do range: ignorado (mantém seleção), só fecha.
        assert_eq!(apply_choice(1, 3, 9), (1, false, false));
        // Lista vazia: nada a escolher.
        assert_eq!(apply_choice(0, 0, 0), (0, false, false));
    }

    #[test]
    fn apply_choice_sempre_fecha_o_menu() {
        // Independente de mudar ou não, o segundo campo (open) volta false.
        for (sel, len, idx) in [(0, 3, 1), (2, 3, 2), (0, 3, 9)] {
            let (_, open, _) = apply_choice(sel, len, idx);
            assert!(!open);
        }
    }

    /// **A convenção de cor da paleta, decodificada de verdade.**
    ///
    /// Todo valor de [`SelectPalette`] é `0xRRGGBBAA` e é consumido por `rgba`; um valor de 6
    /// dígitos esquecido ali vira uma cor completamente diferente, **sem erro de compilação** —
    /// `rgba(0xffffff)` é lido como `0x00FFFFFF`, ou seja ciano. Já aconteceu três vezes no campo
    /// de texto. Em vez de comparar números com números (que não pegaria nada), este teste
    /// decodifica e afirma o que a cor DEVE ser perceptualmente.
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        // Fundo do gatilho no claro e popup nos dois temas: OPACOS (é o que segura a sombra).
        let bg: gpui::Rgba = SELECT_LIGHT.bg.hsla().into();
        assert_eq!(
            (bg.r, bg.g, bg.b, bg.a),
            (1.0, 1.0, 1.0, 1.0),
            "claro: branco opaco"
        );
        for (nome, c) in [
            ("popup claro", SELECT_LIGHT.popover_bg),
            ("popup escuro", SELECT_DARK.popover_bg),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome} tem que ser opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome} é NEUTRO: se r≠g≠b, o valor foi lido deslocado"
            );
        }
        let popup_claro: gpui::Rgba = SELECT_LIGHT.popover_bg.hsla().into();
        let popup_escuro: gpui::Rgba = SELECT_DARK.popover_bg.hsla().into();
        assert!(popup_claro.r > 0.9, "popup do tema claro é claro");
        assert!(popup_escuro.r < 0.2, "popup do tema escuro é escuro");
        // `--background` do tema escuro é #141414 (20/255): o popup (#1d1d1d) LEVANTA sobre ele.
        // Se o `--popover` fosse copiado do `--background`, o menu sumiria contra o painel.
        assert!(
            popup_escuro.r > 20.0 / 255.0,
            "o popup escuro é mais claro que o --background (#141414)"
        );

        // Texto e placeholder: neutros e opacos, o placeholder mais apagado que o texto.
        for (nome, c) in [
            ("texto claro", SELECT_LIGHT.text),
            ("texto escuro", SELECT_DARK.text),
            ("placeholder claro", SELECT_LIGHT.placeholder),
            ("placeholder escuro", SELECT_DARK.placeholder),
            ("accent-fg claro", SELECT_LIGHT.accent_text),
            ("accent-fg escuro", SELECT_DARK.accent_text),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome} é NEUTRO: se r≠g≠b, o valor foi lido deslocado"
            );
        }
        let txt_claro: gpui::Rgba = SELECT_LIGHT.text.hsla().into();
        let ph_claro: gpui::Rgba = SELECT_LIGHT.placeholder.hsla().into();
        assert!(txt_claro.r < 0.2, "texto do claro é quase preto");
        assert!(ph_claro.r > txt_claro.r, "o placeholder é MAIS claro que o texto");
        let txt_escuro: gpui::Rgba = SELECT_DARK.text.hsla().into();
        let ph_escuro: gpui::Rgba = SELECT_DARK.placeholder.hsla().into();
        assert!(txt_escuro.r > 0.8, "texto do escuro é quase branco");
        assert!(ph_escuro.r < txt_escuro.r, "o placeholder é MAIS escuro que o texto");

        // Anel de foco: cinza neutro e OPACO nos dois temas — o sintoma do bug histórico era ele
        // sair teal, porque `0xa3a3a3` lido como rgba perde o canal vermelho.
        for (nome, ring) in [("claro", SELECT_LIGHT.ring), ("escuro", SELECT_DARK.ring)] {
            let c: gpui::Rgba = ring.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: anel opaco");
            assert!(c.r > 0.1, "{nome}: anel sem canal vermelho — leitura deslocada");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: anel é cinza NEUTRO"
            );
        }

        // Inválido: vermelho de verdade (r bem maior que g e b) nos dois temas.
        for (nome, c) in [
            ("claro", SELECT_LIGHT.danger_border_focus),
            ("escuro", SELECT_DARK.danger_border_focus),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert!(
                c.r > 0.7 && c.r > c.g + 0.3 && c.r > c.b + 0.3,
                "{nome}: a borda de inválido é VERMELHA"
            );
        }

        // Os tokens que DEVEM ser translúcidos continuam translúcidos — é isso que os faz
        // funcionar sobre qualquer fundo.
        for (nome, c) in [
            ("borda claro", SELECT_LIGHT.border),
            ("borda escuro", SELECT_DARK.border),
            ("halo claro", SELECT_LIGHT.ring_glow),
            ("halo escuro", SELECT_DARK.ring_glow),
            ("halo de erro claro", SELECT_LIGHT.danger_glow),
            ("halo de erro escuro", SELECT_DARK.danger_glow),
            ("accent claro", SELECT_LIGHT.accent),
            ("accent escuro", SELECT_DARK.accent),
            ("bisel claro", SELECT_LIGHT.bevel),
            ("bisel escuro", SELECT_DARK.bevel),
            ("sombra claro", SELECT_LIGHT.shadow),
            ("fundo escuro", SELECT_DARK.bg),
        ] {
            assert!(
                c.alpha() < 1.0,
                "{nome} tem que ser translúcido, veio com alfa {}",
                c.alpha()
            );
        }

        // O destaque de item é MAIS sutil que a borda do gatilho (`--accent` 4% vs `--input`
        // 10%/8%): ele é um realce de fundo, não uma moldura.
        assert!(SELECT_LIGHT.accent.alpha() < SELECT_LIGHT.border.alpha());
        assert!(SELECT_DARK.accent.alpha() < SELECT_DARK.border.alpha());

        // O bisel troca de sentido entre os temas.
        assert!(SELECT_LIGHT.bevel_dir > 0.0);
        assert!(SELECT_DARK.bevel_dir < 0.0);
    }

    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, porque a 6% o filete é
    /// imperceptível no nosso fundo. É o mesmo desvio já vigente no `input.rs` e no `card.rs` —
    /// o valor tem que ser o MESMO nos três, senão o gatilho do dropdown e o do campo de texto
    /// ficam lado a lado com relevos diferentes. Se alguém "corrigir" pra 6% achando que é erro de
    /// porte, este teste falha e aponta pra cá.
    ///
    /// O tema CLARO segue fiel (preto 4%) — o desvio é só no escuro, onde o problema existia.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = SELECT_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%)"
        );

        let escuro = SELECT_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    /// Precedência da borda, na ordem que os seletores do coss impõem: inválido+foco é o estado
    /// mais específico, depois inválido, depois foco, depois repouso.
    #[test]
    fn precedencia_da_cor_de_borda() {
        theme::set_theme(theme::ThemeMode::Dark);
        let p = palette();

        assert_eq!(border_color_for(true, true), p.danger_border_focus);
        assert_eq!(border_color_for(true, false), p.danger_border);
        assert_eq!(border_color_for(false, true), p.ring);
        assert_eq!(border_color_for(false, false), p.border);

        // Inválido vence o foco: a validade é informação mais importante que "onde está o cursor".
        assert_ne!(border_color_for(true, true), p.ring);
        // E a borda de inválido com foco é MAIS forte que a sem foco (36% → 64%).
        assert!(p.danger_border_focus.alpha() > p.danger_border.alpha());
    }

    /// O **borderless** esconde a borda só em repouso: focado/aberto ele ganha o anel como
    /// qualquer gatilho, e inválido continua vermelho — esconder validade seria pior que o vão
    /// visual que a variante quer evitar.
    #[test]
    fn borderless_esconde_a_borda_so_em_repouso() {
        theme::set_theme(theme::ThemeMode::Dark);
        let p = palette();

        assert_eq!(trigger_border(true, false, false), p.bg, "repouso: invisível");
        assert_eq!(trigger_border(true, false, true), p.ring, "aberto: anel");
        assert_eq!(trigger_border(true, true, false), p.danger_border, "inválido aparece");
        // Com borda, repouso é a borda normal — a variante não muda mais nada.
        assert_eq!(trigger_border(false, false, false), p.border);
    }

    /// Alturas 28/32/36 (o `sm:min-h-*` do coss) e o respiro menor no `Sm`. Um `Lg` mais baixo que
    /// um `Md` passaria batido no código e só apareceria na tela.
    #[test]
    fn alturas_e_respiro_por_tamanho() {
        let (sm, md, lg) = (SelectSize::Sm, SelectSize::Md, SelectSize::Lg);

        assert_eq!(
            (sm.min_height(), md.min_height(), lg.min_height()),
            (28.0, 32.0, 36.0)
        );
        assert!(sm.min_height() < md.min_height() && md.min_height() < lg.min_height());

        // O `Sm` é o único com respiro e vão apertados.
        assert_eq!((sm.pad_x(), md.pad_x(), lg.pad_x()), (9.0, 11.0, 11.0));
        assert!(sm.pad_x() < md.pad_x(), "o Sm tem o padding MENOR");
        assert_eq!(md.pad_x(), lg.pad_x(), "no coss o Md e o Lg têm o mesmo respiro");
        assert_eq!((sm.gap(), md.gap(), lg.gap()), (6.0, 8.0, 8.0));

        assert_eq!(SelectSize::default(), SelectSize::Md);
    }

    /// **O corpo do texto e o raio são os MESMOS nos três tamanhos.** No coss eles vêm de fora das
    /// variantes (`text-sm`, `rounded-lg`), que mudam só altura e respiro. Isto é contra-intuitivo
    /// — parece esquecimento — então fica travado aqui pra ninguém "consertar" de volta pra uma
    /// escala por tamanho e sair do pixel-perfect.
    #[test]
    fn corpo_e_raio_nao_variam_com_o_tamanho() {
        for t in [SelectSize::Sm, SelectSize::Md, SelectSize::Lg] {
            assert_eq!(t.text_size(), TEXT_SIZE);
            assert_eq!(t.radius(), RADIUS);
        }
        assert_eq!(TEXT_SIZE, 14.0);
        assert_eq!(RADIUS, 10.0, "--radius-lg");
        assert_eq!(ITEM_RADIUS, 6.0, "--radius-sm");
        assert!(ITEM_RADIUS < RADIUS, "o item arredonda MENOS que o popup");
    }

    /// O tamanho efetivo: o explícito ganha sempre; sem ele, o borderless é compacto e o resto é
    /// `Md`. É o que preserva a densidade dos blocos do inspector, que usam `.borderless()`.
    #[test]
    fn tamanho_efetivo_por_variante() {
        assert_eq!(effective_size(None, false), SelectSize::Md);
        assert_eq!(effective_size(None, true), SelectSize::Sm, "borderless é compacto");
        // O explícito vence a variante, nos dois sentidos.
        assert_eq!(effective_size(Some(SelectSize::Lg), true), SelectSize::Lg);
        assert_eq!(effective_size(Some(SelectSize::Sm), false), SelectSize::Sm);
    }

    /// A geometria do item: grid `[16px_1fr]` com `gap-2`, `min-h-7`, e o respiro do FIM maior que
    /// o do início (`ps-2 pe-4`) — o inverso deixaria o rótulo encostado na borda direita.
    #[test]
    fn geometria_do_item() {
        assert_eq!(ITEM_MIN_HEIGHT, 28.0, "sm:min-h-7");
        assert_eq!(CHECK_SLOT, 16.0, "a 1ª coluna do grid");
        assert_eq!(ITEM_GAP, 8.0, "gap-2");
        assert_eq!(ITEM_PAD_Y, 4.0, "py-1");
        assert!(
            ITEM_PAD_END > ITEM_PAD_START,
            "pe-4 > ps-2: sobra espaço à direita do rótulo"
        );
        assert_eq!(LIST_PAD, 4.0, "p-1 na lista");
    }

    /// A sombra de repouso existe SÓ em repouso, e o anel só ao focar/abrir — os dois nunca
    /// aparecem juntos (no coss um substitui o outro).
    #[test]
    fn sombra_e_anel_se_excluem() {
        theme::set_theme(theme::ThemeMode::Dark);

        // Repouso: uma sombra `shadow-xs`, sem anel.
        let repouso = shadow_stack_for(false, false, false);
        assert_eq!(repouso.len(), 1);
        assert_eq!(repouso[0].offset.y, px(1.0), "shadow-xs desce 1px");
        assert_eq!(repouso[0].blur_radius, px(2.0));
        assert!(!ring_visible(false, false));

        // Focado/aberto: anel, sem sombra.
        assert!(shadow_stack_for(false, false, true).is_empty());
        assert!(ring_visible(false, true));

        // Inválido apaga a sombra mesmo sem foco (`aria-invalid:shadow-none`).
        assert!(shadow_stack_for(false, true, false).is_empty());

        // Desabilitado e borderless (o `flat`): sem sombra nunca, e sem anel se desabilitado.
        assert!(shadow_stack_for(true, false, false).is_empty());
        assert!(!ring_visible(true, true), "desabilitado não mostra anel");
    }

    /// O bisel só aparece em REPOUSO — foco/aberto, inválido e desabilitado/borderless o apagam.
    #[test]
    fn bisel_so_aparece_em_repouso() {
        theme::set_theme(theme::ThemeMode::Dark);
        assert!(bevel_for(false, false, false, RADIUS, crate::group::Join::NONE).is_some(), "repouso: aparece");
        assert!(bevel_for(false, true, false, RADIUS, crate::group::Join::NONE).is_none(), "aberto: some");
        assert!(bevel_for(false, false, true, RADIUS, crate::group::Join::NONE).is_none(), "inválido: some");
        assert!(
            bevel_for(true, false, false, RADIUS, crate::group::Join::NONE).is_none(),
            "desabilitado/borderless: some"
        );
    }

    /// A altura máxima do popup segue o maior espaço livre, com piso — um gatilho colado no rodapé
    /// abriria um menu de altura ~0 e ficaria inclicável.
    #[test]
    fn altura_maxima_do_popup_segue_o_espaco_livre() {
        // Gatilho no topo de uma janela de 600px: sobra quase tudo pra baixo.
        let alto = menu_max_height(20.0, 52.0, 600.0);
        assert!((alto - (600.0 - 52.0 - MENU_GAP - MENU_MARGIN)).abs() < 1e-4);

        // Gatilho no rodapé: o espaço de CIMA é que vale.
        let baixo = menu_max_height(540.0, 572.0, 600.0);
        assert!((baixo - (540.0 - MENU_GAP - MENU_MARGIN)).abs() < 1e-4);

        // Janela minúscula: o piso segura, mesmo que estoure a margem.
        assert_eq!(menu_max_height(10.0, 42.0, 60.0), MENU_MIN_MAX_HEIGHT);
        // Antes do 1º prepaint os bounds são zero — não pode virar altura negativa.
        assert!(menu_max_height(0.0, 0.0, 0.0) >= MENU_MIN_MAX_HEIGHT);
    }

    /// O gatilho tem largura mínima (`min-w-36`) pra um valor curto não colapsar o campo num
    /// quadradinho ao lado do chevron.
    #[test]
    fn gatilho_tem_largura_minima() {
        assert_eq!(MIN_WIDTH, 144.0, "min-w-36");
        // A largura mínima tem que caber o maior gatilho com folga pro rótulo.
        assert!(MIN_WIDTH > 2.0 * ICON_SIZE + 2.0 * SelectSize::Lg.pad_x());
    }
}

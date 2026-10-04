//! `theme` — os **tokens visuais** do `empire-ui`.
//!
//! Os tokens de **chrome** (backgrounds, borders, textos/ícones, tints, accent, overlays)
//! são **theme-aware**: resolvem pelo tema ATIVO em runtime (claro/escuro). Internamente
//! eles são funções que leem a [`Palette`] corrente (um `thread_local`), mas mantêm o
//! **mesmo nome em MAIÚSCULAS** dos antigos `const` — só que chamados com `()`:
//! `rgb(theme::BG_PANEL())`.
//!
//! Já radii e fontes continuam `const` fixos (não dependem do tema).
//!
//! ## Convenção de cores
//! O GPUI recebe cor via [`gpui::rgb`] (hex opaco `0xRRGGBB` → [`Rgba`] com `a = 1.0`)
//! e [`gpui::rgba`] (hex com alpha `0xRRGGBBAA`). Por isso:
//!
//! - **Cores opacas** são `u32` (`0xRRGGBB`) e se usam com `rgb(...)`:
//!   `.bg(rgb(theme::BG_PANEL()))`.
//! - **Cores com alpha** (ex.: a borda branca a 5%) vêm de **helpers** que já
//!   devolvem [`Rgba`], pra usar direto: `.border_color(theme::border_subtle())`.
//!
//! ## Tema (claro/escuro)
//! [`set_theme`] troca o modo corrente da thread de UI (chame uma vez por frame no topo
//! do `render` do shell). Todos os tokens de chrome resolvem nesse modo. Persistência em
//! disco está fora de escopo (reseta pra Dark ao reabrir).
//!
//! ## Estendendo a paleta na sua app
//! Uma app com tokens PRÓPRIOS (ex.: as cores de timeline do editor Fennel) não deve
//! inflar esta [`Palette`]. O padrão é: declarar o seu próprio struct de paleta, com um
//! par de consts dark/light, e resolvê-lo lendo [`theme_mode`] — assim o seu tema troca
//! junto com o do `empire-ui`, sem acoplamento nos dois sentidos:
//!
//! ```ignore
//! struct MinhaPaleta { grafico_linha: u32 }
//! const MINHA_DARK: MinhaPaleta = MinhaPaleta { grafico_linha: 0x8a8a8a };
//! const MINHA_LIGHT: MinhaPaleta = MinhaPaleta { grafico_linha: 0x52525b };
//!
//! fn minha() -> &'static MinhaPaleta {
//!     match empire_ui::theme::theme_mode() {
//!         empire_ui::theme::ThemeMode::Dark => &MINHA_DARK,
//!         empire_ui::theme::ThemeMode::Light => &MINHA_LIGHT,
//!     }
//! }
//! ```
#![allow(non_snake_case)]

use gpui::{rgba, Rgba};
use std::cell::Cell;

// ===================== Palette (tema) =====================

/// Conjunto completo de tokens de **chrome** que mudam com o tema. Os campos opacos
/// são `u32` no formato `0xRRGGBB` (usados com `rgb`); os de alpha são `u32` no
/// formato `0xRRGGBBAA` (usados com `rgba`).
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    // Backgrounds
    pub bg_app: u32,
    pub bg_panel: u32,
    /// Superfície **rebaixada** — um tom ABAIXO do `bg_app`, pro painel de fundo de uma
    /// área secundária (no Fennel: o painel da timeline).
    pub bg_sunken: u32,
    pub bg_topbar: u32,
    pub bg_field: u32,
    // Borders (alpha = 0xRRGGBBAA)
    pub border_subtle: u32,
    pub border_divider: u32,
    /// Borda de uma superfície rebaixada ([`Palette::bg_sunken`]) — mais marcada que a hairline.
    pub border_sunken: u32,
    pub border_group: u32,
    // White/black tints (alpha = 0xRRGGBBAA) — ícones translúcidos
    pub tint_strong: u32, // white_75 no dark
    pub tint_active: u32, // white_70 no dark
    pub tint_muted: u32,  // white_30 no dark
    // Text / icons
    pub text_title: u32,
    pub text_label: u32,
    pub text_value: u32,
    pub text_muted: u32,
    pub icon: u32,
    pub icon_scrub: u32,
    // Accent
    pub accent_light: u32,
    pub primary: u32,
    pub on_primary: u32,
    pub switch_knob_on: u32,
    /// Trilho do [`crate::switch::Switch`] **desligado** — cinza neutro escuro no dark; cinza BEM
    /// CLARO (perto do fundo, mas visível) no light. A bolinha (off) é branca nos dois.
    pub switch_track_off: u32,
    pub primary_hover: u32,
    // --- extras de chrome (cards / popovers / modais) ---
    /// Fundo de card/popover flutuante (modais, pickers) — `#161619` dark.
    pub bg_overlay: u32,
    /// Fundo de "input box"/pill discreta dentro de overlays — `#26262B` dark.
    pub bg_inset: u32,
    /// Hover de item de lista/inset — `#35353D` dark.
    pub bg_inset_hover: u32,
    /// Borda de overlay/popover (mais marcada que a hairline) — `#32323A` dark.
    pub border_overlay: u32,
    /// Borda em estado de foco/aberto (chips, pickers) — `#3A3A40` dark.
    pub border_strong: u32,
    /// Texto forte sobre overlay (títulos de modal) — `#F1F1F3` dark.
    pub text_strong: u32,
    /// Texto secundário/legenda dentro de overlays — `#9A9AA2` dark.
    pub text_dim: u32,
    /// Texto bem apagado (placeholders, hints) — `#777780` dark.
    pub text_faint: u32,
    /// Backdrop translúcido atrás de modais (cobre a janela) — `#000000AA` dark.
    pub scrim: u32,
    // --- Semânticos de feedback (estado de um controle, não "cor de marca") ---
    /// Anel/borda de **foco** de um controle editável. Azul nos dois temas — é a convenção
    /// que o usuário já reconhece, e destaca bem sobre `bg_field` claro ou escuro.
    pub focus_ring: u32,
    /// **Erro / inválido** — borda do campo e texto da mensagem.
    pub danger: u32,
    /// **Sucesso / válido** — idem.
    pub success: u32,
    /// **Aviso** — para estados que não bloqueiam (ex.: perto do limite de caracteres).
    pub warning: u32,
}

/// Paleta do tema **ESCURO** (default).
pub const DARK: Palette = Palette {
    bg_app: 0x0a0a0a,
    bg_panel: 0x121214,
    bg_sunken: 0x0e0e0e,
    bg_topbar: 0x1a1a1a,
    bg_field: 0x1c1c1c,
    border_subtle: 0xffffff0d,
    border_divider: 0xffffff1a,
    border_sunken: 0x202020,
    border_group: 0x1f1f1f,
    tint_strong: 0xffffffbf,
    tint_active: 0xffffffb3,
    tint_muted: 0xffffff4d,
    text_title: 0xf2f2f2,
    text_label: 0xbcbcbc,
    text_value: 0xe6e6e6,
    text_muted: 0x9b9b9b,
    icon: 0x9b9b9b,
    icon_scrub: 0x8a8a8a,
    accent_light: 0xf3f3f3,
    primary: 0xf3f3f3,
    on_primary: 0x101012,
    switch_knob_on: 0x1d1d1d,
    switch_track_off: 0x3a3a40,
    primary_hover: 0xe2e2e2,
    bg_overlay: 0x161619,
    bg_inset: 0x26262b,
    bg_inset_hover: 0x35353d,
    border_overlay: 0x32323a,
    border_strong: 0x3a3a40,
    text_strong: 0xf1f1f3,
    text_dim: 0x9a9aa2,
    text_faint: 0x777780,
    scrim: 0x000000aa,
    focus_ring: 0x3b82f6,
    danger: 0xf87171,
    success: 0x4ade80,
    warning: 0xfbbf24,
};

/// Paleta do tema **CLARO** (neutro/limpo). Inverte o acento: `primary` vira quase-PRETO
/// e `on_primary` quase-branco, pra os botões preenchidos lerem sobre fundo claro.
/// White-tints viram **black-tints** (ícones translúcidos pretos).
pub const LIGHT: Palette = Palette {
    bg_app: 0xe8e8ea,
    bg_panel: 0xffffff,
    bg_sunken: 0xf4f4f5,
    bg_topbar: 0xffffff,
    bg_field: 0xf1f1f3,
    border_subtle: 0x0000000d,
    border_divider: 0x0000001a,
    border_sunken: 0xe4e4e7,
    border_group: 0xebebed,
    // black-tints: ícone forte ~ preto 75%, ativo ~ preto 60%, inativo ~ preto 30%
    tint_strong: 0x000000bf,
    tint_active: 0x00000099,
    tint_muted: 0x0000004d,
    text_title: 0x18181b,
    text_label: 0x3f3f46,
    text_value: 0x27272a,
    text_muted: 0x71717a,
    icon: 0x71717a,
    icon_scrub: 0x52525b,
    accent_light: 0x18181b,
    primary: 0x18181b,
    on_primary: 0xfafafa,
    switch_knob_on: 0xfafafa,
    switch_track_off: 0xe4e4e7,
    primary_hover: 0x2a2a2e,
    bg_overlay: 0xffffff,
    bg_inset: 0xf1f1f3,
    bg_inset_hover: 0xe6e6e9,
    border_overlay: 0xdcdce0,
    border_strong: 0xc8c8cd,
    text_strong: 0x18181b,
    text_dim: 0x52525b,
    text_faint: 0x9a9aa2,
    scrim: 0x0000004d,
    // Mesmo azul de foco (convenção); erro/sucesso/aviso escurecem pra contrastar com fundo claro.
    focus_ring: 0x3b82f6,
    danger: 0xdc2626,
    success: 0x16a34a,
    warning: 0xd97706,
};

/// O modo de tema ativo. Estado em memória (sem persistência).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Dark,
    Light,
}

thread_local! {
    /// Modo corrente da thread de UI. A UI renderiza numa única thread, então um
    /// `thread_local` basta. Default = [`ThemeMode::Dark`].
    static CURRENT_MODE: Cell<ThemeMode> = const { Cell::new(ThemeMode::Dark) };
}

/// Troca o tema corrente da thread de UI. Chame no INÍCIO do `render` do shell (antes de
/// montar a UI), pra todos os tokens resolverem no tema do frame.
pub fn set_theme(mode: ThemeMode) {
    CURRENT_MODE.with(|c| c.set(mode));
}

/// O modo de tema corrente da thread. Use pra resolver **paletas próprias da sua app**
/// no mesmo tema do `empire-ui` (ver o exemplo no doc do módulo).
pub fn theme_mode() -> ThemeMode {
    CURRENT_MODE.with(|c| c.get())
}

/// Troca o tema **e** sincroniza o do `gpui-component`, num passo só.
///
/// Use esta em vez de [`set_theme`] sempre que a app tiver um toggle de tema. Ela faz as três
/// coisas que precisam andar juntas:
///
/// 1. o modo dos nossos tokens ([`set_theme`]);
/// 2. o tema do `gpui-component` (os `Input` internos leem dele);
/// 3. as cores de texto/placeholder do núcleo, que o elemento de texto pinta com
///    `cx.theme().foreground`/`.muted_foreground` e que um `.text_color()` do nosso lado **não**
///    alcança.
///
/// Esquecer o item 3 dá um campo com moldura no tema novo e texto no tema antigo.
pub fn sync_core_theme(mode: ThemeMode, window: &mut gpui::Window, cx: &mut gpui::App) {
    set_theme(mode);
    let gc = match mode {
        ThemeMode::Dark => gpui_component::ThemeMode::Dark,
        ThemeMode::Light => gpui_component::ThemeMode::Light,
    };
    gpui_component::Theme::change(gc, Some(window), cx);
    crate::input::apply_core_text_colors(cx);
}

/// A paleta corrente da thread.
pub fn palette() -> &'static Palette {
    match theme_mode() {
        ThemeMode::Dark => &DARK,
        ThemeMode::Light => &LIGHT,
    }
}

/// Atalho interno — mesmo papel do antigo `current()`.
fn current() -> &'static Palette {
    palette()
}

// ===================== Backgrounds (fundos) =====================

/// Fundo geral da app — a base, atrás de tudo.
pub fn BG_APP() -> u32 {
    current().bg_app
}

/// Fundo dos painéis laterais.
pub fn BG_PANEL() -> u32 {
    current().bg_panel
}

/// Fundo de uma superfície **rebaixada** — um tom abaixo do [`BG_APP`].
pub fn BG_SUNKEN() -> u32 {
    current().bg_sunken
}

/// Fundo da top bar.
pub fn BG_TOPBAR() -> u32 {
    current().bg_topbar
}

/// Fundo de campo de input.
pub fn BG_FIELD() -> u32 {
    current().bg_field
}

// ===================== Borders (bordas / divisórias) =====================

/// Borda sutil = hairline padrão entre shells (bordas de painel, divisória da top
/// bar, divisórias arrastáveis). Branco 5% no dark, preto 5% no claro.
pub fn border_subtle() -> Rgba {
    rgba(current().border_subtle)
}

/// Divisória de célula — um pouco mais forte que a hairline [`border_subtle`] (10%).
pub fn border_divider() -> Rgba {
    rgba(current().border_divider)
}

/// Borda de superfície rebaixada — um pouco mais marcada que a hairline sutil.
pub fn BORDER_SUNKEN() -> u32 {
    current().border_sunken
}

// ===================== White tints (ícones/realces translúcidos) =====================
//
// No dark são **brancos com alpha**; no claro viram **pretos com alpha** (ícones
// translúcidos legíveis sobre fundo claro). Mantêm o mesmo papel: "forte"/"ativo"/"inativo".

/// Tint "forte" — ícone dentro de caixa. Branco 75% no dark / preto 75% no claro.
pub fn white_75() -> Rgba {
    rgba(current().tint_strong)
}

/// Tint "ativo" — ícone de ação ativo (segmento ATIVO de um toggle).
pub fn white_70() -> Rgba {
    rgba(current().tint_active)
}

/// Tint "inativo" — ícone apagado (magnifier de busca, segmento INATIVO de um toggle).
pub fn white_30() -> Rgba {
    rgba(current().tint_muted)
}

/// Divisória entre grupos de um painel de propriedades.
pub fn BORDER_GROUP() -> u32 {
    current().border_group
}

// ===================== Text (texto) =====================

/// Texto de título — ≈15px, peso 600.
pub fn TEXT_TITLE() -> u32 {
    current().text_title
}

/// Texto de rótulo/label — ≈14px.
pub fn TEXT_LABEL() -> u32 {
    current().text_label
}

/// Texto de valor.
pub fn TEXT_VALUE() -> u32 {
    current().text_value
}

/// Texto/ícone apagado (muted).
pub fn TEXT_MUTED() -> u32 {
    current().text_muted
}

/// Ícone genérico — mesmo tom do texto muted.
pub fn ICON() -> u32 {
    current().icon
}

/// Ícone de scrub — um tom mais escuro que o ícone genérico.
pub fn ICON_SCRUB() -> u32 {
    current().icon_scrub
}

// ===================== Accent (destaque) =====================

/// Acento claro — usado em botões "light" do design.
pub fn ACCENT_LIGHT() -> u32 {
    current().accent_light
}

/// **Acento PRIMÁRIO da app** — a única cor de destaque do produto. No dark é
/// branco-quase-puro; no claro é preto-quase-puro (pra ler sobre fundo claro).
///
/// ⚠️ Qualquer elemento que use `PRIMARY` de **fundo** deve usar texto/ícone do par
/// [`ON_PRIMARY`] (sempre contrastante com o `primary` do tema).
pub fn PRIMARY() -> u32 {
    current().primary
}

/// Texto/ícone **sobre** um fundo [`PRIMARY`] — garante contraste nos botões/pílulas.
pub fn ON_PRIMARY() -> u32 {
    current().on_primary
}

/// Bolinha (knob) do [`crate::switch::Switch`] **ligado** — lê sobre o trilho [`PRIMARY`].
pub fn SWITCH_KNOB_ON() -> u32 {
    current().switch_knob_on
}

/// Trilho do `Switch` **desligado** (cinza escuro no dark; cinza bem claro no light).
pub fn SWITCH_TRACK_OFF() -> u32 {
    current().switch_track_off
}

/// Estado de **hover** dos elementos com fundo [`PRIMARY`].
pub fn PRIMARY_HOVER() -> u32 {
    current().primary_hover
}

// ===================== Overlays / chrome auxiliar (theme-aware) =====================

/// Fundo de card/popover flutuante (modais, pickers).
pub fn BG_OVERLAY() -> u32 {
    current().bg_overlay
}

/// Fundo de "input box"/pill discreta dentro de overlays.
pub fn BG_INSET() -> u32 {
    current().bg_inset
}

/// Hover de item de lista / inset.
pub fn BG_INSET_HOVER() -> u32 {
    current().bg_inset_hover
}

/// Borda de overlay/popover (mais marcada que a hairline).
pub fn BORDER_OVERLAY() -> u32 {
    current().border_overlay
}

/// Borda em estado de foco/aberto (chips, pickers).
pub fn BORDER_STRONG() -> u32 {
    current().border_strong
}

/// Texto forte sobre overlay (títulos de modal).
pub fn TEXT_STRONG() -> u32 {
    current().text_strong
}

/// Texto secundário/legenda dentro de overlays.
pub fn TEXT_DIM() -> u32 {
    current().text_dim
}

/// Texto bem apagado (placeholders, hints).
pub fn TEXT_FAINT() -> u32 {
    current().text_faint
}

/// Backdrop translúcido atrás de modais (cobre a janela inteira). Já tem alpha → `Rgba`.
pub fn scrim() -> Rgba {
    rgba(current().scrim)
}

// ===================== Semânticos de feedback =====================

/// Borda/anel de **foco** de um controle editável (azul, igual nos dois temas).
pub fn FOCUS_RING() -> u32 {
    current().focus_ring
}

/// **Erro / inválido** — borda do controle e texto da mensagem.
pub fn DANGER() -> u32 {
    current().danger
}

/// **Sucesso / válido**.
pub fn SUCCESS() -> u32 {
    current().success
}

/// **Aviso** — estado que não bloqueia (ex.: perto do limite de caracteres).
pub fn WARNING() -> u32 {
    current().warning
}

// ===================== Radii (raios) =====================

/// Raio de campo de input (`7px`).
pub const RADIUS_FIELD: f32 = 7.0;

/// Raio de botão (`4px`).
pub const RADIUS_BUTTON: f32 = 4.0;

// ===================== Font (fonte) =====================

/// Família de fonte primária da UI = **system sans-serif** (system-ui / San
/// Francisco no macOS). Este token existe pra centralizar a escolha.
pub const FONT_UI: &str = ".SystemUIFont";

/// Família **Inter** (`"Inter"`).
///
/// ⚠️ **NÃO está registrada** no text system, e este crate não a embute. O gpui 0.2.2
/// renderiza errado os glifos da Inter **variável** (`InterVariable.ttf`) — texto sai
/// garbled ("Master Track" → "Máš₁ęr Ṫṛáçl"). **Não use** `.font_family(theme::FONT_INTER)`
/// até que pesos **estáticos** da Inter (Regular/Medium/SemiBold) sejam embutidos e
/// registrados pela app; senão o `.font_family` cai no fallback do sistema (melhor caso)
/// ou quebra os glifos (pior caso). Mantido como const só pra documentar a intenção.
pub const FONT_INTER: &str = "Inter";

#[cfg(test)]
mod tests {
    use super::*;

    /// `set_theme` troca o modo, e os tokens passam a resolver na paleta nova. Cobre o
    /// contrato que todo widget do crate depende: ler token DEPOIS do `set_theme` do frame.
    #[test]
    fn set_theme_troca_a_paleta_resolvida() {
        set_theme(ThemeMode::Dark);
        assert_eq!(theme_mode(), ThemeMode::Dark);
        assert_eq!(BG_PANEL(), DARK.bg_panel);
        assert_eq!(PRIMARY(), DARK.primary);

        set_theme(ThemeMode::Light);
        assert_eq!(theme_mode(), ThemeMode::Light);
        assert_eq!(BG_PANEL(), LIGHT.bg_panel);
        assert_eq!(PRIMARY(), LIGHT.primary);

        set_theme(ThemeMode::Dark); // não deixa estado vazando pros outros testes da thread
    }

    /// Os dois temas precisam definir TODOS os campos com valores distintos onde o design
    /// pede inversão — em especial o par `primary`/`on_primary`, que garante contraste de
    /// botão preenchido. Se alguém copiar a paleta e esquecer de inverter, isto pega.
    #[test]
    fn dark_e_light_invertem_o_par_primary() {
        assert_ne!(DARK.primary, LIGHT.primary);
        assert_ne!(DARK.on_primary, LIGHT.on_primary);
        // `primary` do dark é claro; do light é escuro (soma dos canais como proxy de luminância).
        let soma = |c: u32| (c >> 16 & 0xff) + (c >> 8 & 0xff) + (c & 0xff);
        assert!(soma(DARK.primary) > soma(LIGHT.primary));
    }
}

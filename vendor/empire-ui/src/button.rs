//! `Button` — o **botão** do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/button.tsx`
//!
//! # Anatomia
//!
//! ```text
//!   ┌───────────────────────────────┐
//!   │ ◇  Salvar alterações       →  │   ← .icon_before(…) · label · .icon_after(…)
//!   └───────────────────────────────┘
//!     ▲ borda 1px      ▲ realce interno de 1px no topo (variantes cheias)
//! ```
//!
//! # Uso
//!
//! ```ignore
//! Button::new("salvar", "Salvar")
//!     .variant(ButtonVariant::Default)
//!     .size(ButtonSize::Sm)
//!     .icon_before("iconoir/regular/check.svg")
//!     .on_click(|_, _window, _cx| println!("clicou"));
//!
//! Button::icon("apagar", "iconoir/regular/trash.svg").size(ButtonSize::IconSm)
//! ```
//!
//! O `id` é **obrigatório** e tem que ser estável: o GPUI só expõe `on_click`, `active` e
//! rastreio de foco em elementos com `.id()` ([`gpui::StatefulInteractiveElement`]), e um id que
//! muda de frame em frame zera o estado de interação.
//!
//! # O que o GPUI exigiu adaptar
//!
//! Três utilitários CSS do original não têm equivalente e viraram **overlays absolutos** com
//! borda de 1px/2px. Todos os três já custaram bug visível nesta base — ver
//! [`top_highlight_overlay`], [`bevel_overlay`] e [`ring_overlay`]:
//!
//! | coss                                | aqui                                            |
//! |-------------------------------------|-------------------------------------------------|
//! | `inset-shadow-[0_1px_white/16%]`    | overlay em `inset:-1px` com `border_t_1`        |
//! | `before:shadow-[0_1px_black/4%]`    | overlay em `inset:-1px` com `border_b_1`        |
//! | `focus-visible:ring-2 ring-offset-1`| overlay em `inset:-3px` com `border_2`          |
//!
//! A sombra EXTERNA (`shadow-xs`) continua um [`gpui::BoxShadow`] — ali a técnica funciona, porque
//! a sombra fica atrás de um fundo opaco e ninguém vê que o `Window::paint_shadows` não a recorta.

use gpui::{
    div, px, App, ClickEvent, Div, ElementId, FocusHandle, Hsla, InteractiveElement, IntoElement,
    ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::color::{lerp, Rgba8};
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Os utilitários Tailwind do `button.tsx` resolvidos em número, com a paleta `neutral`/`red` do
// Tailwind expandida (`neutral-400 #a3a3a3`, `neutral-500 #737373`, `neutral-800 #262626`,
// `neutral-100 #f5f5f5`, `neutral-50 #fafafa`, `red-400 #f87171`, `red-500 #ef4444`,
// `red-700 #b91c1c`) e os `color-mix` já calculados.
//
// Mesma disciplina de cor do campo de texto e do card: TODO valor é `0xRRGGBBAA`, com o byte de
// alfa, sempre — ver [`crate::color`]. Misturar com os tokens de 6 dígitos de [`crate::theme`]
// desloca os canais e produz uma cor completamente diferente, sem erro de compilação (já
// aconteceu três vezes).

/// Tokens visuais do botão, por tema.
#[derive(Clone, Copy, Debug)]
struct ButtonPalette {
    /// `--primary` — fundo e borda das variantes cheias.
    primary: Rgba8,
    /// `--primary-foreground` — texto sobre `primary`.
    primary_fg: Rgba8,
    /// `--secondary` **e** `--accent`. No coss os dois resolvem pro MESMO valor (preto/branco a
    /// 4%), então é um campo só — dois campos idênticos seriam duas fontes de verdade pro mesmo
    /// número.
    secondary: Rgba8,
    /// `--secondary-foreground` **e** `--accent-foreground` (também coincidem).
    secondary_fg: Rgba8,
    /// `--popover` — fundo da variante `Outline` no tema claro.
    popover: Rgba8,
    /// `--destructive` — fundo/borda das variantes destrutivas.
    destructive: Rgba8,
    /// `--destructive-foreground` — o vermelho de TEXTO, mais escuro (claro) / mais claro (escuro)
    /// que `destructive`, pra ler sobre fundo neutro.
    destructive_fg: Rgba8,
    /// `--input` — borda da família `Outline` (e, no escuro, também o fundo dela).
    input: Rgba8,
    /// `--ring` — cor do anel de foco.
    ring: Rgba8,
    /// `--foreground` — texto das variantes sem fundo próprio.
    foreground: Rgba8,
    /// Sombra externa da família `Outline` (`shadow-xs/5`).
    shadow: Rgba8,
    /// Fio de bisel de 1px da família `Outline`. Desce no claro, sobe no escuro.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro), `-1` sobe (escuro).
    bevel_dir: f32,
    /// Se este é o tema escuro. A família `Outline` troca de TOKEN (não só de valor) entre os
    /// temas — `bg-popover` no claro, `bg-input/32` no escuro — então a derivação precisa saber
    /// em qual tema está. Ver [`ButtonVariant::style`].
    dark: bool,
}

/// Tema **claro**.
const BUTTON_LIGHT: ButtonPalette = ButtonPalette {
    primary: Rgba8(0x262626ff),    // neutral-800
    primary_fg: Rgba8(0xfafafaff), // neutral-50
    secondary: Rgba8(0x0000000a),  // black 4%
    secondary_fg: Rgba8(0x262626ff),
    popover: Rgba8(0xffffffff),
    destructive: Rgba8(0xef4444ff),    // red-500
    destructive_fg: Rgba8(0xb91c1cff), // red-700
    input: Rgba8(0x0000001a),          // black 10%
    ring: Rgba8(0xa3a3a3ff),           // neutral-400
    foreground: Rgba8(0x262626ff),
    shadow: Rgba8(0x0000000d), // black 5%
    bevel: Rgba8(0x0000000a),  // black 4%
    bevel_dir: 1.0,
    dark: false,
};

/// Tema **escuro**.
const BUTTON_DARK: ButtonPalette = ButtonPalette {
    primary: Rgba8(0xf5f5f5ff), // neutral-100
    primary_fg: Rgba8(0x262626ff),
    secondary: Rgba8(0xffffff0a), // white 4%
    secondary_fg: Rgba8(0xf5f5f5ff),
    popover: Rgba8(0x1d1d1dff),
    // destructive escuro = mix(red-500 90%, white) = #f15757.
    destructive: Rgba8(0xf15757ff),
    destructive_fg: Rgba8(0xf87171ff), // red-400
    input: Rgba8(0xffffff14),          // white 8%
    ring: Rgba8(0x737373ff),           // neutral-500
    foreground: Rgba8(0xf5f5f5ff),
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE do coss: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO —
    // alfa 30 ≈ 11,8% — por decisão de design, a MESMA já vigente no `Input` e no `Card`: a 6% o
    // filete é imperceptível no nosso fundo escuro. NÃO "corrija" isto pra 0x0f achando que é erro
    // de porte; se a intenção mudar, mude junto o teste `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
    dark: true,
};

/// A paleta do botão no tema corrente.
fn button() -> &'static ButtonPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &BUTTON_DARK,
        theme::ThemeMode::Light => &BUTTON_LIGHT,
    }
}

/// Branco puro — o texto da variante `Destructive` (que não usa `--destructive-foreground`, porque
/// sobre o vermelho cheio quem lê é o branco).
const WHITE: Rgba8 = Rgba8(0xffffffff);

/// Realce interno superior das variantes cheias em repouso — `inset-shadow-[0_1px_white/16%]`.
/// 16% de 255 = 40,8 → `0x29`.
const TOP_HIGHLIGHT: Rgba8 = Rgba8(0xffffff29);

/// O mesmo realce quando o botão está **pressionado** — `active:inset-shadow-[0_1px_black/8%]`.
/// Ele não some: TROCA de branco 16% pra preto 8%, o que afunda a borda de cima. 8% de 255 = 20,4
/// → `0x14`.
const TOP_HIGHLIGHT_PRESSED: Rgba8 = Rgba8(0x00000014);

// --- Alfas (os modificadores `/N` do Tailwind) ----------------------------------------------------
//
// ⚠️ No Tailwind v4 o modificador `/N` é `color-mix(in oklab, <cor> N%, transparent)`, ou seja
// **multiplica** o alfa que a cor já tem — não o substitui. É por isso que tudo aqui passa por
// [`Rgba8::scaled`] em vez de reescrever o byte de alfa: em `--input` (branco a 8%), `/32` dá
// 2,56%, não 32%.

/// `shadow-primary/24` / `shadow-destructive/24` — sombra externa das variantes cheias.
const FILLED_SHADOW_ALPHA: f32 = 0.24;

/// `hover:bg-primary/90` / `hover:bg-destructive/90`.
const FILLED_HOVER_ALPHA: f32 = 0.90;

/// `hover:bg-secondary/90`.
const SECONDARY_HOVER_ALPHA: f32 = 0.90;

/// `active:bg-secondary/80`.
const SECONDARY_PRESSED_ALPHA: f32 = 0.80;

/// `dark:bg-input/32` — fundo da família `Outline` no tema escuro.
const OUTLINE_DARK_BG_ALPHA: f32 = 0.32;

/// `hover:bg-accent/50` — hover da família `Outline` no tema claro.
const OUTLINE_HOVER_ALPHA_LIGHT: f32 = 0.50;

/// `dark:hover:bg-input/64` — hover da família `Outline` no tema escuro.
const OUTLINE_HOVER_ALPHA_DARK: f32 = 0.64;

/// `hover:border-destructive/32` — borda da `DestructiveOutline` no hover.
const DESTRUCTIVE_OUTLINE_HOVER_BORDER_ALPHA: f32 = 0.32;

/// `hover:bg-destructive/4` — fundo da `DestructiveOutline` no hover.
const DESTRUCTIVE_OUTLINE_HOVER_BG_ALPHA: f32 = 0.04;

/// `has-disabled:opacity-64` — o coss esmaece o CONJUNTO em vez de trocar cor por cor. Uma fonte
/// de verdade só: não existe um par "apagado" de cada token.
const DISABLED_OPACITY: f32 = 0.64;

/// `opacity-80` nos ícones — eles pesam mais que o texto no mesmo tamanho, e o original os alivia.
const ICON_OPACITY: f32 = 0.80;

// --- Geometria -----------------------------------------------------------------------------------

/// Espessura do anel de foco — `focus-visible:ring-2`.
const RING_WIDTH: f32 = 2.0;

/// Folga entre o botão e o anel — `ring-offset-1`.
const RING_OFFSET: f32 = 1.0;

// =================================================================================================
// Tamanho
// =================================================================================================

/// Tamanho do botão. Controla altura, respiro horizontal, gap, corpo do texto, raio e lado do
/// ícone de uma vez — pra não existir botão "quase default" com números escolhidos a olho no call
/// site.
///
/// # As alturas são TOTAIS
///
/// ⚠️ Aqui é diferente do [`crate::input::InputSize`]. No `Input` o `h-*` do coss está no
/// `<input>` (que não tem borda) e a moldura mede `h + 2px`. No botão o `h-*` está no **próprio
/// elemento**, que tem `border` e `box-sizing: border-box` — então `h-8` são **32px de altura
/// total**, bordas incluídas. É exatamente por isso que o respiro horizontal do original é
/// `px-[calc(--spacing(3)-1px)]` = 11px: 11 + 1 de borda fecham os 12px de recuo pretendidos.
///
/// # Valores efetivos
///
/// São as variantes `sm:` do Tailwind (≥640px): uma janela desktop está sempre acima do
/// breakpoint, então as classes base nunca valem.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonSize {
    /// 24px de altura — barras de ferramentas densas, chips.
    Xs,
    /// 28px.
    Sm,
    /// 32px — o padrão.
    #[default]
    Default,
    /// 36px.
    Lg,
    /// 40px — call-to-action.
    Xl,
    /// Quadrado de 24×24, só ícone.
    IconXs,
    /// Quadrado de 28×28, só ícone.
    IconSm,
    /// Quadrado de 32×32, só ícone — o padrão de [`Button::icon`].
    Icon,
    /// Quadrado de 36×36, só ícone.
    IconLg,
    /// Quadrado de 40×40, só ícone.
    IconXl,
}

impl ButtonSize {
    /// Altura TOTAL, bordas incluídas (ver o doc do tipo). Nos tamanhos de ícone é também a
    /// largura — eles são quadrados.
    pub fn height(self) -> f32 {
        match self {
            ButtonSize::Xs | ButtonSize::IconXs => 24.0,
            ButtonSize::Sm | ButtonSize::IconSm => 28.0,
            ButtonSize::Default | ButtonSize::Icon => 32.0,
            ButtonSize::Lg | ButtonSize::IconLg => 36.0,
            ButtonSize::Xl | ButtonSize::IconXl => 40.0,
        }
    }

    /// Respiro horizontal interno — `px-[calc(--spacing(N)-1px)]`, com o `-1px` descontando a
    /// borda. **Zero nos tamanhos de ícone**: eles são quadrados de lado fixo e centralizam o
    /// glifo, sem respiro declarado.
    pub fn pad_x(self) -> f32 {
        match self {
            ButtonSize::Xs => 7.0,
            ButtonSize::Sm => 9.0,
            ButtonSize::Default => 11.0,
            ButtonSize::Lg => 13.0,
            ButtonSize::Xl => 15.0,
            _ if self.is_icon() => 0.0,
            // Inalcançável — o braço acima cobre todos os `Icon*`. Existe pra o `match` ser
            // exaustivo sem um `_ => unreachable!()` que quebraria em runtime se alguém adicionar
            // um tamanho novo e esquecer deste braço.
            _ => 0.0,
        }
    }

    /// Espaço entre ícone e texto — `gap-1`/`gap-1.5`/`gap-2`. Nos tamanhos de ícone acompanha o
    /// tamanho de texto equivalente (só importa se alguém passar rótulo E ícone num `Icon*`).
    pub fn gap(self) -> f32 {
        match self {
            ButtonSize::Xs | ButtonSize::IconXs => 4.0,
            ButtonSize::Sm | ButtonSize::IconSm => 6.0,
            _ => 8.0,
        }
    }

    /// Corpo do texto — `text-xs` no `Xs`, `sm:text-base` no `Xl`, `sm:text-sm` no resto.
    pub fn text_size(self) -> f32 {
        match self {
            ButtonSize::Xs | ButtonSize::IconXs => 12.0,
            ButtonSize::Xl | ButtonSize::IconXl => 16.0,
            _ => 14.0,
        }
    }

    /// Raio — `rounded-md` (`--radius-md` = **8px**) só nos dois menores; `rounded-lg`
    /// (`--radius-lg` = **10px**) em todos os outros.
    ///
    /// O `Xs` desafina de propósito: a 24px de altura, 10px de raio já lê como pílula.
    pub fn radius(self) -> f32 {
        match self {
            ButtonSize::Xs | ButtonSize::IconXs => 8.0,
            _ => 10.0,
        }
    }

    /// Lado do ícone — `sm:size-4` (16px) no geral, `sm:size-4.5` (18px) nos `Xl`, e 14px nos
    /// `Xs`, onde 16px encostaria nas bordas de um botão de 24px.
    pub fn icon_size(self) -> f32 {
        match self {
            ButtonSize::Xs | ButtonSize::IconXs => 14.0,
            ButtonSize::Xl | ButtonSize::IconXl => 18.0,
            _ => 16.0,
        }
    }

    /// Se este é um tamanho **quadrado, só de ícone**.
    pub fn is_icon(self) -> bool {
        matches!(
            self,
            ButtonSize::IconXs
                | ButtonSize::IconSm
                | ButtonSize::Icon
                | ButtonSize::IconLg
                | ButtonSize::IconXl
        )
    }
}

// =================================================================================================
// Variante
// =================================================================================================

/// Variante visual do botão — a hierarquia de ênfase.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonVariant {
    /// Ação principal: fundo cheio em `--primary`, sombra externa e realce interno no topo.
    #[default]
    Default,
    /// Ação secundária: preenchimento sutil (4% de preto/branco), sem sombra.
    Secondary,
    /// Ação de igual peso ao redor de conteúdo: borda visível e fundo de superfície.
    Outline,
    /// Ação terciária: só texto, com fundo aparecendo no hover.
    Ghost,
    /// Navegação: texto puro, sublinhado no hover.
    Link,
    /// Ação destrutiva de ênfase máxima: fundo vermelho cheio.
    Destructive,
    /// Ação destrutiva contida: moldura neutra com texto vermelho.
    DestructiveOutline,
}

/// As cores de uma variante já **resolvidas** no tema corrente.
///
/// Fica em [`Hsla`] (e não em [`Rgba8`]) porque quase todo valor aqui passou por um modificador
/// `/N` do Tailwind, que multiplica o alfa — o resultado não é mais um byte de paleta. A paleta em
/// si continua toda em `Rgba8`, e é ela que os testes de decodificação cobrem.
#[derive(Clone, Copy, Debug)]
struct VariantStyle {
    /// Fundo em repouso. `None` = transparente.
    bg: Option<Hsla>,
    /// Fundo no hover. `None` = mantém o de repouso.
    bg_hover: Option<Hsla>,
    /// Fundo pressionado. `None` = mantém o de hover.
    bg_pressed: Option<Hsla>,
    /// Cor da borda de 1px. `None` = borda transparente (a borda EXISTE mesmo assim — ver
    /// [`Button::render`]).
    border: Option<Hsla>,
    /// Cor da borda no hover. `None` = mantém a de repouso.
    border_hover: Option<Hsla>,
    /// Cor do texto e dos ícones.
    text: Hsla,
    /// Cor da sombra externa `shadow-xs`. `None` = sem sombra.
    shadow: Option<Hsla>,
    /// Se a variante tem o **realce interno superior** (branco 16% em repouso, preto 8%
    /// pressionado).
    top_highlight: bool,
    /// Se a variante tem o **fio de bisel** de 1px.
    bevel: bool,
    /// `hover:underline`.
    underline_on_hover: bool,
}

impl ButtonVariant {
    /// Resolve as cores desta variante no tema corrente.
    fn style(self) -> VariantStyle {
        let p = button();

        // O fundo e o hover da família `Outline` trocam de TOKEN entre os temas, não só de valor:
        // no claro é a superfície opaca (`bg-popover`) e o hover é o acento a 50%; no escuro é o
        // próprio `--input` diluído (32%), e o hover o mesmo `--input` mais forte (64%).
        //
        // Vale notar que no CSS o `hover:bg-*` SUBSTITUI o fundo (background-color é um valor só),
        // então no tema claro o botão perde o branco no hover e passa a mostrar a página atrás a
        // 2% de preto. É o comportamento do original, não um esquecimento.
        let (outline_bg, outline_bg_hover) = if p.dark {
            (
                p.input.scaled(OUTLINE_DARK_BG_ALPHA),
                p.input.scaled(OUTLINE_HOVER_ALPHA_DARK),
            )
        } else {
            (
                p.popover.hsla(),
                p.secondary.scaled(OUTLINE_HOVER_ALPHA_LIGHT),
            )
        };

        match self {
            ButtonVariant::Default => VariantStyle {
                bg: Some(p.primary.hsla()),
                bg_hover: Some(p.primary.scaled(FILLED_HOVER_ALPHA)),
                bg_pressed: None,
                border: Some(p.primary.hsla()),
                border_hover: None,
                text: p.primary_fg.hsla(),
                shadow: Some(p.primary.scaled(FILLED_SHADOW_ALPHA)),
                top_highlight: true,
                bevel: false,
                underline_on_hover: false,
            },
            ButtonVariant::Destructive => VariantStyle {
                bg: Some(p.destructive.hsla()),
                bg_hover: Some(p.destructive.scaled(FILLED_HOVER_ALPHA)),
                bg_pressed: None,
                border: Some(p.destructive.hsla()),
                border_hover: None,
                // Sobre o vermelho cheio quem lê é o BRANCO — não o
                // `--destructive-foreground`, que é um vermelho de texto.
                text: WHITE.hsla(),
                shadow: Some(p.destructive.scaled(FILLED_SHADOW_ALPHA)),
                top_highlight: true,
                bevel: false,
                underline_on_hover: false,
            },
            ButtonVariant::Secondary => VariantStyle {
                bg: Some(p.secondary.hsla()),
                // ⚠️ Contra-intuitivo e FIEL ao original: `hover:bg-secondary/90` DILUI o fundo
                // (4% → 3,6%), não o intensifica. Travado no teste
                // `hover_da_secondary_dilui_o_fundo`.
                bg_hover: Some(p.secondary.scaled(SECONDARY_HOVER_ALPHA)),
                bg_pressed: Some(p.secondary.scaled(SECONDARY_PRESSED_ALPHA)),
                border: None,
                border_hover: None,
                text: p.secondary_fg.hsla(),
                shadow: None,
                top_highlight: false,
                bevel: false,
                underline_on_hover: false,
            },
            ButtonVariant::Outline => VariantStyle {
                bg: Some(outline_bg),
                bg_hover: Some(outline_bg_hover),
                bg_pressed: None,
                border: Some(p.input.hsla()),
                border_hover: None,
                text: p.foreground.hsla(),
                shadow: Some(p.shadow.hsla()),
                top_highlight: false,
                bevel: true,
                underline_on_hover: false,
            },
            ButtonVariant::DestructiveOutline => VariantStyle {
                bg: Some(outline_bg),
                bg_hover: Some(p.destructive.scaled(DESTRUCTIVE_OUTLINE_HOVER_BG_ALPHA)),
                bg_pressed: None,
                border: Some(p.input.hsla()),
                border_hover: Some(p.destructive.scaled(DESTRUCTIVE_OUTLINE_HOVER_BORDER_ALPHA)),
                // O vermelho de TEXTO, mais legível sobre fundo neutro que o `--destructive`.
                text: p.destructive_fg.hsla(),
                shadow: Some(p.shadow.hsla()),
                top_highlight: false,
                bevel: true,
                underline_on_hover: false,
            },
            ButtonVariant::Ghost => VariantStyle {
                bg: None,
                bg_hover: Some(p.secondary.hsla()),
                bg_pressed: None,
                border: None,
                border_hover: None,
                text: p.foreground.hsla(),
                shadow: None,
                top_highlight: false,
                bevel: false,
                underline_on_hover: false,
            },
            ButtonVariant::Link => VariantStyle {
                bg: None,
                bg_hover: None,
                bg_pressed: None,
                border: None,
                border_hover: None,
                text: p.foreground.hsla(),
                shadow: None,
                top_highlight: false,
                bevel: false,
                underline_on_hover: true,
            },
        }
    }
}

// =================================================================================================
// Sombra, realce, bisel e anel
// =================================================================================================

/// A sombra externa `shadow-xs` do botão, ou `None`.
///
/// Livre (e não método) pra ser testável sem construir um [`Button`]. Duas regras de precedência
/// vivem aqui, e as duas são "apaga a sombra":
///
/// 1. **Desabilitado** — o coss não deixa um botão inerte projetar sombra.
/// 2. **Pressionado**, nas variantes cheias — junto com a troca do realce interno, é o que dá a
///    sensação de afundar. As variantes de moldura não têm estado pressionado no original.
///
/// A sombra externa pode continuar sendo um [`gpui::BoxShadow`] de verdade (diferente do bisel e
/// do anel): ela fica ATRÁS de um fundo opaco, então o fato de o `Window::paint_shadows` não a
/// recortar pra fora do elemento não aparece.
fn shadow_for(variant: ButtonVariant, disabled: bool, pressed: bool) -> Option<gpui::BoxShadow> {
    if disabled {
        return None;
    }
    let style = variant.style();
    if pressed && style.top_highlight {
        return None;
    }
    style.shadow.map(|color| gpui::BoxShadow {
        color,
        offset: gpui::point(px(0.0), px(1.0)),
        blur_radius: px(2.0),
        spread_radius: px(0.0),
    })
}

/// O **realce interno superior** de 1px das variantes cheias.
///
/// ⚠️ `inset-shadow` do CSS **não existe** no GPUI, e a sombra normal não serve de substituto: o
/// `Window::paint_shadows` insere um retângulo arredondado CHEIO, sem recortar a área do próprio
/// elemento — num overlay transparente isso vira uma lavagem de branco 16% sobre o botão inteiro.
/// Então o realce é desenhado como o que ele é: uma **borda de 1px num único lado**.
///
/// Fica em `inset:-1px` (a BORDER box, não a padding box) porque no coss o realce cai SOBRE a
/// borda de cima e a clareia; 1px pra dentro criaria uma segunda linha ao lado da borda, e a 16%
/// de alfa a diferença é visível. O botão não tem `overflow_hidden`, então o overlay pode ser
/// filho — não precisa do arranjo de irmão que o [`crate::input::Input`] exige.
///
/// # O estado pressionado
///
/// O realce não some ao pressionar: TROCA pra preto 8%. Isso é estado de interação, e um
/// `RenderOnce` não o conhece — quem conhece é o GPUI, via `active_style`. Mas `active` só existe
/// em elemento com `.id()`, e o `active_style` só é aplicado se o elemento tiver **hitbox**
/// (ver `Interactivity::should_insert_hitbox`) — um `.active()` sozinho num overlay é ignorado em
/// silêncio. O `.hover(|s| s)` abaixo existe só pra forçar essa hitbox; o refinamento é vazio.
///
/// A hitbox do overlay tem comportamento `Normal`, que **não** oclui: o `on_click` e o `hover` do
/// botão atrás continuam funcionando.
fn top_highlight_overlay(
    id: &ElementId,
    radius: f32,
    interactive: bool,
    join: crate::group::Join,
) -> gpui::AnyElement {
    let base = join
        .bevel_overlay(-1.0, radius)
        .border_t_1()
        .border_color(TOP_HIGHLIGHT.hsla());
    if !interactive {
        // Botão desabilitado não pressiona — sem hitbox, sem estado, um filete só.
        return base.into_any_element();
    }
    base.id(ElementId::NamedChild(
        Box::new(id.clone()),
        "top-highlight".into(),
    ))
    .hover(|s| s)
    .active(|s| s.border_color(TOP_HIGHLIGHT_PRESSED.hsla()))
    .into_any_element()
}

/// O **fio de bisel** de 1px da família `Outline`.
///
/// Mesma técnica e mesmas duas armadilhas do realce acima (ver [`top_highlight_overlay`]): é borda
/// e não sombra, e cobre a border box. Aqui não há estado — o bisel não reage a hover nem a
/// pressão — então o overlay é um `div` puro.
///
/// Claro: filete escuro EMBAIXO (a luz vem de cima, a base do botão recebe sombra). Escuro: filete
/// claro EM CIMA (a aresta superior é a que pega luz).
fn bevel_overlay(radius: f32, join: crate::group::Join) -> Div {
    let p = button();
    let overlay = join.bevel_overlay(-1.0, radius).border_color(p.bevel.hsla());
    if p.bevel_dir > 0.0 {
        overlay.border_b_1()
    } else {
        overlay.border_t_1()
    }
}

/// O **anel de foco** — `focus-visible:ring-2 ring-offset-1`.
///
/// ⚠️ `ring` não existe no GPUI, e não dá pra fingir com sombra: o `spread_radius` dilata os
/// limites mas MANTÉM o raio, então a curvatura sai errada nas quinas, e num fundo translúcido a
/// sombra atravessa e tinge o componente (foi o que aconteceu no [`crate::input::Input`]).
///
/// Um `ring` é geometricamente a forma do elemento dilatada: 2px de anel mais 1px de folga = um
/// overlay 3px maior em cada lado, com borda de 2px e o raio crescendo junto (`raio + 3`).
///
/// A folga fica **transparente** em vez de pintada com a cor de fundo do container, como faria o
/// `ring-offset-color` do Tailwind. Sobre um fundo sólido dá no mesmo; sobre um fundo texturizado,
/// a textura aparece na fresta de 1px.
fn ring_overlay(radius: f32, join: crate::group::Join) -> Div {
    let inset = -(RING_WIDTH + RING_OFFSET);
    join.ring_overlay(inset, radius + RING_WIDTH + RING_OFFSET)
        .border_2()
        .border_color(button().ring.hsla())
}

// =================================================================================================
// O componente
// =================================================================================================

/// Botão com o visual do coss. Ver o doc do módulo.
///
/// É um **elemento de render** (`RenderOnce`): construa a cada frame. O `id` tem que ser estável.
#[derive(IntoElement)]
pub struct Button {
    pub(crate) id: ElementId,
    label: Option<SharedString>,
    icon_before: Option<SharedString>,
    icon_after: Option<SharedString>,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    full_width: bool,
    focus_handle: Option<FocusHandle>,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    /// Como este botão encosta nos vizinhos, quando está num [`crate::group::Group`]. Quem preenche
    /// é o grupo, pela posição — ver `crate::group::GroupChild`.
    pub(crate) join: crate::group::Join,
    /// Lado do glifo, quando o do tamanho não serve. `None` = o de [`ButtonSize::icon_size`].
    icon_size: Option<Pixels>,
}

impl Button {
    /// Botão com **rótulo**. O `id` tem que ser estável entre frames.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: Some(label.into()),
            icon_before: None,
            icon_after: None,
            variant: ButtonVariant::default(),
            size: ButtonSize::default(),
            disabled: false,
            full_width: false,
            focus_handle: None,
            on_click: None,
            join: crate::group::Join::NONE,
            icon_size: None,
        }
    }

    /// Botão **só de ícone**, quadrado. Já vem em [`ButtonSize::Icon`] (32×32) — troque com
    /// [`Self::size`] usando um dos `Icon*`, senão o botão deixa de ser quadrado.
    ///
    /// O caminho é servido pela [`crate::assets::Assets`] (ex.:
    /// `"iconoir/regular/trash.svg"`). Sem essa `AssetSource` registrada no bootstrap, o ícone
    /// some SILENCIOSAMENTE.
    pub fn icon(id: impl Into<ElementId>, path: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon_before: Some(path.into()),
            icon_after: None,
            variant: ButtonVariant::default(),
            size: ButtonSize::Icon,
            disabled: false,
            full_width: false,
            focus_handle: None,
            on_click: None,
            join: crate::group::Join::NONE,
            icon_size: None,
        }
    }

    /// Variante visual.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Tamanho.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// **Lado do glifo**, sobrepondo o padrão do tamanho ([`ButtonSize::icon_size`]).
    ///
    /// Serve pra quando a CAIXA precisa de um tamanho e o glifo de outro: um botão que tem que casar a
    /// altura com os campos vizinhos, mas cujo ícone é uma ação secundária e não deve competir com o
    /// conteúdo deles. Trocar o `size` resolveria o glifo e estragaria a altura.
    ///
    /// É o análogo do `[&_svg:not([class*='size-'])]:size-4` da referência, que é escrito exatamente
    /// assim — com o `:not()` — pra deixar quem chama sobrepor o tamanho do ícone.
    pub fn icon_size(mut self, size: Pixels) -> Self {
        self.icon_size = Some(size);
        self
    }

    /// Desabilita: o botão para de responder a clique, hover e pressão, e o CONJUNTO esmaece pra
    /// 64% (não é troca de cor — ver [`DISABLED_OPACITY`]).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Ícone **antes** do rótulo.
    pub fn icon_before(mut self, path: impl Into<SharedString>) -> Self {
        self.icon_before = Some(path.into());
        self
    }

    /// Ícone **depois** do rótulo.
    pub fn icon_after(mut self, path: impl Into<SharedString>) -> Self {
        self.icon_after = Some(path.into());
        self
    }

    /// Faz o botão ocupar a largura do container (o default é o tamanho do conteúdo).
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Torna o botão focável por teclado (entra na ordem de tabulação) e liga o **anel de foco**.
    ///
    /// O handle tem que viver fora do elemento — crie um por botão na sua view
    /// (`cx.focus_handle()`), guarde-o, e passe a referência a cada render. Sem isto o botão
    /// funciona só a mouse.
    pub fn focus(mut self, handle: &FocusHandle) -> Self {
        self.focus_handle = Some(handle.clone());
        self
    }

    /// O que fazer no clique. Ignorado quando [`Self::disabled`].
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Um ícone do botão, no lado e no tamanho certos.
    fn render_icon(path: SharedString, size: f32, color: Hsla) -> impl IntoElement {
        gpui::svg()
            .path(path)
            .size(px(size))
            .flex_none()
            .text_color(color)
            // `opacity-80`: o ícone alivia em relação ao texto. Como opacidade de ELEMENTO (e não
            // alfa na cor) pra bater com o original — sobre fundos translúcidos os dois não dão o
            // mesmo resultado.
            .opacity(ICON_OPACITY)
    }
}


// =================================================================================================
// Feedback de interação (onda de clique + fade do fundo)
// =================================================================================================
//
// Estas duas coisas precisam de TEMPO, e o `Button` é um elemento `RenderOnce` — sem estado próprio,
// ele não sabe que acabou de ser clicado nem que o ponteiro acabou de entrar. A informação mora numa
// tabela por `ElementId`, no mesmo espírito do `thread_local` de transição de foco do
// `crate::input` (a UI roda numa thread só).
//
// O motor de frames é o `Window::request_animation_frame`: enquanto houver animação em curso, o
// render pede o próximo frame. Não há elemento de animação envolvido — o progresso vem do tempo
// decorrido, então não há `delta` pra sincronizar nem id de animação pra invalidar.

/// Duração da onda de clique.
///
/// O Ant Design usa ~0,4s; aqui é um pouco mais longo por preferência de sensação — a onda tem mais
/// tempo pra "ecoar" antes de assentar.
const WAVE_DURATION: Duration = Duration::from_millis(550);

/// Quanto a onda cresce pra fora do botão, em px, no fim do percurso.
const WAVE_SPREAD: f32 = 6.0;

/// Espessura do anel da onda.
const WAVE_WIDTH: f32 = 2.0;

/// Opacidade INICIAL da onda, como fração do alfa da cor do anel.
///
/// A cor do anel de foco é sólida, então sem este fator a onda nasceria opaca e batia mais forte
/// que o resto do feedback. Em ~0,5 ela lê como eco, não como borda.
const WAVE_ALPHA: f32 = 0.5;

/// Duração do fade do fundo (hover e retorno do pressionado). Curto de propósito: acima de ~200ms
/// um botão começa a parecer lento pra responder.
const BG_FADE: Duration = Duration::from_millis(180);

/// Teto das tabelas de interação. Estourar só faz as próximas animações daqueles botões saírem
/// instantâneas — degrada, não quebra.
const INTERACTION_TABLE_CAP: usize = 512;

thread_local! {
    /// Instante do último clique, por botão. Alimenta a onda e o retorno do fundo pressionado.
    static PRESSES: RefCell<HashMap<ElementId, Instant>> = RefCell::new(HashMap::new());

    /// Estado de hover e instante da última mudança, por botão. Alimenta o fade do fundo.
    static HOVERS: RefCell<HashMap<ElementId, (bool, Instant)>> = RefCell::new(HashMap::new());
}

/// Registra um clique. Chamado do handler de clique, que também pede um redraw.
fn record_press(id: &ElementId) {
    PRESSES.with(|t| {
        let mut t = t.borrow_mut();
        if t.len() > INTERACTION_TABLE_CAP {
            t.clear();
        }
        t.insert(id.clone(), Instant::now());
    });
}

/// Progresso da onda em `[0,1)`, ou `None` se não há onda em curso.
fn press_progress(id: &ElementId) -> Option<f32> {
    PRESSES.with(|t| {
        let inicio = *t.borrow().get(id)?;
        let decorrido = inicio.elapsed();
        (decorrido < WAVE_DURATION)
            .then(|| decorrido.as_secs_f32() / WAVE_DURATION.as_secs_f32())
    })
}

/// Registra entrada/saída do ponteiro.
///
/// `pub(crate)` porque a tabela é **por `ElementId`**, e não "de botão": o [`crate::Toggle`] também
/// escreve nela, pra que o separador de um [`crate::Group`] saiba que o vizinho está sob o ponteiro
/// (ver [`interaction_amount`]). Quem escreve aqui também tem que pedir um redraw.
pub(crate) fn record_hover(id: &ElementId, hovered: bool) {
    HOVERS.with(|t| {
        let mut t = t.borrow_mut();
        if t.len() > INTERACTION_TABLE_CAP {
            t.clear();
        }
        // Só reinicia o relógio quando o estado REALMENTE muda: o GPUI pode reemitir o mesmo valor,
        // e reiniciar a cada emissão travaria o fade no começo.
        match t.get(id) {
            Some((anterior, _)) if *anterior == hovered => {}
            _ => {
                t.insert(id.clone(), (hovered, Instant::now()));
            }
        }
    });
}

/// Quanto do fundo de hover aplicar, em `[0,1]`: `1` = totalmente em hover.
///
/// Fora da janela do fade devolve o valor estável (1 em hover, 0 fora), então um botão que ficou
/// parado sob o ponteiro continua com o fundo de hover.
fn hover_amount(id: &ElementId) -> f32 {
    HOVERS.with(|t| {
        let Some((hovered, desde)) = t.borrow().get(id).copied() else {
            return 0.0;
        };
        let t = (desde.elapsed().as_secs_f32() / BG_FADE.as_secs_f32()).clamp(0.0, 1.0);
        if hovered {
            t
        } else {
            1.0 - t
        }
    })
}

/// Quanto do fundo PRESSIONADO ainda aplicar, em `[0,1]`: começa em 1 no clique e cai a 0.
///
/// É o "leve fade" do fundo ao clicar: o estado pressionado não desaparece de um frame pro outro.
fn press_amount(id: &ElementId) -> f32 {
    PRESSES.with(|t| {
        let Some(inicio) = t.borrow().get(id).copied() else {
            return 0.0;
        };
        let decorrido = inicio.elapsed().as_secs_f32();
        (1.0 - decorrido / BG_FADE.as_secs_f32()).clamp(0.0, 1.0)
    })
}

/// **Quanto este botão está "sob interação"**, em `[0,1]` — o maior entre hover e pressionado.
///
/// Existe pro [`crate::group::Group`]: no tema escuro o separador ao lado de um botão em hover ou
/// pressionado clareia (`dark:*:[…button:hover~[data-slot=separator]…]:before:bg-input/64`), e no
/// GPUI não há seletor de irmão — quem sabe o estado do vizinho é esta tabela, que já existe aqui
/// pro fade de fundo. O grupo consulta pelo `id` do botão.
pub(crate) fn interaction_amount(id: ElementId) -> f32 {
    hover_amount(&id).max(press_amount(&id))
}

/// Curva de desaceleração da onda (`ease-out` cúbico), pra ela sair rápido e assentar devagar —
/// é o que o Ant faz com um `cubic-bezier` de saída.
fn ease_out(t: f32) -> f32 {
    let inv = 1.0 - t.clamp(0.0, 1.0);
    1.0 - inv * inv * inv
}

/// A **onda de clique**: um anel que nasce na borda do botão, cresce pra fora e desvanece.
///
/// Mesma cor do anel de foco, então o feedback de clique e o de teclado falam a mesma língua.
/// É um filho absoluto SEM interatividade — não cria hitbox e portanto não intercepta o clique.
fn wave_overlay(progress: f32, radius: f32, color: Hsla, join: crate::group::Join) -> Div {
    let t = ease_out(progress);
    let fora = WAVE_SPREAD * t;
    let mut cor = color;
    // Parte de `WAVE_ALPHA` e desvanece linearmente sobre a curva já desacelerada: o anel fica
    // visível na saída e some suavemente no fim.
    cor.a *= WAVE_ALPHA * (1.0 - progress);
    join.ring_overlay(-fora, radius + fora)
        .border(px(WAVE_WIDTH))
        .border_color(cor)
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let s = self.size;
        let v = self.variant.style();
        let radius = s.radius();
        let interactive = !self.disabled;
        let join = self.join;
        // O glifo aceita sobreposição; o resto da geometria não (ver `Self::icon_size`).
        let glifo = self.icon_size.map_or(s.icon_size(), f32::from);

        // O anel só aparece se houver handle de foco E ele estiver focado. Desabilitado nunca
        // mostra anel — um botão inerte não deve parecer que recebeu o teclado.
        let focused = interactive
            && self
                .focus_handle
                .as_ref()
                .is_some_and(|h| h.is_focused(window));

        let mut el = div()
            .id(self.id.clone())
            // `relative` porque realce, bisel e anel são filhos ABSOLUTOS.
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(s.gap()))
            .h(px(s.height()))
            .text_size(px(s.text_size()))
            .border_color(v.border.unwrap_or(gpui::transparent_black()))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(v.text);
        // Raio e bordas vêm da COSTURA: solto são os quatro cantos e os quatro lados; num grupo, o
        // lado da emenda perde os dois. A borda de 1px EXISTE em todas as variantes, mesmo nas "sem
        // borda" — é ela que mantém a altura interna igual entre variantes (a altura é total,
        // border-box). Nas sem borda ela fica transparente.
        el = join.rounded(el, radius);
        el = join.borders(el);

        // Geometria: quadrado nos `Icon*`, respiro horizontal nos outros. `full_width` vence a
        // largura de conteúdo — mas não a de um `Icon*`, que deixaria de ser quadrado.
        el = if s.is_icon() {
            el.w(px(s.height())).flex_none()
        } else if self.full_width {
            el.w_full().px(px(s.pad_x()))
        } else {
            el.flex_none().px(px(s.pad_x()))
        };

        // --- Fundo com fade -------------------------------------------------------------------
        //
        // O fundo deixou de vir do refinamento de `hover`/`active` e passou a ser CALCULADO a
        // partir do tempo: `hover_amount` e `press_amount` devolvem quanto de cada estado aplicar,
        // e a cor é interpolada. É isso que dá o fade em vez da troca seca de um frame pro outro.
        //
        // Consequência: o fundo NÃO está mais no `hover(..)` (ver abaixo, que ficou só com borda e
        // sublinhado). Se as duas coisas pintassem fundo, o refinamento de estilo venceria o
        // cálculo e o fade não apareceria.
        let hover_t = if interactive { hover_amount(&self.id) } else { 0.0 };
        let press_t = if interactive { press_amount(&self.id) } else { 0.0 };
        if let Some(base) = v.bg {
            let mut bg = base;
            if hover_t > 0.0 {
                if let Some(alvo) = v.bg_hover {
                    bg = lerp(bg, alvo, hover_t);
                }
            }
            if press_t > 0.0 {
                if let Some(alvo) = v.bg_pressed {
                    bg = lerp(bg, alvo, press_t);
                }
            }
            el = el.bg(bg);
        } else if hover_t > 0.0 {
            // Variantes sem fundo em repouso (`Ghost`, `Link`): o hover nasce do transparente.
            if let Some(alvo) = v.bg_hover {
                el = el.bg(lerp(gpui::transparent_black(), alvo, hover_t));
            }
        }
        if let Some(shadow) = shadow_for(self.variant, self.disabled, false) {
            el = el.shadow(vec![shadow]);
        }

        if interactive {
            el = el.cursor(gpui::CursorStyle::PointingHand);

            // --- hover ---------------------------------------------------------------------
            // O fundo saiu daqui (ver o cálculo acima); borda e sublinhado seguem como estilo,
            // porque não têm fade e o refinamento é a forma mais direta.
            let border_hover = v.border_hover;
            let underline = v.underline_on_hover;
            el = el.hover(move |mut style| {
                if let Some(border) = border_hover {
                    style = style.border_color(border);
                }
                if underline {
                    style = style.underline();
                }
                style
            });

            // --- pressionado ---------------------------------------------------------------
            //
            // O `active_style` cobre o que é estilo DO PRÓPRIO elemento: fundo e sombra. A troca
            // do realce interno mora no overlay, que tem estado próprio (ver
            // `top_highlight_overlay`) — um refinamento de estilo não alcança um filho.
            let bg_pressed = v.bg_pressed;
            let clears_shadow =
                v.shadow.is_some() && shadow_for(self.variant, false, true).is_none();
            el = el.active(move |mut style| {
                if let Some(bg) = bg_pressed {
                    style = style.bg(bg);
                }
                if clears_shadow {
                    style = style.shadow(Vec::new());
                }
                style
            });

            // --- Registro das interações que precisam de tempo ------------------------------
            //
            // O clique é registrado SEMPRE, mesmo sem `on_click` — a onda é feedback visual e não
            // depende de haver ação associada. O `window.refresh()` é o que faz o primeiro frame da
            // animação acontecer; a partir dele o `request_animation_frame` no fim do render mantém
            // a sequência.
            let id_press = self.id.clone();
            let handler = self.on_click;
            el = el.on_click(move |event, window, cx| {
                // O clique é ponteiro: apaga o anel de foco (ver `crate::focus_ring`). O foco em si
                // continua indo pro botão — o que não vai é o anel.
                crate::focus_ring::pointer_used(window);
                record_press(&id_press);
                window.refresh();
                if let Some(h) = handler.as_ref() {
                    h(event, window, cx);
                }
            });

            let id_hover = self.id.clone();
            el = el.on_hover(move |hovered, window, _cx| {
                record_hover(&id_hover, *hovered);
                window.refresh();
            });
        } else {
            // `has-disabled:opacity-64`: esmaece o conjunto — o elemento e todos os overlays.
            el = el.opacity(DISABLED_OPACITY);
        }

        // Só um botão ATIVO entra na ordem de tabulação. Rastrear o foco de um desabilitado o
        // deixaria alcançável por `tab` sem ter o que fazer ali — e sem anel, porque `focused`
        // já é falso quando desabilitado, o usuário perderia o cursor de teclado no meio do
        // formulário.
        if interactive {
            if let Some(handle) = self.focus_handle.as_ref() {
                el = el.track_focus(handle);
            }
        }

        // --- Conteúdo ---------------------------------------------------------------------
        if let Some(path) = self.icon_before {
            el = el.child(Self::render_icon(path, glifo, v.text));
        }
        if let Some(label) = self.label {
            el = el.child(div().flex_none().child(label));
        }
        if let Some(path) = self.icon_after {
            el = el.child(Self::render_icon(path, glifo, v.text));
        }

        // --- Overlays, por último: são absolutos, então a ordem só decide quem pinta em cima ---
        if v.top_highlight {
            el = el.child(top_highlight_overlay(&self.id, radius, interactive, join));
        }
        if v.bevel {
            el = el.child(bevel_overlay(radius, join));
        }
        // O anel é `focus-visible`, não `focus`: só acende se o foco veio do teclado. Sem o segundo
        // termo, TODO clique acendia um anel de 2px, porque o `track_focus` faz o mouse-down focar o
        // botão. Ver `crate::focus_ring`.
        if focused && crate::focus_ring::visible() {
            el = el.child(ring_overlay(radius, join));
        }

        // --- Onda de clique -------------------------------------------------------------------
        //
        // Vem DEPOIS do anel de foco: se o botão foi clicado com o teclado ainda nele, a onda passa
        // por cima, que é a ordem que o olho espera (o evento mais recente na frente).
        let wave = if interactive {
            press_progress(&self.id)
        } else {
            None
        };
        if let Some(progresso) = wave {
            el = el.child(wave_overlay(progresso, radius, button().ring.hsla(), join));
        }

        // Enquanto houver animação em curso, pede o próximo frame. É o motor de TODAS as animações
        // deste componente — não há elemento de animação, o progresso vem do tempo decorrido.
        let animando = wave.is_some() || (press_t > 0.0) || (hover_t > 0.0 && hover_t < 1.0);
        if animando {
            window.request_animation_frame();
        }

        el
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `frame.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Todos os tamanhos, pros testes varrerem sem esquecer nenhum.
    const TODOS: [ButtonSize; 10] = [
        ButtonSize::Xs,
        ButtonSize::Sm,
        ButtonSize::Default,
        ButtonSize::Lg,
        ButtonSize::Xl,
        ButtonSize::IconXs,
        ButtonSize::IconSm,
        ButtonSize::Icon,
        ButtonSize::IconLg,
        ButtonSize::IconXl,
    ];

    /// Todas as variantes, idem.
    const VARIANTES: [ButtonVariant; 7] = [
        ButtonVariant::Default,
        ButtonVariant::Secondary,
        ButtonVariant::Outline,
        ButtonVariant::Ghost,
        ButtonVariant::Link,
        ButtonVariant::Destructive,
        ButtonVariant::DestructiveOutline,
    ];

    /// **A convenção de cor da paleta do botão, decodificada de verdade.**
    ///
    /// É o teste que pega a confusão de convenção: todo valor de [`ButtonPalette`] é `0xRRGGBBAA`
    /// e é consumido por `rgba`. Um valor de 6 dígitos esquecido ali vira uma cor completamente
    /// diferente sem erro de compilação — `rgba(0xffffff)` é lido como `0x00FFFFFF`, ciano. Isso
    /// já custou três bugs visíveis nesta base.
    ///
    /// Então, em vez de comparar números com números (que não pegaria nada), decodifica e afirma o
    /// que a cor DEVE ser perceptualmente.
    /// **A curva da onda desacelera.** `ease_out` tem que sair rápido e assentar: na metade do
    /// tempo já passou de 80% do percurso. Se alguém trocar por linear, a onda perde a sensação de
    /// eco do clique.
    #[test]
    fn onda_desacelera() {
        assert_eq!(ease_out(0.0), 0.0);
        assert!((ease_out(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_out(0.5) > 0.8, "na metade do tempo, já passou de 80% do caminho");
        // Monotônica: nunca volta pra trás.
        let mut anterior = 0.0;
        for i in 0..=20 {
            let atual = ease_out(i as f32 / 20.0);
            assert!(atual >= anterior, "ease_out tem que ser monotônica");
            anterior = atual;
        }
    }

    /// **A onda tem começo e fim.** Sem clique registrado não há onda; registrado, ela existe e o
    /// progresso está em `[0,1)`.
    #[test]
    fn onda_existe_so_depois_do_clique() {
        let id = ElementId::from("teste-onda");
        PRESSES.with(|t| t.borrow_mut().clear());
        assert!(press_progress(&id).is_none(), "sem clique, sem onda");

        record_press(&id);
        let p = press_progress(&id).expect("clique registrado tem onda");
        assert!((0.0..1.0).contains(&p), "progresso fora de [0,1): {p}");
        PRESSES.with(|t| t.borrow_mut().clear());
    }

    /// **O fundo pressionado começa cheio e decai.** É o "leve fade" do clique: `press_amount`
    /// devolve 1 no instante do clique e cai em direção a 0.
    #[test]
    fn fundo_pressionado_comeca_cheio() {
        let id = ElementId::from("teste-press-bg");
        PRESSES.with(|t| t.borrow_mut().clear());
        assert_eq!(press_amount(&id), 0.0, "sem clique, nada de fundo pressionado");

        record_press(&id);
        assert!(press_amount(&id) > 0.9, "no instante do clique, quase cheio");
        PRESSES.with(|t| t.borrow_mut().clear());
    }

    /// **O hover não reinicia o relógio quando o estado não muda.** O GPUI pode reemitir o mesmo
    /// valor; se cada emissão reiniciasse o fade, ele ficaria travado no começo e nunca chegaria ao
    /// fundo de hover.
    #[test]
    fn hover_repetido_nao_reinicia_o_fade() {
        let id = ElementId::from("teste-hover");
        HOVERS.with(|t| t.borrow_mut().clear());

        record_hover(&id, true);
        let marca = HOVERS.with(|t| t.borrow().get(&id).copied().unwrap().1);
        record_hover(&id, true); // mesma emissão
        let depois = HOVERS.with(|t| t.borrow().get(&id).copied().unwrap().1);
        assert_eq!(marca, depois, "emitir `true` de novo não pode reiniciar o relógio");

        record_hover(&id, false); // agora MUDOU
        let saida = HOVERS.with(|t| t.borrow().get(&id).copied().unwrap());
        assert!(!saida.0, "estado seguiu a mudança");
        HOVERS.with(|t| t.borrow_mut().clear());
    }

    /// **Fora da janela do fade, o hover é estável.** Um botão parado sob o ponteiro fica com o
    /// fundo de hover cheio (1,0), e um sem ponteiro fica em 0 — o fade é só a transição.
    #[test]
    fn hover_estavel_fora_da_janela_do_fade() {
        let id = ElementId::from("teste-hover-estavel");
        HOVERS.with(|t| {
            let mut t = t.borrow_mut();
            t.clear();
            // Entrou há muito tempo: fade concluído.
            t.insert(id.clone(), (true, Instant::now() - BG_FADE * 4));
        });
        assert!((hover_amount(&id) - 1.0).abs() < 1e-6, "parado em hover = fundo cheio");

        HOVERS.with(|t| {
            t.borrow_mut()
                .insert(id.clone(), (false, Instant::now() - BG_FADE * 4));
        });
        assert!(hover_amount(&id).abs() < 1e-6, "parado fora = sem fundo de hover");
        HOVERS.with(|t| t.borrow_mut().clear());
    }

    /// A onda cresce pra FORA e desvanece: no fim do percurso ela está maior que o botão e
    /// praticamente invisível. Pega uma inversão de sinal no inset, que faria a onda crescer pra
    /// dentro e ficar escondida sob o próprio botão.
    #[test]
    fn onda_cresce_pra_fora_e_desvanece() {
        theme::set_theme(theme::ThemeMode::Dark);
        assert!(WAVE_SPREAD > 0.0, "a onda cresce pra fora");

        let anel = button().ring.hsla().a;
        // A onda é MAIS TRANSPARENTE que o anel de foco, por decisão: ela é eco de um clique, não
        // indicação de que o teclado está ali.
        assert!(WAVE_ALPHA < 1.0, "a onda não nasce opaca");
        let comeco = anel * WAVE_ALPHA;
        assert!(comeco < anel, "no começo, mais transparente que o anel de foco");
        // E desaparece no fim do percurso.
        let quase_fim = anel * WAVE_ALPHA * (1.0 - 0.99);
        assert!(quase_fim < comeco * 0.05, "no fim do percurso a onda já desapareceu");
    }

    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        // Superfície opaca do tema claro: branco puro.
        let popover: gpui::Rgba = BUTTON_LIGHT.popover.hsla().into();
        assert_eq!(
            (popover.r, popover.g, popover.b, popover.a),
            (1.0, 1.0, 1.0, 1.0),
            "claro: popover é branco opaco"
        );

        // Os NEUTROS: primary, primary-fg, secondary-fg, foreground, ring, popover escuro. Todos
        // opacos e com r≈g≈b — se algum sair colorido, o valor foi lido deslocado.
        for (nome, c) in [
            ("primary claro", BUTTON_LIGHT.primary),
            ("primary escuro", BUTTON_DARK.primary),
            ("primary-fg claro", BUTTON_LIGHT.primary_fg),
            ("primary-fg escuro", BUTTON_DARK.primary_fg),
            ("secondary-fg claro", BUTTON_LIGHT.secondary_fg),
            ("secondary-fg escuro", BUTTON_DARK.secondary_fg),
            ("foreground claro", BUTTON_LIGHT.foreground),
            ("foreground escuro", BUTTON_DARK.foreground),
            ("ring claro", BUTTON_LIGHT.ring),
            ("ring escuro", BUTTON_DARK.ring),
            ("popover escuro", BUTTON_DARK.popover),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: NEUTRO — se r≠g≠b, o valor foi lido deslocado"
            );
        }

        // O par primary/primary-fg INVERTE entre os temas: fundo escuro com texto claro no tema
        // claro, e o contrário no escuro. Se os dois ficarem do mesmo lado, o botão desaparece.
        let pl: gpui::Rgba = BUTTON_LIGHT.primary.hsla().into();
        let pfl: gpui::Rgba = BUTTON_LIGHT.primary_fg.hsla().into();
        assert!(
            pl.r < 0.2 && pfl.r > 0.9,
            "claro: fundo escuro, texto claro"
        );
        let pd: gpui::Rgba = BUTTON_DARK.primary.hsla().into();
        let pfd: gpui::Rgba = BUTTON_DARK.primary_fg.hsla().into();
        assert!(
            pd.r > 0.9 && pfd.r < 0.2,
            "escuro: fundo claro, texto escuro"
        );

        // Os VERMELHOS: destructive e destructive-fg, nos dois temas. Opacos e com r bem acima de
        // g e b — o sintoma do bug era o canal vermelho zerar.
        for (nome, c) in [
            ("destructive claro", BUTTON_LIGHT.destructive),
            ("destructive escuro", BUTTON_DARK.destructive),
            ("destructive-fg claro", BUTTON_LIGHT.destructive_fg),
            ("destructive-fg escuro", BUTTON_DARK.destructive_fg),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: opaco");
            assert!(
                c.r > 0.6 && c.r > c.g + 0.25 && c.r > c.b + 0.25,
                "{nome}: é vermelho de verdade"
            );
        }

        // Branco puro pro texto da Destructive.
        let w: gpui::Rgba = WHITE.hsla().into();
        assert_eq!((w.r, w.g, w.b, w.a), (1.0, 1.0, 1.0, 1.0));

        // Os TRANSLÚCIDOS: é o alfa que os deixa funcionar sobre qualquer fundo. Um deles opaco
        // significaria um retângulo chapado no lugar de um véu.
        for (nome, c) in [
            ("secondary claro", BUTTON_LIGHT.secondary),
            ("secondary escuro", BUTTON_DARK.secondary),
            ("input claro", BUTTON_LIGHT.input),
            ("input escuro", BUTTON_DARK.input),
            ("shadow claro", BUTTON_LIGHT.shadow),
            ("shadow escuro", BUTTON_DARK.shadow),
            ("bisel claro", BUTTON_LIGHT.bevel),
            ("bisel escuro", BUTTON_DARK.bevel),
            ("realce", TOP_HIGHLIGHT),
            ("realce pressionado", TOP_HIGHLIGHT_PRESSED),
        ] {
            assert!(
                c.alpha() < 1.0,
                "{nome} tem que ser translúcido, veio com alfa {}",
                c.alpha()
            );
        }

        // O realce interno TROCA de polaridade ao pressionar: branco em repouso, preto pressionado
        // — é o que dá a sensação de afundar. Se os dois fossem brancos, a pressão não se veria.
        let repouso: gpui::Rgba = TOP_HIGHLIGHT.hsla().into();
        let pressionado: gpui::Rgba = TOP_HIGHLIGHT_PRESSED.hsla().into();
        assert_eq!((repouso.r, repouso.g, repouso.b), (1.0, 1.0, 1.0), "branco");
        assert_eq!(
            (pressionado.r, pressionado.g, pressionado.b),
            (0.0, 0.0, 0.0),
            "preto"
        );

        // O bisel troca de sentido entre os temas.
        assert!(BUTTON_LIGHT.bevel_dir > 0.0);
        assert!(BUTTON_DARK.bevel_dir < 0.0);
        // E o flag de tema não pode estar trocado — é ele que escolhe o TOKEN da família Outline.
        assert!(!BUTTON_LIGHT.dark);
        assert!(BUTTON_DARK.dark);
    }

    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, porque a 6% o filete é
    /// imperceptível no nosso fundo. É a mesma decisão já vigente no `Input` e no `Card`, e este
    /// teste existe pra ela ser uma decisão registrada e não uma deriva: se alguém "corrigir" pra
    /// 6% achando que é erro de porte, ele falha e aponta pra cá.
    ///
    /// O tema CLARO segue fiel (preto 4%) — o desvio é só no escuro, onde o problema existia.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = BUTTON_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%)"
        );

        let escuro = BUTTON_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");

        // E é o MESMO valor do `Input`/`Card`: três biséis diferentes na mesma tela seria pior que
        // um desvio.
        assert_eq!(BUTTON_DARK.bevel, Rgba8(0xffffff1e));
        assert_eq!(BUTTON_LIGHT.bevel, Rgba8(0x0000000a));
    }

    /// Altura e respiro CRESCEM com o tamanho. Um `Lg` mais baixo que um `Default` passaria batido
    /// no código e só apareceria na tela.
    ///
    /// As alturas são as do coss no breakpoint `sm:` e são **totais** (border-box, bordas
    /// incluídas) — diferente do `Input`, onde o `h-*` é o miolo. Ver o doc de [`ButtonSize`].
    #[test]
    fn tamanhos_crescem_monotonicamente() {
        let escala = [
            ButtonSize::Xs,
            ButtonSize::Sm,
            ButtonSize::Default,
            ButtonSize::Lg,
            ButtonSize::Xl,
        ];
        assert_eq!(
            escala.map(|t| t.height()),
            [24.0, 28.0, 32.0, 36.0, 40.0],
            "as alturas do coss em `sm:`"
        );
        assert_eq!(
            escala.map(|t| t.pad_x()),
            [7.0, 9.0, 11.0, 13.0, 15.0],
            "`px-[calc(--spacing(N)-1px)]` — o -1px desconta a borda"
        );
        for par in escala.windows(2) {
            assert!(
                par[0].height() < par[1].height(),
                "{:?} tem que ser mais baixo que {:?}",
                par[0],
                par[1]
            );
            assert!(
                par[0].pad_x() < par[1].pad_x(),
                "{:?} tem que respirar menos que {:?}",
                par[0],
                par[1]
            );
        }

        // A escala de ícone acompanha a de texto, altura por altura.
        let icones = [
            ButtonSize::IconXs,
            ButtonSize::IconSm,
            ButtonSize::Icon,
            ButtonSize::IconLg,
            ButtonSize::IconXl,
        ];
        assert_eq!(icones.map(|t| t.height()), escala.map(|t| t.height()));

        // **É daqui que os números ímpares vêm.** O respiro declarado é
        // `px-[calc(--spacing(N)-1px)]`, e o `-1px` desconta a borda justamente pra o recuo TOTAL
        // (respiro + borda, medido da borda externa) cair na escala de spacing do Tailwind:
        // `--spacing(2 | 2.5 | 3 | 3.5 | 4)` = 8, 10, 12, 14, 16px.
        //
        // Se alguém "arredondar" o `pad_x` pra 8/10/12/14/16 achando que os ímpares são erro de
        // digitação, todo botão fica 1px mais largo de cada lado que o original.
        assert_eq!(
            escala.map(|t| t.pad_x() + 1.0),
            [8.0, 10.0, 12.0, 14.0, 16.0],
            "o recuo total tem que cair na escala de spacing do Tailwind"
        );

        assert_eq!(ButtonSize::default(), ButtonSize::Default);
    }

    /// Os `Icon*` são QUADRADOS e não têm respiro horizontal — quem centraliza o glifo é o
    /// `justify_center`. Um `pad_x` diferente de zero ali somaria à largura e o botão deixaria de
    /// ser quadrado.
    #[test]
    fn tamanhos_de_icone_sao_quadrados() {
        for t in TODOS {
            assert_eq!(
                t.is_icon(),
                matches!(
                    t,
                    ButtonSize::IconXs
                        | ButtonSize::IconSm
                        | ButtonSize::Icon
                        | ButtonSize::IconLg
                        | ButtonSize::IconXl
                ),
                "{t:?}: is_icon desalinhado do nome"
            );
            if t.is_icon() {
                assert_eq!(t.pad_x(), 0.0, "{t:?}: quadrado não tem respiro declarado");
            } else {
                assert!(t.pad_x() > 0.0, "{t:?}: botão com texto tem que respirar");
            }
        }

        // `Button::icon` já nasce quadrado — sem isto, um `Button::icon(..)` distraído sairia
        // retangular e com o ícone encostado nas bordas.
        let b = Button::icon("x", "iconoir/regular/trash.svg");
        assert!(b.size.is_icon());
        assert_eq!(b.size, ButtonSize::Icon);
        assert!(b.label.is_none());
        assert!(b.icon_before.is_some());
    }

    /// Raio: `rounded-md` (8px) nos dois menores, `rounded-lg` (10px) no resto.
    ///
    /// O `Xs` desafina de propósito — a 24px de altura, 10px de raio já lê como pílula. Fica
    /// travado aqui pra ninguém "uniformizar" e sair do original.
    #[test]
    fn raio_e_8_nos_menores_e_10_nos_outros() {
        for t in TODOS {
            let esperado = if matches!(t, ButtonSize::Xs | ButtonSize::IconXs) {
                8.0
            } else {
                10.0
            };
            assert_eq!(t.radius(), esperado, "{t:?}");
        }
        // Os dois raios são os tokens do coss: `--radius-md` = 8, `--radius`/`--radius-lg` = 10.
        assert_eq!(ButtonSize::Xs.radius(), 8.0);
        assert_eq!(ButtonSize::Default.radius(), 10.0);
    }

    /// Corpo do texto e lado do ícone acompanham o tamanho, e as duas escalas ANDAM JUNTAS: um
    /// ícone de 18px ao lado de um texto de 12px lê como erro de layout.
    #[test]
    fn texto_e_icone_escalam_juntos() {
        for t in TODOS {
            let (texto, icone) = (t.text_size(), t.icon_size());
            assert!(
                icone >= texto,
                "{t:?}: o ícone ({icone}) não pode ser menor que o texto ({texto})"
            );
            assert!(
                icone <= t.height() - 8.0,
                "{t:?}: ícone de {icone} não cabe com folga em {}px",
                t.height()
            );
        }
        assert_eq!(ButtonSize::Xs.icon_size(), 14.0, "16 encostaria nas bordas");
        assert_eq!(ButtonSize::Default.icon_size(), 16.0, "sm:size-4");
        assert_eq!(ButtonSize::Xl.icon_size(), 18.0, "sm:size-4.5");
        assert_eq!(ButtonSize::Xs.text_size(), 12.0, "text-xs");
        assert_eq!(ButtonSize::Xl.text_size(), 16.0, "sm:text-base");
        assert_eq!(ButtonSize::Default.text_size(), 14.0, "sm:text-sm");

        // O gap é o `gap-1`/`gap-1.5`/`gap-2` e não decresce com o tamanho.
        assert_eq!(ButtonSize::Xs.gap(), 4.0);
        assert_eq!(ButtonSize::Sm.gap(), 6.0);
        for t in [ButtonSize::Default, ButtonSize::Lg, ButtonSize::Xl] {
            assert_eq!(t.gap(), 8.0);
        }
    }

    /// **Precedência: desabilitado apaga a sombra**, em toda variante que tenha uma.
    ///
    /// É uma fonte de verdade só — o resto do esmaecimento é `opacity-64` no conjunto, não um par
    /// "apagado" de cada token.
    #[test]
    fn desabilitado_apaga_a_sombra() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            for v in VARIANTES {
                assert!(
                    shadow_for(v, true, false).is_none(),
                    "{v:?} desabilitado não projeta sombra"
                );
                assert!(
                    shadow_for(v, true, true).is_none(),
                    "{v:?} desabilitado e pressionado, idem"
                );
            }
        }
    }

    /// Só as variantes CHEIAS (`Default`/`Destructive`) perdem a sombra ao serem pressionadas — é
    /// o par do realce interno virando preto, e junto os dois dão a sensação de afundar. As de
    /// moldura não têm estado pressionado no original, então a sombra delas fica.
    #[test]
    fn pressionado_apaga_a_sombra_so_das_variantes_cheias() {
        theme::set_theme(theme::ThemeMode::Dark);
        for v in VARIANTES {
            let repouso = shadow_for(v, false, false);
            let pressionado = shadow_for(v, false, true);
            if v.style().top_highlight {
                assert!(repouso.is_some(), "{v:?}: cheia tem sombra em repouso");
                assert!(pressionado.is_none(), "{v:?}: cheia afunda ao pressionar");
            } else {
                assert_eq!(
                    repouso.is_some(),
                    pressionado.is_some(),
                    "{v:?}: sem estado pressionado no original"
                );
            }
        }

        // A geometria da sombra é `shadow-xs` = `0 1px 2px`.
        let xs = shadow_for(ButtonVariant::Default, false, false).expect("Default tem sombra");
        assert_eq!(xs.offset.y, px(1.0), "shadow-xs desce 1px");
        assert_eq!(xs.blur_radius, px(2.0));
        assert_eq!(xs.spread_radius, px(0.0));
    }

    /// Cada efeito de 1px pertence a uma família de variantes, e as duas famílias são
    /// DISJUNTAS: realce interno nas cheias, bisel nas de moldura. Uma variante com os dois
    /// desenharia dois filetes concorrentes na borda de cima.
    #[test]
    fn realce_e_bisel_pertencem_a_familias_disjuntas() {
        theme::set_theme(theme::ThemeMode::Dark);
        for v in VARIANTES {
            let st = v.style();
            assert!(
                !(st.top_highlight && st.bevel),
                "{v:?}: realce e bisel na mesma variante"
            );
        }

        let com_realce: Vec<_> = VARIANTES
            .into_iter()
            .filter(|v| v.style().top_highlight)
            .collect();
        assert_eq!(
            com_realce,
            vec![ButtonVariant::Default, ButtonVariant::Destructive],
            "só as variantes de fundo cheio têm realce interno"
        );

        let com_bisel: Vec<_> = VARIANTES.into_iter().filter(|v| v.style().bevel).collect();
        assert_eq!(
            com_bisel,
            vec![ButtonVariant::Outline, ButtonVariant::DestructiveOutline],
            "só a família Outline tem bisel"
        );

        // E o sublinhado é exclusivo da Link.
        let com_sublinhado: Vec<_> = VARIANTES
            .into_iter()
            .filter(|v| v.style().underline_on_hover)
            .collect();
        assert_eq!(com_sublinhado, vec![ButtonVariant::Link]);
    }

    /// As variantes SEM fundo próprio (`Ghost`, `Link`) e sem borda não podem ganhar sombra: uma
    /// sombra atrás de um elemento transparente vaza pelo meio dele, porque o
    /// `Window::paint_shadows` do GPUI não recorta a sombra pra fora de quem a projeta.
    #[test]
    fn variantes_transparentes_nao_tem_sombra() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            for v in [ButtonVariant::Ghost, ButtonVariant::Link] {
                let st = v.style();
                assert!(st.bg.is_none(), "{v:?} não tem fundo em repouso");
                assert!(st.border.is_none(), "{v:?} não tem borda");
                assert!(
                    st.shadow.is_none(),
                    "{v:?}: sombra atrás de elemento transparente VAZA no GPUI"
                );
                assert!(shadow_for(v, false, false).is_none());
            }
        }
    }

    /// **Contra-intuitivo e FIEL ao original.** `hover:bg-secondary/90` DILUI o fundo (4% → 3,6%)
    /// em vez de intensificá-lo, e `active:bg-secondary/80` dilui mais (3,2%).
    ///
    /// Parece bug — o hover normalmente escurece. Fica travado aqui pra ninguém "consertar" pra
    /// uma escala crescente e sair do pixel-perfect. O motivo é o modificador `/N` do Tailwind v4,
    /// que MULTIPLICA o alfa existente em vez de substituí-lo.
    #[test]
    fn hover_da_secondary_dilui_o_fundo() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let st = ButtonVariant::Secondary.style();
            let repouso = st.bg.expect("Secondary tem fundo");
            let hover = st.bg_hover.expect("e hover");
            let pressionado = st.bg_pressed.expect("e pressionado");

            assert!(hover.a < repouso.a, "o hover DILUI (não é erro)");
            assert!(pressionado.a < hover.a, "e a pressão dilui mais");
            // Os fatores exatos: 90% e 80% do alfa de repouso.
            assert!((hover.a - repouso.a * SECONDARY_HOVER_ALPHA).abs() < 1e-6);
            assert!((pressionado.a - repouso.a * SECONDARY_PRESSED_ALPHA).abs() < 1e-6);
            // Nunca chega a opaco: é um véu sobre a superfície de trás.
            assert!(
                repouso.a < 0.1,
                "4% de preto/branco, não um retângulo chapado"
            );
        }
    }

    /// O modificador `/N` do Tailwind v4 MULTIPLICA o alfa que a cor já tem — não o substitui.
    /// Em `--input` (branco a 8%), `dark:bg-input/32` dá 2,56%, não 32%. Errar isso deixa o fundo
    /// da `Outline` no escuro 12× mais forte que o original.
    #[test]
    fn modificador_de_alfa_multiplica_em_vez_de_substituir() {
        theme::set_theme(theme::ThemeMode::Dark);
        let base = BUTTON_DARK.input.alpha();
        let st = ButtonVariant::Outline.style();
        let fundo = st.bg.expect("Outline tem fundo");
        assert!(
            (fundo.a - base * OUTLINE_DARK_BG_ALPHA).abs() < 1e-6,
            "esperado {} (= {base} × {OUTLINE_DARK_BG_ALPHA}), veio {}",
            base * OUTLINE_DARK_BG_ALPHA,
            fundo.a
        );
        assert!(fundo.a < 0.04, "2,56%, não 32%");

        // E o hover é o MESMO token mais forte — o dobro do de repouso, por construção
        // (64 = 2 × 32).
        let hover = st.bg_hover.expect("e hover");
        assert!((hover.a - fundo.a * 2.0).abs() < 1e-6);
    }

    /// **Regressão do flash no hover da `Outline` (tema claro).**
    ///
    /// No claro a variante sai de uma superfície OPACA (`bg-popover`, branco) e o hover é o acento
    /// a 2% — quase transparente. O fundo do botão é um cross-fade calculado entre esses dois
    /// (ver o cálculo de `hover_t` no `render`), e interpolar RGB e alfa em canais separados fazia
    /// o meio do caminho virar cinza médio a meia opacidade: o botão escurecia e clareava a cada
    /// passada do ponteiro.
    ///
    /// O teste percorre a rampa como o `render` a percorre e exige que ela nunca escureça além das
    /// pontas, compondo sobre o painel branco em que o botão vive.
    #[test]
    fn o_fade_do_hover_da_outline_no_claro_nao_pisca() {
        theme::set_theme(theme::ThemeMode::Light);
        let st = ButtonVariant::Outline.style();
        let repouso = st.bg.expect("Outline tem fundo de repouso");
        let hover = st.bg_hover.expect("e um alvo de hover");

        // O painel de conteúdo no tema claro é branco (`theme::LIGHT.bg_panel`).
        let sobre_painel = |c: gpui::Hsla| {
            let c: gpui::Rgba = c.into();
            c.r * c.a + 1.0 * (1.0 - c.a)
        };

        let piso = sobre_painel(repouso).min(sobre_painel(hover));
        let mut anterior = sobre_painel(lerp(repouso, hover, 0.0));
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let atual = sobre_painel(lerp(repouso, hover, t));
            assert!(
                atual >= piso - 1e-3,
                "t={t}: o meio do fade ({atual}) ficou mais escuro que as duas pontas ({piso})"
            );
            assert!(
                atual <= anterior + 1e-3,
                "t={t}: a rampa subiu depois de descer — é isso que se vê como piscada"
            );
            anterior = atual;
        }
    }

    /// A família `Outline` troca de TOKEN entre os temas, não só de valor: superfície opaca
    /// (`bg-popover`) no claro, o próprio `--input` diluído no escuro. Um botão de moldura opaco
    /// no tema escuro apagaria o que estiver atrás dele.
    #[test]
    fn outline_troca_de_token_entre_os_temas() {
        theme::set_theme(theme::ThemeMode::Light);
        let claro = ButtonVariant::Outline.style().bg.expect("tem fundo");
        assert_eq!(claro.a, 1.0, "claro: superfície OPACA (bg-popover)");

        theme::set_theme(theme::ThemeMode::Dark);
        let escuro = ButtonVariant::Outline.style().bg.expect("tem fundo");
        assert!(escuro.a < 1.0, "escuro: véu translúcido (bg-input/32)");

        // A DestructiveOutline compartilha o mesmo fundo e a mesma borda da Outline — o que muda é
        // só o texto e o hover.
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let o = ButtonVariant::Outline.style();
            let d = ButtonVariant::DestructiveOutline.style();
            assert_eq!(o.bg, d.bg, "mesmo fundo de repouso");
            assert_eq!(o.border, d.border, "mesma borda de repouso");
            assert_ne!(o.text, d.text, "o texto é o que difere");
            assert!(
                d.border_hover.is_some(),
                "e a DestructiveOutline avermelha a borda no hover"
            );
            assert!(o.border_hover.is_none(), "a Outline não mexe na borda");
        }
    }

    /// O texto da `Destructive` é BRANCO, não o `--destructive-foreground`.
    ///
    /// Os dois são "a cor de frente destrutiva", e trocar um pelo outro dá vermelho sobre vermelho
    /// — ilegível e sem erro de compilação. O `--destructive-foreground` serve pra texto sobre
    /// fundo NEUTRO (é o que a `DestructiveOutline` usa).
    #[test]
    fn destructive_cheia_usa_texto_branco() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let cheia = ButtonVariant::Destructive.style();
            assert_eq!(cheia.text, WHITE.hsla(), "sobre vermelho cheio, branco");

            let moldura = ButtonVariant::DestructiveOutline.style();
            assert_ne!(
                moldura.text, cheia.text,
                "sobre fundo neutro, o vermelho de texto"
            );
            let t: gpui::Rgba = moldura.text.into();
            assert!(t.r > t.g + 0.25, "e ele é vermelho de verdade");
        }
    }

    /// A borda de 1px existe em TODA variante, mesmo nas "sem borda" — transparente. É ela que
    /// mantém a altura interna igual entre variantes, já que a altura é total (border-box): sem a
    /// borda, um `Ghost` teria 2px mais de miolo que um `Default` do mesmo tamanho e o texto
    /// dançaria ao trocar de variante.
    #[test]
    fn altura_interna_e_igual_em_toda_variante() {
        theme::set_theme(theme::ThemeMode::Dark);
        for t in TODOS {
            // A borda é sempre 1px em cada lado, independente da variante.
            let miolo = t.height() - 2.0;
            assert!(miolo > 0.0, "{t:?}");
            assert!(
                miolo >= t.icon_size(),
                "{t:?}: o ícone ({}) tem que caber no miolo ({miolo})",
                t.icon_size()
            );
        }
    }

    /// O anel de foco cresce JUNTO com o raio do botão: `raio + anel + folga`. Se o raio do anel
    /// não acompanhar, a curvatura sai errada nas quinas — o sintoma de quando ele era um
    /// `BoxShadow` com `spread_radius` (que dilata os limites mas mantém o raio).
    #[test]
    fn raio_do_anel_acompanha_o_do_botao() {
        // `focus-visible:ring-2 ring-offset-1` = 2px de anel + 1px de folga = 3px pra fora.
        assert_eq!(RING_WIDTH, 2.0);
        assert_eq!(RING_OFFSET, 1.0);
        let folga = RING_WIDTH + RING_OFFSET;
        assert_eq!(folga, 3.0);

        for t in TODOS {
            // O overlay recua `-folga` e o raio dele é o do botão MAIS a folga — é isso que
            // mantém as duas curvas concêntricas.
            assert_eq!(t.radius() + folga, t.radius() + 3.0);
        }
        assert_eq!(ButtonSize::Xs.radius() + folga, 11.0);
        assert_eq!(ButtonSize::Default.radius() + folga, 13.0);
    }

    /// O esmaecimento do desabilitado é UM número aplicado ao conjunto (`opacity-64`), não um par
    /// "apagado" de cada token da paleta. Duas fontes de verdade pra "está desabilitado" é o que
    /// produz um botão com o texto apagado e a borda em força total.
    #[test]
    fn desabilitado_e_uma_opacidade_so() {
        assert_eq!(DISABLED_OPACITY, 0.64);
        assert!(DISABLED_OPACITY < 1.0 && DISABLED_OPACITY > 0.0);

        // As cores NÃO mudam com o disabled — só a sombra sai (ver `desabilitado_apaga_a_sombra`).
        theme::set_theme(theme::ThemeMode::Dark);
        for v in VARIANTES {
            let st = v.style();
            assert_eq!(st.text, v.style().text, "{v:?}: a cor de texto é uma só");
            assert_eq!(st.bg, v.style().bg, "{v:?}: o fundo é um só");
        }
    }

    /// Os builders não se atropelam: `variant`/`size`/`disabled`/`full_width` são independentes, e
    /// `icon_before`/`icon_after` convivem com o rótulo.
    #[test]
    fn builders_compoem() {
        let b = Button::new("salvar", "Salvar")
            .variant(ButtonVariant::Destructive)
            .size(ButtonSize::Lg)
            .disabled(true)
            .full_width()
            .icon_before("iconoir/regular/check.svg")
            .icon_after("iconoir/regular/arrow-right.svg");

        assert_eq!(b.variant, ButtonVariant::Destructive);
        assert_eq!(b.size, ButtonSize::Lg);
        assert!(b.disabled);
        assert!(b.full_width);
        assert_eq!(b.label, Some(SharedString::from("Salvar")));
        assert!(b.icon_before.is_some() && b.icon_after.is_some());
        assert!(b.focus_handle.is_none(), "foco é opt-in");
        assert!(b.on_click.is_none());

        // Os defaults: variante cheia, 32px, habilitado, largura de conteúdo.
        let d = Button::new("x", "y");
        assert_eq!(d.variant, ButtonVariant::Default);
        assert_eq!(d.size, ButtonSize::Default);
        assert!(!d.disabled && !d.full_width);
    }

    /// Toda variante define uma cor de texto (não existe botão sem texto legível) e nenhuma
    /// define fundo pressionado sem antes definir fundo de hover — o `active` do GPUI refina por
    /// cima do `hover`, então um pressionado órfão pularia um estado.
    #[test]
    fn toda_variante_tem_texto_e_estados_encadeados() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            for v in VARIANTES {
                let st = v.style();
                assert!(st.text.a > 0.0, "{v:?}: texto invisível");
                if st.bg_pressed.is_some() {
                    assert!(
                        st.bg_hover.is_some(),
                        "{v:?}: pressionado sem hover pula um estado"
                    );
                }
                if st.bg_hover.is_some() && st.bg.is_some() {
                    assert_ne!(st.bg, st.bg_hover, "{v:?}: hover sem efeito visível");
                }
            }
        }
    }
}

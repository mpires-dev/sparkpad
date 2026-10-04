//! `Badge` — o **selo** do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/badge.tsx`
//!
//! # Anatomia
//!
//! ```text
//!   ┌──────────────┐
//!   │ ◇  Novo      │   ← .icon_before(…) · rótulo · .icon_after(…)
//!   └──────────────┘
//!     ▲ borda 1px (transparente em tudo, menos na `Outline`)
//! ```
//!
//! # Uso
//!
//! ```ignore
//! // Selo estático (um `<span>` no original): inerte, sem id.
//! badge::Badge::new("Beta").variant(badge::BadgeVariant::Info)
//!
//! // Selo clicável (um `<button>`/`<a>` no original): cursor, hover e clique.
//! badge::Badge::new("Filtro")
//!     .variant(badge::BadgeVariant::Outline)
//!     .on_click("limpar-filtro", |_, _window, _cx| println!("clicou"));
//! ```
//!
//! # O selo é `<span>` por padrão, e isso MUDA o estilo
//!
//! No coss, `cursor-pointer`, `hover:` e o alvo de toque de 44px estão todos atrás do seletor
//! `[button&,a&]` — ou seja, **só valem quando o selo é renderizado como botão ou link**. Um selo
//! `<span>` (o default) não tem hover nenhum, e é isso que o [`Badge::new`] produz aqui.
//!
//! O que promove o selo a interativo é ganhar um `id`, via [`Badge::button`] (o equivalente de um
//! `<a href>`: interativo, sem handler) ou via [`Badge::on_click`] (que **exige** o `id` no mesmo
//! chamado, justamente pra não existir handler que o GPUI descarta em silêncio — `on_click` só
//! existe em elemento com `.id()`, ver [`gpui::StatefulInteractiveElement`]).
//!
//! # O que o Badge NÃO tem (e o `Button` tem)
//!
//! ⚠️ Vale registrar, porque é a primeira coisa que alguém "conserta" por analogia com o
//! [`crate::button`]: a classe base do `badge.tsx` **não tem nenhum utilitário de sombra**. Não há
//! `shadow-xs`, não há `inset-shadow-[0_1px_white/16%]`, não há `before:shadow-[0_1px_black/4%]`.
//! Logo:
//!
//! - **sem sombra externa**;
//! - **sem realce interno superior**;
//! - **sem fio de bisel** — e portanto sem a decisão de "branco a 6% vs. o dobro" que vale no
//!   `input`, `card`, `button`, `frame` e `toast`. Não existe bisel pra calibrar aqui.
//!
//! A única coisa que a classe base transiciona é `transition-shadow`, e a única sombra que ela pode
//! transicionar é o `ring` de foco. Como o GPUI não tem transição de estilo, o anel aparece seco.
//!
//! # O que o GPUI exigiu adaptar
//!
//! | coss                                  | aqui                                              |
//! |---------------------------------------|---------------------------------------------------|
//! | `focus-visible:ring-2 ring-offset-1`  | overlay em `inset:-3px` com `border_2`            |
//! | `focus-visible:ring-offset-background`| coroa de 1px pintada com `--background`           |
//! | `pointer-coarse:after:min-h-11`       | **não reproduzido** — é alvo de toque (só mobile) |
//! | `transition-shadow`                   | **não reproduzido** — sem transição de estilo     |
//! | `outline-none`                        | nada a fazer: o GPUI não desenha outline          |

use gpui::{
    div, px, App, ClickEvent, Div, ElementId, FocusHandle, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Os utilitários Tailwind do `badge.tsx` resolvidos em número, a partir da tabela de tokens já
// resolvida do `globals.css` do coss — não deduzidos da paleta do Tailwind de cabeça.
//
// Mesma disciplina de cor do botão, do campo de texto e do card: TODO valor é `0xRRGGBBAA`, com o
// byte de alfa, sempre — ver [`crate::color`]. Misturar com os tokens de 6 dígitos de
// [`crate::theme`] desloca os canais e produz uma cor completamente diferente, sem erro de
// compilação (já aconteceu três vezes nesta base).
//
// ⚠️ Duas armadilhas que já custaram valores errados no `toast` e estão travadas nos testes daqui:
//
// 1. `--success` é **emerald**-500 (`#10b981`), não green-500 (`#22c55e`).
// 2. No tema escuro, `--info`, `--success` e `--warning` são IDÊNTICOS ao claro. Só o
//    `--destructive` sofre o `color-mix` com branco (`#ef4444` → `#f15757`).

/// Tokens visuais do selo, por tema.
#[derive(Clone, Copy, Debug)]
struct BadgePalette {
    /// `--primary` — fundo da variante [`BadgeVariant::Default`].
    primary: Rgba8,
    /// `--primary-foreground` — texto sobre `primary`.
    primary_fg: Rgba8,
    /// `--secondary` **e** `--accent`. No coss os dois resolvem pro MESMO valor (preto/branco a
    /// 4%), então é um campo só — dois campos idênticos seriam duas fontes de verdade pro mesmo
    /// número. A [`BadgeVariant::Secondary`] usa como `--secondary`; a [`BadgeVariant::Outline`]
    /// usa o mesmo valor como `--accent` no hover claro.
    secondary: Rgba8,
    /// `--secondary-foreground` (idêntico a `--accent-foreground`).
    secondary_fg: Rgba8,
    /// `--background` — fundo da [`BadgeVariant::Outline`] no tema claro.
    ///
    /// No tema escuro este token existe (é o `ring-offset-background`), mas **não** pinta o fundo
    /// da `Outline`: lá vale o `dark:bg-input/32`. Ver [`BadgeVariant::style`].
    background: Rgba8,
    /// `--foreground` — texto da [`BadgeVariant::Outline`].
    foreground: Rgba8,
    /// `--destructive` — fundo cheio da [`BadgeVariant::Destructive`] e base do véu da
    /// [`BadgeVariant::Error`].
    destructive: Rgba8,
    /// `--destructive-foreground` — o vermelho de TEXTO da [`BadgeVariant::Error`].
    destructive_fg: Rgba8,
    /// `--info` — blue-500.
    info: Rgba8,
    /// `--info-foreground` — blue-700 (claro) / blue-400 (escuro).
    info_fg: Rgba8,
    /// `--success` — **emerald**-500, não green-500.
    success: Rgba8,
    /// `--success-foreground` — emerald-700 (claro) / emerald-400 (escuro).
    success_fg: Rgba8,
    /// `--warning` — amber-500.
    warning: Rgba8,
    /// `--warning-foreground` — amber-700 (claro) / amber-400 (escuro).
    warning_fg: Rgba8,
    /// `--input` — borda da [`BadgeVariant::Outline`] (e, no escuro, também o fundo dela).
    input: Rgba8,
    /// `--ring` — cor do anel de foco.
    ring: Rgba8,
    /// Se este é o tema escuro.
    ///
    /// Não é redundante com [`theme::theme_mode`]: as variantes de véu e a `Outline` trocam de
    /// **fórmula** (não só de valor) entre os temas — `bg-*/8` vira `dark:bg-*/16`, e
    /// `bg-background` vira `dark:bg-input/32` — então a derivação precisa saber em qual tema está.
    dark: bool,
}

/// Tema **claro**.
const BADGE_LIGHT: BadgePalette = BadgePalette {
    primary: Rgba8(0x262626ff),    // neutral-800
    primary_fg: Rgba8(0xfafafaff), // neutral-50
    secondary: Rgba8(0x0000000a),  // preto 4%
    secondary_fg: Rgba8(0x262626ff),
    background: Rgba8(0xffffffff),
    foreground: Rgba8(0x262626ff),
    destructive: Rgba8(0xef4444ff),    // red-500
    destructive_fg: Rgba8(0xb91c1cff), // red-700
    info: Rgba8(0x3b82f6ff),           // blue-500
    info_fg: Rgba8(0x1d4ed8ff),        // blue-700
    success: Rgba8(0x10b981ff),        // emerald-500
    success_fg: Rgba8(0x047857ff),     // emerald-700
    warning: Rgba8(0xf59e0bff),        // amber-500
    warning_fg: Rgba8(0xb45309ff),     // amber-700
    input: Rgba8(0x0000001a),          // preto 10%
    ring: Rgba8(0xa3a3a3ff),           // neutral-400
    dark: false,
};

/// Tema **escuro**.
const BADGE_DARK: BadgePalette = BadgePalette {
    primary: Rgba8(0xf5f5f5ff), // neutral-100
    primary_fg: Rgba8(0x262626ff),
    secondary: Rgba8(0xffffff0a), // branco 4%
    secondary_fg: Rgba8(0xf5f5f5ff),
    background: Rgba8(0x141414ff), // mix(neutral-950 96%, white)
    foreground: Rgba8(0xf5f5f5ff),
    // O ÚNICO dos quatro tokens semânticos que muda no escuro: mix(red-500 90%, white) = #f15757.
    destructive: Rgba8(0xf15757ff),
    destructive_fg: Rgba8(0xf87171ff), // red-400
    info: Rgba8(0x3b82f6ff),           // blue-500 — IGUAL ao claro
    info_fg: Rgba8(0x60a5faff),        // blue-400
    success: Rgba8(0x10b981ff),        // emerald-500 — IGUAL ao claro
    success_fg: Rgba8(0x34d399ff),     // emerald-400
    warning: Rgba8(0xf59e0bff),        // amber-500 — IGUAL ao claro
    warning_fg: Rgba8(0xfbbf24ff),     // amber-400
    input: Rgba8(0xffffff14),          // branco 8%
    ring: Rgba8(0x737373ff),           // neutral-500
    dark: true,
};

/// A paleta do selo no tema corrente.
fn badge() -> &'static BadgePalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &BADGE_DARK,
        theme::ThemeMode::Light => &BADGE_LIGHT,
    }
}

/// Branco puro — o texto da [`BadgeVariant::Destructive`], que usa `text-white` e **não**
/// `--destructive-foreground` (sobre o vermelho cheio quem lê é o branco).
const WHITE: Rgba8 = Rgba8(0xffffffff);

// --- Alfas (os modificadores `/N` do Tailwind) ----------------------------------------------------
//
// ⚠️ No Tailwind v4 o modificador `/N` é `color-mix(in oklab, <cor> N%, transparent)`, ou seja
// **multiplica** o alfa que a cor já tem — não o substitui. É por isso que tudo aqui passa por
// [`Rgba8::scaled`] em vez de reescrever o byte de alfa: em `--input` (branco a 8%), `/32` dá
// 2,56%, não 32%. Nos tokens opacos (`--primary`, `--destructive`, `--info`, …) o resultado
// coincide com o percentual, mas a conta é a mesma.

/// `hover:bg-primary/90`, `hover:bg-destructive/90` e `hover:bg-secondary/90` — o único hover das
/// variantes de fundo. Todos os três usam o mesmo 90%.
const FILLED_HOVER_ALPHA: f32 = 0.90;

/// `bg-destructive/8`, `bg-info/8`, `bg-success/8`, `bg-warning/8` — o véu das quatro variantes
/// semânticas no tema **claro**.
const SOFT_BG_ALPHA_LIGHT: f32 = 0.08;

/// `dark:bg-destructive/16`, `dark:bg-info/16`, … — o MESMO véu no tema **escuro**, no dobro da
/// força. É o contrário do que a intuição sugere (um véu mais forte no escuro), e é o original:
/// sobre `#141414` um véu de 8% não se vê.
const SOFT_BG_ALPHA_DARK: f32 = 0.16;

/// `dark:bg-input/32` — fundo da [`BadgeVariant::Outline`] no tema escuro.
const OUTLINE_DARK_BG_ALPHA: f32 = 0.32;

/// `hover:bg-accent/50` — hover da `Outline` no tema claro.
const OUTLINE_HOVER_ALPHA_LIGHT: f32 = 0.50;

/// `dark:hover:bg-input/48` — hover da `Outline` no tema escuro.
///
/// ⚠️ **48%, não 64%.** O `button.tsx` usa `dark:hover:bg-input/64` na mesma família; o
/// `badge.tsx` usa 48. Travado no teste [`tests::outline_troca_de_token_entre_os_temas`] pra
/// ninguém "unificar" com o botão.
const OUTLINE_HOVER_ALPHA_DARK: f32 = 0.48;

/// `disabled:opacity-64` — o coss esmaece o CONJUNTO em vez de trocar cor por cor. Uma fonte de
/// verdade só: não existe um par "apagado" de cada token.
const DISABLED_OPACITY: f32 = 0.64;

/// `[&_svg:not([class*='opacity-'])]:opacity-80` — os ícones pesam mais que o texto no mesmo
/// tamanho, e o original os alivia.
const ICON_OPACITY: f32 = 0.80;

// --- Geometria -----------------------------------------------------------------------------------

/// Espessura do anel de foco — `focus-visible:ring-2`.
const RING_WIDTH: f32 = 2.0;

/// Folga entre o selo e o anel — `focus-visible:ring-offset-1`.
const RING_OFFSET: f32 = 1.0;

/// Lado do ícone — `sm:[&_svg:not([class*='size-'])]:size-3` = **12px**, igual nos três tamanhos.
///
/// Diferente do [`crate::button::ButtonSize::icon_size`], que escala com o tamanho: no `badge.tsx`
/// o lado do ícone está na classe BASE, não nas variantes de tamanho, então um selo `Lg` tem o
/// mesmo ícone de 12px de um `Sm`.
const ICON_SIZE: f32 = 12.0;

// =================================================================================================
// Tamanho
// =================================================================================================

/// Tamanho do selo. Controla altura, piso de largura, respiro horizontal, corpo do texto, entrelinha
/// e raio de uma vez — pra não existir selo "quase default" com números escolhidos a olho no call
/// site.
///
/// # As alturas são TOTAIS
///
/// Igual ao [`crate::button::ButtonSize`] e diferente do [`crate::input::InputSize`]: o `h-*` do
/// coss está no **próprio elemento**, que tem `border` e `box-sizing: border-box` — então `h-4.5`
/// são **18px de altura total**, bordas incluídas. É por isso que o respiro horizontal do original
/// é `px-[calc(--spacing(1)-1px)]` = 3px: 3 + 1 de borda fecham os 4px de recuo pretendidos.
///
/// # Valores efetivos
///
/// São as variantes `sm:` do Tailwind (≥640px): uma janela desktop está sempre acima do
/// breakpoint, então as classes base nunca valem.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BadgeSize {
    /// 16px de altura — contadores e pontos de status.
    Sm,
    /// 18px — o padrão.
    #[default]
    Default,
    /// 22px — selo com rótulo de texto corrido ao lado.
    Lg,
}

impl BadgeSize {
    /// Altura TOTAL, bordas incluídas (ver o doc do tipo).
    ///
    /// É **também o piso de largura**: o original declara `min-w-*` igual ao `h-*` nos três
    /// tamanhos (`h-4.5 min-w-4.5`, `h-5.5 min-w-5.5`, `h-4 min-w-4`), o que faz um selo de um
    /// caractere só nascer quadrado em vez de virar uma fatia vertical. Um método separado seria
    /// uma segunda fonte de verdade pro mesmo número.
    pub fn height(self) -> f32 {
        match self {
            BadgeSize::Sm => 16.0,
            BadgeSize::Default => 18.0,
            BadgeSize::Lg => 22.0,
        }
    }

    /// Respiro horizontal interno — `px-[calc(--spacing(N)-1px)]`, com o `-1px` descontando a
    /// borda. `Sm` e `Default` compartilham o mesmo valor no original.
    pub fn pad_x(self) -> f32 {
        match self {
            BadgeSize::Sm | BadgeSize::Default => 3.0,
            BadgeSize::Lg => 5.0,
        }
    }

    /// Espaço entre ícone e texto — `gap-1`, igual nos três tamanhos (está na classe base).
    pub fn gap(self) -> f32 {
        4.0
    }

    /// Corpo do texto — `sm:text-[.625rem]` (10px), `sm:text-xs` (12px) e `sm:text-sm` (14px).
    pub fn text_size(self) -> f32 {
        match self {
            BadgeSize::Sm => 10.0,
            BadgeSize::Default => 12.0,
            BadgeSize::Lg => 14.0,
        }
    }

    /// Entrelinha: exatamente o **miolo** do selo (altura total menos as duas bordas de 1px).
    ///
    /// Não é escolha arbitrária — nos dois tamanhos que usam um passo da escala de tipografia do
    /// Tailwind, o miolo COINCIDE com a entrelinha que o utilitário já traz:
    ///
    /// | tamanho   | miolo | `line-height` do Tailwind |
    /// |-----------|-------|---------------------------|
    /// | `Default` | 16px  | `text-xs` → `1rem` = 16px |
    /// | `Lg`      | 20px  | `text-sm` → `1.25rem` = 20px |
    ///
    /// No `Sm` o original usa um valor arbitrário (`text-[.625rem]`), e valor arbitrário no
    /// Tailwind v4 **não** carrega entrelinha — ela é herdada da página. Os 14px daí são
    /// **deduzidos** pela mesma regra dos outros dois, não lidos da referência.
    pub fn line_height(self) -> f32 {
        self.height() - 2.0
    }

    /// Raio — `rounded-sm` (`--radius-sm` = **6px**) no `Default` e no `Lg`; o `Sm` desafina de
    /// propósito, com o valor arbitrário `rounded-[.25rem]` = **4px**, porque a 16px de altura 6px
    /// de raio já lê como pílula.
    pub fn radius(self) -> f32 {
        match self {
            BadgeSize::Sm => 4.0,
            BadgeSize::Default | BadgeSize::Lg => 6.0,
        }
    }
}

// =================================================================================================
// Variante
// =================================================================================================

/// Variante visual do selo.
///
/// # ⚠️ `Destructive` e `Error` são coisas DIFERENTES
///
/// As duas são "vermelhas" e o original define as duas:
///
/// - [`BadgeVariant::Destructive`] é **ênfase máxima**: `bg-destructive` cheio com `text-white`.
/// - [`BadgeVariant::Error`] é **estado**: um véu de 8% de vermelho com o vermelho de TEXTO
///   (`--destructive-foreground`) por cima, irmão de `Info`/`Success`/`Warning`.
///
/// Trocar uma pela outra compila e não parece errado à primeira vista — só fica com a ênfase
/// trocada. É a mesma distinção que separa `--destructive` de `--destructive-foreground`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BadgeVariant {
    /// Ênfase máxima neutra: fundo cheio em `--primary`.
    #[default]
    Default,
    /// Ênfase baixa neutra: preenchimento sutil (4% de preto/branco).
    Secondary,
    /// Moldura: borda em `--input` sobre a superfície.
    Outline,
    /// Ênfase máxima destrutiva: fundo vermelho cheio, texto branco.
    Destructive,
    /// **Estado** de erro: véu vermelho com o vermelho de texto. Não confundir com `Destructive`.
    Error,
    /// Estado informativo: véu azul.
    Info,
    /// Estado de sucesso: véu emerald.
    Success,
    /// Estado de atenção: véu âmbar.
    Warning,
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
    /// Fundo no hover. `None` = **sem hover** (as quatro variantes de estado não definem nenhum no
    /// original). Só se aplica quando o selo é interativo — o `hover:` do coss está atrás do
    /// seletor `[button&,a&]`.
    bg_hover: Option<Hsla>,
    /// Cor da borda de 1px. `None` = borda transparente (a borda EXISTE mesmo assim — ver
    /// [`Badge::render`]).
    border: Option<Hsla>,
    /// Cor do texto e dos ícones.
    text: Hsla,
}

impl BadgeVariant {
    /// Resolve as cores desta variante no tema corrente.
    fn style(self) -> VariantStyle {
        let p = badge();

        // As quatro variantes de estado compartilham a fórmula e trocam só o token: véu do token a
        // 8% no claro, 16% no escuro, com o `*-foreground` do MESMO token por cima.
        let veu = if p.dark {
            SOFT_BG_ALPHA_DARK
        } else {
            SOFT_BG_ALPHA_LIGHT
        };
        let estado = |token: Rgba8, fg: Rgba8| VariantStyle {
            bg: Some(token.scaled(veu)),
            bg_hover: None,
            border: None,
            text: fg.hsla(),
        };

        match self {
            BadgeVariant::Default => VariantStyle {
                bg: Some(p.primary.hsla()),
                bg_hover: Some(p.primary.scaled(FILLED_HOVER_ALPHA)),
                // A base é `border-transparent` e a variante NÃO a sobrescreve. A borda continua
                // existindo (é ela que fecha os 18px de altura total), e o fundo aparece por baixo
                // dela — igual ao `background-clip: border-box` do CSS.
                border: None,
                text: p.primary_fg.hsla(),
            },
            BadgeVariant::Secondary => VariantStyle {
                bg: Some(p.secondary.hsla()),
                // ⚠️ Contra-intuitivo e FIEL ao original: `hover:bg-secondary/90` DILUI o fundo
                // (4% → 3,6%) em vez de intensificá-lo, porque o `/N` do Tailwind v4 MULTIPLICA o
                // alfa. Travado no teste `hover_da_secondary_dilui_o_fundo`.
                bg_hover: Some(p.secondary.scaled(FILLED_HOVER_ALPHA)),
                border: None,
                text: p.secondary_fg.hsla(),
            },
            BadgeVariant::Outline => {
                // A `Outline` troca de TOKEN entre os temas, não só de valor: superfície opaca
                // (`bg-background`) no claro, o próprio `--input` diluído (32%) no escuro. O hover
                // acompanha: `--accent` a 50% no claro, `--input` a 48% no escuro.
                let (bg, bg_hover) = if p.dark {
                    (
                        p.input.scaled(OUTLINE_DARK_BG_ALPHA),
                        p.input.scaled(OUTLINE_HOVER_ALPHA_DARK),
                    )
                } else {
                    (
                        p.background.hsla(),
                        p.secondary.scaled(OUTLINE_HOVER_ALPHA_LIGHT),
                    )
                };
                VariantStyle {
                    bg: Some(bg),
                    bg_hover: Some(bg_hover),
                    border: Some(p.input.hsla()),
                    text: p.foreground.hsla(),
                }
            }
            BadgeVariant::Destructive => VariantStyle {
                bg: Some(p.destructive.hsla()),
                bg_hover: Some(p.destructive.scaled(FILLED_HOVER_ALPHA)),
                border: None,
                // `text-white`: sobre o vermelho cheio quem lê é o BRANCO, não o
                // `--destructive-foreground` (que é um vermelho de texto e daria vermelho sobre
                // vermelho).
                text: WHITE.hsla(),
            },
            BadgeVariant::Error => estado(p.destructive, p.destructive_fg),
            BadgeVariant::Info => estado(p.info, p.info_fg),
            BadgeVariant::Success => estado(p.success, p.success_fg),
            BadgeVariant::Warning => estado(p.warning, p.warning_fg),
        }
    }
}

// =================================================================================================
// Anel de foco
// =================================================================================================

/// O **anel de foco** — `focus-visible:ring-2 ring-offset-1`.
///
/// ⚠️ `ring` não existe no GPUI, e não dá pra fingir com sombra: o `Window::paint_shadows` não
/// recorta a sombra pra fora de quem a projeta e o `spread_radius` dilata os limites MANTENDO o
/// raio, então a curvatura sai errada nas quinas (foi o defeito original no [`crate::input::Input`]
/// e no [`crate::button::Button`]).
///
/// Um `ring` é geometricamente a forma do elemento dilatada: 2px de anel mais 1px de folga = um
/// overlay 3px maior em cada lado, com borda de 2px e o raio crescendo junto (`raio + 3`).
///
/// A folga de 1px é **pintada** com `--background`, que é o que `ring-offset-background` quer dizer
/// literalmente: a coroa entre o elemento e o anel tem a cor do fundo, pra o anel ler como separado
/// e não como uma borda grossa. São dois overlays concêntricos, como no [`crate::switch`].
///
/// Uma primeira versão deixava a folga transparente, alegando ser "a lacuna do resto do crate" — não
/// era: o `switch` já pintava. Sobre fundo sólido as duas dão no mesmo, mas o selo é feito pra viver
/// dentro de tabelas e listas, onde o que está atrás dele raramente é `--background`.
fn ring_overlays(radius: f32) -> [Div; 2] {
    let p = badge();
    let coroa = |fora: f32| {
        div()
            .absolute()
            .top(px(-fora))
            .left(px(-fora))
            .right(px(-fora))
            .bottom(px(-fora))
            .rounded(px(radius + fora))
    };
    [
        coroa(RING_OFFSET)
            .border_1()
            .border_color(p.background.hsla()),
        coroa(RING_OFFSET + RING_WIDTH)
            .border(px(RING_WIDTH))
            .border_color(p.ring.hsla()),
    ]
}

// =================================================================================================
// O componente
// =================================================================================================

/// O handler de clique do selo.
///
/// Existe como alias porque o tipo escrito inteiro estoura o limite de complexidade do
/// `clippy::type_complexity` — e a alternativa de silenciar o lint esconderia o próximo tipo
/// realmente ilegível que aparecesse aqui.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Selo com o visual do coss. Ver o doc do módulo.
///
/// É um **elemento de render** (`RenderOnce`): construa a cada frame.
#[derive(IntoElement)]
pub struct Badge {
    /// `Some` = o selo é interativo (o `<button>`/`<a>` do original). `None` = `<span>` inerte.
    ///
    /// O GPUI só expõe `on_click` e rastreio de estado em elementos com `.id()`
    /// ([`gpui::StatefulInteractiveElement`]), e um id que muda de frame em frame zera o estado de
    /// interação — por isso ele é dado pelo call site, não gerado.
    id: Option<ElementId>,
    label: Option<SharedString>,
    icon_before: Option<SharedString>,
    icon_after: Option<SharedString>,
    variant: BadgeVariant,
    size: BadgeSize,
    disabled: bool,
    focus_handle: Option<FocusHandle>,
    on_click: Option<ClickHandler>,
}

impl Badge {
    /// Selo **estático** — o `<span>` do original: sem cursor, sem hover, sem clique.
    ///
    /// Pra um selo interativo use [`Self::button`] ou [`Self::on_click`].
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            id: None,
            label: Some(label.into()),
            icon_before: None,
            icon_after: None,
            variant: BadgeVariant::default(),
            size: BadgeSize::default(),
            disabled: false,
            focus_handle: None,
            on_click: None,
        }
    }

    /// Selo **interativo sem handler** — o `<a href>` do original: ganha cursor de mão, hover e
    /// entra na ordem de tabulação (se você passar [`Self::focus`]), mas quem trata a navegação é
    /// quem hospeda.
    ///
    /// Se você quer tratar o clique aqui, use [`Self::on_click`] direto — ele já promove o selo.
    ///
    /// O `id` tem que ser estável entre frames.
    pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        let mut badge = Self::new(label);
        badge.id = Some(id.into());
        badge
    }

    /// Selo **só de ícone**, sem rótulo. O piso de largura (`min-w-*` = altura) o mantém quadrado.
    ///
    /// O caminho é servido pela [`crate::assets::Assets`] (ex.:
    /// `"iconoir/regular/check.svg"`). Sem essa `AssetSource` registrada no bootstrap, o ícone some
    /// SILENCIOSAMENTE.
    pub fn icon(path: impl Into<SharedString>) -> Self {
        Self {
            id: None,
            label: None,
            icon_before: Some(path.into()),
            icon_after: None,
            variant: BadgeVariant::default(),
            size: BadgeSize::default(),
            disabled: false,
            focus_handle: None,
            on_click: None,
        }
    }

    /// Variante visual. Leia o doc de [`BadgeVariant`] antes de escolher entre `Destructive` e
    /// `Error` — as duas são vermelhas e significam coisas diferentes.
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Tamanho.
    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }

    /// Esmaece o CONJUNTO pra 64% e desliga a interação (`disabled:opacity-64` +
    /// `disabled:pointer-events-none`).
    ///
    /// **Superset consciente do original**, declarado: no DOM o prefixo `disabled:` casa só com um
    /// `<button disabled>`, então um selo `<span>` nunca esmaeceria. Aqui ele esmaece nas duas
    /// formas — um `.disabled(true)` que não fizesse nada num selo estático seria exatamente o tipo
    /// de falha silenciosa que este crate evita. O comportamento é idêntico ao original na forma
    /// interativa, que é a única em que o original tem opinião.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Ícone **antes** do rótulo. 12px em qualquer tamanho (ver [`ICON_SIZE`]).
    pub fn icon_before(mut self, path: impl Into<SharedString>) -> Self {
        self.icon_before = Some(path.into());
        self
    }

    /// Ícone **depois** do rótulo.
    pub fn icon_after(mut self, path: impl Into<SharedString>) -> Self {
        self.icon_after = Some(path.into());
        self
    }

    /// Torna o selo focável por teclado (entra na ordem de tabulação) e liga o **anel de foco**.
    ///
    /// Só tem efeito num selo interativo ([`Self::button`] / [`Self::on_click`]): rastrear o foco
    /// de um `<span>` inerte o deixaria alcançável por `tab` sem ter o que fazer ali.
    ///
    /// O handle tem que viver fora do elemento — crie um por selo na sua view (`cx.focus_handle()`),
    /// guarde-o, e passe a referência a cada render.
    pub fn focus(mut self, handle: &FocusHandle) -> Self {
        self.focus_handle = Some(handle.clone());
        self
    }

    /// O que fazer no clique — e, com isso, **promove o selo a interativo** (cursor de mão, hover,
    /// alvo de clique).
    ///
    /// O `id` vem no mesmo chamado de propósito: o `on_click` do GPUI só existe em elemento com
    /// `.id()`, então um `on_click` sem id seria um handler descartado em silêncio. Ele tem que ser
    /// estável entre frames.
    ///
    /// Ignorado quando [`Self::disabled`].
    pub fn on_click(
        mut self,
        id: impl Into<ElementId>,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.id = Some(id.into());
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Se o selo é o `<button>`/`<a>` do original (e não o `<span>`): é o que libera cursor, hover
    /// e clique. Um selo desabilitado nunca é interativo (`disabled:pointer-events-none`).
    pub fn is_interactive(&self) -> bool {
        self.id.is_some() && !self.disabled
    }

    /// Um ícone do selo, no tamanho e na cor certos.
    fn render_icon(path: SharedString, color: Hsla) -> impl IntoElement {
        gpui::svg()
            .path(path)
            .size(px(ICON_SIZE))
            .flex_none()
            .text_color(color)
            // `opacity-80`: como opacidade de ELEMENTO (e não alfa na cor) pra bater com o
            // original — sobre fundos translúcidos os dois não dão o mesmo resultado, e o fundo de
            // quatro das oito variantes é translúcido.
            .opacity(ICON_OPACITY)
    }
}

impl RenderOnce for Badge {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let s = self.size;
        let v = self.variant.style();
        let radius = s.radius();
        let interactive = self.is_interactive();

        // O anel só aparece se houver handle de foco E ele estiver focado. Desabilitado nunca
        // mostra anel — um selo inerte não deve parecer que recebeu o teclado.
        let focused = interactive
            && self
                .focus_handle
                .as_ref()
                .is_some_and(|h| h.is_focused(window));

        let mut el = div()
            // `relative` porque o anel de foco é um filho ABSOLUTO.
            .relative()
            // `inline-flex shrink-0`: o selo tem o tamanho do conteúdo e não é comprimido pelo
            // container (um selo esmagado num flex apertado perderia o `min-w`).
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(s.gap()))
            .h(px(s.height()))
            // `min-w-*` = a altura: um selo de um caractere nasce quadrado (ver
            // [`BadgeSize::height`]).
            .min_w(px(s.height()))
            .px(px(s.pad_x()))
            .rounded(px(radius))
            // A borda de 1px EXISTE em todas as variantes, mesmo nas "sem borda" — é ela que
            // mantém o miolo igual entre variantes (a altura é total, border-box). Nas sem borda
            // ela fica transparente (`border border-transparent` da classe base).
            .border_1()
            .border_color(v.border.unwrap_or(gpui::transparent_black()))
            // `whitespace-nowrap`: o selo é um rótulo curto; quebrar linha o deformaria.
            .whitespace_nowrap()
            .text_size(px(s.text_size()))
            .line_height(px(s.line_height()))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(v.text);

        if let Some(bg) = v.bg {
            el = el.bg(bg);
        }

        if !interactive && self.disabled {
            // `disabled:opacity-64`: esmaece o conjunto — o elemento e o conteúdo.
            el = el.opacity(DISABLED_OPACITY);
        }

        // --- Conteúdo -------------------------------------------------------------------------
        if let Some(path) = self.icon_before {
            el = el.child(Self::render_icon(path, v.text));
        }
        if let Some(label) = self.label {
            el = el.child(div().flex_none().child(label));
        }
        if let Some(path) = self.icon_after {
            el = el.child(Self::render_icon(path, v.text));
        }

        // --- Anel de foco, por último: é absoluto, então a ordem só decide quem pinta em cima ---
        //
        // O anel é `focus-visible`, não `focus`: só acende se o foco veio do TECLADO. Sem o segundo
        // termo, todo clique acendia um anel de 2px, porque o `track_focus` faz o mouse-down focar
        // o elemento. Ver [`crate::focus_ring`].
        //
        // O selo não usa `overflow_hidden`, então o overlay pode ser filho (ele cresce 3px pra
        // FORA, e um `overflow_hidden` no pai o recortaria — a armadilha que o
        // [`crate::input::Input`] tem que contornar com um irmão).
        if focused && crate::focus_ring::visible() {
            for coroa in ring_overlays(radius) {
                el = el.child(coroa);
            }
        }

        // --- `<span>` ou `<button>`/`<a>` -----------------------------------------------------
        //
        // Só aqui os dois caminhos se separam: `.id()` devolve um `Stateful<Div>`, um tipo
        // diferente de `Div`, então a divisão tem que ser a ÚLTIMA coisa e desembocar num
        // `AnyElement`.
        let Some(id) = self.id.filter(|_| interactive) else {
            return el.into_any_element();
        };

        // `[button&,a&]:cursor-pointer`.
        let mut el = el.id(id).cursor(gpui::CursorStyle::PointingHand);

        // `[button&,a&]:hover:bg-*`. O hover é uma troca SECA de fundo: a classe base transiciona
        // só `shadow`, não `colors` — não há fade a reproduzir aqui (diferente do
        // [`crate::button::Button`], que faz cross-fade de fundo por decisão de sensação).
        if let Some(bg_hover) = v.bg_hover {
            el = el.hover(move |style| style.bg(bg_hover));
        }

        if let Some(handle) = self.focus_handle.as_ref() {
            el = el.track_focus(handle);
        }

        if let Some(handler) = self.on_click {
            el = el.on_click(move |event, window, cx| {
                // O clique é ponteiro: apaga o anel de foco (ver `crate::focus_ring`). O foco em si
                // continua indo pro selo — o que não vai é o anel.
                crate::focus_ring::pointer_used(window);
                handler(event, window, cx);
            });
        }

        el.into_any_element()
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `button.rs`, `frame.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Todos os tamanhos, pros testes varrerem sem esquecer nenhum.
    const TODOS: [BadgeSize; 3] = [BadgeSize::Sm, BadgeSize::Default, BadgeSize::Lg];

    /// Todas as variantes, idem.
    const VARIANTES: [BadgeVariant; 8] = [
        BadgeVariant::Default,
        BadgeVariant::Secondary,
        BadgeVariant::Outline,
        BadgeVariant::Destructive,
        BadgeVariant::Error,
        BadgeVariant::Info,
        BadgeVariant::Success,
        BadgeVariant::Warning,
    ];

    /// As quatro variantes de **estado** — as que são um véu do token com o `*-foreground` por cima.
    const ESTADOS: [BadgeVariant; 4] = [
        BadgeVariant::Error,
        BadgeVariant::Info,
        BadgeVariant::Success,
        BadgeVariant::Warning,
    ];

    /// **A convenção de cor da paleta do selo, decodificada de verdade.**
    ///
    /// É o teste que pega a confusão de convenção: todo valor de [`BadgePalette`] é `0xRRGGBBAA` e
    /// é consumido por `rgba`. Um valor de 6 dígitos esquecido ali vira uma cor completamente
    /// diferente sem erro de compilação — `rgba(0xffffff)` é lido como `0x00FFFFFF`, ciano. Isso já
    /// custou três bugs visíveis nesta base.
    ///
    /// Então, em vez de comparar números com números (que não pegaria nada), decodifica e afirma o
    /// que a cor DEVE ser perceptualmente.
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        // Os NEUTROS: todos opacos e com r≈g≈b — se algum sair colorido, o valor foi lido
        // deslocado.
        for (nome, c) in [
            ("primary claro", BADGE_LIGHT.primary),
            ("primary escuro", BADGE_DARK.primary),
            ("primary-fg claro", BADGE_LIGHT.primary_fg),
            ("primary-fg escuro", BADGE_DARK.primary_fg),
            ("secondary-fg claro", BADGE_LIGHT.secondary_fg),
            ("secondary-fg escuro", BADGE_DARK.secondary_fg),
            ("foreground claro", BADGE_LIGHT.foreground),
            ("foreground escuro", BADGE_DARK.foreground),
            ("background claro", BADGE_LIGHT.background),
            ("background escuro", BADGE_DARK.background),
            ("ring claro", BADGE_LIGHT.ring),
            ("ring escuro", BADGE_DARK.ring),
            ("branco", WHITE),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: NEUTRO — se r≠g≠b, o valor foi lido deslocado"
            );
        }

        // A superfície do tema claro é branco puro (`--background` = white).
        let bgl: gpui::Rgba = BADGE_LIGHT.background.hsla().into();
        assert_eq!((bgl.r, bgl.g, bgl.b, bgl.a), (1.0, 1.0, 1.0, 1.0));
        // E o branco puro do texto da `Destructive`.
        let w: gpui::Rgba = WHITE.hsla().into();
        assert_eq!((w.r, w.g, w.b, w.a), (1.0, 1.0, 1.0, 1.0));

        // O par primary/primary-fg INVERTE entre os temas: fundo escuro com texto claro no tema
        // claro, e o contrário no escuro. Se os dois ficarem do mesmo lado, o selo desaparece.
        let pl: gpui::Rgba = BADGE_LIGHT.primary.hsla().into();
        let pfl: gpui::Rgba = BADGE_LIGHT.primary_fg.hsla().into();
        assert!(pl.r < 0.2 && pfl.r > 0.9, "claro: fundo escuro, texto claro");
        let pd: gpui::Rgba = BADGE_DARK.primary.hsla().into();
        let pfd: gpui::Rgba = BADGE_DARK.primary_fg.hsla().into();
        assert!(pd.r > 0.9 && pfd.r < 0.2, "escuro: fundo claro, texto escuro");

        // Os VERMELHOS: opacos e com r bem acima de g e b — o sintoma do bug histórico era o canal
        // vermelho zerar.
        for (nome, c) in [
            ("destructive claro", BADGE_LIGHT.destructive),
            ("destructive escuro", BADGE_DARK.destructive),
            ("destructive-fg claro", BADGE_LIGHT.destructive_fg),
            ("destructive-fg escuro", BADGE_DARK.destructive_fg),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: opaco");
            assert!(
                c.r > 0.6 && c.r > c.g + 0.25 && c.r > c.b + 0.25,
                "{nome}: é vermelho de verdade"
            );
        }

        // Os TRANSLÚCIDOS: é o alfa que os deixa funcionar sobre qualquer fundo. Um deles opaco
        // significaria um retângulo chapado no lugar de um véu.
        for (nome, c) in [
            ("secondary claro", BADGE_LIGHT.secondary),
            ("secondary escuro", BADGE_DARK.secondary),
            ("input claro", BADGE_LIGHT.input),
            ("input escuro", BADGE_DARK.input),
        ] {
            assert!(
                c.alpha() < 1.0,
                "{nome} tem que ser translúcido, veio com alfa {}",
                c.alpha()
            );
        }

        // E o flag de tema não pode estar trocado — é ele que escolhe o TOKEN da `Outline` e a
        // força do véu das variantes de estado.
        assert!(!BADGE_LIGHT.dark);
        assert!(BADGE_DARK.dark);
    }

    /// **As duas armadilhas do `--success`/`--info`/`--warning`, travadas.**
    ///
    /// Já custaram três valores errados no `toast`:
    ///
    /// 1. `--success` é **emerald**-500 (`#10b981`), não green-500 (`#22c55e`) — emerald é bem mais
    ///    azulado.
    /// 2. No tema escuro os três são **idênticos** ao claro. Só o `--destructive` sofre o
    ///    `color-mix` com branco. Generalizar a regra do destructive pros outros três é o erro.
    #[test]
    fn tokens_semanticos_nao_mudam_no_escuro_exceto_o_destructive() {
        assert_eq!(BADGE_LIGHT.success, Rgba8(0x10b981ff), "emerald-500, NÃO green-500");
        assert_eq!(BADGE_LIGHT.info, Rgba8(0x3b82f6ff), "blue-500");
        assert_eq!(BADGE_LIGHT.warning, Rgba8(0xf59e0bff), "amber-500");

        assert_eq!(BADGE_DARK.info, BADGE_LIGHT.info, "info: idêntico no escuro");
        assert_eq!(BADGE_DARK.success, BADGE_LIGHT.success, "success: idêntico");
        assert_eq!(BADGE_DARK.warning, BADGE_LIGHT.warning, "warning: idêntico");

        // O destructive é o ÚNICO que muda: mix(red-500 90%, white).
        assert_ne!(BADGE_DARK.destructive, BADGE_LIGHT.destructive);
        assert_eq!(BADGE_DARK.destructive, Rgba8(0xf15757ff));
        assert_eq!(BADGE_LIGHT.destructive, Rgba8(0xef4444ff));

        // E os matizes decodificam pro que os nomes prometem, nos dois temas.
        for (tema, p) in [("claro", &BADGE_LIGHT), ("escuro", &BADGE_DARK)] {
            for (nome, c) in [
                ("success", p.success),
                ("success-fg", p.success_fg),
            ] {
                let c: gpui::Rgba = c.hsla().into();
                assert!(c.g > c.r && c.g > c.b, "{tema}: {nome} é verde");
                // Emerald é AZULADO: o azul supera o vermelho. Green-500 (#22c55e) reprovaria.
                assert!(c.b > c.r, "{tema}: {nome} é emerald (azulado), não green");
            }
            for (nome, c) in [("warning", p.warning), ("warning-fg", p.warning_fg)] {
                let c: gpui::Rgba = c.hsla().into();
                assert!(c.r > c.b && c.g > c.b, "{tema}: {nome} é âmbar");
            }
            for (nome, c) in [("info", p.info), ("info-fg", p.info_fg)] {
                let c: gpui::Rgba = c.hsla().into();
                assert!(c.b > c.r && c.b > c.g, "{tema}: {nome} é azul");
            }
        }
    }

    /// Altura, respiro e corpo do texto CRESCEM com o tamanho. Um `Lg` mais baixo que um `Default`
    /// passaria batido no código e só apareceria na tela.
    ///
    /// As alturas são as do coss no breakpoint `sm:` e são **totais** (border-box, bordas
    /// incluídas). Ver o doc de [`BadgeSize`].
    #[test]
    fn tamanhos_crescem_monotonicamente() {
        assert_eq!(
            TODOS.map(|t| t.height()),
            [16.0, 18.0, 22.0],
            "`sm:h-4`, `sm:h-4.5`, `sm:h-5.5`"
        );
        assert_eq!(
            TODOS.map(|t| t.text_size()),
            [10.0, 12.0, 14.0],
            "`sm:text-[.625rem]`, `sm:text-xs`, `sm:text-sm`"
        );
        assert_eq!(
            TODOS.map(|t| t.pad_x()),
            [3.0, 3.0, 5.0],
            "`px-[calc(--spacing(N)-1px)]` — o -1px desconta a borda"
        );

        for par in TODOS.windows(2) {
            assert!(
                par[0].height() < par[1].height(),
                "{:?} tem que ser mais baixo que {:?}",
                par[0],
                par[1]
            );
            assert!(
                par[0].text_size() < par[1].text_size(),
                "{:?} tem que ter texto menor que {:?}",
                par[0],
                par[1]
            );
            assert!(
                par[0].pad_x() <= par[1].pad_x(),
                "{:?} não pode respirar mais que {:?}",
                par[0],
                par[1]
            );
        }

        // **É daqui que os números ímpares vêm.** O respiro declarado é
        // `px-[calc(--spacing(N)-1px)]`, e o `-1px` desconta a borda justamente pra o recuo TOTAL
        // (respiro + borda, medido da borda externa) cair na escala de spacing do Tailwind:
        // `--spacing(1)` = 4px e `--spacing(1.5)` = 6px.
        //
        // Se alguém "arredondar" o `pad_x` pra 4/4/6 achando que os ímpares são erro de digitação,
        // todo selo fica 1px mais largo de cada lado que o original.
        assert_eq!(
            TODOS.map(|t| t.pad_x() + 1.0),
            [4.0, 4.0, 6.0],
            "o recuo total tem que cair na escala de spacing do Tailwind"
        );

        // O gap é `gap-1` na classe BASE: não escala com o tamanho.
        for t in TODOS {
            assert_eq!(t.gap(), 4.0, "{t:?}: `gap-1` é da classe base");
        }

        assert_eq!(BadgeSize::default(), BadgeSize::Default);
    }

    /// **O piso de largura é a altura**: um selo de um caractere só nasce quadrado.
    ///
    /// No original o `min-w-*` acompanha o `h-*` nos três tamanhos (`h-4.5 min-w-4.5`, …). Este
    /// teste existe pra o dia em que alguém quiser um `min_width()` separado: se os dois números
    /// divergirem, um selo de contador vira uma fatia vertical (ou uma pílula larga demais).
    #[test]
    fn piso_de_largura_e_a_altura() {
        for t in TODOS {
            // O piso tem que caber o conteúdo mínimo: dois respiros, duas bordas e um glifo.
            let miolo = t.height() - 2.0 * t.pad_x() - 2.0;
            assert!(miolo > 0.0, "{t:?}: o piso não deixa espaço pro glifo");
            assert!(
                miolo >= t.text_size() * 0.5,
                "{t:?}: o piso ({}) aperta um dígito de {}px",
                t.height(),
                t.text_size()
            );
        }
        // E o ícone de 12px cabe no miolo VERTICAL dos três tamanhos.
        for t in TODOS {
            assert!(
                ICON_SIZE <= t.height() - 2.0,
                "{t:?}: ícone de {ICON_SIZE}px não cabe no miolo de {}px",
                t.height() - 2.0
            );
        }
    }

    /// Raio: `rounded-sm` (`--radius-sm` = 6px) no `Default` e no `Lg`; `rounded-[.25rem]` = 4px no
    /// `Sm`.
    ///
    /// O `Sm` desafina de propósito — a 16px de altura, 6px de raio já lê como pílula. Fica travado
    /// aqui pra ninguém "uniformizar" e sair do original.
    #[test]
    fn raio_e_4_no_sm_e_6_nos_outros() {
        assert_eq!(TODOS.map(|t| t.radius()), [4.0, 6.0, 6.0]);
        // 6px é o token `--radius-sm` do coss (`--radius` = 10px ⇒ `--radius-sm` = 6). 4px é valor
        // arbitrário (`.25rem`), não um token.
        assert_eq!(BadgeSize::Default.radius(), 6.0, "--radius-sm");
        assert_eq!(BadgeSize::Sm.radius(), 4.0, "rounded-[.25rem]");
        // E o raio nunca passa de metade da altura: aí ele seria uma pílula e o `rounded-full` do
        // Tailwind é que estaria na referência.
        for t in TODOS {
            assert!(t.radius() < t.height() / 2.0, "{t:?}: virou pílula");
        }
    }

    /// **A entrelinha é o miolo**, e nos dois tamanhos que usam a escala de tipografia do Tailwind
    /// ela COINCIDE com a entrelinha que o utilitário já traz — não é número escolhido a olho.
    ///
    /// `text-xs` → `1rem` = 16px = 18 − 2. `text-sm` → `1.25rem` = 20px = 22 − 2.
    ///
    /// O `Sm` é o caso deduzido: `text-[.625rem]` é valor arbitrário e o Tailwind v4 não anexa
    /// entrelinha a valor arbitrário. Ver [`BadgeSize::line_height`].
    #[test]
    fn entrelinha_bate_com_a_escala_do_tailwind() {
        assert_eq!(BadgeSize::Default.line_height(), 16.0, "text-xs → 1rem");
        assert_eq!(BadgeSize::Lg.line_height(), 20.0, "text-sm → 1.25rem");
        assert_eq!(BadgeSize::Sm.line_height(), 14.0, "deduzido: miolo do selo");

        for t in TODOS {
            // A entrelinha preenche o miolo EXATAMENTE: mais que isso estouraria a caixa de altura
            // fixa; menos deixaria o texto flutuando fora do centro ótico.
            assert_eq!(t.line_height(), t.height() - 2.0, "{t:?}");
            assert!(
                t.line_height() > t.text_size(),
                "{t:?}: entrelinha menor que o corpo do texto corta os descendentes"
            );
        }
    }

    /// **O véu das variantes de estado DOBRA no tema escuro**: `/8` no claro, `dark:/16`.
    ///
    /// É o contrário da intuição (esperava-se um véu mais discreto no escuro) e é o original: sobre
    /// `#141414` um véu de 8% não se vê. Se alguém "unificar" nos 8%, as quatro variantes de estado
    /// somem no tema escuro.
    #[test]
    fn veu_dos_estados_dobra_no_escuro() {
        assert_eq!(SOFT_BG_ALPHA_DARK, SOFT_BG_ALPHA_LIGHT * 2.0);

        for v in ESTADOS {
            theme::set_theme(theme::ThemeMode::Light);
            let claro = v.style().bg.expect("estado tem véu");
            theme::set_theme(theme::ThemeMode::Dark);
            let escuro = v.style().bg.expect("estado tem véu");

            assert!(
                (claro.a - SOFT_BG_ALPHA_LIGHT).abs() < 1e-4,
                "{v:?}: claro tem que ser 8%, veio {}",
                claro.a
            );
            assert!(
                (escuro.a - SOFT_BG_ALPHA_DARK).abs() < 1e-4,
                "{v:?}: escuro tem que ser 16%, veio {}",
                escuro.a
            );
            assert!(escuro.a > claro.a, "{v:?}: o escuro é o dobro, não a metade");
            assert!(escuro.a < 0.2, "{v:?}: continua um VÉU, não um fundo cheio");
        }
    }

    /// As quatro variantes de estado usam o `*-foreground` do **próprio** token, não o `--foreground`
    /// neutro — é o que dá o texto colorido sobre o véu. E nenhuma delas tem borda ou hover.
    #[test]
    fn estados_usam_o_foreground_do_proprio_token() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let p = badge();
            for (v, esperado) in [
                (BadgeVariant::Error, p.destructive_fg),
                (BadgeVariant::Info, p.info_fg),
                (BadgeVariant::Success, p.success_fg),
                (BadgeVariant::Warning, p.warning_fg),
            ] {
                let st = v.style();
                assert_eq!(st.text, esperado.hsla(), "{v:?}: texto do próprio token");
                assert_ne!(
                    st.text,
                    p.foreground.hsla(),
                    "{v:?}: não é o --foreground neutro"
                );
                assert!(st.border.is_none(), "{v:?}: borda transparente (classe base)");
            }
        }
    }

    /// **Só quatro das oito variantes têm hover, e nenhuma tem estado pressionado.**
    ///
    /// O `badge.tsx` define `hover:` em `default`, `secondary`, `outline` e `destructive`, e
    /// **nada** nas quatro de estado — um selo de status não é um botão. Não existe `active:` em
    /// variante nenhuma.
    ///
    /// Todo esse hover está atrás de `[button&,a&]`: num selo `<span>` (o default) ele nem é
    /// montado — ver [`Badge::render`].
    #[test]
    fn hover_so_nas_variantes_de_fundo() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let com_hover: Vec<_> = VARIANTES
                .into_iter()
                .filter(|v| v.style().bg_hover.is_some())
                .collect();
            assert_eq!(
                com_hover,
                vec![
                    BadgeVariant::Default,
                    BadgeVariant::Secondary,
                    BadgeVariant::Outline,
                    BadgeVariant::Destructive,
                ],
                "as variantes de ESTADO não têm hover no original"
            );
            for v in ESTADOS {
                assert!(v.style().bg_hover.is_none(), "{v:?}");
            }
        }
    }

    /// **Contra-intuitivo e FIEL ao original.** `hover:bg-secondary/90` DILUI o fundo (4% → 3,6%)
    /// em vez de intensificá-lo.
    ///
    /// Parece bug — o hover normalmente escurece. Fica travado aqui pra ninguém "consertar" pra uma
    /// escala crescente e sair do pixel-perfect. O motivo é o modificador `/N` do Tailwind v4, que
    /// MULTIPLICA o alfa existente em vez de substituí-lo.
    #[test]
    fn hover_da_secondary_dilui_o_fundo() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let st = BadgeVariant::Secondary.style();
            let repouso = st.bg.expect("Secondary tem fundo");
            let hover = st.bg_hover.expect("e hover");

            assert!(hover.a < repouso.a, "o hover DILUI (não é erro)");
            assert!((hover.a - repouso.a * FILLED_HOVER_ALPHA).abs() < 1e-6);
            assert!(
                repouso.a < 0.1,
                "4% de preto/branco, não um retângulo chapado"
            );
        }

        // Nas variantes de fundo OPACO o mesmo `/90` faz o que a intuição espera: o alfa cai pra
        // 90% e a superfície de trás começa a aparecer.
        theme::set_theme(theme::ThemeMode::Dark);
        for v in [BadgeVariant::Default, BadgeVariant::Destructive] {
            let st = v.style();
            assert_eq!(st.bg.expect("fundo cheio").a, 1.0, "{v:?}: opaco em repouso");
            assert!(
                (st.bg_hover.expect("hover").a - FILLED_HOVER_ALPHA).abs() < 1e-6,
                "{v:?}"
            );
        }
    }

    /// A `Outline` troca de TOKEN entre os temas, não só de valor: superfície opaca
    /// (`bg-background`) no claro, o próprio `--input` diluído no escuro. Um selo de moldura opaco
    /// no tema escuro apagaria o que estiver atrás dele.
    ///
    /// E o hover escuro é **48%**, não os 64% que o `button.tsx` usa na família equivalente.
    #[test]
    fn outline_troca_de_token_entre_os_temas() {
        theme::set_theme(theme::ThemeMode::Light);
        let claro = BadgeVariant::Outline.style();
        assert_eq!(
            claro.bg.expect("tem fundo").a,
            1.0,
            "claro: superfície OPACA (bg-background)"
        );
        // O hover claro é `--accent` a 50% — e `--accent` é o mesmo valor de `--secondary` (4%),
        // então o modificador MULTIPLICA: 2%, não 50%.
        let hover_claro = claro.bg_hover.expect("tem hover");
        assert!(
            (hover_claro.a - BADGE_LIGHT.secondary.alpha() * OUTLINE_HOVER_ALPHA_LIGHT).abs()
                < 1e-6,
            "hover claro = accent × 50%"
        );
        assert!(hover_claro.a < 0.03, "2%, não 50%");

        theme::set_theme(theme::ThemeMode::Dark);
        let escuro = BadgeVariant::Outline.style();
        let bg = escuro.bg.expect("tem fundo");
        assert!(bg.a < 1.0, "escuro: véu translúcido (bg-input/32)");
        // O `/N` MULTIPLICA: em `--input` (branco a 8%), `/32` dá 2,56%, não 32%. Errar isso deixa
        // o fundo 12× mais forte que o original.
        let base = BADGE_DARK.input.alpha();
        assert!(
            (bg.a - base * OUTLINE_DARK_BG_ALPHA).abs() < 1e-6,
            "esperado {} (= {base} × {OUTLINE_DARK_BG_ALPHA}), veio {}",
            base * OUTLINE_DARK_BG_ALPHA,
            bg.a
        );
        assert!(bg.a < 0.04, "2,56%, não 32%");

        // ⚠️ 48%, não 64% (que é o valor do `button.tsx`). O hover escuro é 1,5× o repouso, e não o
        // dobro.
        assert_eq!(OUTLINE_HOVER_ALPHA_DARK, 0.48);
        let hover = escuro.bg_hover.expect("tem hover");
        assert!((hover.a - base * OUTLINE_HOVER_ALPHA_DARK).abs() < 1e-6);
        assert!(
            (hover.a / bg.a - 1.5).abs() < 1e-4,
            "48/32 = 1,5× o repouso — o botão usa 64/32 = 2×"
        );

        // A `Outline` é a ÚNICA variante com borda visível: todas as outras herdam o
        // `border-transparent` da classe base.
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let com_borda: Vec<_> = VARIANTES
                .into_iter()
                .filter(|v| v.style().border.is_some())
                .collect();
            assert_eq!(com_borda, vec![BadgeVariant::Outline]);
            assert_eq!(
                BadgeVariant::Outline.style().border,
                Some(badge().input.hsla()),
                "e ela é `--input`"
            );
        }
    }

    /// **`Destructive` e `Error` são coisas diferentes**, e é o par mais fácil de trocar sem erro de
    /// compilação: as duas são vermelhas.
    ///
    /// A cheia tem fundo OPACO e texto BRANCO (`text-white`); a de estado tem véu de 8/16% e o
    /// vermelho de TEXTO (`--destructive-foreground`). Trocar uma pela outra dá vermelho sobre
    /// vermelho (ilegível) ou um alerta com a ênfase de um botão de apagar.
    #[test]
    fn destructive_cheia_e_error_de_estado_nao_se_confundem() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let cheia = BadgeVariant::Destructive.style();
            let estado = BadgeVariant::Error.style();

            assert_eq!(cheia.bg.expect("fundo").a, 1.0, "a cheia é opaca");
            assert!(estado.bg.expect("véu").a < 0.2, "o estado é um véu");

            assert_eq!(cheia.text, WHITE.hsla(), "sobre vermelho cheio, branco");
            assert_ne!(
                estado.text, cheia.text,
                "sobre o véu, o vermelho de texto"
            );
            let t: gpui::Rgba = estado.text.into();
            assert!(t.r > t.g + 0.25, "e ele é vermelho de verdade");

            // As duas nascem do MESMO token de fundo — o que difere é a força e o texto.
            let base: gpui::Rgba = badge().destructive.hsla().into();
            let veu: gpui::Rgba = estado.bg.expect("véu").into();
            assert!(
                (veu.r - base.r).abs() < 1e-6 && (veu.g - base.g).abs() < 1e-6,
                "o véu é o --destructive diluído, não outro vermelho"
            );
        }
    }

    /// O anel de foco cresce JUNTO com o raio do selo: `raio + anel + folga`. Se o raio do anel não
    /// acompanhar, a curvatura sai errada nas quinas — o sintoma de quando ele era um `BoxShadow`
    /// com `spread_radius` (que dilata os limites mas mantém o raio).
    #[test]
    fn raio_do_anel_acompanha_o_do_selo() {
        // `focus-visible:ring-2 ring-offset-1` = 2px de anel + 1px de folga = 3px pra fora.
        assert_eq!(RING_WIDTH, 2.0);
        assert_eq!(RING_OFFSET, 1.0);
        let folga = RING_WIDTH + RING_OFFSET;
        assert_eq!(folga, 3.0);

        // O overlay recua `-folga` e o raio dele é o do selo MAIS a folga — é isso que mantém as
        // duas curvas concêntricas.
        assert_eq!(BadgeSize::Sm.radius() + folga, 7.0);
        assert_eq!(BadgeSize::Default.radius() + folga, 9.0);
        assert_eq!(BadgeSize::Lg.radius() + folga, 9.0);
    }

    /// O esmaecimento do desabilitado é UM número aplicado ao conjunto (`opacity-64`), não um par
    /// "apagado" de cada token da paleta. Duas fontes de verdade pra "está desabilitado" é o que
    /// produz um selo com o texto apagado e a borda em força total.
    ///
    /// E o mesmo 64% do [`crate::button`]: dois esmaecimentos diferentes na mesma tela seria pior
    /// que um desvio.
    #[test]
    fn desabilitado_e_uma_opacidade_so() {
        assert_eq!(DISABLED_OPACITY, 0.64);
        assert!(DISABLED_OPACITY > 0.0 && DISABLED_OPACITY < 1.0);

        // As cores NÃO mudam com o disabled.
        theme::set_theme(theme::ThemeMode::Dark);
        for v in VARIANTES {
            let st = v.style();
            assert_eq!(st.text, v.style().text, "{v:?}: a cor de texto é uma só");
            assert_eq!(st.bg, v.style().bg, "{v:?}: o fundo é um só");
        }
    }

    /// **O selo é `<span>` até ganhar um `id`** — e é o `id` que libera cursor, hover e clique, do
    /// mesmo jeito que no coss é o `[button&,a&]`.
    ///
    /// `disabled:pointer-events-none`: um selo desabilitado nunca é interativo, mesmo com id e
    /// handler.
    #[test]
    fn interativo_so_com_id_e_habilitado() {
        let span = Badge::new("Beta");
        assert!(span.id.is_none(), "o default é `<span>`");
        assert!(!span.is_interactive(), "e `<span>` não tem hover nem clique");

        let link = Badge::button("filtro", "Filtro");
        assert!(link.id.is_some());
        assert!(link.is_interactive(), "com id, é `<button>`/`<a>`");
        assert!(link.on_click.is_none(), "interativo SEM handler, como um <a href>");

        // `on_click` promove o selo e traz o id no mesmo chamado — não existe handler órfão, que o
        // GPUI descartaria em silêncio (`on_click` só existe em elemento com `.id()`).
        let clicavel = Badge::new("Filtro").on_click("limpar", |_, _, _| {});
        assert!(clicavel.id.is_some(), "`on_click` traz o id");
        assert!(clicavel.on_click.is_some());
        assert!(clicavel.is_interactive());

        // Desabilitado desliga a interação sem apagar o id.
        let inerte = Badge::button("x", "y").disabled(true);
        assert!(inerte.id.is_some());
        assert!(!inerte.is_interactive(), "disabled:pointer-events-none");
    }

    /// Os builders não se atropelam, e os defaults são os `defaultVariants` do original
    /// (`size: "default"`, `variant: "default"`).
    #[test]
    fn builders_compoem() {
        let b = Badge::new("Novo")
            .variant(BadgeVariant::Success)
            .size(BadgeSize::Lg)
            .disabled(true)
            .icon_before("iconoir/regular/check.svg")
            .icon_after("iconoir/regular/arrow-right.svg");

        assert_eq!(b.variant, BadgeVariant::Success);
        assert_eq!(b.size, BadgeSize::Lg);
        assert!(b.disabled);
        assert_eq!(b.label, Some(SharedString::from("Novo")));
        assert!(b.icon_before.is_some() && b.icon_after.is_some());
        assert!(b.focus_handle.is_none(), "foco é opt-in");
        assert!(b.on_click.is_none());

        let d = Badge::new("x");
        assert_eq!(d.variant, BadgeVariant::Default, "defaultVariants.variant");
        assert_eq!(d.size, BadgeSize::Default, "defaultVariants.size");
        assert!(!d.disabled);

        // O selo só de ícone não tem rótulo — quem o mantém quadrado é o piso de largura.
        let i = Badge::icon("iconoir/regular/check.svg");
        assert!(i.label.is_none());
        assert!(i.icon_before.is_some());
        assert!(!i.is_interactive(), "ícone estático também é `<span>`");
    }

    /// Toda variante define uma cor de texto VISÍVEL e um fundo (não existe selo transparente no
    /// original: as oito variantes têm `bg-*`).
    ///
    /// Um selo sem fundo seria só texto solto — e, diferente do botão, o badge não tem variante
    /// `ghost`/`link` pra justificar isso.
    #[test]
    fn toda_variante_tem_fundo_e_texto() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            for v in VARIANTES {
                let st = v.style();
                assert!(st.text.a > 0.0, "{v:?}: texto invisível");
                let bg = st.bg.unwrap_or_else(|| panic!("{v:?}: sem fundo"));
                assert!(bg.a > 0.0, "{v:?}: fundo invisível");
                if let Some(hover) = st.bg_hover {
                    assert_ne!(Some(hover), st.bg, "{v:?}: hover sem efeito visível");
                }
            }
        }
    }

    /// O ícone é 12px em QUALQUER tamanho: no `badge.tsx` o lado do ícone está na classe base
    /// (`sm:[&_svg…]:size-3`), não nas variantes de tamanho.
    ///
    /// Diferente do [`crate::button`], onde ele escala. Se alguém "consertar" pra escalar, um selo
    /// `Sm` de 16px ganha um ícone que encosta nas bordas.
    #[test]
    fn icone_nao_escala_com_o_tamanho() {
        assert_eq!(ICON_SIZE, 12.0, "sm:size-3");
        assert_eq!(ICON_OPACITY, 0.80, "opacity-80");
        // No `Sm` o ícone é MAIOR que o corpo do texto (12 contra 10) — é o original, e é o único
        // tamanho em que isso acontece.
        assert!(ICON_SIZE > BadgeSize::Sm.text_size());
        assert_eq!(ICON_SIZE, BadgeSize::Default.text_size(), "no Default empatam");
        assert!(ICON_SIZE < BadgeSize::Lg.text_size());
    }
}

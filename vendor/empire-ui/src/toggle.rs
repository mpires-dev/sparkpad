//! `Toggle` — o botão de **dois estados**. Porte do `toggle.tsx` do coss.
//!
//! ```ignore
//! Toggle::icon("negrito", "iconoir/regular/bold.svg")
//!     .pressed(self.negrito)
//!     .variant(ToggleVariant::Outline)
//!     .on_toggle(cx.listener(|this, ligado, _w, cx| { this.negrito = *ligado; cx.notify() }))
//! ```
//!
//! # Toggle não é Switch, e não é Button
//!
//! Três controles parecidos, e a diferença é o que eles dizem:
//!
//! | | o que significa | onde vive |
//! |---|---|---|
//! | [`crate::Button`] | "faça isso agora" | qualquer lugar |
//! | `Toggle` | "isto está ligado" — e o ROSTO dele é o de um botão | barra de ferramentas |
//! | [`crate::Switch`] | "isto está ligado" — e o rosto é o de um interruptor | formulário |
//!
//! O `Toggle` é o negrito/itálico de um editor: mora numa barra, tem cara de botão, e o estado é a
//! aparência dele. O [`crate::Switch`] é a linha "Receber e-mails" de uma tela de preferências.
//!
//! # Controlado, sempre
//!
//! Diferente do [`crate::Switch`] e do [`crate::Checkbox`] (que são `Entity` e guardam o próprio
//! estado), o `Toggle` é [`RenderOnce`] e recebe o estado por [`Toggle::pressed`]. Não é
//! inconsistência: o estado de um toggle NUNCA é dele. Ou é do documento que ele edita (o negrito do
//! trecho selecionado, que muda quando o cursor anda) ou é do [`crate::toggle_group::ToggleGroup`],
//! que precisa apagar os irmãos quando um acende. Um `Toggle` com estado próprio seria uma segunda
//! fonte de verdade sobre a mesma coisa — e a que fica dessincronizada é sempre a de dentro.
//!
//! É o mesmo raciocínio do [`crate::tabs::TabsTab`], que também é dado puro dentro de um dono.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`pointer-coarse:after:min-h-11 min-w-11`**: o original põe um alvo invisível de 44px atrás do
//!   toggle **em ponteiro grosso** (dedo). `pointer-coarse` é uma consulta de MÍDIA, e numa janela de
//!   desktop com mouse ela nunca vale — o ramo é código morto aqui, e não uma omissão. Se um dia esta
//!   lib rodar em tablet, é este parágrafo que fica desatualizado.
//! - **`transition-shadow`**: o GPUI não interpola sombra. Na prática o original só transiciona a
//!   sombra (e o anel, que é sombra no Tailwind), então o que se perde é o suavizado do anel de foco.
//!   O fundo do original é INSTANTÂNEO (não há `transition-colors`), e aqui também — a diferença é
//!   deliberada em relação ao [`crate::Button`], que tem fade de fundo e onda de clique como
//!   superset da casa. Num toggle a mudança de estado já é o feedback; uma onda por cima disputaria
//!   atenção com ela.
//! - **`select-none`**: não há seleção de texto de rótulo de controle no GPUI.
//! - **`outline-none`**: o GPUI não desenha contorno de foco nativo — não há o que desligar.
//!
//! **Resolvido em número**
//!
//! - Tudo que é `sm:` (≥640px) vale, e a classe base não: uma janela de desktop está sempre acima do
//!   breakpoint. É por isso que o texto é 14 e não 16, e o ícone 16 e não 18. Mesma decisão do
//!   [`crate::Button`] e do [`crate::empty`].
//! - `[&_svg]:-mx-0.5` (o ícone puxa 2px de cada lado) entra como GEOMETRIA, não como margem
//!   negativa: `.mx(px(-2.0))` já colapsou layout nesta base (ver `crate::tabs`). Ver
//!   [`ToggleSize::pad_x_icon`].
//!
//! **Desvio consciente**
//!
//! - O fio de bisel do tema escuro é o DOBRO do original (branco ~11,8% em vez de 6%), como no
//!   [`crate::Button`], no [`crate::Input`] e no [`crate::card`]: a 6% ele é imperceptível no nosso
//!   fundo escuro. Ver [`TOGGLE_DARK`].

use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, ClickEvent, Div, ElementId, FocusHandle, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement as _, Styled, Window,
};

use crate::color::Rgba8;
use crate::group::Join;
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Convenção da casa: TODO valor é `0xRRGGBBAA`, com o byte de alfa, SEMPRE (ver [`crate::color`]).
// Misturar com os tokens de 6 dígitos de [`crate::theme`] desloca os canais e produz outra cor sem
// erro de compilação — já custou três bugs visíveis nesta base.

/// Tokens visuais do toggle, por tema.
#[derive(Clone, Copy, Debug)]
struct TogglePalette {
    /// `--foreground` — o texto e os ícones em repouso.
    foreground: Rgba8,
    /// `--accent` — o fundo de hover (as duas variantes o usam, é classe base).
    accent: Rgba8,
    /// `--accent-foreground` — o texto quando LIGADO (`data-pressed:text-accent-foreground`).
    accent_fg: Rgba8,
    /// `--input` — a borda da variante `Outline`, e a base do fundo de ligado.
    input: Rgba8,
    /// `--background` — o fundo da `Outline` no tema claro.
    ///
    /// ⚠️ O [`crate::Button`] usa `--popover` no mesmo lugar. Os dois resolvem pro MESMO branco no
    /// claro, mas são tokens diferentes na referência, e cada porte segue o seu — não unifique.
    background: Rgba8,
    /// `--ring` — o anel de foco.
    ring: Rgba8,
    /// Sombra externa da `Outline` (`shadow-xs/5`).
    shadow: Rgba8,
    /// Fio de bisel de 1px da `Outline`, em repouso.
    bevel: Rgba8,
    /// O mesmo fio enquanto o botão está PRESSIONADO pelo mouse. `None` = o bisel some.
    ///
    /// O original tem duas regras `dark:before:shadow-…`, e a diferença entre elas é um `not-active`:
    /// no escuro o filete não desaparece ao apertar, ele ESCURECE (6% → 2%). No claro não há regra
    /// equivalente, então lá ele mesmo desaparece — daí o [`Option`].
    bevel_active: Option<Rgba8>,
    /// Sentido do bisel: `+1` desce (claro), `-1` sobe (escuro). É o SINAL que escolhe o lado, e não
    /// um segundo campo que poderia divergir dele.
    bevel_dir: f32,
    /// Se este é o tema escuro. A `Outline` troca de TOKEN (não só de valor) entre os temas — fundo
    /// `--background` no claro, `--input`/32 no escuro — então a derivação precisa saber onde está.
    dark: bool,
}

/// Tema **claro**.
const TOGGLE_LIGHT: TogglePalette = TogglePalette {
    foreground: Rgba8(0x262626ff), // neutral-800
    accent: Rgba8(0x0000000a),     // black 4%
    accent_fg: Rgba8(0x262626ff),  // neutral-800
    input: Rgba8(0x0000001a),      // black 10%
    background: Rgba8(0xffffffff),
    ring: Rgba8(0xa3a3a3ff),  // neutral-400
    shadow: Rgba8(0x0000000d), // black 5% (o `/5` do `shadow-xs/5`)
    bevel: Rgba8(0x0000000a),  // black 4%
    bevel_active: None,        // o `not-active` do claro não tem par: o filete some
    bevel_dir: 1.0,
    dark: false,
};

/// Tema **escuro**.
const TOGGLE_DARK: TogglePalette = TogglePalette {
    foreground: Rgba8(0xf5f5f5ff), // neutral-100
    accent: Rgba8(0xffffff0a),     // white 4%
    accent_fg: Rgba8(0xf5f5f5ff),  // neutral-100
    input: Rgba8(0xffffff14),      // white 8%
    // `--background` escuro = mix(neutral-950 96%, white) = #141414. Só entra em conta pra provar
    // que o ramo claro não é usado aqui (a `Outline` escura usa `--input`/32) — ver `variant_style`.
    background: Rgba8(0x141414ff),
    ring: Rgba8(0x737373ff), // neutral-500
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE, o MESMO já vigente no `Input`, no `Card`, no `Frame`, no `Button` e no
    // `Empty`: a referência usa branco a 6% (alfa 15) e aqui é o DOBRO — alfa 30 ≈ 11,8% — porque a
    // 6% o filete é imperceptível no nosso fundo escuro. NÃO "corrija" pra 0x0f achando que é erro de
    // porte; se a intenção mudar, mude junto o teste `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    // O par apertado, dobrado pela mesma régua: 2% → ~4% (alfa 10).
    bevel_active: Some(Rgba8(0xffffff0a)),
    bevel_dir: -1.0,
    dark: true,
};

/// A paleta do toggle no tema corrente.
fn palette() -> &'static TogglePalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &TOGGLE_DARK,
        theme::ThemeMode::Light => &TOGGLE_LIGHT,
    }
}

/// O **sentido** do fio de bisel no tema corrente: `+1` desce (filete embaixo), `-1` sobe (filete
/// em cima). Ver [`TogglePalette::bevel_dir`].
///
/// Existe pro [`crate::toggle_group::ToggleGroup`] **vertical**: lá o filete só pode aparecer na
/// PONTA do grupo do lado pra onde o bisel aponta, e é este sinal que escolhe a ponta. Exportar o
/// sinal (e não um "é o primeiro ou o último?") é o que faz a regra acompanhar a paleta sozinha se o
/// sentido do bisel mudar.
pub(crate) fn bevel_dir() -> f32 {
    palette().bevel_dir
}

// --- Alfas (os modificadores `/N` do Tailwind) ----------------------------------------------------
//
// ⚠️ No Tailwind v4 o modificador `/N` é `color-mix(in oklab, <cor> N%, transparent)`: ele
// **multiplica** o alfa que a cor já tem, não o substitui. Em `--input` (branco a 8%), `/64` dá
// 5,12%, não 64%. É por isso que tudo aqui passa por [`Rgba8::scaled`].

/// `data-pressed:bg-input/64` — o fundo do estado LIGADO, nas duas variantes.
const PRESSED_BG_ALPHA: f32 = 0.64;

/// `dark:bg-input/32` — o fundo da `Outline` em repouso, no tema escuro.
const OUTLINE_DARK_BG_ALPHA: f32 = 0.32;

/// `dark:hover:bg-input/64` — o hover da `Outline` no tema escuro (que VENCE o `hover:bg-accent` da
/// classe base, por ser mais específico).
const OUTLINE_DARK_HOVER_ALPHA: f32 = 0.64;

/// `disabled:opacity-64` — o coss esmaece o CONJUNTO em vez de trocar cor por cor. Uma fonte de
/// verdade só: não existe um par "apagado" de cada token.
const DISABLED_OPACITY: f32 = 0.64;

/// `[&_svg:not([class*='opacity-'])]:opacity-80` — ícones pesam mais que texto no mesmo corpo, e o
/// original os alivia.
const ICON_OPACITY: f32 = 0.80;

// --- Geometria -----------------------------------------------------------------------------------

/// Raio — `rounded-lg` = `--radius-lg` = `--radius` = **10px**.
///
/// ⚠️ Diferente do [`crate::Button`], **não muda com o tamanho**: no original o `rounded-lg` está na
/// classe base do `cva`, e nenhuma variante de `size` o troca. Um toggle `Sm` de 28px tem os mesmos
/// 10px de raio que um `Lg` de 36 (o `ButtonSize::Xs` desafina de propósito; aqui não há esse caso).
const RADIUS: f32 = 10.0;

/// Espessura do anel de foco — `focus-visible:ring-2`.
const RING_WIDTH: f32 = 2.0;

/// Folga entre o toggle e o anel — `ring-offset-1`.
const RING_OFFSET: f32 = 1.0;

/// Corpo do texto — `sm:text-sm`.
const TEXT_SIZE: f32 = 14.0;

/// Entrelinha do texto — o PAR do `text-sm` no Tailwind: 14 de fonte, **20** de linha.
///
/// Sem fixar, o GPUI usa `relative(1.618_034)` e a linha sai com 22,7px. Num toggle de altura fixa
/// isso não muda a altura, mas desloca o texto meio pixel do centro ótico. Ver a armadilha nº 1 da
/// casa (documentada no [`crate::group`]).
const TEXT_LINE_HEIGHT: f32 = 20.0;

/// Vão entre ícone e rótulo — `gap-2`.
const GAP: f32 = 8.0;

/// Lado do ícone — `sm:size-4`.
const ICON_SIZE: f32 = 16.0;

/// O `[&_svg]:-mx-0.5` da referência: o ícone puxa 2px de cada lado.
const ICON_PULL: f32 = 2.0;

// =================================================================================================
// Tamanho
// =================================================================================================

/// Tamanho do toggle. Ver a nota sobre alturas TOTAIS no [`crate::ButtonSize`]: aqui é igual, o
/// `h-*` está no próprio elemento, que tem `border` e `box-sizing: border-box`.
///
/// Os valores são os ramos `sm:` do Tailwind — ver o doc do módulo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToggleSize {
    /// 28px de altura (e 28 de largura mínima).
    Sm,
    /// 32px — o padrão.
    #[default]
    Default,
    /// 36px.
    Lg,
}

impl ToggleSize {
    /// Todos os tamanhos, na ordem de tamanho. É a lista ÚNICA — quem varre tamanhos (teste,
    /// storybook) usa esta, pra um tamanho novo não passar batido.
    pub const ALL: [ToggleSize; 3] = [ToggleSize::Sm, ToggleSize::Default, ToggleSize::Lg];

    /// Altura TOTAL, bordas incluídas — `sm:h-7`/`sm:h-8`/`sm:h-9`.
    pub fn height(self) -> f32 {
        match self {
            ToggleSize::Sm => 28.0,
            ToggleSize::Default => 32.0,
            ToggleSize::Lg => 36.0,
        }
    }

    /// Largura MÍNIMA — `sm:min-w-7`/`min-w-8`/`min-w-9`, que é sempre igual à altura.
    ///
    /// É ela que faz um toggle só de ícone sair quadrado sem precisar de tamanhos `Icon*` como o
    /// [`crate::ButtonSize`] tem: o respiro é pequeno (5/7/9px), então o conteúdo de um ícone de 16px
    /// não alcança o mínimo e a caixa fica no piso.
    pub fn min_width(self) -> f32 {
        self.height()
    }

    /// Respiro horizontal — `px-[calc(--spacing(N)-1px)]`, com o `-1px` descontando a borda.
    ///
    /// `1.5`→6−1=5, `2`→8−1=7, `2.5`→10−1=9.
    pub fn pad_x(self) -> f32 {
        match self {
            ToggleSize::Sm => 5.0,
            ToggleSize::Default => 7.0,
            ToggleSize::Lg => 9.0,
        }
    }

    /// O respiro quando há **ícone na ponta**: o `-mx-0.5` do ícone come 2px dele.
    ///
    /// Reproduzir a margem negativa como geometria (e não como `.mx(px(-2.0))`) é regra da casa —
    /// margem negativa já colapsou o layout das abas. O piso em zero importa no `Sm`, onde 5−2=3
    /// ainda é positivo, mas um tamanho novo mais apertado não deve virar respiro negativo.
    pub fn pad_x_icon(self) -> f32 {
        (self.pad_x() - ICON_PULL).max(0.0)
    }
}

// =================================================================================================
// Variante
// =================================================================================================

/// Variante visual do toggle.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToggleVariant {
    /// Sem moldura: borda transparente, fundo só no hover e no ligado. É o toggle de barra de
    /// ferramentas.
    #[default]
    Default,
    /// Com moldura: borda `--input`, fundo de superfície, sombra e bisel. É o que forma botão
    /// segmentado no [`crate::toggle_group::ToggleGroup`].
    Outline,
}

/// As cores de um par (variante, estado) já **resolvidas** no tema corrente.
///
/// Em [`Hsla`] e não em [`Rgba8`] porque quase todo valor aqui passou por um modificador `/N`, que
/// multiplica o alfa — o resultado não é mais um byte de paleta. A paleta em si continua em `Rgba8`,
/// e é ela que os testes de decodificação cobrem.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ToggleStyle {
    /// Fundo em repouso. `None` = transparente.
    bg: Option<Hsla>,
    /// Fundo sob o ponteiro.
    bg_hover: Hsla,
    /// Cor da borda de 1px. `None` = transparente (a borda EXISTE mesmo assim — é ela que mantém a
    /// altura interna igual entre variantes, porque a altura é total).
    border: Option<Hsla>,
    /// Cor do texto e dos ícones.
    text: Hsla,
    /// Sombra externa `shadow-xs/5`. `None` = sem sombra.
    shadow: Option<Hsla>,
    /// Se esta combinação tem o fio de bisel.
    bevel: bool,
}

/// **As cores de uma variante num estado.** É a tabela do `cva` resolvida, e a única função deste
/// módulo que decide cor — daí ela ser livre e testável sem construir um [`Toggle`].
///
/// A precedência entre as regras do original, que é o que esta função encapsula:
///
/// 1. `data-pressed:bg-input/64` e `data-pressed:text-accent-foreground` valem pras DUAS variantes
///    (são classe base), mas a `Outline` escura sobrescreve o fundo com `dark:data-pressed:bg-input`
///    — 8% cheio em vez de 5,12%.
/// 2. `hover:bg-accent` também é classe base; a `Outline` escura sobrescreve com
///    `dark:hover:bg-input/64`, por ser mais específico.
/// 3. Ligado + hover: o original não tem regra de hover pra o estado ligado além do `hover:bg-accent`
///    da base, que PERDE do `data-pressed:bg-input/64` na ordem em que o Tailwind emite as camadas.
///    Então um toggle ligado não muda de fundo sob o ponteiro — e é isso que esta função devolve.
/// 4. Sombra e bisel morrem em `:disabled`, `:active` e `[data-pressed]`
///    (`[:disabled,:active,[data-pressed]]:shadow-none`).
fn variant_style(
    variant: ToggleVariant,
    pressed: bool,
    disabled: bool,
    p: &TogglePalette,
) -> ToggleStyle {
    // O fundo do estado ligado: `--input`/64, ou `--input` cheio na `Outline` escura.
    let ligado = if variant == ToggleVariant::Outline && p.dark {
        p.input.hsla()
    } else {
        p.input.scaled(PRESSED_BG_ALPHA)
    };
    // Sombra e bisel são da `Outline`, e só em repouso-não-ligado.
    let moldura = variant == ToggleVariant::Outline;
    let realce = moldura && !pressed && !disabled;

    ToggleStyle {
        bg: match (variant, pressed) {
            (_, true) => Some(ligado),
            (ToggleVariant::Default, false) => None,
            (ToggleVariant::Outline, false) => Some(if p.dark {
                p.input.scaled(OUTLINE_DARK_BG_ALPHA)
            } else {
                p.background.hsla()
            }),
        },
        bg_hover: if pressed {
            // Ver a regra 3: o ligado não reage ao ponteiro.
            ligado
        } else if moldura && p.dark {
            p.input.scaled(OUTLINE_DARK_HOVER_ALPHA)
        } else {
            p.accent.hsla()
        },
        border: moldura.then(|| p.input.hsla()),
        text: if pressed {
            p.accent_fg.hsla()
        } else {
            p.foreground.hsla()
        },
        shadow: realce.then(|| p.shadow.hsla()),
        bevel: realce,
    }
}

/// A sombra externa `shadow-xs/5`, na geometria do Tailwind v4 (`0 1px 2px 0`).
///
/// Continua sendo um [`gpui::BoxShadow`] de verdade (diferente do bisel e do anel): ela fica ATRÁS de
/// um fundo opaco, então o fato de o `Window::paint_shadows` não a recortar pra fora do elemento não
/// aparece.
fn shadow(color: Hsla) -> gpui::BoxShadow {
    gpui::BoxShadow {
        color,
        offset: gpui::point(px(0.0), px(1.0)),
        blur_radius: px(2.0),
        spread_radius: px(0.0),
    }
}

/// O **fio de bisel** de 1px da variante `Outline`.
///
/// ⚠️ As duas armadilhas da casa, as mesmas do [`crate::button`] e do [`crate::empty`]:
///
/// 1. **É borda, não sombra.** O `Window::paint_shadows` não recorta a sombra pra fora do elemento
///    que a projeta, então o `before:box-shadow` do coss viraria uma lavagem de cor sobre o toggle
///    inteiro.
/// 2. **Cobre a BORDER box.** O `before:rounded-[calc(var(--radius-lg)-1px)]` do original descreve um
///    pseudo-elemento na *padding* box, e é de lá que sai o `− 1`. O overlay daqui fica em
///    `inset: -1px`, ou seja na border box, então o raio dele é o da superfície ([`RADIUS`]).
///
/// O estado APERTADO troca a cor do filete em vez de apagá-lo (no escuro) — e isso é estado de
/// interação, que um [`RenderOnce`] não conhece. Quem conhece é o GPUI, via `active_style`; e como
/// `active` só é aplicado em elemento com `id` **e hitbox** (ver `Interactivity::should_insert_hitbox`),
/// o `.hover(|s| s)` abaixo existe só pra forçar a hitbox — o refinamento é vazio de propósito. A
/// hitbox é `Normal`, que não oclui: o clique continua chegando no toggle atrás.
fn bevel_overlay(id: &ElementId, join: Join, interactive: bool) -> gpui::AnyElement {
    let p = palette();
    let base = join.bevel_overlay(-1.0, RADIUS).border_color(p.bevel.hsla());
    let base = if p.bevel_dir > 0.0 {
        base.border_b_1()
    } else {
        base.border_t_1()
    };
    if !interactive {
        return base.into_any_element();
    }
    // No claro o filete SOME ao apertar (não há regra `active` na referência); no escuro ele escurece.
    let apertado = p
        .bevel_active
        .map_or(gpui::transparent_black(), |c| c.hsla());
    base.id(ElementId::NamedChild(Box::new(id.clone()), "bevel".into()))
        .hover(|s| s)
        .active(move |s| s.border_color(apertado))
        .into_any_element()
}

/// O **anel de foco** — `focus-visible:ring-2 ring-offset-1`.
///
/// Um `ring` do Tailwind é a forma do elemento DILATADA: 2px de anel mais 1px de folga = um overlay
/// 3px maior de cada lado, com o raio crescendo junto. Não dá pra fingir com sombra — o
/// `spread_radius` dilata os limites mas MANTÉM o raio, e a curvatura sai errada nas quinas (foi o
/// que aconteceu no [`crate::Input`]).
///
/// A geometria (incluindo o que fazer na emenda de um grupo) é do [`Join`]; daqui sai só a espessura
/// e a cor.
fn ring_overlay(join: Join) -> Div {
    let inset = -(RING_WIDTH + RING_OFFSET);
    join.ring_overlay(inset, RADIUS + RING_WIDTH + RING_OFFSET)
        .border_2()
        .border_color(palette().ring.hsla())
}

// =================================================================================================
// O componente
// =================================================================================================

/// Botão de dois estados com o visual do coss. Ver o doc do módulo.
///
/// É um **elemento de render** ([`RenderOnce`]): construa a cada frame, e passe o estado por
/// [`Self::pressed`].
#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<SharedString>,
    pressed: bool,
    disabled: bool,
    size: ToggleSize,
    variant: ToggleVariant,
    focus_handle: Option<FocusHandle>,
    /// Costura com os vizinhos, quando ele está num grupo. Ver [`crate::group::Join`].
    pub(crate) join: Join,
    /// **Esconde o fio de bisel**, mesmo numa combinação que teria um.
    ///
    /// Existe por uma regra só, a do [`crate::toggle_group::ToggleGroup`] vertical: lá o filete de
    /// um item do meio cairia DENTRO da emenda, em cima do separador. O grupo apaga o filete de
    /// todos menos o item da ponta pra onde o bisel aponta — ver `bevel_visible` naquele módulo. É
    /// o `*:data-[slot=toggle]:not-last:before:hidden` do coss.
    pub(crate) bevel_hidden: bool,
    /// `Rc` (e não `Box`) porque o [`crate::toggle_group::ToggleGroup`] precisa CLONAR o callback pra
    /// dentro do handler de clique dele.
    on_toggle: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl Toggle {
    /// Um toggle com **rótulo**.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: Some(label.into()),
            icon: None,
            pressed: false,
            disabled: false,
            size: ToggleSize::default(),
            variant: ToggleVariant::default(),
            focus_handle: None,
            join: Join::NONE,
            bevel_hidden: false,
            on_toggle: None,
        }
    }

    /// Um toggle só de **ícone** — quadrado, pelo `min-w` (ver [`ToggleSize::min_width`]).
    ///
    /// O caminho é servido pela [`crate::assets::Assets`] (ex.: `"iconoir/regular/bold.svg"`). Sem
    /// essa `AssetSource` registrada no bootstrap, o ícone some SILENCIOSAMENTE.
    pub fn icon(id: impl Into<ElementId>, path: impl Into<SharedString>) -> Self {
        Self {
            icon: Some(path.into()),
            label: None,
            ..Self::new(id, "")
        }
    }

    /// Um ícone **antes do rótulo**.
    pub fn with_icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }

    /// **Se está ligado.** É o estado, e ele vem de fora — ver "Controlado, sempre" no doc do módulo.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Liga o anel de foco a um [`FocusHandle`]. Sem isto o toggle não entra na navegação por
    /// teclado — igual ao [`crate::Button`].
    pub fn focus(mut self, handle: &FocusHandle) -> Self {
        self.focus_handle = Some(handle.clone());
        self
    }

    /// Chamado no clique, com o valor **novo** (o inverso do atual).
    ///
    /// Recebe `&bool` pra casar com a forma de `cx.listener`, que passa o evento por referência.
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    /// O estado atual — o [`crate::toggle_group::ToggleGroup`] lê isto pra saber a cor do separador
    /// vizinho.
    pub(crate) fn is_pressed(&self) -> bool {
        self.pressed
    }

    pub(crate) fn id(&self) -> &ElementId {
        &self.id
    }
}

impl RenderOnce for Toggle {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = palette();
        let s = self.size;
        let join = self.join;
        let interactive = !self.disabled;
        let style = variant_style(self.variant, self.pressed, self.disabled, p);

        // O anel só aparece com handle de foco E foco de TECLADO. Desabilitado nunca mostra anel —
        // um controle inerte não deve parecer que recebeu o cursor.
        let focused = interactive
            && self
                .focus_handle
                .as_ref()
                .is_some_and(|h| h.is_focused(window));

        // O respiro encolhe do lado que tem ícone (o `-mx-0.5`). Só de ícone, encolhe dos dois — e o
        // `min-w` garante que a caixa não fique menor que um quadrado.
        let (pad_l, pad_r) = match (&self.icon, &self.label) {
            (Some(_), None) => (s.pad_x_icon(), s.pad_x_icon()),
            (Some(_), Some(_)) => (s.pad_x_icon(), s.pad_x()),
            _ => (s.pad_x(), s.pad_x()),
        };

        let mut el = div()
            .id(self.id.clone())
            // `relative` porque bisel e anel são filhos ABSOLUTOS.
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(GAP))
            .h(px(s.height()))
            .min_w(px(s.min_width()))
            .pl(px(pad_l))
            .pr(px(pad_r))
            .text_size(px(TEXT_SIZE))
            .line_height(px(TEXT_LINE_HEIGHT))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(style.text)
            .border_color(style.border.unwrap_or(gpui::transparent_black()));
        // Raio e bordas vêm da COSTURA: solto são os quatro cantos e os quatro lados; num grupo, o
        // lado da emenda perde os dois. A borda de 1px EXISTE nas duas variantes, mesmo na "sem
        // borda" — é ela que mantém a altura interna igual entre elas (a altura é total, border-box).
        el = join.rounded(el, RADIUS);
        el = join.borders(el);

        if let Some(bg) = style.bg {
            el = el.bg(bg);
        }
        if let Some(cor) = style.shadow {
            el = el.shadow(vec![shadow(cor)]);
        }

        if interactive {
            let hover_bg = style.bg_hover;
            // O fundo de hover é refinamento de estilo, e não interpolação como no `Button`: a
            // referência só transiciona sombra (ver o doc do módulo).
            el = el
                .cursor(gpui::CursorStyle::PointingHand)
                .hover(move |s| s.bg(hover_bg));
            // A sombra morre ao apertar (`[:active]:shadow-none`).
            if style.shadow.is_some() {
                el = el.active(|s| s.shadow(Vec::new()));
            }

            let id_hover = self.id.clone();
            el = el.on_hover(move |hovered, window, _cx| {
                // A tabela de interação do `Button` é por `ElementId`, não é "de botão": é ela que
                // deixa o separador de um grupo saber que o vizinho está sob o ponteiro. Ver
                // `crate::group::Group`.
                crate::button::record_hover(&id_hover, *hovered);
                window.refresh();
            });

            let novo = !self.pressed;
            let handler = self.on_toggle.clone();
            el = el.on_click(move |event: &ClickEvent, window, cx| {
                // Clique é ponteiro: apaga o anel de foco (o foco em si continua indo pro controle).
                crate::focus_ring::pointer_used(window);
                if let Some(h) = handler.as_ref() {
                    h(&novo, window, cx);
                }
                // O evento não é usado, mas a assinatura do GPUI o entrega — nomeá-lo documenta que
                // o toggle não olha modificador nem contagem de cliques.
                let _ = event;
            });

            if let Some(handle) = self.focus_handle.as_ref() {
                el = el.track_focus(handle);
            }
        } else {
            // `disabled:opacity-64` + `disabled:pointer-events-none`: esmaece o conjunto (elemento e
            // overlays) e sai do caminho do ponteiro.
            el = el.opacity(DISABLED_OPACITY);
        }

        // --- Conteúdo ---------------------------------------------------------------------------
        if let Some(path) = self.icon {
            let mut cor = style.text;
            cor.a *= ICON_OPACITY;
            el = el.child(
                gpui::svg()
                    .path(path)
                    .size(px(ICON_SIZE))
                    .flex_none()
                    .text_color(cor),
            );
        }
        if let Some(label) = self.label {
            el = el.child(div().flex_none().child(label));
        }

        // --- Overlays, por último: são absolutos, então a ordem só decide quem pinta em cima ------
        el = el.when(style.bevel && !self.bevel_hidden, |el| {
            el.child(bevel_overlay(&self.id, join, interactive))
        });
        // O anel é `focus-visible`, não `focus`: só acende se o foco veio do teclado. Sem o segundo
        // termo, todo clique acenderia 2px de anel, porque o `track_focus` faz o mouse-down focar.
        el.when(focused && crate::focus_ring::visible(), |el| {
            el.child(ring_overlay(join))
        })
    }
}

/// Um toggle **dentro de um [`crate::Group`]**.
///
/// O grupo genérico serve pra misturar toggle com botão e campo numa peça só; pra um botão segmentado
/// homogêneo o caminho é o [`crate::toggle_group::ToggleGroup`], que propaga variante e tamanho e
/// conhece a regra de seleção.
impl crate::group::GroupChild for Toggle {
    fn into_joined(
        mut self: Box<Self>,
        join: Join,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::AnyElement {
        self.join = join;
        (*self).render(window, cx).into_any_element()
    }

    fn button_id(&self) -> Option<ElementId> {
        // O separador vizinho clareia quando o toggle está sob o ponteiro — mesma regra do botão.
        Some(self.id.clone())
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::color::opaque;

    /// As alturas e os respiros, na escala do Tailwind (1 unidade = 4px), no ramo `sm:`.
    ///
    /// O respiro carrega o `-1px` da borda: é ele que faz o recuo ÓTICO fechar o número do
    /// `--spacing` (7 + 1 de borda = os 8px de `--spacing(2)`).
    #[test]
    fn geometria_e_a_do_ramo_sm() {
        let esperado = [
            (ToggleSize::Sm, 28.0, 5.0, 6.0),      // sm:h-7, calc(spacing(1.5) - 1px)
            (ToggleSize::Default, 32.0, 7.0, 8.0), // sm:h-8, calc(spacing(2)   - 1px)
            (ToggleSize::Lg, 36.0, 9.0, 10.0),     // sm:h-9, calc(spacing(2.5) - 1px)
        ];
        for (size, altura, pad, spacing) in esperado {
            assert_eq!(size.height(), altura, "{size:?}: altura");
            assert_eq!(size.pad_x(), pad, "{size:?}: respiro");
            assert_eq!(
                size.pad_x() + 1.0,
                spacing,
                "{size:?}: respiro + borda tem que fechar o --spacing da referência"
            );
            // O `min-w-N` é o mesmo N do `h-N` em todos os três — é o que deixa o toggle de ícone
            // quadrado sem um tamanho `Icon*` separado.
            assert_eq!(size.min_width(), altura, "{size:?}: min-w acompanha o h");
        }
        assert_eq!(ToggleSize::ALL.len(), 3, "a lista única cobre os três");
    }

    /// O `-mx-0.5` do ícone sai do respiro, e nunca o deixa negativo.
    #[test]
    fn icone_puxa_dois_pixels_do_respiro() {
        for size in ToggleSize::ALL {
            assert_eq!(size.pad_x_icon(), size.pad_x() - ICON_PULL);
            assert!(size.pad_x_icon() >= 0.0, "{size:?}: respiro negativo");
        }
        // No menor deles ainda sobra respiro — se um tamanho novo zerar, o piso é o que segura.
        assert_eq!(ToggleSize::Sm.pad_x_icon(), 3.0);
    }

    /// **O raio NÃO muda com o tamanho** — ao contrário do [`crate::ButtonSize`]. No original o
    /// `rounded-lg` está na classe base do `cva` e nenhuma variante de `size` o troca.
    #[test]
    fn raio_e_o_mesmo_em_todos_os_tamanhos() {
        assert_eq!(RADIUS, 10.0, "rounded-lg = --radius-lg = --radius");
        // O contraste com o botão é a razão deste teste existir: lá o `Xs` cai pra 8.
        assert_eq!(crate::ButtonSize::Xs.radius(), 8.0);
        assert_eq!(crate::ButtonSize::Default.radius(), RADIUS);
    }

    /// O texto é o par `(14, 20)` do `text-sm`, e não a razão de ouro do GPUI.
    #[test]
    fn entrelinha_e_o_par_do_tailwind() {
        assert_eq!((TEXT_SIZE, TEXT_LINE_HEIGHT), (14.0, 20.0), "o par do text-sm");
        const RAZAO_DE_OURO: f32 = 1.618_034;
        assert!(
            TEXT_SIZE * RAZAO_DE_OURO > TEXT_LINE_HEIGHT + 2.0,
            "sem fixar, a linha sairia {:.1}px",
            TEXT_SIZE * RAZAO_DE_OURO
        );
    }

    /// **A variante `Default` não tem moldura, e a `Outline` tem tudo.** É o eixo do `cva`.
    #[test]
    fn variantes_diferem_em_moldura_sombra_e_bisel() {
        for p in [&TOGGLE_LIGHT, &TOGGLE_DARK] {
            let simples = variant_style(ToggleVariant::Default, false, false, p);
            let moldura = variant_style(ToggleVariant::Outline, false, false, p);

            assert!(simples.border.is_none(), "Default: borda transparente");
            assert!(simples.bg.is_none(), "Default: sem fundo em repouso");
            assert!(simples.shadow.is_none(), "Default: sem sombra");
            assert!(!simples.bevel, "Default: sem bisel");

            assert_eq!(moldura.border, Some(p.input.hsla()), "Outline: borda --input");
            assert!(moldura.bg.is_some(), "Outline: fundo de superfície");
            assert!(moldura.shadow.is_some(), "Outline: shadow-xs/5");
            assert!(moldura.bevel, "Outline: bisel");
        }
    }

    /// **A `Outline` troca de TOKEN entre os temas**, não só de valor: `--background` no claro,
    /// `--input`/32 no escuro. Um porte que resolvesse só o valor daria branco no escuro.
    #[test]
    fn fundo_da_outline_troca_de_token_entre_os_temas() {
        let claro = variant_style(ToggleVariant::Outline, false, false, &TOGGLE_LIGHT);
        assert_eq!(claro.bg, Some(TOGGLE_LIGHT.background.hsla()));

        let escuro = variant_style(ToggleVariant::Outline, false, false, &TOGGLE_DARK);
        assert_eq!(escuro.bg, Some(TOGGLE_DARK.input.scaled(0.32)));
        assert_ne!(
            escuro.bg,
            Some(TOGGLE_DARK.background.hsla()),
            "no escuro o fundo NÃO é --background"
        );
    }

    /// O estado LIGADO: fundo `--input`/64 e texto `--accent-foreground` nas duas variantes — com a
    /// `Outline` escura subindo pra `--input` cheio (`dark:data-pressed:bg-input`).
    #[test]
    fn ligado_usa_input_e_o_texto_do_accent() {
        for p in [&TOGGLE_LIGHT, &TOGGLE_DARK] {
            for variant in [ToggleVariant::Default, ToggleVariant::Outline] {
                let ligado = variant_style(variant, true, false, p);
                assert_eq!(ligado.text, p.accent_fg.hsla(), "{variant:?}: texto de ligado");
                assert!(ligado.bg.is_some(), "{variant:?}: ligado sempre tem fundo");
                // Sombra e bisel morrem no ligado (`[[data-pressed]]:shadow-none`).
                assert!(ligado.shadow.is_none(), "{variant:?}: ligado sem sombra");
                assert!(!ligado.bevel, "{variant:?}: ligado sem bisel");
            }
        }

        // O alfa: `/64` MULTIPLICA o alfa que a cor já tem (armadilha do Tailwind v4).
        let claro = variant_style(ToggleVariant::Default, true, false, &TOGGLE_LIGHT);
        assert_eq!(claro.bg, Some(TOGGLE_LIGHT.input.scaled(0.64)));
        let alfa = claro.bg.expect("fundo de ligado").a;
        assert!(
            (alfa - 0.10 * 0.64).abs() < 0.01,
            "black 10% × 64% ≈ 6,4%, e não 64% — saiu {alfa:.3}"
        );

        // E a exceção da `Outline` escura: `--input` CHEIO.
        let escuro = variant_style(ToggleVariant::Outline, true, false, &TOGGLE_DARK);
        assert_eq!(escuro.bg, Some(TOGGLE_DARK.input.hsla()));
    }

    /// **Um toggle ligado não muda de fundo sob o ponteiro.** O `hover:bg-accent` da classe base
    /// perde do `data-pressed:bg-input/64`, e é isso que faz o estado ligado continuar legível
    /// enquanto o ponteiro passa.
    #[test]
    fn hover_nao_apaga_o_estado_ligado() {
        for p in [&TOGGLE_LIGHT, &TOGGLE_DARK] {
            for variant in [ToggleVariant::Default, ToggleVariant::Outline] {
                let ligado = variant_style(variant, true, false, p);
                assert_eq!(
                    Some(ligado.bg_hover),
                    ligado.bg,
                    "{variant:?}: o hover do ligado é o próprio fundo de ligado"
                );
            }
        }
        // Já o DESLIGADO reage: é o `hover:bg-accent` (ou o `dark:hover:bg-input/64` da Outline).
        let desligado = variant_style(ToggleVariant::Default, false, false, &TOGGLE_LIGHT);
        assert_eq!(desligado.bg_hover, TOGGLE_LIGHT.accent.hsla());
        let outline_escuro = variant_style(ToggleVariant::Outline, false, false, &TOGGLE_DARK);
        assert_eq!(
            outline_escuro.bg_hover,
            TOGGLE_DARK.input.scaled(0.64),
            "no escuro a Outline usa --input/64, não --accent"
        );
    }

    /// Desabilitado apaga sombra e bisel — o `[:disabled]:shadow-none`.
    #[test]
    fn desabilitado_apaga_sombra_e_bisel() {
        for p in [&TOGGLE_LIGHT, &TOGGLE_DARK] {
            let s = variant_style(ToggleVariant::Outline, false, true, p);
            assert!(s.shadow.is_none());
            assert!(!s.bevel);
        }
        assert_eq!(DISABLED_OPACITY, 0.64, "disabled:opacity-64");
    }

    /// O bisel troca de LADO entre os temas, e o lado sai do SINAL do deslocamento — `0 1px` desce →
    /// borda de baixo; `0 -1px` sobe → borda de topo.
    #[test]
    fn bisel_troca_de_lado_entre_os_temas() {
        assert!(TOGGLE_LIGHT.bevel_dir > 0.0, "claro: 0 1px desce");
        assert!(TOGGLE_DARK.bevel_dir < 0.0, "escuro: 0 -1px sobe");
    }

    /// ⚠️ O desvio consciente da casa: o filete escuro é o DOBRO da referência.
    ///
    /// Se a intenção mudar, é este teste que tem que mudar junto — não o valor sozinho.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        let referencia = 0.06_f32;
        let nosso = TOGGLE_DARK.bevel.alpha();
        assert!(
            (nosso - 2.0 * referencia).abs() < 0.01,
            "esperado ~{:.3} (2 × 6%), veio {nosso:.3}",
            2.0 * referencia
        );
        // O par APERTADO segue a mesma régua: 2% → ~4%.
        let apertado = TOGGLE_DARK
            .bevel_active
            .expect("o escuro tem filete apertado")
            .alpha();
        assert!(
            (apertado - 2.0 * 0.02).abs() < 0.01,
            "esperado ~0,04 (2 × 2%), veio {apertado:.3}"
        );
        assert!(
            apertado < nosso,
            "apertar ESCURECE o filete, não clareia"
        );
    }

    /// **No claro o filete some ao apertar; no escuro ele escurece.** A diferença é um `not-active` a
    /// mais na regra escura da referência, e é a razão de [`TogglePalette::bevel_active`] ser
    /// [`Option`] em vez de uma cor.
    #[test]
    fn filete_apertado_existe_so_no_escuro() {
        assert!(TOGGLE_LIGHT.bevel_active.is_none());
        assert!(TOGGLE_DARK.bevel_active.is_some());
    }

    /// Os tokens de tema entram por [`opaque`], que é a única ponte entre os `0xRRGGBB` de
    /// [`crate::theme`] e os `0xRRGGBBAA` daqui.
    #[test]
    fn tokens_do_tema_entram_por_opaque() {
        assert_eq!(TOGGLE_LIGHT.foreground, opaque(0x262626), "neutral-800");
        assert_eq!(TOGGLE_DARK.foreground, opaque(0xf5f5f5), "neutral-100");
        assert_eq!(TOGGLE_LIGHT.ring, opaque(0xa3a3a3), "neutral-400");
        assert_eq!(TOGGLE_DARK.ring, opaque(0x737373), "neutral-500");
        assert_eq!(TOGGLE_LIGHT.background, opaque(0xffffff));
        // `--background` escuro = mix(neutral-950 96%, white) = 10·0,96 + 255·0,04 ≈ 20 = 0x14.
        assert_eq!(TOGGLE_DARK.background, opaque(0x141414));
    }

    /// As paletas decodificam pras cores pretendidas — o teste que pega a inversão de canais que a
    /// convenção `0xRRGGBBAA` existe pra evitar.
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        let casos = [
            ("accent claro", TOGGLE_LIGHT.accent, 0.04),
            ("accent escuro", TOGGLE_DARK.accent, 0.04),
            ("input claro", TOGGLE_LIGHT.input, 0.10),
            ("input escuro", TOGGLE_DARK.input, 0.08),
            ("sombra", TOGGLE_LIGHT.shadow, 0.05),
        ];
        for (nome, cor, alfa) in casos {
            assert!(
                (cor.alpha() - alfa).abs() < 0.01,
                "{nome}: esperado {alfa:.2} de alfa, veio {:.3}",
                cor.alpha()
            );
        }
        // E o RGB de cada um é preto no claro, branco no escuro.
        assert_eq!(TOGGLE_LIGHT.accent.0 & 0xffffff00, 0x00000000);
        assert_eq!(TOGGLE_DARK.accent.0 & 0xffffff00, 0xffffff00);
    }

    /// O anel é 2px com 1px de folga, e o raio dele CRESCE junto — senão a curvatura foge nas quinas.
    #[test]
    fn anel_dilata_a_forma() {
        assert_eq!((RING_WIDTH, RING_OFFSET), (2.0, 1.0), "ring-2 ring-offset-1");
        let raio_do_anel = RADIUS + RING_WIDTH + RING_OFFSET;
        assert_eq!(raio_do_anel, 13.0, "10 + 2 + 1");
    }

    /// O construtor de ícone não deixa rótulo vazio pra trás — um `""` viraria um filho de texto
    /// invisível que ainda ocupa o `gap`.
    #[test]
    fn toggle_de_icone_nao_tem_rotulo() {
        let t = Toggle::icon("i", "x.svg");
        assert!(t.label.is_none());
        assert!(t.icon.is_some());
        // E o de rótulo não tem ícone.
        let r = Toggle::new("r", "Negrito");
        assert!(r.icon.is_none());
        assert_eq!(r.label, Some("Negrito".into()));
    }

    /// O default do componente é `Default`/`Default`/desligado/ativo — o mesmo do `cva`.
    #[test]
    fn defaults_batem_com_o_cva() {
        let t = Toggle::new("t", "x");
        assert_eq!(t.size, ToggleSize::Default);
        assert_eq!(t.variant, ToggleVariant::Default);
        assert!(!t.pressed);
        assert!(!t.disabled);
        assert_eq!(t.join, Join::NONE, "solto por default");
        assert!(
            !t.bevel_hidden,
            "o filete aparece por default — só o grupo VERTICAL o apaga"
        );
    }
}

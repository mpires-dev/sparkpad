//! `Sheet` — o **painel que entra por uma borda da janela**, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/sheet.tsx`
//!
//! # Duas peças
//!
//! Exatamente a mesma divisão do [`crate::dialog`], e pelos mesmos motivos:
//!
//! - [`Sheet`] — o **estado**, um [`gpui::Entity`]. Guarda se está aberto, o relógio da transição, o
//!   [`gpui::FocusHandle`] do painel (é dele que o `Escape` chega) e o [`gpui::ScrollHandle`] do
//!   conteúdo. Emite [`SheetEvent`].
//! - [`sheet_layer`] — o **elemento montável**: um overlay absoluto que cobre a raiz da view
//!   hospedeira e desenha backdrop + viewport + painel. Existe porque o GPUI **não tem portal nem
//!   `position: fixed`** — o `Sheet.Portal`/`Sheet.Viewport` da referência se plantam na janela
//!   sozinhos, e aqui quem escolhe o lugar é a app.
//!
//! O controlador **não** é `Render`: o conteúdo de um painel é elemento ([`AnyElement`], de uso
//! único), que não sobrevive a um frame — então quem constrói o painel a cada frame é o chamador, e o
//! [`Sheet`] só guarda estado.
//!
//! # Montagem
//!
//! ```ignore
//! struct Shell { sheet: Entity<Sheet> }
//!
//! impl Shell {
//!     fn new(cx: &mut Context<Self>) -> Self {
//!         Self { sheet: cx.new(Sheet::new) }
//!     }
//! }
//!
//! impl Render for Shell {
//!     fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!         let sheet = self.sheet.clone();
//!         div()
//!             .relative()      // ⚠️ obrigatório: o overlay é ABSOLUTO
//!             .size_full()
//!             .child(o_conteudo_da_app())
//!             .child(sheet_layer(                        // por ÚLTIMO: pinta em cima
//!                 &self.sheet,
//!                 SheetPopup::new()
//!                     .side(SheetSide::Right)            // o default
//!                     .header(
//!                         SheetHeader::new()
//!                             .title("Propriedades")
//!                             .description("Ajustes do clipe selecionado."),
//!                     )
//!                     .panel(meu_formulario)
//!                     .footer(
//!                         SheetFooter::new()
//!                             .child(Button::new("cancelar", "Cancelar").on_click({
//!                                 let s = sheet.clone();
//!                                 move |_e, window, cx| {
//!                                     s.update(cx, |s, cx| s.close(window, cx));
//!                                 }
//!                             }))
//!                             .child(Button::new("salvar", "Salvar")),
//!                     ),
//!             ))
//!     }
//! }
//! ```
//!
//! E pra abrir, de qualquer handler que tenha `&mut Window`:
//!
//! ```ignore
//! self.sheet.update(cx, |s, cx| s.open(window, cx));
//! ```
//!
//! ## Por que os mutadores pedem `&mut Window`
//!
//! [`Sheet::open`] e [`Sheet::close`] recebem `&mut Window` por **dois** motivos, os dois
//! obrigatórios:
//!
//! 1. **Foco.** Abrir foca o painel (é assim que o `Escape` chega até ele) e fechar devolve o foco a
//!    quem o tinha. Focar exige janela.
//! 2. **Redesenho.** O painel é construído dentro do `render` da view HOSPEDEIRA. Um `cx.notify()`
//!    no [`Sheet`] só sujaria quem o **observa**, e a hospedeira não observa nada — o painel abriria
//!    e a tela não mudaria até o próximo frame provocado por outra coisa. O
//!    [`gpui::Window::refresh`] resolve sem exigir um `cx.observe` no call site.
//!
//! # A geometria de cada lado
//!
//! Toda a diferença em relação ao [`crate::dialog`] está aqui. A referência dá ao painel **uma**
//! medida própria e deixa a outra cheia, e reserva sempre uma faixa de `--spacing(12)` = **48px** do
//! lado oposto, por onde a app continua aparecendo:
//!
//! | `side`   | medida própria                          | medida cheia | borda      | entra deslizando |
//! |----------|-----------------------------------------|--------------|------------|------------------|
//! | `Right`  | largura `min(100%−48, max-w-md = 448)`   | altura       | `border-s` | pra direita      |
//! | `Left`   | largura `min(100%−48, max-w-md = 448)`   | altura       | `border-e` | pra esquerda     |
//! | `Bottom` | altura do conteúdo, até `100%−48`       | largura      | `border-t` | pra baixo        |
//! | `Top`    | altura do conteúdo, até `100%−48`       | largura      | `border-b` | pra cima         |
//!
//! Nos lados verticais os 48px são `pt-12`/`pb-12` **no viewport**; nos horizontais são o
//! `calc(100% - --spacing(12))` **na largura do painel**. Aqui os dois casos viraram padding do
//! viewport (ver [`viewport_pads`]), o que dá a mesma conta com o `w_full` do painel — e a diferença
//! aparece só na variante `inset`, onde o `p-4` **soma** com os 48 nos lados horizontais e **perde**
//! deles nos verticais (no Tailwind o utilitário de um lado é emitido depois do `p-*`).
//!
//! ⚠️ Repare que na variante `Default` o painel **não tem raio nenhum** e tem **uma borda só**, a do
//! lado de dentro. Raio (`sm:rounded-2xl`) e as quatro bordas (`sm:border`) só existem na variante
//! [`SheetVariant::Inset`], que também afasta o painel 16px de todas as bordas da janela.
//!
//! # A animação de entrada: translação por geometria
//!
//! A referência é `transition-[opacity,translate] duration-200 ease-in-out`, com
//! `data-starting-style`/`data-ending-style` = `opacity-0` **e** `translate-*-8` = **32px pra fora**.
//! O GPUI não tem `translate` em `div` ([`gpui::TransformationMatrix`] só serve pra `svg`/imagem),
//! então o deslocamento é **geometria calculada**: um `inset` de posicionamento RELATIVO no eixo do
//! lado, `32 × (1 − progresso)` pra fora.
//!
//! Isto é fiel, e não é o caso do `scale` que o [`crate::dialog`] omitiu: escalar mudaria a caixa e
//! **refluiria o texto** durante os 200ms; transladar não muda tamanho nenhum — o painel tem a mesma
//! largura e a mesma altura em todo o percurso, só está noutro lugar. O teste de janela
//! `a_translacao_nao_muda_o_tamanho` trava exatamente isso.
//!
//! # O que o GPUI exigiu adaptar
//!
//! | coss                                            | aqui                                              |
//! |-------------------------------------------------|---------------------------------------------------|
//! | `Sheet.Portal` + `fixed inset-0`                | [`sheet_layer`], montado pela app                 |
//! | `grid grid-rows-[1fr_auto]` + `row-start-2`     | flex coluna com `justify_end`                     |
//! | `w-[calc(100%-(--spacing(12)))]`                | padding de 48px no viewport + `w_full` no painel   |
//! | `translate-x-8` / `translate-y-8`               | `inset` relativo calculado do progresso           |
//! | `before:shadow-[0_1px_…]` (bisel)               | overlay em `inset:-1px` com `border_b_1`          |
//! | `transition-opacity duration-200`               | lerp por [`Instant::elapsed`] + `Styled::opacity` |
//! | `backdrop-blur-sm`                              | **nada** — ver as omissões                        |
//!
//! # Onde isto NÃO é a referência
//!
//! Tudo abaixo é decisão consciente, e não descuido.
//!
//! **Não implementado por limitação do GPUI**
//!
//! - **`backdrop-blur-sm` não tem equivalente no GPUI.** Não há desfoque de fundo em nenhuma API de
//!   `div`. O backdrop entrega o `bg-black/32` sozinho. Emular com camadas fica pior que a omissão.
//! - **`not-dark:bg-clip-padding`**: o GPUI pinta o fundo na border box. No tema claro a borda
//!   (`--border` = preto 8%, translúcida) fica sobre o branco do fundo em vez de sobre o backdrop —
//!   na prática a borda lê como `#ebebeb` fixo. Mesma diferença já documentada no [`crate::dialog`]
//!   e no [`crate::toast`].
//! - **`overscrollContain`** do `ScrollArea` da referência: o [`crate::ScrollArea`] não tem a opção.
//!
//! **Fiel ao arquivo, mesmo parecendo esquisito**
//!
//! - **O bisel quase nunca aparece na variante `Default`, e nunca na `inset`.** No `inset` a
//!   referência traz `before:hidden` sem nenhum `sm:before:block` que o devolva — o
//!   `sm:before:rounded-[calc(var(--radius-2xl)-1px)]` que vem logo depois é código morto. Está
//!   reproduzido como está: [`SheetVariant::has_bevel`] é `false` no `inset`. E no `Default` o
//!   `before:inset-0` + `shadow-[0_1px_…]` põem o filete 1px pra **fora** da caixa, que nos lados
//!   cheios é a borda da janela: o filete só se vê onde ele cai sobre a borda do painel, ou seja no
//!   `Top` do tema claro (deslocamento pra baixo, `border-b`) e no `Bottom` do tema escuro
//!   (deslocamento pra cima, `border-t`). Nos lados horizontais ele é sempre invisível — no CSS
//!   também. **Se você esperava ver bisel num painel da direita, não é bug: é o arquivo.**
//!
//! **Não implementado por escopo**
//!
//! - **`SheetTrigger`/`SheetClose`**: são `render`-props do base-ui que só encaminham um `onClick`.
//!   Aqui o gatilho é qualquer botão seu chamando [`Sheet::open`], e o fechar é [`Sheet::close`].
//! - **Todo o bloco `max-sm:`** (`max-sm:pb-4` do header, `max-sm:before:hidden`): nesta base `sm:`
//!   **sempre** vale (desktop ≥ 640px), então o ramo de mobile é código morto.
//! - **Aprisionamento de foco** (o `Tab` circular do base-ui). Abrir FOCA o painel e fechar devolve o
//!   foco; o que falta é impedir o `Tab` de escapar, e isso depende de saber a ordem de tabulação da
//!   janela inteira.
//! - **Painéis aninhados.** Dois [`Sheet`] montados são independentes e não se enxergam.
//!
//! **Valores deduzidos**
//!
//! - **`font-heading`** do [`SheetTitle`](SheetHeader::title): é um token de FAMÍLIA de fonte, e não
//!   estava na tabela de tokens resolvidos. O título sai na fonte herdada, com o peso e o corpo
//!   certos (`font-semibold`, `text-xl`/`leading-none`).
//! - **`--popover-foreground`** também não estava na tabela. Uso `--foreground` (é o que o coss faz,
//!   e é o valor que o [`crate::toast`] e o [`crate::dialog`] já tinham resolvido).

use gpui::{
    div, px, relative, AnyElement, App, Context, Div, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    RenderOnce, ScrollHandle, SharedString, Styled, Window,
};

use std::time::{Duration, Instant};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Mesma disciplina de cor do dialog, do card e do botão: TODO valor é `0xRRGGBBAA`, com o byte de
// alfa, SEMPRE — ver [`crate::color`]. Os tokens de [`crate::theme`] são `0xRRGGBB`, e misturar as
// duas convenções desloca os canais e produz uma cor completamente diferente sem erro de compilação
// (`rgba(0xffffff)` é lido como ciano; já custou três bugs visíveis nesta base).

/// Tokens visuais do painel, por tema.
#[derive(Clone, Copy, Debug)]
struct SheetPalette {
    /// `--popover` — fundo da superfície do painel.
    popover: Rgba8,
    /// `--popover-foreground` — cor do texto. ⚠️ Não estava na tabela; é `--foreground` (ver o doc
    /// do módulo).
    popover_fg: Rgba8,
    /// `--muted-foreground` — a descrição do header.
    muted_fg: Rgba8,
    /// `--border` — a(s) borda(s) de 1px do painel e a linha de topo do footer.
    border: Rgba8,
    /// `shadow-lg/5` — a cor das duas sombras externas.
    shadow: Rgba8,
    /// Fio de bisel de 1px. Desce no claro, sobe no escuro.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (`0 1px`, base), `-1` sobe (`0 -1px`, topo).
    bevel_dir: f32,
    /// `bg-muted/72` — fundo da faixa de footer da variante default.
    footer_bg: Rgba8,
    /// `bg-black/32` — o backdrop. Igual nos dois temas: é preto puro, não um token.
    backdrop: Rgba8,
}

/// Tema **claro**.
const SHEET_LIGHT: SheetPalette = SheetPalette {
    popover: Rgba8(0xffffffff),
    popover_fg: Rgba8(0x262626ff), // neutral-800
    // mix(neutral-500 90%, black) = #686868 — o mesmo do `crate::card` e do `crate::dialog`.
    muted_fg: Rgba8(0x686868ff),
    border: Rgba8(0x00000014), // black 8%
    shadow: Rgba8(0x0000000d), // black 5% (o `/5` do `shadow-lg/5`)
    bevel: Rgba8(0x0000000a),  // black 4% — o `--color-black/4%` do `before:shadow`
    bevel_dir: 1.0,
    // `--muted` é black 4% (alfa 10); o `/72` do Tailwind multiplica o alfa → 4% × 0,72 ≈ 2,9%
    // (alfa 7). O mesmo valor que o `crate::frame` e o `crate::dialog` já tinham resolvido.
    footer_bg: Rgba8(0x00000007),
    backdrop: Rgba8(0x00000052), // black 32%
};

/// Tema **escuro**.
const SHEET_DARK: SheetPalette = SheetPalette {
    // `--popover` escuro = mix(background 96%, white) = #1d1d1d.
    popover: Rgba8(0x1d1d1dff),
    popover_fg: Rgba8(0xf5f5f5ff), // neutral-100
    // mix(neutral-500 90%, white) = #818181.
    muted_fg: Rgba8(0x818181ff),
    border: Rgba8(0xffffff0f), // white 6%
    shadow: Rgba8(0x0000000d), // black 5% — a sombra não muda com o tema
    // ⚠️ DESVIO CONSCIENTE do coss, o MESMO já vigente no `Input`, `Card`, `Button`, `Frame`,
    // `Toast` e `Dialog`: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO — alfa 30 ≈
    // 11,8% — porque a 6% o filete é imperceptível no nosso fundo escuro. NÃO "corrija" isto pra
    // 0x0f achando que é erro de porte; se a intenção mudar, mude junto o teste
    // `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
    footer_bg: Rgba8(0xffffff07), // white 4% × 72%
    // O backdrop é `bg-black/32` — preto literal, sem token de tema. Idêntico no claro e no escuro.
    backdrop: Rgba8(0x00000052),
};

/// A paleta do painel no tema corrente.
fn palette() -> &'static SheetPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &SHEET_DARK,
        theme::ThemeMode::Light => &SHEET_LIGHT,
    }
}

// =================================================================================================
// Geometria (utilitários Tailwind da referência resolvidos em número)
// =================================================================================================

/// Raio da superfície na variante `inset` — `sm:rounded-2xl`. O coss **não** redefine `--radius-2xl`,
/// então vale o default do Tailwind: `1rem` = **16px**.
///
/// ⚠️ Na variante `Default` o painel **não tem raio** — ver [`SheetVariant::radius`].
const RADIUS: f32 = 16.0;

/// Raio das quinas de BAIXO da faixa de footer na variante `inset` —
/// `rounded-b-[calc(var(--radius-2xl)-1px)]` = 15px.
///
/// A faixa tem fundo próprio (`bg-muted/72`) e é o último filho da superfície, que **não** recorta os
/// filhos (o `overflow_hidden` comeria o overlay de bisel — ver [`bevel_overlay`]). Sem este raio o
/// retângulo do fundo apareceria pra fora da curva da superfície. É `RADIUS − 1` porque a faixa vive
/// na padding box, uma borda pra dentro.
const FOOTER_RADIUS: f32 = RADIUS - 1.0;

/// Largura máxima do painel nos lados horizontais — `max-w-md` = `28rem` = **448px**.
///
/// ⚠️ Não é o `max-w-lg` (512) do [`crate::dialog`].
const MAX_WIDTH: f32 = 448.0;

/// A faixa da janela que o painel NUNCA cobre, do lado oposto ao dele — `--spacing(12)` = **48px**.
///
/// Na referência ela aparece de duas formas, que dão no mesmo: `pt-12`/`pb-12` no viewport (lados
/// vertical) e `w-[calc(100%-(--spacing(12)))]` no painel (lados horizontais). Ver
/// [`viewport_pads`].
const OUTER_GAP: f32 = 48.0;

/// Respiro do viewport contra as bordas da janela na variante `inset` — `sm:p-4`.
const INSET_PAD: f32 = 16.0;

/// De quanto o painel entra deslizando — `translate-x-8`/`translate-y-8` = `2rem` = **32px**.
///
/// É um deslocamento CURTO de propósito: o painel não vem de fora da janela, ele aparece
/// desvanecendo com um empurrãozinho. Um slide da largura inteira seria outra animação.
const SLIDE: f32 = 32.0;

/// Respiro padrão das fatias — `p-6`.
const PAD: f32 = 24.0;

/// Respiro apertado, no lado em que uma fatia encosta noutra — `pb-3`/`pt-3`.
const PAD_SNUG: f32 = 12.0;

/// Respiro mínimo do conteúdo contra a fatia vizinha — `pt-1`/`pb-1`.
///
/// Some com o `PAD_SNUG` do vizinho: 12 + 4 = 16px entre o texto do header e o conteúdo.
const PAD_HAIR: f32 = 4.0;

/// Respiro vertical da faixa de footer da variante default — `py-4`.
const FOOTER_PAD_Y: f32 = 16.0;

/// Respiro de topo da faixa de footer da variante `bare` quando ela NÃO segue um painel — `pt-4`.
const FOOTER_BARE_PAD_TOP: f32 = 16.0;

/// Espaço entre título e descrição, e entre as ações do footer — `gap-2`.
const GAP: f32 = 8.0;

/// Corpo do título — `text-xl`.
const TITLE_SIZE: f32 = 20.0;

/// Altura de linha do título — `leading-none`, ou seja **igual ao corpo**.
///
/// Sem isto a entrelinha default do GPUI (`relative(1.618)`) engordaria o header em vários pixels —
/// o mesmo achado que o [`crate::frame`] documenta pro `text-sm`.
const TITLE_LINE: f32 = TITLE_SIZE;

/// Corpo da descrição — `text-sm`.
const DESC_SIZE: f32 = 14.0;

/// Altura de linha da descrição — o par do `text-sm` do Tailwind é 14px/**20px**.
const DESC_LINE: f32 = 20.0;

/// Recuo do botão de fechar contra as bordas do painel — `absolute end-2 top-2`.
const CLOSE_INSET: f32 = 8.0;

/// Duração da transição de entrada e de saída — `duration-200`.
const TRANSITION: Duration = Duration::from_millis(200);

// =================================================================================================
// Lado e variante
// =================================================================================================

/// Por qual borda da janela o painel entra.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SheetSide {
    /// Encosta no topo: largura cheia, altura do conteúdo, `border-b`.
    Top,
    /// Encosta na direita: altura cheia, largura própria, `border-s`. **O default da referência.**
    #[default]
    Right,
    /// Encosta na base: largura cheia, altura do conteúdo, `border-t`.
    Bottom,
    /// Encosta na esquerda: altura cheia, largura própria, `border-e`.
    Left,
}

impl SheetSide {
    /// Se o painel encosta numa borda **vertical** (esquerda/direita).
    ///
    /// É o que decide o eixo de tudo: nos horizontais o painel tem LARGURA própria (limitada por
    /// [`MAX_WIDTH`]) e altura cheia, e desliza em X; nos verticais é o contrário.
    fn horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// O sinal do "pra fora" no eixo deste lado: `+1` pra direita/baixo, `−1` pra esquerda/cima.
    ///
    /// É o `translate-x-8` contra o `-translate-x-8` da referência. Este sinal DIRIGE o
    /// deslocamento da animação, pra não haver como divergir do lado escolhido.
    fn outward(self) -> f32 {
        match self {
            Self::Right | Self::Bottom => 1.0,
            Self::Left | Self::Top => -1.0,
        }
    }
}

/// Como o painel se separa das bordas da janela.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SheetVariant {
    /// Encostado, sem raio e com uma borda só — a do lado de dentro. O default da referência.
    #[default]
    Default,
    /// Afastado 16px de todas as bordas (`sm:p-4`), com raio (`sm:rounded-2xl`) e as quatro bordas
    /// (`sm:border`).
    Inset,
}

impl SheetVariant {
    /// O raio da superfície. **Zero na variante `Default`**: a referência só arredonda no `inset`.
    fn radius(self) -> f32 {
        match self {
            Self::Default => 0.0,
            Self::Inset => RADIUS,
        }
    }

    /// O respiro do viewport contra a janela — `sm:p-4` no `inset`, nada no `Default`.
    fn pad(self) -> f32 {
        match self {
            Self::Default => 0.0,
            Self::Inset => INSET_PAD,
        }
    }

    /// Se o filete de bisel existe.
    ///
    /// ⚠️ `false` no `inset`, e isso é fiel: a referência põe `before:hidden` na variante e nunca o
    /// devolve (ver o doc do módulo).
    fn has_bevel(self) -> bool {
        self == Self::Default
    }
}

/// Os quatro respiros do viewport, na ordem do CSS: **(topo, direita, base, esquerda)**.
///
/// Aqui mora a única assimetria de verdade entre os lados, e ela vem do arquivo:
///
/// - **Lados verticais** (topo/base): os 48px são `pt-12`/`pb-12`, utilitários de UM lado, e no
///   Tailwind eles são emitidos DEPOIS do `p-*` — então na variante `inset` o `pt-12` **vence** o
///   `p-4`, e o respiro daquele lado é 48, não 64.
/// - **Lados horizontais** (esquerda/direita): os 48px não são padding na referência, são o
///   `calc(100% - --spacing(12))` da LARGURA do painel — que é uma porcentagem do content box do
///   viewport, ou seja **já descontado** o `p-4`. Logo os dois se **somam**: 16 + 48 = 64.
///
/// Virar padding do viewport (com `w_full`/altura esticada no painel) dá exatamente a mesma conta e
/// evita um `calc` que o GPUI não tem.
fn viewport_pads(side: SheetSide, variant: SheetVariant) -> [f32; 4] {
    let pad = variant.pad();
    let gap = if side.horizontal() {
        pad + OUTER_GAP
    } else {
        OUTER_GAP
    };
    let mut e = [pad; 4];
    match side {
        SheetSide::Top => e[2] = gap,
        SheetSide::Right => e[3] = gap,
        SheetSide::Bottom => e[0] = gap,
        SheetSide::Left => e[1] = gap,
    }
    e
}

/// As espessuras de borda do painel, na ordem do CSS: **(topo, direita, base, esquerda)**.
///
/// Na variante `Default` existe **uma** borda só, a do lado de DENTRO (`border-s` num painel da
/// direita, `border-e` num da esquerda, `border-t` num de baixo, `border-b` num de cima); na
/// `inset`, as quatro (`sm:border`).
///
/// Esta função dirige tanto o desenho quanto os testes de janela (que precisam converter a padding
/// box medida pela sonda na border box da superfície).
fn border_edges(side: SheetSide, variant: SheetVariant) -> [f32; 4] {
    if variant == SheetVariant::Inset {
        return [1.0; 4];
    }
    match side {
        SheetSide::Top => [0.0, 0.0, 1.0, 0.0],    // border-b
        SheetSide::Right => [0.0, 0.0, 0.0, 1.0],  // border-s (LTR: esquerda)
        SheetSide::Bottom => [1.0, 0.0, 0.0, 0.0], // border-t
        SheetSide::Left => [0.0, 1.0, 0.0, 0.0],   // border-e (LTR: direita)
    }
}

// =================================================================================================
// Curva de animação
// =================================================================================================

/// A curva da referência — `ease-in-out`, que no CSS é `cubic-bezier(.42,0,.58,1)`.
///
/// Está duplicada em relação ao [`crate::dialog`] e ao [`crate::toast`] (que implementam a MESMA
/// bissecção) porque lá ela é privada do módulo. Ver o relatório: é a primeira candidata a extrair.
fn ease_in_out(t: f32) -> f32 {
    cubic_bezier(0.42, 0.0, 0.58, 1.0, t)
}

/// Uma `cubic-bezier(x1,y1,x2,y2)` de CSS avaliada em `t` ∈ `[0,1]`.
///
/// Implementada de verdade (e não aproximada): a bezier do CSS é paramétrica em `u`, então achar `y`
/// pra um `t` dado exige inverter `x(u) = t`. Vinte passos de bissecção dão ~1e-6 no domínio, muito
/// além do que 60fps mostra, e sem o risco de divergência de um Newton perto das pontas.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    // Coordenada de uma bezier cúbica com P0 = 0 e P3 = 1, no parâmetro `u`.
    let axis = |a: f32, b: f32, u: f32| {
        let v = 1.0 - u;
        3.0 * v * v * u * a + 3.0 * v * u * u * b + u * u * u
    };
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..20 {
        let mid = 0.5 * (lo + hi);
        if axis(x1, x2, mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    axis(y1, y2, 0.5 * (lo + hi))
}

// =================================================================================================
// A transição, isolada
// =================================================================================================

/// O relógio da entrada e da saída — o `transition-[opacity,translate] duration-200 ease-in-out` da
/// referência.
///
/// **Um** progresso governa as duas propriedades, e é assim que o CSS faz: as duas transicionam com
/// a mesma duração e a mesma curva, das mesmas `data-starting-style` até o repouso. Guardar dois
/// relógios independentes seria só uma chance de eles dessincronizarem.
///
/// Está num tipo próprio (e não solto no [`Sheet`]) porque é a única parte da abertura que é
/// **pura**: o [`Sheet`] carrega [`FocusHandle`]s, que só existem com uma [`App`] viva, e isso
/// tornaria a máquina de estados intestável sem abrir janela.
#[derive(Clone, Copy, Debug, Default)]
struct Transition {
    /// O alvo: aberto ou fechado.
    open: bool,
    /// De que progresso a transição corrente partiu, e quando. `None` = parado no alvo.
    anim: Option<(f32, Instant)>,
}

impl Transition {
    /// O progresso de repouso do estado corrente: `1` aberto, `0` fechado.
    fn target(&self) -> f32 {
        if self.open {
            1.0
        } else {
            0.0
        }
    }

    /// O progresso VISÍVEL agora, em `[0,1]`. `0` = fora e apagado, `1` = no lugar e opaco.
    ///
    /// Não precisa de limpeza: passado o tempo, `ease_in_out` satura em 1 e a expressão devolve
    /// exatamente o alvo. É por isso que `anim` pode ficar `Some` pra sempre sem consequência.
    fn progress(&self) -> f32 {
        match self.anim {
            None => self.target(),
            Some((from, start)) => {
                let t = start.elapsed().as_secs_f32() / TRANSITION.as_secs_f32();
                from + (self.target() - from) * ease_in_out(t)
            }
        }
    }

    /// A opacidade agora — o `data-*-style:opacity-0` da referência.
    fn opacity(&self) -> f32 {
        self.progress()
    }

    /// O deslocamento agora, em pixels e **com sinal**, no eixo de `side` — o `translate-*-8`.
    ///
    /// Zero em repouso aberto, [`SLIDE`] pra fora em repouso fechado.
    fn offset(&self, side: SheetSide) -> f32 {
        SLIDE * (1.0 - self.progress()) * side.outward()
    }

    /// Troca o alvo, partindo de onde o olho está vendo o painel agora. Devolve `false` se o alvo já
    /// era esse (e então nada acontece — nem evento, nem reinício de animação).
    ///
    /// Partir do valor VISÍVEL, e não de 0/1, é o que faz fechar-e-reabrir no meio da transição não
    /// dar um salto: opacidade e posição continuam de onde estavam.
    fn set(&mut self, open: bool) -> bool {
        if self.open == open {
            return false;
        }
        self.anim = Some((self.progress(), Instant::now()));
        self.open = open;
        true
    }

    /// Se ainda há transição em curso — é o que decide pedir o próximo frame.
    fn animating(&self) -> bool {
        self.anim.is_some_and(|(_, start)| start.elapsed() < TRANSITION)
    }

    /// Se o painel deve estar MONTADO: aberto, ou ainda saindo.
    ///
    /// Fechado e parado, isto é `false` e o [`sheet_layer`] não monta nada: nem pintura, nem
    /// hitbox. Um backdrop invisível que ainda engolisse cliques seria pior que não ter painel.
    fn mounted(&self) -> bool {
        self.open || self.progress() > 0.0
    }

    /// Põe a transição no fim, sem esperar. Só em teste: o progresso vem de [`Instant`] (relógio de
    /// parede), que o executor de teste do GPUI não adianta.
    #[cfg(test)]
    fn settle(&mut self) {
        self.anim = None;
    }

    /// Congela a transição num progresso dado, abrindo. Só em teste — é a única forma de medir o
    /// MEIO da animação, já que o relógio de parede não se adianta.
    ///
    /// O erro é o tempo que o frame de teste leva pra rodar: `ease_in_out` começa com derivada
    /// ~0, então alguns milissegundos valem menos de 0,05px de deslocamento.
    #[cfg(test)]
    fn hold(&mut self, progress: f32) {
        self.open = true;
        self.anim = Some((progress, Instant::now()));
    }
}

// =================================================================================================
// Espaçamento entre as fatias
// =================================================================================================
//
// Idêntico ao do [`crate::dialog`] — o `sheet.tsx` e o `dialog.tsx` têm o MESMO sistema de fatias,
// com os mesmos seletores `:has()`. A referência resolve isto no CSS porque lá a única forma de uma
// fatia saber que existe outra é olhar o pai. Aqui o [`SheetPopup`] é DONO das três fatias, então é
// uma função pura da presença de cada uma — e é testável sem layout.

/// Os respiros que dependem de quais fatias existem.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Spacing {
    /// `p-6`, mas `pb-3` quando existe painel — `in-[…:has([data-slot=sheet-panel])]:pb-3`.
    header_pb: f32,
    /// `p-6`, mas `pt-1` quando existe header — `in-[…:has([data-slot=sheet-header])]:pt-1`.
    panel_pt: f32,
    /// `p-6`, mas `pb-1` quando existe footer SEM linha — `:has([data-slot=sheet-footer]:not(.border-t))`.
    panel_pb: f32,
    /// `py-4` na variante default; `pt-4`, ou `pt-3` depois de um painel, na `bare`.
    footer_pt: f32,
    /// `py-4` na variante default; `pb-6` na `bare`.
    footer_pb: f32,
}

/// Resolve os respiros das três fatias.
///
/// `footer` é `None` quando não há footer. A ordem das fatias não entra na conta: os seletores da
/// referência são todos `:has()`, que não olha posição.
fn spacing(has_header: bool, has_panel: bool, footer: Option<SheetFooterVariant>) -> Spacing {
    let bare = footer == Some(SheetFooterVariant::Bare);
    Spacing {
        header_pb: if has_panel { PAD_SNUG } else { PAD },
        panel_pt: if has_header { PAD_HAIR } else { PAD },
        panel_pb: if bare { PAD_HAIR } else { PAD },
        footer_pt: match footer {
            // A faixa com linha e fundo é simétrica: `py-4`.
            Some(SheetFooterVariant::Default) => FOOTER_PAD_Y,
            // Sem faixa, o footer é só o respiro do painel: encosta no conteúdo (`pt-3`) ou abre o
            // `pt-4` cheio.
            Some(SheetFooterVariant::Bare) if has_panel => PAD_SNUG,
            Some(SheetFooterVariant::Bare) => FOOTER_BARE_PAD_TOP,
            None => 0.0,
        },
        footer_pb: match footer {
            Some(SheetFooterVariant::Default) => FOOTER_PAD_Y,
            Some(SheetFooterVariant::Bare) => PAD,
            None => 0.0,
        },
    }
}

// =================================================================================================
// O estado
// =================================================================================================

/// Evento emitido pelo [`Sheet`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetEvent {
    /// O painel abriu.
    Opened,
    /// O painel começou a fechar — por [`Sheet::close`], por `Escape` ou por clique no backdrop.
    Closed,
}

/// O **estado** de um painel. Ver o doc do módulo.
pub struct Sheet {
    transition: Transition,
    /// O foco do painel. É por ele que o `Escape` chega: no GPUI tecla só vai pra quem tem foco.
    focus_handle: FocusHandle,
    /// O foco do botão de fechar — o [`crate::Button`] exige um handle de fora pra entrar na ordem
    /// de tabulação e acender o anel de `focus-visible`.
    close_focus: FocusHandle,
    /// Quem tinha o foco antes de abrir, pra devolver no fechamento.
    restore_focus: Option<FocusHandle>,
    /// A rolagem do conteúdo. Precisa viver aqui: o [`crate::ScrollArea`] guarda a posição no
    /// handle, e um handle novo por frame zeraria a rolagem.
    scroll: ScrollHandle,
    /// Id estável pra compor os `ElementId` dos filhos.
    entity_id: u64,
    /// Só em teste: a caixa que o layout de verdade deu ao painel. Ver [`Sheet::probe`].
    #[cfg(test)]
    probe: Option<gpui::Bounds<gpui::Pixels>>,
}

impl Sheet {
    /// Um painel fechado.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            transition: Transition::default(),
            focus_handle: cx.focus_handle(),
            close_focus: cx.focus_handle(),
            restore_focus: None,
            scroll: ScrollHandle::new(),
            entity_id: cx.entity_id().as_u64(),
            #[cfg(test)]
            probe: None,
        }
    }

    /// Abre o painel, foca-o e guarda o foco anterior. Idempotente.
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.transition.set(true) {
            return;
        }
        // Guardado ANTES de focar o painel, senão o "anterior" seria o próprio painel.
        self.restore_focus = window.focused(cx);
        window.focus(&self.focus_handle);
        cx.emit(SheetEvent::Opened);
        self.wake(window, cx);
    }

    /// Fecha o painel e devolve o foco a quem o tinha. Idempotente.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.transition.set(false) {
            return;
        }
        if let Some(handle) = self.restore_focus.take() {
            window.focus(&handle);
        }
        cx.emit(SheetEvent::Closed);
        self.wake(window, cx);
    }

    /// Abre se estiver fechado, fecha se estiver aberto.
    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_open(!self.is_open(), window, cx);
    }

    /// Abre ou fecha, por valor.
    pub fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if open {
            self.open(window, cx);
        } else {
            self.close(window, cx);
        }
    }

    /// Se o painel está aberto. Continua `false` durante a animação de SAÍDA — o painel já está indo
    /// embora, e quem pergunta quer saber a intenção, não a pintura.
    pub fn is_open(&self) -> bool {
        self.transition.open
    }

    /// O foco do painel — útil pra quem quer devolver o foco pra cá depois de um desvio.
    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    /// Marca a janela pra redesenhar.
    ///
    /// O `cx.notify()` sozinho não basta: ele suja quem OBSERVA este `Entity`, e a view hospedeira
    /// (que é quem constrói o painel) não observa nada. Ver o doc do módulo.
    fn wake(&self, window: &mut Window, cx: &mut Context<Self>) {
        cx.notify();
        window.refresh();
    }

    /// A caixa que o layout de verdade deu ao painel (a padding box: a superfície recuada pelas
    /// bordas que ela tiver — ver [`border_edges`]).
    ///
    /// ⚠️ Só existe em teste. É a única via de verificar, com layout real, o dimensionamento de cada
    /// lado e o deslocamento da animação.
    #[cfg(test)]
    fn probe(&self) -> Option<gpui::Bounds<gpui::Pixels>> {
        self.probe
    }

    /// Põe a transição no fim. Só em teste — ver [`Transition::settle`].
    #[cfg(test)]
    fn settle(&mut self, cx: &mut Context<Self>) {
        self.transition.settle();
        cx.notify();
    }

    /// Congela a transição num progresso. Só em teste — ver [`Transition::hold`].
    #[cfg(test)]
    fn hold(&mut self, progress: f32, cx: &mut Context<Self>) {
        self.transition.hold(progress);
        cx.notify();
    }
}

impl EventEmitter<SheetEvent> for Sheet {}

impl Focusable for Sheet {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// =================================================================================================
// Header
// =================================================================================================

/// O **header** do painel: título e descrição, `flex flex-col gap-2 p-6`.
///
/// O respiro de baixo aperta pra `pb-3` quando o painel tem conteúdo — quem decide é o
/// [`SheetPopup`], via [`spacing`].
#[derive(Default)]
pub struct SheetHeader {
    title: Option<SharedString>,
    description: Option<SharedString>,
}

impl SheetHeader {
    pub fn new() -> Self {
        Self::default()
    }

    /// O título — `text-xl leading-none font-semibold`.
    ///
    /// ⚠️ A referência também pede `font-heading`, uma FAMÍLIA de fonte que não estava nos tokens
    /// resolvidos. Aqui o título sai na fonte herdada. Ver o doc do módulo.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// A descrição — `text-sm text-muted-foreground`.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Se o header não tem nada pra mostrar. Um header vazio não é renderizado — senão um
    /// `.header(SheetHeader::new())` distraído somaria 48px de padding do nada **e** apertaria o
    /// `pt` do conteúdo pra 4px, deixando-o colado no topo.
    fn is_empty(&self) -> bool {
        self.title.is_none() && self.description.is_none()
    }

    fn render(self, p: &SheetPalette, pad_bottom: f32) -> Div {
        let mut el = div()
            .flex()
            .flex_col()
            // Nem o header nem o footer encolhem quando o painel bate no limite de altura: quem cede
            // é o conteúdo, que é o único que tem pra onde rolar.
            .flex_none()
            .gap(px(GAP))
            .p(px(PAD))
            .pb(px(pad_bottom));

        if let Some(t) = self.title {
            el = el.child(
                div()
                    .text_size(px(TITLE_SIZE))
                    .line_height(px(TITLE_LINE))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(t),
            );
        }
        if let Some(d) = self.description {
            el = el.child(
                div()
                    .text_size(px(DESC_SIZE))
                    .line_height(px(DESC_LINE))
                    .text_color(p.muted_fg.hsla())
                    .child(d),
            );
        }
        el
    }
}

// =================================================================================================
// Footer
// =================================================================================================

/// Como o footer se separa do resto.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SheetFooterVariant {
    /// Faixa com linha de topo e fundo `--muted/72` — o default da referência.
    #[default]
    Default,
    /// Só as ações, sem faixa (`variant="bare"`).
    Bare,
}

/// O **footer** do painel: as ações, alinhadas à direita.
///
/// A referência é `flex flex-col-reverse gap-2 px-6 sm:flex-row sm:justify-end`. Como `sm:` sempre
/// vale nesta base, o que sobra é **linha, alinhada à direita, na ordem declarada** — o
/// `flex-col-reverse` do mobile é código morto aqui.
#[derive(Default)]
pub struct SheetFooter {
    variant: SheetFooterVariant,
    children: Vec<AnyElement>,
}

impl SheetFooter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sem faixa: nem linha de topo, nem fundo (`variant="bare"`).
    pub fn bare(mut self) -> Self {
        self.variant = SheetFooterVariant::Bare;
        self
    }

    /// A variante, por valor.
    pub fn variant(mut self, variant: SheetFooterVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Adiciona uma ação. A ordem de chamada é a ordem na tela, da esquerda pra direita.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Um footer sem ações não vira faixa — senão ele somaria 32px de altura e uma linha
    /// separadora sem nada embaixo.
    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    fn render(self, p: &SheetPalette, sp: Spacing, radius: f32) -> Div {
        let mut el = div()
            .flex()
            .flex_none()
            .justify_end()
            .gap(px(GAP))
            .px(px(PAD))
            .pt(px(sp.footer_pt))
            .pb(px(sp.footer_pb));

        if self.variant == SheetFooterVariant::Default {
            el = el
                .border_t_1()
                .border_color(p.border.hsla())
                .bg(p.footer_bg.hsla());
            // `sm:**:data-[slot=sheet-footer]:rounded-b-[calc(var(--radius-2xl)-1px)]` — só existe
            // onde a superfície tem raio, ou seja na variante `inset`. Ver [`FOOTER_RADIUS`]: a
            // superfície não recorta filhos, então a faixa arredonda as próprias quinas de baixo.
            if radius > 0.0 {
                el = el
                    .rounded_bl(px(FOOTER_RADIUS))
                    .rounded_br(px(FOOTER_RADIUS));
            }
        }

        for c in self.children {
            el = el.child(c);
        }
        el
    }
}

// =================================================================================================
// Popup
// =================================================================================================

/// O **painel**: a superfície do sheet, com as três fatias opcionais.
///
/// É construído pelo chamador a cada frame e passado pro [`sheet_layer`] — ele não guarda estado (os
/// `AnyElement` das ações e do conteúdo são de uso único).
pub struct SheetPopup {
    side: SheetSide,
    variant: SheetVariant,
    header: Option<SheetHeader>,
    panel: Option<AnyElement>,
    panel_fade: bool,
    footer: Option<SheetFooter>,
    show_close: bool,
    max_width: f32,
}

/// ⚠️ `Default` delega pro [`SheetPopup::new`] **de propósito**, e não é `derive`: um `derive` daria
/// `max_width = 0`, `show_close = false` e `panel_fade = false` — um painel de largura zero, em
/// silêncio.
impl Default for SheetPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl SheetPopup {
    /// Um painel vazio na direita (o default da referência é `side = "right"`), com o botão de
    /// fechar (`showCloseButton = true`).
    pub fn new() -> Self {
        Self {
            side: SheetSide::default(),
            variant: SheetVariant::default(),
            header: None,
            panel: None,
            // `scrollFade = true` é o default do `SheetPanel` da referência.
            panel_fade: true,
            footer: None,
            show_close: true,
            max_width: MAX_WIDTH,
        }
    }

    /// Por qual borda o painel entra. Default [`SheetSide::Right`].
    pub fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    /// Encostado (default) ou afastado das bordas ([`SheetVariant::Inset`]).
    pub fn variant(mut self, variant: SheetVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Atalho pra [`SheetVariant::Inset`].
    pub fn inset(mut self) -> Self {
        self.variant = SheetVariant::Inset;
        self
    }

    /// O header (título e descrição).
    pub fn header(mut self, header: SheetHeader) -> Self {
        self.header = Some(header);
        self
    }

    /// O corpo do painel. Vai dentro de um [`crate::ScrollArea`], como na referência: se o painel
    /// bater no limite de tamanho, é ESTE bloco que rola — o header e o footer ficam.
    pub fn panel(mut self, panel: impl IntoElement) -> Self {
        self.panel = Some(panel.into_any_element());
        self
    }

    /// Liga/desliga o fade nas bordas do conteúdo (`scrollFade`). Default ligado.
    pub fn panel_scroll_fade(mut self, fade: bool) -> Self {
        self.panel_fade = fade;
        self
    }

    /// O footer (as ações).
    pub fn footer(mut self, footer: SheetFooter) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Mostra ou esconde o **X** do canto (`showCloseButton`). Default: mostra.
    pub fn show_close_button(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    /// Largura máxima do painel nos lados **horizontais** (`max-w-md` = 448px por default). A
    /// largura efetiva é `min(essa, largura do viewport − 48 − o respiro da variante)`.
    ///
    /// ⚠️ **Não tem efeito nos lados `Top`/`Bottom`**, onde a referência não põe limite nenhum: lá o
    /// painel tem a largura cheia e a altura do conteúdo.
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = max_width;
        self
    }
}

/// O bisel de 1px sobreposto — o `before:shadow-[0_1px_…]` da referência.
///
/// ⚠️ As quatro coisas que já custaram bug visível nesta base (ver [`crate::card::Card::bevel`],
/// [`crate::frame`], [`crate::toast`] e [`crate::dialog`]):
///
/// 1. **É borda, não sombra.** O [`gpui::Window::paint_shadows`] não recorta a sombra pra fora do
///    elemento que a projeta (o CSS recorta), então o `before:box-shadow` viraria uma lavagem de cor
///    sobre o painel inteiro.
/// 2. **Cobre a BORDER box** (`inset: -1px`), porque no CSS a sombra do pseudo-elemento sai 1px pra
///    fora da padding box e cai sobre a borda.
/// 3. **O raio é o da SUPERFÍCIE**, não `raio − 1`: o nosso overlay está uma caixa pra FORA. (Na
///    variante `Default` os dois são zero, porque não há raio nenhum.)
/// 4. **A direção sai do SINAL do deslocamento.** `0 1px` (claro) desenha na BASE; `0 -1px`
///    (escuro) desenha no TOPO.
///
/// Consequência de (2) neste componente: num painel de largura/altura cheia o filete cai FORA da
/// janela e não se vê — no CSS também. Ver o doc do módulo.
fn bevel_overlay(radius: f32) -> Div {
    let p = palette();
    let overlay = div()
        .absolute()
        .top(px(-1.0))
        .left(px(-1.0))
        .right(px(-1.0))
        .bottom(px(-1.0))
        .rounded(px(radius))
        .border_color(p.bevel.hsla());
    if p.bevel_dir > 0.0 {
        overlay.border_b_1()
    } else {
        overlay.border_t_1()
    }
}

/// As duas sombras do `shadow-lg/5`.
///
/// O `--shadow-lg` do Tailwind é `0 10px 15px -3px black/10, 0 4px 6px -4px black/10`, e o
/// modificador `/5` troca o alfa das duas por 5%. É sombra EXTERNA sobre fundo OPACO (`bg-popover`),
/// que é o caso em que a técnica do GPUI funciona.
fn surface_shadows() -> Vec<gpui::BoxShadow> {
    let cor = palette().shadow.hsla();
    vec![
        gpui::BoxShadow {
            color: cor,
            offset: gpui::point(px(0.0), px(10.0)),
            blur_radius: px(15.0),
            spread_radius: px(-3.0),
        },
        gpui::BoxShadow {
            color: cor,
            offset: gpui::point(px(0.0), px(4.0)),
            blur_radius: px(6.0),
            spread_radius: px(-4.0),
        },
    ]
}

// =================================================================================================
// O elemento montável
// =================================================================================================

/// O overlay que carrega o backdrop e o painel. **Monte-o como último filho de uma raiz
/// `relative()`** — ver o exemplo no doc do módulo.
///
/// Existe porque o GPUI não tem portal nem `position: fixed`. Quando o painel está fechado (e a
/// animação de saída já acabou) ele não monta nada: nem pintura, nem hitbox.
pub fn sheet_layer(sheet: &gpui::Entity<Sheet>, popup: SheetPopup) -> impl IntoElement {
    SheetLayer {
        sheet: sheet.clone(),
        popup,
    }
}

/// O elemento que [`sheet_layer`] devolve.
#[derive(IntoElement)]
struct SheetLayer {
    sheet: gpui::Entity<Sheet>,
    popup: SheetPopup,
}

/// O que o painel precisa do [`Sheet`], lido de uma vez.
///
/// Existe pra o empréstimo do `Entity` terminar antes de montar os elementos (os handlers precisam
/// de `&mut App`) — e pra [`SheetPopup::render`] não virar uma função de dez parâmetros.
struct Chrome {
    sheet: gpui::Entity<Sheet>,
    /// O foco do painel — é daqui que o `Escape` sai.
    focus: FocusHandle,
    /// O foco do botão de fechar.
    close_focus: FocusHandle,
    /// A posição de rolagem do conteúdo.
    scroll: ScrollHandle,
    /// Id estável pros `ElementId` dos filhos.
    entity_id: u64,
}

impl RenderOnce for SheetLayer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette();
        let side = self.popup.side;
        let variant = self.popup.variant;

        // Tudo o que o estado tem a dizer, copiado de uma vez: o empréstimo morre aqui, e os
        // handlers abaixo podem tocar no `Entity` sem conflito.
        let (montado, animando, opacidade, desloc, chrome) = {
            let s = self.sheet.read(cx);
            (
                s.transition.mounted(),
                s.transition.animating(),
                s.transition.opacity(),
                s.transition.offset(side),
                Chrome {
                    sheet: self.sheet.clone(),
                    focus: s.focus_handle.clone(),
                    close_focus: s.close_focus.clone(),
                    scroll: s.scroll.clone(),
                    entity_id: s.entity_id,
                },
            )
        };

        // A raiz é ABSOLUTA nos dois ramos: assim o layer nunca entra no fluxo da view hospedeira —
        // um filho em fluxo somaria um `gap` do container dela, mesmo vazio.
        let raiz = div().absolute().top_0().left_0();
        if !montado {
            // A sonda de teste vira `None` junto: é assim que o teste distingue "desmontou" de
            // "sobrou a medida do último frame em que estava aberto".
            #[cfg(test)]
            self.sheet.update(cx, |s, _| s.probe = None);
            return raiz.w_0().h_0();
        }

        // --- Backdrop -------------------------------------------------------------------------
        //
        // `fixed inset-0 bg-black/32 backdrop-blur-sm transition-all duration-200`. O blur não
        // existe no GPUI (ver o doc do módulo); o `bg-black/32` vai sozinho.
        //
        // `occlude()` é o que dá a MODALIDADE: sem ele o clique atravessaria o backdrop e chegaria
        // na app atrás (no GPUI o hit test acumula todos os hitboxes sob o cursor, não só o de cima).
        let fechar_por_backdrop = self.sheet.clone();
        let backdrop = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(p.backdrop.hsla())
            .opacity(opacidade)
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_e: &MouseDownEvent, window, cx| {
                fechar_por_backdrop.update(cx, |s, cx| s.close(window, cx));
            });

        // --- Viewport -------------------------------------------------------------------------
        //
        // `fixed inset-0 grid` + o layout do lado. O `grid grid-rows-[1fr_auto]` de um painel de
        // baixo é uma coluna com o painel encostado no fim; o `flex justify-end` de um painel da
        // direita é uma linha com ele encostado no fim. Os respiros (incluindo a faixa de 48px que
        // o painel nunca cobre) saem de [`viewport_pads`].
        let [pt, pr, pb, pl] = viewport_pads(side, variant);
        let viewport = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .pt(px(pt))
            .pr(px(pr))
            .pb(px(pb))
            .pl(px(pl));
        let viewport = match side {
            SheetSide::Top => viewport.flex_col().justify_start(),
            SheetSide::Bottom => viewport.flex_col().justify_end(),
            SheetSide::Left => viewport.flex_row().justify_start(),
            SheetSide::Right => viewport.flex_row().justify_end(),
        }
        .child(self.popup.render(p, opacidade, desloc, &chrome));

        // Enquanto a transição andar, pede o próximo frame — é o motor da animação, já que não há
        // elemento de animação e o progresso vem do tempo decorrido.
        //
        // ⚠️ `on_next_frame` + `refresh`, e NÃO `window.request_animation_frame()`: o
        // `request_animation_frame` notifica a view CORRENTE, e a view corrente aqui é quem quer que
        // esteja montando o layer — o painel é reconstruído pelo `render` dela, não por um `Entity`
        // nosso. Sujar a janela inteira não depende de acertar qual view é essa, e é o que o painel
        // precisa de qualquer forma (o backdrop cobre a janela toda). O `refresh` vai DENTRO do
        // callback porque, durante o desenho, ele é um no-op de propósito.
        if animando {
            window.on_next_frame(|window, _cx| window.refresh());
        }

        raiz.size_full().child(backdrop).child(viewport)
    }
}

impl SheetPopup {
    /// A superfície do painel, com as fatias que existirem.
    ///
    /// `desloc` é o deslocamento da animação, com sinal, no eixo do lado.
    fn render(self, p: &SheetPalette, opacidade: f32, desloc: f32, chrome: &Chrome) -> Div {
        let side = self.side;
        let variant = self.variant;
        let radius = variant.radius();
        let header = self.header.filter(|h| !h.is_empty());
        let footer = self.footer.filter(|f| !f.is_empty());
        let sp = spacing(
            header.is_some(),
            self.panel.is_some(),
            footer.as_ref().map(|f| f.variant),
        );

        // `relative flex max-h-full min-h-0 w-full min-w-0 flex-col bg-popover
        //  text-popover-foreground shadow-lg/5` + o que o lado e a variante mandam.
        //
        // ⚠️ SEM `overflow_hidden`: ele recortaria o overlay de bisel, que vive em `inset:-1px`
        // (armadilha conhecida — ver [`bevel_overlay`]). É por isso que quem arredonda as quinas de
        // baixo é a faixa de footer, e não um recorte da superfície.
        let fechar_por_tecla = chrome.sheet.clone();
        let [bt, br, bb, bl] = border_edges(side, variant);
        let mut surface = div()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .min_w(px(0.0))
            // `max-h-full` + `min-h-0`: o painel nunca passa da altura útil do viewport, e pode
            // encolher abaixo do conteúdo (é o que deixa o conteúdo rolar em vez de vazar). Nos
            // lados horizontais a altura vem esticada do viewport, e este `max-h` é inerte.
            .max_h(relative(1.0))
            .min_h(px(0.0))
            .rounded(px(radius))
            // A(s) borda(s): uma só na variante default, a do lado de dentro. Ver
            // [`border_edges`] — é ela que dirige, pra o teste de janela medir a mesma caixa.
            .border_t(px(bt))
            .border_r(px(br))
            .border_b(px(bb))
            .border_l(px(bl))
            .border_color(p.border.hsla())
            .bg(p.popover.hsla())
            .text_color(p.popover_fg.hsla())
            .shadow(surface_shadows())
            .opacity(opacidade)
            // O foco vive AQUI porque é daqui que o `Escape` sai: no GPUI tecla só chega a quem tem
            // foco, e o `Sheet::open` foca este handle.
            .track_focus(&chrome.focus)
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                if e.keystroke.key == "escape" {
                    fechar_por_tecla.update(cx, |s, cx| s.close(window, cx));
                }
            })
            // Sem isto, um clique NO painel também acertaria o hitbox do backdrop (que está atrás) e
            // fecharia o painel.
            .occlude();

        // O `translate-*-8` da referência, por GEOMETRIA: um `inset` de posicionamento RELATIVO no
        // eixo do lado. Não há `transform` em `div` no GPUI (o `TransformationMatrix` só serve pra
        // `svg`/imagem), mas um inset relativo desloca a caixa DEPOIS do layout — não muda tamanho
        // nenhum e não reflui o texto, que é exatamente o que `translate` faz no CSS.
        //
        // Em repouso aberto `desloc` é 0, e um inset de 0 é o mesmo que não ter inset.
        surface = if side.horizontal() {
            surface.left(px(desloc))
        } else {
            surface.top(px(desloc))
        };

        // `max-w-md` — só nos lados horizontais, que são os únicos em que a referência limita a
        // largura. Nos verticais o painel tem a largura cheia.
        if side.horizontal() {
            surface = surface.max_w(px(self.max_width));
        }

        if let Some(h) = header {
            surface = surface.child(h.render(p, sp.header_pb));
        }

        if let Some(panel) = self.panel {
            // O `<ScrollArea overscrollContain scrollFade>` da referência. O `min-h-0` é o que
            // permite ele ceder quando o painel bate no limite — sem isso o mínimo automático do
            // flex seria a altura do conteúdo e o painel estouraria a janela.
            surface = surface.child(
                crate::ScrollArea::new(("sheet-panel", chrome.entity_id), &chrome.scroll)
                    .fade(self.panel_fade)
                    // O fade é um gradiente SOBREPOSTO, não uma máscara: a cor tem que ser a que
                    // está atrás do conteúdo, que aqui é `--popover` (e não o `--background` que o
                    // `ScrollArea` assume por default).
                    .fade_color(p.popover.hsla())
                    // Uma borda pra dentro do raio da superfície, que é onde o conteúdo de fato
                    // vive. Na variante default isto é 0, porque não há raio.
                    .radius((radius - 1.0).max(0.0))
                    .min_h(px(0.0))
                    .child(
                        div()
                            .px(px(PAD))
                            .pt(px(sp.panel_pt))
                            .pb(px(sp.panel_pb))
                            .child(panel),
                    ),
            );
        }

        if let Some(f) = footer {
            surface = surface.child(f.render(p, sp, radius));
        }

        // O bisel ANTES do botão de fechar: os dois são absolutos, então a ordem aqui só decide
        // quem pinta em cima — e no CSS o `::before` é o primeiro filho, logo fica EMBAIXO de
        // qualquer filho posicionado que venha depois.
        //
        // ⚠️ Na variante `inset` não há bisel: `before:hidden`, e a referência nunca o devolve.
        if variant.has_bevel() {
            surface = surface.child(bevel_overlay(radius));
        }

        if self.show_close {
            // `absolute end-2 top-2` + `<Button size="icon" variant="ghost">` com o `XIcon` do
            // lucide. O `ButtonSize::Icon` daqui é 32×32, igual ao `size-8` do coss; o ícone do
            // iconoir equivalente ao `XIcon` é o `xmark`.
            let fechar_por_botao = chrome.sheet.clone();
            surface = surface.child(
                div()
                    .absolute()
                    .top(px(CLOSE_INSET))
                    .right(px(CLOSE_INSET))
                    .child(
                        crate::Button::icon(
                            ("sheet-close", chrome.entity_id),
                            "iconoir/regular/xmark.svg",
                        )
                        .variant(crate::ButtonVariant::Ghost)
                        .focus(&chrome.close_focus)
                        .on_click(move |_e, window, cx| {
                            fechar_por_botao.update(cx, |s, cx| s.close(window, cx));
                        }),
                    ),
            );
        }

        // A sonda de teste é um `canvas` que só mede e não pinta. Mede a PADDING box (a superfície
        // menos as bordas que ela tiver), que é o que um filho absoluto em `inset:0` cobre.
        #[cfg(test)]
        {
            surface = surface.child(probe(&chrome.sheet));
        }

        surface
    }
}

/// Um `canvas` overlay que grava a caixa do painel e não pinta nada. Ver [`Sheet::probe`].
#[cfg(test)]
fn probe(sheet: &gpui::Entity<Sheet>) -> impl IntoElement {
    let sheet = sheet.clone();
    gpui::canvas(
        move |bounds: gpui::Bounds<gpui::Pixels>, _window, cx| {
            sheet.update(cx, |s, _| s.probe = Some(bounds));
        },
        |_, _, _, _| {},
    )
    // `inset:0` explícito: um absoluto de insets `auto` cairia na posição estática (dentro do
    // padding) e mediria menos. Mesmo truque do `crate::dialog` e do `crate::tabs`.
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

// =================================================================================================
// Testes
// =================================================================================================

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável ("this assertion has a constant
// value"), presumindo que quem escreveu quis testar algo variável. Aqui é o contrário: travar o valor
// que veio da referência É o propósito destes testes.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// As duas paletas, com nome pras mensagens de falha.
    const PALETAS: [(&str, &SheetPalette); 2] = [("claro", &SHEET_LIGHT), ("escuro", &SHEET_DARK)];

    /// Os quatro lados, pros testes de tabela.
    const LADOS: [SheetSide; 4] = [
        SheetSide::Top,
        SheetSide::Right,
        SheetSide::Bottom,
        SheetSide::Left,
    ];

    // --- Paleta -----------------------------------------------------------------------------

    /// **A convenção de cor, decodificada de verdade.**
    ///
    /// Todo valor é `0xRRGGBBAA`; um valor de 6 dígitos esquecido aqui vira uma cor completamente
    /// diferente **sem erro de compilação** — `rgba(0xffffff)` é lido como `0x00FFFFFF`, ou seja
    /// ciano. Já aconteceu três vezes nesta base.
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        for (tema, p) in PALETAS {
            // Os neutros opacos: se algum sair colorido, o valor foi lido deslocado.
            for (nome, c) in [
                ("popover", p.popover),
                ("popover-foreground", p.popover_fg),
                ("muted-foreground", p.muted_fg),
            ] {
                let c: gpui::Rgba = c.hsla().into();
                assert_eq!(c.a, 1.0, "{tema}: {nome} tem que ser opaco");
                assert!(
                    (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                    "{tema}: {nome} tem que ser neutro — se r≠g≠b, o valor foi lido deslocado"
                );
            }
            // Borda, bisel, fundo do footer, sombra e backdrop são TRANSLÚCIDOS: é isso que os faz
            // funcionar sobre qualquer fundo.
            for (nome, c) in [
                ("border", p.border),
                ("bevel", p.bevel),
                ("footer-bg", p.footer_bg),
                ("shadow", p.shadow),
                ("backdrop", p.backdrop),
            ] {
                assert!(c.alpha() < 1.0, "{tema}: {nome} tem que ser translúcido");
                assert!(c.alpha() > 0.0, "{tema}: {nome} não pode ser invisível");
            }
        }

        // O tema claro tem popover branco; o escuro, quase preto.
        let claro: gpui::Rgba = SHEET_LIGHT.popover.hsla().into();
        let escuro: gpui::Rgba = SHEET_DARK.popover.hsla().into();
        assert_eq!((claro.r, claro.g, claro.b), (1.0, 1.0, 1.0));
        assert!(escuro.r < 0.2);

        // E o texto acompanha, invertido.
        let txt_claro: gpui::Rgba = SHEET_LIGHT.popover_fg.hsla().into();
        let txt_escuro: gpui::Rgba = SHEET_DARK.popover_fg.hsla().into();
        assert!(txt_claro.r < 0.2, "texto escuro sobre popover claro");
        assert!(txt_escuro.r > 0.8, "texto claro sobre popover escuro");
    }

    /// **O backdrop é `bg-black/32`, preto literal e IGUAL nos dois temas.**
    ///
    /// `bg-black/32` não é um token, então não muda com o tema. Tratar o backdrop como "o fundo do
    /// tema com alfa" daria um véu quase branco no tema claro.
    #[test]
    fn backdrop_e_preto_32_nos_dois_temas() {
        assert_eq!(SHEET_LIGHT.backdrop, SHEET_DARK.backdrop);
        let c: gpui::Rgba = SHEET_LIGHT.backdrop.hsla().into();
        assert_eq!((c.r, c.g, c.b), (0.0, 0.0, 0.0), "preto puro");
        // 32% em 8 bits = 82 (0x52). A tabela de alfas desta base: 32% = 0x52.
        assert!(
            (c.a - 82.0 / 255.0).abs() < 1e-4,
            "alfa tem que ser 32%; veio {}",
            c.a
        );
    }

    /// **O fundo do footer é `--muted` com o alfa MULTIPLICADO por 72%**, e não `--muted` cheio.
    #[test]
    fn fundo_do_footer_e_muted_a_72_por_cento() {
        for (tema, p) in PALETAS {
            // `--muted` é 4% nos dois temas; × 0,72 ≈ 2,9%.
            let esperado = 0.04 * 0.72;
            let veio = p.footer_bg.alpha();
            assert!(
                (veio - esperado).abs() < 0.005,
                "{tema}: footer-bg deveria ser ~{esperado:.4}; veio {veio:.4}"
            );
            assert!(
                veio < p.border.alpha(),
                "{tema}: a faixa é MAIS SUTIL que a linha de topo dela"
            );
        }
    }

    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, porque a 6% o filete é
    /// imperceptível no nosso fundo. É a MESMA decisão já vigente no `input`, `card`, `button`,
    /// `frame`, `toast` e `dialog`; este teste existe pra o desvio ser uma decisão registrada e não
    /// uma deriva. O tema CLARO segue fiel (preto 4%).
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = SHEET_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%); veio {claro}"
        );

        let escuro = SHEET_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");

        // Ligados em `let` pra o clippy não tratar as asserções como constantes: o que interessa é o
        // SENTIDO — `0 1px` desce (base), `0 -1px` sobe (topo).
        let (dir_claro, dir_escuro) = (SHEET_LIGHT.bevel_dir, SHEET_DARK.bevel_dir);
        assert!(dir_claro > 0.0, "no claro o filete DESCE (border_b)");
        assert!(dir_escuro < 0.0, "no escuro o filete SOBE (border_t)");
    }

    // --- Geometria --------------------------------------------------------------------------

    /// **A variante `Default` não tem raio; a `inset` tem `rounded-2xl`.**
    ///
    /// É a diferença mais fácil de portar errado a partir do `dialog`, que é arredondado sempre: um
    /// painel encostado na borda da janela com quinas redondas nas costas fica visivelmente errado.
    #[test]
    fn so_a_variante_inset_tem_raio() {
        assert_eq!(SheetVariant::Default.radius(), 0.0, "sem `rounded-*`");
        assert_eq!(SheetVariant::Inset.radius(), RADIUS, "sm:rounded-2xl");
        assert_eq!(RADIUS, 16.0, "o coss não redefine --radius-2xl");
        assert_eq!(FOOTER_RADIUS, 15.0, "calc(var(--radius-2xl) - 1px)");
        assert!(FOOTER_RADIUS < RADIUS);
    }

    /// **E é a `Default` que tem bisel, não a `inset`** — o contrário do que a intuição diz.
    ///
    /// A referência põe `before:hidden` na variante `inset` e nunca o devolve (o
    /// `sm:before:rounded-…` que vem depois só mexe no raio). Fielmente reproduzido.
    #[test]
    fn a_variante_inset_nao_tem_bisel() {
        assert!(SheetVariant::Default.has_bevel());
        assert!(
            !SheetVariant::Inset.has_bevel(),
            "`before:hidden` sem nada que o devolva"
        );
    }

    /// Os respiros vêm todos da escala do Tailwind (1 unidade = 4px), e a relação entre eles é o que
    /// importa: a fatia que encosta noutra aperta, e o conteúdo encosta com um fio.
    #[test]
    fn respiros_seguem_a_escala_do_tailwind() {
        assert_eq!(PAD, 24.0, "p-6");
        assert_eq!(PAD_SNUG, 12.0, "pb-3 / pt-3");
        assert_eq!(PAD_HAIR, 4.0, "pt-1 / pb-1");
        assert_eq!(FOOTER_PAD_Y, 16.0, "py-4");
        assert_eq!(GAP, 8.0, "gap-2");
        assert_eq!(CLOSE_INSET, 8.0, "top-2 end-2");
        assert!(PAD_HAIR < PAD_SNUG && PAD_SNUG < PAD);

        assert_eq!(OUTER_GAP, 48.0, "--spacing(12) / pt-12 / pb-12");
        assert_eq!(INSET_PAD, 16.0, "sm:p-4");
        assert_eq!(SLIDE, 32.0, "translate-x-8 / translate-y-8");
        assert_eq!(MAX_WIDTH, 448.0, "max-w-md = 28rem (NÃO o max-w-lg do dialog)");

        // O par do `text-xl leading-none`: corpo e entrelinha IGUAIS. Se alguém tirar o
        // `line_height`, a entrelinha default do GPUI engorda o header (o achado do `frame`).
        assert_eq!(TITLE_SIZE, 20.0, "text-xl");
        assert_eq!(TITLE_LINE, TITLE_SIZE, "leading-none");
        // E o par do `text-sm`: 14/20.
        assert_eq!((DESC_SIZE, DESC_LINE), (14.0, 20.0));
    }

    /// **A faixa de 48px fica sempre do lado OPOSTO ao do painel**, e é a única coisa que garante
    /// que a app continua aparecendo atrás.
    #[test]
    fn a_faixa_de_48px_fica_do_lado_oposto() {
        // [topo, direita, base, esquerda]
        assert_eq!(viewport_pads(SheetSide::Top, SheetVariant::Default), [0.0, 0.0, OUTER_GAP, 0.0]);
        assert_eq!(viewport_pads(SheetSide::Right, SheetVariant::Default), [0.0, 0.0, 0.0, OUTER_GAP]);
        assert_eq!(viewport_pads(SheetSide::Bottom, SheetVariant::Default), [OUTER_GAP, 0.0, 0.0, 0.0]);
        assert_eq!(viewport_pads(SheetSide::Left, SheetVariant::Default), [0.0, OUTER_GAP, 0.0, 0.0]);
    }

    /// **A assimetria do `inset`, que vem do arquivo e não de mim.**
    ///
    /// Nos lados horizontais os 48px são LARGURA (`calc(100% - --spacing(12))`, uma porcentagem do
    /// content box já descontado o `p-4`), então SOMAM com o `p-4` → 64. Nos verticais são
    /// `pt-12`/`pb-12`, utilitários de um lado, que no Tailwind VENCEM o `p-*` → 48, não 64.
    #[test]
    fn no_inset_o_p4_soma_no_horizontal_e_perde_no_vertical() {
        let dir = viewport_pads(SheetSide::Right, SheetVariant::Inset);
        assert_eq!(dir, [INSET_PAD, INSET_PAD, INSET_PAD, INSET_PAD + OUTER_GAP]);

        let base = viewport_pads(SheetSide::Bottom, SheetVariant::Inset);
        assert_eq!(base, [OUTER_GAP, INSET_PAD, INSET_PAD, INSET_PAD]);
        assert!(
            base[0] < INSET_PAD + OUTER_GAP,
            "o pt-12 VENCE o p-4; não soma"
        );

        // E em toda combinação a faixa oposta continua garantindo os 48px.
        for side in LADOS {
            for variant in [SheetVariant::Default, SheetVariant::Inset] {
                let e = viewport_pads(side, variant);
                let oposto = match side {
                    SheetSide::Top => e[2],
                    SheetSide::Right => e[3],
                    SheetSide::Bottom => e[0],
                    SheetSide::Left => e[1],
                };
                assert!(
                    oposto >= OUTER_GAP,
                    "{side:?}/{variant:?}: a faixa oposta é {oposto}, menos que {OUTER_GAP}"
                );
            }
        }
    }

    /// **Uma borda só na variante `Default`, e é a do lado de DENTRO.**
    ///
    /// `border-s` num painel da direita é a borda ESQUERDA (LTR), e é a única que se vê: as outras
    /// três estão encostadas na borda da janela. Trocar o lado aqui desenharia um fio no lugar
    /// errado.
    #[test]
    fn a_borda_do_default_e_a_do_lado_de_dentro() {
        // [topo, direita, base, esquerda]
        assert_eq!(border_edges(SheetSide::Right, SheetVariant::Default), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(border_edges(SheetSide::Left, SheetVariant::Default), [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(border_edges(SheetSide::Bottom, SheetVariant::Default), [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(border_edges(SheetSide::Top, SheetVariant::Default), [0.0, 0.0, 1.0, 0.0]);

        // Sempre exatamente UMA borda no default…
        for side in LADOS {
            let e = border_edges(side, SheetVariant::Default);
            assert_eq!(e.iter().sum::<f32>(), 1.0, "{side:?}: uma borda só");
            // …e ela é oposta ao lado em que o painel encosta.
            let indice = match side {
                SheetSide::Top => 2,
                SheetSide::Right => 3,
                SheetSide::Bottom => 0,
                SheetSide::Left => 1,
            };
            assert_eq!(e[indice], 1.0, "{side:?}: a borda é a do lado de dentro");
            // …e as quatro no inset (`sm:border`).
            assert_eq!(border_edges(side, SheetVariant::Inset), [1.0; 4]);
        }
    }

    /// O eixo de cada lado: os horizontais têm largura própria e deslizam em X.
    #[test]
    fn o_eixo_sai_do_lado() {
        assert!(SheetSide::Left.horizontal() && SheetSide::Right.horizontal());
        assert!(!SheetSide::Top.horizontal() && !SheetSide::Bottom.horizontal());

        // O sinal do "pra fora": direita/baixo crescem, esquerda/topo decrescem.
        assert_eq!(SheetSide::Right.outward(), 1.0);
        assert_eq!(SheetSide::Bottom.outward(), 1.0);
        assert_eq!(SheetSide::Left.outward(), -1.0);
        assert_eq!(SheetSide::Top.outward(), -1.0);
    }

    // --- Espaçamento entre fatias -----------------------------------------------------------

    /// **A cadeia de respiros do caso completo**: header + conteúdo + footer `bare`.
    ///
    /// 12 do header + 4 do conteúdo = 16px entre o texto e o corpo; 4 do corpo + 12 do footer =
    /// 16px entre o corpo e as ações. Os dois `16` são o respiro real, e é por isso que o conteúdo
    /// encosta com um fio em vez de somar dois `p-6`.
    #[test]
    fn header_conteudo_e_footer_bare_somam_16px_em_cada_junta() {
        let sp = spacing(true, true, Some(SheetFooterVariant::Bare));
        assert_eq!(sp.header_pb, PAD_SNUG);
        assert_eq!(sp.panel_pt, PAD_HAIR);
        assert_eq!(sp.panel_pb, PAD_HAIR);
        assert_eq!(sp.footer_pt, PAD_SNUG);
        assert_eq!(sp.footer_pb, PAD);

        assert_eq!(sp.header_pb + sp.panel_pt, 16.0, "header → conteúdo");
        assert_eq!(sp.panel_pb + sp.footer_pt, 16.0, "conteúdo → footer");
    }

    /// Com o footer DEFAULT (faixa com linha e fundo), o conteúdo volta ao `pb-6`: a faixa tem fundo
    /// próprio, então o respiro pertence ao conteúdo e não pode ser um fio de 4px.
    #[test]
    fn footer_com_faixa_devolve_o_pb_cheio_ao_conteudo() {
        let sp = spacing(true, true, Some(SheetFooterVariant::Default));
        assert_eq!(sp.panel_pb, PAD, "o `:not(.border-t)` não casa");
        assert_eq!(
            (sp.footer_pt, sp.footer_pb),
            (FOOTER_PAD_Y, FOOTER_PAD_Y),
            "py-4, simétrico"
        );
    }

    /// Sem conteúdo, o header volta ao `p-6` em todos os lados.
    #[test]
    fn sem_conteudo_o_header_nao_aperta() {
        assert_eq!(
            spacing(true, false, Some(SheetFooterVariant::Default)).header_pb,
            PAD
        );
        assert_eq!(spacing(true, false, None).header_pb, PAD);
    }

    /// Sem header, o conteúdo abre o `pt-6` cheio: o `pt-1` é o fio que encosta no header.
    #[test]
    fn sem_header_o_conteudo_abre_o_topo() {
        let sp = spacing(false, true, None);
        assert_eq!(sp.panel_pt, PAD);
        assert_eq!(sp.panel_pb, PAD, "e sem footer bare, o de baixo também");
    }

    /// O footer `bare` só encosta (`pt-3`) quando há conteúdo; sozinho ele abre o `pt-4`.
    #[test]
    fn footer_bare_encosta_apenas_no_conteudo() {
        assert_eq!(
            spacing(true, true, Some(SheetFooterVariant::Bare)).footer_pt,
            PAD_SNUG
        );
        assert_eq!(
            spacing(true, false, Some(SheetFooterVariant::Bare)).footer_pt,
            FOOTER_BARE_PAD_TOP
        );
    }

    /// Sem footer, os respiros dele são zero — e nada mais muda.
    #[test]
    fn sem_footer_nao_ha_respiro_de_footer() {
        let sp = spacing(true, true, None);
        assert_eq!((sp.footer_pt, sp.footer_pb), (0.0, 0.0));
        assert_eq!(sp.panel_pb, PAD, "e o conteúdo mantém o `pb-6`");
    }

    // --- Fatias vazias ----------------------------------------------------------------------

    /// Uma fatia sem nada dentro não vira fatia — senão um `.header(SheetHeader::new())` distraído
    /// somaria 48px de padding do nada **e** (pior) faria o conteúdo achar que há header e apertar o
    /// `pt` pra 4px.
    #[test]
    fn fatias_vazias_sao_detectadas() {
        assert!(SheetHeader::new().is_empty());
        assert!(!SheetHeader::new().title("x").is_empty());
        assert!(!SheetHeader::new().description("x").is_empty());

        assert!(SheetFooter::new().is_empty());
        assert!(!SheetFooter::new().child(div()).is_empty());
        assert!(
            SheetFooter::new().bare().is_empty(),
            "a variante não enche o footer"
        );
    }

    /// Os defaults seguem a referência: `side = "right"`, `variant = "default"`,
    /// `showCloseButton` e `scrollFade` ligados, largura `max-w-md`.
    #[test]
    fn defaults_do_popup_seguem_a_referencia() {
        let popup = SheetPopup::new();
        assert_eq!(popup.side, SheetSide::Right, "side = right");
        assert_eq!(popup.variant, SheetVariant::Default);
        assert!(popup.show_close, "showCloseButton = true");
        assert!(popup.panel_fade, "scrollFade = true");
        assert_eq!(popup.max_width, MAX_WIDTH);
        assert_eq!(
            SheetFooter::new().variant,
            SheetFooterVariant::Default,
            "variant = default"
        );
        assert_eq!(
            SheetPopup::new().inset().variant,
            SheetVariant::Inset,
            "o atalho"
        );

        // E o `Default` é o `new`, não um `derive` que daria largura zero em silêncio.
        let d = SheetPopup::default();
        assert_eq!(d.max_width, MAX_WIDTH);
        assert!(d.show_close && d.panel_fade);
        assert_eq!(d.side, SheetSide::Right);
    }

    /// O ícone do X existe de verdade no iconoir embutido. Um caminho errado não dá erro de
    /// compilação — o glifo só some silenciosamente do botão.
    #[test]
    fn o_x_de_fechar_existe_no_iconoir() {
        assert!(crate::iconoir::has("iconoir/regular/xmark.svg"));
    }

    // --- Curva ------------------------------------------------------------------------------

    /// A bezier da referência é o `ease-in-out` do CSS: passa pelas pontas, é monótona, e é
    /// SIMÉTRICA — acelera e desacelera igual.
    #[test]
    fn a_curva_e_um_ease_in_out_simetrico() {
        assert!(ease_in_out(0.0).abs() < 1e-4);
        assert!((ease_in_out(1.0) - 1.0).abs() < 1e-4);
        assert!(
            (ease_in_out(0.5) - 0.5).abs() < 1e-3,
            "meio do caminho no meio do tempo"
        );

        let mut anterior = -1.0;
        for k in 0..=100 {
            let y = ease_in_out(k as f32 / 100.0);
            assert!(y >= anterior - 1e-5, "monótona em t={k}");
            anterior = y;
        }
        // Simetria: `f(t) + f(1-t) = 1`.
        for k in 0..=20 {
            let t = k as f32 / 20.0;
            let soma = ease_in_out(t) + ease_in_out(1.0 - t);
            assert!((soma - 1.0).abs() < 1e-3, "simétrica em t={t}; soma {soma}");
        }
        // Começa DEVAGAR (é o "in" do ease-in-out) — um `ease-out` já teria passado da metade.
        assert!(ease_in_out(0.25) < 0.25);

        // Aparada nas duas pontas: `t` deriva de tempo decorrido e pode estourar.
        assert_eq!(ease_in_out(-1.0), ease_in_out(0.0));
        assert_eq!(ease_in_out(5.0), ease_in_out(1.0));
    }

    // --- Transição --------------------------------------------------------------------------

    /// Uma transição que já terminou, pra os testes não dependerem de dormir.
    fn ha(ms: u64) -> Instant {
        Instant::now()
            .checked_sub(Duration::from_millis(ms))
            .expect("o relógio da máquina tem mais de 1s de vida")
    }

    /// **Fechado e parado, o painel não está montado** — e é isso que impede um backdrop invisível
    /// de continuar engolindo cliques depois que ele sai.
    #[test]
    fn fechado_e_parado_nao_monta_nada() {
        let t = Transition::default();
        assert!(!t.open);
        assert_eq!(t.progress(), 0.0);
        assert!(!t.mounted());
        assert!(!t.animating());
    }

    /// **Um relógio, duas propriedades.** Abrir leva o progresso de 0 a 1 em 200ms: a opacidade
    /// acompanha, e o deslocamento vai de 32px pra fora até zero.
    #[test]
    fn abrir_e_fechar_percorrem_opacidade_e_deslocamento() {
        let mut t = Transition::default();

        // Fechado e parado: apagado e 32px pra fora.
        assert_eq!(t.opacity(), 0.0);
        assert_eq!(t.offset(SheetSide::Right), SLIDE);

        assert!(t.set(true), "abriu");
        assert!(t.mounted());
        assert!(t.animating());
        // No instante zero ainda está apagado e fora.
        assert!(t.opacity() < 0.05);
        assert!(t.offset(SheetSide::Right) > SLIDE * 0.95);
        assert_eq!(t.target(), 1.0);

        // Passados os 200ms, chega no alvo — sem precisar de limpeza do campo `anim`.
        t.anim = Some((0.0, ha(300)));
        assert_eq!(t.opacity(), 1.0);
        assert_eq!(t.offset(SheetSide::Right), 0.0, "em repouso, no lugar");
        assert!(!t.animating());

        assert!(t.set(false), "fechou");
        assert!(t.animating());
        assert!(t.mounted(), "continua montado durante a saída");
        assert!(t.opacity() > 0.95, "a saída parte de onde estava");

        t.anim = Some((1.0, ha(300)));
        assert_eq!(t.opacity(), 0.0);
        assert_eq!(t.offset(SheetSide::Right), SLIDE);
        assert!(!t.mounted(), "e aí desmonta");
    }

    /// **O deslocamento é sempre pra FORA**, e no eixo do lado. Errar o sinal faria o painel entrar
    /// de dentro da tela — parecendo que ele sai do meio da app em vez da borda.
    #[test]
    fn o_deslocamento_aponta_pra_fora_de_cada_lado() {
        let mut t = Transition::default();
        // Meio do caminho, congelado: 16px pra fora.
        t.hold(0.5);
        for side in LADOS {
            let d = t.offset(side);
            assert!(
                (d.abs() - SLIDE * 0.5).abs() < 0.1,
                "{side:?}: metade do percurso é {} px; veio {d}",
                SLIDE * 0.5
            );
            assert_eq!(
                d.signum(),
                side.outward(),
                "{side:?}: o deslocamento tem que apontar pra fora"
            );
        }
        // Direita e esquerda são simétricos; topo e base também.
        assert_eq!(t.offset(SheetSide::Right), -t.offset(SheetSide::Left));
        assert_eq!(t.offset(SheetSide::Bottom), -t.offset(SheetSide::Top));
    }

    /// **Trocar de alvo no meio do caminho parte de onde o olho está vendo**, e não de 0/1. Sem
    /// isso, fechar e reabrir depressa daria um salto — de opacidade E de posição.
    #[test]
    fn inverter_no_meio_nao_da_salto() {
        let mut t = Transition::default();
        t.set(true);
        // No meio do percurso. A faixa é larga de propósito: o progresso vem do relógio de PAREDE, e
        // apertar isto em torno de `ease_in_out(0.5)` = 0,5 tornaria o teste sensível a alguns
        // milissegundos de agendamento.
        t.anim = Some((0.0, ha(100)));
        let meio = t.opacity();
        let meio_d = t.offset(SheetSide::Right);
        assert!(
            (0.1..0.9).contains(&meio),
            "a metade do percurso; veio {meio}"
        );

        // Estas comparações, sim, são exatas: a inversão grava o progresso visível como ponto de
        // partida, e no instante zero da nova transição é ele que sai.
        t.set(false);
        assert!(
            (t.opacity() - meio).abs() < 0.01,
            "a inversão parte de {meio}; veio {}",
            t.opacity()
        );
        assert!(
            (t.offset(SheetSide::Right) - meio_d).abs() < 0.5,
            "e a posição também: partiu de {meio_d}; veio {}",
            t.offset(SheetSide::Right)
        );
        assert_eq!(t.target(), 0.0);
    }

    /// Pedir o alvo que já vale não faz nada: nem reinicia a animação, nem (no [`Sheet`]) emite
    /// evento. É o que torna `open`/`close` idempotentes.
    #[test]
    fn alvo_repetido_e_inerte() {
        let mut t = Transition::default();
        assert!(!t.set(false), "já estava fechado");
        assert!(t.anim.is_none(), "e não começou animação nenhuma");

        t.set(true);
        let antes = t.anim;
        assert!(!t.set(true), "já estava aberto");
        assert_eq!(t.anim.map(|(k, _)| k), antes.map(|(k, _)| k));
    }

    /// `settle` (só em teste) põe a transição no fim — é o que os testes de janela usam, porque o
    /// progresso vem do relógio de parede e o executor do GPUI não o adianta.
    #[test]
    fn settle_leva_ao_alvo() {
        let mut t = Transition::default();
        t.set(true);
        t.settle();
        assert_eq!(t.opacity(), 1.0);
        assert_eq!(t.offset(SheetSide::Right), 0.0);
        assert!(!t.animating());
    }

    /// A transição é a `duration-200` da referência.
    #[test]
    fn a_transicao_dura_200ms() {
        assert_eq!(TRANSITION, Duration::from_millis(200));
    }
}

/// Testes com janela de verdade. Posição, tamanho, deslocamento da animação, `Escape` e clique
/// passam pelo layout e pelo despacho de eventos do GPUI — nada disto é verificável por aritmética, e
/// é exatamente onde um painel destes costuma quebrar.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{
        point, size, AppContext as _, Bounds, Entity, Modifiers, Pixels, Render, TestAppContext,
        VisualTestContext,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    const PANEL_H: f32 = 120.0;
    const ACAO_H: f32 = 32.0;

    /// Uma view hospedeira igualzinha à do doc do módulo: raiz `relative` e o layer por último.
    struct Harness {
        sheet: Entity<Sheet>,
        side: SheetSide,
        variant: SheetVariant,
        /// Se o painel mostra um conteúdo alto (pra exercitar o `max-h-full`).
        alto: bool,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut popup = SheetPopup::new()
                .side(self.side)
                .variant(self.variant)
                .header(
                    SheetHeader::new()
                        .title("Propriedades")
                        .description("Ajustes do clipe selecionado."),
                )
                .footer(SheetFooter::new().child(div().w(px(80.0)).h(px(ACAO_H))));
            popup = if self.alto {
                popup.panel(div().h(px(4000.0)))
            } else {
                popup.panel(div().h(px(PANEL_H)))
            };
            div()
                .relative()
                .size_full()
                .child(div().size_full())
                .child(sheet_layer(&self.sheet, popup))
        }
    }

    /// O que uma janela de teste devolve.
    struct Aberta {
        sheet: Entity<Sheet>,
        eventos: Rc<RefCell<Vec<SheetEvent>>>,
        vcx: VisualTestContext,
        side: SheetSide,
        variant: SheetVariant,
        /// O tamanho REAL do viewport. Medido, e não o pedido: os testes de posição comparam pixel
        /// com pixel, e uma janela que não tivesse ficado do tamanho pedido faria as contas baterem
        /// por acidente (ou falharem por engano).
        viewport: gpui::Size<Pixels>,
    }

    /// Abre uma janela do tamanho pedido, com o layer montado como no doc do módulo.
    fn abrir(
        cx: &mut TestAppContext,
        largura: f32,
        altura: f32,
        side: SheetSide,
        variant: SheetVariant,
        alto: bool,
    ) -> Aberta {
        let eventos: Rc<RefCell<Vec<SheetEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let bounds = Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(largura), px(altura)),
        };
        let window = cx.update(|cx| {
            cx.open_window(
                gpui::WindowOptions {
                    window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_w, cx| {
                    cx.new(|cx| {
                        let sheet = cx.new(Sheet::new);
                        cx.subscribe(&sheet, move |_h, _s, ev: &SheetEvent, _cx| {
                            capturados.borrow_mut().push(*ev);
                        })
                        .detach();
                        Harness {
                            sheet,
                            side,
                            variant,
                            alto,
                        }
                    })
                },
            )
            .expect("abrir a janela de teste")
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let sheet = vcx.read(|cx| harness.read(cx).sheet.clone());
        let viewport = vcx.update(|window, _cx| window.viewport_size());
        Aberta {
            sheet,
            eventos,
            vcx,
            side,
            variant,
            viewport,
        }
    }

    /// Põe a transição no fim e força um frame, pra o teste medir o estado de REPOUSO.
    ///
    /// O `refresh` é explícito porque o `settle` é só de teste e não passa pelo [`Sheet::wake`] —
    /// sem ele o frame novo não vem, já que a view hospedeira não observa o `Entity`.
    fn assentar(a: &mut Aberta) {
        let sheet = a.sheet.clone();
        a.vcx.update(|window, cx| {
            sheet.update(cx, |s, cx| s.settle(cx));
            window.refresh();
        });
        a.vcx.run_until_parked();
    }

    /// Abre o painel e assenta a transição.
    fn escancarar(a: &mut Aberta) {
        let sheet = a.sheet.clone();
        a.vcx
            .update(|window, cx| sheet.update(cx, |s, cx| s.open(window, cx)));
        a.vcx.run_until_parked();
        assentar(a);
    }

    /// Congela a animação num progresso e força o frame.
    fn congelar(a: &mut Aberta, progresso: f32) {
        let sheet = a.sheet.clone();
        a.vcx.update(|window, cx| {
            sheet.update(cx, |s, cx| s.hold(progresso, cx));
            window.refresh();
        });
        a.vcx.run_until_parked();
    }

    /// A caixa da SUPERFÍCIE do painel — a sonda mede a padding box, e as bordas que existem (que
    /// dependem do lado e da variante, ver [`border_edges`]) devolvem a border box.
    fn superficie(a: &Aberta) -> Bounds<Pixels> {
        let b = a
            .vcx
            .read(|cx| a.sheet.read(cx).probe())
            .expect("o painel tem que estar montado");
        let [t, r, bo, l] = border_edges(a.side, a.variant);
        Bounds {
            origin: point(b.origin.x - px(l), b.origin.y - px(t)),
            size: size(b.size.width + px(l + r), b.size.height + px(t + bo)),
        }
    }

    /// **Fechado, o painel não existe no layout.** Se a sonda medisse algo, haveria um painel
    /// invisível (e um backdrop com hitbox) sobre a app.
    #[gpui::test]
    fn fechado_nao_monta_nada(cx: &mut TestAppContext) {
        let a = abrir(cx, 1000.0, 800.0, SheetSide::Right, SheetVariant::Default, false);
        assert!(!a.vcx.read(|cx| a.sheet.read(cx).is_open()));
        assert!(
            a.vcx.read(|cx| a.sheet.read(cx).probe()).is_none(),
            "nada montado"
        );
        assert!(a.eventos.borrow().is_empty());
    }

    /// **Os quatro lados, medidos.** Cada um encosta na sua borda, deixa os 48px do lado oposto, e
    /// enche a outra dimensão inteira.
    ///
    /// É o teste que um teste puro não pode fazer: o dimensionamento sai do `w_full`/esticamento
    /// resolvidos pelo layout de verdade.
    #[gpui::test]
    fn cada_lado_encosta_na_sua_borda(cx: &mut TestAppContext) {
        const W: f32 = 1000.0;
        const H: f32 = 800.0;

        for side in [
            SheetSide::Top,
            SheetSide::Right,
            SheetSide::Bottom,
            SheetSide::Left,
        ] {
            let mut a = abrir(cx, W, H, side, SheetVariant::Default, false);
            escancarar(&mut a);
            let s = superficie(&a);
            let (x, y) = (f32::from(s.origin.x), f32::from(s.origin.y));
            let (w, h) = (f32::from(s.size.width), f32::from(s.size.height));
            let (jw, jh) = (f32::from(a.viewport.width), f32::from(a.viewport.height));

            if side.horizontal() {
                // Altura cheia, largura limitada pelo `max-w-md`.
                assert!((h - jh).abs() < 1.0, "{side:?}: altura cheia; veio {h}");
                assert!((y - 0.0).abs() < 1.0, "{side:?}: topo em 0; veio {y}");
                assert!(
                    (w - MAX_WIDTH).abs() < 1.0,
                    "{side:?}: largura {MAX_WIDTH}; veio {w}"
                );
                // E a faixa livre do outro lado é MAIOR que os 48px garantidos (aqui o `max-w-md`
                // corta antes).
                match side {
                    SheetSide::Right => {
                        assert!((x + w - jw).abs() < 1.0, "{side:?}: encosta na direita");
                        assert!(x >= OUTER_GAP, "{side:?}: sobra ao menos {OUTER_GAP}px");
                    }
                    _ => {
                        assert!(x.abs() < 1.0, "{side:?}: encosta na esquerda; veio {x}");
                        assert!(jw - w >= OUTER_GAP, "{side:?}: sobra ao menos {OUTER_GAP}px");
                    }
                }
            } else {
                // Largura cheia, altura do conteúdo.
                assert!((w - jw).abs() < 1.0, "{side:?}: largura cheia; veio {w}");
                assert!(x.abs() < 1.0, "{side:?}: x em 0; veio {x}");
                assert!(
                    h < jh - OUTER_GAP + 1.0,
                    "{side:?}: a altura tem que caber em {} px; veio {h}",
                    jh - OUTER_GAP
                );
                match side {
                    SheetSide::Bottom => {
                        assert!((y + h - jh).abs() < 1.0, "{side:?}: encosta na base");
                        assert!(y >= OUTER_GAP, "{side:?}: sobra ao menos {OUTER_GAP}px");
                    }
                    _ => {
                        assert!(y.abs() < 1.0, "{side:?}: encosta no topo; veio {y}");
                    }
                }
            }
        }
    }

    /// Numa janela mais estreita que `max-w-md`, manda o `w-full`: a largura é a da janela menos os
    /// 48px do `calc(100% - --spacing(12))`.
    #[gpui::test]
    fn em_janela_estreita_manda_o_calc(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 360.0, 800.0, SheetSide::Right, SheetVariant::Default, false);
        escancarar(&mut a);
        let s = superficie(&a);
        let esperado = f32::from(a.viewport.width) - OUTER_GAP;
        assert!(
            esperado < MAX_WIDTH,
            "a janela precisa ser mais estreita que o max-w-md pra o teste testar algo (deu {esperado})"
        );
        assert!(
            (f32::from(s.size.width) - esperado).abs() < 1.0,
            "esperado {esperado}, veio {}",
            f32::from(s.size.width)
        );
        assert!(
            (f32::from(s.origin.x) - OUTER_GAP).abs() < 1.0,
            "e ele começa depois dos 48px"
        );
    }

    /// **A variante `inset` afasta 16px de todas as bordas** — e, no lado horizontal, os 48px do
    /// `calc` SOMAM com esse respiro (é a assimetria que o teste puro
    /// `no_inset_o_p4_soma_no_horizontal_e_perde_no_vertical` trava).
    #[gpui::test]
    fn a_variante_inset_afasta_16px_de_todas_as_bordas(cx: &mut TestAppContext) {
        const W: f32 = 1000.0;
        const H: f32 = 800.0;

        // Direita: 16px de respiro em cima, embaixo e na direita.
        let mut a = abrir(cx, W, H, SheetSide::Right, SheetVariant::Inset, false);
        escancarar(&mut a);
        let s = superficie(&a);
        assert!(
            (f32::from(s.origin.y) - INSET_PAD).abs() < 1.0,
            "topo em {INSET_PAD}; veio {}",
            f32::from(s.origin.y)
        );
        assert!(
            (f32::from(s.size.height) - (H - 2.0 * INSET_PAD)).abs() < 1.0,
            "altura {} ; veio {}",
            H - 2.0 * INSET_PAD,
            f32::from(s.size.height)
        );
        let direita = f32::from(a.viewport.width - s.origin.x - s.size.width);
        assert!(
            (direita - INSET_PAD).abs() < 1.0,
            "respiro de {INSET_PAD} na direita; veio {direita}"
        );
        assert!(
            (f32::from(s.size.width) - MAX_WIDTH).abs() < 1.0,
            "o max-w-md continua cortando"
        );

        // Base: 16px em baixo e nos lados; a faixa de cima continua sendo 48 (o `pt-12` vence o
        // `p-4`), então a altura útil é 800 − 48 − 16.
        let mut a = abrir(cx, W, H, SheetSide::Bottom, SheetVariant::Inset, true);
        escancarar(&mut a);
        let s = superficie(&a);
        let base = f32::from(a.viewport.height - s.origin.y - s.size.height);
        assert!(
            (base - INSET_PAD).abs() < 1.0,
            "respiro de {INSET_PAD} na base; veio {base}"
        );
        assert!(
            (f32::from(s.origin.x) - INSET_PAD).abs() < 1.0,
            "e 16 na esquerda"
        );
        assert!(
            (f32::from(s.size.width) - (W - 2.0 * INSET_PAD)).abs() < 1.0,
            "largura = janela − 2×16"
        );
        let util = H - OUTER_GAP - INSET_PAD;
        assert!(
            (f32::from(s.size.height) - util).abs() < 1.0,
            "com conteúdo alto a altura enche os {util}px (48 em cima, 16 embaixo); veio {}",
            f32::from(s.size.height)
        );
    }

    /// **A altura do painel, pixel por pixel.**
    ///
    /// É o teste que amarra TUDO o que um teste puro não vê de uma vez: as alturas de linha do
    /// título (`leading-none`) e da descrição (`text-sm` = 14/20), o `gap-2` entre as duas, a cadeia
    /// de respiros da [`spacing`], a linha de 1px do footer, a borda única da variante `Default`, e o
    /// fato de o [`crate::ScrollArea`] não somar altura nenhuma por conta própria.
    ///
    /// O conteúdo do [`Harness`] é: header com título e descrição, conteúdo de 120px, footer default
    /// com uma ação de 32px.
    #[gpui::test]
    fn a_altura_do_painel_bate_com_a_soma_dos_utilitarios(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 800.0, SheetSide::Bottom, SheetVariant::Default, false);
        escancarar(&mut a);
        let s = superficie(&a);

        let sp = spacing(true, true, Some(SheetFooterVariant::Default));
        // Header: `p-6` no topo, uma linha de título, o `gap-2`, uma linha de descrição, e o `pb-3`
        // porque existe conteúdo.
        let header = PAD + TITLE_LINE + GAP + DESC_LINE + sp.header_pb;
        // Conteúdo: o fio contra o header, o corpo, e o `pb-6` (o footer default tem linha).
        let corpo = sp.panel_pt + PANEL_H + sp.panel_pb;
        // Footer: a linha de topo de 1px conta na altura (é border-box), mais `py-4` e a ação.
        let footer = 1.0 + sp.footer_pt + ACAO_H + sp.footer_pb;
        // E as bordas do painel no eixo vertical — num painel de baixo, só o `border-t`.
        let [bt, _, bb, _] = border_edges(SheetSide::Bottom, SheetVariant::Default);
        let esperado = header + corpo + footer + bt + bb;

        let veio = f32::from(s.size.height);
        assert!(
            (veio - esperado).abs() < 0.5,
            "altura esperada {esperado}, veio {veio} \
             (header {header} + corpo {corpo} + footer {footer} + {} de borda)",
            bt + bb
        );
    }

    /// **O `max-h-full` de verdade**: com 4000px de conteúdo, o painel de baixo não passa da altura
    /// útil (janela − 48) — quem cede é o conteúdo (que rola), não o header nem o footer.
    #[gpui::test]
    fn painel_vertical_nao_passa_da_altura_util(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 600.0, SheetSide::Bottom, SheetVariant::Default, true);
        escancarar(&mut a);

        let s = superficie(&a);
        let util = f32::from(a.viewport.height) - OUTER_GAP;
        assert!(
            f32::from(s.size.height) <= util + 1.0,
            "painel de {:.1}px numa altura útil de {util}",
            f32::from(s.size.height)
        );
        // E ele de fato ENCHEU a altura útil (senão o teste passaria por vacuidade).
        assert!(
            f32::from(s.size.height) > util - 2.0,
            "com 4000px de conteúdo o painel tinha que encher os {util}px; veio {:.1}",
            f32::from(s.size.height)
        );
        // Encostando no limite, ele começa exatamente depois da faixa de 48px.
        assert!((f32::from(s.origin.y) - OUTER_GAP).abs() < 1.0);
    }

    /// **A translação de entrada, medida.** No meio do caminho o painel está 16px pra fora da
    /// posição de repouso, no eixo do seu lado — e nada mais se moveu.
    ///
    /// Este é o teste que justifica a técnica: o GPUI não tem `translate` em `div`, então o
    /// deslocamento é um `inset` de posicionamento RELATIVO. Se o taffy ignorasse esse inset (ou se
    /// ele mexesse no layout em vez de só reposicionar), é aqui que apareceria.
    #[gpui::test]
    fn a_translacao_desloca_o_painel_pra_fora(cx: &mut TestAppContext) {
        for side in [
            SheetSide::Top,
            SheetSide::Right,
            SheetSide::Bottom,
            SheetSide::Left,
        ] {
            let mut a = abrir(cx, 1000.0, 800.0, side, SheetVariant::Default, false);
            escancarar(&mut a);
            let repouso = superficie(&a);

            // Metade do percurso: 16px pra fora.
            congelar(&mut a, 0.5);
            let meio = superficie(&a);
            let esperado = SLIDE * 0.5 * side.outward();

            let (dx, dy) = (
                f32::from(meio.origin.x - repouso.origin.x),
                f32::from(meio.origin.y - repouso.origin.y),
            );
            if side.horizontal() {
                assert!(
                    (dx - esperado).abs() < 1.0,
                    "{side:?}: esperava {esperado}px em X; veio {dx}"
                );
                assert!(dy.abs() < 1.0, "{side:?}: nada em Y; veio {dy}");
            } else {
                assert!(
                    (dy - esperado).abs() < 1.0,
                    "{side:?}: esperava {esperado}px em Y; veio {dy}"
                );
                assert!(dx.abs() < 1.0, "{side:?}: nada em X; veio {dx}");
            }

            // No começo do percurso, os 32px cheios.
            congelar(&mut a, 0.0);
            let fora = superficie(&a);
            let d = if side.horizontal() {
                f32::from(fora.origin.x - repouso.origin.x)
            } else {
                f32::from(fora.origin.y - repouso.origin.y)
            };
            assert!(
                (d - SLIDE * side.outward()).abs() < 1.0,
                "{side:?}: no início são {} px; veio {d}",
                SLIDE * side.outward()
            );
        }
    }

    /// **Transladar não reflui.** O painel tem exatamente o mesmo tamanho parado e no meio da
    /// animação — que é a razão de esta animação ser reproduzível por geometria e a escala do
    /// [`crate::dialog`] não ser.
    #[gpui::test]
    fn a_translacao_nao_muda_o_tamanho(cx: &mut TestAppContext) {
        for side in [SheetSide::Right, SheetSide::Bottom] {
            let mut a = abrir(cx, 1000.0, 800.0, side, SheetVariant::Default, false);
            escancarar(&mut a);
            let repouso = superficie(&a).size;

            for progresso in [0.0, 0.25, 0.5, 0.75] {
                congelar(&mut a, progresso);
                let agora = superficie(&a).size;
                assert!(
                    (f32::from(agora.width - repouso.width)).abs() < 0.5
                        && (f32::from(agora.height - repouso.height)).abs() < 0.5,
                    "{side:?} em {progresso}: o tamanho mudou de {repouso:?} pra {agora:?}"
                );
            }
        }
    }

    /// **`Escape` fecha.** Depende de duas coisas que só a janela mostra: o `open` ter FOCADO o
    /// painel, e a tecla subir pelo caminho de foco até o `on_key_down` dele.
    #[gpui::test]
    fn escape_fecha(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 800.0, SheetSide::Right, SheetVariant::Default, false);
        escancarar(&mut a);
        assert!(a.vcx.read(|cx| a.sheet.read(cx).is_open()));

        a.vcx.simulate_keystrokes("escape");
        a.vcx.run_until_parked();

        assert!(!a.vcx.read(|cx| a.sheet.read(cx).is_open()), "fechou");
        assert_eq!(
            *a.eventos.borrow(),
            vec![SheetEvent::Opened, SheetEvent::Closed]
        );

        // E, acabada a saída, o layer DESMONTA. Um backdrop invisível que continuasse ali seguiria
        // engolindo todo clique da app — o pior defeito possível num painel fechado.
        assentar(&mut a);
        assert!(
            a.vcx.read(|cx| a.sheet.read(cx).probe()).is_none(),
            "nada montado depois da saída"
        );

        // Outra tecla não fecha nada.
        escancarar(&mut a);
        a.vcx.simulate_keystrokes("a");
        a.vcx.run_until_parked();
        assert!(
            a.vcx.read(|cx| a.sheet.read(cx).is_open()),
            "só o escape fecha"
        );
    }

    /// **Clicar no backdrop fecha; clicar NO painel não.**
    ///
    /// O segundo é o que exige o `occlude()` do painel: sem ele o clique também acertaria o hitbox
    /// do backdrop, que está atrás, e o painel fecharia ao usuário clicar no próprio conteúdo.
    #[gpui::test]
    fn clique_no_backdrop_fecha_e_no_painel_nao(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 800.0, SheetSide::Right, SheetVariant::Default, false);
        escancarar(&mut a);
        let s = superficie(&a);

        // Dentro do painel: o centro dele.
        let dentro = point(
            s.origin.x + s.size.width / 2.0,
            s.origin.y + s.size.height / 2.0,
        );
        clicar(&mut a.vcx, dentro);
        assert!(
            a.vcx.read(|cx| a.sheet.read(cx).is_open()),
            "clique no painel NÃO fecha"
        );

        // Fora dele: na faixa de 48px da esquerda, que é backdrop puro.
        let fora = point(px(4.0), a.viewport.height / 2.0);
        assert!(
            fora.x < s.origin.x,
            "o ponto tem que estar À ESQUERDA do painel, senão o teste não testa nada"
        );
        clicar(&mut a.vcx, fora);
        assert!(
            !a.vcx.read(|cx| a.sheet.read(cx).is_open()),
            "clique no backdrop fecha"
        );
    }

    /// Abrir manda o foco pro painel (é o que faz o `Escape` funcionar) e fechar devolve pra quem o
    /// tinha.
    #[gpui::test]
    fn o_foco_vai_pro_painel_e_volta(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 800.0, SheetSide::Right, SheetVariant::Default, false);

        // Um handle qualquer focado antes de abrir.
        let anterior = a.vcx.update(|window, cx| {
            let h = cx.focus_handle();
            window.focus(&h);
            h
        });
        a.vcx.run_until_parked();

        escancarar(&mut a);
        let painel = a.vcx.read(|cx| a.sheet.read(cx).focus_handle());
        assert!(
            a.vcx.update(|window, _cx| painel.is_focused(window)),
            "o painel fica focado enquanto aberto"
        );

        a.vcx.simulate_keystrokes("escape");
        a.vcx.run_until_parked();
        assert!(
            a.vcx.update(|window, _cx| anterior.is_focused(window)),
            "e o foco volta pra quem o tinha"
        );
    }

    /// Um clique completo (down + up) num ponto.
    fn clicar(vcx: &mut VisualTestContext, at: gpui::Point<Pixels>) {
        vcx.simulate_event(gpui::MouseDownEvent {
            button: gpui::MouseButton::Left,
            position: at,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(gpui::MouseUpEvent {
            button: gpui::MouseButton::Left,
            position: at,
            modifiers: Modifiers::default(),
            click_count: 1,
        });
        vcx.run_until_parked();
    }
}

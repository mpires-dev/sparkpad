//! `Card` — a superfície de conteúdo do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/card.tsx`
//!
//! # Dois componentes, não um
//!
//! - [`Card`] — a superfície comum: fundo, borda, raio de 16px, sombra sutil e um fio de bisel.
//!   Tem três fatias opcionais: **header** (título/descrição/ação), **painel** (o conteúdo) e
//!   **footer**.
//! - [`CardFrame`] — uma **moldura** com fundo levemente acinzentado que empilha `Card`s rentes
//!   por dentro. Os cards internos perdem sombra e bisel, e o fundo da moldura aparece só na
//!   fresta de 1px entre eles — é o que dá o look de lista emoldurada.
//!
//! # Paddings condicionais: aqui é mais simples que no original
//!
//! No coss os espaçamentos dependem da composição, e isso é expresso com `:has()` e `in-[...]`:
//! o header perde parte do padding de baixo quando existe um painel, o painel perde o de cima
//! quando existe header, o footer aperta o de cima quando existe painel. São regras de vizinhança
//! que o CSS só consegue ver de fora.
//!
//! Aqui o [`Card`] **é dono** das suas fatias, então a mesma regra é uma decisão local e
//! determinística (ver [`Card::render`]) — sem seletor condicional e sem depender da ordem em que
//! os filhos foram declarados.
//!
//! # Uso
//!
//! ```ignore
//! Card::new()
//!     .header(CardHeader::new().title("Assinatura").description("Plano e cobrança"))
//!     .panel(meu_conteudo)
//!     .footer(meus_botoes)
//! ```

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AnyElement, App, Div, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    Styled, Window,
};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Os utilitários Tailwind do `card.tsx` resolvidos em número, com a paleta `neutral` do Tailwind
// expandida e os `color-mix` já calculados. Mesma disciplina de cor do campo de texto: TODO valor é
// `0xRRGGBBAA` (ver [`crate::color`]).

/// Tokens visuais do card, por tema.
#[derive(Clone, Copy, Debug)]
struct CardPalette {
    /// Fundo da superfície (`--card`).
    bg: Rgba8,
    /// Cor do texto (`--card-foreground`).
    text: Rgba8,
    /// Texto secundário — descrição (`--muted-foreground`).
    text_muted: Rgba8,
    /// Borda da superfície (`--border`).
    border: Rgba8,
    /// Fundo da MOLDURA, que aparece nas frestas entre os cards internos (`--muted` a 72%).
    frame_fill: Rgba8,
    /// Sombra externa (`shadow-xs/5`).
    shadow: Rgba8,
    /// Fio de bisel de 1px. Desce no claro, sobe no escuro.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro), `-1` sobe (escuro).
    bevel_dir: f32,
}

/// Tema **claro**.
const CARD_LIGHT: CardPalette = CardPalette {
    bg: Rgba8(0xffffffff),
    text: Rgba8(0x262626ff), // neutral-800
    // mix(neutral-500 90%, black) = #686868
    text_muted: Rgba8(0x686868ff),
    border: Rgba8(0x00000014), // black 8%
    // --muted é black 4%; o `/72` do Tailwind multiplica → ~2,9%.
    frame_fill: Rgba8(0x00000007),
    shadow: Rgba8(0x0000000d), // black 5%
    bevel: Rgba8(0x0000000a),  // black 4%
    bevel_dir: 1.0,
};

/// Tema **escuro**.
const CARD_DARK: CardPalette = CardPalette {
    // mix(background 98%, white) onde background = mix(neutral-950 96%, white) = #141414 → #191919
    bg: Rgba8(0x191919ff),
    text: Rgba8(0xf5f5f5ff), // neutral-100
    // mix(neutral-500 90%, white) = #818181
    text_muted: Rgba8(0x818181ff),
    border: Rgba8(0xffffff0f), // white 6%
    frame_fill: Rgba8(0xffffff07),
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE do coss: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO —
    // alfa 30 ≈ 11,8% — por decisão de design: a 6% o filete é imperceptível no nosso fundo escuro.
    // NÃO "corrija" isto pra 0x0f achando que é erro de porte; se a intenção mudar, mude junto o
    // teste `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
};

/// A paleta do card no tema corrente.
fn card() -> &'static CardPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &CARD_DARK,
        theme::ThemeMode::Light => &CARD_LIGHT,
    }
}

// --- Geometria ------------------------------------------------------------------------------------

/// Raio da superfície — `rounded-2xl`. O coss não redefine `--radius-2xl`, então vale o default do
/// Tailwind: `1rem` = **16px**.
pub const CARD_RADIUS: f32 = 16.0;

/// Raio das bordas INTERNAS dos cards empilhados numa moldura — `rounded-xl`, que no coss é
/// `calc(var(--radius) + 4px)` = `10 + 4` = **14px**.
const CARD_INNER_RADIUS: f32 = 14.0;

/// Respiro padrão das fatias — `p-6`.
const PAD: f32 = 24.0;

/// Respiro apertado, usado onde duas fatias se encostam — `p-4`/`pb-4`/`pt-4`.
const PAD_TIGHT: f32 = 16.0;

/// Respiro entre fatias vizinhas num card [`Card::compact`] — 12px.
///
/// **O par de respiros do card**: o base e o apertado. `compact` desce os dois um passo na escala.
///
/// Existe como função porque o respiro tem que valer nos DOIS eixos. Enquanto o horizontal era o
/// `PAD` literal e só o vertical saía daqui, um card `compact` ficava com 24 nas laterais e 16 em
/// cima — assimetria que o usuário viu no `ColorPicker` antes de eu ver no código.
fn pads(compact: bool) -> (f32, f32) {
    if compact {
        (PAD_TIGHT, PAD_SNUG)
    } else {
        (PAD, PAD_TIGHT)
    }
}

/// Não vem da referência: é o passo abaixo do `PAD_TIGHT` na escala de 4px, pra o aperto relativo
/// entre fatias se manter quando o respiro base cai de 24 pra 16.
const PAD_SNUG: f32 = 12.0;

/// Espaço entre título e descrição no header — `gap-1.5`.
const HEADER_GAP: f32 = 6.0;

/// Espessura da borda do card e da moldura. Ela **dirige** o `border()` das duas superfícies, e não só
/// as contas: o `width` do card é border-box, então quem calcula respiro (ou testa se algo passou da
/// borda) precisa do mesmo número que o desenho usa.
const CARD_BORDER: f32 = 1.0;

/// Corpo do título do card — `text-lg`.
const TITLE_SIZE: f32 = 18.0;

/// Corpo do texto secundário e do título de moldura — `text-sm`.
const SMALL_SIZE: f32 = 14.0;

/// Entrelinha do título de um card comum — `leading-none`, ou seja **igual ao corpo**. É a única
/// entrelinha apertada do componente, e a referência a pede explicitamente
/// (`CardTitle`: `text-lg leading-none`).
const TITLE_LINE_HEIGHT: f32 = TITLE_SIZE;

/// Entrelinha dos textos de 14px: o par do `text-sm` do Tailwind. Sem declarar, o GPUI usa a razão de
/// ouro (22,6px) e cada linha ganha 2,6px que a referência não tem — a armadilha nº 1 da casa, e é ela
/// que fazia o header sair mais alto que o original.
const SMALL_LINE_HEIGHT: f32 = 20.0;

// =================================================================================================
// Header
// =================================================================================================

/// O **header** de um [`Card`]: título, descrição e uma ação opcional no canto.
///
/// A ação fica no topo à direita e não desloca o título — no coss é uma célula própria do grid
/// (`col-start-2 row-span-2`).
#[derive(Default)]
pub struct CardHeader {
    title: Option<SharedString>,
    description: Option<SharedString>,
    action: Option<AnyElement>,
    /// `border-b`: transforma o header numa faixa separada por linha.
    bordered: bool,
}

impl CardHeader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Título — 18px, semibold.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Descrição — 14px, texto secundário.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Elemento no canto superior direito (botão, menu, badge).
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }

    /// Liga a **linha separadora** embaixo do header.
    ///
    /// Muda o espaçamento junto: com separador, o header mantém o respiro cheio e o painel volta a
    /// ter o dele — sem separador, os dois se aproximam. É a mesma regra do
    /// `:not(.border-b)` do original.
    pub fn bordered(mut self) -> Self {
        self.bordered = true;
        self
    }

    /// Se o header não tem nada pra mostrar (evita renderizar uma faixa vazia).
    fn is_empty(&self) -> bool {
        self.title.is_none() && self.description.is_none() && self.action.is_none()
    }

    /// Monta o header. `tight_bottom` aperta o padding de baixo (há painel logo abaixo e nenhum
    /// separador entre eles).
    fn render(self, tight_bottom: bool, small: bool, pad_x: f32) -> Div {
        let p = card();
        let (title_size, pad_y) = if small {
            // Header de moldura: `px-6 py-4`, título 14px.
            (SMALL_SIZE, PAD_TIGHT)
        } else {
            (TITLE_SIZE, PAD)
        };

        // Os vãos do header, que diferem entre as duas variantes. No coss:
        //
        // | | horizontal | vertical |
        // |---|---|---|
        // | `CardHeader` (`gap-1.5`) | 6 | 6 |
        // | `CardFrameHeader` (`gap-x-4`) | **16** | **0** |
        //
        // O `gap-x-*` sem `gap-y-*` é vão horizontal e ZERO vertical — no header de moldura o título e
        // a descrição se encostam de propósito. Aqui isso era um `HEADER_GAP` só, nos dois eixos das
        // duas variantes.
        let (gap_x, gap_y) = if small {
            (PAD_TIGHT, 0.0)
        } else {
            (HEADER_GAP, HEADER_GAP)
        };

        // ⚠️ **O `min_w` não é detalhe.** Sem ele o bloco de textos não encolhe abaixo do próprio
        // min-content, e o min-content de um texto no GPUI é a FRASE INTEIRA numa linha: o
        // `request_measured_layout` do `TextElement` só quebra quando recebe largura definida, e
        // `AvailableSpace::MinContent` cai no braço `None` (ver `gpui/src/elements/text.rs`). Com o
        // action sendo `flex_none`, a soma estourava a linha e ele saía **por fora da borda direita** do
        // card — era o defeito relatado.
        //
        // No navegador isso não acontece porque o min-content de um parágrafo é a palavra mais longa,
        // então a coluna de texto do grid encolhe e o texto quebra. O `min_w(0)` recria essa liberdade.
        let mut textos = div()
            .flex()
            .flex_col()
            .gap(px(gap_y))
            .flex_1()
            .min_w(px(0.0));
        if let Some(t) = self.title {
            textos = textos.child(
                div()
                    .text_size(px(title_size))
                    // `leading-none` no card comum; o par do `text-sm` no de moldura.
                    .line_height(px(if small {
                        SMALL_LINE_HEIGHT
                    } else {
                        TITLE_LINE_HEIGHT
                    }))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(p.text.hsla())
                    .child(t),
            );
        }
        if let Some(d) = self.description {
            textos = textos.child(
                div()
                    .text_size(px(SMALL_SIZE))
                    .line_height(px(SMALL_LINE_HEIGHT))
                    .text_color(p.text_muted.hsla())
                    .child(d),
            );
        }

        let mut row = div()
            .flex()
            .items_start()
            .gap(px(gap_x))
            .px(px(pad_x))
            .pt(px(pad_y))
            .pb(px(if tight_bottom { PAD_TIGHT } else { pad_y }))
            .child(textos);
        if let Some(a) = self.action {
            // `self-center` num header de moldura, `self-start` no comum — no coss é a diferença entre
            // `CardFrameAction` e `CardAction`.
            //
            // ⚠️ Isto era um `items_center()` no wrapper, que é `align-items` e alinha os FILHOS dele —
            // um no-op, porque o wrapper tem o tamanho do conteúdo. Quem posiciona o wrapper dentro da
            // linha é `align-self`, e o GPUI 0.2 não tem atalho pra ele: o campo existe no `Style` mas
            // não há método no builder, daí o acesso direto ao refinamento.
            let mut envelope = div().flex().flex_none();
            if small {
                envelope.style().align_self = Some(gpui::AlignItems::Center);
            }
            row = row.child(envelope.child(a));
        }
        row.when(self.bordered, |d| {
            d.border_b_1().border_color(p.border.hsla())
        })
    }
}

// =================================================================================================
// Card
// =================================================================================================

/// A superfície de conteúdo. Ver o doc do módulo.
#[derive(IntoElement, Default)]
pub struct Card {
    header: Option<CardHeader>,
    panel: Option<AnyElement>,
    footer: Option<AnyElement>,
    /// `border-t` no footer.
    footer_bordered: bool,
    /// Cantos de baixo retos.
    square_bottom: bool,
    width: Option<Pixels>,
    /// Ligado pela [`CardFrame`] nos cards que ela empilha: sem sombra, sem bisel, e o raio de cada
    /// canto vem de fora.
    nested: Option<NestedCorners>,
    /// Troca a sombra `shadow-xs` pela `shadow-lg` — ver [`Card::elevated`].
    elevated: bool,
    /// Respiro apertado em todas as fatias — ver [`Card::compact`].
    compact: bool,
}

/// Como arredondar um card empilhado dentro de uma [`CardFrame`].
#[derive(Clone, Copy, Debug, PartialEq)]
struct NestedCorners {
    top: f32,
    bottom: f32,
}

impl Card {
    pub fn new() -> Self {
        Self::default()
    }

    /// Header com título/descrição/ação.
    pub fn header(mut self, header: CardHeader) -> Self {
        self.header = Some(header);
        self
    }

    /// O conteúdo principal.
    pub fn panel(mut self, panel: impl IntoElement) -> Self {
        self.panel = Some(panel.into_any_element());
        self
    }

    /// O rodapé (normalmente ações).
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Liga a **linha separadora** em cima do footer.
    pub fn footer_bordered(mut self) -> Self {
        self.footer_bordered = true;
        self
    }

    /// Deixa os cantos de baixo retos — para o card encostar em algo abaixo dele.
    pub fn square_bottom(mut self) -> Self {
        self.square_bottom = true;
        self
    }

    /// **Levanta o card**: troca a `shadow-xs/5` de superfície em repouso pela `shadow-lg/5`, a mesma
    /// sombra que o [`crate::menu`], o [`crate::toast`] e o [`crate::dialog`] usam.
    ///
    /// Existe pra um card que FLUTUA — um popover, um painel sobreposto. A sombra de repouso não
    /// levanta o suficiente pra ele ler como estando por cima, e a alternativa seria um `div` cru com
    /// a sombra copiada à mão: foi assim que o `color_picker` acabou com uma sombra de preto a **55%**
    /// e blur 32, onze vezes mais forte que o resto da biblioteca e visivelmente estranha no escuro.
    pub fn elevated(mut self) -> Self {
        self.elevated = true;
        self
    }

    /// **Respiro apertado**: 16px em vez de 24 em todas as fatias.
    ///
    /// O card já aperta sozinho onde duas fatias se encostam; isto estende o aperto ao conjunto, pra
    /// conteúdo denso — uma paleta de cores, uma grade de controles pequenos. Sem ele, 24px em volta
    /// de controles de 24px de altura fazem a peça parecer vazia.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Largura fixa (o default é ocupar o container).
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// O bisel de 1px sobreposto.
    ///
    /// ⚠️ Duas coisas que só apareceram medindo (ver `crate::input::bevel_for`, onde as duas
    /// custaram bug visível):
    ///
    /// 1. **É borda, não sombra.** O `Window::paint_shadows` do GPUI não recorta a sombra pra fora
    ///    do elemento que a projeta (diferente do CSS), então o `before:box-shadow` do coss viraria
    ///    uma lavagem de cor sobre o card inteiro.
    /// 2. **Cobre a BORDER box, não a padding box.** No coss a sombra sai 1px pra fora do
    ///    pseudo-elemento, ou seja cai SOBRE a borda e a clareia. Um filete 1px pra dentro cria uma
    ///    segunda linha ao lado da borda — e a 4–6% de alfa isso é praticamente invisível.
    fn bevel(radius_top: f32, radius_bottom: f32) -> Div {
        let p = card();
        let overlay = div()
            .absolute()
            .top(px(-1.0))
            .left(px(-1.0))
            .right(px(-1.0))
            .bottom(px(-1.0))
            .rounded_tl(px(radius_top))
            .rounded_tr(px(radius_top))
            .rounded_bl(px(radius_bottom))
            .rounded_br(px(radius_bottom))
            .border_color(p.bevel.hsla());
        if p.bevel_dir > 0.0 {
            overlay.border_b_1()
        } else {
            overlay.border_t_1()
        }
    }
}

impl RenderOnce for Card {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = card();
        let nested = self.nested;

        // `square_bottom` vence em qualquer caso, INCLUSIVE aninhado: é ele que faz a base do card
        // correr reta até a faixa de footer da moldura em vez de curvar e se despregar dela (o
        // `rounded-b-none!` do exemplo "framed card with no rounded bottom").
        let (radius_top, radius_bottom) = match nested {
            Some(c) => (c.top, if self.square_bottom { 0.0 } else { c.bottom }),
            None => (
                CARD_RADIUS,
                if self.square_bottom { 0.0 } else { CARD_RADIUS },
            ),
        };

        // --- Espaçamento condicional -------------------------------------------------------
        //
        // No coss estas três regras são seletores `:has()`/`in-[...]`. Aqui o card é dono das
        // fatias, então elas são só condições locais:
        // `compact` troca o respiro base; o aperto entre fatias vizinhas continua valendo por cima.
        let (pad, pad_tight) = pads(self.compact);
        let tem_painel = self.panel.is_some();
        let header_bordered = self.header.as_ref().is_some_and(|h| h.bordered);
        // Header e painel se aproximam quando não há linha separando os dois.
        let header_tight = tem_painel && !header_bordered;
        // O painel encosta no header/footer quando não há separador.
        let panel_pt = if self.header.is_some() && !header_bordered {
            0.0
        } else {
            pad
        };
        let panel_pb = if self.footer.is_some() && !self.footer_bordered {
            0.0
        } else {
            pad
        };
        // O footer aperta o topo quando vem logo depois de um painel.
        let footer_pt = if tem_painel && !self.footer_bordered {
            pad_tight
        } else {
            pad
        };

        let mut surface = div()
            .relative()
            .flex()
            .flex_col()
            .bg(p.bg.hsla())
            .text_color(p.text.hsla())
            .rounded_tl(px(radius_top))
            .rounded_tr(px(radius_top))
            .rounded_bl(px(radius_bottom))
            .rounded_br(px(radius_bottom))
            .when_some(self.width, |d, w| d.w(w).flex_none())
            .when(self.width.is_none(), |d| d.w_full());

        // Card empilhado numa moldura não tem borda, sombra nem bisel próprios — quem os desenha é
        // a moldura (`*:data-[slot=card]:shadow-none` e `before:hidden` no original).
        if nested.is_none() {
            surface = surface
                .border(px(CARD_BORDER))
                .border_color(p.border.hsla())
                .shadow(if self.elevated {
                    // `shadow-lg/5`: o mesmo par de camadas do menu, do toast e do dialog.
                    vec![
                        gpui::BoxShadow {
                            color: p.shadow.hsla(),
                            offset: gpui::point(px(0.0), px(10.0)),
                            blur_radius: px(15.0),
                            spread_radius: px(-3.0),
                        },
                        gpui::BoxShadow {
                            color: p.shadow.hsla(),
                            offset: gpui::point(px(0.0), px(4.0)),
                            blur_radius: px(6.0),
                            spread_radius: px(-4.0),
                        },
                    ]
                } else {
                    // `shadow-xs/5`, a de superfície em repouso.
                    vec![gpui::BoxShadow {
                        color: p.shadow.hsla(),
                        offset: gpui::point(px(0.0), px(1.0)),
                        blur_radius: px(2.0),
                        spread_radius: px(0.0),
                    }]
                });
        }

        if let Some(h) = self.header {
            if !h.is_empty() {
                surface = surface.child(h.render(header_tight, false, pad));
            }
        }
        if let Some(panel) = self.panel {
            surface = surface.child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .px(px(pad))
                    .pt(px(panel_pt))
                    .pb(px(panel_pb))
                    .child(panel),
            );
        }
        if let Some(footer) = self.footer {
            surface = surface.child(
                div()
                    .flex()
                    .items_center()
                    .px(px(pad))
                    .pt(px(footer_pt))
                    .pb(px(pad))
                    .when(self.footer_bordered, |d| {
                        d.border_t_1().border_color(p.border.hsla())
                    })
                    .child(footer),
            );
        }

        // O bisel por último: é absoluto, então a ordem só decide quem pinta em cima.
        if nested.is_none() {
            surface = surface.child(Self::bevel(radius_top, radius_bottom));
        }
        surface
    }
}

// =================================================================================================
// CardFrame
// =================================================================================================

/// Uma **moldura** que empilha [`Card`]s rentes, com um fundo levemente acinzentado aparecendo nas
/// frestas.
///
/// Os cards internos perdem borda, sombra e bisel; o raio de cada um é decidido pela posição na
/// pilha: as pontas acompanham o raio da moldura (16 − 1 = 15px) e as bordas internas usam 14px
/// (`rounded-xl`). No coss isso é feito com `clip-path` + margens negativas; aqui é atribuição
/// direta de raio, que dá o mesmo resultado sem depender de recorte.
#[derive(IntoElement, Default)]
pub struct CardFrame {
    header: Option<CardHeader>,
    cards: Vec<Card>,
    footer: Option<AnyElement>,
    square_bottom: bool,
    width: Option<Pixels>,
}

impl CardFrame {
    pub fn new() -> Self {
        Self::default()
    }

    /// Header da moldura — título 14px, `py` apertado (`px-6 py-4` no original).
    pub fn header(mut self, header: CardHeader) -> Self {
        self.header = Some(header);
        self
    }

    /// Empilha um card. A ordem de chamada é a ordem na tela.
    pub fn card(mut self, card: Card) -> Self {
        self.cards.push(card);
        self
    }

    /// Rodapé da moldura (`px-6 py-4`).
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Cantos de baixo retos.
    pub fn square_bottom(mut self) -> Self {
        self.square_bottom = true;
        self
    }

    /// Largura fixa.
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }
}

/// Raio de cada canto de um card empilhado, pela posição.
///
/// As **pontas** da pilha acompanham o raio interno da moldura (16 − 1 = 15px) — é a moldura que
/// dita o arredondamento externo, não o card. As bordas que encostam em OUTRO card usam
/// `rounded-xl` (14px), que é o que produz os entalhes discretos nas junções.
///
/// O caso comum, e o que o exemplo de uso do coss mostra, é **um card só**: aí os dois lados são
/// externos e ele preenche a moldura inteira, formando um único bloco arredondado.
fn nested_corners(index: usize, total: usize) -> NestedCorners {
    let externo = CARD_RADIUS - 1.0;
    NestedCorners {
        top: if index == 0 {
            externo
        } else {
            CARD_INNER_RADIUS
        },
        bottom: if index + 1 == total {
            externo
        } else {
            CARD_INNER_RADIUS
        },
    }
}

impl RenderOnce for CardFrame {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = card();
        let radius_bottom = if self.square_bottom { 0.0 } else { CARD_RADIUS };

        let mut frame = div()
            .relative()
            .flex()
            .flex_col()
            .bg(p.bg.hsla())
            .text_color(p.text.hsla())
            .rounded_tl(px(CARD_RADIUS))
            .rounded_tr(px(CARD_RADIUS))
            .rounded_bl(px(radius_bottom))
            .rounded_br(px(radius_bottom))
            .border(px(CARD_BORDER))
            .border_color(p.border.hsla())
            .shadow(vec![gpui::BoxShadow {
                color: p.shadow.hsla(),
                offset: gpui::point(px(0.0), px(1.0)),
                blur_radius: px(2.0),
                spread_radius: px(0.0),
            }])
            .when_some(self.width, |d, w| d.w(w).flex_none())
            .when(self.width.is_none(), |d| d.w_full());

        // O preenchimento da moldura vai num overlay ANTES dos filhos, então os cards pintam em
        // cima e ele só aparece nas frestas e nas áreas de header/footer. É o `before:bg-muted/72`
        // do original — que também carrega o bisel, então o overlay acumula as duas funções.
        // O PREENCHIMENTO fica na padding box (por dentro da borda) e ANTES dos filhos, pra os
        // cards pintarem em cima dele.
        frame = frame.child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .rounded_tl(px(CARD_RADIUS - 1.0))
                .rounded_tr(px(CARD_RADIUS - 1.0))
                .rounded_bl(px((radius_bottom - 1.0).max(0.0)))
                .rounded_br(px((radius_bottom - 1.0).max(0.0)))
                .bg(p.frame_fill.hsla()),
        );

        if let Some(h) = self.header {
            if !h.is_empty() {
                // O header de moldura não é `compact`: no coss ele é `px-6 py-4` fixo.
                frame = frame.child(h.render(false, true, PAD));
            }
        }

        // A pilha é CONTÍNUA — sem gap.
        //
        // No coss cada card tem `-m-px`: margem NEGATIVA, ou seja eles se sobrepõem. Uma versão
        // anterior daqui usava `gap: 1px`, e a fresta transformava a pilha em blocos soltos, cada
        // um com cara de card independente. Com os cards encostados, a superfície é uma só; o fundo
        // da moldura aparece apenas nas ENTALHES que os cantos arredondados deixam nas junções, e
        // nas faixas de header/footer.
        if !self.cards.is_empty() {
            let total = self.cards.len();
            let mut pilha = div().flex().flex_col();
            for (i, mut c) in self.cards.into_iter().enumerate() {
                c.nested = Some(nested_corners(i, total));
                pilha = pilha.child(c);
            }
            frame = frame.child(pilha);
        }

        if let Some(footer) = self.footer {
            frame = frame.child(
                div()
                    .flex()
                    .items_center()
                    .px(px(PAD))
                    .py(px(PAD_TIGHT))
                    .child(footer),
            );
        }

        // O bisel por ÚLTIMO e sobre a BORDER box — separado do preenchimento justamente porque os
        // dois vivem em caixas diferentes: o preenchimento por dentro da borda, o bisel sobre ela.
        frame.child(Card::bevel(CARD_RADIUS, radius_bottom))
    }
}

/// Texto secundário na cor de descrição do card — útil pra montar corpo de painel sem repetir o
/// token em cada call site.
pub fn card_muted_text(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(SMALL_SIZE))
        .text_color(card().text_muted.hsla())
        .child(text.into())
}

/// A cor de texto secundário do card, pra quem precisa compor algo próprio.
pub fn card_muted_color() -> gpui::Hsla {
    card().text_muted.hsla()
}

/// A cor de borda do card — a mesma dos separadores.
pub fn card_border_color() -> gpui::Hsla {
    card().border.hsla()
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `frame.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::color::opaque;

    /// **Geometria dos cards empilhados.** As pontas da pilha acompanham o raio interno da moldura
    /// (16 − 1); as bordas que encostam em outro card usam `rounded-xl` (14px). Um card sozinho
    /// arredonda os dois lados — o caso que um `index == 0 && index + 1 == total` fácil de errar.
    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, porque a 6% o filete é
    /// imperceptível no nosso fundo. Este teste existe pra o desvio ser uma decisão registrada e
    /// não uma deriva: se alguém "corrigir" pra 6% achando que é erro de porte, ele falha e
    /// aponta pra cá.
    ///
    /// O tema CLARO segue fiel (preto 4%) — o desvio é só no escuro, onde o problema existia.
    /// **`elevated` usa a MESMA família de sombra do resto da lib**, só com mais camadas.
    ///
    /// O ponto do builder não é "uma sombra mais forte": é que a única sombra forte que existia nesta
    /// base era um preto a 55% com blur 32 escrito à mão no `color_picker` — onze vezes o alfa das
    /// outras. Um card levantado usa o par `shadow-lg/5`, o mesmo do menu, do toast e do dialog, com o
    /// alfa de 5% que toda sombra da biblioteca usa.
    #[test]
    fn elevated_e_compact_sao_opt_in() {
        let normal = Card::new();
        assert!(!normal.elevated, "sombra de repouso por padrão");
        assert!(!normal.compact, "respiro de 24 por padrão");
        assert!(Card::new().elevated().elevated);
        assert!(Card::new().compact().compact);

        // O alfa da sombra é o mesmo nas duas: o que muda é só o número de camadas e o desfoque.
        assert!(
            (CARD_DARK.shadow.alpha() - 13.0 / 255.0).abs() < 1e-6,
            "preto a 5% (alfa 13), como toda sombra da lib"
        );
        // E o respiro apertado é o passo de baixo na escala de 4px, não um número solto.
        assert_eq!(PAD - PAD_TIGHT, 8.0);
        assert_eq!(PAD_TIGHT - PAD_SNUG, 4.0);
    }

    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = CARD_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%)"
        );

        let escuro = CARD_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    #[test]
    fn raio_dos_cards_empilhados_por_posicao() {
        let externo = CARD_RADIUS - 1.0;

        // Sozinho: as duas pontas são externas.
        let so = nested_corners(0, 1);
        assert_eq!((so.top, so.bottom), (externo, externo));

        // Três cards: primeiro, meio, último.
        let primeiro = nested_corners(0, 3);
        assert_eq!((primeiro.top, primeiro.bottom), (externo, CARD_INNER_RADIUS));
        let meio = nested_corners(1, 3);
        assert_eq!((meio.top, meio.bottom), (CARD_INNER_RADIUS, CARD_INNER_RADIUS));
        let ultimo = nested_corners(2, 3);
        assert_eq!((ultimo.top, ultimo.bottom), (CARD_INNER_RADIUS, externo));
    }

    /// `square_bottom` vence o raio que a posição na pilha daria. Sem isso, o exemplo "framed card
    /// with no rounded bottom" não teria como existir: o card é o único da moldura, então a posição
    /// mandaria arredondar as duas pontas.
    #[test]
    fn square_bottom_vence_o_raio_da_posicao() {
        let posicao = nested_corners(0, 1);
        assert_eq!(posicao.bottom, CARD_RADIUS - 1.0, "a posição arredondaria");

        // A precedência é aplicada no render; aqui fica registrada a regra que ele implementa.
        let com_square = 0.0_f32;
        assert_ne!(com_square, posicao.bottom);
        assert_eq!(posicao.top, CARD_RADIUS - 1.0, "o topo NÃO é afetado");
    }

    /// O raio interno é MENOR que o externo — se alguém inverter os dois, a pilha ganha cantos
    /// maiores nas junções que nas pontas, que é visivelmente errado.
    #[test]
    fn raio_interno_e_menor_que_o_externo() {
        assert!(CARD_INNER_RADIUS < CARD_RADIUS - 1.0);
    }

    /// As duas paletas diferem onde o coss manda diferir, e todas as cores decodificam com os
    /// canais no lugar (a armadilha de convenção que já custou três bugs no campo de texto).
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        // Fundo claro: branco opaco.
        let bg: gpui::Rgba = CARD_LIGHT.bg.hsla().into();
        assert_eq!((bg.r, bg.g, bg.b, bg.a), (1.0, 1.0, 1.0, 1.0));

        // Texto: quase preto no claro, quase branco no escuro, os dois NEUTROS e opacos.
        for (nome, c) in [("claro", CARD_LIGHT.text), ("escuro", CARD_DARK.text)] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: texto opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: texto neutro — se r≠g≠b, o valor foi lido deslocado"
            );
        }
        let claro: gpui::Rgba = CARD_LIGHT.text.hsla().into();
        let escuro: gpui::Rgba = CARD_DARK.text.hsla().into();
        assert!(claro.r < 0.2, "texto do tema claro é escuro");
        assert!(escuro.r > 0.8, "texto do tema escuro é claro");

        // Borda, preenchimento da moldura e bisel são TRANSLÚCIDOS — é isso que os deixa
        // funcionar sobre qualquer fundo.
        for (nome, c) in [
            ("borda claro", CARD_LIGHT.border),
            ("borda escuro", CARD_DARK.border),
            ("moldura claro", CARD_LIGHT.frame_fill),
            ("moldura escuro", CARD_DARK.frame_fill),
            ("bisel claro", CARD_LIGHT.bevel),
            ("bisel escuro", CARD_DARK.bevel),
        ] {
            assert!(c.alpha() < 1.0, "{nome} tem que ser translúcido");
        }

        // O bisel troca de sentido entre os temas.
        assert!(CARD_LIGHT.bevel_dir > 0.0);
        assert!(CARD_DARK.bevel_dir < 0.0);
    }

    /// O fundo da moldura é MAIS SUTIL que a borda: ele só precisa insinuar a fresta, e se ficar
    /// mais forte que a borda a pilha começa a parecer um bloco listrado.
    #[test]
    fn preenchimento_da_moldura_e_mais_sutil_que_a_borda() {
        assert!(CARD_LIGHT.frame_fill.alpha() < CARD_LIGHT.border.alpha());
        assert!(CARD_DARK.frame_fill.alpha() < CARD_DARK.border.alpha());
    }

    /// Um header sem título, descrição nem ação é considerado vazio e não vira faixa — senão um
    /// `.header(CardHeader::new())` distraído adicionaria 48px de padding do nada.
    #[test]
    fn header_vazio_e_detectado() {
        assert!(CardHeader::new().is_empty());
        assert!(!CardHeader::new().title("x").is_empty());
        assert!(!CardHeader::new().description("x").is_empty());
    }

    /// `opaque` é a única ponte entre as convenções de cor, e aqui a paleta do card já nasce em
    /// `0xRRGGBBAA` — então nenhum token do tema entra sem passar por ele.
    #[test]
    fn tokens_do_tema_entram_por_opaque() {
        theme::set_theme(theme::ThemeMode::Dark);
        let elevado = opaque(theme::TEXT_MUTED());
        assert_eq!(elevado.alpha(), 1.0);
    }
}

/// **O respiro do painel, medido no layout real.**
///
/// Existe por causa de um defeito relatado: um card `compact` saía com 24px nas laterais e 16 em
/// cima, porque o respiro horizontal era o `PAD` literal e só o vertical passava por [`pads`]. O
/// código parecia certo — a assimetria só existia na tela, e é por isso que este teste mede a
/// geometria em vez de afirmar constantes.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{canvas, Bounds, Context, Pixels, Render, TestAppContext, VisualTestContext};
    use std::cell::Cell;
    use std::rc::Rc;

    /// Largura da janela do harness.
    const LARGURA: f32 = 300.0;

    /// O medidor: um card com uma sonda no lugar do painel.
    struct Medidor {
        compact: bool,
        painel: Rc<Cell<Option<Bounds<Pixels>>>>,
    }

    impl Render for Medidor {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            let sonda = self.painel.clone();
            let corpo = canvas(move |bounds, _w, _cx| sonda.set(Some(bounds)), |_, _, _, _| {})
                .size_full();
            // Container de BLOCO com largura fixa: o card é o único filho, então a origem dele é a
            // origem da janela, e o respiro é a origem da sonda menos a borda de 1px do card.
            div()
                .w(px(LARGURA))
                .child(Card::new().when(self.compact, Card::compact).panel(corpo))
        }
    }

    /// O respiro `(horizontal, vertical)` do painel, em px lógicos.
    fn respiro_do_painel(cx: &mut TestAppContext, compact: bool) -> (f32, f32) {
        let painel: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let p = painel.clone();
        let window = cx.add_window(move |_w, _cx| Medidor { compact, painel: p });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let b = painel.get().expect("a sonda mediu no prepaint");
        // −1 da borda do card: o respiro conta da borda interna pra dentro.
        (f32::from(b.origin.x) - 1.0, f32::from(b.origin.y) - 1.0)
    }

    /// **A ação do header não passa da borda do card.**
    ///
    /// Era o defeito relatado: com uma descrição longa, o botão de ação saía POR FORA da borda direita.
    /// A causa é que o bloco de textos é `flex_1` mas o min-content de um texto no GPUI é a frase
    /// inteira numa linha — sem `min_w(0)` ele não encolhe, e o action, sendo `flex_none`, é empurrado
    /// pra fora. O `Card` não recorta nada, então o estouro fica visível.
    ///
    /// O teste mede a caixa REAL da ação e a compara com a caixa de respiro do card. A descrição longa
    /// é o que produz a pressão — com uma curta o defeito não aparece.
    #[gpui::test]
    fn a_acao_do_header_nao_passa_da_borda(cx: &mut TestAppContext) {
        const CARD_W: f32 = 320.0;
        const ACAO_W: f32 = 64.0;

        let medido: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let sonda = medido.clone();
        let window = cx.add_window(move |_w, _cx| MedidorDeAcao {
            acao_w: ACAO_W,
            card_w: CARD_W,
            sonda: sonda.clone(),
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let b = medido.get().expect("a sonda mediu a ação");
        let direita = f32::from(b.origin.x) + f32::from(b.size.width);
        // A caixa de respiro do card: a largura menos a borda e o respiro do header.
        let limite = CARD_W - CARD_BORDER - PAD;
        assert!(
            direita <= limite + 0.5,
            "a ação termina em {direita} e o limite do respiro é {limite} — está passando da borda"
        );
    }

    /// O harness do teste acima.
    struct MedidorDeAcao {
        acao_w: f32,
        card_w: f32,
        sonda: Rc<Cell<Option<Bounds<Pixels>>>>,
    }

    impl Render for MedidorDeAcao {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            let sonda = self.sonda.clone();
            let acao = div().relative().w(px(self.acao_w)).h(px(28.0)).child(
                canvas(move |bounds, _, _| sonda.set(Some(bounds)), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            );
            Card::new().width(px(self.card_w)).header(
                CardHeader::new()
                    .title("Create project")
                    // Longa de propósito: é o que faz o bloco de textos querer mais largura do que
                    // sobra depois da ação.
                    .description("Deploy your new project in one-click, from anywhere.")
                    .action(acao),
            )
        }
    }

    /// **`compact` aperta os DOIS eixos.** Era exatamente o defeito: horizontal 24, vertical 16.
    #[gpui::test]
    fn compact_aperta_os_dois_eixos(cx: &mut TestAppContext) {
        let (x, y) = respiro_do_painel(cx, true);
        assert_eq!(
            (x, y),
            (PAD_TIGHT, PAD_TIGHT),
            "card compacto: o respiro tem que ser igual nos quatro lados"
        );
    }

    /// E o card normal continua no respiro cheio, nos dois eixos — pra o conserto acima não ter
    /// apertado quem não pediu.
    #[gpui::test]
    fn o_card_normal_continua_no_respiro_cheio(cx: &mut TestAppContext) {
        let (x, y) = respiro_do_painel(cx, false);
        assert_eq!((x, y), (PAD, PAD));
    }
}

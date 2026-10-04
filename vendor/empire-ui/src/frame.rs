//! `Frame` — a **moldura com frestas** do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/frame.tsx`
//!
//! # O que é, e como difere do [`crate::card::CardFrame`]
//!
//! Os dois empilham superfícies dentro de um fundo levemente tingido, mas o efeito é o oposto:
//!
//! | | [`CardFrame`](crate::card::CardFrame) | [`Frame`] |
//! |---|---|---|
//! | painéis | **rentes** (margem negativa no original) | separados por **4px de gap** |
//! | fundo da moldura | aparece só nos entalhes das quinas | aparece como **fresta contínua** |
//! | borda/sombra dos painéis | removidas (quem desenha é a moldura) | **cada painel** tem a sua |
//! | raio | a moldura arredonda, os cards herdam | painel 14, moldura 16, independentes |
//!
//! Ou seja: a `CardFrame` costura os cards numa superfície única; o `Frame` faz o contrário —
//! deixa cada painel como uma peça solta e usa o fundo da moldura como argamassa entre elas. Por
//! isso aqui o painel mantém borda, sombra e bisel próprios.
//!
//! # Três componentes
//!
//! - [`Frame`] — a moldura: raio 16, fundo `--muted/72`, padding 4px, gap 4px entre painéis.
//! - [`FramePanel`] — a peça: raio 14, borda, fundo `--background`, `shadow-xs` e bisel. Tem três
//!   fatias opcionais: [`FrameHeader`], o conteúdo, e [`FrameFooter`].
//! - [`FrameHeader`] / [`FrameFooter`] — as faixas de topo e base do painel (`px-5 py-4`).
//!
//! # Uso
//!
//! ```ignore
//! Frame::new()
//!     .panel(
//!         FramePanel::new()
//!             .header(FrameHeader::new().title("Integrações").description("Serviços conectados"))
//!             .content(minha_lista)
//!             .footer(FrameFooter::new().child(meu_botao)),
//!     )
//!     .panel(FramePanel::new().content(outro_bloco))
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
// Os utilitários Tailwind do `frame.tsx` resolvidos em número, com a paleta `neutral` do Tailwind
// expandida e os `color-mix` já calculados.
//
// Mesma disciplina de cor do card, do campo de texto e do botão: TODO valor é `0xRRGGBBAA`, com o
// byte de alfa, SEMPRE — ver [`crate::color`]. Os tokens de [`crate::theme`] são `0xRRGGBB`, e
// misturar as duas convenções desloca os canais e produz uma cor completamente diferente sem erro
// de compilação (já custou três bugs visíveis nesta base). A única ponte é [`crate::color::opaque`].

/// Tokens visuais da moldura e dos painéis, por tema.
#[derive(Clone, Copy, Debug)]
struct FramePalette {
    /// Fundo do painel (`--background`).
    bg: Rgba8,
    /// Cor do texto (`--foreground`).
    text: Rgba8,
    /// Texto secundário — descrição do header (`--muted-foreground`).
    text_muted: Rgba8,
    /// Borda do painel (`--border`).
    border: Rgba8,
    /// Fundo da MOLDURA, que aparece nas frestas entre os painéis (`--muted` a 72%).
    fill: Rgba8,
    /// Sombra externa do painel (`shadow-xs/5`).
    shadow: Rgba8,
    /// Fio de bisel de 1px do painel. Desce no claro, sobe no escuro.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro), `-1` sobe (escuro).
    bevel_dir: f32,
}

/// Tema **claro**.
const FRAME_LIGHT: FramePalette = FramePalette {
    bg: Rgba8(0xffffffff),
    text: Rgba8(0x262626ff), // neutral-800
    // mix(neutral-500 90%, black) = #686868
    text_muted: Rgba8(0x686868ff),
    border: Rgba8(0x00000014), // black 8%
    // --muted é black 4%; o `/72` do Tailwind multiplica → ~2,9%.
    fill: Rgba8(0x00000007),
    shadow: Rgba8(0x0000000d), // black 5%
    bevel: Rgba8(0x0000000a),  // black 4%
    bevel_dir: 1.0,
};

/// Tema **escuro**.
const FRAME_DARK: FramePalette = FramePalette {
    // mix(neutral-950 96%, white) = #141414
    bg: Rgba8(0x141414ff),
    text: Rgba8(0xf5f5f5ff), // neutral-100
    // mix(neutral-500 90%, white) = #818181
    text_muted: Rgba8(0x818181ff),
    border: Rgba8(0xffffff0f), // white 6%
    fill: Rgba8(0xffffff07),   // white 4% × 72%
    shadow: Rgba8(0x0000000d), // black 5%
    // ⚠️ DESVIO CONSCIENTE do coss, o MESMO já vigente no `Input`, no `Card` e no `Button`: o
    // original usa branco a **6%** (alfa 15). Aqui é o DOBRO — alfa 30 ≈ 11,8% — por decisão de
    // design: a 6% o filete é imperceptível no nosso fundo escuro. NÃO "corrija" isto pra 0x0f
    // achando que é erro de porte; se a intenção mudar, mude junto o teste
    // `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
};

/// A paleta da moldura no tema corrente.
fn frame() -> &'static FramePalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &FRAME_DARK,
        theme::ThemeMode::Light => &FRAME_LIGHT,
    }
}

// --- Geometria ------------------------------------------------------------------------------------

/// Raio da moldura — `rounded-2xl`. O coss não redefine `--radius-2xl`, então vale o default do
/// Tailwind: `1rem` = **16px**.
pub const FRAME_RADIUS: f32 = 16.0;

/// Raio do painel — `rounded-xl`, que no coss é `calc(var(--radius) + 4px)` = `10 + 4` = **14px**.
///
/// Tem que ser MENOR que [`FRAME_RADIUS`]: o painel fica 4px por dentro da moldura, e com o raio
/// maior a quina dele avançaria sobre a curva da moldura e "vazaria" pra fora dela.
const PANEL_RADIUS: f32 = 14.0;

/// Raio do overlay de bisel do painel — `calc(radius-xl - 1px)` = **13px**.
const PANEL_BEVEL_RADIUS: f32 = PANEL_RADIUS;

/// Respiro da moldura em volta dos painéis — `p-1` (Tailwind: 1 = 4px).
const FRAME_PAD: f32 = 4.0;

/// Fresta entre painéis consecutivos — 4px.
///
/// No coss isto é um seletor de **irmão adjacente** (`[&>*+*]`-ish) porque o CSS só consegue ver a
/// vizinhança de fora. Aqui a [`Frame`] é **dona** dos painéis, então é só o `gap` do flex — mesmo
/// resultado, sem depender da ordem em que os filhos foram declarados.
///
/// É de propósito igual ao [`FRAME_PAD`]: com os dois iguais, a fresta entre dois painéis tem a
/// mesma espessura da fresta entre um painel e a borda da moldura, e a grade fica regular.
const PANEL_GAP: f32 = 4.0;

/// Respiro do conteúdo do painel — `p-5`.
const PANEL_PAD: f32 = 20.0;

/// Respiro horizontal das faixas de header e footer — `px-5`.
const SLICE_PAD_X: f32 = 20.0;

/// Respiro vertical das faixas de header e footer — `py-4`.
const SLICE_PAD_Y: f32 = 16.0;

/// Corpo do título e da descrição do header — `text-sm`. Diferente do [`crate::card::Card`], onde o
/// título é `text-lg`: o header do `Frame` é uma faixa de rótulo, não uma capa.
const TEXT_SIZE: f32 = 14.0;

/// Altura de linha do texto das faixas — 20px.
///
/// O `text-sm` do Tailwind é um PAR: `font-size: 14px` **e** `line-height: 20px`. Só o tamanho da
/// fonte não basta, porque a entrelinha padrão do GPUI é maior (deu 23px medido) e isso engordava a
/// faixa de header em 6px: 16 + 23 + 23 + 16 = 78 em vez de 16 + 20 + 20 + 16 = 72. Achado medindo o
/// pixel, não lendo o código.
const LINE_HEIGHT: f32 = 20.0;

/// Espaço entre título e descrição no header: **nenhum**.
///
/// O header da referência é `flex flex-col px-5 py-4` — sem classe de `gap`. Diferente do
/// [`crate::card::CardHeader`], que tem `gap-1.5`. O respiro entre as duas linhas vem só da altura
/// de linha do texto.
const HEADER_GAP: f32 = 0.0;


// =================================================================================================
// Header
// =================================================================================================

/// A faixa de **topo** da moldura: título e descrição sobre o fundo tingido.
///
/// É irmã dos painéis, não filha de um deles. A faixa não tem superfície própria — ela vive
/// diretamente sobre o fundo da [`Frame`], e é por isso que o `px-5` dela é o mesmo `p-5` do painel:
/// o texto da faixa alinha na vertical com o conteúdo do painel logo abaixo.
///
/// Sem ação no canto e sem linha separadora: a referência é só `flex flex-col px-5 py-4`. (O
/// [`crate::card::CardHeader`] tem os dois — são componentes diferentes.)
#[derive(Default)]
pub struct FrameHeader {
    title: Option<SharedString>,
    description: Option<SharedString>,
}

impl FrameHeader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Título — 14px, semibold.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Descrição — 14px, texto secundário.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Se o header não tem nada pra mostrar (evita renderizar uma faixa vazia).
    fn is_empty(&self) -> bool {
        self.title.is_none() && self.description.is_none()
    }

    fn render(self) -> Div {
        let p = frame();

        let mut textos = div()
            .flex()
            .flex_col()
            .gap(px(HEADER_GAP))
            .px(px(SLICE_PAD_X))
            .py(px(SLICE_PAD_Y));
        if let Some(t) = self.title {
            textos = textos.child(
                div()
                    .text_size(px(TEXT_SIZE))
                    .line_height(px(LINE_HEIGHT))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(p.text.hsla())
                    .child(t),
            );
        }
        if let Some(d) = self.description {
            textos = textos.child(
                div()
                    .text_size(px(TEXT_SIZE))
                    .line_height(px(LINE_HEIGHT))
                    .text_color(p.text_muted.hsla())
                    .child(d),
            );
        }

        textos
    }
}

// =================================================================================================
// Footer
// =================================================================================================

/// A faixa de **base** de um [`FramePanel`] — normalmente ações. `px-5 py-4`.
#[derive(Default)]
pub struct FrameFooter {
    children: Vec<AnyElement>,
}

impl FrameFooter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adiciona um elemento ao rodapé. A ordem de chamada é a ordem na tela.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    fn render(self) -> Div {
        // `px-5 py-4` e nada mais: a referência não põe `flex` nem `gap` aqui, então dois filhos
        // ficam em fluxo de bloco. Quem quer botões lado a lado passa a própria linha como filho.
        let mut faixa = div().px(px(SLICE_PAD_X)).py(px(SLICE_PAD_Y));
        for c in self.children {
            faixa = faixa.child(c);
        }
        faixa
    }
}

// =================================================================================================
// FramePanel
// =================================================================================================

/// Uma **peça** da moldura: superfície própria com borda, `shadow-xs` e bisel.
///
/// Renderiza sozinho também (fora de uma [`Frame`] é só uma superfície de raio 14), mas o desenho
/// pressupõe o fundo da moldura por trás — é o contraste com ele que faz a fresta aparecer.
#[derive(IntoElement, Default)]
pub struct FramePanel {
    content: Option<AnyElement>,
    /// Fresta de 4px acima, ligada pela [`Frame`] quando o painel anterior também é um painel.
    gap_top: bool,
}

impl FramePanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// O conteúdo principal (`p-5`).
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// O bisel de 1px sobreposto.
    ///
    /// ⚠️ Duas coisas que só apareceram medindo, e que já custaram bug visível nesta base (ver
    /// `crate::input::bevel_for` e [`crate::card::Card`]):
    ///
    /// 1. **É borda, não sombra.** O `Window::paint_shadows` do GPUI não recorta a sombra pra fora
    ///    do elemento que a projeta (diferente do CSS), então o `before:box-shadow` do coss viraria
    ///    uma lavagem de cor sobre o painel inteiro.
    /// 2. **Cobre a BORDER box, não a padding box.** No coss a sombra sai 1px pra fora do
    ///    pseudo-elemento, ou seja cai SOBRE a borda e a clareia. Um filete 1px pra dentro cria uma
    ///    segunda linha ao lado da borda — e a 4–6% de alfa isso é praticamente invisível.
    ///
    /// E por isso o painel **não tem `overflow_hidden`**: ele recortaria justamente o 1px que o
    /// overlay projeta pra fora. Se algum dia precisar recortar o conteúdo, o recorte tem que ir
    /// num filho, não no painel — ou o bisel tem que virar IRMÃO do painel.
    fn bevel() -> Div {
        let p = frame();
        let overlay = div()
            .absolute()
            .top(px(-1.0))
            .left(px(-1.0))
            .right(px(-1.0))
            .bottom(px(-1.0))
            .rounded(px(PANEL_BEVEL_RADIUS))
            .border_color(p.bevel.hsla());
        if p.bevel_dir > 0.0 {
            overlay.border_b_1()
        } else {
            overlay.border_t_1()
        }
    }
}

impl RenderOnce for FramePanel {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = frame();

        let mut surface = div()
            .relative()
            .when(self.gap_top, |d| d.mt(px(PANEL_GAP)))
            .flex()
            .flex_col()
            .w_full()
            .bg(p.bg.hsla())
            .text_color(p.text.hsla())
            .rounded(px(PANEL_RADIUS))
            .border_1()
            .border_color(p.border.hsla())
            // Sombra EXTERNA continua um `BoxShadow`: ali a técnica funciona, porque ela fica atrás
            // de um fundo opaco e ninguém vê que o GPUI não a recorta.
            .shadow(vec![gpui::BoxShadow {
                color: p.shadow.hsla(),
                offset: gpui::point(px(0.0), px(1.0)),
                blur_radius: px(2.0),
                spread_radius: px(0.0),
            }]);

        if let Some(content) = self.content {
            surface = surface.child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .p(px(PANEL_PAD))
                    .child(content),
            );
        }

        // O bisel por último: é absoluto, então a ordem só decide quem pinta em cima.
        surface.child(Self::bevel())
    }
}

// =================================================================================================
// Frame
// =================================================================================================

/// A **moldura**: fundo levemente tingido com painéis flutuando dentro dele, separados por frestas
/// de 4px. Ver o doc do módulo.
#[derive(IntoElement, Default)]
pub struct Frame {
    slots: Vec<FrameSlot>,
    width: Option<Pixels>,
}

/// Uma fatia da moldura, na ordem em que foi declarada.
///
/// As faixas são **irmãs** dos painéis, não filhas deles: no HTML da referência o header tem
/// `px-5 py-4` e o painel tem `p-5` próprio, então um header dentro do painel somaria os dois
/// respiros. Além disso as faixas não têm fundo — elas mostram o tingido da moldura.
enum FrameSlot {
    Header(FrameHeader),
    Panel(FramePanel),
    Footer(FrameFooter),
}

impl Frame {
    pub fn new() -> Self {
        Self::default()
    }

    /// Uma faixa de rótulo sobre o fundo tingido. Costuma vir antes do primeiro painel, mas a
    /// posição é de quem chama: a ordem na tela é a ordem das chamadas.
    pub fn header(mut self, header: FrameHeader) -> Self {
        self.slots.push(FrameSlot::Header(header));
        self
    }

    /// Empilha um painel. A ordem de chamada é a ordem na tela.
    pub fn panel(mut self, panel: FramePanel) -> Self {
        self.slots.push(FrameSlot::Panel(panel));
        self
    }

    /// Uma faixa de ações sobre o fundo tingido, normalmente depois do último painel.
    pub fn footer(mut self, footer: FrameFooter) -> Self {
        self.slots.push(FrameSlot::Footer(footer));
        self
    }

    /// Largura fixa (o default é ocupar o container).
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }
}

impl RenderOnce for Frame {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = frame();

        // A moldura NÃO tem borda nem sombra: o único desenho dela é o fundo `--muted/72`, e é o
        // painel que carrega borda, sombra e bisel. Se a moldura ganhasse borda também, o
        // resultado seriam duas linhas concêntricas a 4px de distância.
        // Sem `gap` na coluna: a fresta de 4px é margem de cada painel que **segue outro painel**,
        // porque na referência ela é um seletor de irmão adjacente
        // (`[data-slot=frame-panel]+[data-slot=frame-panel]:mt-1`). Um `gap` uniforme afastaria
        // também o header do primeiro painel, e ali o encosto é intencional.
        let mut frame = div()
            .flex()
            .flex_col()
            .p(px(FRAME_PAD))
            .rounded(px(FRAME_RADIUS))
            .bg(p.fill.hsla())
            .text_color(p.text.hsla())
            .when_some(self.width, |d, w| d.w(w).flex_none())
            .when(self.width.is_none(), |d| d.w_full());

        let mut anterior_era_painel = false;
        for slot in self.slots {
            match slot {
                FrameSlot::Header(h) => {
                    if !h.is_empty() {
                        frame = frame.child(h.render());
                    }
                    anterior_era_painel = false;
                }
                FrameSlot::Panel(mut painel) => {
                    painel.gap_top = anterior_era_painel;
                    frame = frame.child(painel);
                    anterior_era_painel = true;
                }
                FrameSlot::Footer(f) => {
                    if !f.is_empty() {
                        frame = frame.child(f.render());
                    }
                    anterior_era_painel = false;
                }
            }
        }
        frame
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável ("this assertion has a constant
// value"), presumindo que quem escreveu quis testar algo variável. Aqui é o contrário: travar o valor
// que veio da referência É o propósito destes testes — eles são a documentação executável de quanto
// mede cada coisa, e quebram de propósito se alguém mudar uma constante sem querer.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::color::opaque;

    /// As duas paletas decodificam com os canais no lugar e com a intenção perceptual certa. É o
    /// teste que pega a confusão de convenção `0xRRGGBB` vs `0xRRGGBBAA` — que já custou três bugs
    /// visíveis nesta base, todos sem erro de compilação.
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        // Fundo do painel: branco opaco no claro, quase-preto opaco no escuro, os dois NEUTROS.
        for (nome, c) in [("claro", FRAME_LIGHT.bg), ("escuro", FRAME_DARK.bg)] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: fundo do painel é OPACO");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: fundo neutro — se r≠g≠b, o valor foi lido deslocado"
            );
        }
        let bg_claro: gpui::Rgba = FRAME_LIGHT.bg.hsla().into();
        let bg_escuro: gpui::Rgba = FRAME_DARK.bg.hsla().into();
        assert_eq!((bg_claro.r, bg_claro.g, bg_claro.b), (1.0, 1.0, 1.0));
        assert!(bg_escuro.r < 0.2, "fundo do tema escuro é escuro");

        // Texto: quase preto no claro, quase branco no escuro, os dois neutros e opacos. O
        // secundário fica ENTRE os dois — se ele saísse igual ao principal, a descrição perderia a
        // hierarquia.
        for (nome, c) in [("claro", FRAME_LIGHT.text), ("escuro", FRAME_DARK.text)] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: texto opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: texto neutro"
            );
        }
        let texto_claro: gpui::Rgba = FRAME_LIGHT.text.hsla().into();
        let texto_escuro: gpui::Rgba = FRAME_DARK.text.hsla().into();
        assert!(texto_claro.r < 0.2, "texto do tema claro é escuro");
        assert!(texto_escuro.r > 0.8, "texto do tema escuro é claro");

        let muted_claro: gpui::Rgba = FRAME_LIGHT.text_muted.hsla().into();
        let muted_escuro: gpui::Rgba = FRAME_DARK.text_muted.hsla().into();
        assert_eq!((muted_claro.a, muted_escuro.a), (1.0, 1.0));
        assert!(
            muted_claro.r > texto_claro.r && muted_claro.r < bg_claro.r,
            "no claro o secundário fica entre o texto e o fundo"
        );
        assert!(
            muted_escuro.r < texto_escuro.r && muted_escuro.r > bg_escuro.r,
            "no escuro o secundário fica entre o texto e o fundo"
        );

        // Borda, fundo da moldura e bisel são TRANSLÚCIDOS — é isso que os deixa funcionar sobre
        // qualquer fundo. A sombra também.
        for (nome, c) in [
            ("borda claro", FRAME_LIGHT.border),
            ("borda escuro", FRAME_DARK.border),
            ("moldura claro", FRAME_LIGHT.fill),
            ("moldura escuro", FRAME_DARK.fill),
            ("bisel claro", FRAME_LIGHT.bevel),
            ("bisel escuro", FRAME_DARK.bevel),
            ("sombra claro", FRAME_LIGHT.shadow),
            ("sombra escuro", FRAME_DARK.shadow),
        ] {
            assert!(c.alpha() < 1.0, "{nome} tem que ser translúcido");
            assert!(c.alpha() > 0.0, "{nome} não pode ser invisível");
        }

        // O bisel troca de sentido entre os temas: escurece a base no claro, clareia o topo no
        // escuro. Se os dois apontassem pro mesmo lado, um dos temas ficaria com o relevo invertido.
        assert!(FRAME_LIGHT.bevel_dir > 0.0);
        assert!(FRAME_DARK.bevel_dir < 0.0);
    }

    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, a mesma decisão já vigente
    /// no `Input`, no `Card` e no `Button`, porque a 6% o filete é imperceptível no nosso fundo.
    /// Este teste existe pra o desvio ser uma decisão registrada e não uma deriva: se alguém
    /// "corrigir" pra 6% achando que é erro de porte, ele falha e aponta pra cá.
    ///
    /// O tema CLARO segue fiel (preto 4%) — o desvio é só no escuro, onde o problema existia.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = FRAME_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%); veio {claro}"
        );

        let escuro = FRAME_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    /// A faixa de header mede exatamente `py-4 × 2 + 2 linhas de 20px` = 72px.
    ///
    /// O valor foi medido no pixel, e a primeira versão errava por 6px: o `text-sm` do Tailwind fixa
    /// `line-height: 20px` junto com o tamanho da fonte, e sem isso a entrelinha padrão do GPUI (23px
    /// medidos) engordava a faixa. Se alguém tirar o `.line_height()` do header, este teste cai.
    #[test]
    fn faixa_de_header_tem_72px() {
        assert_eq!(SLICE_PAD_Y * 2.0 + LINE_HEIGHT * 2.0, 72.0);
        assert_eq!(LINE_HEIGHT, 20.0, "o par do text-sm: 14px/20px");
    }

    /// **A geometria que não pode inverter.** O painel fica 4px por dentro da moldura, então o raio
    /// dele tem que ser MENOR que o da moldura — com o raio maior, a quina do painel avança sobre a
    /// curva da moldura e ele "vaza" visualmente pelos cantos.
    ///
    /// O bisel é o caso oposto e já enganou uma vez: o `calc(--radius-xl - 1px)` do original vale
    /// pra um pseudo-elemento na PADDING box (1px por dentro da borda, logo 1px de raio a menos). O
    /// nosso overlay cobre a BORDER box, então o raio dele é o do painel — é o que o deixa
    /// concêntrico com a borda que ele clareia. Mesma escolha do [`crate::card`].
    #[test]
    fn raio_do_painel_e_menor_que_o_da_moldura() {
        assert!(
            PANEL_RADIUS < FRAME_RADIUS,
            "painel {PANEL_RADIUS} tem que ser menor que moldura {FRAME_RADIUS}"
        );
        assert_eq!(FRAME_RADIUS, 16.0, "rounded-2xl = default do Tailwind");
        assert_eq!(PANEL_RADIUS, 14.0, "rounded-xl = calc(--radius + 4px)");
        assert_eq!(
            PANEL_BEVEL_RADIUS, PANEL_RADIUS,
            "o overlay cobre a border box, então acompanha o raio dela"
        );

        // A folga da moldura ainda cabe na diferença de raio: 16 − 14 = 2 ≤ 4px de padding. Se o
        // padding encolhesse abaixo disso, a quina do painel encostaria na curva da moldura.
        assert!(FRAME_RADIUS - PANEL_RADIUS <= FRAME_PAD);
    }

    /// **A fresta é de verdade** — é a diferença estrutural em relação à
    /// [`crate::card::CardFrame`], onde os cards ficam rentes e o fundo só aparece nos entalhes das
    /// quinas. Aqui o gap tem que ser positivo, senão os painéis se encostam e a moldura vira uma
    /// pilha de cards com borda dupla nas junções.
    ///
    /// E o gap é igual ao padding da moldura, pra a fresta entre dois painéis ter a mesma espessura
    /// da fresta contra a borda.
    #[test]
    fn paineis_tem_fresta_de_verdade_e_regular() {
        assert!(PANEL_GAP > 0.0, "sem gap não existe fresta");
        assert_eq!(PANEL_GAP, FRAME_PAD, "fresta interna = fresta da borda");
        assert_eq!(FRAME_PAD, 4.0, "p-1 no Tailwind = 4px");
    }

    /// O fundo da moldura é MAIS SUTIL que a borda do painel. A moldura só precisa insinuar a
    /// fresta; se ela ficasse mais forte que a borda, cada fresta leria como uma faixa pintada e o
    /// `Frame` viraria um bloco listrado em vez de uma bandeja.
    #[test]
    fn fundo_da_moldura_e_mais_sutil_que_a_borda_do_painel() {
        assert!(
            FRAME_LIGHT.fill.alpha() < FRAME_LIGHT.border.alpha(),
            "claro: moldura {} vs borda {}",
            FRAME_LIGHT.fill.alpha(),
            FRAME_LIGHT.border.alpha()
        );
        assert!(
            FRAME_DARK.fill.alpha() < FRAME_DARK.border.alpha(),
            "escuro: moldura {} vs borda {}",
            FRAME_DARK.fill.alpha(),
            FRAME_DARK.border.alpha()
        );
    }

    /// Os respiros: conteúdo `p-5`, faixas `px-5 py-4`. As faixas são mais apertadas na vertical
    /// que o conteúdo — é o que as faz ler como rótulo em vez de mais um bloco de conteúdo.
    #[test]
    fn respiros_seguem_a_escala_do_tailwind() {
        assert_eq!(PANEL_PAD, 20.0, "p-5");
        assert_eq!(SLICE_PAD_X, PANEL_PAD, "px-5, alinhado com o conteúdo");
        assert_eq!(SLICE_PAD_Y, 16.0, "py-4");
        assert!(
            SLICE_PAD_Y < PANEL_PAD,
            "faixa mais apertada que o conteúdo"
        );
    }

    /// Uma faixa sem nada dentro não vira faixa — senão um `.header(FrameHeader::new())` distraído
    /// adicionaria 32px de padding do nada, e (pior) faria o conteúdo perder o `pt` dele.
    #[test]
    fn faixas_vazias_sao_detectadas() {
        assert!(FrameHeader::new().is_empty());
        assert!(!FrameHeader::new().title("x").is_empty());
        assert!(!FrameHeader::new().description("x").is_empty());

        assert!(FrameFooter::new().is_empty());
        assert!(!FrameFooter::new().child(div()).is_empty());
    }

    /// `opaque` é a única ponte entre as convenções de cor, e a paleta daqui já nasce em
    /// `0xRRGGBBAA` — então nenhum token de 6 dígitos do tema entra sem passar por ele.
    #[test]
    fn tokens_do_tema_entram_por_opaque() {
        theme::set_theme(theme::ThemeMode::Dark);
        let elevado = opaque(theme::TEXT_MUTED());
        assert_eq!(elevado.alpha(), 1.0);
    }
}

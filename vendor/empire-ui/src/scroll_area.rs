//! `scroll_area` — a área de rolagem do [coss][1], com barra que se esconde e fade nas bordas.
//!
//! [1]: https://github.com/cosscom/coss/blob/main/apps/ui/registry/default/ui/scroll-area.tsx
//!
//! # O que ela é
//!
//! Um contêiner que rola o conteúdo e desenha **por cima** dele:
//!
//! - uma **barra de 6px** encostada na borda, com polegar totalmente arredondado em
//!   `--foreground/20`, que fica **invisível em repouso** e aparece ao passar o mouse na área ou
//!   enquanto se rola;
//! - um **fade de 24px** em cada borda que tem conteúdo escondido além dela — e o fade CRESCE com a
//!   rolagem, de 0 até 24px, em vez de aparecer inteiro de uma vez.
//!
//! # Desvio deliberado: o fade vem LIGADO
//!
//! Na referência `scrollFade` é `false` por padrão. Aqui é [`true`][ScrollArea::fade] — foi pedido
//! explicitamente. Todo o resto segue o original; este é o único default divergente, e o teste
//! [`tests::fade_vem_ligado_por_padrao`] existe pra que ninguém o "conserte" achando que é erro de
//! porte.
//!
//! # O fade é gradiente, não máscara — e isso tem consequência
//!
//! A referência usa `mask-image`: o conteúdo fica **transparente** nas bordas, deixando ver o que
//! houver atrás, qualquer que seja. O GPUI não tem máscara de alfa — o `ContentMask` dele é recorte
//! retangular, não gradiente.
//!
//! Aqui o fade é um retângulo com gradiente da cor de fundo até a mesma cor com alfa 0, pintado sobre
//! o conteúdo. O efeito é idêntico **enquanto a cor do fade for a cor que está atrás**. Por isso
//! existe [`ScrollArea::fade_color`]: o default é `--background`, que serve pra quem está dentro de um
//! painel, mas sobre uma superfície de outra cor (um card `--card`, uma moldura tingida) o fade tem de
//! ser dito. Sobre um fundo com padrão ou imagem, não há como reproduzir — use `fade(false)`.
//!
//! O gradiente vai da cor até **a mesma cor com alfa 0**, não até "transparente" genérico: interpolar
//! até um transparente de outro matiz produz um halo acinzentado no meio do caminho.
//!
//! # Por que a barra é nossa, e não a do GPUI
//!
//! O `overflow_y_scroll` do GPUI dá o comportamento de rolagem mas **não desenha barra nenhuma** —
//! quem quer barra desenha. É o que permite (e obriga) reproduzir a do coss.
//!
//! O arraste do polegar precisa de eventos de mouse FORA do polegar: quem arrasta sai dos 6px de
//! largura no primeiro movimento. A solução é registrar ouvintes no nível da janela
//! (`Window::on_mouse_event`) durante o paint, o que aqui é feito por um [`gpui::canvas`] de tamanho
//! zero — o mesmo mecanismo que o [`crate::tabs`] usa pra medir as abas, e o mesmo que o
//! `gpui_component::scroll` usa pra arrastar.
//!
//! # O que não foi reproduzido
//!
//! - **`overscrollContain`**: não há controle de overscroll/rubber-band no GPUI.
//! - **`rounded-[inherit]` no viewport**: não existe `inherit`. Quem precisa passa
//!   [`ScrollArea::radius`].
//! - **Anel de foco**: a referência torna o viewport focável (`focus-visible:ring-2` com offset).
//!   Um elemento `RenderOnce` não pode possuir um `FocusHandle` entre frames, então isso é opt-in via
//!   [`ScrollArea::focus`]. Sem handle não há foco, e sem foco não há anel — nada fica visualmente
//!   errado, só indisponível.
//! - **Barra de rolagem horizontal do teclado / paginação por `PageUp`**: fora de escopo.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, linear_color_stop, linear_gradient, point, px, AnyElement, App, DispatchPhase, ElementId, FocusHandle, Hsla, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Refineable,
    RenderOnce, ScrollHandle, StatefulInteractiveElement, StyleRefinement, Styled, Window,
};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta
// =================================================================================================

struct ScrollPalette {
    /// `--foreground`, a base do polegar (que o usa a 20%).
    foreground: Rgba8,
    /// `--background`, o default do fade.
    background: Rgba8,
    /// `--ring`, do anel de foco opcional.
    ring: Rgba8,
}

const SCROLL_LIGHT: ScrollPalette = ScrollPalette {
    foreground: Rgba8(0x262626ff), // neutral-800
    background: Rgba8(0xffffffff),
    ring: Rgba8(0xa3a3a3ff), // neutral-400
};

const SCROLL_DARK: ScrollPalette = ScrollPalette {
    // mix(neutral-950 96%, white) = #141414
    background: Rgba8(0x141414ff),
    foreground: Rgba8(0xf5f5f5ff), // neutral-100
    ring: Rgba8(0x737373ff),       // neutral-500
};

fn palette() -> &'static ScrollPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &SCROLL_DARK,
        theme::ThemeMode::Light => &SCROLL_LIGHT,
    }
}

// =================================================================================================
// Medidas e tempos, todos da referência
// =================================================================================================

/// Tamanho do fade em cada borda — o `--fade-size: 1.5rem`.
const FADE_SIZE: f32 = 24.0;

/// Espessura da barra: `w-1.5` na vertical, `h-1.5` na horizontal.
const BAR_THICKNESS: f32 = 6.0;

/// Folga da barra até as bordas do viewport — o `m-1`.
const BAR_MARGIN: f32 = 4.0;

/// Respiro que o `scrollbar_gutter` reserva — `pe-2.5` / `pb-2.5`.
const GUTTER: f32 = 10.0;

/// Opacidade do polegar sobre `--foreground` — o `/20` do `bg-foreground/20`.
const THUMB_ALPHA: f32 = 0.20;

/// Comprimento mínimo do polegar.
///
/// **Não vem da referência.** O primitivo do base-ui dimensiona o polegar pela proporção
/// viewport/conteúdo, e numa lista muito longa isso dá um polegar de 1px — impossível de agarrar. É
/// uma adição de usabilidade, não um valor portado.
const THUMB_MIN: f32 = 24.0;

/// Aparecer: `data-hovering:duration-100`, sem atraso.
const SHOW_DUR: Duration = Duration::from_millis(100);

/// Desaparecer: `delay-300` e depois a duração default do Tailwind (150ms).
const HIDE_DELAY: Duration = Duration::from_millis(300);
const HIDE_DUR: Duration = Duration::from_millis(150);

/// Anel de foco opcional: `ring-2` com `ring-offset-1`.
const RING_WIDTH: f32 = 2.0;
const RING_OFFSET: f32 = 1.0;

// =================================================================================================
// Eixo
// =================================================================================================

/// Em que direções a área rola.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScrollAxis {
    /// Só vertical — o caso comum.
    #[default]
    Vertical,
    /// Só horizontal.
    Horizontal,
    /// As duas.
    Both,
}

impl ScrollAxis {
    fn vertical(self) -> bool {
        matches!(self, ScrollAxis::Vertical | ScrollAxis::Both)
    }
    fn horizontal(self) -> bool {
        matches!(self, ScrollAxis::Horizontal | ScrollAxis::Both)
    }
}

// =================================================================================================
// Estado por instância
// =================================================================================================

/// Um arraste de polegar em curso.
#[derive(Clone, Copy, Debug)]
struct Drag {
    /// Se é o polegar vertical (senão, o horizontal).
    vertical: bool,
    /// Posição do mouse quando o arraste começou, no eixo que importa.
    from_mouse: f32,
    /// Deslocamento de rolagem quando o arraste começou.
    from_offset: f32,
    /// Quantos pixels de conteúdo cada pixel de trilho vale. Congelado no início do arraste: a
    /// geometria não muda no meio de um arraste, e recalculá-la a cada frame faria o conteúdo
    /// "escapar" do ponteiro se o polegar mudasse de tamanho.
    content_per_track: f32,
}

/// O que cada área de rolagem lembra entre frames.
#[derive(Clone, Copy, Debug)]
struct State {
    /// O ponteiro está sobre a área.
    hovering: bool,
    /// O deslocamento visto no frame anterior — comparar com o atual é como se detecta "rolando".
    last_offset: Point<Pixels>,
    /// Quando a visibilidade-alvo mudou pela última vez, e qual opacidade estava valendo naquele
    /// instante. Guardar a opacidade de partida é o que faz um hover interrompido no meio do fade
    /// continuar de onde estava, em vez de saltar.
    changed_at: Instant,
    from_opacity: f32,
    target: bool,
    drag: Option<Drag>,
}

thread_local! {
    static STATES: RefCell<HashMap<ElementId, State>> = RefCell::new(HashMap::new());
}

/// Teto da [`STATES`], no mesmo número das tabelas de interação do [`crate::button`] e do
/// [`crate::tooltip`].
const TABLE_CAP: usize = 512;

/// Poda a [`STATES`] quando ela passa do [`TABLE_CAP`].
///
/// ⚠️ **Aqui NÃO é o `t.clear()` que o `button` e o `tooltip` usam, e a diferença é deliberada.**
/// Nesta tabela a entrada guarda um [`Drag`] — o arraste de barra EM CURSO. Um `clear()` no meio de um
/// arraste apagaria a âncora dele (`from_mouse`/`from_offset`), e a barra saltaria ou pararia debaixo
/// do dedo. Nas outras duas tabelas o pior caso é um fade recomeçar do zero, que degrada e não quebra;
/// aqui quebraria.
///
/// Então a poda RETÉM o que está sendo arrastado e descarta o resto. Sobra no máximo um punhado de
/// entradas (não há como arrastar duas barras com um ponteiro), o que é o mesmo efeito de limpar sem o
/// efeito colateral. O que se perde é o mesmo que num `clear()`: as áreas descartadas voltam com o
/// relógio do fade zerado, ou seja a barra delas reaparece do zero na próxima vez que aparecer.
///
/// Sem isto a tabela era a ÚNICA ilimitada da lib: 88 bytes por [`gpui::ElementId`] distinto já
/// renderizado (medido), mais a alocação da `SharedString` que a chave `ElementId::Name` retém. Doze
/// mil ids ⇒ ~1 MB que nunca volta.
fn poda(t: &mut HashMap<ElementId, State>) {
    if t.len() > TABLE_CAP {
        t.retain(|_, s| s.drag.is_some());
    }
}

/// A opacidade da barra agora, dada a curva da referência.
///
/// Aparecer é 100ms sem atraso; desaparecer é 300ms de espera e 150ms de fade. As duas pontas partem
/// da opacidade que estava valendo, então interromper o movimento não salta.
fn opacity_now(s: &State) -> f32 {
    let t = s.changed_at.elapsed();
    if s.target {
        let k = (t.as_secs_f32() / SHOW_DUR.as_secs_f32()).clamp(0.0, 1.0);
        s.from_opacity + (1.0 - s.from_opacity) * k
    } else if t < HIDE_DELAY {
        s.from_opacity
    } else {
        let k = ((t - HIDE_DELAY).as_secs_f32() / HIDE_DUR.as_secs_f32()).clamp(0.0, 1.0);
        s.from_opacity * (1.0 - k)
    }
}

/// Se a barra ainda está se movendo — é o que decide pedir outro frame.
fn animating(s: &State) -> bool {
    let o = opacity_now(s);
    if s.target {
        o < 1.0
    } else {
        o > 0.0
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// Área de rolagem com barra que se esconde e fade nas bordas. Ver o doc do módulo.
///
/// O [`ScrollHandle`] é de quem chama: a área não o possui, o que permite rolar por código de fora
/// (`handle.scroll_to_bottom()`) e sobreviver a re-renders sem virar uma view.
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    handle: ScrollHandle,
    axis: ScrollAxis,
    fade: bool,
    fade_size: f32,
    fade_color: Option<Hsla>,
    gutter: bool,
    fill: bool,
    clamp_content_min_width: bool,
    radius: f32,
    focus: Option<FocusHandle>,
    children: Vec<AnyElement>,
    /// Estilo de quem chama, aplicado à raiz — é o `className` que a `Root` da referência aceita.
    /// Serve principalmente pra dimensionar (`flex_1`, `h`, `max_h`): a raiz é `size-full`, e quem
    /// decide o tamanho dela é o pai, como no original.
    style: StyleRefinement,
}

impl ScrollArea {
    /// Uma área nova. O `id` tem que ser estável entre frames — é a chave do estado de hover e
    /// arraste; um id que muda a cada frame faz a barra piscar.
    pub fn new(id: impl Into<ElementId>, handle: &ScrollHandle) -> Self {
        Self {
            id: id.into(),
            handle: handle.clone(),
            axis: ScrollAxis::default(),
            // ⚠️ LIGADO por padrão, ao contrário da referência. Ver o doc do módulo.
            fade: true,
            fade_size: FADE_SIZE,
            fade_color: None,
            gutter: false,
            fill: false,
            clamp_content_min_width: true,
            radius: 0.0,
            focus: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Em que direções rola. Default: vertical.
    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Liga/desliga o fade nas bordas. **Default: ligado** (a referência tem `false`).
    pub fn fade(mut self, fade: bool) -> Self {
        self.fade = fade;
        self
    }

    /// A cor do fade — tem que ser a cor que está ATRÁS do conteúdo, porque isto é um gradiente
    /// sobreposto e não uma máscara. Default: `--background`. Ver o doc do módulo.
    pub fn fade_color(mut self, color: Hsla) -> Self {
        self.fade_color = Some(color);
        self
    }

    /// Tamanho do fade em pixels. Default 24 (`--fade-size: 1.5rem`).
    pub fn fade_size(mut self, size: f32) -> Self {
        self.fade_size = size.max(0.0);
        self
    }

    /// Reserva respiro pro conteúdo não passar sob a barra (`scrollbarGutter`). O respiro só é
    /// aplicado no eixo que de fato transborda, como na referência.
    pub fn scrollbar_gutter(mut self, gutter: bool) -> Self {
        self.gutter = gutter;
        self
    }

    /// O conteúdo ocupa a área toda (`fill`), em vez de ter a altura natural dele.
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// `clampContentMinWidth` — `min-width: 0` no conteúdo. Default `true`, como na referência.
    pub fn clamp_content_min_width(mut self, clamp: bool) -> Self {
        self.clamp_content_min_width = clamp;
        self
    }

    /// O raio do viewport, que na referência é `rounded-[inherit]`. Sem `inherit` no GPUI, quem
    /// conhece o raio do pai é quem chama.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Torna a área focável e liga o anel de `focus-visible`. Ver o doc do módulo.
    pub fn focus(mut self, handle: &FocusHandle) -> Self {
        self.focus = Some(handle.clone());
        self
    }
}

impl Styled for ScrollArea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// Piso pra considerar que ainda há conteúdo escondido de um lado — ver o encadeamento de rolagem em
/// [`ScrollArea`]. Meio pixel: abaixo disso é resíduo de arredondamento do layout, e tratar resíduo
/// como "tem para onde rolar" prenderia o evento no fim do curso.
const SOBRA_MINIMA: f32 = 0.5;

/// Quanto conteúdo está escondido além de cada borda.
///
/// O deslocamento do GPUI é **negativo** conforme se rola pra frente (é uma translação do conteúdo),
/// então o que está escondido no início é `-offset`.
fn overflow(offset: Pixels, max: Pixels) -> (f32, f32) {
    let inicio = (-f32::from(offset)).max(0.0);
    let fim = (f32::from(max) - inicio).max(0.0);
    (inicio, fim)
}

impl RenderOnce for ScrollArea {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = palette();
        let id = self.id.clone();

        // --- Medidas ----------------------------------------------------------------------------
        //
        // Tudo vem do `ScrollHandle`, que só é preenchido DEPOIS do primeiro layout. No primeiro
        // frame, portanto, não há barra nem fade — e é por isso que o fim do render pede um frame a
        // mais quando ainda não há medida. Em repouso isso não se vê: a barra nasce invisível.
        let offset = self.handle.offset();
        let max = self.handle.max_offset();
        let viewport = self.handle.bounds().size;

        let (over_top, over_bottom) = overflow(offset.y, max.height);
        let (over_left, over_right) = overflow(offset.x, max.width);

        let tem_over_y = self.axis.vertical() && f32::from(max.height) > 0.5;
        let tem_over_x = self.axis.horizontal() && f32::from(max.width) > 0.5;

        // --- Visibilidade da barra --------------------------------------------------------------
        //
        // "rolando" é detectado comparando o deslocamento com o do frame anterior. Não precisa de
        // temporizador próprio: os 300ms de atraso pra esconder cobrem com folga o intervalo entre
        // dois eventos de roda (~16ms num trackpad), então a barra não pisca durante a rolagem.
        let (opacidade, arrastando, anima) = STATES.with(|t| {
            let mut t = t.borrow_mut();
            poda(&mut t);
            let s = t.entry(id.clone()).or_insert_with(|| State {
                hovering: false,
                last_offset: offset,
                changed_at: Instant::now(),
                from_opacity: 0.0,
                target: false,
                drag: None,
            });

            let rolou = s.last_offset != offset;
            s.last_offset = offset;

            let alvo = s.hovering || rolou || s.drag.is_some();
            if alvo != s.target {
                // Parte da opacidade que está valendo AGORA, não de 0 ou 1.
                s.from_opacity = opacity_now(s);
                s.target = alvo;
                s.changed_at = Instant::now();
            }
            (opacity_now(s), s.drag.is_some(), animating(s))
        });

        // --- Viewport ---------------------------------------------------------------------------
        let mut conteudo = div().when(self.fill, |d| d.size_full());
        if self.clamp_content_min_width {
            conteudo = conteudo.min_w(px(0.0));
        }
        for c in self.children {
            conteudo = conteudo.child(c);
        }

        let focado = self
            .focus
            .as_ref()
            .is_some_and(|h| h.is_focused(window))
            && crate::focus_ring::visible();

        // ⚠️ **Pra rolar na horizontal, o conteúdo NÃO pode ser esticado pela largura do viewport.**
        //
        // O GPUI calcula o curso de rolagem como `content_size − bounds` (ver
        // `Interactivity::prepaint`). O conteúdo, sendo filho de bloco, esticava à largura do viewport
        // por padrão — então `content_size.width == bounds.width` e o curso horizontal dava **zero**.
        // Era o defeito relatado: a barra aparecia mas nada rolava, porque não havia curso.
        //
        // A saída é o viewport virar uma LINHA de flex e o conteúdo um item `flex_none`: aí a largura
        // dele passa a ser a do próprio conteúdo, que é o que transborda.
        //
        // Por eixo:
        //
        // - **vertical**: nada muda. A altura de um filho de bloco já vem do conteúdo, que é por isso
        //   que este eixo sempre funcionou.
        // - **horizontal**: largura do conteúdo, altura esticada (o `stretch` do flex mantém o
        //   comportamento anterior no eixo que não rola).
        // - **os dois**: `items_start` também, senão a altura voltaria a ser esticada e o curso
        //   vertical zeraria pelo mesmo motivo.
        // No eixo horizontal o conteúdo é **absoluto**, e é isso que faz a rolagem funcionar.
        //
        // Duas coisas tinham que ser verdade ao mesmo tempo, e elas se contradiziam com conteúdo em
        // fluxo:
        //
        // 1. o conteúdo precisa ser MAIS LARGO que o viewport, senão `content_size − bounds` dá zero e
        //    não há curso (era o defeito "a horizontal não rola");
        // 2. essa largura NÃO pode subir pela árvore, senão o viewport cresce atrás dela, a máscara de
        //    recorte cresce junto e o conteúdo aparece por fora do container (era o defeito
        //    "redimensionei e ficou bugado").
        //
        // Filho absoluto atende as duas: o taffy o conta no `content_size` do pai (então há curso) mas
        // não no tamanho intrínseco dele (então nada cresce). E o deslocamento de rolagem alcança ele —
        // o GPUI envolve o prepaint de TODOS os filhos com `with_element_offset`.
        //
        // **As três combinações, medidas na janela** numa coluna de 768 com 2096 de conteúdo:
        //
        // | conteúdo | pisos de `min_w(0)` | viewport | curso |
        // |---|---|---|---|
        // | em fluxo | — | 2096 (estourou o container) | 0 |
        // | absoluto | só no viewport | 0 (colapsou) | — |
        // | absoluto | no viewport **e** na raiz | **736** | **1360** |
        //
        // Ou seja: os dois pisos são carga estrutural junto com o absoluto, não enfeite. Não tenho a
        // explicação em termos do taffy pra o piso da RAIZ importar — `min-width` é piso e o efeito
        // aparente é sobre resolução de largura —, então ficam a medição e o aviso, não uma teoria
        // inventada. Se alguém tirar um dos três, é aqui que está o mapa.
        if self.axis.horizontal() {
            conteudo = conteudo.flex_none().absolute().top_0().left_0();
        }

        let mut vp = div()
            // Ids de filho não precisam do id da instância: o `GlobalElementId` do GPUI é o CAMINHO
            // na árvore, e a raiz logo abaixo já carrega o id da instância. Basta serem únicos entre
            // irmãos — e as duas barras são irmãs, por isso levam sufixo de eixo.
            .id("scroll-vp")
            .size_full()
            // Os pisos em zero: ver a tabela de medições no comentário do conteúdo absoluto, acima. Sem
            // eles (aqui E na raiz) o viewport colapsa pra largura zero.
            .min_w(px(0.0))
            .min_h(px(0.0))
            .when(self.radius > 0.0, |d| d.rounded(px(self.radius)))
            .when(self.axis.horizontal(), |d| d.flex())
            .when(matches!(self.axis, ScrollAxis::Both), |d| d.items_start())
            .when(self.axis.vertical(), |d| d.overflow_y_scroll())
            .when(self.axis.horizontal(), |d| d.overflow_x_scroll())
            .track_scroll(&self.handle)
            // O respiro do gutter só no eixo que transborda — é o `data-has-overflow-*` da
            // referência, não um respiro constante.
            .when(self.gutter && tem_over_y, |d| d.pr(px(GUTTER)))
            .when(self.gutter && tem_over_x, |d| d.pb(px(GUTTER)))
            .child(conteudo);

        // --- Encadeamento de rolagem ------------------------------------------------------------
        //
        // **O problema.** Todo elemento rolável do GPUI registra o próprio ouvinte de roda e NUNCA
        // interrompe a propagação (ver `Interactivity::paint_scroll_listener`). Com uma área dentro de
        // outra, os dois hitboxes contêm o ponteiro, os dois ouvintes rodam, e as duas rolam ao mesmo
        // tempo — foi o defeito relatado: mexer na roda sobre a área interna arrastava a página junto.
        //
        // **O conserto.** Interromper a propagação depois que ESTA área consumiu o evento. Duas coisas
        // do GPUI fazem isso funcionar sem gambiarra:
        //
        // 1. na fase `Bubble` os ouvintes rodam na ordem REVERSA de registro, e um filho é pintado
        //    depois do pai — então o ouvinte do filho roda antes do do pai;
        // 2. no MESMO elemento, `paint_mouse_listeners` (este `on_scroll_wheel`) é registrado ANTES do
        //    `paint_scroll_listener` interno, o que na ordem reversa põe o interno primeiro.
        //
        // Ou seja: o GPUI rola esta área, depois nós cortamos o evento antes que ele chegue no pai.
        //
        // **E o encadeamento propriamente.** Só cortamos se esta área REALMENTE tinha para onde ir
        // naquele sentido. No fim do curso o evento segue para o pai, que é o que o navegador e o
        // macOS fazem — sem isso, um ponteiro parado sobre a área interna prenderia a página inteira.
        let eixo = self.axis;
        let handle = self.handle.clone();
        vp = vp.on_scroll_wheel(move |ev, window, cx| {
            let d = ev.delta.pixel_delta(window.line_height());
            // ⚠️ As medidas vêm do handle AO VIVO, não de um instantâneo do render.
            //
            // Tentei com instantâneo primeiro e não funciona: o ouvinte nasce no render, e no PRIMEIRO
            // render o `ScrollHandle` ainda não foi medido (`max_offset` = 0). O ouvinte daquele frame
            // acharia que não há para onde rolar e deixaria o evento passar — medido, era exatamente
            // isso que acontecia.
            //
            // E o offset lido aqui é o de DEPOIS: o `paint_scroll_listener` do GPUI já somou o delta
            // quando chegamos (conferido — com `dy = -90` o offset lia -90 partindo de zero). Então o
            // estado de ANTES do evento é `offset - delta`, e é sobre ele que a pergunta "esta área
            // tinha para onde ir?" se responde.
            let max = handle.max_offset();
            let offset = handle.offset();
            let pode = |offset: f32, delta: f32, max: f32| {
                let antes = offset - delta;
                let (inicio, fim) = overflow(px(antes), px(max));
                (delta < 0.0 && fim > SOBRA_MINIMA) || (delta > 0.0 && inicio > SOBRA_MINIMA)
            };
            // ⚠️ **O delta que o GPUI aplicou não é necessariamente o do evento.** Quando o elemento
            // rola em UM eixo só e o delta daquele eixo é zero, ele usa o do outro — é assim que uma
            // roda vertical (ou dois dedos pra baixo) rola uma faixa horizontal. A regra está no
            // `paint_scroll_listener`, e o `restrict_scroll_to_axis` do `Style` é `false` por padrão.
            //
            // Espelhar isso aqui não é preciosismo: sem espelhar, a decisão olhava um delta zero, não
            // cortava, e a faixa horizontal rolava junto com a página — medido.
            let (dx, dy) = (f32::from(d.x), f32::from(d.y));
            let so_x = eixo.horizontal() && !eixo.vertical();
            let so_y = eixo.vertical() && !eixo.horizontal();
            let efetivo = |proprio: f32, outro: f32, so_este_eixo: bool| {
                if proprio != 0.0 {
                    proprio
                } else if so_este_eixo {
                    outro
                } else {
                    0.0
                }
            };
            let y = eixo.vertical()
                && pode(
                    f32::from(offset.y),
                    efetivo(dy, dx, so_y),
                    f32::from(max.height),
                );
            let x = eixo.horizontal()
                && pode(
                    f32::from(offset.x),
                    efetivo(dx, dy, so_x),
                    f32::from(max.width),
                );
            if y || x {
                cx.stop_propagation();
            }
        });

        if let Some(h) = self.focus.as_ref() {
            vp = vp.track_focus(h);
        }

        // --- A raiz -----------------------------------------------------------------------------
        let id_hover = self.id.clone();
        let mut raiz = div()
            .id(self.id.clone())
            .relative()
            .size_full()
            // Idem: os dois pisos, e os dois importam — ver a tabela no comentário do conteúdo
            // absoluto. O `min_h` já estava aqui de antes.
            .min_w(px(0.0))
            .min_h(px(0.0))
            .on_hover(move |hovered, window, _cx| {
                let mudou = STATES.with(|t| {
                    let mut t = t.borrow_mut();
                    match t.get_mut(&id_hover) {
                        Some(s) if s.hovering != *hovered => {
                            s.hovering = *hovered;
                            true
                        }
                        _ => false,
                    }
                });
                // Só pede frame quando o estado MUDA: `on_hover` dispara em rajada, e um refresh por
                // evento sujaria a janela inteira sem motivo.
                if mudou {
                    window.refresh();
                }
            })
            .child(vp);
        // Por ÚLTIMO, pra vencer o que a raiz define — é a ordem do `cn()` na referência.
        raiz.style().refine(&self.style);

        // --- Fade -------------------------------------------------------------------------------
        //
        // Antes das barras: se a ordem se invertesse, o gradiente cobriria o polegar nas pontas.
        if self.fade {
            let cor = self.fade_color.unwrap_or_else(|| p.background.hsla());
            let f = self.fade_size;
            if self.axis.vertical() {
                if over_top > 0.0 {
                    raiz = raiz.child(fade_edge(cor, Edge::Top, f.min(over_top)));
                }
                if over_bottom > 0.0 {
                    raiz = raiz.child(fade_edge(cor, Edge::Bottom, f.min(over_bottom)));
                }
            }
            if self.axis.horizontal() {
                if over_left > 0.0 {
                    raiz = raiz.child(fade_edge(cor, Edge::Left, f.min(over_left)));
                }
                if over_right > 0.0 {
                    raiz = raiz.child(fade_edge(cor, Edge::Right, f.min(over_right)));
                }
            }
        }

        // --- Barras -----------------------------------------------------------------------------
        if opacidade > 0.001 {
            if tem_over_y {
                raiz = raiz.child(scrollbar(
                    &self.id,
                    &self.handle,
                    true,
                    f32::from(viewport.height),
                    f32::from(max.height),
                    over_top,
                    opacidade,
                    p,
                ));
            }
            if tem_over_x {
                raiz = raiz.child(scrollbar(
                    &self.id,
                    &self.handle,
                    false,
                    f32::from(viewport.width),
                    f32::from(max.width),
                    over_left,
                    opacidade,
                    p,
                ));
            }
        }

        // --- Anel de foco -----------------------------------------------------------------------
        if focado {
            // `ring-offset-1`: o anel fica 1px PARA FORA, com uma coroa da cor do fundo entre ele e
            // o elemento. Reproduzido por dois overlays concêntricos, e não por sombra — o
            // `paint_shadows` do GPUI não recorta a sombra pra fora do elemento, e usá-la aqui
            // lavaria o conteúdo de cor. Já custou três defeitos visíveis nesta base.
            let cor_fundo = self.fade_color.unwrap_or_else(|| p.background.hsla());
            raiz = raiz
                .child(
                    div()
                        .absolute()
                        .inset(px(-RING_OFFSET))
                        .rounded(px(self.radius + RING_OFFSET))
                        .border(px(RING_OFFSET))
                        .border_color(cor_fundo),
                )
                .child(
                    div()
                        .absolute()
                        .inset(px(-RING_OFFSET - RING_WIDTH))
                        .rounded(px(self.radius + RING_OFFSET + RING_WIDTH))
                        .border(px(RING_WIDTH))
                        .border_color(p.ring.hsla()),
                );
        }

        // --- Arraste ----------------------------------------------------------------------------
        //
        // Um canvas de tamanho zero que, no paint, registra os ouvintes de mouse NA JANELA. Só
        // enquanto há arraste: quem arrasta sai dos 6px do polegar no primeiro movimento, e um
        // ouvinte de janela ligado permanentemente custaria um par de closures por frame e por área.
        if arrastando {
            let id_drag = self.id.clone();
            let handle = self.handle.clone();
            raiz = raiz.child(
                canvas(
                    |_, _, _| (),
                    move |_bounds, _, window, _cx| {
                        let id_move = id_drag.clone();
                        let h = handle.clone();
                        window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, _cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            let Some(d) = STATES
                                .with(|t| t.borrow().get(&id_move).and_then(|s| s.drag))
                            else {
                                return;
                            };
                            let mouse = if d.vertical {
                                f32::from(e.position.y)
                            } else {
                                f32::from(e.position.x)
                            };
                            let novo = d.from_offset - (mouse - d.from_mouse) * d.content_per_track;
                            let atual = h.offset();
                            h.set_offset(if d.vertical {
                                point(atual.x, px(novo))
                            } else {
                                point(px(novo), atual.y)
                            });
                            window.refresh();
                        });

                        let id_up = id_drag.clone();
                        window.on_mouse_event(move |_e: &MouseUpEvent, phase, window, _cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            let tinha = STATES.with(|t| {
                                t.borrow_mut()
                                    .get_mut(&id_up)
                                    .map(|s| s.drag.take().is_some())
                                    .unwrap_or(false)
                            });
                            if tinha {
                                window.refresh();
                            }
                        });
                    },
                )
                .absolute()
                .size(px(0.0)),
            );
        }

        // Enquanto a barra se move — ou enquanto não há medida — pede o frame seguinte. Sem isto a
        // animação congela no primeiro frame: o GPUI só redesenha quando alguém pede.
        if anima || f32::from(viewport.height) < 0.5 {
            window.request_animation_frame();
        }

        raiz
    }
}

// =================================================================================================
// Fade
// =================================================================================================

#[derive(Clone, Copy)]
enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Um retângulo de gradiente numa borda: cor cheia na borda, alfa 0 a `size` de distância.
///
/// O `div` não tem ouvintes, então não cria hitbox e não intercepta a rolagem nem o clique.
fn fade_edge(cor: Hsla, edge: Edge, size: f32) -> gpui::Div {
    let mut transparente = cor;
    transparente.a = 0.0;

    // Ângulo no sentido CSS: 0° aponta pra cima. O gradiente vai da BORDA pra dentro.
    let angulo = match edge {
        Edge::Top => 180.0,
        Edge::Bottom => 0.0,
        Edge::Left => 90.0,
        Edge::Right => 270.0,
    };
    let g = linear_gradient(
        angulo,
        linear_color_stop(cor, 0.0),
        linear_color_stop(transparente, 1.0),
    );

    let base = div().absolute().bg(g);
    match edge {
        Edge::Top => base.top_0().left_0().right_0().h(px(size)),
        Edge::Bottom => base.bottom_0().left_0().right_0().h(px(size)),
        Edge::Left => base.left_0().top_0().bottom_0().w(px(size)),
        Edge::Right => base.right_0().top_0().bottom_0().w(px(size)),
    }
}

// =================================================================================================
// Barra
// =================================================================================================

/// Uma barra: trilho invisível de 6px encostado na borda com `m-1`, e o polegar dentro dele.
///
/// `viewport` é o tamanho visível no eixo, `max` o deslocamento máximo (ou seja
/// `conteúdo − viewport`) e `escondido` quanto já passou pra trás da borda de início.
#[allow(clippy::too_many_arguments)]
fn scrollbar(
    id: &ElementId,
    handle: &ScrollHandle,
    vertical: bool,
    viewport: f32,
    max: f32,
    escondido: f32,
    opacidade: f32,
    p: &'static ScrollPalette,
) -> gpui::Stateful<gpui::Div> {
    let trilho = (viewport - 2.0 * BAR_MARGIN).max(0.0);
    let conteudo = viewport + max;

    // Proporção viewport/conteúdo, com piso pra o polegar continuar agarrável (ver `THUMB_MIN`).
    let polegar = if conteudo > 0.0 {
        (trilho * viewport / conteudo).clamp(THUMB_MIN.min(trilho), trilho)
    } else {
        trilho
    };
    // O polegar percorre o trilho MENOS o próprio comprimento, e a fração percorrida é
    // `escondido / max` — não `escondido / conteúdo`, que nunca chegaria ao fim.
    let curso = (trilho - polegar).max(0.0);
    let pos = if max > 0.0 {
        curso * (escondido / max).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let mut cor = p.foreground.hsla();
    cor.a = THUMB_ALPHA * opacidade;

    let id_drag = id.clone();
    let h_drag = handle.clone();
    // Um pixel de trilho vale este tanto de conteúdo. É o fator que transforma o movimento do mouse
    // em rolagem, e ele é congelado no início do arraste (ver `Drag::content_per_track`).
    let content_per_track = if curso > 0.0 { max / curso } else { 0.0 };

    let mut thumb = div()
        .id(if vertical { "scroll-thumb-y" } else { "scroll-thumb-x" })
        .absolute()
        .rounded_full()
        .bg(cor)
        .on_mouse_down(
            MouseButton::Left,
            move |e: &MouseDownEvent, window, cx| {
                // `stop_propagation` pra o mouse-down não chegar ao conteúdo (que poderia iniciar uma
                // seleção de texto enquanto se arrasta a barra).
                cx.stop_propagation();
                let atual = h_drag.offset();
                STATES.with(|t| {
                    if let Some(s) = t.borrow_mut().get_mut(&id_drag) {
                        s.drag = Some(Drag {
                            vertical,
                            from_mouse: if vertical {
                                f32::from(e.position.y)
                            } else {
                                f32::from(e.position.x)
                            },
                            from_offset: f32::from(if vertical { atual.y } else { atual.x }),
                            content_per_track,
                        });
                    }
                });
                window.refresh();
            },
        );

    thumb = if vertical {
        thumb.top(px(pos)).h(px(polegar)).left_0().right_0()
    } else {
        thumb.left(px(pos)).w(px(polegar)).top_0().bottom_0()
    };

    let base = div()
        .id(if vertical { "scroll-bar-y" } else { "scroll-bar-x" })
        .absolute();
    if vertical {
        base.top(px(BAR_MARGIN))
            .bottom(px(BAR_MARGIN))
            .right(px(BAR_MARGIN))
            .w(px(BAR_THICKNESS))
            .child(thumb)
    } else {
        base.left(px(BAR_MARGIN))
            .right(px(BAR_MARGIN))
            .bottom(px(BAR_MARGIN))
            .h(px(BAR_THICKNESS))
            .child(thumb)
    }
}

/// Testes com janela de verdade — os únicos que provam o que aritmética nenhuma prova.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{
        Context, Modifiers, Render, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase,
        VisualTestContext,
    };

    /// Uma view mínima que hospeda a área — a `ScrollArea` é um elemento, então precisa de uma view
    /// pra viver dentro.
    struct Harness {
        handle: ScrollHandle,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut conteudo = div().flex().flex_col();
            // 40 linhas de 30px = 1200px de conteúdo numa janela de teste bem menor: há o que rolar.
            for i in 0..40 {
                conteudo = conteudo.child(div().h(px(30.0)).child(format!("linha {i}")));
            }
            div()
                .size_full()
                .child(ScrollArea::new("sa", &self.handle).child(conteudo))
        }
    }

    fn abrir(cx: &mut TestAppContext) -> (ScrollHandle, VisualTestContext) {
        let handle = ScrollHandle::new();
        let h = handle.clone();
        let window = cx.add_window(move |_w, _cx| Harness { handle: h });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        (handle, vcx)
    }

    /// Uma view que hospeda MUITAS áreas de ids distintos — é o cenário do vazamento.
    struct Multidao {
        handle: ScrollHandle,
        quantas: usize,
    }

    impl Render for Multidao {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut raiz = div().size_full().flex().flex_col();
            for i in 0..self.quantas {
                raiz = raiz.child(
                    ScrollArea::new(gpui::SharedString::from(format!("muitas-{i}")), &self.handle)
                        .child(div().h(px(200.0)).child("conteúdo")),
                );
            }
            raiz
        }
    }

    /// **A poda está LIGADA no render**, e não só escrita.
    ///
    /// ⚠️ Este teste existe porque os testes puros de [`poda`] a chamam DIRETO — eles provam a regra e
    /// não provam a ligação. Tirar a chamada do corpo do elemento não faria nenhum deles cair, e
    /// "função escrita e nunca chamada" é o defeito mais recorrente desta base. Aqui a tabela é
    /// observada de fora, depois de um render de verdade com mais ids que o [`TABLE_CAP`].
    #[gpui::test]
    fn a_poda_roda_no_render_e_a_tabela_para_de_crescer(cx: &mut TestAppContext) {
        STATES.with(|t| t.borrow_mut().clear());

        let handle = ScrollHandle::new();
        let h = handle.clone();
        let window = cx.add_window(move |_w, _cx| Multidao {
            handle: h,
            quantas: TABLE_CAP + 40,
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let quantas = STATES.with(|t| t.borrow().len());
        assert!(
            quantas <= TABLE_CAP + 1,
            "a tabela ficou com {quantas} entradas depois de renderizar {} áreas — a poda não está \
             ligada no render",
            TABLE_CAP + 40
        );
    }

    /// Harness de área DENTRO de área, pro encadeamento de rolagem.
    ///
    /// A interna é uma caixa de 120px com 900 de conteúdo, no topo da externa — assim um evento de roda
    /// sobre ela cai nos dois hitboxes, que é a situação do defeito.
    struct Aninhado {
        externa: ScrollHandle,
        interna: ScrollHandle,
    }

    impl Render for Aninhado {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let linhas = |n: usize| {
                let mut c = div().flex().flex_col();
                for i in 0..n {
                    c = c.child(div().h(px(30.0)).child(format!("linha {i}")));
                }
                c
            };
            div().size_full().child(
                ScrollArea::new("externa", &self.externa).child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .h(px(120.0))
                                .child(ScrollArea::new("interna", &self.interna).child(linhas(30))),
                        )
                        .child(linhas(40)),
                ),
            )
        }
    }

    fn abrir_aninhado(cx: &mut TestAppContext) -> (ScrollHandle, ScrollHandle, VisualTestContext) {
        let (externa, interna) = (ScrollHandle::new(), ScrollHandle::new());
        let (e, i) = (externa.clone(), interna.clone());
        let window = cx.add_window(move |_w, _cx| Aninhado {
            externa: e,
            interna: i,
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        (externa, interna, vcx)
    }

    fn roda(vcx: &mut VisualTestContext, x: f32, y: f32, dy: f32) {
        vcx.simulate_event(ScrollWheelEvent {
            position: point(px(x), px(y)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(dy))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        vcx.run_until_parked();
    }

    /// **Rolar a área interna NÃO arrasta a externa.** Era o defeito relatado: as duas andavam juntas,
    /// porque o ouvinte de roda do GPUI não interrompe a propagação e os dois hitboxes contêm o
    /// ponteiro.
    #[gpui::test]
    fn rolar_a_area_interna_nao_arrasta_a_externa(cx: &mut TestAppContext) {
        let (externa, interna, mut vcx) = abrir_aninhado(cx);
        assert!(
            f32::from(interna.max_offset().height) > 0.0 && f32::from(externa.max_offset().height) > 0.0,
            "as duas têm que transbordar, senão o teste não testa nada"
        );

        // Ponteiro sobre a área INTERNA (ela ocupa os 120px de cima).
        roda(&mut vcx, 50.0, 60.0, -90.0);

        assert!(
            f32::from(interna.offset().y) < -1.0,
            "a interna tinha que rolar (deu {})",
            f32::from(interna.offset().y)
        );
        assert_eq!(
            f32::from(externa.offset().y),
            0.0,
            "e a externa tinha que ficar PARADA — é o defeito relatado"
        );
    }

    /// **No fim do curso da interna, a rolagem passa pra externa.** É o encadeamento: sem ele, um
    /// ponteiro parado sobre a área interna prenderia a página inteira.
    #[gpui::test]
    fn no_fim_da_interna_a_rolagem_encadeia_pra_externa(cx: &mut TestAppContext) {
        let (externa, interna, mut vcx) = abrir_aninhado(cx);

        // Leva a interna ao fim do curso POR CÓDIGO, e não com uma rajada de eventos: o excedente de
        // uma rajada encadearia pra externa (é o que esta feature faz), e aí o teste não saberia dizer
        // se a externa andou pelo encadeamento ou por vazamento.
        let curso = f32::from(interna.max_offset().height);
        interna.set_offset(point(px(0.0), px(-curso)));
        vcx.run_until_parked();
        let fim_interna = f32::from(interna.offset().y);
        assert!(
            (fim_interna + curso).abs() < 1.0,
            "a interna tinha que estar no fim do curso (offset {fim_interna}, curso {curso})"
        );
        assert_eq!(
            f32::from(externa.offset().y),
            0.0,
            "e a externa continua parada antes do evento"
        );

        // Agora o evento tem que vazar pra externa.
        roda(&mut vcx, 50.0, 60.0, -90.0);
        assert!(
            f32::from(externa.offset().y) < -1.0,
            "no fim da interna a externa tinha que assumir (deu {})",
            f32::from(externa.offset().y)
        );
        assert!(
            (f32::from(interna.offset().y) - fim_interna).abs() < 1.0,
            "e a interna não pode andar mais"
        );
    }

    /// Harness de área HORIZONTAL dentro da vertical — é o caso da página do storybook.
    struct Horizontal {
        pagina: ScrollHandle,
        faixa: ScrollHandle,
    }

    impl Render for Horizontal {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut faixa = div().flex();
            for i in 0..30 {
                faixa = faixa.child(div().w(px(80.0)).flex_none().child(format!("c{i}")));
            }
            let mut abaixo = div().flex().flex_col();
            for i in 0..40 {
                abaixo = abaixo.child(div().h(px(30.0)).child(format!("linha {i}")));
            }
            div().size_full().child(
                ScrollArea::new("pagina", &self.pagina).child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div().h(px(60.0)).child(
                                ScrollArea::new("faixa", &self.faixa)
                                    .axis(ScrollAxis::Horizontal)
                                    .child(faixa),
                            ),
                        )
                        .child(abaixo),
                ),
            )
        }
    }

    /// **A faixa horizontal tem curso de rolagem, e o viewport não estoura a caixa.**
    ///
    /// ⚠️ **Este teste NÃO reproduz o defeito de redimensionamento** (o conteúdo escapando pra fora do
    /// container). Tentei três estruturas — caixa de largura fixa, caixa esticada, e um espelho da
    /// cadeia do storybook com `max_w` e respiro — e em nenhuma o viewport cresce. Aquele defeito só
    /// aparece na árvore inteira do app, e foi lá que ele foi medido e conferido (ver o comentário do
    /// conteúdo absoluto em `render`).
    ///
    /// O que ele pega de verdade é a **existência do curso**: com o conteúdo em fluxo,
    /// `content_size − bounds` dá zero e a asserção do `max_offset` cai. É o defeito "a horizontal não
    /// rola", e esse é reproduzível aqui.
    #[gpui::test]
    fn o_viewport_nao_cresce_com_o_conteudo(cx: &mut TestAppContext) {
        const CAIXA: f32 = 300.0;

        struct Caixa {
            faixa: ScrollHandle,
        }
        impl Render for Caixa {
            fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                let mut linha = div().flex();
                // 30 × 80 = 2400 de conteúdo numa caixa de 300: oito vezes mais largo.
                for i in 0..30 {
                    linha = linha.child(div().w(px(80.0)).flex_none().child(format!("c{i}")));
                }
                // Espelha a cadeia do storybook: coluna com TETO de largura (`max_w`, não `w`), card
                // com respiro, e dentro dele a caixa da faixa sem largura declarada. É essa combinação
                // que deixa a largura indefinida o bastante pro conteúdo mandar.
                div()
                    .flex()
                    .flex_col()
                    .max_w(px(CAIXA))
                    .child(
                        div().flex().flex_col().p(px(14.0)).child(
                            div().h(px(60.0)).child(
                                ScrollArea::new("faixa", &self.faixa)
                                    .axis(ScrollAxis::Horizontal)
                                    .child(linha),
                            ),
                        ),
                    )
            }
        }

        let faixa = ScrollHandle::new();
        let f = faixa.clone();
        let window = cx.add_window(move |_w, _cx| Caixa { faixa: f });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let largura = f32::from(faixa.bounds().size.width);
        assert!(
            largura <= CAIXA + 0.5,
            "o viewport mediu {largura} numa caixa de {CAIXA} — ele cresceu com o conteúdo, e a \
             máscara de recorte cresce junto"
        );
        assert!(
            f32::from(faixa.max_offset().width) > 0.0,
            "e ainda tem que haver curso pra rolar"
        );
    }

    /// **A roda vertical rola uma faixa horizontal — e não arrasta a página.**
    ///
    /// O GPUI mapeia delta vertical no eixo X quando o elemento só rola em X (o
    /// `restrict_scroll_to_axis` é `false` por padrão), então é assim que se rola uma faixa horizontal
    /// com roda de mouse ou com dois dedos pra baixo. O nosso corte de propagação tem que entender esse
    /// mapeamento — senão a faixa anda e a página anda junto, que é o defeito de rolagem dupla noutro
    /// eixo.
    #[gpui::test]
    fn a_roda_vertical_rola_a_faixa_horizontal_sem_arrastar_a_pagina(cx: &mut TestAppContext) {
        let (pagina, faixa) = (ScrollHandle::new(), ScrollHandle::new());
        let (pg, fx) = (pagina.clone(), faixa.clone());
        let window = cx.add_window(move |_w, _cx| Horizontal {
            pagina: pg,
            faixa: fx,
        });
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        assert!(
            f32::from(faixa.max_offset().width) > 0.0,
            "a faixa tem que transbordar na horizontal"
        );

        // Ponteiro sobre a faixa (os 60px de cima), roda VERTICAL.
        roda(&mut vcx, 50.0, 30.0, -90.0);

        assert!(
            f32::from(faixa.offset().x) < -1.0,
            "a faixa tinha que andar na horizontal (deu {})",
            f32::from(faixa.offset().x)
        );
        assert_eq!(
            f32::from(pagina.offset().y),
            0.0,
            "e a página tinha que ficar parada"
        );
    }

    /// **A roda do mouse continua rolando.**
    ///
    /// Este teste existe por um risco concreto do desenho: a raiz da área tem um ouvinte de hover, o
    /// que cria hitbox. Se um hitbox de ancestral engolisse o evento de roda, a área ficaria imóvel —
    /// e como a barra é invisível em repouso, o defeito passaria despercebido numa captura de tela.
    #[gpui::test]
    fn a_roda_do_mouse_rola(cx: &mut TestAppContext) {
        let (handle, mut vcx) = abrir(cx);

        assert_eq!(f32::from(handle.offset().y), 0.0, "começa no topo");
        assert!(
            f32::from(handle.max_offset().height) > 0.0,
            "o conteúdo tem que transbordar, senão o teste não testa nada"
        );

        vcx.simulate_event(ScrollWheelEvent {
            position: point(px(50.0), px(50.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-120.0))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        vcx.run_until_parked();

        // Rolar pra baixo deixa o deslocamento NEGATIVO (é uma translação do conteúdo).
        assert!(
            f32::from(handle.offset().y) < -1.0,
            "a roda tem que mover o conteúdo (deu {})",
            f32::from(handle.offset().y)
        );
    }

    /// **Rolar até o fim zera o transbordo do fim.** É o que faz o fade de baixo desaparecer ao
    /// encostar, em vez de ficar pendurado.
    #[gpui::test]
    fn no_fim_o_fade_de_baixo_desaparece(cx: &mut TestAppContext) {
        let (handle, mut vcx) = abrir(cx);

        // Uma rolagem enorme, que o GPUI apara no máximo.
        vcx.simulate_event(ScrollWheelEvent {
            position: point(px(50.0), px(50.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-100_000.0))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        vcx.run_until_parked();

        let (inicio, fim) = overflow(handle.offset().y, handle.max_offset().height);
        assert!(inicio > 0.0, "no fim há muito escondido acima");
        assert!(
            fim < 1.0,
            "e nada mais por rolar, logo nenhum fade embaixo (deu {fim})"
        );
    }
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Fabrica uma entrada de estado, com ou sem arraste em curso.
    fn estado(arrastando: bool) -> State {
        State {
            hovering: false,
            last_offset: Point::default(),
            changed_at: Instant::now(),
            from_opacity: 0.0,
            target: false,
            drag: arrastando.then_some(Drag {
                vertical: true,
                from_mouse: 10.0,
                from_offset: 20.0,
                content_per_track: 2.0,
            }),
        }
    }

    /// Enche a tabela com `n` entradas inertes, com ids distintos.
    fn enche(t: &mut HashMap<ElementId, State>, n: usize) {
        for i in 0..n {
            t.insert(ElementId::Name(format!("area-{i}").into()), estado(false));
        }
    }

    /// **Abaixo do teto a poda não mexe em nada.** Sem isto, uma poda que rodasse sempre jogaria fora o
    /// relógio do fade de toda área a cada frame.
    #[test]
    fn abaixo_do_teto_a_poda_nao_descarta_nada() {
        let mut t = HashMap::new();
        enche(&mut t, 512);
        poda(&mut t);
        assert_eq!(t.len(), 512, "512 é o teto, e o teto ainda não foi PASSADO");
    }

    /// **Passando do teto, a tabela para de crescer.** É o defeito que existia: esta era a única tabela
    /// da lib sem teto nenhum.
    #[test]
    fn passando_do_teto_a_tabela_e_podada() {
        let mut t = HashMap::new();
        enche(&mut t, 513);
        poda(&mut t);
        assert_eq!(t.len(), 0, "nada estava sendo arrastado, então nada precisava sobreviver");
    }

    /// **O que está sendo ARRASTADO sobrevive à poda** — é a diferença deliberada em relação ao
    /// `t.clear()` do `button` e do `tooltip`, e o motivo de a poda existir como função própria.
    ///
    /// ⚠️ Trocar o `retain` por `clear` faz este teste cair, e é isso que ele guarda: com `clear`, a
    /// âncora do arraste (`from_mouse`/`from_offset`) desaparece no meio do gesto e a barra salta.
    #[test]
    fn a_poda_preserva_a_barra_que_esta_sendo_arrastada() {
        let mut t = HashMap::new();
        let arrastando = ElementId::Name("a-que-eu-seguro".into());
        t.insert(arrastando.clone(), estado(true));
        enche(&mut t, 512);
        assert!(t.len() > TABLE_CAP, "o cenário só vale se a poda for disparar");

        poda(&mut t);

        assert_eq!(t.len(), 1, "sobra exatamente a que tem arraste em curso");
        assert!(
            t.get(&arrastando).is_some_and(|s| s.drag.is_some()),
            "a entrada do arraste sobreviveu com a âncora intacta"
        );
    }

    /// O teto é o mesmo número das outras tabelas de interação da casa. Literal de propósito: comparar
    /// contra a própria constante não protegeria nada.
    #[test]
    fn o_teto_e_o_mesmo_das_outras_tabelas_da_casa() {
        assert_eq!(TABLE_CAP, 512);
    }

    /// **O fade vem LIGADO, ao contrário da referência.** Foi pedido explicitamente; a referência tem
    /// `scrollFade = false`. Se alguém "corrigir" isto pra bater com o coss, este teste cai e o
    /// comentário explica por quê.
    #[test]
    fn fade_vem_ligado_por_padrao() {
        let handle = ScrollHandle::new();
        let a = ScrollArea::new("t", &handle);
        assert!(a.fade, "o fade é o único default divergente da referência");
        assert_eq!(a.fade_size, FADE_SIZE);
        assert_eq!(a.fade_size, 24.0, "--fade-size: 1.5rem");
    }

    /// Os outros defaults seguem a referência: sem gutter, sem fill, e `clampContentMinWidth` ligado.
    #[test]
    fn os_outros_defaults_seguem_a_referencia() {
        let handle = ScrollHandle::new();
        let a = ScrollArea::new("t", &handle);
        assert!(!a.gutter, "scrollbarGutter = false");
        assert!(!a.fill, "fill = false");
        assert!(a.clamp_content_min_width, "clampContentMinWidth = true");
        assert_eq!(a.axis, ScrollAxis::Vertical);
    }

    /// As medidas da referência.
    #[test]
    fn medidas_da_referencia() {
        assert_eq!(BAR_THICKNESS, 6.0, "w-1.5 / h-1.5");
        assert_eq!(BAR_MARGIN, 4.0, "m-1");
        assert_eq!(GUTTER, 10.0, "pe-2.5 / pb-2.5");
        assert_eq!(THUMB_ALPHA, 0.20, "bg-foreground/20");
        assert_eq!(SHOW_DUR, Duration::from_millis(100), "duration-100");
        assert_eq!(HIDE_DELAY, Duration::from_millis(300), "delay-300");
        assert_eq!(RING_WIDTH, 2.0, "ring-2");
        assert_eq!(RING_OFFSET, 1.0, "ring-offset-1");
    }

    /// **O deslocamento do GPUI é negativo.** Rolar pra baixo dá `offset.y` negativo, e o que está
    /// escondido acima é o módulo disso. Trocar o sinal aqui inverteria o fade e a barra sem erro de
    /// compilação — é o tipo de bug que só aparece na tela.
    #[test]
    fn o_deslocamento_negativo_vira_transbordo_positivo() {
        // Conteúdo 300 num viewport 100 → max 200. Rolado 60 pra baixo.
        let (inicio, fim) = overflow(px(-60.0), px(200.0));
        assert_eq!(inicio, 60.0, "escondido acima");
        assert_eq!(fim, 140.0, "ainda por rolar");

        // No topo: nada escondido acima, tudo por rolar.
        let (inicio, fim) = overflow(px(0.0), px(200.0));
        assert_eq!(inicio, 0.0);
        assert_eq!(fim, 200.0);

        // No fim: o inverso.
        let (inicio, fim) = overflow(px(-200.0), px(200.0));
        assert_eq!(inicio, 200.0);
        assert_eq!(fim, 0.0);

        // Sem transbordo nenhum: as duas pontas zeradas, e portanto nenhum fade.
        let (inicio, fim) = overflow(px(0.0), px(0.0));
        assert_eq!(inicio, 0.0);
        assert_eq!(fim, 0.0);
    }

    /// **O fade CRESCE com a rolagem** — é o `min(--fade-size, overflow)` da referência, e é o que
    /// diferencia este fade de um degradê fixo: nos primeiros 24px ele abre gradualmente, e some por
    /// completo ao encostar na borda.
    #[test]
    fn o_fade_cresce_ate_o_tamanho_cheio() {
        let f = FADE_SIZE;
        assert_eq!(f.min(0.0), 0.0, "encostado no topo: sem fade");
        assert_eq!(f.min(6.0), 6.0, "6px rolados: fade de 6px");
        assert_eq!(f.min(24.0), 24.0);
        assert_eq!(f.min(9999.0), 24.0, "nunca passa de --fade-size");
    }

    /// A curva de opacidade: aparece em 100ms sem atraso, e ao esconder espera 300ms ANTES de começar
    /// a apagar. O atraso é o que impede a barra de piscar entre dois eventos de roda.
    #[test]
    fn a_curva_de_opacidade_respeita_o_atraso() {
        let agora = Instant::now();
        let aparecendo = State {
            hovering: true,
            last_offset: Point::default(),
            changed_at: agora,
            from_opacity: 0.0,
            target: true,
            drag: None,
        };
        // No instante 0 ainda está em 0 e precisa de mais frames.
        assert!(opacity_now(&aparecendo) < 0.05);
        assert!(animating(&aparecendo));

        // Um instante já passado o suficiente: cheio.
        let pronto = State {
            changed_at: agora - Duration::from_millis(200),
            ..aparecendo
        };
        assert_eq!(opacity_now(&pronto), 1.0);
        assert!(!animating(&pronto), "cheio e parado: não pede mais frame");

        // Escondendo: dentro do atraso, segura o valor de partida.
        let escondendo = State {
            hovering: false,
            target: false,
            from_opacity: 1.0,
            changed_at: agora - Duration::from_millis(100),
            ..aparecendo
        };
        assert_eq!(opacity_now(&escondendo), 1.0, "300ms de espera antes de apagar");

        // Passado o atraso mais a duração: apagada.
        let apagada = State {
            changed_at: agora - HIDE_DELAY - HIDE_DUR - Duration::from_millis(10),
            ..escondendo
        };
        assert_eq!(opacity_now(&apagada), 0.0);
        assert!(!animating(&apagada));
    }

    /// **Interromper o fade parte de onde ele estava**, não de 0 nem de 1. Sem isto, sair e voltar
    /// com o mouse durante o fade faria a barra saltar.
    #[test]
    fn interromper_o_fade_nao_salta() {
        let meio = State {
            hovering: true,
            last_offset: Point::default(),
            changed_at: Instant::now(),
            from_opacity: 0.4,
            target: true,
            drag: None,
        };
        let o = opacity_now(&meio);
        assert!((0.4..0.5).contains(&o), "continua de 0,4 (deu {o})");
    }
}

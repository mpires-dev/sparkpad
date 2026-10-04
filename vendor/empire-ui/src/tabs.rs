//! `Tabs` — as **abas** do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/tabs.tsx`
//!
//! # Anatomia
//!
//! ```text
//!        ┌──────────────────────────────────────┐
//!        │ ▏ Geral ▕ │   Áudio   │   Vídeo      │  ← TabsList · TabsTab · INDICADOR na ativa
//!        └──────────────────────────────────────┘
//!        ┌──────────────────────────────────────┐
//!        │ o conteúdo da aba ativa              │  ← TabsPanel (ocupa o espaço restante)
//!        └──────────────────────────────────────┘
//! ```
//!
//! # Uso
//!
//! ```ignore
//! let tabs = cx.new(|cx| {
//!     Tabs::new(
//!         vec![
//!             TabsTab::new("Geral").icon("iconoir/regular/settings.svg"),
//!             TabsTab::new("Áudio"),
//!             TabsTab::new("Vídeo").disabled(true),
//!         ],
//!         0,
//!         cx,
//!     )
//!     .variant(TabsVariant::Underline)
//!     .content(|i, _window, _cx| div().child(format!("painel {i}")).into_any_element())
//! });
//!
//! cx.subscribe(&tabs, |_this, _tabs, e: &TabsEvent, _cx| match e {
//!     TabsEvent::Change(i) => println!("aba {i}"),
//! })
//! .detach();
//! ```
//!
//! # Contrato
//!
//! Mesmo contrato desacoplado dos outros controles do crate (ver [`crate::select::Select`]): é um
//! `Entity` (view) próprio que mantém o seu estado (`selected`) e **emite**
//! [`TabsEvent::Change`] com o novo índice ao trocar de aba por interação. `set_selected`
//! sincroniza de fora **sem** emitir (evita loop de feedback).
//!
//! O painel é opcional: com [`Tabs::content`] a view desenha a raiz completa (lista + painel);
//! sem ele, desenha só a tira de abas e quem usa monta o painel onde quiser (o container está
//! exposto como [`TabsPanel`]).
//!
//! # O indicador desliza
//!
//! O realce da aba ativa é **posicionado e dimensionado pela aba**, com 200ms de `ease-in-out` em
//! posição e largura — é o deslize característico do componente. No coss isso é CSS; aqui exige
//! saber os *bounds* da aba, então há dois caminhos, nesta ordem de precedência:
//!
//! 1. **Medido** — cada aba (e a lista) carrega um [`gpui::canvas`] overlay que grava os bounds no
//!    prepaint (mesmo truque do [`crate::select::Select`]). Com eles, o indicador é filho absoluto
//!    da LISTA, posicionado em coordenadas dela, e a troca de aba interpola do retângulo antigo pro
//!    novo. O motor de frames é o [`gpui::Window::request_animation_frame`], e o progresso vem do
//!    tempo decorrido — mesmo padrão de animação do [`crate::button`], sem elemento de animação nem
//!    `delta` pra sincronizar.
//! 2. **Fallback** — no primeiro frame ainda não há medida. Ali o indicador é filho absoluto da
//!    ABA ATIVA, preenchendo-a: a posição fica exata sem medir nada, só não desliza. É também a
//!    rede de segurança se a medição falhar por qualquer motivo.
//!
//! Os dois caminhos produzem o MESMO retângulo (a border box da aba), então a troca de um pro outro
//! não se vê. Ver [`rect_from_bounds`], que é onde a conversão de coordenadas vive.
//!
//! # Teclado
//!
//! Cada aba habilitada entra na ordem de tabulação e mostra o anel ao receber o foco. As setas do
//! eixo em uso movem a seleção **e** o foco (ativação automática, como o Radix faz por padrão),
//! pulando as abas desabilitadas e dando a volta nas pontas; `Home`/`End` vão pras extremidades; e
//! `Enter`/`Space` ativam a aba focada, pro caso de quem chegou nela por `Tab`.
//!
//! O clique **não** move o foco de teclado — é a aproximação mais próxima do `focus-visible` do
//! CSS, que o GPUI não distingue: se o clique focasse, todo clique acenderia o anel.
//!
//! # O que o GPUI exigiu adaptar
//!
//! | coss                                  | aqui                                                |
//! |---------------------------------------|-----------------------------------------------------|
//! | `focus-visible:ring-2`                | overlay em `inset:-2px` com `border_2` (raio+2)     |
//! | indicador posicionado por JS (Radix)  | filho absoluto da LISTA, posicionado por bounds     |
//! | `shadow-sm` no indicador              | só no tema **claro** (ver [`indicator_shadow`])     |
//! | posição da lista (`w-fit` no `flex-col`) | um wrap `flex justify_start` (não há `align_self`) |
//!
//! Duas armadilhas desta base valem repetir, porque as duas aparecem aqui:
//!
//! 1. **`ring` não existe no GPUI** e sombra não substitui — o `Window::paint_shadows` dilata os
//!    limites mas mantém o raio, e num fundo translúcido a sombra atravessa e tinge o
//!    componente. O anel é um overlay com **borda**. Ver [`ring_overlay`].
//! 2. **`Window::paint_shadows` não recorta a sombra pra fora do elemento** (diferente do CSS).
//!    Sombra externa atrás de fundo **opaco** é segura; atrás de fundo translúcido ela aparece
//!    ATRAVÉS do elemento. É por isso que a `shadow-sm` do indicador só existe no tema claro, o
//!    único em que ele é opaco. Ver [`indicator_shadow`].

use gpui::{
    canvas, div, point, prelude::FluentBuilder as _, px, svg, AnyElement, App, Bounds, Context,
    CursorStyle, Div, EventEmitter, FocusHandle, Hsla, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Pixels, Render, RenderOnce, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window,
};

use std::time::{Duration, Instant};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Os utilitários Tailwind do `tabs.tsx` resolvidos em número, com a paleta `neutral` do Tailwind
// expandida (`neutral-400 #a3a3a3`, `neutral-500 #737373`, `neutral-800 #262626`,
// `neutral-100 #f5f5f5`) e os `color-mix` já calculados.
//
// Mesma disciplina de cor do botão, do card e do campo de texto: TODO valor é `0xRRGGBBAA`, com o
// byte de alfa, sempre — ver [`crate::color`]. Misturar com os tokens de 6 dígitos de
// [`crate::theme`] desloca os canais e produz uma cor completamente diferente, **sem erro de
// compilação** (já aconteceu três vezes nesta base). A ponte é [`crate::color::opaque`].

/// Tokens visuais das abas, por tema.
#[derive(Clone, Copy, Debug)]
struct TabsPalette {
    /// `--background` — fundo do indicador da variante [`TabsVariant::Default`] no tema claro.
    background: Rgba8,
    /// `--input` — fundo do mesmo indicador no tema **escuro**, onde o coss troca de TOKEN (não só
    /// de valor). Ver [`indicator_bg`].
    input: Rgba8,
    /// `--muted` **e** `--accent`. No coss os dois resolvem pro MESMO valor (preto/branco a 4%),
    /// então é um campo só — dois campos idênticos seriam duas fontes de verdade pro mesmo número.
    /// É o fundo da lista na variante `Default` e o fundo de hover da aba na `Underline`.
    muted: Rgba8,
    /// `--muted-foreground` — cor do texto das abas INATIVAS.
    muted_foreground: Rgba8,
    /// `--foreground` — cor do texto da aba ATIVA.
    foreground: Rgba8,
    /// `--primary` — a barra do indicador na variante [`TabsVariant::Underline`].
    ///
    /// Nos dois temas do coss ele coincide com o `--foreground`; fica como token próprio de
    /// propósito, porque é assim no original e um tema derivado pode divergir.
    primary: Rgba8,
    /// `--ring` — cor do anel de foco.
    ring: Rgba8,
    /// Sombra externa do indicador (`shadow-sm` a 5%).
    shadow: Rgba8,
    /// Se este é o tema escuro. O indicador da variante `Default` troca de TOKEN entre os temas
    /// (`--background` no claro, `--input` no escuro), e é isso que decide se ele é opaco — logo,
    /// se pode ter sombra. Ver [`indicator_bg`] e [`indicator_shadow`].
    dark: bool,
}

/// Tema **claro**.
const TABS_LIGHT: TabsPalette = TabsPalette {
    background: Rgba8(0xffffffff),
    input: Rgba8(0x0000001a), // black 10%
    muted: Rgba8(0x0000000a), // black 4%
    // muted-foreground = mix(neutral-500 90%, black) = #686868
    muted_foreground: Rgba8(0x686868ff),
    foreground: Rgba8(0x262626ff), // neutral-800
    primary: Rgba8(0x262626ff),
    ring: Rgba8(0xa3a3a3ff),   // neutral-400
    shadow: Rgba8(0x0000000d), // black 5%
    dark: false,
};

/// Tema **escuro**.
const TABS_DARK: TabsPalette = TabsPalette {
    background: Rgba8(0x141414ff),
    input: Rgba8(0xffffff14), // white 8%
    muted: Rgba8(0xffffff0a), // white 4%
    // muted-foreground = mix(neutral-500 90%, white) = #818181
    muted_foreground: Rgba8(0x818181ff),
    foreground: Rgba8(0xf5f5f5ff), // neutral-100
    primary: Rgba8(0xf5f5f5ff),
    ring: Rgba8(0x737373ff),   // neutral-500
    shadow: Rgba8(0x0000000d), // black 5%
    dark: true,
};

/// A paleta das abas no tema corrente.
fn palette() -> &'static TabsPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &TABS_DARK,
        theme::ThemeMode::Light => &TABS_LIGHT,
    }
}

// --- Alfas (os modificadores `/N` do Tailwind) ---------------------------------------------------
//
// ⚠️ No Tailwind v4 o modificador `/N` é `color-mix(in oklab, <cor> N%, transparent)`, ou seja
// **multiplica** o alfa que a cor já tem — não o substitui. É por isso que tudo aqui passa por
// [`Rgba8::scaled`] em vez de reescrever o byte de alfa.

/// `text-muted-foreground/72` — o texto das abas inativas na variante [`TabsVariant::Default`],
/// mais apagado que na `Underline` porque ali a lista já tem um fundo próprio competindo com ele.
const MUTED_TEXT_ALPHA: f32 = 0.72;

/// `disabled:opacity-64` — o coss esmaece o CONJUNTO em vez de trocar cor por cor. Uma fonte de
/// verdade só: não existe um par "apagado" de cada token.
const DISABLED_OPACITY: f32 = 0.64;

// --- Geometria -----------------------------------------------------------------------------------
//
// Spacing do Tailwind: `--spacing(1)` = 4px. Raios do coss: `--radius-lg` = 10px,
// `--radius-md` = 8px. Todos os valores são as variantes `sm:` (≥640px) — uma janela desktop está
// sempre acima do breakpoint, então as classes base nunca valem.

/// `gap-2` entre a lista e o painel, na raiz.
const ROOT_GAP: f32 = 8.0;

/// `gap-0.5` entre as abas, dentro da lista.
const LIST_GAP: f32 = 2.0;

/// `rounded-lg` da lista (`--radius-lg`).
const LIST_RADIUS: f32 = 10.0;

/// `p-0.5` da lista na variante [`TabsVariant::Default`] — o respiro que faz o indicador "sentar"
/// dentro do trilho em vez de encostar na borda dele.
const LIST_PAD: f32 = 2.0;

/// `py-1` (horizontal) / `px-1` (vertical) da lista na variante [`TabsVariant::Underline`]: só no
/// eixo TRANSVERSAL. No eixo principal ela não tem respiro — a primeira aba começa na borda.
const LIST_PAD_UNDERLINE: f32 = 4.0;

/// `h-8` da aba. É a altura TOTAL (border-box, borda incluída), igual à do
/// [`crate::button::ButtonSize::Default`] — aba e botão têm que alinhar quando aparecem na mesma
/// barra.
const TAB_HEIGHT: f32 = 32.0;

/// `gap-1.5` entre o ícone e o rótulo da aba.
const TAB_GAP: f32 = 6.0;

/// `rounded-md` da aba (`--radius-md`), o mesmo do indicador da variante `Default`.
const TAB_RADIUS: f32 = 8.0;

/// `px-[calc(--spacing(2.5)-1px)]` — 9px, com o `-1px` descontando a borda pra o recuo TOTAL
/// (respiro + borda) fechar os 10px de `--spacing(2.5)`. Se alguém "arredondar" pra 10, toda aba
/// fica 1px mais larga de cada lado que o original.
const TAB_PAD_X: f32 = 9.0;

/// Espessura da borda da aba. Ela **existe em toda aba**, transparente, porque é ela que mantém a
/// altura interna estável — e é o que faz o `-1px` do [`TAB_PAD_X`] fazer sentido.
const TAB_BORDER: f32 = 1.0;

/// `sm:text-sm` — corpo do texto da aba.
const TAB_TEXT_SIZE: f32 = 14.0;

/// `sm:size-4` — lado do ícone da aba.
const TAB_ICON_SIZE: f32 = 16.0;

/// Quanto o ícone puxa pra fora de cada lado — o `-mx-0.5` do original, em módulo.
///
/// ⚠️ **Não implemente isso como margem negativa.** O `.mx(px(-2.0))` do GPUI colapsa a aba: as três
/// abas do exemplo com ícone caíram uma sobre a outra, com os rótulos sobrepostos. O efeito é
/// reproduzido pela geometria equivalente, que é o que a margem negativa produz no CSS:
///
/// - do lado de FORA, o ícone come 2px do respiro → o padding de início vira [`TAB_PAD_X`] − 2 = 7px;
/// - do lado de DENTRO, ele come 2px do vão → o gap ícone↔rótulo vira [`TAB_GAP`] − 2 = 4px;
/// - o padding do lado do rótulo continua [`TAB_PAD_X`] inteiro, porque o texto não tem margem.
///
/// Ou seja: uma aba com ícone é **assimétrica**, e é assim na referência também.
const TAB_ICON_PULL: f32 = 2.0;

/// Espessura da barra do indicador na variante [`TabsVariant::Underline`].
const UNDERLINE_THICKNESS: f32 = 2.0;

/// Espessura do anel de foco — `focus-visible:ring-2`.
const RING_WIDTH: f32 = 2.0;

/// Duração do deslize do indicador — `transition-[width,translate] duration-200 ease-in-out`.
const SLIDE: Duration = Duration::from_millis(200);

// =================================================================================================
// Variante e orientação
// =================================================================================================

/// Variante visual das abas.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TabsVariant {
    /// Trilho: a lista tem fundo `--muted` e a aba ativa recebe uma "pastilha" atrás do texto.
    #[default]
    Default,
    /// Sublinhado: a lista não tem fundo e a aba ativa recebe uma barra de 2px em `--primary`.
    Underline,
}

/// Eixo em que as abas se organizam.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TabsOrientation {
    /// Abas em linha, painel embaixo.
    #[default]
    Horizontal,
    /// Abas em coluna, painel ao lado.
    Vertical,
}

impl TabsOrientation {
    /// Se as abas empilham em coluna.
    pub fn is_vertical(self) -> bool {
        matches!(self, TabsOrientation::Vertical)
    }
}

// =================================================================================================
// Uma aba
// =================================================================================================

/// A **definição** de uma aba: o que ela mostra e se dá pra clicar.
///
/// Não é um elemento — é o dado que o [`Tabs`] guarda e re-renderiza a cada frame (o mesmo papel
/// que o `Vec<SharedString>` de opções tem no [`crate::select::Select`], só que com ícone e estado
/// desabilitado por item).
#[derive(Clone, Debug)]
pub struct TabsTab {
    label: SharedString,
    icon: Option<SharedString>,
    disabled: bool,
}

impl TabsTab {
    /// Uma aba com rótulo.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
        }
    }

    /// Um ícone **antes** do rótulo.
    ///
    /// O caminho é servido pela [`crate::assets::Assets`] (ex.:
    /// `"iconoir/regular/settings.svg"`). Sem essa `AssetSource` registrada no bootstrap, o ícone
    /// some SILENCIOSAMENTE.
    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }

    /// Desabilita a aba: ela para de responder a clique e hover, sai da ordem de tabulação e
    /// esmaece pra 64%. A navegação por setas pula por cima dela.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// O rótulo.
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// O caminho do ícone, se houver.
    pub fn icon_path(&self) -> Option<&SharedString> {
        self.icon.as_ref()
    }

    /// Se a aba está desabilitada.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

// =================================================================================================
// O painel
// =================================================================================================

/// O container do conteúdo da aba ativa: ocupa **o espaço restante** da raiz.
///
/// É um elemento de render (`RenderOnce`) sem estado — quem decide o que vai dentro é quem usa,
/// olhando [`Tabs::selected`]. O [`Tabs`] o monta sozinho quando recebe [`Tabs::content`].
#[derive(IntoElement, Default)]
pub struct TabsPanel {
    children: Vec<AnyElement>,
}

impl TabsPanel {
    /// Um painel vazio.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for TabsPanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for TabsPanel {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex_1()
            // Sem os mínimos em zero, um flex item não encolhe abaixo do conteúdo: um painel com
            // texto longo empurraria a raiz e estouraria o container em vez de recortar/rolar.
            .min_w(px(0.0))
            .min_h(px(0.0))
            .children(self.children)
    }
}

// =================================================================================================
// Estados visuais (funções livres, testáveis sem construir um `Tabs`)
// =================================================================================================
//
// Todas recebem a paleta por parâmetro em vez de chamar [`palette`]: assim os testes cobrem os DOIS
// temas sem mexer no tema global (que é estado de processo e vazaria entre testes em paralelo).

/// Fundo da lista. `None` = transparente.
///
/// É aqui que as duas variantes se separam de forma mais visível: a `Default` é um trilho (fundo
/// `--muted`), a `Underline` não tem fundo nenhum — é só a linha de base das abas.
fn list_bg(p: &TabsPalette, variant: TabsVariant) -> Option<Hsla> {
    match variant {
        TabsVariant::Default => Some(p.muted.hsla()),
        TabsVariant::Underline => None,
    }
}

/// Respiro interno da lista, por eixo — `(principal, transversal)`.
///
/// A `Default` respira nos dois eixos (`p-0.5`), pra a pastilha do indicador não encostar na borda
/// do trilho. A `Underline` respira **só no transversal** (`py-1`), o que dá altura pra barra sem
/// deslocar a primeira aba do início.
fn list_pad(variant: TabsVariant) -> (f32, f32) {
    match variant {
        TabsVariant::Default => (LIST_PAD, LIST_PAD),
        TabsVariant::Underline => (0.0, LIST_PAD_UNDERLINE),
    }
}

/// Fundo da aba no **hover**. `None` = sem fundo de hover.
///
/// Só a `Underline` tem: na `Default` o trilho já é um fundo, e um segundo fundo por cima dele
/// brigaria com a pastilha do indicador.
fn tab_hover_bg(p: &TabsPalette, variant: TabsVariant) -> Option<Hsla> {
    match variant {
        TabsVariant::Default => None,
        TabsVariant::Underline => Some(p.muted.hsla()),
    }
}

/// Cor do texto da aba **inativa em repouso**.
fn tab_text(p: &TabsPalette, variant: TabsVariant) -> Hsla {
    match variant {
        TabsVariant::Default => p.muted_foreground.scaled(MUTED_TEXT_ALPHA),
        TabsVariant::Underline => p.muted_foreground.hsla(),
    }
}

/// Cor do texto da aba inativa **sob o ponteiro** — o `--muted-foreground` cheio. Na `Default` é o
/// que faz o hover aparecer (o repouso está a 72%); na `Underline` coincide com o repouso, e é
/// assim no original.
fn tab_text_hover(p: &TabsPalette) -> Hsla {
    p.muted_foreground.hsla()
}

/// Cor do texto da aba **ativa** — `--foreground`, o valor mais forte da escala.
fn tab_text_active(p: &TabsPalette) -> Hsla {
    p.foreground.hsla()
}

/// Fundo do indicador.
///
/// Na `Default` o coss troca de TOKEN entre os temas, não só de valor: no claro é a superfície
/// opaca (`--background`), no escuro é o `--input` translúcido — que "levanta" sobre o trilho em
/// vez de tapá-lo. É essa troca que decide se cabe sombra (ver [`indicator_shadow`]).
fn indicator_bg(p: &TabsPalette, variant: TabsVariant) -> Hsla {
    match variant {
        TabsVariant::Default if p.dark => p.input.hsla(),
        TabsVariant::Default => p.background.hsla(),
        TabsVariant::Underline => p.primary.hsla(),
    }
}

/// Raio do indicador: `rounded-md` na `Default` (ele é uma pastilha do tamanho da aba), reto na
/// `Underline` (uma barra de 2px arredondada não leria como barra).
fn indicator_radius(variant: TabsVariant) -> f32 {
    match variant {
        TabsVariant::Default => TAB_RADIUS,
        TabsVariant::Underline => 0.0,
    }
}

/// A sombra externa do indicador (`shadow-sm` a 5%), ou vazio.
///
/// ⚠️ **DESVIO CONSCIENTE do coss, e é uma armadilha desta base.** O
/// [`gpui::Window::paint_shadows`] não recorta a sombra pra fora do elemento que a projeta
/// (diferente do CSS): ela é um retângulo arredondado CHEIO desenhado atrás. Atrás de um fundo
/// **opaco** ninguém vê — é o caso da `Default` no tema claro, onde o indicador é `--background`
/// (branco puro). No tema escuro o indicador é `--input` (branco a 8%, translúcido) e a sombra
/// apareceria ATRAVÉS dele, escurecendo justamente a pastilha que deveria levantar.
///
/// Então a sombra existe só onde é segura. Travado no teste
/// `sombra_do_indicador_so_onde_o_fundo_e_opaco`.
fn indicator_shadow(p: &TabsPalette, variant: TabsVariant) -> Vec<gpui::BoxShadow> {
    if variant != TabsVariant::Default || p.dark {
        return Vec::new();
    }
    // `shadow-sm` do Tailwind, com a cor em `--shadow` (preto 5%): duas camadas, uma mais espalhada
    // e uma justa e recolhida.
    vec![
        gpui::BoxShadow {
            color: p.shadow.hsla(),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(3.0),
            spread_radius: px(0.0),
        },
        gpui::BoxShadow {
            color: p.shadow.hsla(),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(2.0),
            spread_radius: px(-1.0),
        },
    ]
}

/// O **anel de foco** da aba — `focus-visible:ring-2`.
///
/// ⚠️ `ring` não existe no GPUI, e não dá pra fingir com sombra: o `spread_radius` dilata os
/// limites mas MANTÉM o raio, então a curvatura sai errada nas quinas, e num fundo translúcido a
/// sombra atravessa e tinge o componente (foi o que aconteceu no [`crate::input::Input`]).
///
/// Um `ring` é geometricamente a forma do elemento dilatada: 2px de anel = um overlay 2px maior em
/// cada lado, com borda de 2px e o raio crescendo junto (`raio + espessura`). Diferente do
/// [`crate::button::Button`], aqui **não** há `ring-offset`: a aba vive encostada nas vizinhas
/// dentro do trilho e uma folga extra faria o anel invadir o gap de 2px.
fn ring_overlay(p: &TabsPalette) -> Div {
    inset_overlay(-RING_WIDTH, TAB_RADIUS + RING_WIDTH)
        .border_2()
        .border_color(p.ring.hsla())
}

/// Um overlay absoluto recuado igualmente nos quatro lados. `inset` negativo cresce pra FORA.
///
/// ⚠️ O recuo é medido a partir da **padding box** (é assim que o GPUI/taffy resolve `inset` de
/// filho absoluto), então `inset:-1px` é o que cobre a BORDER box de um elemento com borda de 1px.
/// É a mesma convenção dos overlays do [`crate::button`].
fn inset_overlay(inset: f32, radius: f32) -> Div {
    div()
        .absolute()
        .top(px(inset))
        .left(px(inset))
        .right(px(inset))
        .bottom(px(inset))
        .rounded(px(radius))
}

// =================================================================================================
// Geometria do indicador
// =================================================================================================

/// Um retângulo em px, relativo à origem da LISTA. Existe pra a matemática do indicador ser lógica
/// pura (testável sem GPU) em vez de andar em [`gpui::Bounds`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// O retângulo da ABA, em coordenadas da lista, a partir dos bounds medidos das duas.
///
/// Devolve `None` enquanto a medição não aconteceu (primeiro frame): quem chama cai no indicador
/// de fallback, que é filho da própria aba e por isso não precisa de medida nenhuma.
///
/// ⚠️ **A compensação de 1px é intencional.** O `canvas` de medida é um filho absoluto com
/// `inset:0`, e no GPUI/taffy isso é a **padding box** — 1px menor que a border box de cada lado,
/// porque a aba tem borda de 1px (ver [`TAB_BORDER`]). O indicador tem que cobrir a border box (é
/// o tamanho externo da aba, que é o que o Radix mede com `offsetWidth`), então o retângulo cresce
/// [`TAB_BORDER`] pra cada lado.
fn rect_from_bounds(list: Bounds<Pixels>, tab: Bounds<Pixels>) -> Option<Rect> {
    if f32::from(list.size.width) <= 0.0 || f32::from(tab.size.width) <= 0.0 {
        return None;
    }
    Some(Rect {
        x: f32::from(tab.origin.x - list.origin.x) - TAB_BORDER,
        y: f32::from(tab.origin.y - list.origin.y) - TAB_BORDER,
        w: f32::from(tab.size.width) + 2.0 * TAB_BORDER,
        h: f32::from(tab.size.height) + 2.0 * TAB_BORDER,
    })
}

/// O retângulo do INDICADOR a partir do retângulo da aba ativa.
///
/// Na `Default` ele preenche a aba (é uma pastilha atrás do texto). Na `Underline` é uma barra de
/// [`UNDERLINE_THICKNESS`]: embaixo quando as abas estão em linha, na lateral de início quando
/// estão em coluna.
fn indicator_rect(variant: TabsVariant, orientation: TabsOrientation, tab: Rect) -> Rect {
    match (variant, orientation) {
        (TabsVariant::Default, _) => tab,
        (TabsVariant::Underline, TabsOrientation::Horizontal) => Rect {
            x: tab.x,
            y: tab.y + tab.h - UNDERLINE_THICKNESS,
            w: tab.w,
            h: UNDERLINE_THICKNESS,
        },
        (TabsVariant::Underline, TabsOrientation::Vertical) => Rect {
            x: tab.x,
            y: tab.y,
            w: UNDERLINE_THICKNESS,
            h: tab.h,
        },
    }
}

/// Interpola dois retângulos com `t` em `[0,1]` (`0` = `a`, `1` = `b`). `t` é aparado, porque quem
/// chama deriva de tempo decorrido e pode passar de 1.
fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: f32, y: f32| x + (y - x) * t;
    Rect {
        x: mix(a.x, b.x),
        y: mix(a.y, b.y),
        w: mix(a.w, b.w),
        h: mix(a.h, b.h),
    }
}

/// Curva do deslize — aproximação do `ease-in-out` do CSS (`cubic-bezier(.42,0,.58,1)`) por uma
/// quadrática simétrica: sai devagar, acelera no meio, freia no fim.
fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        let inv = -2.0 * t + 2.0;
        1.0 - inv * inv / 2.0
    }
}

// =================================================================================================
// Lógica de seleção (pura)
// =================================================================================================

/// Clampa um índice ao range válido de uma lista de tamanho `len` (último índice se estourar,
/// 0 se vazia).
fn clamp_idx(idx: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        idx.min(len - 1)
    }
}

/// Aplica a escolha de uma aba sobre o estado `selected`, devolvendo `(novo_selected, mudou)`.
///
/// É o miolo puro de [`Tabs::choose`] — extraído pra ser testável sem `Context`/GPU. `mudou` indica
/// se um [`TabsEvent::Change`] seria emitido: reescolher a aba ATIVA não emite (evento redundante),
/// e um índice fora do range é ignorado.
fn apply_choice(selected: usize, len: usize, idx: usize) -> (usize, bool) {
    if idx < len && idx != selected {
        (idx, true)
    } else {
        (selected, false)
    }
}

/// O próximo índice **habilitado** a partir de `from`, andando `dir` (`+1`/`-1`) e dando a volta.
/// `None` se não há nenhuma aba habilitada.
///
/// É a navegação por setas: uma aba desabilitada não pode receber a seleção, então ela é pulada em
/// vez de virar um beco sem saída.
fn step_index(from: usize, enabled: &[bool], dir: isize) -> Option<usize> {
    let len = enabled.len();
    if len == 0 {
        return None;
    }
    let mut i = from.min(len - 1);
    for _ in 0..len {
        i = (i as isize + dir).rem_euclid(len as isize) as usize;
        if enabled[i] {
            return Some(i);
        }
    }
    None
}

/// A primeira (`last = false`) ou a última (`last = true`) aba habilitada — `Home`/`End`.
fn edge_index(enabled: &[bool], last: bool) -> Option<usize> {
    if last {
        enabled.iter().rposition(|e| *e)
    } else {
        enabled.iter().position(|e| *e)
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// Evento emitido pelo [`Tabs`] quando o usuário troca de aba. Carrega o **índice** recém-ativado
/// (no mesmo espírito do [`crate::select::SelectEvent`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabsEvent {
    /// Nova aba ativa.
    Change(usize),
}

/// Um deslize do indicador em curso.
#[derive(Clone, Copy, Debug)]
struct Slide {
    /// De onde ele partiu, em coordenadas da lista. Já é a posição VISÍVEL no instante da troca —
    /// se um deslize anterior estava no meio do caminho, o novo começa de onde o olho o viu, não da
    /// aba anterior.
    from: Rect,
    /// Quando começou. O progresso vem do tempo decorrido (mesmo padrão de animação do
    /// [`crate::button`]), não de um `delta` de animador.
    start: Instant,
}

/// Abas com o visual do coss. Ver o doc do módulo.
pub struct Tabs {
    /// As abas, na ordem dos índices.
    tabs: Vec<TabsTab>,
    /// Índice ativo (`< tabs.len()`, salvo lista vazia).
    selected: usize,
    variant: TabsVariant,
    orientation: TabsOrientation,
    /// Id estável (pra `div().id(..)` único na árvore).
    id: u64,
    /// Um handle de foco por aba — é o que põe as abas na ordem de tabulação e liga o anel.
    focus_handles: Vec<FocusHandle>,
    /// Bounds de cada aba, capturados por um `canvas` overlay no prepaint. Alimentam o deslize.
    tab_bounds: Vec<Bounds<Pixels>>,
    /// Bounds da lista, pela mesma via — o indicador é posicionado em coordenadas DELA.
    list_bounds: Bounds<Pixels>,
    /// O deslize em curso, se houver.
    slide: Option<Slide>,
    /// O conteúdo do painel, em função do índice ativo. `None` = a view desenha só a tira de abas.
    #[allow(clippy::type_complexity)]
    content: Option<Box<dyn Fn(usize, &mut Window, &mut App) -> AnyElement + 'static>>,
}

impl Tabs {
    /// Cria um `Tabs` com `tabs` e o índice inicial `selected`. Se `selected` estourar a lista, é
    /// clampado ao último índice válido (ou 0 numa lista vazia).
    pub fn new(tabs: Vec<TabsTab>, selected: usize, cx: &mut Context<Self>) -> Self {
        let selected = clamp_idx(selected, tabs.len());
        Self {
            focus_handles: tabs.iter().map(|_| cx.focus_handle()).collect(),
            tab_bounds: vec![Bounds::default(); tabs.len()],
            tabs,
            selected,
            variant: TabsVariant::default(),
            orientation: TabsOrientation::default(),
            id: cx.entity_id().as_u64(),
            list_bounds: Bounds::default(),
            slide: None,
            content: None,
        }
    }

    /// Variante visual.
    pub fn variant(mut self, variant: TabsVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Eixo das abas.
    pub fn orientation(mut self, orientation: TabsOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Define o **conteúdo do painel** em função do índice ativo. Com ele, a view desenha a raiz
    /// completa (lista + [`TabsPanel`]); sem ele, desenha só a tira de abas.
    ///
    /// O fechamento é `'static` (é guardado na view e chamado a cada frame), então não pode capturar
    /// referências emprestadas — capture `Entity`/`WeakEntity` se precisar de estado de fora.
    pub fn content(
        mut self,
        content: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.content = Some(Box::new(content));
        self
    }

    /// Índice da aba ativa.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// As abas, na ordem dos índices.
    pub fn tabs(&self) -> &[TabsTab] {
        &self.tabs
    }

    /// A variante em uso.
    pub fn current_variant(&self) -> TabsVariant {
        self.variant
    }

    /// O eixo em uso.
    pub fn current_orientation(&self) -> TabsOrientation {
        self.orientation
    }

    /// Ativa uma aba de fora (sincronização externa). **Não** emite `Change` — evita loops de
    /// feedback com quem assina (mesma regra do [`crate::select::Select::set_selected`]). O índice
    /// é clampado ao range válido, e o indicador desliza como se fosse um clique.
    pub fn set_selected(&mut self, selected: usize, cx: &mut Context<Self>) {
        let selected = clamp_idx(selected, self.tabs.len());
        if self.selected == selected {
            return;
        }
        self.start_slide();
        self.selected = selected;
        cx.notify();
    }

    /// Troca a lista de abas (e re-clampa a seleção). Sincronização externa, **não** emite.
    pub fn set_tabs(&mut self, tabs: Vec<TabsTab>, cx: &mut Context<Self>) {
        // Os handles e os bounds são POR ABA: se a lista muda de tamanho, os antigos não descrevem
        // mais a tira. Recriar os handles é o que impede uma aba nova de herdar o foco de uma que
        // não existe mais.
        if tabs.len() != self.tabs.len() {
            self.focus_handles = tabs.iter().map(|_| cx.focus_handle()).collect();
            self.tab_bounds = vec![Bounds::default(); tabs.len()];
            self.slide = None;
        }
        self.selected = clamp_idx(self.selected, tabs.len());
        self.tabs = tabs;
        cx.notify();
    }

    /// Ativa uma aba por interação: **emite** `Change(idx)` se mudou.
    fn choose(&mut self, idx: usize, cx: &mut Context<Self>) {
        let (selected, changed) = apply_choice(self.selected, self.tabs.len(), idx);
        if changed {
            self.start_slide();
            self.selected = selected;
            cx.emit(TabsEvent::Change(idx));
        }
        cx.notify();
    }

    /// Marca o começo de um deslize, partindo de onde o indicador está VISÍVEL agora.
    ///
    /// Tem que ser chamado **antes** de `self.selected` mudar — é a posição antiga que vira o ponto
    /// de partida. Sem bounds medidos (primeiro frame) não há deslize: o indicador simplesmente
    /// aparece na aba nova, que é o fallback correto.
    fn start_slide(&mut self) {
        self.slide = self.visible_rect().map(|from| Slide {
            from,
            start: Instant::now(),
        });
    }

    /// O retângulo da aba `i` em coordenadas da lista, se já medido.
    fn tab_rect(&self, i: usize) -> Option<Rect> {
        rect_from_bounds(self.list_bounds, *self.tab_bounds.get(i)?)
    }

    /// Onde o indicador está NESTE frame: no destino, ou no meio do caminho se há deslize em curso.
    fn visible_rect(&self) -> Option<Rect> {
        let target = indicator_rect(self.variant, self.orientation, self.tab_rect(self.selected)?);
        Some(match self.slide {
            Some(s) => {
                let t = s.start.elapsed().as_secs_f32() / SLIDE.as_secs_f32();
                lerp_rect(s.from, target, ease_in_out(t))
            }
            None => target,
        })
    }

    /// Quais abas estão habilitadas — a máscara que a navegação por setas consulta.
    fn enabled_mask(&self) -> Vec<bool> {
        self.tabs.iter().map(|t| !t.disabled).collect()
    }

    /// O índice da aba que está com o foco de teclado, se alguma.
    fn focused_tab(&self, window: &Window) -> Option<usize> {
        self.focus_handles.iter().position(|h| h.is_focused(window))
    }

    /// Teclado na tira de abas.
    ///
    /// As setas do eixo em uso movem a seleção **e** o foco (ativação automática, como o Radix faz
    /// por padrão); `Home`/`End` vão pras pontas; `Enter`/`Space` ativam a aba que está com o foco
    /// — o caso de quem chegou ali por `Tab`, sem passar pelas setas.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let enabled = self.enabled_mask();
        let vertical = self.orientation.is_vertical();
        let key = event.keystroke.key.as_str();

        let alvo = match key {
            "right" if !vertical => step_index(self.selected, &enabled, 1),
            "left" if !vertical => step_index(self.selected, &enabled, -1),
            "down" if vertical => step_index(self.selected, &enabled, 1),
            "up" if vertical => step_index(self.selected, &enabled, -1),
            "home" => edge_index(&enabled, false),
            "end" => edge_index(&enabled, true),
            // Ativa a aba FOCADA (não a vizinha): aqui o usuário já escolheu onde está.
            "enter" | "space" => self.focused_tab(window).filter(|i| enabled[*i]),
            _ => return,
        };

        let Some(alvo) = alvo else { return };
        // Chegou aqui = a tecla é uma das que a tira trata, ou seja o usuário está navegando pelo
        // teclado. O `focus_ring::init` já teria marcado isso; marcar de novo aqui faz as setas
        // funcionarem mesmo num app que esqueceu de inicializar o crate.
        crate::focus_ring::keyboard_used(window);
        self.choose(alvo, cx);
        if let Some(handle) = self.focus_handles.get(alvo) {
            window.focus(handle);
        }
    }

    /// Uma aba.
    fn render_tab(&self, i: usize, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = palette();
        let tab = &self.tabs[i];
        let active = i == self.selected;
        let vertical = self.orientation.is_vertical();
        let interactive = !tab.disabled;
        let focused = interactive
            && self
                .focus_handles
                .get(i)
                .is_some_and(|h| h.is_focused(window));

        let text = if active {
            tab_text_active(p)
        } else {
            tab_text(p, self.variant)
        };

        // Decide o respiro e o vão: a aba com ícone é assimétrica. Ver [`TAB_ICON_PULL`].
        let tem_icone = tab.icon.is_some();

        let mut el = div()
            // O id só precisa ser único ENTRE AS IRMÃS: o `GlobalElementId` do GPUI é o caminho de
            // ids dos ancestrais, e a raiz já carrega o id da view (ver o fim do `render`).
            .id(("tabs-tab", i))
            // `relative` porque o anel, o canvas de medida e o indicador de fallback são filhos
            // ABSOLUTOS.
            .relative()
            .flex()
            .items_center()
            // O vão ícone↔rótulo: 4px quando há ícone (o `gap-1.5` menos o que o `-mx-0.5` puxa),
            // 6px quando não há — sem ícone não há nada pra puxar. Ver [`TAB_ICON_PULL`].
            .gap(px(if tem_icone { TAB_GAP - TAB_ICON_PULL } else { TAB_GAP }))
            .h(px(TAB_HEIGHT))
            // `flex-1`: as abas dividem o espaço da lista igualmente. Numa lista de largura de
            // conteúdo isso as deixa todas com a largura da mais larga.
            .flex_1()
            // Respiro assimétrico quando há ícone: o lado do ícone perde 2px, o lado do rótulo não.
            .pl(px(if tem_icone {
                TAB_PAD_X - TAB_ICON_PULL
            } else {
                TAB_PAD_X
            }))
            .pr(px(TAB_PAD_X))
            .rounded(px(TAB_RADIUS))
            // A borda de 1px EXISTE em toda aba, transparente: é ela que mantém a altura interna
            // estável e é o que o `-1px` do respiro horizontal desconta.
            .border(px(TAB_BORDER))
            .border_color(gpui::transparent_black())
            .text_size(px(TAB_TEXT_SIZE))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(text);

        // Vertical: a aba ocupa a largura da coluna e o conteúdo alinha ao INÍCIO (uma pilha de
        // rótulos centralizados não lê como lista). Horizontal: conteúdo centralizado.
        el = if vertical {
            el.w_full().justify_start()
        } else {
            el.justify_center()
        };

        if interactive {
            el = el.cursor(CursorStyle::PointingHand).track_focus(
                self.focus_handles
                    .get(i)
                    .expect("um handle de foco por aba (ver `new`/`set_tabs`)"),
            );

            // O hover é refinamento de estilo, sem fade: as abas não têm o motor de interpolação de
            // fundo do `Button` — aqui o que se move é o indicador, e dois movimentos ao mesmo
            // tempo brigariam pela atenção.
            //
            // A aba ATIVA não muda de cor de texto no hover: ela já está em `--foreground`, e
            // "apagar" pra `--muted-foreground` sob o ponteiro seria um passo pra trás.
            let hover_bg = tab_hover_bg(p, self.variant);
            let hover_text = (!active).then(|| tab_text_hover(p));
            el = el.hover(move |mut style| {
                if let Some(bg) = hover_bg {
                    style = style.bg(bg);
                }
                if let Some(color) = hover_text {
                    style = style.text_color(color);
                }
                style
            });

            el = el.on_click(cx.listener(move |this, _e, window, cx| {
                // O `track_focus` acima faz o GPUI focar a aba no mouse-down. O foco em si é
                // desejável (o teclado continua de onde o clique parou); o que não é desejável é o
                // ANEL, que na referência é `focus-visible` e não acende no clique.
                crate::focus_ring::pointer_used(window);
                this.choose(i, cx);
            }));
        } else {
            // `disabled:opacity-64`: esmaece o conjunto — a aba e os overlays dela.
            el = el.opacity(DISABLED_OPACITY);
        }

        // --- Indicador de FALLBACK ------------------------------------------------------------
        //
        // Enquanto os bounds não estiverem medidos (primeiro frame), o indicador é filho ABSOLUTO
        // da aba ativa: a posição fica exata sem medir nada, só não desliza. Depois disso ele passa
        // a ser filho da lista (ver `render_list`) — e é lá, e só lá, que o deslize existe.
        let fallback = active && self.visible_rect().is_none();
        if fallback && self.variant == TabsVariant::Default {
            // Antes do conteúdo: filho absoluto pintado primeiro fica ATRÁS do texto.
            el = el.child(indicator_element(p, self.variant, None, vertical));
        }

        // --- Conteúdo ---------------------------------------------------------------------------
        if let Some(path) = tab.icon.clone() {
            el = el.child(
                svg()
                    .path(path)
                    .size(px(TAB_ICON_SIZE))
                    .flex_none()
                    .text_color(text),
            );
        }
        el = el.child(div().flex_none().child(tab.label.clone()));

        if fallback && self.variant == TabsVariant::Underline {
            // Depois do conteúdo: a barra da `Underline` fica NA FRENTE.
            el = el.child(indicator_element(p, self.variant, None, vertical));
        }

        // --- Overlays, por último ---------------------------------------------------------------
        if focused && crate::focus_ring::visible() {
            el = el.child(ring_overlay(p));
        }

        // Mede a aba pro deslize. É um filho absoluto SEM interatividade: não cria hitbox, então
        // não intercepta o clique. E não chama `cx.notify()` — só grava; notificar aqui daria um
        // laço de render infinito.
        el.child(measure_canvas(cx, move |this, bounds| {
            if let Some(slot) = this.tab_bounds.get_mut(i) {
                *slot = bounds;
            }
        }))
    }

    /// A tira de abas (`TabsList`).
    fn render_list(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let p = palette();
        let vertical = self.orientation.is_vertical();
        let (pad_main, pad_cross) = list_pad(self.variant);

        let mut list = div()
            // `relative` porque o indicador medido e o canvas de medida são filhos ABSOLUTOS.
            .relative()
            .flex()
            .gap(px(LIST_GAP))
            // Largura de conteúdo (`w-fit`): sem grow nem shrink, a lista mede o que as abas pedem.
            .flex_none()
            .rounded(px(LIST_RADIUS));

        list = if vertical {
            list.flex_col().px(px(pad_cross)).py(px(pad_main))
        } else {
            list.py(px(pad_cross)).px(px(pad_main))
        };

        if let Some(bg) = list_bg(p, self.variant) {
            list = list.bg(bg);
        }

        // O indicador MEDIDO da `Default` vem antes das abas: filho absoluto pintado primeiro fica
        // ATRÁS delas (que têm fundo transparente).
        let rect = self.visible_rect();
        if rect.is_some() && self.variant == TabsVariant::Default {
            list = list.child(indicator_element(p, self.variant, rect, vertical));
        }

        for i in 0..self.tabs.len() {
            list = list.child(self.render_tab(i, window, cx));
        }

        // O da `Underline` vem depois: a barra fica NA FRENTE.
        if rect.is_some() && self.variant == TabsVariant::Underline {
            list = list.child(indicator_element(p, self.variant, rect, vertical));
        }

        // ⚠️ A lista NÃO pode ter `overflow_hidden`: o anel de foco cresce 2px pra fora da aba e a
        // barra da `Underline` encosta na borda — os dois seriam recortados.
        list.child(measure_canvas(cx, |this, bounds| {
            this.list_bounds = bounds;
        }))
        // O teclado é ouvido pela LISTA, não pela aba: o evento sobe do elemento focado pelos
        // ancestrais, então um listener só cobre todas as abas.
        .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
            this.on_key(e, window, cx);
        }))
    }
}

/// O elemento do indicador.
///
/// Com `rect`, posicionado em coordenadas da LISTA (o caminho que desliza). Sem, um overlay que
/// cobre a aba inteira — o `inset:-1px` é o que faz ele cobrir a BORDER box da aba (ver
/// [`inset_overlay`]), pra bater exatamente com o retângulo que [`rect_from_bounds`] calcula.
fn indicator_element(
    p: &TabsPalette,
    variant: TabsVariant,
    rect: Option<Rect>,
    vertical: bool,
) -> Div {
    let base = match rect {
        Some(r) => div()
            .absolute()
            .left(px(r.x))
            .top(px(r.y))
            .w(px(r.w))
            .h(px(r.h)),
        None => match variant {
            TabsVariant::Default => inset_overlay(-TAB_BORDER, indicator_radius(variant)),
            // A barra do fallback encosta nas três bordas do lado dela e mede 2px na outra direção.
            TabsVariant::Underline if vertical => div()
                .absolute()
                .top(px(-TAB_BORDER))
                .bottom(px(-TAB_BORDER))
                .left(px(-TAB_BORDER))
                .w(px(UNDERLINE_THICKNESS)),
            TabsVariant::Underline => div()
                .absolute()
                .left(px(-TAB_BORDER))
                .right(px(-TAB_BORDER))
                .bottom(px(-TAB_BORDER))
                .h(px(UNDERLINE_THICKNESS)),
        },
    };

    base.rounded(px(indicator_radius(variant)))
        .bg(indicator_bg(p, variant))
        .shadow(indicator_shadow(p, variant))
}

/// Um `canvas` overlay que só **mede**: grava os bounds do elemento pai na view e não pinta nada.
///
/// Mesmo truque do [`crate::select::Select`] (que o usa pra largura do popup). O `inset:0`
/// explícito é de propósito: sem ele, um absoluto de insets `auto` cai na posição estática (dentro
/// do padding), e a medida sairia deslocada pelo respiro da aba.
fn measure_canvas(
    cx: &mut Context<Tabs>,
    write: impl Fn(&mut Tabs, Bounds<Pixels>) + 'static,
) -> impl IntoElement {
    let view = cx.entity();
    canvas(
        move |bounds, _window, cx| {
            view.update(cx, |this, _| write(this, bounds));
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

impl EventEmitter<TabsEvent> for Tabs {}

impl Render for Tabs {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Um deslize concluído é descartado aqui, no começo do frame: enquanto ele existisse,
        // `visible_rect` continuaria interpolando (com `t` aparado em 1, mas fazendo conta à toa).
        if self.slide.is_some_and(|s| s.start.elapsed() >= SLIDE) {
            self.slide = None;
        }
        let sliding = self.slide.is_some();
        let vertical = self.orientation.is_vertical();

        let list = self.render_list(window, cx);

        // A tira assenta no INÍCIO do eixo transversal, não no centro.
        //
        // Ela é `w-fit` dentro de um `flex-col`: como a largura não é `auto`, o `align-items:
        // stretch` do CSS não a estica, e o item cai no cross-start — ou seja, à esquerda. Uma
        // versão anterior a centralizava, por um `mx-auto` que não existe na referência (veio de uma
        // imprecisão na especificação que originou este arquivo).
        //
        // O wrap continua sendo necessário porque o GPUI não expõe `align_self`, e sem ele a tira
        // esticaria pela largura toda; `items_center` na RAIZ resolveria a tira e estragaria o
        // painel, que precisa ocupar a largura inteira.
        let wrap = div()
            .flex()
            .flex_none()
            .when(vertical, |d| d.flex_col())
            .justify_start()
            .child(list);

        // A raiz: coluna (lista em cima, painel embaixo) ou linha, quando as abas são verticais.
        let mut root = div()
            .id(("tabs", self.id))
            .flex()
            .gap(px(ROOT_GAP))
            .when(!vertical, |d| d.flex_col())
            .child(wrap);

        if let Some(content) = self.content.as_ref() {
            let selected = self.selected;
            root = root.child(TabsPanel::new().child(content(selected, window, cx)));
        }

        // Enquanto o indicador estiver deslizando, pede o próximo frame. É o motor da animação — não
        // há elemento de animação, o progresso vem do tempo decorrido (mesmo padrão do
        // [`crate::button`]).
        if sliding {
            window.request_animation_frame();
        }

        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::size;

    /// As duas variantes, pros testes varrerem sem esquecer nenhuma.
    const VARIANTES: [TabsVariant; 2] = [TabsVariant::Default, TabsVariant::Underline];

    /// As duas paletas, com nome pras mensagens de falha.
    const PALETAS: [(&str, &TabsPalette); 2] = [("claro", &TABS_LIGHT), ("escuro", &TABS_DARK)];

    /// Um retângulo de aba plausível, pra geometria.
    const ABA: Rect = Rect {
        x: 10.0,
        y: 4.0,
        w: 80.0,
        h: TAB_HEIGHT,
    };

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(x), px(y)),
            size: size(px(w), px(h)),
        }
    }

    /// **A convenção de cor da paleta, decodificada de verdade.**
    ///
    /// Todo valor de [`TabsPalette`] é `0xRRGGBBAA` e é consumido por `rgba`; um valor de 6 dígitos
    /// esquecido ali vira uma cor completamente diferente **sem erro de compilação** —
    /// `rgba(0xffffff)` é lido como `0x00FFFFFF`, ou seja ciano. Já aconteceu três vezes nesta base.
    /// Então, em vez de comparar números com números (que não pegaria nada), este teste decodifica e
    /// afirma o que a cor DEVE ser perceptualmente.
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        // Os NEUTROS OPACOS: se algum sair colorido, o valor foi lido deslocado.
        for (tema, p) in PALETAS {
            for (nome, c) in [
                ("background", p.background),
                ("muted-foreground", p.muted_foreground),
                ("foreground", p.foreground),
                ("primary", p.primary),
                ("ring", p.ring),
            ] {
                let c: gpui::Rgba = c.hsla().into();
                assert_eq!(c.a, 1.0, "{tema}/{nome}: opaco");
                assert!(
                    (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                    "{tema}/{nome}: NEUTRO — se r≠g≠b, o valor foi lido deslocado"
                );
            }
        }

        // Os TRANSLÚCIDOS: é o alfa que os deixa funcionar sobre qualquer fundo. Um deles opaco
        // significaria um retângulo chapado no lugar de um véu.
        for (tema, p) in PALETAS {
            for (nome, c) in [("muted", p.muted), ("input", p.input), ("shadow", p.shadow)] {
                assert!(
                    c.alpha() < 1.0,
                    "{tema}/{nome} tem que ser translúcido, veio com alfa {}",
                    c.alpha()
                );
            }
        }

        // O par background/foreground INVERTE entre os temas — se os dois ficarem do mesmo lado, a
        // aba ativa desaparece contra o fundo.
        let bg_claro: gpui::Rgba = TABS_LIGHT.background.hsla().into();
        let fg_claro: gpui::Rgba = TABS_LIGHT.foreground.hsla().into();
        assert!(bg_claro.r > 0.9 && fg_claro.r < 0.2, "claro: fundo claro, texto escuro");
        let bg_escuro: gpui::Rgba = TABS_DARK.background.hsla().into();
        let fg_escuro: gpui::Rgba = TABS_DARK.foreground.hsla().into();
        assert!(bg_escuro.r < 0.2 && fg_escuro.r > 0.9, "escuro: fundo escuro, texto claro");

        // E o `--muted-foreground` fica ENTRE os dois: é o texto da aba inativa, que tem que ler
        // menos que o da ativa e mais que o fundo.
        for (tema, p) in PALETAS {
            let bg: gpui::Rgba = p.background.hsla().into();
            let fg: gpui::Rgba = p.foreground.hsla().into();
            let mf: gpui::Rgba = p.muted_foreground.hsla().into();
            let (baixo, alto) = if bg.r < fg.r { (bg.r, fg.r) } else { (fg.r, bg.r) };
            assert!(
                mf.r > baixo && mf.r < alto,
                "{tema}: muted-foreground tem que ficar entre o fundo e o texto ativo"
            );
        }

        // O flag de tema não pode estar trocado — é ele que escolhe o TOKEN do indicador.
        assert_eq!(
            (TABS_LIGHT.dark, TABS_DARK.dark),
            (false, true),
            "o flag `dark` de uma das paletas está trocado"
        );
    }

    /// A seleção clampa, troca e **só emite quando muda de verdade** — reescolher a aba ativa não
    /// pode disparar um `Change` redundante (é o que faria um assinante gravar/renderizar à toa).
    #[test]
    fn selecao_clampa_e_so_emite_quando_muda() {
        assert_eq!(clamp_idx(0, 3), 0);
        assert_eq!(clamp_idx(2, 3), 2);
        assert_eq!(clamp_idx(5, 3), 2, "estoura → último válido");
        assert_eq!(clamp_idx(0, 0), 0, "lista vazia → 0");
        assert_eq!(clamp_idx(9, 0), 0);

        // Escolher uma aba válida e diferente: muda e sinaliza Change.
        assert_eq!(apply_choice(0, 3, 2), (2, true));
        // Reescolher a ativa: nada muda, e SEM Change.
        assert_eq!(apply_choice(2, 3, 2), (2, false));
        // Fora do range: ignorado, mantém a seleção.
        assert_eq!(apply_choice(1, 3, 9), (1, false));
        // Lista vazia: não há o que escolher.
        assert_eq!(apply_choice(0, 0, 0), (0, false));
    }

    /// A navegação por setas **pula as desabilitadas** e dá a volta. Sem isso, uma aba desabilitada
    /// no meio viraria um beco sem saída pro teclado.
    #[test]
    fn navegacao_por_setas_pula_desabilitadas_e_da_a_volta() {
        let todas = [true, true, true];
        assert_eq!(step_index(0, &todas, 1), Some(1));
        assert_eq!(step_index(2, &todas, 1), Some(0), "dá a volta pra frente");
        assert_eq!(step_index(0, &todas, -1), Some(2), "dá a volta pra trás");

        let meio_off = [true, false, true];
        assert_eq!(step_index(0, &meio_off, 1), Some(2), "pula a do meio");
        assert_eq!(step_index(2, &meio_off, -1), Some(0), "pula a do meio, de volta");

        // Só uma habilitada: a seta volta pra ela mesma (e não devolve uma desabilitada).
        let so_uma = [false, true, false];
        assert_eq!(step_index(1, &so_uma, 1), Some(1));

        // Nenhuma habilitada ou lista vazia: não há alvo.
        assert_eq!(step_index(0, &[false, false], 1), None);
        assert_eq!(step_index(0, &[], 1), None);

        // Home/End respeitam o desabilitado nas pontas.
        assert_eq!(edge_index(&[false, true, true, false], false), Some(1));
        assert_eq!(edge_index(&[false, true, true, false], true), Some(2));
        assert_eq!(edge_index(&[false], false), None);
    }

    /// A geometria do coss, em número. São os valores que o olho compara com o resto da UI: a aba
    /// tem a MESMA altura e o MESMO raio de um [`crate::button::ButtonSize::Default`], e a lista usa
    /// o raio maior (`--radius-lg`) porque envolve as abas.
    #[test]
    fn geometria_segue_a_referencia() {
        assert_eq!(TAB_HEIGHT, 32.0, "h-8");
        assert_eq!(TAB_RADIUS, 8.0, "--radius-md");
        assert_eq!(LIST_RADIUS, 10.0, "--radius-lg");
        assert_eq!(UNDERLINE_THICKNESS, 2.0, "a barra da Underline");
        assert_eq!(TAB_TEXT_SIZE, 14.0, "sm:text-sm");
        assert_eq!(TAB_ICON_SIZE, 16.0, "sm:size-4");
        assert_eq!(TAB_GAP, 6.0, "gap-1.5");
        assert_eq!(LIST_GAP, 2.0, "gap-0.5");
        assert_eq!(ROOT_GAP, 8.0, "gap-2");

        // A lista é MAIS arredondada que a aba: ela envolve as abas, e um raio menor faria a
        // pastilha do indicador vazar pela quina do trilho.
        let (lista, aba) = (LIST_RADIUS, TAB_RADIUS);
        assert!(lista > aba, "a lista tem que ser mais arredondada que a aba");

        // O respiro horizontal vem de `px-[calc(--spacing(2.5)-1px)]`: o -1px desconta a borda pra
        // o recuo TOTAL cair na escala de spacing do Tailwind (2.5 = 10px). Se alguém "arredondar"
        // pra 10, toda aba fica 1px mais larga de cada lado que o original.
        assert_eq!(TAB_PAD_X, 9.0);
        assert_eq!(TAB_PAD_X + TAB_BORDER, 10.0, "--spacing(2.5)");

        // A aba COM ícone é assimétrica: o `-mx-0.5` do ícone come 2px do respiro de início e 2px do
        // vão até o rótulo. Reproduzido por geometria, porque margem negativa colapsa a aba no GPUI
        // (ver [`TAB_ICON_PULL`]) — se alguém "simplificar" isso pra um `px` simétrico, o teste cai.
        assert_eq!(TAB_ICON_PULL, 2.0, "-mx-0.5");
        assert_eq!(TAB_PAD_X - TAB_ICON_PULL, 7.0, "respiro do lado do ícone");
        assert_eq!(TAB_GAP - TAB_ICON_PULL, 4.0, "vão visível ícone↔rótulo");

        // O anel cresce a própria espessura pra fora, e o raio acompanha — senão a curvatura das
        // quinas não fecha com a da aba.
        assert_eq!(RING_WIDTH, 2.0, "focus-visible:ring-2");
        let anel_raio = TAB_RADIUS + RING_WIDTH;
        assert_eq!(anel_raio, 10.0, "raio do anel = raio da aba + espessura");

        // A `Underline` respira só no eixo transversal: no principal a primeira aba começa na borda
        // da lista. Se os dois fossem iguais, a barra ficaria descolada do início.
        assert_eq!(list_pad(TabsVariant::Default), (LIST_PAD, LIST_PAD));
        assert_eq!(list_pad(TabsVariant::Underline), (0.0, LIST_PAD_UNDERLINE));

        assert_eq!(SLIDE, Duration::from_millis(200), "duration-200");
    }

    /// **As duas variantes diferem onde devem.** A `Default` é um trilho (a lista tem fundo e a aba
    /// ativa ganha uma pastilha do tamanho dela); a `Underline` não tem fundo de lista nenhum e o
    /// realce é uma barra. Trocar uma pela outra sem trocar isso daria duas variantes idênticas.
    #[test]
    fn variantes_diferem_no_trilho_e_no_indicador() {
        for (tema, p) in PALETAS {
            // Fundo da lista: existe na Default, não existe na Underline.
            let trilho = list_bg(p, TabsVariant::Default).expect("a Default tem trilho");
            assert!(trilho.a > 0.0 && trilho.a < 1.0, "{tema}: o trilho é um véu");
            assert!(
                list_bg(p, TabsVariant::Underline).is_none(),
                "{tema}: a Underline NÃO tem fundo de lista"
            );

            // Fundo de hover da aba: só a Underline, pela mesma razão inversa (na Default o trilho
            // já é o fundo).
            assert!(tab_hover_bg(p, TabsVariant::Default).is_none());
            assert!(tab_hover_bg(p, TabsVariant::Underline).is_some());

            // Raio do indicador: pastilha arredondada vs. barra reta.
            assert_eq!(indicator_radius(TabsVariant::Default), TAB_RADIUS);
            assert_eq!(indicator_radius(TabsVariant::Underline), 0.0);

            // E as duas nunca pintam o indicador da mesma cor: uma é superfície, a outra é acento.
            assert_ne!(
                indicator_bg(p, TabsVariant::Default),
                indicator_bg(p, TabsVariant::Underline),
                "{tema}: superfície e acento não podem coincidir"
            );
        }

        // O indicador da Underline é o `--primary`, opaco e no extremo da escala — é ele que marca
        // a aba ativa quando não há pastilha.
        for (tema, p) in PALETAS {
            let barra: gpui::Rgba = indicator_bg(p, TabsVariant::Underline).into();
            assert_eq!(barra.a, 1.0, "{tema}: a barra é opaca");
        }

        // Já o da Default TROCA de token entre os temas: superfície opaca no claro, `--input`
        // translúcido no escuro (que levanta sobre o trilho em vez de tapá-lo).
        assert_eq!(
            indicator_bg(&TABS_LIGHT, TabsVariant::Default),
            TABS_LIGHT.background.hsla()
        );
        assert_eq!(
            indicator_bg(&TABS_DARK, TabsVariant::Default),
            TABS_DARK.input.hsla()
        );
        assert_eq!(indicator_bg(&TABS_LIGHT, TabsVariant::Default).a, 1.0);
        assert!(indicator_bg(&TABS_DARK, TabsVariant::Default).a < 1.0);
    }

    /// O texto sobe de ênfase: repouso → hover → ativa. Na `Default` o repouso está a 72%, e é essa
    /// diferença que faz o hover ser perceptível sem trocar de cor.
    #[test]
    fn texto_ganha_enfase_de_repouso_pra_ativa() {
        assert_eq!(MUTED_TEXT_ALPHA, 0.72);
        for (tema, p) in PALETAS {
            let repouso = tab_text(p, TabsVariant::Default);
            let hover = tab_text_hover(p);
            let ativa = tab_text_active(p);

            assert!(
                repouso.a < hover.a,
                "{tema}/Default: o hover tem que acender o texto (72% → 100%)"
            );
            assert!((hover.a - 1.0).abs() < 1e-6, "{tema}: o hover é o token cheio");
            assert_eq!(ativa, p.foreground.hsla(), "{tema}: a ativa é --foreground");
            assert_ne!(hover, ativa, "{tema}: hover e ativa não podem coincidir");

            // Na Underline o repouso já é o token cheio (não há trilho competindo com o texto).
            assert_eq!(tab_text(p, TabsVariant::Underline), hover);
        }

        assert_eq!(DISABLED_OPACITY, 0.64, "disabled:opacity-64");
    }

    /// **Desvio consciente do coss, travado aqui.** A `shadow-sm` do indicador só existe onde o
    /// fundo dele é OPACO — o tema claro. No escuro ele é `--input` (translúcido) e a sombra
    /// apareceria através dele, porque o `Window::paint_shadows` não a recorta pra fora do elemento
    /// (diferente do CSS). Ver [`indicator_shadow`].
    ///
    /// Se alguém "corrigir" isto achando que é esquecimento, este teste falha e aponta pra cá.
    #[test]
    fn sombra_do_indicador_so_onde_o_fundo_e_opaco() {
        let claro = indicator_shadow(&TABS_LIGHT, TabsVariant::Default);
        assert!(!claro.is_empty(), "claro: o indicador é opaco, a sombra é segura");
        assert_eq!(indicator_bg(&TABS_LIGHT, TabsVariant::Default).a, 1.0);

        assert!(
            indicator_shadow(&TABS_DARK, TabsVariant::Default).is_empty(),
            "escuro: o indicador é translúcido, a sombra vazaria por dentro dele"
        );

        // A barra da `Underline` também não projeta sombra em tema nenhum: são 2px, e uma sombra
        // ali só sujaria a linha.
        for (_, p) in PALETAS {
            assert!(indicator_shadow(p, TabsVariant::Underline).is_empty());
        }

        // E a sombra que existe é sutil: preto a 5%, deslocada 1px pra baixo.
        for s in &claro {
            assert!(s.color.a < 0.1, "sombra sutil (5%)");
            assert_eq!(f32::from(s.offset.y), 1.0);
        }
    }

    /// O indicador da `Default` preenche a aba; o da `Underline` é uma barra de 2px, embaixo quando
    /// as abas estão em linha e na lateral quando estão em coluna. Uma inversão de eixo aqui daria
    /// uma barra atravessada no meio do texto.
    #[test]
    fn indicador_preenche_ou_e_barra_de_2px() {
        // Default: exatamente a aba, nos dois eixos.
        for orientacao in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
            assert_eq!(indicator_rect(TabsVariant::Default, orientacao, ABA), ABA);
        }

        // Underline horizontal: largura cheia, 2px de altura, encostada na BASE da aba.
        let barra = indicator_rect(TabsVariant::Underline, TabsOrientation::Horizontal, ABA);
        assert_eq!(barra.w, ABA.w, "a barra acompanha a largura da aba");
        assert_eq!(barra.h, UNDERLINE_THICKNESS);
        assert_eq!(barra.x, ABA.x);
        assert_eq!(barra.y + barra.h, ABA.y + ABA.h, "encostada na base");

        // Underline vertical: altura cheia, 2px de largura, no INÍCIO da aba.
        let lateral = indicator_rect(TabsVariant::Underline, TabsOrientation::Vertical, ABA);
        assert_eq!(lateral.h, ABA.h, "a barra acompanha a altura da aba");
        assert_eq!(lateral.w, UNDERLINE_THICKNESS);
        assert_eq!(lateral.x, ABA.x, "na lateral de início");
        assert_eq!(lateral.y, ABA.y);

        // Em nenhum caso o indicador sai da aba.
        for variante in VARIANTES {
            for orientacao in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
                let r = indicator_rect(variante, orientacao, ABA);
                assert!(r.x >= ABA.x && r.y >= ABA.y);
                assert!(r.x + r.w <= ABA.x + ABA.w + 1e-6);
                assert!(r.y + r.h <= ABA.y + ABA.h + 1e-6);
            }
        }
    }

    /// A medida vira coordenada RELATIVA à lista e cresce 1px pra cada lado.
    ///
    /// Os dois pontos são fáceis de errar: o `canvas` de medida devolve bounds absolutos da janela
    /// (se alguém esquecer de subtrair a origem da lista, o indicador voa pra fora), e ele mede a
    /// **padding box**, 1px menor que a border box da aba de cada lado (ver [`rect_from_bounds`]).
    #[test]
    fn medida_da_aba_vira_coordenada_da_lista() {
        let lista = bounds(100.0, 50.0, 300.0, 36.0);
        let aba = bounds(203.0, 55.0, 78.0, 30.0);

        let r = rect_from_bounds(lista, aba).expect("lista e aba medidas");
        // 203 - 100 = 103, menos 1px de borda.
        assert_eq!(r.x, 102.0);
        assert_eq!(r.y, 4.0);
        // A border box é 2px maior que a padding box em cada dimensão.
        assert_eq!(r.w, 80.0);
        assert_eq!(r.h, 32.0, "a altura volta pros 32px totais da aba");

        // Antes da primeira medição não há retângulo — quem chama cai no indicador de fallback, que
        // é filho da aba e não precisa de medida.
        assert!(rect_from_bounds(Bounds::default(), aba).is_none(), "lista não medida");
        assert!(rect_from_bounds(lista, Bounds::default()).is_none(), "aba não medida");
    }

    /// O deslize sai de onde estava e chega onde tem que chegar, desacelerando nas duas pontas.
    #[test]
    fn deslize_interpola_e_desacelera() {
        let a = Rect { x: 0.0, y: 0.0, w: 40.0, h: 32.0 };
        let b = Rect { x: 100.0, y: 0.0, w: 80.0, h: 32.0 };

        assert_eq!(lerp_rect(a, b, 0.0), a, "começa em a");
        assert_eq!(lerp_rect(a, b, 1.0), b, "termina em b");
        // A LARGURA também interpola — é metade do efeito: sem ela o indicador teleporta de tamanho
        // e só a posição desliza.
        let meio = lerp_rect(a, b, 0.5);
        assert_eq!(meio.x, 50.0);
        assert_eq!(meio.w, 60.0);
        // `t` é aparado: quem chama deriva de tempo decorrido e passa de 1 no último frame.
        assert_eq!(lerp_rect(a, b, 3.0), b);
        assert_eq!(lerp_rect(a, b, -1.0), a);

        // A curva: simétrica, monotônica, e passando pelo meio no meio do tempo.
        assert_eq!(ease_in_out(0.0), 0.0);
        assert!((ease_in_out(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6);
        assert!(ease_in_out(0.25) < 0.25, "sai devagar (ease-in)");
        assert!(ease_in_out(0.75) > 0.75, "chega devagar (ease-out)");
        let mut anterior = -1.0;
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let atual = ease_in_out(t);
            assert!(atual > anterior, "ease_in_out tem que ser monotônica");
            // Simetria: f(t) + f(1-t) = 1.
            assert!((atual + ease_in_out(1.0 - t) - 1.0).abs() < 1e-5, "simétrica em t={t}");
            anterior = atual;
        }
    }

    /// A definição de uma aba: rótulo obrigatório, ícone e desabilitado opcionais.
    #[test]
    fn definicao_de_aba_guarda_o_que_foi_pedido() {
        let simples = TabsTab::new("Geral");
        assert_eq!(simples.label().as_ref(), "Geral");
        assert!(simples.icon_path().is_none());
        assert!(!simples.is_disabled(), "por padrão a aba é clicável");

        let completa = TabsTab::new("Vídeo")
            .icon("iconoir/regular/media_video.svg")
            .disabled(true);
        assert_eq!(
            completa.icon_path().map(|s| s.as_ref()),
            Some("iconoir/regular/media_video.svg")
        );
        assert!(completa.is_disabled());

        assert_eq!(TabsVariant::default(), TabsVariant::Default);
        assert_eq!(TabsOrientation::default(), TabsOrientation::Horizontal);
        assert!(TabsOrientation::Vertical.is_vertical());
        assert!(!TabsOrientation::Horizontal.is_vertical());
    }
}

/// Testes que precisam de uma JANELA — os únicos deste crate que abrem uma.
///
/// O resto do módulo é lógica pura (mesma disciplina do [`crate::select`]), mas duas coisas aqui
/// **não são verificáveis sem layout**, e as duas são de quebrar em silêncio:
///
/// 1. a hipótese de que o `canvas` de medida devolve a **padding box** do elemento pai (é o que a
///    compensação de 1px de [`rect_from_bounds`] assume). Se o GPUI/taffy mudar isso, o indicador
///    sai 1px torto — e nenhum teste puro nota, porque a aritmética continua certa;
/// 2. que a aba desabilitada realmente não recebe clique, e que reclicar a ativa não emite. Isso
///    depende de hitbox, não de [`apply_choice`].
///
/// A janela é headless (`VisualTestContext`), no mesmo espírito de
/// `crates/empire-ui/tests/multi_click_selection.rs`.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{point, AppContext as _, Modifiers, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Largura do container do harness — a lista é centralizada DENTRO dele.
    const LARGURA: f32 = 600.0;

    /// Um container de largura fixa, que é o que o `Tabs` precisa pra ter onde centralizar a tira.
    struct Harness {
        tabs: gpui::Entity<Tabs>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(LARGURA))
                .flex()
                .flex_col()
                .child(self.tabs.clone())
        }
    }

    /// Abre a janela com três abas (a do meio DESABILITADA) e devolve a view, os eventos
    /// capturados e o contexto visual.
    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
        variant: TabsVariant,
    ) -> (gpui::Entity<Tabs>, Rc<RefCell<Vec<usize>>>, VisualTestContext) {
        let eventos: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_window, cx| {
            let tabs = cx.new(|cx| {
                Tabs::new(
                    vec![
                        TabsTab::new("Geral"),
                        TabsTab::new("Audio").disabled(true),
                        TabsTab::new("Video"),
                    ],
                    0,
                    cx,
                )
                .variant(variant)
            });
            cx.subscribe(&tabs, move |_this, _t, ev: &TabsEvent, _cx| match ev {
                TabsEvent::Change(i) => capturados.borrow_mut().push(*i),
            })
            .detach();
            Harness { tabs }
        });

        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let tabs = vcx.read(|cx| harness.read(cx).tabs.clone());
        (tabs, eventos, vcx)
    }

    /// O centro da aba `i`, em coordenadas da janela.
    fn centro(tabs: &gpui::Entity<Tabs>, vcx: &mut VisualTestContext, i: usize) -> gpui::Point<Pixels> {
        vcx.read(|cx| {
            let b = tabs.read(cx).tab_bounds[i];
            point(
                b.origin.x + b.size.width / 2.0,
                b.origin.y + b.size.height / 2.0,
            )
        })
    }

    /// **A medida bate com a geometria declarada.** É o teste que valida a compensação de 1px: o
    /// retângulo derivado tem que ser a BORDER box da aba (32px de altura, o `h-8` do coss), com a
    /// origem no respiro da lista — e não a padding box, 2px menor, que é o que o `canvas` mede.
    #[gpui::test]
    fn a_medida_reconstroi_a_border_box_da_aba(cx: &mut TestAppContext) {
        let (tabs, _eventos, vcx) = abrir(cx, TabsVariant::Default);

        vcx.read(|cx| {
            let t = tabs.read(cx);

            // A lista mede 32px de aba + o respiro dos dois lados.
            assert_eq!(
                f32::from(t.list_bounds.size.height),
                TAB_HEIGHT + 2.0 * LIST_PAD,
                "altura da lista = altura da aba + p-0.5 em cima e embaixo"
            );

            // A tira assenta no INÍCIO do container, não no centro: ela é `w-fit` dentro de um
            // `flex-col`, e um item cuja largura não é `auto` não sofre o `align-items: stretch` —
            // cai no cross-start. A primeira versão centralizava, por um `mx-auto` que a referência
            // não tem.
            assert_eq!(
                f32::from(t.list_bounds.origin.x),
                0.0,
                "a lista encosta na borda de início do container"
            );
            assert!(
                f32::from(t.list_bounds.size.width) < LARGURA,
                "e é `w-fit`: não estica pelo container ({} de {LARGURA})",
                f32::from(t.list_bounds.size.width)
            );

            let primeira = t.tab_rect(0).expect("aba medida depois do primeiro frame");
            assert_eq!(primeira.h, TAB_HEIGHT, "a aba tem 32px TOTAIS (border-box)");
            assert_eq!(
                (primeira.x, primeira.y),
                (LIST_PAD, LIST_PAD),
                "a primeira aba começa no respiro da lista"
            );

            // As abas dividem o espaço igualmente (`flex-1`) e são separadas pelo gap de 2px.
            let segunda = t.tab_rect(1).expect("aba medida");
            assert_eq!(segunda.w, primeira.w, "flex-1: mesma largura");
            assert_eq!(
                segunda.x,
                primeira.x + primeira.w + LIST_GAP,
                "gap-0.5 entre as abas"
            );

            // E o indicador da `Default` preenche exatamente a aba ativa.
            assert_eq!(t.visible_rect(), Some(primeira));
        });
    }

    /// **A barra da `Underline` encosta na base da aba ativa.** Um erro de sinal no eixo a colocaria
    /// atravessada no meio do texto.
    #[gpui::test]
    fn a_barra_da_underline_fica_na_base(cx: &mut TestAppContext) {
        let (tabs, _eventos, vcx) = abrir(cx, TabsVariant::Underline);

        vcx.read(|cx| {
            let t = tabs.read(cx);
            // Sem respiro no eixo principal: a primeira aba começa NA borda da lista.
            let aba = t.tab_rect(0).expect("aba medida");
            assert_eq!(aba.x, 0.0, "a Underline não respira no eixo principal");
            assert_eq!(aba.y, LIST_PAD_UNDERLINE, "py-1 no eixo transversal");

            let barra = t.visible_rect().expect("indicador posicionado");
            assert_eq!(barra.h, UNDERLINE_THICKNESS);
            assert_eq!(barra.w, aba.w, "a barra acompanha a largura da aba");
            assert_eq!(barra.y + barra.h, aba.y + aba.h, "encostada na base da aba");
        });
    }

    /// **O clique troca de aba e emite — uma vez.** E a aba desabilitada não tem hitbox: clicar nela
    /// não faz nada. As três regras juntas são o contrato de quem assina o evento.
    #[gpui::test]
    fn clique_troca_emite_e_respeita_o_desabilitado(cx: &mut TestAppContext) {
        let (tabs, eventos, mut vcx) = abrir(cx, TabsVariant::Default);

        let terceira = centro(&tabs, &mut vcx, 2);
        vcx.simulate_click(terceira, Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| tabs.read(cx).selected()), 2);
        assert_eq!(&*eventos.borrow(), &[2], "trocar de aba emite Change");

        // A aba do meio está desabilitada: sem cursor, sem clique, sem evento.
        let desabilitada = centro(&tabs, &mut vcx, 1);
        vcx.simulate_click(desabilitada, Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            vcx.read(|cx| tabs.read(cx).selected()),
            2,
            "clicar numa aba desabilitada não muda a seleção"
        );
        assert_eq!(&*eventos.borrow(), &[2], "e não emite nada");

        // Reclicar a ativa não emite de novo (evento redundante).
        vcx.simulate_click(terceira, Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(&*eventos.borrow(), &[2], "reclicar a ativa não emite");

        // E a troca acendeu um deslize, que é o que o `request_animation_frame` mantém vivo.
        vcx.read(|cx| assert!(tabs.read(cx).slide.is_some(), "a troca acende o deslize"));
    }

    /// **O clique NÃO acende o anel de foco; a seta acende.**
    ///
    /// Este é o `focus-visible` da referência. O `track_focus` faz o GPUI focar a aba no mouse-down,
    /// e como o GPUI não distingue a modalidade do foco, a primeira versão acendia um anel de 2px em
    /// toda aba clicada — o defeito que motivou este teste. O foco em si continua indo pra aba
    /// clicada (o teclado precisa dele pra continuar de onde o clique parou); só o anel espera pelo
    /// teclado.
    #[gpui::test]
    fn o_clique_nao_acende_o_anel_mas_a_seta_acende(cx: &mut TestAppContext) {
        let (tabs, _eventos, mut vcx) = abrir(cx, TabsVariant::Default);

        let terceira = centro(&tabs, &mut vcx, 2);
        vcx.simulate_click(terceira, Modifiers::default());
        vcx.run_until_parked();

        vcx.update(|window, cx| {
            let t = tabs.read(cx);
            assert!(
                !crate::focus_ring::visible(),
                "clicar não pode acender o anel (a referência é focus-visible)"
            );
            assert_eq!(
                t.focused_tab(window),
                Some(2),
                "mas o foco VAI pra aba clicada — é o que faz o teclado continuar dali"
            );
        });

        // Agora pelo teclado: o anel passa a valer.
        vcx.simulate_keystrokes("left");
        vcx.run_until_parked();
        assert!(
            crate::focus_ring::visible(),
            "navegar por seta acende o anel"
        );

        // E um clique depois o apaga de novo — é a modalidade do ÚLTIMO input, não um trilho de mão
        // única.
        let primeira = centro(&tabs, &mut vcx, 0);
        vcx.simulate_click(primeira, Modifiers::default());
        vcx.run_until_parked();
        assert!(
            !crate::focus_ring::visible(),
            "voltar pro mouse apaga o anel"
        );
    }

    /// **As setas movem a seleção pulando as desabilitadas.** O listener vive na LISTA e recebe a
    /// tecla porque o evento sobe da aba focada — se ele estivesse na aba, cada aba precisaria do
    /// seu, e a aba sem foco nunca ouviria nada.
    #[gpui::test]
    fn seta_move_a_selecao_pela_lista(cx: &mut TestAppContext) {
        let (tabs, eventos, mut vcx) = abrir(cx, TabsVariant::Default);

        vcx.update(|window, cx| {
            let handle = tabs.read(cx).focus_handles[0].clone();
            window.focus(&handle);
        });
        vcx.run_until_parked();

        // Da 0 pra direita: a 1 está desabilitada, então o alvo é a 2.
        vcx.simulate_keystrokes("right");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| tabs.read(cx).selected()), 2, "a seta pula a desabilitada");
        assert_eq!(&*eventos.borrow(), &[2]);

        // E dá a volta: da 2 pra direita cai na 0.
        vcx.simulate_keystrokes("right");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| tabs.read(cx).selected()), 0, "dá a volta");
        assert_eq!(&*eventos.borrow(), &[2, 0]);

        // O foco acompanha a seleção — senão a próxima seta partiria de outro lugar.
        let focada = vcx.update(|window, cx| tabs.read(cx).focused_tab(window));
        assert_eq!(focada, Some(0), "o foco segue a seleção");
    }
}

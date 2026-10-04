//! `Toast` — as **notificações transitórias** do `empire-ui`, com o visual do design system
//! [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/toast.tsx`
//!
//! # Duas peças
//!
//! - [`ToastManager`] — a **fila**, um [`gpui::Entity`] (view). Recebe [`ToastManager::push`],
//!   [`ToastManager::dismiss`] e [`ToastManager::clear`], emite [`ToastEvent`], conta o tempo do
//!   auto-dismiss e **desenha o viewport**. É view (e não só estado) porque o auto-dismiss precisa
//!   que `cx.notify()` provoque um novo frame: um elemento `RenderOnce` só redesenharia quando a
//!   view hospedeira redesenhasse.
//! - [`toast_layer`] — o **elemento montável**: um overlay absoluto que cobre a raiz da view
//!   hospedeira e dentro do qual o viewport se posiciona. Existe porque o GPUI **não tem portal
//!   nem `position: fixed`** — quem planta o overlay na janela é a app, não o componente.
//!
//! Não há [`gpui::Global`] envolvido: a fila é um `Entity` normal, então dá pra ter duas
//! independentes (ou nenhuma) e testar sem tocar em estado de processo.
//!
//! # Montagem
//!
//! ```ignore
//! struct Shell { toasts: Entity<ToastManager> }
//!
//! impl Shell {
//!     fn new(cx: &mut Context<Self>) -> Self {
//!         Self {
//!             toasts: cx.new(|cx| {
//!                 ToastManager::new(cx).with_position(ToastPosition::BottomRight)
//!             }),
//!         }
//!     }
//! }
//!
//! impl Render for Shell {
//!     fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
//!         div()
//!             .relative()      // ⚠️ obrigatório: o overlay é ABSOLUTO
//!             .size_full()
//!             .child(o_conteudo_da_app())
//!             .child(toast_layer(&self.toasts))   // por ÚLTIMO: pinta em cima
//!     }
//! }
//! ```
//!
//! E pra empurrar um toast, de qualquer handler:
//!
//! ```ignore
//! self.toasts.update(cx, |m, cx| {
//!     m.push(
//!         Toast::new("Salvo").description("As alterações foram publicadas.").success(),
//!         cx,
//!     );
//! });
//! ```
//!
//! # O que o GPUI exigiu adaptar
//!
//! O empilhamento da referência é **todo** `transform: translateY(...) scale(...)`, e o GPUI não
//! tem `transform` em `div` (a [`gpui::TransformationMatrix`] só serve pra `svg`/imagem). Então o
//! empilhamento aqui é **geometria calculada**: cada toast é um filho absoluto com
//! `w`/`h`/`bottom` (ou `top`) computados, e a escala é reproduzida por largura fracionária
//! (`gpui::relative`) com o recuo lateral que centraliza. Ver [`geom`], onde a álgebra do
//! `transform-origin` da referência está resolvida.
//!
//! | coss                                       | aqui                                          |
//! |--------------------------------------------|-----------------------------------------------|
//! | `transform: translateY(…) scale(…)`        | `w`/`left` fracionários + `h`/`bottom` em px   |
//! | `transition: transform .5s cubic-bezier(…)`| lerp de geometria por tempo ([`ease`])         |
//! | `position: fixed` + portal                 | [`toast_layer`], montado pela app             |
//! | `before:shadow-[0_1px_…]` (bisel)          | overlay em `inset:-1px` com `border_b_1`      |
//! | `after:h-[calc(gap+1px)]` (ponte de hover) | uma região de hover que cobre a pilha inteira |
//!
//! # Onde isto NÃO é a referência
//!
//! Tudo abaixo é decisão consciente, e não descuido. O que foi **deduzido** (e não lido) está
//! marcado com ⚠️ no ponto exato do código.
//!
//! **Não implementado**
//!
//! - **Swipe-to-dismiss.** A referência dispensa por arraste (`swipeDirection`, e todo o bloco de
//!   `data-[swipe-direction=…]` de saída direcional). Aqui dispensa por **clique** no toast — a
//!   referência não tem `Toast.Close` nem qualquer afordância de fechar, e inventar um X visível
//!   mudaria o pixel.
//! - **`AnchoredToasts`** — o segundo componente do `.tsx`, um toast ancorado num elemento
//!   (`Toast.Positioner`, com a variante `tooltipStyle` de raio `md` e `px-2 py-1`). É outro
//!   componente, com outro `toastManager`, e está fora do escopo desta entrega.
//! - **`upsertReplayClassName`** — as classes `animate-toast-{error,success}-{even,odd}` que a
//!   referência aplica quando um toast é ATUALIZADO no lugar (`updateKey`). Não há API de update
//!   aqui, e as animações em si não estão no `.tsx` (moram no config do Tailwind, indisponível).
//!
//! **Diferente por limitação do GPUI**
//!
//! - `not-dark:bg-clip-padding` **não tem equivalente**: o GPUI pinta o fundo na border box. No
//!   tema claro a borda (`--border` = preto 8%, translúcida) fica sobre o branco do fundo em vez de
//!   sobre o que houver atrás do toast — na prática, a borda lê como `#ebebeb` fixo. Diferença de
//!   um pixel de contorno, visível só com o toast sobre um fundo escuro.
//! - A **altura** transiciona em 0,5s junto com o resto da geometria; na referência ela tem
//!   `height .15s` própria. Um lerp só, com a curva da geometria, no lugar de dois relógios.
//!
//! **Valores deduzidos**
//!
//! - Os tokens `--info`, `--success` e `--warning` do coss **não estavam resolvidos em nenhum lugar
//!   desta base**, e o `globals.css` do coss não estava disponível. Os valores em
//!   [`TOAST_LIGHT`]/[`TOAST_DARK`] seguem a mesma regra que o `--destructive` já resolvido no
//!   [`crate::button`] (cor-500 do Tailwind no claro, `mix(cor-500 90%, white)` no escuro).
//! - [`LINE_HEIGHT`] (o `h-lh` do ícone e a altura de linha do `text-sm`), [`DEFAULT_LIMIT`] e
//!   [`DEFAULT_TIMEOUT`] (defaults do `@base-ui/react/toast`, que a referência não sobrescreve).

use gpui::{
    canvas, div, point, px, radians, AnyElement, App, Bounds, Context, CursorStyle, Div,
    EventEmitter, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, Render, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled, Transformation, Window,
};

use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::color::{lerp, Rgba8};
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Mesma disciplina de cor do campo de texto, do card e do botão: TODO valor é `0xRRGGBBAA`, com o
// byte de alfa, sempre — ver [`crate::color`]. Misturar com os tokens de 6 dígitos de
// [`crate::theme`] desloca os canais e produz uma cor completamente diferente, sem erro de
// compilação (já aconteceu três vezes nesta base).

/// Tokens visuais do toast, por tema.
#[derive(Clone, Copy, Debug)]
struct ToastPalette {
    /// `--popover` — fundo da superfície. Os valores vêm do [`crate::button`], que já os tinha
    /// resolvidos (`popover`).
    popover: Rgba8,
    /// `--popover-foreground` — texto do título (= `--foreground`).
    popover_fg: Rgba8,
    /// `--muted-foreground` — texto da descrição.
    muted_fg: Rgba8,
    /// `--border` — a borda de 1px da superfície.
    border: Rgba8,
    /// `shadow-lg/5` — a cor das duas sombras externas.
    shadow: Rgba8,
    /// Fio de bisel de 1px. Desce no claro, sobe no escuro.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro), `-1` sobe (escuro).
    bevel_dir: f32,
    /// `--destructive` — ícone do tipo `error`.
    destructive: Rgba8,
    /// `--info` — ícone do tipo `info`. **DEDUZIDO** (ver o doc do módulo).
    info: Rgba8,
    /// `--success` — ícone do tipo `success`. **DEDUZIDO**.
    success: Rgba8,
    /// `--warning` — ícone do tipo `warning`. **DEDUZIDO**.
    warning: Rgba8,
    /// Quanto de PRETO misturar no fundo por índice de pilha — o
    /// `color-mix(in srgb, var(--popover), black calc(N% * index))` da referência. 1% no claro,
    /// 6% no escuro.
    darken_step: f32,
}

/// Tema **claro**.
const TOAST_LIGHT: ToastPalette = ToastPalette {
    popover: Rgba8(0xffffffff),
    popover_fg: Rgba8(0x262626ff), // neutral-800
    // mix(neutral-500 90%, black) = #686868 — o mesmo do `crate::card`.
    muted_fg: Rgba8(0x686868ff),
    border: Rgba8(0x00000014), // black 8%
    shadow: Rgba8(0x0000000d), // black 5% (o `/5` do `shadow-lg/5`)
    bevel: Rgba8(0x0000000a),  // black 4%
    bevel_dir: 1.0,
    destructive: Rgba8(0xef4444ff), // red-500
    // LIDOS do `packages/ui/src/styles/globals.css` do coss, não deduzidos.
    //
    // Uma versão anterior supôs que os três seguiam a cor-500 "óbvia" do Tailwind e errou o
    // `success`: ele é **emerald**, não green. Vale a pena reparar que emerald-500 é bem mais
    // azulado que green-500 (`#10b981` contra `#22c55e`).
    info: Rgba8(0x3b82f6ff),    // blue-500
    success: Rgba8(0x10b981ff), // emerald-500
    warning: Rgba8(0xf59e0bff), // amber-500
    darken_step: 0.01,
};

/// Tema **escuro**.
const TOAST_DARK: ToastPalette = ToastPalette {
    popover: Rgba8(0x1d1d1dff),
    popover_fg: Rgba8(0xf5f5f5ff), // neutral-100
    // mix(neutral-500 90%, white) = #818181 — o mesmo do `crate::card`.
    muted_fg: Rgba8(0x818181ff),
    border: Rgba8(0xffffff0f), // white 6%
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE do coss: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO —
    // alfa 30 ≈ 11,8% — pela MESMA decisão de design já vigente no `Input`, no `Card`, no `Button`
    // e no `Frame`: a 6% o filete é imperceptível no nosso fundo escuro. NÃO "corrija" isto pra
    // 0x0f achando que é erro de porte; se a intenção mudar, mude junto o teste
    // `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
    // destructive escuro = mix(red-500 90%, white) = #f15757 (valor já resolvido no `crate::button`).
    destructive: Rgba8(0xf15757ff),
    // LIDOS do `globals.css` do coss: no tema escuro estes três são **idênticos ao claro**. Só o
    // `--destructive` sofre o `color-mix` com branco — uma versão anterior generalizou essa regra
    // pros três e errou todos. Se em algum momento parecerem escuros demais no fundo escuro, a
    // correção é uma decisão de design nossa, não um erro de porte.
    info: Rgba8(0x3b82f6ff),    // blue-500, igual ao claro
    success: Rgba8(0x10b981ff), // emerald-500, igual ao claro
    warning: Rgba8(0xf59e0bff), // amber-500, igual ao claro
    darken_step: 0.06,
};

/// A paleta do toast no tema corrente.
fn toast() -> &'static ToastPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &TOAST_DARK,
        theme::ThemeMode::Light => &TOAST_LIGHT,
    }
}

// =================================================================================================
// Geometria (utilitários Tailwind da referência resolvidos em número)
// =================================================================================================

/// Raio da superfície — `rounded-lg`, que no coss é `--radius-lg` = **10px**.
///
/// ⚠️ O bisel usa ESTE raio, não `10 − 1`. O `calc(var(--radius-lg) - 1px)` do `before:` da
/// referência descreve um pseudo-elemento na *padding* box; o nosso overlay está na *border* box
/// (ver [`bevel_overlay`] e o mesmo raciocínio em [`crate::card`]).
const RADIUS: f32 = 10.0;

/// Respiro horizontal do conteúdo — `px-3.5`.
const PAD_X: f32 = 14.0;

/// Respiro vertical do conteúdo — `py-3`.
const PAD_Y: f32 = 12.0;

/// Corpo do texto — `text-sm`.
const TEXT_SIZE: f32 = 14.0;

/// Altura de linha do `text-sm`.
///
/// ⚠️ **DEDUZIDO**, não lido: é o default do Tailwind pra `text-sm` (`1.25rem` = 20px). O coss pode
/// redefinir `--text-sm--line-height`; se redefinir, é aqui que muda — e a altura natural de um
/// toast muda junto (ver [`estimated_height`]).
const LINE_HEIGHT: f32 = 20.0;

/// Espaço entre o bloco de texto e a ação — `gap-1.5`.
const GAP_ACTION: f32 = 6.0;

/// Espaço entre o ícone e o texto — `gap-2`.
const GAP_ICON: f32 = 8.0;

/// Espaço entre título e descrição — `gap-0.5`.
const GAP_TEXT: f32 = 2.0;

/// Largura do ícone — `w-4`. A ALTURA é [`LINE_HEIGHT`] (`h-lh`), e o SVG fica centrado nela: no
/// CSS o `preserveAspectRatio` default encaixa o `viewBox` quadrado numa caixa 16×20 como 16×16
/// centrado, e é isso que reproduzimos (o GPUI esticaria o SVG pra caixa inteira).
const ICON_SIZE: f32 = 16.0;

/// Opacidade do ícone de `loading` — `opacity-80`.
const LOADING_OPACITY: f32 = 0.8;

/// Recuo do viewport em relação à borda da janela — `sm:[--toast-inset:--spacing(8)]` = **32px**.
///
/// A referência tem 16px no mobile (`--spacing(4)`); nesta base `sm:` sempre se aplica (desktop),
/// então só o 32 existe.
const INSET: f32 = 32.0;

/// Largura máxima do viewport — `max-w-90` = **360px**.
const MAX_WIDTH: f32 = 360.0;

/// Quanto cada toast de trás aparece acima do da frente, na pilha COLAPSADA — `--toast-peek`
/// (`--spacing(3)`).
const PEEK: f32 = 12.0;

/// Fresta entre toasts na pilha EXPANDIDA — `--toast-gap` (`--spacing(3)`).
const GAP: f32 = 12.0;

/// Quanto cada índice encolhe o toast — `--toast-scale: max(0, 1 - index*.1)`.
const SCALE_STEP: f32 = 0.1;

/// Altura do botão de ação — o `ButtonSize::Xs` do [`crate::button`] tem 24px. Está repetido aqui
/// (e travado num teste) porque a métrica é privada lá, e a altura natural do toast depende dela.
const ACTION_HEIGHT: f32 = 24.0;

/// Duração da transição de geometria — `transition: transform .5s`.
const TRANSITION: Duration = Duration::from_millis(500);

/// Duração do fade do conteúdo — `transition-opacity duration-250`.
const CONTENT_FADE: Duration = Duration::from_millis(250);

/// Volta completa do ícone de `loading` — `animate-spin` (1s linear, infinito).
const SPIN_PERIOD: Duration = Duration::from_millis(1000);

/// Quantos toasts ficam visíveis. Além disso a referência marca `data-limited` e apaga
/// (`opacity-0`).
///
/// ⚠️ **DEDUZIDO**: é o default do `limit` do `@base-ui/react/toast`, que a referência não
/// sobrescreve. Não foi lido do código do base-ui.
const DEFAULT_LIMIT: usize = 3;

/// Quanto tempo um toast fica na tela antes de sair sozinho.
///
/// ⚠️ **DEDUZIDO**: é o default do base-ui (5s), que a referência não sobrescreve.
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(5000);

// =================================================================================================
// Curva de animação
// =================================================================================================

/// A curva da referência — `cubic-bezier(.22,1,.36,1)`, um `ease-out` forte.
///
/// Implementada de verdade (e não aproximada): a bezier de CSS é paramétrica em `u`, então achar
/// `y` pra um `t` dado exige inverter `x(u) = t`. Vinte passos de bissecção dão ~1e-6 no domínio,
/// muito além do que 60fps mostra, e não têm o risco de divergência de um Newton perto de `u = 0`
/// (onde `x'(u)` desta curva é pequeno).
fn ease(t: f32) -> f32 {
    cubic_bezier(0.22, 1.0, 0.36, 1.0, t)
}

/// Uma `cubic-bezier(x1,y1,x2,y2)` de CSS avaliada em `t` ∈ `[0,1]`.
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
// Tipos públicos
// =================================================================================================

/// Onde o viewport de toasts se ancora na janela.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    /// O default da referência.
    #[default]
    BottomRight,
}

impl ToastPosition {
    /// Se a pilha cresce a partir da borda de BAIXO (o caso `bottom-*`).
    fn anchors_bottom(self) -> bool {
        matches!(
            self,
            ToastPosition::BottomLeft | ToastPosition::BottomCenter | ToastPosition::BottomRight
        )
    }
}

/// O tipo de um toast — decide o ícone e a cor dele. Os cinco do `TOAST_ICONS` da referência.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastType {
    Error,
    Info,
    Loading,
    Success,
    Warning,
}

impl ToastType {
    /// O caminho do ícone, na [`crate::assets::Assets`].
    ///
    /// A referência usa **lucide**; o crate embute **iconoir**, então cada um foi mapeado no
    /// equivalente mais próximo:
    ///
    /// | lucide             | iconoir            | observação                                   |
    /// |--------------------|--------------------|----------------------------------------------|
    /// | `CircleAlert`      | `warning-circle`   | círculo com `!` — equivalente direto          |
    /// | `Info`             | `info-circle`      | círculo com `i` — equivalente direto          |
    /// | `CircleCheck`      | `check-circle`     | círculo com check — equivalente direto        |
    /// | `TriangleAlert`    | `warning-triangle` | triângulo com `!` — equivalente direto        |
    /// | `LoaderCircle`     | `refresh`          | ⚠️ o iconoir **não tem spinner**; `refresh` é |
    /// |                    |                    | um arco de ~300° com ponta de flecha, que     |
    /// |                    |                    | girando lê como spinner — mas TEM a flecha    |
    pub fn icon(self) -> &'static str {
        match self {
            ToastType::Error => "iconoir/regular/warning-circle.svg",
            ToastType::Info => "iconoir/regular/info-circle.svg",
            ToastType::Loading => "iconoir/regular/refresh.svg",
            ToastType::Success => "iconoir/regular/check-circle.svg",
            ToastType::Warning => "iconoir/regular/warning-triangle.svg",
        }
    }

    /// A cor do ícone no tema corrente.
    fn color(self) -> Hsla {
        let p = toast();
        match self {
            ToastType::Error => p.destructive.hsla(),
            ToastType::Info => p.info.hsla(),
            ToastType::Success => p.success.hsla(),
            ToastType::Warning => p.warning.hsla(),
            // `loading` não tem classe de cor na referência: herda a cor do texto.
            ToastType::Loading => p.popover_fg.hsla(),
        }
    }
}

/// Identificador de um toast na fila. Devolvido por [`ToastManager::push`] e aceito por
/// [`ToastManager::dismiss`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

/// Evento emitido pelo [`ToastManager`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastEvent {
    /// Um toast entrou na fila.
    Pushed(ToastId),
    /// Um toast começou a sair — por tempo, por clique ou por [`ToastManager::dismiss`].
    Dismissed(ToastId),
    /// O botão de ação de um toast foi clicado. O callback de [`Toast::action`] JÁ rodou; este
    /// evento existe pra quem prefere reagir de fora.
    Action(ToastId),
}

/// O **descritor** de um toast: o que a app empurra pra fila.
///
/// Só o título é obrigatório — é o que a referência assume (`Toast.Title` sempre existe,
/// `Toast.Description` é opcional).
pub struct Toast {
    title: SharedString,
    description: Option<SharedString>,
    kind: Option<ToastType>,
    action: Option<SharedString>,
    #[allow(clippy::type_complexity)]
    on_action: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    /// `Some(None)` = nunca sai sozinho; `None` = usa o default do tipo.
    timeout: Option<Option<Duration>>,
}

impl Toast {
    /// Um toast com título.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            kind: None,
            action: None,
            on_action: None,
            timeout: None,
        }
    }

    /// Segunda linha, em `--muted-foreground`.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// O tipo (ícone + cor). Sem tipo, o toast não tem ícone — como na referência, onde
    /// `TOAST_ICONS[toast.type]` só resolve se `type` existir.
    pub fn kind(mut self, kind: ToastType) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Atalho pra `.kind(ToastType::Error)`.
    pub fn error(self) -> Self {
        self.kind(ToastType::Error)
    }

    /// Atalho pra `.kind(ToastType::Info)`.
    pub fn info(self) -> Self {
        self.kind(ToastType::Info)
    }

    /// Atalho pra `.kind(ToastType::Success)`.
    pub fn success(self) -> Self {
        self.kind(ToastType::Success)
    }

    /// Atalho pra `.kind(ToastType::Warning)`.
    pub fn warning(self) -> Self {
        self.kind(ToastType::Warning)
    }

    /// Atalho pra `.kind(ToastType::Loading)`. Um toast de `loading` **não sai sozinho** (ver
    /// [`Self::timeout`]) — quem o criou é que sabe quando a operação acabou.
    pub fn loading(self) -> Self {
        self.kind(ToastType::Loading)
    }

    /// O botão de ação, à direita. É o nosso [`crate::Button`] no [`crate::ButtonSize::Xs`], que é
    /// exatamente o `buttonVariants({ size: "xs" })` da referência.
    ///
    /// Clicar na ação roda o `handler` **e** dispensa o toast (é o comportamento do
    /// `Toast.Action` do base-ui).
    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.action = Some(label.into());
        self.on_action = Some(Rc::new(handler));
        self
    }

    /// Quanto tempo o toast fica na tela. O default é [`DEFAULT_TIMEOUT`] (5s), exceto no tipo
    /// `loading`, que nunca sai sozinho.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(Some(timeout));
        self
    }

    /// O toast **não** sai sozinho: só por clique ou [`ToastManager::dismiss`].
    pub fn persistent(mut self) -> Self {
        self.timeout = Some(None);
        self
    }

    /// O tempo de vida efetivo, resolvendo o default do tipo.
    fn effective_timeout(&self) -> Option<Duration> {
        match self.timeout {
            Some(t) => t,
            None if self.kind == Some(ToastType::Loading) => None,
            None => Some(DEFAULT_TIMEOUT),
        }
    }
}

// =================================================================================================
// Geometria calculada (a parte pura, e a que o `transform` da referência virou)
// =================================================================================================

/// Onde e como um toast é desenhado, num instante.
///
/// Tudo aqui é interpolável, e é isso que substitui o `transition: transform .5s` do CSS: a cada
/// mudança de alvo, o valor VISÍVEL vira o ponto de partida de um lerp de 0,5s (mesmo padrão do
/// deslize do [`crate::tabs`]).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Geom {
    /// Fração da largura do viewport, em `[0,1]` — o `scale()` da referência. `1` = largura cheia.
    scale: f32,
    /// Altura da superfície, em px.
    height: f32,
    /// Distância da borda ANCORADA do viewport (de baixo nas posições `bottom-*`, de cima nas
    /// `top-*`) até a borda ancorada do toast, em px. Negativo joga o toast pra fora.
    offset: f32,
    /// Opacidade da superfície inteira — `data-limited:opacity-0` e `data-ending-style:opacity-0`.
    opacity: f32,
    /// Opacidade do CONTEÚDO — `data-behind:opacity-0`.
    content_opacity: f32,
    /// Quanto de preto misturar no fundo, em `[0,1]` — o `color-mix` por índice de pilha.
    darken: f32,
}

impl Geom {
    /// O raio da superfície neste estado. No CSS o `scale()` escala o raio junto, e é só o raio
    /// dos cantos de CIMA de um toast de trás que aparece (o resto fica escondido) — então vale a
    /// pena reproduzir.
    fn radius(&self) -> f32 {
        RADIUS * self.scale
    }
}

/// Interpola duas geometrias, com `t` em `[0,1]`.
fn lerp_geom(a: Geom, b: Geom, t: f32) -> Geom {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: f32, y: f32| x + (y - x) * t;
    Geom {
        scale: mix(a.scale, b.scale),
        height: mix(a.height, b.height),
        offset: mix(a.offset, b.offset),
        opacity: mix(a.opacity, b.opacity),
        content_opacity: mix(a.content_opacity, b.content_opacity),
        darken: mix(a.darken, b.darken),
    }
}

/// A geometria de repouso do toast de índice `i` (0 = o da frente).
///
/// # De onde vem a álgebra
///
/// Colapsada, a referência põe todo toast com `height: var(--toast-frontmost-height)` — ou seja
/// **todos com a altura do da frente** — e aplica, numa pilha ancorada embaixo:
///
/// ```text
/// transform-origin: 50% calc(50% + 50%*min(i,1))     // centro em i=0, base-centro em i>=1
/// transform: translateY(calc(-(i*peek) - (shrink*H))) scale(s)
///   com s = max(0, 1 - i*0.1)  e  shrink = 1 - s
/// ```
///
/// Escalando em torno da base-centro, a base fica onde está e a altura vira `s·H`; o `translateY`
/// então sobe a caixa em `i·peek + shrink·H`. Logo:
///
/// - **altura visível** = `s·H`;
/// - **base** (distância acima da borda do viewport) = `i·peek + (1−s)·H`;
/// - **topo** = `s·H + i·peek + (1−s)·H` = `H + i·peek`.
///
/// Esse último resultado é o que dá sentido ao `peek`: o topo de cada toast fica exatamente 12px
/// acima do topo do anterior, independente da altura. É por isso que a pilha lê como um baralho.
///
/// Horizontalmente a origem é `50%`, então a caixa de largura `s·W` fica CENTRADA — daí o recuo
/// lateral `(1−s)/2` em [`ToastManager::render`].
///
/// Expandida (`data-expanded`), a referência troca a altura pela natural de cada um
/// (`h-(--toast-height)`) e o transform por `translateY(var(--toast-calc-offset-y))`, onde o
/// offset acumula as alturas dos toasts da frente mais `i·gap`. Sem escala nenhuma.
fn geom(i: usize, expanded: bool, heights: &[f32], limit: usize, darken_step: f32) -> Geom {
    let front = heights.first().copied().unwrap_or(0.0);
    let own = heights.get(i).copied().unwrap_or(front);

    let (scale, height, offset) = if expanded {
        let acumulado: f32 = heights.iter().take(i).sum();
        (1.0, own, acumulado + GAP * i as f32)
    } else {
        let s = (1.0 - SCALE_STEP * i as f32).max(0.0);
        (s, front * s, PEEK * i as f32 + (1.0 - s) * front)
    };

    Geom {
        scale,
        height,
        offset,
        // `data-limited:opacity-0` — além do limite, o toast continua no lugar mas invisível.
        opacity: if i >= limit { 0.0 } else { 1.0 },
        // `data-behind:opacity-0 data-expanded:opacity-100` — só o da frente mostra o conteúdo
        // enquanto a pilha está colapsada.
        content_opacity: if expanded || i == 0 { 1.0 } else { 0.0 },
        darken: if expanded {
            0.0 // `data-expanded:bg-popover`
        } else {
            darken_step * i as f32
        },
    }
}

/// A geometria de SAÍDA a partir de um estado visível: o toast desliza pra fora e desvanece.
///
/// É o `data-ending-style` da referência: `translateY(±(100% + inset))` (a própria altura mais o
/// recuo do viewport, o que o põe rente à borda da janela) com `opacity: 0`.
fn leaving_geom(from: Geom) -> Geom {
    Geom {
        offset: -(from.height + INSET),
        opacity: 0.0,
        ..from
    }
}

/// A geometria de ENTRADA: a mesma coisa, mas sem apagar — o `data-starting-style` da referência
/// só mexe no `translateY`, não na opacidade.
fn entering_geom(target: Geom) -> Geom {
    Geom {
        offset: -(target.height + INSET),
        ..target
    }
}

/// A altura natural ESTIMADA de um toast, usada no primeiro frame (antes de a medida chegar).
///
/// É exata quando nenhum texto quebra linha, que é o caso comum. Existe pra o toast não nascer com
/// altura zero e dar um pulo no frame seguinte — a medida real vem do `canvas` de
/// [`ToastManager::measure`] e corrige silenciosamente.
fn estimated_height(has_description: bool, has_action: bool) -> f32 {
    let texto = if has_description {
        LINE_HEIGHT * 2.0 + GAP_TEXT
    } else {
        LINE_HEIGHT
    };
    // O conteúdo é `items-center`, então a linha tem a altura do mais alto.
    let interno = if has_action {
        texto.max(ACTION_HEIGHT)
    } else {
        texto
    };
    interno + PAD_Y * 2.0 + 2.0 // + a borda de 1px de cada lado (a altura é border-box)
}

// =================================================================================================
// Estado interno de um toast vivo
// =================================================================================================

/// Um toast na fila, com o estado que só o tempo produz.
struct Live {
    id: ToastId,
    toast: Toast,
    /// Altura natural: começa no estimado e é substituída pela medida do `canvas`.
    natural: f32,
    /// Quanto falta pro auto-dismiss. `None` = nunca. Decrementa por frame, e **pausa** enquanto a
    /// pilha está expandida (é o que o base-ui faz: o ponteiro em cima segura o toast).
    remaining: Option<Duration>,
    /// Quando a saída começou. `Some` = está indo embora; some da fila em [`TRANSITION`].
    leaving: Option<Instant>,
    /// A geometria que ele está PERSEGUINDO.
    target: Geom,
    /// De onde a transição corrente partiu, e quando. `None` = está parado no alvo.
    anim: Option<(Geom, Instant)>,
    /// Quando ele nasceu — alimenta a fase do spinner de `loading`.
    born: Instant,
    /// Só em teste: a caixa que o layout de verdade deu ao conteúdo. Ver [`ToastManager::probe`].
    #[cfg(test)]
    probe: Option<Bounds<Pixels>>,
}

impl Live {
    /// A geometria VISÍVEL agora.
    fn visible(&self) -> Geom {
        match self.anim {
            Some((from, start)) => {
                let t = start.elapsed().as_secs_f32() / TRANSITION.as_secs_f32();
                // O conteúdo tem transição própria, mais curta (`duration-250`).
                let tc = start.elapsed().as_secs_f32() / CONTENT_FADE.as_secs_f32();
                let mut g = lerp_geom(from, self.target, ease(t));
                g.content_opacity = from.content_opacity
                    + (self.target.content_opacity - from.content_opacity) * tc.clamp(0.0, 1.0);
                g
            }
            None => self.target,
        }
    }

    /// Aponta pra um alvo novo, partindo de onde o olho está vendo o toast agora.
    ///
    /// A comparação é por épsilon de propósito: a altura vem de uma medida em ponto flutuante, e
    /// comparar por igualdade exata reiniciaria a transição todo frame por ruído de 1e-7.
    fn retarget(&mut self, want: Geom) {
        const EPS: f32 = 0.01;
        let perto = |a: f32, b: f32| (a - b).abs() < EPS;
        if perto(want.scale, self.target.scale)
            && perto(want.height, self.target.height)
            && perto(want.offset, self.target.offset)
            && perto(want.opacity, self.target.opacity)
            && perto(want.content_opacity, self.target.content_opacity)
            && perto(want.darken, self.target.darken)
        {
            return;
        }
        self.anim = Some((self.visible(), Instant::now()));
        self.target = want;
    }

    /// Anda `dt` no relógio do auto-dismiss. Devolve `true` quando o tempo acabou AGORA.
    ///
    /// Ao vencer, o relógio é **desarmado** (`remaining = None`). Sem isso ele continuaria zerado e
    /// devolveria `true` a cada frame: o `dismiss` é idempotente e não quebraria, mas o render
    /// pediria um `Dismissed` por frame e nunca pararia de pedir animação.
    fn tick(&mut self, dt: Duration, paused: bool) -> bool {
        if self.leaving.is_some() || paused {
            return false;
        }
        let Some(rem) = self.remaining else {
            return false;
        };
        let novo = rem.saturating_sub(dt);
        if novo.is_zero() {
            self.remaining = None;
            true
        } else {
            self.remaining = Some(novo);
            false
        }
    }

    /// Se ainda há transição de geometria em curso.
    fn animating(&self) -> bool {
        self.anim.is_some_and(|(_, start)| start.elapsed() < TRANSITION)
    }
}

// =================================================================================================
// O manager
// =================================================================================================

/// A fila de toasts — e o dono do viewport. Ver o doc do módulo.
pub struct ToastManager {
    /// Índice 0 = o da FRENTE = o mais recente (é a ordem do base-ui: novo entra na cabeça).
    toasts: Vec<Live>,
    position: ToastPosition,
    limit: usize,
    /// A pilha está aberta (ponteiro em cima).
    expanded: bool,
    next_id: u64,
    /// Id estável pra compor os `ElementId` dos filhos.
    entity_id: u64,
    /// Quando o último frame rodou — o `dt` do relógio de auto-dismiss.
    last_frame: Instant,
}

impl ToastManager {
    /// Uma fila vazia, em [`ToastPosition::BottomRight`] (o default da referência).
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            toasts: Vec::new(),
            position: ToastPosition::default(),
            limit: DEFAULT_LIMIT,
            expanded: false,
            next_id: 0,
            entity_id: cx.entity_id().as_u64(),
            last_frame: Instant::now(),
        }
    }

    /// Onde o viewport se ancora (builder, pra usar dentro do `cx.new(..)`).
    pub fn with_position(mut self, position: ToastPosition) -> Self {
        self.position = position;
        self
    }

    /// Quantos toasts ficam visíveis ao mesmo tempo (builder). Os demais continuam na fila,
    /// apagados, e reaparecem quando os da frente saírem — é o `data-limited` da referência.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    /// Troca a ancoragem em runtime.
    pub fn set_position(&mut self, position: ToastPosition, cx: &mut Context<Self>) {
        if self.position == position {
            return;
        }
        self.position = position;
        cx.notify();
    }

    /// A ancoragem corrente.
    pub fn position(&self) -> ToastPosition {
        self.position
    }

    /// Empurra um toast pra frente da fila e devolve o id dele.
    pub fn push(&mut self, toast: Toast, cx: &mut Context<Self>) -> ToastId {
        let id = ToastId(self.next_id);
        self.next_id += 1;

        let natural = estimated_height(toast.description.is_some(), toast.action.is_some());
        let remaining = toast.effective_timeout();
        // O alvo definitivo depende do índice de todo mundo e é recalculado no render; aqui basta
        // um alvo plausível pra a entrada ter de onde partir.
        let alvo = geom(0, false, &[natural], self.limit, toast_darken_step());
        self.toasts.insert(
            0,
            Live {
                id,
                toast,
                natural,
                remaining,
                leaving: None,
                target: alvo,
                anim: Some((entering_geom(alvo), Instant::now())),
                born: Instant::now(),
                #[cfg(test)]
                probe: None,
            },
        );
        cx.emit(ToastEvent::Pushed(id));
        cx.notify();
        id
    }

    /// Começa a saída de um toast. Idempotente: chamar duas vezes não reinicia a animação nem
    /// emite dois eventos.
    pub fn dismiss(&mut self, id: ToastId, cx: &mut Context<Self>) {
        if self.begin_leave(id) {
            cx.emit(ToastEvent::Dismissed(id));
            cx.notify();
        }
    }

    /// Dispensa todos.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<ToastId> = self.toasts.iter().map(|t| t.id).collect();
        for id in ids {
            if self.begin_leave(id) {
                cx.emit(ToastEvent::Dismissed(id));
            }
        }
        cx.notify();
    }

    /// Quantos toasts há na fila (contando os que estão saindo).
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    /// Se a fila está vazia.
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Marca a saída, sem tocar em `cx`. Devolve `false` se já estava saindo (ou não existe) — é o
    /// que torna [`Self::dismiss`] idempotente e permite chamar isto de dentro do `render`, onde
    /// `cx.notify()` seria um laço.
    fn begin_leave(&mut self, id: ToastId) -> bool {
        let Some(t) = self.toasts.iter_mut().find(|t| t.id == id) else {
            return false;
        };
        if t.leaving.is_some() {
            return false;
        }
        t.leaving = Some(Instant::now());
        // A saída parte de onde o toast está agora.
        let visivel = t.visible();
        t.target = leaving_geom(visivel);
        t.anim = Some((visivel, Instant::now()));
        true
    }

    /// Grava a altura natural medida. `content_height` é a altura do bloco de conteúdo; a
    /// superfície é 2px mais alta (a borda de 1px de cada lado, porque a altura é border-box).
    fn record_height(&mut self, id: ToastId, content_height: f32) {
        if content_height <= 0.0 {
            return;
        }
        if let Some(t) = self.toasts.iter_mut().find(|t| t.id == id) {
            t.natural = content_height + 2.0;
        }
    }

    /// A caixa do conteúdo, como o layout de verdade a posicionou.
    ///
    /// ⚠️ Só existe em teste. É a ÚNICA via de verificar, com layout real, que a geometria calculada
    /// aterrissa onde a álgebra de [`geom`] diz — as hipóteses de flex/absoluto (padding + alinhamento
    /// no lugar do `fixed`, largura fracionária no lugar do `scale()`, filho absoluto de um pai de
    /// altura zero) quebrariam em silêncio para um teste puro. A caixa do conteúdo é a da superfície
    /// recuada 1px de cada lado, então dá pras duas.
    #[cfg(test)]
    fn probe(&self, id: ToastId) -> Option<Bounds<Pixels>> {
        self.toasts.iter().find(|t| t.id == id)?.probe
    }

    /// Zera as transições em curso, pondo todo toast no seu alvo. Só em teste: os testes de janela
    /// verificam a geometria de REPOUSO, e o progresso das transições vem de [`Instant`] (relógio de
    /// parede), que o executor de teste do GPUI não adianta.
    #[cfg(test)]
    fn settle(&mut self, cx: &mut Context<Self>) {
        for t in &mut self.toasts {
            t.anim = None;
        }
        cx.notify();
    }

    /// Um `canvas` overlay que só MEDE a altura natural do conteúdo e não pinta nada.
    ///
    /// Mesmo truque do [`crate::tabs`]: `inset:0` explícito, senão um absoluto de insets `auto`
    /// cairia na posição estática (dentro do padding) e a medida sairia menor. E, como lá, ele
    /// **não** chama `cx.notify()` — a correção entra no frame seguinte, que já vem de graça
    /// enquanto houver animação.
    fn measure(&self, cx: &mut Context<Self>, id: ToastId) -> impl IntoElement {
        let view = cx.entity();
        canvas(
            move |bounds: Bounds<Pixels>, _window, cx| {
                view.update(cx, |this, _| {
                    this.record_height(id, f32::from(bounds.size.height));
                    #[cfg(test)]
                    if let Some(t) = this.toasts.iter_mut().find(|t| t.id == id) {
                        t.probe = Some(bounds);
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }
}

/// O `darken_step` da paleta corrente — atalho, porque [`ToastManager::push`] precisa dele antes
/// de ter uma paleta em mão.
fn toast_darken_step() -> f32 {
    toast().darken_step
}

impl EventEmitter<ToastEvent> for ToastManager {}

// =================================================================================================
// Pintura
// =================================================================================================

/// O bisel de 1px sobreposto.
///
/// ⚠️ As três coisas que já custaram bug visível nesta base (ver [`crate::card::Card::bevel`] e
/// [`crate::frame`]):
///
/// 1. **É borda, não sombra.** O [`gpui::Window::paint_shadows`] não recorta a sombra pra fora do
///    elemento que a projeta (diferente do CSS), então o `before:box-shadow` da referência viraria
///    uma lavagem de cor sobre o toast inteiro.
/// 2. **Cobre a BORDER box** (`inset: -1px`), porque no CSS a sombra do pseudo-elemento sai 1px
///    pra fora e cai SOBRE a borda, clareando-a.
/// 3. **O raio é o da SUPERFÍCIE**, não `raio − 1`. O `calc(var(--radius-lg) - 1px)` da referência
///    é o raio do pseudo-elemento na padding box; o nosso está uma caixa pra fora.
fn bevel_overlay(radius: f32) -> Div {
    let p = toast();
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
/// que é o caso em que a técnica do GPUI funciona — o `blur_radius` recebe o raio do CSS direto,
/// mesma convenção do [`crate::card`].
fn surface_shadows() -> Vec<gpui::BoxShadow> {
    let cor = toast().shadow.hsla();
    vec![
        gpui::BoxShadow {
            color: cor,
            offset: point(px(0.0), px(10.0)),
            blur_radius: px(15.0),
            spread_radius: px(-3.0),
        },
        gpui::BoxShadow {
            color: cor,
            offset: point(px(0.0), px(4.0)),
            blur_radius: px(6.0),
            spread_radius: px(-4.0),
        },
    ]
}

/// O ícone de um toast: 16px de largura, centrado numa caixa de uma linha de altura.
fn icon(kind: ToastType, spin: f32) -> impl IntoElement {
    let mut svg = gpui::svg()
        .path(kind.icon())
        .size(px(ICON_SIZE))
        .flex_none()
        .text_color(kind.color());
    if kind == ToastType::Loading {
        // `animate-spin` — 1s linear, infinito. Aqui o `TransformationMatrix` do GPUI de fato
        // serve: ele funciona em `svg`/imagem (só não em `div`).
        svg = svg
            .with_transformation(Transformation::rotate(radians(
                spin * std::f32::consts::TAU,
            )))
            .opacity(LOADING_OPACITY);
    }
    div()
        .flex_none()
        .h(px(LINE_HEIGHT))
        .flex()
        .items_center()
        .child(svg)
}

/// O que o render precisa saber de um toast, já resolvido — separado do `&mut self` pra o
/// empréstimo do [`ToastManager`] terminar antes de montar os elementos.
struct Plan {
    id: ToastId,
    geom: Geom,
    kind: Option<ToastType>,
    title: SharedString,
    description: Option<SharedString>,
    action: Option<SharedString>,
    #[allow(clippy::type_complexity)]
    on_action: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    spin: f32,
}

impl Render for ToastManager {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = toast();
        let agora = Instant::now();
        let dt = agora.saturating_duration_since(self.last_frame);
        self.last_frame = agora;

        // 1. Quem terminou a animação de saída deixa a fila.
        self.toasts
            .retain(|t| t.leaving.is_none_or(|s| s.elapsed() < TRANSITION));
        if self.toasts.is_empty() {
            // A região de hover deixa de existir junto com a pilha, então o `on_hover(false)` que
            // colapsaria nunca chega. Sem este reset, o PRÓXIMO toast nasceria numa pilha aberta.
            self.expanded = false;
        }

        // 2. Relógio do auto-dismiss. Pausa com a pilha aberta.
        let pausado = self.expanded;
        let venceram: Vec<ToastId> = self
            .toasts
            .iter_mut()
            .filter_map(|t| t.tick(dt, pausado).then_some(t.id))
            .collect();
        for id in venceram {
            // `begin_leave` em vez de `dismiss`: aqui já estamos dentro do render, e o
            // `cx.notify()` do `dismiss` agendaria outro frame à toa (o
            // `request_animation_frame` do fim já cobre).
            if self.begin_leave(id) {
                cx.emit(ToastEvent::Dismissed(id));
            }
        }

        // 3. Índices e alturas — só dos que NÃO estão saindo. Um toast em saída perde o lugar na
        //    pilha imediatamente, e os de trás sobem (é o que a referência faz reindexando
        //    `--toast-index`).
        let vivos: Vec<usize> = (0..self.toasts.len())
            .filter(|&i| self.toasts[i].leaving.is_none())
            .collect();
        let alturas: Vec<f32> = vivos.iter().map(|&i| self.toasts[i].natural).collect();
        let (expandida, limite, passo) = (self.expanded, self.limit, p.darken_step);

        // 4. Cada toast persegue o seu alvo.
        for (rank, &i) in vivos.iter().enumerate() {
            let want = geom(rank, expandida, &alturas, limite, passo);
            self.toasts[i].retarget(want);
        }
        // Um toast em saída já tem o alvo definitivo (posto em `begin_leave`); mas a altura dele
        // pode ter mudado, e o alvo continua válido — nada a fazer.

        // 5. Descarta transições concluídas (senão `visible()` continua interpolando à toa).
        for t in &mut self.toasts {
            if !t.animating() {
                t.anim = None;
            }
        }

        // 6. O plano de pintura.
        let planos: Vec<Plan> = self
            .toasts
            .iter()
            .map(|t| Plan {
                id: t.id,
                geom: t.visible(),
                kind: t.toast.kind,
                title: t.toast.title.clone(),
                description: t.toast.description.clone(),
                action: t.toast.action.clone(),
                on_action: t.toast.on_action.clone(),
                spin: (t.born.elapsed().as_secs_f32() / SPIN_PERIOD.as_secs_f32()).fract(),
            })
            .collect();

        let animando = self.toasts.iter().any(|t| t.animating())
            || planos.iter().any(|pl| pl.kind == Some(ToastType::Loading))
            || (!pausado
                && self
                    .toasts
                    .iter()
                    .any(|t| t.leaving.is_none() && t.remaining.is_some()));

        // 7. A raiz. É um flex que ocupa o overlay inteiro e usa `padding` + alinhamento pra
        //    posicionar o viewport — o equivalente geométrico do `fixed` + `inset` + `left-1/2
        //    -translate-x-1/2` da referência, e que dá `min(100% - inset*2, 360px)` de graça:
        //    `w_full` resolve contra a caixa de conteúdo (já descontado o padding) e `max_w` corta
        //    em 360.
        let de_baixo = self.position.anchors_bottom();
        let mut raiz = div().size_full().flex().p(px(INSET));
        raiz = if de_baixo {
            raiz.items_end()
        } else {
            raiz.items_start()
        };
        raiz = match self.position {
            ToastPosition::TopLeft | ToastPosition::BottomLeft => raiz.justify_start(),
            ToastPosition::TopCenter | ToastPosition::BottomCenter => raiz.justify_center(),
            ToastPosition::TopRight | ToastPosition::BottomRight => raiz.justify_end(),
        };

        if planos.is_empty() {
            return raiz;
        }

        // O viewport: altura ZERO (todos os filhos são absolutos), largura limitada. É a caixa em
        // relação à qual toda a geometria dos toasts é resolvida.
        let mut viewport = div()
            .relative()
            .w_full()
            .max_w(px(MAX_WIDTH))
            .flex_none()
            .text_size(px(TEXT_SIZE))
            .line_height(px(LINE_HEIGHT));

        // 8. A região de hover.
        //
        // No CSS quem detecta o hover é o viewport, e os `after:` de cada toast fazem a ponte de
        // 12px+1 sobre as frestas pra o ponteiro não "cair" entre dois toasts e colapsar a pilha.
        // Aqui o viewport tem altura zero (não dá hitbox), então a ponte é uma região ÚNICA que
        // cobre a pilha inteira — frestas incluídas. Ela é o primeiro filho, ou seja pinta ATRÁS:
        // os hitboxes dos toasts continuam ganhando o clique, e o hover registra nos dois (o
        // `hit_test` do GPUI acumula todos os hitboxes sob o cursor, não só o de cima).
        let alcance = planos
            .iter()
            .map(|pl| pl.geom.offset + pl.geom.height)
            .fold(0.0f32, f32::max);
        let mut regiao = div()
            .id(("toast-hover", self.entity_id))
            .absolute()
            .left_0()
            .w_full()
            .h(px(alcance.max(0.0)))
            .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                if this.expanded != *hovered {
                    this.expanded = *hovered;
                    cx.notify();
                }
            }));
        regiao = if de_baixo {
            regiao.bottom_0()
        } else {
            regiao.top_0()
        };
        viewport = viewport.child(regiao);

        // 9. Os toasts, do fundo pra frente: o GPUI pinta na ordem dos filhos, então o índice 0
        //    (que no CSS tem o maior `z-index`) tem que ser o ÚLTIMO.
        for plano in planos.into_iter().rev() {
            viewport = viewport.child(self.render_toast(plano, de_baixo, cx));
        }

        raiz = raiz.child(viewport);

        // Enquanto houver transição, spinner ou relógio andando, pede o próximo frame. É o motor de
        // TODAS as animações daqui — não há elemento de animação, o progresso vem do tempo
        // decorrido (mesmo padrão do [`crate::button`] e do [`crate::tabs`]).
        if animando {
            window.request_animation_frame();
        }

        raiz
    }
}

impl ToastManager {
    /// Um toast, na geometria que o frame calculou.
    fn render_toast(&self, plan: Plan, de_baixo: bool, cx: &mut Context<Self>) -> AnyElement {
        let p = toast();
        let g = plan.geom;
        let id = plan.id;
        let raio = g.radius();

        // --- Conteúdo -------------------------------------------------------------------------
        let mut bloco = div().flex().gap(px(GAP_ICON));
        if let Some(kind) = plan.kind {
            bloco = bloco.child(icon(kind, plan.spin));
        }
        let mut textos = div()
            .flex()
            .flex_col()
            .gap(px(GAP_TEXT))
            .child(
                div()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(plan.title),
            );
        if let Some(d) = plan.description {
            textos = textos.child(div().text_color(p.muted_fg.hsla()).child(d));
        }
        bloco = bloco.child(textos);

        // `pointer-events-auto`, mas só onde a referência o dá: o conteúdo de um toast que está
        // ATRÁS e colapsado é `data-behind:not-data-expanded:pointer-events-none`, e um além do
        // limite está em `opacity-0`. Sem esta guarda, o filete de 12px que aparece de um toast de
        // trás (ou um toast INVISÍVEL do outro lado do limite) responderia a clique — e dispensaria
        // algo que o usuário não está vendo.
        let interativo = g.opacity > 0.01 && g.content_opacity > 0.01;

        let entidade = cx.entity();
        let mut conteudo = div()
            .id(("toast", id.0))
            .relative()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(GAP_ACTION))
            .overflow_hidden()
            .px(px(PAD_X))
            .py(px(PAD_Y))
            .opacity(g.content_opacity)
            .child(bloco);

        if interativo {
            conteudo = conteudo.cursor(CursorStyle::PointingHand);
        }

        if let Some(label) = plan.action {
            // O `buttonVariants({ size: "xs" })` da referência, que é a variante DEFAULT do botão
            // no tamanho `Xs`.
            let mut botao = crate::Button::new(("toast-action", id.0), label)
                .size(crate::ButtonSize::Xs);
            // Sem `on_click` quando o toast não está interativo — e NÃO `.disabled()`, que mudaria a
            // aparência do botão (esmaece pra 64%) num toast que está só atrás, não desabilitado.
            if interativo {
                let acao = plan.on_action.clone();
                let manager = entidade.clone();
                botao = botao.on_click(move |_event, window, cx| {
                    if let Some(h) = acao.as_ref() {
                        h(window, cx);
                    }
                    manager.update(cx, |m, cx| {
                        cx.emit(ToastEvent::Action(id));
                        m.dismiss(id, cx);
                    });
                });
            }
            conteudo = conteudo.child(botao);
        }

        // Clicar dispensa.
        //
        // ⚠️ **DIVERGÊNCIA da referência**, consciente: o `.tsx` não tem `Toast.Close` nem
        // qualquer afordância de fechar — lá se dispensa por SWIPE (não implementado aqui). Um X
        // visível mudaria o pixel; o clique não muda nada e mantém o toast dispensável.
        if interativo {
            let manager = entidade.clone();
            conteudo = conteudo.on_click(move |_event, _window, cx| {
                manager.update(cx, |m, cx| m.dismiss(id, cx));
            });
        }

        // A medida da altura natural vive DENTRO do conteúdo: é o conteúdo que tem altura
        // natural (a superfície tem a altura imposta pela pilha).
        conteudo = conteudo.child(self.measure(cx, id));

        // --- Superfície -----------------------------------------------------------------------
        //
        // A escala do CSS virou largura fracionária + recuo lateral que centraliza: no `scale()` da
        // referência a origem horizontal é `50%`, então um toast a 0,9 fica 10% mais estreito e
        // recuado 5% de cada lado.
        let mut superficie = div()
            .absolute()
            .w(gpui::relative(g.scale))
            .left(gpui::relative((1.0 - g.scale) / 2.0))
            .h(px(g.height))
            .rounded(px(raio))
            .border_1()
            .border_color(p.border.hsla())
            .bg(lerp(p.popover.hsla(), gpui::black(), g.darken))
            .text_color(p.popover_fg.hsla())
            .shadow(surface_shadows())
            .child(conteudo)
            // O bisel por último: é absoluto, então a ordem só decide quem pinta em cima. E ele
            // vive na SUPERFÍCIE, não no conteúdo — o conteúdo tem `overflow_hidden` (fiel à
            // referência) e recortaria um filho em `inset:-1px`.
            .child(bevel_overlay(raio));

        superficie = if de_baixo {
            superficie.bottom(px(g.offset))
        } else {
            superficie.top(px(g.offset))
        };
        if g.opacity < 1.0 {
            superficie = superficie.opacity(g.opacity);
        }
        superficie.into_any_element()
    }
}

// =================================================================================================
// O elemento montável
// =================================================================================================

/// O overlay que carrega o viewport de toasts. **Monte-o como último filho de uma raiz
/// `relative()`** — ver o exemplo no doc do módulo.
///
/// Existe porque o GPUI não tem portal nem `position: fixed`: o `Toast.Portal` da referência se
/// planta no `body` sozinho, e aqui quem escolhe o lugar é a app.
///
/// Ele **não** intercepta o mouse: é um `div` sem interatividade, e no GPUI só elementos com
/// listeners (ou `occlude()`) criam hitbox. Quem responde a ponteiro são os toasts — o
/// `pointer-events-auto` da referência, que também está só no conteúdo.
pub fn toast_layer(manager: &gpui::Entity<ToastManager>) -> impl IntoElement {
    ToastLayer {
        manager: manager.clone(),
    }
}

/// O elemento que [`toast_layer`] devolve.
#[derive(IntoElement)]
struct ToastLayer {
    manager: gpui::Entity<ToastManager>,
}

impl RenderOnce for ToastLayer {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(self.manager)
    }
}

// =================================================================================================
// Testes
// =================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// As duas paletas, com nome pras mensagens de falha.
    const PALETAS: [(&str, &ToastPalette); 2] = [("claro", &TOAST_LIGHT), ("escuro", &TOAST_DARK)];

    /// Os cinco tipos, pros testes varrerem sem esquecer nenhum.
    const TIPOS: [ToastType; 5] = [
        ToastType::Error,
        ToastType::Info,
        ToastType::Loading,
        ToastType::Success,
        ToastType::Warning,
    ];

    /// Um manager sem `Context` — o estado e a fila são lógica pura, e é isso que estes testes
    /// exercitam (mesmo caminho dos testes do [`crate::switch`]).
    fn manager() -> ToastManager {
        ToastManager {
            toasts: Vec::new(),
            position: ToastPosition::default(),
            limit: DEFAULT_LIMIT,
            expanded: false,
            next_id: 0,
            entity_id: 0,
            last_frame: Instant::now(),
        }
    }

    /// Insere um toast direto na fila, sem `Context`.
    fn push_raw(m: &mut ToastManager, toast: Toast) -> ToastId {
        let id = ToastId(m.next_id);
        m.next_id += 1;
        let natural = estimated_height(toast.description.is_some(), toast.action.is_some());
        let remaining = toast.effective_timeout();
        let alvo = geom(0, false, &[natural], m.limit, 0.01);
        m.toasts.insert(
            0,
            Live {
                id,
                toast,
                natural,
                remaining,
                leaving: None,
                target: alvo,
                anim: None,
                born: Instant::now(),
                probe: None,
            },
        );
        id
    }

    // --- Paleta -----------------------------------------------------------------------------

    /// **Os três semânticos vêm do `globals.css` do coss, e no escuro são IGUAIS ao claro.**
    ///
    /// Este teste existe porque a primeira versão errou quatro dos seis valores, por deduzir em vez
    /// de ler:
    ///
    /// - supôs `green-500` para `--success`, mas o coss usa **emerald**-500 (bem mais azulado);
    /// - e generalizou pros três a regra `mix(cor 90%, white)` que vale **só** pro `--destructive`
    ///   no tema escuro.
    ///
    /// A fonte é `packages/ui/src/styles/globals.css` no repositório do coss. Se alguém "consertar"
    /// o escuro clareando os três "por coerência com o destructive", isto cai.
    #[test]
    fn semanticos_saem_do_globals_do_coss() {
        assert_eq!(TOAST_LIGHT.info, Rgba8(0x3b82f6ff), "blue-500");
        assert_eq!(TOAST_LIGHT.success, Rgba8(0x10b981ff), "emerald-500, NÃO green-500");
        assert_eq!(TOAST_LIGHT.warning, Rgba8(0xf59e0bff), "amber-500");

        assert_eq!(TOAST_DARK.info, TOAST_LIGHT.info);
        assert_eq!(TOAST_DARK.success, TOAST_LIGHT.success);
        assert_eq!(TOAST_DARK.warning, TOAST_LIGHT.warning);

        // O contraste: o `destructive` é o ÚNICO que muda entre os temas.
        assert_ne!(
            TOAST_DARK.destructive, TOAST_LIGHT.destructive,
            "só o destructive sofre o color-mix no escuro"
        );
    }

    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, porque a 6% o filete é
    /// imperceptível no nosso fundo. É a MESMA decisão já vigente no `input`, `card`, `button` e
    /// `frame`; este teste existe pra o desvio ser uma decisão registrada e não uma deriva.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = TOAST_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%); veio {claro}"
        );

        let escuro = TOAST_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");

        // Ligados em `let` pra o clippy não tratar as asserções como constantes: o valor é de
        // paleta, e o que interessa é o SENTIDO — claro desce, escuro sobe.
        let (dir_claro, dir_escuro) = (TOAST_LIGHT.bevel_dir, TOAST_DARK.bevel_dir);
        assert!(dir_claro > 0.0, "no claro o filete DESCE");
        assert!(dir_escuro < 0.0, "no escuro o filete SOBE");
    }

    /// **A convenção de cor da paleta, decodificada de verdade.**
    ///
    /// Todo valor é `0xRRGGBBAA`; um valor de 6 dígitos esquecido aqui vira uma cor completamente
    /// diferente **sem erro de compilação** — `rgba(0xffffff)` é lido como `0x00FFFFFF`, ou seja
    /// ciano. Já aconteceu três vezes nesta base. Então o teste decodifica e afirma o que a cor
    /// DEVE ser perceptualmente.
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
            // Borda e bisel são TRANSLÚCIDOS: é isso que os faz funcionar sobre qualquer fundo.
            for (nome, c) in [("border", p.border), ("bevel", p.bevel), ("shadow", p.shadow)] {
                assert!(c.alpha() < 1.0, "{tema}: {nome} tem que ser translúcido");
            }
            // Os semânticos são OPACOS e COLORIDOS, cada um no matiz certo.
            let d: gpui::Rgba = p.destructive.hsla().into();
            assert!(d.r > d.g && d.r > d.b, "{tema}: destructive é vermelho");
            let s: gpui::Rgba = p.success.hsla().into();
            assert!(s.g > s.r && s.g > s.b, "{tema}: success é verde");
            let w: gpui::Rgba = p.warning.hsla().into();
            assert!(w.r > w.b && w.g > w.b, "{tema}: warning é âmbar");
            let i: gpui::Rgba = p.info.hsla().into();
            assert!(i.b > i.r && i.b > i.g, "{tema}: info é azul");
        }

        // O tema claro tem fundo branco; o escuro, quase preto.
        let claro: gpui::Rgba = TOAST_LIGHT.popover.hsla().into();
        let escuro: gpui::Rgba = TOAST_DARK.popover.hsla().into();
        assert_eq!((claro.r, claro.g, claro.b), (1.0, 1.0, 1.0));
        assert!(escuro.r < 0.2);

        // O escurecimento por índice é MUITO mais forte no escuro (1% vs 6%) — se alguém igualar os
        // dois, a pilha do tema claro fica cinza.
        let (passo_claro, passo_escuro) = (TOAST_LIGHT.darken_step, TOAST_DARK.darken_step);
        assert!(passo_escuro > passo_claro * 3.0);
    }

    /// O fundo escurece MONOTONICAMENTE com o índice, e o `lerp` vai pro preto (não pro branco).
    #[test]
    fn fundo_escurece_por_indice() {
        for (tema, p) in PALETAS {
            let base: gpui::Rgba = p.popover.hsla().into();
            let mut anterior = base.r + base.g + base.b;
            for i in 1..4 {
                let c: gpui::Rgba =
                    lerp(p.popover.hsla(), gpui::black(), p.darken_step * i as f32).into();
                let soma = c.r + c.g + c.b;
                assert!(soma < anterior, "{tema}: índice {i} tem que ser mais escuro");
                assert_eq!(c.a, 1.0, "{tema}: o fundo continua OPACO");
                anterior = soma;
            }
        }
    }

    // --- Ícones -----------------------------------------------------------------------------

    /// Todo tipo mapeia num ícone que EXISTE no iconoir embutido. Um caminho errado não dá erro de
    /// compilação — o ícone só some silenciosamente da UI.
    #[test]
    fn todo_tipo_tem_icone_existente_e_unico() {
        let mut vistos = Vec::new();
        for t in TIPOS {
            let path = t.icon();
            assert!(crate::iconoir::has(path), "{t:?}: {path} não existe");
            assert!(!vistos.contains(&path), "{t:?}: {path} repetido");
            vistos.push(path);
        }
        assert_eq!(vistos.len(), 5, "os cinco tipos do TOAST_ICONS");
    }

    /// Cada tipo tem a cor que a referência manda, e `loading` herda a do texto (a referência não
    /// lhe dá classe de cor, só `opacity-80`).
    #[test]
    fn cor_do_icone_por_tipo() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let p = toast();
            assert_eq!(ToastType::Error.color(), p.destructive.hsla());
            assert_eq!(ToastType::Info.color(), p.info.hsla());
            assert_eq!(ToastType::Success.color(), p.success.hsla());
            assert_eq!(ToastType::Warning.color(), p.warning.hsla());
            assert_eq!(ToastType::Loading.color(), p.popover_fg.hsla());
        }
        theme::set_theme(theme::ThemeMode::Dark);
    }

    // --- Curva ------------------------------------------------------------------------------

    /// A bezier da referência: passa pelas pontas, é monótona e é um `ease-out` FORTE — a metade do
    /// percurso é vencida em bem menos de metade do tempo. Se alguém trocar por uma linear (ou
    /// inverter os pontos de controle), isto pega.
    #[test]
    fn a_bezier_e_um_ease_out_forte() {
        assert!(ease(0.0).abs() < 1e-4);
        assert!((ease(1.0) - 1.0).abs() < 1e-4);
        let mut anterior = -1.0;
        for k in 0..=100 {
            let y = ease(k as f32 / 100.0);
            assert!(y >= anterior - 1e-5, "monótona em t={k}");
            anterior = y;
        }
        assert!(
            ease(0.25) > 0.6,
            "a 25% do tempo já passou de 60% do caminho; veio {}",
            ease(0.25)
        );
        assert!(ease(0.5) > 0.85, "a 50% já está quase lá");
        // Aparada nas duas pontas: `t` deriva de tempo decorrido e pode estourar.
        assert_eq!(ease(-1.0), ease(0.0));
        assert_eq!(ease(5.0), ease(1.0));
    }

    // --- Geometria colapsada ----------------------------------------------------------------

    /// **O invariante do `peek`.** O topo de cada toast colapsado fica exatamente 12px acima do
    /// topo do anterior, qualquer que seja a altura — é o que faz a pilha ler como um baralho, e é
    /// o resultado que a álgebra do `transform-origin` da referência produz.
    #[test]
    fn topo_de_cada_toast_colapsado_sobe_um_peek() {
        let alturas = [68.0, 90.0, 46.0, 120.0];
        for i in 0..4 {
            let g = geom(i, false, &alturas, 4, 0.01);
            let topo = g.offset + g.height;
            assert!(
                (topo - (alturas[0] + PEEK * i as f32)).abs() < 1e-4,
                "índice {i}: topo veio {topo}, esperado {}",
                alturas[0] + PEEK * i as f32
            );
        }
    }

    /// Colapsada, TODOS têm a altura do da frente vezes a escala (`--toast-frontmost-height`), e a
    /// escala cai 10% por índice, com piso em 0.
    #[test]
    fn escala_e_altura_colapsadas() {
        let alturas = [80.0, 200.0, 30.0];
        assert_eq!(geom(0, false, &alturas, 3, 0.0).scale, 1.0);
        assert!((geom(1, false, &alturas, 3, 0.0).scale - 0.9).abs() < 1e-6);
        assert!((geom(2, false, &alturas, 3, 0.0).scale - 0.8).abs() < 1e-6);
        // A altura NÃO é a própria: é a do da frente, escalada.
        assert!((geom(1, false, &alturas, 3, 0.0).height - 80.0 * 0.9).abs() < 1e-4);
        assert!((geom(2, false, &alturas, 3, 0.0).height - 80.0 * 0.8).abs() < 1e-4);
        // `max(0, ...)`: uma pilha de 12 não produz escala negativa (que inverteria o toast).
        let mut longas = vec![50.0; 20];
        longas[0] = 50.0;
        for i in 10..20 {
            assert_eq!(geom(i, false, &longas, 3, 0.0).scale, 0.0, "índice {i}");
        }
    }

    /// O raio acompanha a escala — no CSS o `scale()` escala o raio junto, e é o canto de cima de
    /// um toast de trás que aparece na pilha.
    #[test]
    fn raio_acompanha_a_escala() {
        let alturas = [68.0, 68.0];
        assert_eq!(geom(0, false, &alturas, 2, 0.0).radius(), RADIUS);
        assert!((geom(1, false, &alturas, 2, 0.0).radius() - 9.0).abs() < 1e-5);
    }

    /// Só o da frente mostra o conteúdo quando colapsada (`data-behind:opacity-0`); expandida,
    /// todos mostram (`data-expanded:opacity-100`).
    #[test]
    fn conteudo_visivel_so_no_da_frente_quando_colapsada() {
        let alturas = [68.0, 68.0, 68.0];
        assert_eq!(geom(0, false, &alturas, 3, 0.0).content_opacity, 1.0);
        assert_eq!(geom(1, false, &alturas, 3, 0.0).content_opacity, 0.0);
        assert_eq!(geom(2, false, &alturas, 3, 0.0).content_opacity, 0.0);
        for i in 0..3 {
            assert_eq!(geom(i, true, &alturas, 3, 0.0).content_opacity, 1.0);
        }
    }

    /// Além do limite o toast fica invisível (`data-limited:opacity-0`) — mas continua ocupando o
    /// seu lugar na geometria, exatamente como na referência.
    #[test]
    fn alem_do_limite_apaga_mas_nao_desaparece() {
        let alturas = [68.0; 5];
        for i in 0..3 {
            assert_eq!(geom(i, false, &alturas, 3, 0.0).opacity, 1.0, "índice {i}");
        }
        for i in 3..5 {
            let g = geom(i, false, &alturas, 3, 0.0);
            assert_eq!(g.opacity, 0.0, "índice {i}");
            assert!(g.offset > 0.0, "índice {i}: ainda tem posição");
        }
    }

    /// Expandida, o fundo volta a `--popover` (sem escurecimento); colapsada, escurece por índice.
    #[test]
    fn escurecimento_apenas_colapsada() {
        let alturas = [68.0; 3];
        assert_eq!(geom(2, false, &alturas, 3, 0.06).darken, 0.12);
        assert_eq!(geom(2, true, &alturas, 3, 0.06).darken, 0.0);
        assert_eq!(geom(0, false, &alturas, 3, 0.06).darken, 0.0, "o da frente");
    }

    // --- Geometria expandida ----------------------------------------------------------------

    /// Expandida, cada toast fica na SUA altura, largura cheia, e o deslocamento é a soma das
    /// alturas dos da frente mais 12px de fresta por posição.
    #[test]
    fn expandida_empilha_pelo_gap_acumulado() {
        let alturas = [68.0, 90.0, 46.0];
        let g0 = geom(0, true, &alturas, 3, 0.0);
        let g1 = geom(1, true, &alturas, 3, 0.0);
        let g2 = geom(2, true, &alturas, 3, 0.0);

        assert_eq!((g0.scale, g1.scale, g2.scale), (1.0, 1.0, 1.0));
        assert_eq!((g0.height, g1.height, g2.height), (68.0, 90.0, 46.0));
        assert_eq!(g0.offset, 0.0);
        assert_eq!(g1.offset, 68.0 + GAP);
        assert_eq!(g2.offset, 68.0 + 90.0 + GAP * 2.0);
        // E não há sobreposição: a base de cada um fica ACIMA do topo do anterior.
        assert!(g1.offset > g0.offset + g0.height);
        assert!(g2.offset > g1.offset + g1.height);
    }

    /// Uma pilha vazia não explode (o `heights.first()` é `Option`) — isso acontece de verdade no
    /// frame em que o último toast sai.
    #[test]
    fn pilha_vazia_nao_explode() {
        let g = geom(0, false, &[], 3, 0.01);
        assert_eq!(g.height, 0.0);
        assert_eq!(g.offset, 0.0);
    }

    // --- Entrada e saída --------------------------------------------------------------------

    /// A entrada parte de fora da janela: a própria altura mais o recuo do viewport, que é o que
    /// põe o toast rente à borda. E **não** parte apagado — a referência só mexe no `translateY` no
    /// `data-starting-style`.
    #[test]
    fn entrada_vem_de_fora_e_opaca() {
        let alvo = geom(0, false, &[68.0], 3, 0.0);
        let inicio = entering_geom(alvo);
        assert_eq!(inicio.offset, -(68.0 + INSET));
        assert_eq!(inicio.opacity, 1.0, "a entrada NÃO é um fade");
        assert_eq!(inicio.height, alvo.height);
        assert_eq!(inicio.scale, alvo.scale);
    }

    /// A saída sai pro mesmo lado E apaga (`data-ending-style:opacity-0`).
    #[test]
    fn saida_desliza_e_apaga() {
        let visivel = geom(0, false, &[68.0], 3, 0.0);
        let fim = leaving_geom(visivel);
        assert_eq!(fim.offset, -(68.0 + INSET));
        assert_eq!(fim.opacity, 0.0);
        assert_eq!(fim.height, visivel.height, "a altura não muda na saída");
    }

    /// O lerp de geometria anda campo por campo e é aparado nas duas pontas.
    #[test]
    fn lerp_de_geometria() {
        let a = geom(0, false, &[68.0, 68.0], 3, 0.06);
        let b = geom(1, false, &[68.0, 68.0], 3, 0.06);
        let meio = lerp_geom(a, b, 0.5);
        assert!((meio.scale - 0.95).abs() < 1e-5);
        assert!((meio.darken - 0.03).abs() < 1e-5);
        assert_eq!(lerp_geom(a, b, -1.0), a, "aparado em 0");
        assert_eq!(lerp_geom(a, b, 9.0), b, "aparado em 1");
    }

    // --- Altura natural ---------------------------------------------------------------------

    /// A altura estimada bate com a soma dos utilitários da referência: `py-3` dos dois lados, a
    /// borda de 1px de cada lado, e o interior sendo a maior coisa da linha.
    #[test]
    fn altura_estimada_soma_os_utilitarios() {
        // Só título: 20 + 12 + 12 + 2.
        assert_eq!(estimated_height(false, false), 46.0);
        // Título + descrição: 20 + 2 + 20 + 24 + 2.
        assert_eq!(estimated_height(true, false), 68.0);
        // Só título + ação: a ação (24) é mais alta que a linha (20).
        assert_eq!(estimated_height(false, true), 50.0);
        // Com descrição o texto (42) já é mais alto que a ação.
        assert_eq!(estimated_height(true, true), 68.0);
    }

    /// A altura do botão de ação está repetida aqui porque a métrica é privada no `crate::button`.
    /// Este teste é o que faz a repetição não derivar: se o `Xs` mudar de altura, ele quebra.
    #[test]
    fn altura_da_acao_bate_com_o_button_xs() {
        // `ButtonSize::Xs` tem `h-6` = 24px (ver `crate::button::ButtonSize::height`).
        assert_eq!(ACTION_HEIGHT, 24.0);
    }

    /// A medida do `canvas` mede o CONTEÚDO; a superfície é 2px mais alta (a altura é border-box).
    /// Uma medida zero é ignorada — é o que o primeiro prepaint devolve, e aceitá-la faria a pilha
    /// colapsar por um frame.
    #[test]
    fn medida_soma_a_borda_e_ignora_zero() {
        let mut m = manager();
        let id = push_raw(&mut m, Toast::new("x"));
        let estimado = m.toasts[0].natural;

        m.record_height(id, 0.0);
        assert_eq!(m.toasts[0].natural, estimado, "zero é ignorado");

        m.record_height(id, 66.0);
        assert_eq!(m.toasts[0].natural, 68.0, "66 de conteúdo + 2 de borda");

        m.record_height(ToastId(999), 200.0); // id inexistente: não faz nada
        assert_eq!(m.toasts[0].natural, 68.0);
    }

    // --- Fila -------------------------------------------------------------------------------

    /// O mais recente vai pra FRENTE da fila (índice 0), como no base-ui.
    #[test]
    fn o_mais_recente_fica_na_frente() {
        let mut m = manager();
        let a = push_raw(&mut m, Toast::new("primeiro"));
        let b = push_raw(&mut m, Toast::new("segundo"));
        assert_eq!(m.toasts[0].id, b);
        assert_eq!(m.toasts[1].id, a);
        assert_eq!(m.len(), 2);
        assert!(!m.is_empty());
    }

    /// `begin_leave` é idempotente — é o que torna `dismiss` seguro de chamar duas vezes (o clique
    /// na ação dispensa, e o clique no corpo do toast chega junto).
    #[test]
    fn saida_e_idempotente() {
        let mut m = manager();
        let id = push_raw(&mut m, Toast::new("x"));
        assert!(m.begin_leave(id), "a primeira marca");
        let alvo = m.toasts[0].target;
        assert!(!m.begin_leave(id), "a segunda não");
        assert_eq!(m.toasts[0].target, alvo, "e não reinicia a animação");
        assert!(!m.begin_leave(ToastId(999)), "id inexistente");
    }

    /// Um toast em saída deixa a fila DE GEOMETRIA na hora (os de trás sobem), mas continua na
    /// lista até a animação acabar.
    #[test]
    fn toast_em_saida_perde_o_lugar_na_pilha() {
        let mut m = manager();
        push_raw(&mut m, Toast::new("a"));
        push_raw(&mut m, Toast::new("b"));
        let frente = m.toasts[0].id;
        m.begin_leave(frente);

        let vivos: Vec<usize> = (0..m.toasts.len())
            .filter(|&i| m.toasts[i].leaving.is_none())
            .collect();
        assert_eq!(vivos.len(), 1, "só um na pilha");
        assert_eq!(m.len(), 2, "mas os dois ainda na lista");
    }

    // --- Relógio ----------------------------------------------------------------------------

    /// O relógio anda, vence uma vez só, e **pausa** com a pilha aberta (é o que o base-ui faz com
    /// o ponteiro em cima).
    #[test]
    fn relogio_anda_pausa_e_vence_uma_vez() {
        let mut m = manager();
        push_raw(&mut m, Toast::new("x").timeout(Duration::from_millis(100)));
        let t = &mut m.toasts[0];

        assert!(!t.tick(Duration::from_millis(40), false));
        assert_eq!(t.remaining, Some(Duration::from_millis(60)));

        assert!(!t.tick(Duration::from_millis(50), true), "pausado não anda");
        assert_eq!(t.remaining, Some(Duration::from_millis(60)));

        assert!(t.tick(Duration::from_millis(60), false), "venceu");
        assert!(!t.tick(Duration::from_millis(10), false), "não vence duas vezes");
    }

    /// Um toast em saída não conta mais tempo (senão emitiria um segundo `Dismissed`).
    #[test]
    fn toast_em_saida_para_o_relogio() {
        let mut m = manager();
        push_raw(&mut m, Toast::new("x").timeout(Duration::from_millis(10)));
        m.toasts[0].leaving = Some(Instant::now());
        assert!(!m.toasts[0].tick(Duration::from_secs(9), false));
    }

    /// Os defaults de tempo: 5s no geral, **nunca** no `loading`, e o builder vence os dois.
    #[test]
    fn defaults_de_tempo() {
        assert_eq!(Toast::new("x").effective_timeout(), Some(DEFAULT_TIMEOUT));
        assert_eq!(
            Toast::new("x").loading().effective_timeout(),
            None,
            "um `loading` não sai sozinho: quem o criou sabe quando acabou"
        );
        assert_eq!(
            Toast::new("x").loading().timeout(Duration::from_secs(1)).effective_timeout(),
            Some(Duration::from_secs(1)),
            "o builder vence o default do tipo"
        );
        assert_eq!(Toast::new("x").persistent().effective_timeout(), None);
        assert_eq!(Toast::new("x").success().effective_timeout(), Some(DEFAULT_TIMEOUT));
    }

    // --- Alvo e transição -------------------------------------------------------------------

    /// `retarget` só reinicia a transição quando o alvo MUDA DE VERDADE. Comparar por igualdade
    /// exata reiniciaria todo frame por ruído de medida — e a pilha nunca chegaria ao lugar.
    #[test]
    fn retarget_ignora_ruido_de_medida() {
        let mut m = manager();
        push_raw(&mut m, Toast::new("x"));
        let t = &mut m.toasts[0];
        let alvo = t.target;
        assert!(t.anim.is_none());

        // Mesmo alvo com ruído de 1e-4: nada acontece.
        let ruido = Geom {
            height: alvo.height + 0.0001,
            ..alvo
        };
        t.retarget(ruido);
        assert!(t.anim.is_none(), "ruído não reinicia a transição");
        assert_eq!(t.target, alvo, "e não troca o alvo");

        // Uma mudança real acende a transição.
        t.retarget(Geom {
            offset: alvo.offset + 50.0,
            ..alvo
        });
        assert!(t.anim.is_some());
        assert_eq!(t.target.offset, alvo.offset + 50.0);
    }

    /// Sem transição em curso, o visível É o alvo.
    #[test]
    fn visivel_e_o_alvo_quando_parado() {
        let mut m = manager();
        push_raw(&mut m, Toast::new("x"));
        assert_eq!(m.toasts[0].visible(), m.toasts[0].target);
        assert!(!m.toasts[0].animating());
    }

    /// A transição parte de onde o olho está vendo, não do alvo anterior — se um deslocamento
    /// anterior estava no meio do caminho, o novo começa dali (mesma regra do `Slide` do
    /// [`crate::tabs`]).
    #[test]
    fn transicao_parte_do_visivel() {
        let mut m = manager();
        push_raw(&mut m, Toast::new("x"));
        let t = &mut m.toasts[0];
        let a = t.target;
        let b = Geom { offset: 100.0, ..a };
        t.retarget(b);
        // No instante 0 o visível ainda é `a`.
        let visivel = t.visible();
        assert!((visivel.offset - a.offset).abs() < 1.0);

        let c = Geom { offset: -100.0, ..a };
        t.retarget(c);
        let (from, _) = t.anim.expect("transição acesa");
        assert!(
            (from.offset - visivel.offset).abs() < 2.0,
            "o novo parte de onde o olho viu ({}), não do alvo anterior ({})",
            visivel.offset,
            b.offset
        );
    }

    // --- Posições ---------------------------------------------------------------------------

    /// As seis posições da referência existem, e só as `bottom-*` ancoram embaixo.
    #[test]
    fn as_seis_posicoes_e_a_ancoragem() {
        for p in [
            ToastPosition::BottomLeft,
            ToastPosition::BottomCenter,
            ToastPosition::BottomRight,
        ] {
            assert!(p.anchors_bottom(), "{p:?}");
        }
        for p in [
            ToastPosition::TopLeft,
            ToastPosition::TopCenter,
            ToastPosition::TopRight,
        ] {
            assert!(!p.anchors_bottom(), "{p:?}");
        }
        assert_eq!(
            ToastPosition::default(),
            ToastPosition::BottomRight,
            "o default da referência"
        );
    }

    /// Os números do viewport, resolvidos: `max-w-90` = 360, `--toast-inset` = `--spacing(8)` = 32
    /// (o `sm:`, que nesta base sempre se aplica), `peek` e `gap` = `--spacing(3)` = 12.
    #[test]
    fn medidas_do_viewport() {
        assert_eq!(MAX_WIDTH, 360.0, "max-w-90");
        assert_eq!(INSET, 32.0, "sm:--spacing(8)");
        assert_eq!(PEEK, 12.0, "--spacing(3)");
        assert_eq!(GAP, 12.0, "--spacing(3)");
        assert_eq!(RADIUS, 10.0, "--radius-lg");
        assert_eq!((PAD_X, PAD_Y), (14.0, 12.0), "px-3.5 py-3");
        assert_eq!(TEXT_SIZE, 14.0, "text-sm");
        assert_eq!((GAP_ACTION, GAP_ICON, GAP_TEXT), (6.0, 8.0, 2.0));
        assert_eq!(ICON_SIZE, 16.0, "w-4");
        assert_eq!(SCALE_STEP, 0.1);
    }

    /// A geometria das duas sombras do `shadow-lg/5`. Se alguém trocar por uma sombra só (ou por um
    /// `shadow-xs` copiado do card), o toast perde a profundidade que o separa do fundo.
    #[test]
    fn sombra_e_o_shadow_lg() {
        theme::set_theme(theme::ThemeMode::Dark);
        let s = surface_shadows();
        assert_eq!(s.len(), 2, "shadow-lg são DUAS sombras");
        assert_eq!(s[0].offset.y, px(10.0));
        assert_eq!(s[0].blur_radius, px(15.0));
        assert_eq!(s[0].spread_radius, px(-3.0));
        assert_eq!(s[1].offset.y, px(4.0));
        assert_eq!(s[1].blur_radius, px(6.0));
        assert_eq!(s[1].spread_radius, px(-4.0));
        for sh in &s {
            assert_eq!(sh.offset.x, px(0.0), "as duas descem, não deslocam de lado");
            assert!(sh.color.a < 0.1, "o `/5` deixa a sombra bem discreta");
        }
    }

    /// O builder do descritor: o título é obrigatório e o resto é opcional, e os atalhos de tipo
    /// realmente trocam o tipo (o último chamado ganha).
    #[test]
    fn builder_do_descritor() {
        let t = Toast::new("Título");
        assert_eq!(t.title.as_ref(), "Título");
        assert!(t.description.is_none());
        assert!(t.kind.is_none(), "sem tipo = sem ícone, como na referência");
        assert!(t.action.is_none());

        let t = Toast::new("x").error().success();
        assert_eq!(t.kind, Some(ToastType::Success), "o último builder ganha");

        let t = Toast::new("x")
            .description("d")
            .warning()
            .action("Desfazer", |_, _| {});
        assert_eq!(t.description.as_ref().map(|d| d.to_string()), Some("d".into()));
        assert_eq!(t.kind, Some(ToastType::Warning));
        assert_eq!(t.action.as_ref().map(|a| a.to_string()), Some("Desfazer".into()));
        assert!(t.on_action.is_some());
    }

    /// O limite nunca é zero — um `with_limit(0)` apagaria a pilha inteira em silêncio.
    #[test]
    fn limite_tem_piso_de_um() {
        assert_eq!(manager().with_limit(0).limit, 1);
        assert_eq!(manager().with_limit(5).limit, 5);
        assert_eq!(manager().limit, DEFAULT_LIMIT);
        let default = DEFAULT_LIMIT;
        assert_eq!(default, 3, "o default do base-ui");
    }

    /// A fase do spinner dá uma volta por segundo e volta pra zero — se o `fract()` sumir, o
    /// ângulo cresce sem limite e a precisão de `f32` degrada depois de alguns minutos.
    #[test]
    fn fase_do_spinner_da_uma_volta_por_segundo() {
        let fase = |ms: u64| {
            (Duration::from_millis(ms).as_secs_f32() / SPIN_PERIOD.as_secs_f32()).fract()
        };
        assert!(fase(0).abs() < 1e-6);
        assert!((fase(500) - 0.5).abs() < 1e-5);
        assert!(fase(1000).abs() < 1e-5, "volta pra zero");
        assert!((fase(1500) - 0.5).abs() < 1e-5);
        assert_eq!(SPIN_PERIOD, Duration::from_millis(1000), "animate-spin");
    }

    /// As durações da referência, travadas: `.5s` na geometria, `250ms` no conteúdo.
    #[test]
    fn duracoes_da_referencia() {
        assert_eq!(TRANSITION, Duration::from_millis(500));
        assert_eq!(CONTENT_FADE, Duration::from_millis(250));
        assert!(CONTENT_FADE < TRANSITION, "o conteúdo aparece antes de assentar");
    }
}

/// Testes que precisam de uma JANELA — os que verificam **layout**, não aritmética.
///
/// O resto do módulo é lógica pura, mas o porte deste componente troca três construções de CSS por
/// hipóteses de layout do GPUI/taffy, e as três quebrariam em silêncio para um teste puro:
///
/// 1. o `position: fixed` + `inset` + `left-1/2 -translate-x-1/2` do viewport virou **padding +
///    alinhamento de flex**, com `w_full` + `max_w` produzindo o `min(100% - inset*2, 360px)`;
/// 2. o `transform: scale()` dos toasts de trás virou **largura fracionária** (`gpui::relative`)
///    com recuo lateral — e depende de o `w`/`left` percentual resolver contra a caixa do viewport;
/// 3. os toasts são filhos **absolutos de um pai de altura zero**, e a altura natural de cada um vem
///    de um `canvas` de medida dentro de um bloco com altura IMPOSTA pela pilha.
///
/// Se qualquer uma falhar, a pilha inteira sai do lugar sem nenhum teste puro notar. A janela é
/// headless (`VisualTestContext`), no mesmo espírito de [`crate::tabs::tests_de_janela`].
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{AppContext as _, TestAppContext, VisualTestContext};

    /// Caixa do harness — é ela (e não a janela) que define o `size_full` do overlay, pra os números
    /// esperados não dependerem do tamanho de janela default do GPUI.
    const LARGURA: f32 = 800.0;
    const ALTURA: f32 = 600.0;

    /// Largura esperada do viewport: `min(800 - 32*2, 360)` = 360.
    const VIEWPORT_W: f32 = MAX_WIDTH;

    struct Harness {
        toasts: gpui::Entity<ToastManager>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .relative()
                .w(px(LARGURA))
                .h(px(ALTURA))
                .child(toast_layer(&self.toasts))
        }
    }

    /// Abre a janela, empurra `titulos` (o último fica na FRENTE) e deixa a pilha assentada.
    ///
    /// Todos os toasts são `persistent()`: com relógio andando, o render pediria
    /// `request_animation_frame` pra sempre e o `run_until_parked` nunca voltaria.
    fn abrir(
        cx: &mut TestAppContext,
        position: ToastPosition,
        titulos: &[&str],
    ) -> (gpui::Entity<ToastManager>, Vec<ToastId>, VisualTestContext) {
        let titulos: Vec<String> = titulos.iter().map(|t| t.to_string()).collect();
        let window = cx.add_window(move |_window, cx| {
            let toasts = cx.new(|cx| ToastManager::new(cx).with_position(position));
            Harness { toasts }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let manager = vcx.read(|cx| harness.read(cx).toasts.clone());
        let ids = manager.update(&mut vcx, |m, cx| {
            titulos
                .iter()
                .map(|t| m.push(Toast::new(t.clone()).persistent(), cx))
                .collect::<Vec<_>>()
        });

        // Três voltas: a primeira faz a medida chegar, a segunda deixa o alvo ser recalculado com a
        // altura medida, a terceira assenta. O progresso das transições vem do relógio de parede, que
        // o executor de teste não adianta — daí o `settle`.
        for _ in 0..3 {
            vcx.run_until_parked();
            manager.update(&mut vcx, |m, cx| m.settle(cx));
        }
        vcx.run_until_parked();
        (manager, ids, vcx)
    }

    /// A caixa do CONTEÚDO de um toast, e a da SUPERFÍCIE (1px maior de cada lado).
    ///
    /// ⚠️ A superfície derivada só é confiável **onde a altura imposta é a natural** — ou seja no
    /// toast da FRENTE. Num toast de trás a superfície é 10% mais baixa e o conteúdo TRANSBORDA
    /// (assim como no CSS, onde `Toast.Root` não tem `overflow-hidden`; quem esconde o transbordo é a
    /// opacidade zero do conteúdo e o toast da frente pintando em cima). Então de um toast de trás
    /// use `origin` e `size.width` — pra altura, leia a geometria calculada com [`altura_alvo`].
    fn caixas(
        manager: &gpui::Entity<ToastManager>,
        vcx: &mut VisualTestContext,
        id: ToastId,
    ) -> (Bounds<Pixels>, Bounds<Pixels>) {
        let conteudo = vcx.read(|cx| manager.read(cx).probe(id).expect("toast medido"));
        let superficie = Bounds {
            origin: gpui::point(conteudo.origin.x - px(1.0), conteudo.origin.y - px(1.0)),
            size: gpui::size(
                conteudo.size.width + px(2.0),
                conteudo.size.height + px(2.0),
            ),
        };
        (conteudo, superficie)
    }

    /// **A altura natural medida bate com a estimada.**
    ///
    /// É o teste que valida [`estimated_height`] — e com ela [`LINE_HEIGHT`], que é o único número
    /// DEDUZIDO da geometria (o coss pode redefinir `--text-sm--line-height`). Se o texto renderizar
    /// numa altura de linha diferente, é aqui que aparece, com o valor real na mensagem.
    #[gpui::test]
    fn altura_natural_medida_bate_com_a_estimada(cx: &mut TestAppContext) {
        let (manager, ids, vcx) = abrir(cx, ToastPosition::BottomRight, &["Só título"]);
        let medida = vcx.read(|cx| {
            manager
                .read(cx)
                .toasts
                .iter()
                .find(|t| t.id == ids[0])
                .expect("toast na fila")
                .natural
        });
        assert!(
            (medida - estimated_height(false, false)).abs() < 0.5,
            "a altura MEDIDA foi {medida}; a estimada é {} — se divergirem, LINE_HEIGHT ({LINE_HEIGHT}) \
             não é a altura de linha real do `text-sm`",
            estimated_height(false, false)
        );
    }

    /// **O viewport aterrissa no canto certo, com a largura certa.**
    ///
    /// `bottom-right`: 32px de cada borda, `min(largura - 64, 360)` de largura. É o padding + o
    /// `justify_end`/`items_end` fazendo o trabalho do `fixed` + `inset` da referência.
    #[gpui::test]
    fn viewport_no_canto_e_com_a_largura_limitada(cx: &mut TestAppContext) {
        let (manager, ids, mut vcx) = abrir(cx, ToastPosition::BottomRight, &["Salvo"]);
        let (_, s) = caixas(&manager, &mut vcx, ids[0]);

        // O toast da frente ocupa o viewport inteiro (escala 1), então a caixa dele É o viewport.
        assert!(
            (f32::from(s.size.width) - VIEWPORT_W).abs() < 0.5,
            "largura veio {}, esperado {VIEWPORT_W} (max-w-90)",
            f32::from(s.size.width)
        );
        let direita = f32::from(s.origin.x + s.size.width);
        assert!(
            (direita - (LARGURA - INSET)).abs() < 0.5,
            "a borda direita veio em {direita}, esperado {} (inset de 32)",
            LARGURA - INSET
        );
        let base = f32::from(s.origin.y + s.size.height);
        assert!(
            (base - (ALTURA - INSET)).abs() < 0.5,
            "a base veio em {base}, esperado {}",
            ALTURA - INSET
        );
    }

    /// As posições `center` centralizam e as `left` encostam à esquerda — o `left-1/2
    /// -translate-x-1/2` e o `left-(--toast-inset)` da referência, sem transform.
    #[gpui::test]
    fn as_posicoes_horizontais(cx: &mut TestAppContext) {
        let (m, ids, mut vcx) = abrir(cx, ToastPosition::BottomCenter, &["x"]);
        let (_, s) = caixas(&m, &mut vcx, ids[0]);
        let centro = f32::from(s.origin.x + s.size.width / 2.0);
        assert!(
            (centro - LARGURA / 2.0).abs() < 0.5,
            "centro veio em {centro}, esperado {}",
            LARGURA / 2.0
        );

        let (m, ids, mut vcx) = abrir(cx, ToastPosition::TopLeft, &["x"]);
        let (_, s) = caixas(&m, &mut vcx, ids[0]);
        assert!(
            (f32::from(s.origin.x) - INSET).abs() < 0.5,
            "esquerda veio em {}, esperado {INSET}",
            f32::from(s.origin.x)
        );
        assert!(
            (f32::from(s.origin.y) - INSET).abs() < 0.5,
            "no topo a superfície começa no inset; veio {}",
            f32::from(s.origin.y)
        );
    }

    /// **O invariante do `peek`, medido.** Numa pilha colapsada ancorada embaixo, o topo de cada
    /// toast fica 12px acima do topo do da frente — e o de trás é 10% mais estreito e CENTRADO.
    ///
    /// Este é o teste que prova que a largura fracionária substitui o `scale()` sem deslocar a
    /// pilha: se o recuo lateral estivesse errado, os centros não coincidiriam.
    #[gpui::test]
    fn pilha_colapsada_faz_o_peek_e_centraliza_o_de_tras(cx: &mut TestAppContext) {
        let (m, ids, mut vcx) = abrir(cx, ToastPosition::BottomRight, &["antigo", "novo"]);
        // O último empurrado é o da FRENTE.
        let (frente, tras) = (ids[1], ids[0]);
        let (_, sf) = caixas(&m, &mut vcx, frente);
        let (_, st) = caixas(&m, &mut vcx, tras);

        // Largura: 10% menor.
        assert!(
            (f32::from(st.size.width) - f32::from(sf.size.width) * 0.9).abs() < 0.6,
            "o de trás veio com {} de largura; esperado 90% de {}",
            f32::from(st.size.width),
            f32::from(sf.size.width)
        );
        // Centros coincidem (`transform-origin: 50%`).
        let cf = f32::from(sf.origin.x + sf.size.width / 2.0);
        let ct = f32::from(st.origin.x + st.size.width / 2.0);
        assert!((cf - ct).abs() < 0.6, "centros: frente {cf}, trás {ct}");

        // O topo dele fica exatamente um `peek` acima — o invariante do baralho, agora medido.
        let topo_f = f32::from(sf.origin.y);
        let topo_t = f32::from(st.origin.y);
        assert!(
            (topo_f - topo_t - PEEK).abs() < 0.6,
            "topo da frente {topo_f}, topo de trás {topo_t}: a diferença tem que ser {PEEK}"
        );

        // A altura vem da geometria calculada (o conteúdo do de trás transborda a superfície mais
        // baixa, então a caixa medida não serve pra isso — ver o doc de `caixas`).
        let (hf, ht) = (
            altura_alvo(&m, &mut vcx, frente),
            altura_alvo(&m, &mut vcx, tras),
        );
        assert!(
            (ht - hf * 0.9).abs() < 0.6,
            "o de trás tem que ter 90% da altura DO DA FRENTE ({hf}); veio {ht}"
        );
    }

    /// A altura que a geometria calculada impôs à superfície (o valor que vai pro `.h()`).
    fn altura_alvo(
        manager: &gpui::Entity<ToastManager>,
        vcx: &mut VisualTestContext,
        id: ToastId,
    ) -> f32 {
        vcx.read(|cx| {
            manager
                .read(cx)
                .toasts
                .iter()
                .find(|t| t.id == id)
                .expect("na fila")
                .visible()
                .height
        })
    }

    /// Expandida, cada toast fica na SUA altura, largura cheia, e separado por 12px de fresta.
    #[gpui::test]
    fn pilha_expandida_abre_com_a_fresta(cx: &mut TestAppContext) {
        let (m, ids, mut vcx) = abrir(cx, ToastPosition::BottomRight, &["antigo", "novo"]);
        // O hover não existe num teste headless; a expansão é o estado que ele produziria.
        m.update(&mut vcx, |m, cx| {
            m.expanded = true;
            cx.notify();
        });
        for _ in 0..3 {
            vcx.run_until_parked();
            m.update(&mut vcx, |m, cx| m.settle(cx));
        }
        vcx.run_until_parked();

        let (frente, tras) = (ids[1], ids[0]);
        let (_, sf) = caixas(&m, &mut vcx, frente);
        let (_, st) = caixas(&m, &mut vcx, tras);

        // Largura cheia nos dois: sem escala.
        assert!((f32::from(st.size.width) - f32::from(sf.size.width)).abs() < 0.6);
        assert!((f32::from(sf.size.width) - VIEWPORT_W).abs() < 0.6);
        // Cada um na sua altura natural (os dois são iguais aqui, então basta conferir que NÃO
        // encolheu).
        assert!((f32::from(st.size.height) - f32::from(sf.size.height)).abs() < 0.6);
        // E a fresta de 12px entre a base do de trás e o topo do da frente.
        let fresta = f32::from(sf.origin.y) - f32::from(st.origin.y + st.size.height);
        assert!(
            (fresta - GAP).abs() < 0.6,
            "a fresta veio {fresta}, esperado {GAP}"
        );
    }

    /// Um toast com descrição e ação é mais alto que um só de título — e a ação (24px) é o que manda
    /// quando não há descrição.
    #[gpui::test]
    fn altura_cresce_com_descricao_e_com_acao(cx: &mut TestAppContext) {
        let window = cx.add_window(|_window, cx| {
            let toasts = cx.new(ToastManager::new);
            Harness { toasts }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let m = vcx.read(|cx| harness.read(cx).toasts.clone());

        let (so_titulo, com_descricao, com_acao) = m.update(&mut vcx, |m, cx| {
            (
                m.push(Toast::new("A").persistent(), cx),
                m.push(
                    Toast::new("B").description("segunda linha").persistent(),
                    cx,
                ),
                m.push(Toast::new("C").action("Ok", |_, _| {}).persistent(), cx),
            )
        });
        for _ in 0..3 {
            vcx.run_until_parked();
            m.update(&mut vcx, |m, cx| m.settle(cx));
        }
        vcx.run_until_parked();

        let altura = |id: ToastId, vcx: &mut VisualTestContext| {
            vcx.read(|cx| {
                manager_natural(manager_ref(cx, &m), id)
            })
        };
        let (a, b, c) = (
            altura(so_titulo, &mut vcx),
            altura(com_descricao, &mut vcx),
            altura(com_acao, &mut vcx),
        );
        assert!((a - estimated_height(false, false)).abs() < 0.5, "só título: {a}");
        assert!((b - estimated_height(true, false)).abs() < 0.5, "com descrição: {b}");
        assert!((c - estimated_height(false, true)).abs() < 0.5, "com ação: {c}");
        assert!(b > c && c > a, "descrição > ação > nada; veio {b} > {c} > {a}");
    }

    fn manager_ref<'a>(cx: &'a App, m: &gpui::Entity<ToastManager>) -> &'a ToastManager {
        m.read(cx)
    }

    fn manager_natural(m: &ToastManager, id: ToastId) -> f32 {
        m.toasts.iter().find(|t| t.id == id).expect("na fila").natural
    }

    /// Dispensar remove o toast da pilha e, passada a animação, da fila.
    #[gpui::test]
    fn dispensar_tira_da_pilha(cx: &mut TestAppContext) {
        let (m, ids, mut vcx) = abrir(cx, ToastPosition::BottomRight, &["a", "b"]);
        m.update(&mut vcx, |m, cx| m.dismiss(ids[0], cx));
        vcx.run_until_parked();

        let (na_fila, na_pilha) = vcx.read(|cx| {
            let m = m.read(cx);
            (
                m.len(),
                m.toasts.iter().filter(|t| t.leaving.is_none()).count(),
            )
        });
        assert_eq!(na_fila, 2, "continua na fila enquanto anima a saída");
        assert_eq!(na_pilha, 1, "mas já saiu da pilha");
    }
}

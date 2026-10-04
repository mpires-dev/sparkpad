//! `Switch` — o **toggle** (liga/desliga em pílula) do `empire-ui`, com o visual do design system
//! [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/switch.tsx`
//!
//! Alternativa visual ao [`crate::checkbox::Checkbox`] para params booleanos: em vez de um quadrado
//! com check, é um **trilho pílula** com uma **bolinha que desliza** de um lado ao outro.
//!
//! # Anatomia
//!
//! ```text
//!    30                                 30
//!  ┌──────────────┐                   ┌──────────────┐
//!  │ ●            │ 18   →            │            ● │ 18
//!  └──────────────┘                   └──────────────┘
//!   ▲1px de respiro (`p-px`)            ▲ a bolinha andou 12px
//!   bg-input (desligado)                bg-primary (ligado)
//! ```
//!
//! Toda a geometria deriva de `--thumb-size`, que na referência é `--spacing(5)` (20px) com um
//! `sm:[--thumb-size:--spacing(4)]` derrubando pra 16px. O `sm:` do Tailwind é ≥640px e uma janela
//! de desktop está SEMPRE acima disso, então o valor efetivo é **16px** — e daí saem os 18 de
//! altura (`thumb + 2`), os 30 de largura (`thumb * 2 - 2`) e os 12 de curso (`thumb - 4`).
//!
//! # Contrato
//!
//! É um `Entity` (view) próprio que mantém o seu estado (`on`) e **emite**
//! [`SwitchEvent::Toggle`] com o novo valor a cada clique. Quem usa só assina o evento
//! (`cx.subscribe`) e grava onde quiser — mesmo padrão desacoplado do `Checkbox`/`ScrubInput`.
//!
//! ```ignore
//! let sw = cx.new(|cx| Switch::new(false, cx));
//! cx.subscribe(&sw, |_this, _e, ev: &SwitchEvent, _cx| println!("{ev:?}")).detach();
//! ```
//!
//! # O que o GPUI exigiu adaptar
//!
//! | coss                                       | aqui                                              |
//! |--------------------------------------------|---------------------------------------------------|
//! | `translate-x` da bolinha                   | `left` calculado (não há `transform` em `div`)     |
//! | `scale-x-110` no `:active`                 | **largura** 10% maior, ancorada pelo lado certo    |
//! | `origin-left` / `origin-[--thumb-size_50%]`| o lado que fica parado ao esticar                  |
//! | `focus-visible:ring-2 ring-offset-1`       | dois overlays absolutos concêntricos               |
//! | `transition-*`                             | progresso por tempo + `request_animation_frame`    |
//!
//! Duas coisas que NÃO existem aqui porque não existem na referência: o switch **não tem bisel**
//! (nenhum `before:shadow-*`/`inset-shadow-*` no `switch.tsx`, diferente do `Input`, `Card`,
//! `Button` e `Frame`) e o trilho **não tem borda**. Não adicione nenhum dos dois "por coerência".
//!
//! # O que ficou de fora (e por quê)
//!
//! - **O raio elíptico do pressionado** (`rounded-[16px/17.6px]`): `rounded` é escalar no GPUI. O
//!   efeito prático é ~0,73px de diferença no raio horizontal de cada quina, porque os dois raios
//!   da referência já são aparados pra um formato de pílula — ver
//!   [`tests::raio_da_bolinha_e_sempre_pilula`], que faz a conta.
//! - **`focus-visible` vs `focus`**: o GPUI não distingue foco por teclado de foco por ponteiro.
//!   Ver [`ring_overlays`].
//! - **A interpolação de cor do trilho é em RGBA**, não no espaço de alfa pré-multiplicado que o
//!   CSS usa. Divergem alguns por cento de alfa no MEIO da transição de 200ms; as duas pontas são
//!   idênticas. Ver [`track_bg`].
//! - **`cursor-pointer`**: a referência não declara nenhum (o default de `<button>` é a seta); aqui
//!   o estado ativo usa a mãozinha, pela convenção da casa. O `cursor-not-allowed` do desabilitado
//!   é fiel.

use crate::color::{lerp, Rgba8};
use crate::theme;
use gpui::{
    div, point, px, App, BoxShadow, Context, CursorStyle, Div, EventEmitter, FocusHandle, Focusable,
    Hsla, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseUpEvent,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};
use std::time::{Duration, Instant};

/// Evento emitido pelo [`Switch`] quando o usuário alterna o estado (clique ou espaço/enter com o
/// foco nele). Carrega o **novo valor** (já alternado), no mesmo espírito do
/// [`crate::checkbox::CheckboxEvent`].
#[derive(Debug, Clone, Copy)]
pub enum SwitchEvent {
    /// Novo estado do switch após a interação.
    Toggle(bool),
}

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Os utilitários Tailwind do `switch.tsx` resolvidos em número, com a paleta `neutral` do Tailwind
// expandida (`neutral-100 #f5f5f5`, `neutral-400 #a3a3a3`, `neutral-500 #737373`,
// `neutral-800 #262626`) e os `color-mix` do tema já calculados. São os MESMOS tokens que o
// `crate::button` e o `crate::frame` já resolveram — os valores aqui batem com os de lá de
// propósito, porque `--primary`, `--input`, `--ring` e `--background` são tokens do tema, não do
// componente.
//
// Mesma disciplina de cor do resto do crate: TODO valor é `0xRRGGBBAA`, com o byte de alfa,
// sempre — ver [`crate::color`]. Misturar com os tokens de 6 dígitos de [`crate::theme`] desloca os
// canais e produz uma cor completamente diferente sem erro de compilação (já custou três bugs
// visíveis nesta base). A única ponte é [`crate::color::opaque`].

/// Tokens visuais do switch, por tema.
#[derive(Clone, Copy, Debug)]
struct SwitchPalette {
    /// `--primary` — trilho **ligado** (`data-checked:bg-primary`).
    track_on: Rgba8,
    /// `--input` — trilho **desligado** (`data-unchecked:bg-input`). É translúcido nos dois temas,
    /// então o trilho apagado toma o tom da superfície embaixo dele.
    track_off: Rgba8,
    /// `--background` — a bolinha (`bg-background`).
    ///
    /// ⚠️ No tema escuro isso é um cinza QUASE PRETO (`#141414`), não branco: a bolinha do coss
    /// é da cor do fundo da página nos dois temas. Ligada ela lê por contraste com o trilho
    /// `--primary` (que no escuro é quase branco); desligada, quem a separa do trilho é a sombra.
    /// Não é erro de porte — é o que `bg-background` significa.
    thumb: Rgba8,
    /// `--ring` — cor do anel de foco (`focus-visible:ring-ring`).
    ring: Rgba8,
    /// `--ring-offset-background`, que no coss é o próprio `--background` — a coroa de 1px entre o
    /// trilho e o anel (`ring-offset-1 ring-offset-background`).
    ring_offset: Rgba8,
    /// Cor das duas camadas do `shadow-sm/5` da bolinha (preto a 5%).
    thumb_shadow: Rgba8,
}

/// Tema **claro**.
const SWITCH_LIGHT: SwitchPalette = SwitchPalette {
    track_on: Rgba8(0x262626ff),  // neutral-800
    track_off: Rgba8(0x0000001a), // black 10%
    thumb: Rgba8(0xffffffff),
    ring: Rgba8(0xa3a3a3ff), // neutral-400
    ring_offset: Rgba8(0xffffffff),
    thumb_shadow: Rgba8(0x0000000d), // black 5%
};

/// Tema **escuro**.
const SWITCH_DARK: SwitchPalette = SwitchPalette {
    track_on: Rgba8(0xf5f5f5ff),  // neutral-100
    track_off: Rgba8(0xffffff14), // white 8%
    // mix(neutral-950 96%, white) = #141414 — o mesmo `--background` do `crate::frame`.
    thumb: Rgba8(0x141414ff),
    ring: Rgba8(0x737373ff), // neutral-500
    ring_offset: Rgba8(0x141414ff),
    thumb_shadow: Rgba8(0x0000000d),
};

/// A paleta do switch no tema corrente.
fn palette() -> &'static SwitchPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &SWITCH_DARK,
        theme::ThemeMode::Light => &SWITCH_LIGHT,
    }
}

// =================================================================================================
// Geometria
// =================================================================================================

/// `--thumb-size` — o lado da bolinha, e a raiz de TODA a geometria.
///
/// A referência declara `--spacing(5)` (20px) e sobrescreve com `sm:[--thumb-size:--spacing(4)]`
/// (16px). Como o `sm:` do Tailwind (≥640px) sempre vale numa janela de desktop, o valor efetivo é
/// **16**. Se algum dia existir um switch "grande", ele é `THUMB_SIZE = 20` e todo o resto
/// acompanha sozinho.
const THUMB_SIZE: f32 = 16.0;

/// Respiro entre a bolinha e a borda do trilho — `p-px`.
const PAD: f32 = 1.0;

/// Altura do trilho — `h-[calc(var(--thumb-size)+2px)]` = **18px** (a bolinha mais o `p-px` de
/// cada lado).
const TRACK_H: f32 = THUMB_SIZE + 2.0 * PAD;

/// Largura do trilho — `w-[calc(var(--thumb-size)*2-2px)]` = **30px**.
const TRACK_W: f32 = THUMB_SIZE * 2.0 - 2.0;

/// Curso horizontal da bolinha — `data-checked:translate-x-[calc(var(--thumb-size)-4px)]` = **12px**.
///
/// Fecha com a geometria por construção: `30 − 1 − 1 − 16 = 12`, ou seja, ligada a bolinha encosta
/// no respiro do lado direito. O teste [`tests::curso_fecha_com_a_geometria`] trava as duas contas
/// juntas — se alguém mudar `THUMB_SIZE` e uma das duas fórmulas deixar de bater, ele falha.
const TRAVEL: f32 = THUMB_SIZE - 4.0;

/// Fator do `scale-x-110` do estado pressionado.
const PRESSED_SCALE_X: f32 = 1.10;

/// Espessura do anel de foco — `focus-visible:ring-2`.
const RING_WIDTH: f32 = 2.0;

/// Folga entre o trilho e o anel — `ring-offset-1`. Diferente do [`crate::tabs`], aqui o offset
/// EXISTE, e é pintado com `--ring-offset-background` (uma coroa da cor do fundo).
const RING_OFFSET: f32 = 1.0;

/// `data-disabled:opacity-64` — o coss esmaece o CONJUNTO em vez de trocar cor por cor. Uma fonte
/// de verdade só: não existe um par "apagado" de cada token.
const DISABLED_OPACITY: f32 = 0.64;

// =================================================================================================
// Tempo (as transições da referência)
// =================================================================================================

/// `transition-[background-color,box-shadow] duration-200` do trilho — vale pro cross-fade do fundo
/// **e** pro anel de foco, que no CSS é `box-shadow` e portanto entra na mesma transição.
const TRACK_TRANSITION: Duration = Duration::from_millis(200);

/// `translate .15s` **e** `transform-origin .15s` da bolinha. As duas andam juntas com a mesma
/// duração e a mesma curva, então um progresso só serve pras duas (ver [`thumb_box`]).
const THUMB_TRANSITION: Duration = Duration::from_millis(150);

/// `scale .1s .1s` — a duração do esticão do pressionado.
const PRESS_DURATION: Duration = Duration::from_millis(100);

/// `scale .1s .1s` — o **atraso** do esticão. Ele começa 100ms depois do mouse-down (e, na volta,
/// 100ms depois do mouse-up): é o que dá a sensação de "a bolinha cede depois de você apertar".
const PRESS_DELAY: Duration = Duration::from_millis(100);

/// `cubic-bezier(0.4, 0, 0.2, 1)` — o `--default-transition-timing-function` do Tailwind, que é a
/// curva das classes `transition-*`. Vale no trilho.
const EASE_TAILWIND: [f32; 4] = [0.4, 0.0, 0.2, 1.0];

/// `cubic-bezier(0.25, 0.1, 0.25, 1)` — o valor **inicial** de `transition-timing-function` no CSS
/// (a palavra-chave `ease`). Vale na bolinha.
///
/// A diferença é sutil mas real, e não é escolha nossa: o `[transition:translate_.15s,…]` da
/// referência é a forma ABREVIADA de `transition`, e uma abreviada reseta as sub-propriedades que
/// não menciona pro valor inicial — inclusive a curva. Ou seja, a bolinha NÃO usa a curva do
/// Tailwind que o trilho usa.
const EASE_CSS: [f32; 4] = [0.25, 0.1, 0.25, 1.0];

/// Avalia uma `cubic-bezier(x1, y1, x2, y2)` do CSS em `t`.
///
/// A curva do CSS é paramétrica: `x` e `y` são polinômios de Bézier no parâmetro `u`, e o que se
/// quer é `y` no `u` onde `x(u) == t`. Com `x1`/`x2` em `[0,1]` a curva `x(u)` é monótona, então
/// uma bisseção resolve — 24 passos dão precisão de ~6e-8, muito além do que um pixel percebe, e
/// isto roda algumas vezes por frame.
fn ease(curve: [f32; 4], t: f32) -> f32 {
    let [x1, y1, x2, y2] = curve;
    let t = t.clamp(0.0, 1.0);
    // Bézier cúbica com P0=(0,0) e P3=(1,1).
    let bezier = |a: f32, b: f32, u: f32| {
        let inv = 1.0 - u;
        3.0 * inv * inv * u * a + 3.0 * inv * u * u * b + u * u * u
    };
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let mid = 0.5 * (lo + hi);
        if bezier(x1, x2, mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    bezier(y1, y2, 0.5 * (lo + hi))
}

/// Progresso linear `[0,1]` de uma transição a partir do **decorrido**, com `delay` de atraso e
/// duração `dur`. Parte pura (sem relógio) de [`progress`], pra ser testável.
fn elapsed_progress(elapsed: Duration, delay: Duration, dur: Duration) -> f32 {
    if dur.is_zero() {
        return 1.0;
    }
    let andado = elapsed.saturating_sub(delay).as_secs_f32();
    (andado / dur.as_secs_f32()).clamp(0.0, 1.0)
}

/// Progresso linear `[0,1]` de uma transição que começou em `since`.
///
/// `None` (nunca começou) devolve `1.0` — "já assentada". É o que o PRIMEIRO frame precisa: sem
/// isso todo switch da tela deslizaria a bolinha ao abrir a janela.
fn progress(since: Option<Instant>, delay: Duration, dur: Duration) -> f32 {
    match since {
        None => 1.0,
        Some(t) => elapsed_progress(t.elapsed(), delay, dur),
    }
}

/// A caixa da bolinha — `(left, largura)` em px a partir da borda esquerda do trilho.
///
/// `pos_t` é o quanto ela já andou rumo à posição de LIGADO (`0` = desligada, `1` = ligada) e
/// `press_t` o quanto do esticão de pressionado já entrou (`0` = repouso, `1` = cheio).
///
/// # Por que não é `translate` + `scale`
///
/// Não existe `transform` em `div` no GPUI (o `TransformationMatrix` só serve pra `svg`/imagem), e
/// margem negativa colapsa layout de flex nesta base. Então a bolinha é um filho **absoluto** e
/// aqui se calcula a geometria EQUIVALENTE:
///
/// - o `translate-x` vira o `left`;
/// - o `scale-x-110` vira 10% de largura a mais;
/// - o `transform-origin` vira **qual lado fica parado** enquanto ela engorda. Desligada a origem
///   é `origin-left` (cresce pra direita); ligada é `origin-[var(--thumb-size)_50%]`, ou seja a
///   borda DIREITA da bolinha (cresce pra esquerda). Nos dois casos ela cresce pro lado oposto ao
///   movimento, e a ponta que já estava encostada no respiro continua encostada.
///
/// A conta é a definição de escala em torno de uma origem: `left = base + o - o*k`. Como o `o` do
/// CSS também é animado (`transform-origin .15s`, a MESMA duração e curva do `translate`),
/// interpolá-lo com o próprio `pos_t` reproduz a referência de graça.
fn thumb_box(pos_t: f32, press_t: f32) -> (f32, f32) {
    let base_left = PAD + TRAVEL * pos_t;
    let k = 1.0 + (PRESSED_SCALE_X - 1.0) * press_t;
    // `origin-left` (0) quando desligado → `origin-[var(--thumb-size)_50%]` (16) quando ligado.
    let origin_x = THUMB_SIZE * pos_t;
    (base_left - origin_x * (k - 1.0), THUMB_SIZE * k)
}

/// Cor do trilho no meio do `transition-[background-color] duration-200`.
///
/// `t` é o progresso JÁ com a curva aplicada; `on` é o estado de DESTINO (o que o switch é agora),
/// então a interpolação sai da cor do estado anterior.
///
/// Interpola em RGBA via [`crate::color::lerp`]. O CSS interpola cor com alfa **pré-multiplicado**,
/// e `--input` é translúcido — os caminhos não são idênticos, mas divergem no máximo alguns por
/// cento de alfa no meio de uma transição de 200ms.
fn track_bg(on: bool, t: f32) -> Hsla {
    let p = palette();
    let (de, para) = if on {
        (p.track_off, p.track_on)
    } else {
        (p.track_on, p.track_off)
    };
    lerp(de.hsla(), para.hsla(), t)
}

/// As duas camadas do `shadow-sm/5` da bolinha.
///
/// `--shadow-sm` do Tailwind é `0 1px 3px 0 rgb(0 0 0/.1), 0 1px 2px -1px rgb(0 0 0/.1)`; o
/// modificador `/5` troca o alfa das DUAS camadas por 5% (`0x0d`).
///
/// Aqui pode ser sombra de verdade ([`gpui::BoxShadow`]) e não overlay: o `Window::paint_shadows`
/// não recorta o retângulo da sombra pra fora do elemento, mas a bolinha é **opaca**
/// (`bg-background`), então a parte que cairia embaixo dela fica escondida — o mesmo caso seguro do
/// `shadow-xs` da moldura do [`crate::input::Input`].
///
/// ⚠️ No tema **escuro** ela é praticamente invisível: 5% de preto sobre um trilho escuro. É assim
/// na referência também (o coss não troca a cor da sombra por tema), então não é omissão — o que
/// separa a bolinha do trilho no escuro é o `--primary` claro quando ligado.
fn thumb_shadow() -> Vec<BoxShadow> {
    let cor = palette().thumb_shadow.hsla();
    vec![
        BoxShadow {
            color: cor,
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(3.0),
            spread_radius: px(0.0),
        },
        BoxShadow {
            color: cor,
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(2.0),
            spread_radius: px(-1.0),
        },
    ]
}

/// Um overlay absoluto recuado igualmente nos quatro lados; `inset` negativo cresce pra FORA.
///
/// Sempre `rounded_full`: o switch é uma pílula, e uma pílula dilatada continua pílula. O GPUI apara
/// o raio em `min(w,h)/2` na hora de pintar, então `rounded_full` dá o raio CERTO em cada uma das
/// três caixas (18, 20 e 24 de altura) sem precisar de três constantes.
fn inset_overlay(inset: f32) -> Div {
    div()
        .absolute()
        .top(px(inset))
        .left(px(inset))
        .right(px(inset))
        .bottom(px(inset))
        .rounded_full()
}

/// O **anel de foco** — `focus-visible:ring-2 ring-ring ring-offset-1 ring-offset-background`.
///
/// ⚠️ `ring` não existe no GPUI, e não dá pra fingir com sombra: o `spread_radius` dilata os
/// limites mas MANTÉM o raio, então a curvatura sai errada nas quinas, e num fundo translúcido a
/// sombra atravessa e tinge o componente (já aconteceu no [`crate::input::Input`]).
///
/// No CSS um `ring` com offset são duas sombras concêntricas: 1px da cor do FUNDO colado no
/// elemento, e 2px da cor do anel por fora dela. Então são dois overlays, com bandas **adjacentes
/// e sem sobreposição**: a coroa ocupa de `-1` a `0`, o anel de `-3` a `-1`.
///
/// `k` é a opacidade do conjunto — o anel entra e sai desvanecendo, porque no CSS ele é
/// `box-shadow` e o trilho declara `transition-[…,box-shadow] duration-200`.
///
/// ⚠️ A coroa é pintada com `--background`, literalmente como o `ring-offset-background` do
/// Tailwind. Se o switch estiver sobre uma superfície que NÃO é `--background` (um card, um
/// popover), aparece um fio de 1px da cor errada em volta dele — e é assim na referência também.
///
/// O `focus-visible` da referência é resolvido por [`crate::focus_ring`], que guarda a modalidade do
/// último input: tecla acende, ponteiro apaga. Quem chama aqui já combinou `focado` com
/// `focus_ring::visible()`, então este par de coroas só desenha quando é foco de teclado.
///
/// (Uma versão anterior deste doc dizia que o anel aparecia com QUALQUER foco. Era verdade antes do
/// `focus_ring` existir; ficou obsoleta quando o módulo foi ligado aqui.)
fn ring_overlays(k: f32) -> [Div; 2] {
    let p = palette();
    [
        inset_overlay(-RING_OFFSET)
            .border_1()
            .border_color(p.ring_offset.scaled(k)),
        inset_overlay(-(RING_OFFSET + RING_WIDTH))
            .border(px(RING_WIDTH))
            .border_color(p.ring.scaled(k)),
    ]
}

// =================================================================================================
// O componente
// =================================================================================================

/// Controle booleano em pílula (trilho + bolinha que desliza), com o visual do coss.
///
/// # Onde mora o estado de animação
///
/// Nos **campos deste struct**, e não numa tabela `thread_local` indexada por id como no
/// [`crate::button::Button`] e no [`crate::input::Input`]. A diferença é a natureza dos dois: eles
/// são `RenderOnce` (não têm onde guardar nada e precisam de uma tabela externa, com teto e
/// descarte), e o `Switch` é um `Entity` com identidade própria. Além de mais simples, é mais
/// correto: o relógio da animação é "quando o `on` mudou", e o `on` já mora aqui — as duas coisas
/// ficam juntas, sem chance de um id colidir ou de a entrada ser descartada por estouro de tabela.
///
/// O motor de frames é o mesmo do `Button`: `Window::request_animation_frame` no fim do render
/// enquanto houver alguma transição em curso.
pub struct Switch {
    on: bool,
    disabled: bool,
    /// Id estável deste switch (pra `div().id(...)` único na árvore).
    id: u64,
    /// Handle de foco DESTE switch — é ele que decide se o anel aparece.
    ///
    /// `Option` só por causa dos testes: `gpui::FocusHandle::new` é `pub(crate)` no gpui, então um
    /// teste unitário (que não tem `App`) não consegue construir um. [`Switch::new`] sempre
    /// preenche.
    focus_handle: Option<FocusHandle>,
    /// Instante da última troca de `on` — o relógio da translação da bolinha e do cross-fade do
    /// trilho. `None` = nunca trocou (primeiro frame renderiza o estado final, sem animar).
    toggled_at: Option<Instant>,
    /// Se o ponteiro está pressionado sobre o switch (o `:active` da referência).
    pressed: bool,
    /// Instante da última troca de [`Self::pressed`] — o relógio do esticão.
    pressed_at: Option<Instant>,
    /// Se o anel estava aceso no último render (`is_focused` **e** modalidade de teclado, ver
    /// [`crate::focus_ring`]). `None` = ainda não renderizou, e o primeiro render nunca anima o anel
    /// (mesma regra do `crate::input`).
    seen_ring: Option<bool>,
    /// Instante da última troca do anel — o relógio dele.
    focus_at: Option<Instant>,
}

impl Switch {
    /// Cria um `Switch` com estado inicial `on`.
    pub fn new(on: bool, cx: &mut Context<Self>) -> Self {
        Self {
            on,
            disabled: false,
            id: cx.entity_id().as_u64(),
            focus_handle: Some(cx.focus_handle()),
            toggled_at: None,
            pressed: false,
            pressed_at: None,
            seen_ring: None,
            focus_at: None,
        }
    }

    /// Desabilita (builder, pra usar no `cx.new`): o switch para de responder e o conjunto esmaece
    /// pra 64%, com cursor de "não permitido".
    ///
    /// ```ignore
    /// let sw = cx.new(|cx| Switch::new(true, cx).disabled(true));
    /// ```
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Estado atual.
    pub fn on(&self) -> bool {
        self.on
    }

    /// Se está desabilitado.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Define o estado de fora (sincronização externa). **Não** emite `Toggle` — evita loops de
    /// feedback com quem assina (mesma regra do `Checkbox::set_checked`).
    ///
    /// A bolinha desliza igual: a transição é do estado, não da causa.
    pub fn set_on(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.on == on {
            return;
        }
        self.on = on;
        self.toggled_at = Some(Instant::now());
        cx.notify();
    }

    /// Habilita/desabilita em runtime.
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        // Um switch desabilitado no meio de um clique não pode ficar com a bolinha esticada.
        if disabled {
            self.pressed = false;
        }
        cx.notify();
    }

    /// Alterna o estado por interação do usuário e **emite** `Toggle(novo)`. Desabilitado, não faz
    /// nada.
    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.on = !self.on;
        self.toggled_at = Some(Instant::now());
        cx.emit(SwitchEvent::Toggle(self.on));
        cx.notify();
    }

    /// Registra o `:active` (mouse-down/up). Só reinicia o relógio quando o estado REALMENTE muda:
    /// o GPUI pode reemitir o mesmo valor, e reiniciar a cada emissão travaria o esticão no começo.
    fn set_pressed(&mut self, pressed: bool, cx: &mut Context<Self>) {
        if self.pressed == pressed || self.disabled {
            return;
        }
        self.pressed = pressed;
        self.pressed_at = Some(Instant::now());
        cx.notify();
    }
}

impl EventEmitter<SwitchEvent> for Switch {}

impl Render for Switch {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ativo = !self.disabled;
        let focado = ativo
            && self
                .focus_handle
                .as_ref()
                .is_some_and(|h| h.is_focused(window));
        // A referência pede `focus-visible:ring-2`, não `focus:ring-2` — o anel só acende quando o
        // foco veio do TECLADO. O GPUI não tem essa distinção (e o `track_focus` faz o mouse-down
        // focar o elemento), então a modalidade vem de [`crate::focus_ring`], que é estado da
        // SESSÃO e não deste componente. Ver o doc daquele módulo.
        let anel_aceso = focado && crate::focus_ring::visible();

        // --- Relógio do anel ------------------------------------------------------------------
        // Nem o foco nem a modalidade chegam por callback: as duas são lidas a cada render. A troca
        // é detectada comparando com o último valor visto, e o PRIMEIRO render não anima (senão um
        // switch que nasce focado piscaria o anel).
        if self.seen_ring != Some(anel_aceso) {
            let primeiro = self.seen_ring.is_none();
            self.seen_ring = Some(anel_aceso);
            if !primeiro {
                self.focus_at = Some(Instant::now());
            }
        }

        // --- Progresso das quatro transições --------------------------------------------------
        let trilho_raw = progress(self.toggled_at, Duration::ZERO, TRACK_TRANSITION);
        let pos_raw = progress(self.toggled_at, Duration::ZERO, THUMB_TRANSITION);
        let press_raw = progress(self.pressed_at, PRESS_DELAY, PRESS_DURATION);
        let anel_raw = progress(self.focus_at, Duration::ZERO, TRACK_TRANSITION);

        // Cada progresso é "quanto andou desde a última troca"; virar "quanto do estado NOVO
        // aplicar" é uma inversão quando o destino é o estado desligado/solto/desfocado.
        let pos_t = {
            let e = ease(EASE_CSS, pos_raw);
            if self.on {
                e
            } else {
                1.0 - e
            }
        };
        let press_t = if !ativo {
            // `:not-data-disabled:scale-x-110` — desabilitado não estica.
            0.0
        } else {
            let e = ease(EASE_CSS, press_raw);
            if self.pressed {
                e
            } else {
                1.0 - e
            }
        };
        let anel_k = if !ativo {
            0.0
        } else {
            let e = ease(EASE_TAILWIND, anel_raw);
            if anel_aceso {
                e
            } else {
                1.0 - e
            }
        };

        // --- Bolinha ---------------------------------------------------------------------------
        //
        // Filho ABSOLUTO, não item de flex: assim mexer na largura (o `scale-x-110`) e no `left`
        // (o `translate-x`) não empurra nada, e não é preciso margem negativa — que nesta base já
        // colapsou um layout de flex.
        //
        // `rounded_full` cobre os dois raios da referência: `rounded-(--thumb-size)` (16px numa
        // caixa de 16px) e o elíptico `16px/17.6px` do pressionado são AMBOS aparados pelo CSS pra
        // um formato de pílula. Ver `tests::raio_da_bolinha_e_sempre_pilula`.
        let (left, largura) = thumb_box(pos_t, press_t);
        let p = palette();
        let bolinha = div()
            .absolute()
            .top(px(PAD))
            .left(px(left))
            .w(px(largura))
            .h(px(THUMB_SIZE))
            .rounded_full()
            .bg(p.thumb.hsla())
            .shadow(thumb_shadow());

        // --- Trilho ----------------------------------------------------------------------------
        let mut trilho = div()
            .id(("switch", self.id))
            // `relative` porque bolinha e anel são filhos ABSOLUTOS. Sem `overflow_hidden`: a
            // bolinha nunca sai do trilho, e o anel PRECISA sair (um recorte o comeria).
            .relative()
            .flex_none()
            .w(px(TRACK_W))
            .h(px(TRACK_H))
            .rounded_full()
            .bg(track_bg(self.on, ease(EASE_TAILWIND, trilho_raw)))
            .cursor(if ativo {
                // Desvio consciente: o `<button>` da referência não declara `cursor-pointer` (o
                // default do navegador pra botão é a seta). Aqui segue a convenção da casa, a
                // mesma do `crate::button::Button`, que põe a mãozinha em tudo que é clicável.
                CursorStyle::PointingHand
            } else {
                // `data-disabled:cursor-not-allowed`.
                CursorStyle::OperationNotAllowed
            });

        if ativo {
            // Só um switch ATIVO entra na ordem de tabulação: rastrear o foco de um desabilitado o
            // deixaria alcançável por `tab` sem ter o que fazer ali, e sem anel (porque `focado` já
            // é falso) o usuário perderia o cursor de teclado.
            if let Some(h) = self.focus_handle.as_ref() {
                trilho = trilho.track_focus(h);
            }
            trilho = trilho
                .on_click(cx.listener(|this, _e, _window, cx| this.toggle(cx)))
                // Espaço e enter alternam quando o switch tem o foco — é o que um `<button>` faz, e
                // é o que dá sentido ao anel de foco.
                .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                    if matches!(e.keystroke.key.as_str(), "space" | "enter") {
                        // Chegou aqui = o usuário está no teclado. O `focus_ring::init` já teria
                        // marcado isso; marcar de novo faz a tecla acender o anel mesmo num app que
                        // esqueceu de inicializar o crate.
                        crate::focus_ring::keyboard_used(window);
                        this.toggle(cx);
                    }
                }))
                // O `:active` da referência. `on_mouse_up_out` também solta: sem ele, arrastar pra
                // fora e largar deixaria a bolinha esticada pra sempre.
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _e: &MouseDownEvent, window, cx| {
                        // O mouse-down é ponteiro: apaga o anel (ver `crate::focus_ring`). Fica aqui
                        // e não no `on_click` porque é o evento mais cedo, e porque um press que
                        // termina fora do switch (sem clique) também mudou a modalidade — é o que o
                        // navegador faz.
                        crate::focus_ring::pointer_used(window);
                        this.set_pressed(true, cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _e: &MouseUpEvent, _window, cx| this.set_pressed(false, cx)),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|this, _e: &MouseUpEvent, _window, cx| this.set_pressed(false, cx)),
                );
        } else {
            // `data-disabled:opacity-64`: esmaece o conjunto — trilho, bolinha e sombra.
            trilho = trilho.opacity(DISABLED_OPACITY);
        }

        trilho = trilho.child(bolinha);

        // O anel vem por último: é absoluto, então a ordem só decide quem pinta em cima.
        if anel_k > 0.0 {
            trilho = trilho.children(ring_overlays(anel_k));
        }

        // Enquanto houver transição em curso, pede o próximo frame. É o motor de TODAS as animações
        // deste componente — não há elemento de animação, o progresso vem do tempo decorrido.
        if trilho_raw < 1.0 || pos_raw < 1.0 || press_raw < 1.0 || anel_raw < 1.0 {
            window.request_animation_frame();
        }

        trilho
    }
}

impl Focusable for Switch {
    /// O handle DESTE switch — é o que faz `.track_focus` e o anel de foco funcionarem.
    ///
    /// (Antes isto devolvia um `cx.focus_handle()` novo a cada chamada, ou seja, um handle que não
    /// era o do switch e nunca podia estar focado.)
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.focus_handle
            .clone()
            .unwrap_or_else(|| cx.focus_handle())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um `Switch` sem `App` — o [`gpui::FocusHandle`] não é construível fora do gpui, e a lógica de
    /// estado não precisa dele.
    fn probe(on: bool) -> Switch {
        Switch {
            on,
            disabled: false,
            id: 0,
            focus_handle: None,
            toggled_at: None,
            pressed: false,
            pressed_at: None,
            seen_ring: None,
            focus_at: None,
        }
    }

    // --- Estado -----------------------------------------------------------------------------

    // O estado e o toggle são lógica pura (sem GPU/janela) — testáveis manipulando o campo,
    // espelhando a regra do `toggle`/`set_on`.
    #[test]
    fn toggle_inverte_o_estado() {
        let mut s = probe(false);
        s.on = !s.on;
        assert!(s.on());
        s.on = !s.on;
        assert!(!s.on());
    }

    #[test]
    fn on_reflete_inicial() {
        assert!(probe(true).on());
        assert!(!probe(false).on());
    }

    /// O builder de desabilitado tem default igual ao comportamento antigo (habilitado).
    #[test]
    fn desabilitado_e_opcional_e_default_habilitado() {
        assert!(!probe(false).is_disabled());
        assert!(probe(false).disabled(true).is_disabled());
        assert!(!probe(false).disabled(true).disabled(false).is_disabled());
    }

    /// `data-disabled:opacity-64`.
    #[test]
    fn desabilitado_esmaece_o_conjunto_pra_64() {
        assert_eq!(DISABLED_OPACITY, 0.64);
    }

    // --- Geometria --------------------------------------------------------------------------

    /// Os números da referência, resolvidos no breakpoint `sm:` (que numa janela de desktop sempre
    /// vale): bolinha 16, trilho 30×18, respiro 1.
    #[test]
    fn geometria_do_trilho_e_da_bolinha() {
        assert_eq!(THUMB_SIZE, 16.0, "--thumb-size no sm: é --spacing(4)");
        assert_eq!(PAD, 1.0, "p-px");
        assert_eq!(TRACK_H, 18.0, "h-[calc(--thumb-size+2px)]");
        assert_eq!(TRACK_W, 30.0, "w-[calc(--thumb-size*2-2px)]");
        assert_eq!(TRAVEL, 12.0, "translate-x-[calc(--thumb-size-4px)]");
    }

    /// As duas contas da referência têm que fechar entre si: o curso declarado (`thumb − 4`) é
    /// exatamente o que sobra na largura depois do respiro dos dois lados e da bolinha. Se alguém
    /// mexer no `THUMB_SIZE` e uma das fórmulas deixar de bater, a bolinha para de encostar no
    /// respiro — e é este teste que avisa, não a tela.
    #[test]
    fn curso_fecha_com_a_geometria() {
        assert_eq!(TRAVEL, TRACK_W - 2.0 * PAD - THUMB_SIZE);
    }

    /// Em repouso a bolinha encosta no respiro dos dois extremos, e nunca invade o trilho.
    #[test]
    fn bolinha_encosta_nos_dois_extremos() {
        let (left_off, w_off) = thumb_box(0.0, 0.0);
        assert_eq!((left_off, w_off), (PAD, THUMB_SIZE), "desligada: colada à esquerda");

        let (left_on, w_on) = thumb_box(1.0, 0.0);
        assert_eq!((left_on, w_on), (13.0, THUMB_SIZE), "ligada: 1 + 12");
        assert_eq!(left_on + w_on, TRACK_W - PAD, "ligada: colada à direita");
    }

    /// No meio do percurso a bolinha está no meio do curso — a translação é linear no progresso já
    /// curvado (a curva entra antes, em `pos_t`).
    #[test]
    fn meio_do_percurso_e_meio_do_curso() {
        let (left, _) = thumb_box(0.5, 0.0);
        assert_eq!(left, PAD + TRAVEL / 2.0);
    }

    /// O `scale-x-110` do pressionado: 10% de largura a mais, crescendo pro lado OPOSTO ao
    /// movimento — a ponta que já estava no respiro fica parada.
    ///
    /// É o ponto onde o `transform-origin` da referência importa. Se as duas ancoragens fossem
    /// iguais, ligada a bolinha estouraria 1,6px pra fora do trilho.
    #[test]
    fn pressionado_estica_dez_por_cento_pro_lado_oposto_ao_movimento() {
        let esticada = THUMB_SIZE * PRESSED_SCALE_X;
        assert_eq!(esticada, 17.6, "16 × 1,1");

        // Desligada: `origin-left` → a esquerda fica parada, cresce pra direita.
        let (left, w) = thumb_box(0.0, 1.0);
        assert_eq!(left, PAD, "esquerda parada");
        assert_eq!(w, esticada);

        // Ligada: `origin-[var(--thumb-size)_50%]` → a DIREITA fica parada, cresce pra esquerda.
        let (left, w) = thumb_box(1.0, 1.0);
        assert_eq!(w, esticada);
        assert!((left - 11.4).abs() < 1e-4, "esquerda recuou 1,6px; veio {left}");
        assert!(
            (left + w - (TRACK_W - PAD)).abs() < 1e-4,
            "direita parada no respiro"
        );

        // Em nenhum dos dois casos ela sai do trilho.
        for pos in [0.0, 0.5, 1.0] {
            let (left, w) = thumb_box(pos, 1.0);
            assert!(left >= 0.0, "não sai pela esquerda (pos {pos})");
            assert!(left + w <= TRACK_W, "não sai pela direita (pos {pos})");
        }
    }

    /// O esticão é contínuo: com `press_t` no meio, a largura está no meio dos 10%.
    #[test]
    fn esticao_e_continuo() {
        let (_, w) = thumb_box(0.0, 0.5);
        assert!((w - 16.8).abs() < 1e-4, "16 + metade de 1,6; veio {w}");
    }

    /// **Omissão documentada**: o raio elíptico da referência não é expressável.
    ///
    /// No pressionado o coss pede `border-radius: 16px / 17.6px` — raio horizontal diferente do
    /// vertical. No GPUI `rounded` é escalar. Não custa nada, porém: os DOIS raios (o de repouso,
    /// `rounded-(--thumb-size)` = 16px numa caixa de 16px, e o elíptico do pressionado) estouram a
    /// caixa e são aparados pelo CSS pro formato de pílula. Com o fator de aparo do CSS aplicado, o
    /// elíptico resolve pra ~7,27 × 8; o nosso `rounded_full` (que o GPUI apara em `min(w,h)/2`) dá
    /// 8 × 8. A diferença é 0,73px no raio HORIZONTAL de cada quina, numa bolinha de 17,6px que só
    /// fica esticada enquanto o botão está apertado.
    #[test]
    fn raio_da_bolinha_e_sempre_pilula() {
        // O aparo do GPUI: `min(w,h)/2` em cada quina.
        let raio_gpui = |w: f32, h: f32| (9999.0f32).min(w.min(h) / 2.0);
        assert_eq!(raio_gpui(THUMB_SIZE, THUMB_SIZE), 8.0, "repouso: círculo");
        assert_eq!(
            raio_gpui(THUMB_SIZE * PRESSED_SCALE_X, THUMB_SIZE),
            8.0,
            "pressionada: pílula de raio 8"
        );

        // O aparo do CSS no raio elíptico `16 / 17.6` numa caixa de 17,6 × 16: o fator é o MENOR
        // entre os dois eixos (0,55 no horizontal, 0,4545 no vertical).
        let (rx, ry) = (THUMB_SIZE, THUMB_SIZE * PRESSED_SCALE_X);
        let (w, h) = (THUMB_SIZE * PRESSED_SCALE_X, THUMB_SIZE);
        let fator = (w / (2.0 * rx)).min(h / (2.0 * ry)).min(1.0);
        assert!((rx * fator - 7.2727).abs() < 1e-3, "raio horizontal do CSS");
        assert!((ry * fator - 8.0).abs() < 1e-3, "raio vertical do CSS");
    }

    /// O anel tem offset (diferente do `crate::tabs`), e as duas bandas são **adjacentes**: a coroa
    /// da cor do fundo ocupa de −1 a 0, o anel de −3 a −1. Sobreposição pintaria a coroa por cima
    /// do anel; folga deixaria uma fresta.
    #[test]
    fn anel_tem_offset_e_as_bandas_sao_adjacentes() {
        assert_eq!(RING_WIDTH, 2.0, "ring-2");
        assert_eq!(RING_OFFSET, 1.0, "ring-offset-1");

        let coroa = (-RING_OFFSET, 0.0);
        let anel = (-(RING_OFFSET + RING_WIDTH), -RING_OFFSET);
        assert_eq!(coroa.0, anel.1, "sem fresta e sem sobreposição");
        assert_eq!(anel.0, -3.0, "o anel para 3px fora do trilho");
    }

    // --- Tempo ------------------------------------------------------------------------------

    /// As durações da referência, cada uma no seu lugar.
    #[test]
    fn duracoes_da_referencia() {
        assert_eq!(TRACK_TRANSITION, Duration::from_millis(200), "duration-200");
        assert_eq!(THUMB_TRANSITION, Duration::from_millis(150), "translate .15s");
        assert_eq!(PRESS_DURATION, Duration::from_millis(100), "scale .1s");
        assert_eq!(PRESS_DELAY, Duration::from_millis(100), "scale .1s .1s");
    }

    /// O progresso é linear no decorrido, com o atraso descontado, e apara nas duas pontas.
    ///
    /// O atraso é o que faz o esticão do pressionado só começar 100ms depois do mouse-down. Sem
    /// descontá-lo, o esticão terminaria antes de começar.
    #[test]
    fn progresso_respeita_o_atraso_e_apara() {
        let z = Duration::ZERO;
        let cem = Duration::from_millis(100);

        assert_eq!(elapsed_progress(z, z, cem), 0.0);
        assert_eq!(elapsed_progress(Duration::from_millis(50), z, cem), 0.5);
        assert_eq!(elapsed_progress(cem, z, cem), 1.0);
        assert_eq!(elapsed_progress(Duration::from_secs(9), z, cem), 1.0, "apara em 1");

        // Com atraso: nada acontece antes dele.
        assert_eq!(elapsed_progress(Duration::from_millis(99), cem, cem), 0.0);
        assert_eq!(elapsed_progress(cem, cem, cem), 0.0, "começa no fim do atraso");
        assert_eq!(elapsed_progress(Duration::from_millis(150), cem, cem), 0.5);
        assert_eq!(elapsed_progress(Duration::from_millis(200), cem, cem), 1.0);
    }

    /// Sem relógio (`None`) a transição já está assentada — é o que faz o PRIMEIRO frame renderizar
    /// o estado final em vez de todo switch da tela animar ao abrir a janela.
    #[test]
    fn transicao_que_nunca_comecou_ja_esta_no_fim() {
        assert_eq!(progress(None, Duration::ZERO, TRACK_TRANSITION), 1.0);
        assert_eq!(progress(None, PRESS_DELAY, PRESS_DURATION), 1.0);
    }

    /// As duas curvas: passam por (0,0) e (1,1), são monótonas, e **não são a mesma**.
    ///
    /// Não são a mesma porque a referência usa a abreviada `transition:` na bolinha (que reseta a
    /// curva pro `ease` inicial do CSS) e a classe `transition-[…]` do Tailwind no trilho (que usa
    /// a curva do Tailwind). Se alguém unificar as duas achando que é duplicação, este teste falha.
    #[test]
    fn curvas_de_easing() {
        for curva in [EASE_TAILWIND, EASE_CSS] {
            assert!(ease(curva, 0.0).abs() < 1e-4, "f(0) = 0");
            assert!((ease(curva, 1.0) - 1.0).abs() < 1e-4, "f(1) = 1");
            // Monotonia (com folga pro erro da bisseção).
            let mut anterior = -1.0;
            for i in 0..=50 {
                let y = ease(curva, i as f32 / 50.0);
                assert!(y >= anterior - 1e-5, "monótona em {i}: {y} < {anterior}");
                anterior = y;
            }
            // Aparada fora de [0,1] — o decorrido pode passar de 1 por arredondamento.
            assert_eq!(ease(curva, -1.0), ease(curva, 0.0));
            assert_eq!(ease(curva, 3.0), ease(curva, 1.0));
            // As duas são "ease-out"-ish: no meio do tempo já andaram mais da metade.
            assert!(ease(curva, 0.5) > 0.5);
        }
        // O `ease` do CSS sai mais rápido que a curva do Tailwind (que tem um começo mais lento).
        assert!(
            ease(EASE_CSS, 0.5) > ease(EASE_TAILWIND, 0.5),
            "as duas curvas são distintas"
        );
    }

    // --- Cores ------------------------------------------------------------------------------

    /// **A convenção de cor da paleta, decodificada de verdade.** Todo valor é `0xRRGGBBAA`; um de
    /// 6 dígitos esquecido aqui vira uma cor completamente diferente sem erro de compilação
    /// (`rgba(0xffffff)` é ciano) — foi o que aconteceu três vezes nesta base.
    #[test]
    fn paleta_decodifica_pras_cores_pretendidas() {
        for (nome, p) in [("claro", SWITCH_LIGHT), ("escuro", SWITCH_DARK)] {
            // `--primary`: neutro e OPACO nos dois temas (quase preto no claro, quase branco no
            // escuro). Se o canal vermelho vier zerado, o valor foi lido deslocado.
            let on: gpui::Rgba = p.track_on.hsla().into();
            assert_eq!(on.a, 1.0, "{nome}: trilho ligado é opaco");
            assert!(
                (on.r - on.g).abs() < 0.01 && (on.g - on.b).abs() < 0.01,
                "{nome}: trilho ligado é NEUTRO"
            );

            // `bg-background`: opaco e neutro nos dois temas.
            let thumb: gpui::Rgba = p.thumb.hsla().into();
            assert_eq!(thumb.a, 1.0, "{nome}: bolinha é opaca");
            assert!(
                (thumb.r - thumb.g).abs() < 0.01 && (thumb.g - thumb.b).abs() < 0.01,
                "{nome}: bolinha é NEUTRA"
            );

            // `--ring`: cinza médio, neutro e opaco — o sintoma do bug histórico era ele sair teal
            // (r = 0), porque `0xa3a3a3` lido como rgba perde o canal vermelho.
            let ring: gpui::Rgba = p.ring.hsla().into();
            assert_eq!(ring.a, 1.0, "{nome}: anel opaco");
            assert!(ring.r > 0.1, "{nome}: anel sem canal vermelho — leitura deslocada");
            assert!(
                (ring.r - ring.g).abs() < 0.01 && (ring.g - ring.b).abs() < 0.01,
                "{nome}: anel é cinza NEUTRO"
            );

            // `--input` e a sombra são translúcidos — o trilho apagado toma o tom da superfície.
            assert!(p.track_off.alpha() < 1.0, "{nome}: --input é translúcido");
            assert!(p.thumb_shadow.alpha() < 1.0, "{nome}: sombra é translúcida");

            // `ring-offset-background` É o `--background`, o mesmo da bolinha. Um par diferente
            // aqui significaria que alguém inventou um token.
            assert_eq!(p.ring_offset, p.thumb, "{nome}: a coroa é --background");
        }
    }

    /// Os dois temas invertem o par trilho/bolinha, que é o que garante contraste no estado ligado:
    /// trilho escuro + bolinha branca no claro, trilho claro + bolinha escura no escuro.
    #[test]
    fn temas_invertem_o_par_trilho_bolinha() {
        let luma = |c: Rgba8| {
            let c: gpui::Rgba = c.hsla().into();
            c.r + c.g + c.b
        };
        assert!(luma(SWITCH_LIGHT.track_on) < luma(SWITCH_LIGHT.thumb), "claro: trilho escuro");
        assert!(luma(SWITCH_DARK.track_on) > luma(SWITCH_DARK.thumb), "escuro: trilho claro");
    }

    /// O `shadow-sm/5` são DUAS camadas (é o que `--shadow-sm` do Tailwind define), as duas com o
    /// alfa trocado pra 5% pelo modificador `/5`. Uma camada só (que é o `shadow-xs` da moldura do
    /// `Input`) daria uma sombra visivelmente mais dura.
    #[test]
    fn sombra_da_bolinha_tem_as_duas_camadas_do_shadow_sm() {
        theme::set_theme(theme::ThemeMode::Dark);
        let s = thumb_shadow();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].offset.y, px(1.0));
        assert_eq!(s[0].blur_radius, px(3.0));
        assert_eq!(s[0].spread_radius, px(0.0));
        assert_eq!(s[1].blur_radius, px(2.0));
        assert_eq!(s[1].spread_radius, px(-1.0), "a 2ª camada encolhe 1px");
        for c in &s {
            assert!((c.color.a - 13.0 / 255.0).abs() < 1e-3, "/5 → alfa 0x0d");
        }
    }

    /// O cross-fade do trilho sai da cor do estado ANTERIOR e chega na do atual — nos dois
    /// sentidos. Se o `on` fosse lido como origem em vez de destino, a transição andaria ao
    /// contrário e o trilho piscaria a cor errada a cada clique.
    #[test]
    fn trilho_faz_cross_fade_entre_input_e_primary() {
        // O `lerp` faz um round-trip HSLA→RGBA→HSLA, então as pontas voltam com erro de ~1e-8: a
        // comparação é por proximidade, não por igualdade de bits.
        fn mesma_cor(a: Hsla, b: Hsla, quem: &str) {
            let (a, b): (gpui::Rgba, gpui::Rgba) = (a.into(), b.into());
            for (canal, (x, y)) in [("r", (a.r, b.r)), ("g", (a.g, b.g)), ("b", (a.b, b.b)), ("a", (a.a, b.a))] {
                assert!((x - y).abs() < 1e-5, "{quem}: canal {canal} veio {x}, esperado {y}");
            }
        }

        for modo in [theme::ThemeMode::Dark, theme::ThemeMode::Light] {
            theme::set_theme(modo);
            let p = palette();

            // Ligando: começa em `--input` e termina em `--primary`.
            mesma_cor(track_bg(true, 0.0), p.track_off.hsla(), "ligando, início");
            mesma_cor(track_bg(true, 1.0), p.track_on.hsla(), "ligando, fim");
            // Desligando: o contrário.
            mesma_cor(track_bg(false, 0.0), p.track_on.hsla(), "desligando, início");
            mesma_cor(track_bg(false, 1.0), p.track_off.hsla(), "desligando, fim");

            // No meio está entre as duas (e não numa terceira cor: o `lerp` é em RGBA justamente
            // pra não passear pelo círculo de matiz).
            let meio: gpui::Rgba = track_bg(true, 0.5).into();
            let de: gpui::Rgba = p.track_off.hsla().into();
            let para: gpui::Rgba = p.track_on.hsla().into();
            assert!(meio.a >= de.a.min(para.a) - 1e-6 && meio.a <= de.a.max(para.a) + 1e-6);
            assert!(meio.r >= de.r.min(para.r) - 1e-6 && meio.r <= de.r.max(para.r) + 1e-6);
        }
        theme::set_theme(theme::ThemeMode::Dark); // não deixa estado vazando pros outros testes
    }
}

/// Testes que precisam de uma **janela de verdade**: os de cima cobrem geometria e cor (aritmética
/// pura), mas nada ali prova que o clique chega ao `toggle`, que a tecla chega ao `on_key_down` ou
/// que o `track_focus` está pendurado no elemento certo. Isso só se vê passando eventos reais pelo
/// sistema de despacho — é o mesmo arranjo do `crate::tabs`.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{AppContext as _, Modifiers, Pixels, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Um harness mínimo que só ancora o switch no canto da janela. Diferente do `Tabs`, aqui não é
    /// preciso medir nada: a geometria é FIXA (30×18 no canto), então o centro é conhecido.
    struct Harness {
        sw: gpui::Entity<Switch>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().flex().flex_col().child(self.sw.clone())
        }
    }

    /// O centro do trilho, em coordenadas da janela.
    fn centro() -> gpui::Point<Pixels> {
        point(px(TRACK_W / 2.0), px(TRACK_H / 2.0))
    }

    /// Abre a janela com um switch e devolve a entidade, os eventos capturados e o contexto visual.
    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
        on: bool,
        disabled: bool,
    ) -> (gpui::Entity<Switch>, Rc<RefCell<Vec<bool>>>, VisualTestContext) {
        let eventos: Rc<RefCell<Vec<bool>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_window, cx| {
            let sw = cx.new(|cx| Switch::new(on, cx).disabled(disabled));
            cx.subscribe(&sw, move |_this, _s, ev: &SwitchEvent, _cx| match ev {
                SwitchEvent::Toggle(v) => capturados.borrow_mut().push(*v),
            })
            .detach();
            Harness { sw }
        });

        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let sw = vcx.read(|cx| harness.read(cx).sw.clone());
        (sw, eventos, vcx)
    }

    /// **O clique alterna e emite.** É o contrato público do componente, e o único jeito de provar
    /// que o `on_click` está pendurado no trilho é passar um clique de verdade.
    #[gpui::test]
    fn clique_alterna_e_emite(cx: &mut TestAppContext) {
        let (sw, eventos, mut vcx) = abrir(cx, false, false);

        vcx.simulate_click(centro(), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert!(sw.read(cx).on(), "o clique ligou"));
        assert_eq!(*eventos.borrow(), vec![true]);

        vcx.simulate_click(centro(), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert!(!sw.read(cx).on(), "o segundo clique desligou"));
        assert_eq!(*eventos.borrow(), vec![true, false]);
    }

    /// **Desabilitado não alterna e não emite** — e o `opacity-64` não é o que impede, é o guarda no
    /// `toggle`. Um switch que esmaece mas continua respondendo é o pior dos dois mundos.
    #[gpui::test]
    fn desabilitado_ignora_o_clique(cx: &mut TestAppContext) {
        let (sw, eventos, mut vcx) = abrir(cx, false, true);

        vcx.simulate_click(centro(), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert!(!sw.read(cx).on(), "não alternou"));
        assert!(eventos.borrow().is_empty(), "não emitiu");
    }

    /// **Espaço e enter alternam com o switch focado.** É o que faz o anel de foco significar algo:
    /// sem tecla que alterne, um switch alcançável por `tab` seria um beco sem saída.
    ///
    /// O foco chega pelo clique (o `track_focus` faz o mouse-down focar), que é justamente o caminho
    /// que um usuário de teclado usaria depois de um `tab` — e aqui é o jeito de chegar ao mesmo
    /// estado sem depender da ordem de tabulação do harness.
    #[gpui::test]
    fn espaco_e_enter_alternam_com_o_foco(cx: &mut TestAppContext) {
        let (sw, eventos, mut vcx) = abrir(cx, false, false);

        vcx.simulate_click(centro(), Modifiers::default()); // liga e leva o foco
        vcx.run_until_parked();

        vcx.simulate_keystrokes("space");
        vcx.run_until_parked();
        vcx.read(|cx| assert!(!sw.read(cx).on(), "espaço desligou"));

        vcx.simulate_keystrokes("enter");
        vcx.run_until_parked();
        vcx.read(|cx| assert!(sw.read(cx).on(), "enter ligou de novo"));

        assert_eq!(*eventos.borrow(), vec![true, false, true], "cada interação emitiu");
    }

    /// **O clique NÃO acende o anel de foco; a tecla acende.**
    ///
    /// Este é o `focus-visible` da referência (ver [`crate::focus_ring`]). O `track_focus` faz o
    /// GPUI focar o switch no mouse-down, e o foco em si é desejável — é o que deixa o teclado
    /// continuar de onde o clique parou. O que não é desejável é o ANEL.
    #[gpui::test]
    fn o_clique_nao_acende_o_anel_mas_a_tecla_acende(cx: &mut TestAppContext) {
        let (sw, _eventos, mut vcx) = abrir(cx, false, false);

        vcx.simulate_click(centro(), Modifiers::default());
        vcx.run_until_parked();

        vcx.update(|window, cx| {
            assert!(
                !crate::focus_ring::visible(),
                "clicar não pode acender o anel (a referência é focus-visible)"
            );
            assert!(
                sw.read(cx)
                    .focus_handle
                    .as_ref()
                    .expect("o `new` sempre preenche")
                    .is_focused(window),
                "mas o foco VAI pro switch — é o que faz o teclado continuar dali"
            );
        });

        // Agora pelo teclado: o anel passa a valer.
        vcx.simulate_keystrokes("space");
        vcx.run_until_parked();
        assert!(crate::focus_ring::visible(), "a tecla acende o anel");

        // E um clique depois o apaga de novo — é a modalidade do ÚLTIMO input.
        vcx.simulate_click(centro(), Modifiers::default());
        vcx.run_until_parked();
        assert!(!crate::focus_ring::visible(), "voltar pro mouse apaga o anel");
    }

    /// **O clique atravessa a bolinha.** Ela cobre mais da metade do trilho e é pintada por cima, e
    /// na referência é `pointer-events-none`. Se ela criasse hitbox, clicar no meio dela não
    /// alternaria nada — e o meio dela é exatamente onde o usuário clica.
    #[gpui::test]
    fn a_bolinha_nao_intercepta_o_clique(cx: &mut TestAppContext) {
        let (sw, _eventos, mut vcx) = abrir(cx, false, false);

        // Centro da bolinha em repouso e desligada: `PAD + THUMB_SIZE/2` na horizontal.
        let sobre_a_bolinha = point(px(PAD + THUMB_SIZE / 2.0), px(TRACK_H / 2.0));
        vcx.simulate_click(sobre_a_bolinha, Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert!(sw.read(cx).on(), "o clique na bolinha alternou o switch"));
    }
}

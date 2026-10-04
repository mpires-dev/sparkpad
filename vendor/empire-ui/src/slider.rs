//! `slider` — o controle de faixa do [coss][1].
//!
//! [1]: https://github.com/cosscom/coss/blob/main/apps/ui/registry/default/ui/slider.tsx
//!
//! # A anatomia
//!
//! Um trilho de 4px com o carril em `--input`, um indicador em `--primary` que o preenche até o valor,
//! e um polegar branco de 16px com borda, bisel e sombra. Arrastar, clicar no trilho e o teclado
//! (setas, `Home`/`End`, `PageUp`/`PageDown`) mudam o valor. Aceita **vários** valores — é o que faz
//! dele um controle de faixa e não só de valor único.
//!
//! # `thumbAlignment="edge"`: o polegar não passa das pontas
//!
//! O detalhe de geometria que define o componente. Com alinhamento por BORDA, o que percorre o trilho
//! de ponta a ponta é a **borda** do polegar, não o centro dele: no mínimo a borda esquerda encosta em
//! 0, no máximo a direita encosta em `L`. O curso útil é portanto `L − 16`, e não `L`.
//!
//! É a razão de o carril ter `inset-x-0.5` e o indicador `ms-0.5`: com o polegar nunca ultrapassando as
//! pontas, o carril pode recuar 2px de cada lado sem que apareça trilho descoberto nos extremos.
//!
//! ## Onde o indicador para é indiferente
//!
//! O `.tsx` não diz onde o indicador termina — isso mora no primitivo. Fiz a conta: o polegar é opaco e
//! tem 16px, e para QUALQUER fração o ponto de parada cai dentro da área que ele cobre. Terminar na
//! borda esquerda do polegar, no centro dele ou na borda direita produz **o mesmo pixel na tela**.
//! Escolhi o centro; a escolha não é observável, e é por isso que ela não precisou ser adivinhada com
//! cuidado.
//!
//! # Adaptações
//!
//! - **`data-dragging:scale-120` é geometria, não transform.** O GPUI não tem `scale` em `div` (o
//!   `TransformationMatrix` só serve pra `svg`/imagem). O polegar cresce de 16 para 19,2px e é
//!   reposicionado em −1,6px nos dois eixos, o que reproduz um crescimento em torno do centro.
//! - **Anel e bisel são BORDA em overlay, não sombra.** O `Window::paint_shadows` do GPUI não recorta
//!   a sombra pra fora do elemento como o CSS faz; usá-la pra filete ou anel lava a peça inteira de
//!   cor. Já custou três defeitos visíveis nesta base.
//! - **Área de clique maior que o trilho.** O trilho tem 4px de altura, e um alvo de 4px é
//!   impraticável com o mouse. Há um retângulo transparente da altura do POLEGAR, centrado no trilho,
//!   que recebe o clique. Não muda o layout (é absoluto) e vem ANTES do polegar, então o polegar
//!   continua vencendo onde os dois se sobrepõem. É adição de usabilidade, não valor portado.
//! - **`not-dark:bg-clip-padding`** não tem equivalente: o fundo do polegar é pintado na border box.
//!   Como o polegar é branco opaco e a borda dele também é opaca, não há diferença visível.
//!
//! # Além da referência
//!
//! [`SliderEvent::Commit`], emitido ao terminar o arraste. O `.tsx` não trata eventos, e um controle
//! que só emite a cada pixel obriga quem escuta a debouncear na mão quando a reação é caríssima.

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, px, App, Bounds, Context, DispatchPhase, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Point, Render, SharedString, Styled, Window,
};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta
// =================================================================================================

struct SliderPalette {
    /// `--input`: o carril, e a borda do polegar no tema claro.
    input: Rgba8,
    /// `--primary`: o indicador.
    primary: Rgba8,
    /// A borda do polegar. No claro é `--input`; no escuro a referência troca pra `--background`.
    thumb_border: Rgba8,
    /// `--ring` já com o alfa do tema: `/24` no claro, `/48` no escuro.
    ring: Rgba8,
    /// A sombra `shadow-xs/5` do polegar.
    shadow: Rgba8,
    /// O texto do [`slider_value`].
    text: Rgba8,
}

const SLIDER_LIGHT: SliderPalette = SliderPalette {
    input: Rgba8(0x0000001a),   // preto 10%
    primary: Rgba8(0x262626ff), // neutral-800
    thumb_border: Rgba8(0x0000001a),
    // neutral-400 a 24%
    ring: Rgba8(0xa3a3a33d),
    shadow: Rgba8(0x0000000d), // preto 5%
    text: Rgba8(0x262626ff),
};

const SLIDER_DARK: SliderPalette = SliderPalette {
    input: Rgba8(0xffffff14),   // branco 8%
    primary: Rgba8(0xf5f5f5ff), // neutral-100
    // `dark:border-background` — a borda vira a cor do fundo, o que "recorta" o polegar branco do
    // trilho claro em vez de contorná-lo.
    thumb_border: Rgba8(0x141414ff),
    // neutral-500 a 48% — no escuro a referência DOBRA o alfa do anel (`ring-ring/48`).
    ring: Rgba8(0x7373737a),
    shadow: Rgba8(0x0000000d),
    text: Rgba8(0xf5f5f5ff),
};

fn palette() -> &'static SliderPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &SLIDER_DARK,
        theme::ThemeMode::Light => &SLIDER_LIGHT,
    }
}

/// O polegar é **branco nos dois temas** (`bg-white`, sem variante escura) — não é `--background`.
/// No escuro é justamente o contraste dele com o fundo que o faz ler como um controle.
const THUMB_FILL: Rgba8 = Rgba8(0xffffffff);

/// O bisel do polegar: preto a 4%, deslocado 1px pra BAIXO, nos dois temas.
///
/// A referência não dá variante escura porque o polegar é branco em ambos — um filete escuro sobre
/// branco funciona igual nos dois. É por isso que aqui NÃO vale o desvio de dobrar o alfa do bisel no
/// escuro, que vale pro `input`, `card`, `button`, `frame` e `toast`: lá o bisel é branco sobre fundo
/// escuro; aqui é preto sobre branco.
const THUMB_BEVEL: Rgba8 = Rgba8(0x0000000a);

/// O deslocamento vertical do bisel, em pixels, com o SINAL da referência (`0 1px`).
///
/// Positivo empurra a sombra pra baixo, o que faz o filete visível ser o de BAIXO. O sinal é a única
/// coisa que decide de que lado o filete aparece, e trocá-lo não quebra nada que compile.
const BEVEL_OFFSET_Y: f32 = 1.0;

// =================================================================================================
// Medidas, todas da referência
// =================================================================================================

/// Diâmetro do polegar — `size-5` com `sm:size-4`, e o `sm:` sempre vale em desktop.
const THUMB: f32 = 16.0;

/// Espessura do trilho — `h-1` (horizontal) / `w-1` (vertical).
const TRACK: f32 = 4.0;

/// Recuo do carril nas pontas — `before:inset-x-0.5`, e o `ms-0.5` do indicador.
const RAIL_INSET: f32 = 2.0;

/// Espessura do anel de foco — `ring-[3px]`.
const RING: f32 = 3.0;

/// Espessura do bisel e da borda do polegar.
const HAIRLINE: f32 = 1.0;

/// Quanto o polegar cresce enquanto arrastado — `data-dragging:scale-120`.
const DRAG_SCALE: f32 = 1.2;

/// Comprimento mínimo do controle — `min-w-44` / `min-h-44`.
const MIN_LENGTH: f32 = 176.0;

/// Opacidade quando desabilitado — `data-disabled:opacity-64`.
const DISABLED_OPACITY: f32 = 0.64;

// =================================================================================================
// Orientação
// =================================================================================================

/// Eixo do controle.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SliderOrientation {
    /// Da esquerda pra direita — o mínimo à esquerda.
    #[default]
    Horizontal,
    /// De baixo pra cima — o mínimo EMBAIXO, que é o que o `mb-0.5` do indicador da referência
    /// revela: ele cresce a partir da base.
    Vertical,
}

impl SliderOrientation {
    fn is_vertical(self) -> bool {
        matches!(self, SliderOrientation::Vertical)
    }
}

// =================================================================================================
// Eventos
// =================================================================================================

/// O que o [`Slider`] emite.
#[derive(Clone, Debug, PartialEq)]
pub enum SliderEvent {
    /// O valor mudou — durante o arraste isto sai a cada pixel.
    Change(Vec<f32>),
    /// O arraste terminou. **Não vem da referência**: existe pra quem reage de forma caríssima
    /// (recalcular um preview, escrever em disco) não ter que debouncear na mão.
    Commit(Vec<f32>),
}

// =================================================================================================
// Aritmética — separada do render pra ser testável sem janela
// =================================================================================================

/// Quantiza um valor no passo, ancorado no mínimo, e apara na faixa.
///
/// Ancorar no mínimo importa: com `min = 3` e `step = 5`, os valores válidos são 3, 8, 13 — e não
/// 0, 5, 10. Quantizar sem âncora deixaria o mínimo inalcançável.
fn quantize(v: f32, min: f32, max: f32, step: f32) -> f32 {
    let v = v.clamp(min.min(max), max.max(min));
    if step <= 0.0 {
        return v;
    }
    let passos = ((v - min) / step).round();
    (min + passos * step).clamp(min, max)
}

/// A fração `[0,1]` que um valor ocupa na faixa. Faixa degenerada (`min == max`) vira 0.
fn fraction(v: f32, min: f32, max: f32) -> f32 {
    let span = max - min;
    if span.abs() < f32::EPSILON {
        0.0
    } else {
        ((v - min) / span).clamp(0.0, 1.0)
    }
}

/// Onde a BORDA de início do polegar assenta, dado o comprimento do trilho.
///
/// É aqui que o `thumbAlignment="edge"` vive: o curso é `comprimento − polegar`, não `comprimento`.
fn thumb_start(v: f32, min: f32, max: f32, length: f32) -> f32 {
    let curso = (length - THUMB).max(0.0);
    curso * fraction(v, min, max)
}

/// O valor que corresponde a uma posição do ponteiro medida a partir do início do trilho.
///
/// O ponteiro comanda o CENTRO do polegar, então o curso útil é deslocado de meio polegar em cada
/// ponta — é o inverso exato de [`thumb_start`].
fn value_at(pos: f32, min: f32, max: f32, length: f32, step: f32) -> f32 {
    let curso = (length - THUMB).max(0.0);
    if curso <= 0.0 {
        return min;
    }
    let f = ((pos - THUMB / 2.0) / curso).clamp(0.0, 1.0);
    quantize(min + f * (max - min), min, max, step)
}

/// Apara o valor do polegar `i` entre os vizinhos, pra a lista nunca sair da ordem.
///
/// Sem isto, arrastar o polegar de baixo por cima do de cima trocaria a ordem dos valores e o
/// indicador (que vai do menor ao maior) piscaria.
fn clamp_between(values: &[f32], i: usize, v: f32, min: f32, max: f32) -> f32 {
    let piso = if i == 0 { min } else { values[i - 1] };
    let teto = if i + 1 >= values.len() {
        max
    } else {
        values[i + 1]
    };
    v.clamp(piso, teto)
}

/// O índice do polegar mais próximo de um valor — quem o clique no trilho move.
///
/// Empate resolve pro de menor índice, que é o comportamento estável (o mesmo clique repetido move
/// sempre o mesmo polegar).
fn nearest(values: &[f32], v: f32) -> usize {
    let mut melhor = 0usize;
    let mut dist = f32::INFINITY;
    for (i, &x) in values.iter().enumerate() {
        let d = (x - v).abs();
        if d < dist {
            dist = d;
            melhor = i;
        }
    }
    melhor
}

// =================================================================================================
// O componente
// =================================================================================================

/// O controle de faixa. Ver o doc do módulo.
pub struct Slider {
    values: Vec<f32>,
    min: f32,
    max: f32,
    step: f32,
    orientation: SliderOrientation,
    disabled: bool,
    focus_handle: FocusHandle,
    /// Qual polegar o teclado move. Só importa com mais de um.
    focused_thumb: usize,
    /// O polegar sendo arrastado, se algum.
    drag: Option<usize>,
    /// Bounds do trilho, medidos por um `canvas` no prepaint — é o que converte pixel em valor.
    track: Bounds<Pixels>,
    id: u64,
}

impl Slider {
    /// Um controle com um valor só.
    pub fn new(value: f32, cx: &mut Context<Self>) -> Self {
        Self::range(vec![value], cx)
    }

    /// Um controle de faixa, com um polegar por valor. A lista é ordenada na entrada: um indicador
    /// que vai do menor ao maior valor não faz sentido com a lista fora de ordem.
    pub fn range(mut values: Vec<f32>, cx: &mut Context<Self>) -> Self {
        if values.is_empty() {
            values.push(0.0);
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Self {
            values,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            orientation: SliderOrientation::default(),
            disabled: false,
            focus_handle: cx.focus_handle(),
            focused_thumb: 0,
            drag: None,
            track: Bounds::default(),
            id: cx.entity_id().as_u64(),
        }
    }

    /// A faixa. Se `min > max` os dois são trocados — uma faixa invertida não é expressável e falhar
    /// em silêncio seria pior.
    pub fn bounds(mut self, min: f32, max: f32) -> Self {
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        self.min = min;
        self.max = max;
        self.reconcile();
        self
    }

    /// O passo. `0` (ou negativo) desliga a quantização e o valor fica contínuo.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step.max(0.0);
        self.reconcile();
        self
    }

    pub fn orientation(mut self, orientation: SliderOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Os valores atuais, em ordem.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// O primeiro valor — a conveniência do caso de valor único.
    pub fn value(&self) -> f32 {
        self.values[0]
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Troca os valores por código. Quantiza, ordena e **não emite**: quem chama já sabe o que pôs, e
    /// emitir daqui criaria laço com quem escuta pra espelhar o estado.
    pub fn set_values(&mut self, values: Vec<f32>, cx: &mut Context<Self>) {
        self.values = values;
        if self.values.is_empty() {
            self.values.push(self.min);
        }
        self.reconcile();
        cx.notify();
    }

    /// Ordena e quantiza — o invariante que todo caminho de escrita tem que restabelecer.
    fn reconcile(&mut self) {
        for v in &mut self.values {
            *v = quantize(*v, self.min, self.max, self.step);
        }
        self.values
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// Escreve o valor de um polegar, aparado entre os vizinhos, e emite se mudou.
    fn set_thumb(&mut self, i: usize, v: f32, cx: &mut Context<Self>) {
        if self.disabled || i >= self.values.len() {
            return;
        }
        let v = clamp_between(
            &self.values,
            i,
            quantize(v, self.min, self.max, self.step),
            self.min,
            self.max,
        );
        if (self.values[i] - v).abs() < f32::EPSILON {
            return;
        }
        self.values[i] = v;
        cx.emit(SliderEvent::Change(self.values.clone()));
        cx.notify();
    }

    /// O comprimento do trilho no eixo em uso.
    fn track_length(&self) -> f32 {
        let s = self.track.size;
        f32::from(if self.orientation.is_vertical() {
            s.height
        } else {
            s.width
        })
    }

    /// A posição do ponteiro ao longo do trilho, contada do início da FAIXA DE VALORES.
    ///
    /// No vertical o eixo se inverte: o mínimo fica embaixo, então a posição é medida da base pra
    /// cima. Errar esse sinal deixa o controle funcionando de cabeça pra baixo — e é o tipo de erro
    /// que passa por revisão de código.
    fn pos_along(&self, mouse: Point<Pixels>) -> f32 {
        if self.orientation.is_vertical() {
            f32::from(self.track.bottom() - mouse.y)
        } else {
            f32::from(mouse.x - self.track.left())
        }
    }

    fn teclado(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let vertical = self.orientation.is_vertical();
        let passo = if self.step > 0.0 {
            self.step
        } else {
            (self.max - self.min) / 100.0
        };
        let grande = passo * 10.0;
        let i = self.focused_thumb.min(self.values.len() - 1);
        let atual = self.values[i];

        let alvo = match e.keystroke.key.as_str() {
            "right" if !vertical => atual + passo,
            "left" if !vertical => atual - passo,
            "up" if vertical => atual + passo,
            "down" if vertical => atual - passo,
            // Nas setas do eixo TRANSVERSAL a referência não define nada; seguir o eixo em uso
            // (cima/direita aumentam) é o que o navegador faz num `input[type=range]`.
            "up" if !vertical => atual + passo,
            "down" if !vertical => atual - passo,
            "right" if vertical => atual + passo,
            "left" if vertical => atual - passo,
            "home" => self.min,
            "end" => self.max,
            "pageup" => atual + grande,
            "pagedown" => atual - grande,
            _ => return,
        };

        crate::focus_ring::keyboard_used(window);
        self.set_thumb(i, alvo, cx);
        cx.emit(SliderEvent::Commit(self.values.clone()));
    }
}

impl EventEmitter<SliderEvent> for Slider {}

impl Focusable for Slider {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Slider {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette();
        let vertical = self.orientation.is_vertical();
        let comprimento = self.track_length();
        let focado = self.focus_handle.is_focused(window) && crate::focus_ring::visible();
        let arrastando = self.drag.is_some();

        // --- Carril ----------------------------------------------------------------------------
        //
        // `before:inset-x-0.5 before:inset-y-0` no horizontal (e o transposto no vertical): ele recua
        // 2px nas PONTAS do eixo em uso e preenche a espessura toda.
        let carril = div()
            .absolute()
            .rounded_full()
            .bg(p.input.hsla())
            .map(|d| {
                if vertical {
                    d.left_0().right_0().top(px(RAIL_INSET)).bottom(px(RAIL_INSET))
                } else {
                    d.top_0().bottom_0().left(px(RAIL_INSET)).right(px(RAIL_INSET))
                }
            });

        // --- Indicador -------------------------------------------------------------------------
        //
        // Vai do MENOR ao MAIOR valor: com um valor só isso é "do início até ele"; com vários, é o
        // trecho entre as pontas da faixa. Começa em `RAIL_INSET`, o `ms-0.5` da referência.
        //
        // Onde ele termina é indiferente — o polegar opaco cobre o ponto de parada em qualquer
        // fração. Ver o doc do módulo.
        let menor = *self.values.first().unwrap_or(&self.min);
        let maior = *self.values.last().unwrap_or(&self.min);
        let de = if self.values.len() > 1 {
            thumb_start(menor, self.min, self.max, comprimento) + THUMB / 2.0
        } else {
            RAIL_INSET
        };
        let ate = thumb_start(maior, self.min, self.max, comprimento) + THUMB / 2.0;
        let extensao = (ate - de).max(0.0);

        let indicador = div()
            .absolute()
            .rounded_full()
            .bg(p.primary.hsla())
            .map(|d| {
                if vertical {
                    d.left_0().right_0().bottom(px(de)).h(px(extensao))
                } else {
                    d.top_0().bottom_0().left(px(de)).w(px(extensao))
                }
            });

        // --- Área de clique ---------------------------------------------------------------------
        //
        // Da altura do POLEGAR, centrada no trilho: 4px de alvo é impraticável. Vem antes dos
        // polegares, então onde os dois se sobrepõem quem recebe o clique é o polegar.
        let sobra = (THUMB - TRACK) / 2.0;
        let area = div()
            .absolute()
            .map(|d| {
                if vertical {
                    d.top_0().bottom_0().left(px(-sobra)).right(px(-sobra))
                } else {
                    d.left_0().right_0().top(px(-sobra)).bottom(px(-sobra))
                }
            })
            .when(!self.disabled, |d| {
                d.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, e: &MouseDownEvent, window, cx| {
                        // Clicar no trilho move o polegar MAIS PRÓXIMO e já começa a arrastá-lo, que
                        // é o que permite clicar e continuar ajustando sem soltar.
                        crate::focus_ring::pointer_used(window);
                        window.focus(&this.focus_handle);
                        let alvo = value_at(
                            this.pos_along(e.position),
                            this.min,
                            this.max,
                            this.track_length(),
                            this.step,
                        );
                        let i = nearest(&this.values, alvo);
                        this.focused_thumb = i;
                        this.drag = Some(i);
                        this.set_thumb(i, alvo, cx);
                        cx.notify();
                    }),
                )
            });

        // --- Polegares --------------------------------------------------------------------------
        let mut trilho = div()
            .relative()
            .flex_grow()
            .map(|d| {
                if vertical {
                    d.w(px(TRACK)).h_full()
                } else {
                    d.h(px(TRACK)).w_full()
                }
            })
            .child(carril)
            .child(indicador)
            .child(area);

        for i in 0..self.values.len() {
            trilho = trilho.child(self.render_thumb(i, comprimento, focado, p, cx));
        }

        // Mede o trilho. Filho absoluto SEM interatividade: não cria hitbox, então não intercepta o
        // clique. E não chama `notify` — só grava, senão seria laço infinito de render.
        let eu = cx.entity();
        trilho = trilho.child(
            canvas(
                move |bounds, _window, cx| {
                    eu.update(cx, |this, _| this.track = bounds);
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );

        // --- Arraste no nível da janela ---------------------------------------------------------
        //
        // Quem arrasta sai dos 16px do polegar no primeiro movimento, então os ouvintes têm que ser
        // da JANELA. Só enquanto há arraste.
        if arrastando {
            trilho = trilho.child(
                canvas(
                    |_, _, _| (),
                    {
                        let entidade = cx.entity();
                        move |_bounds, _, window: &mut Window, _cx: &mut App| {
                        let e2 = entidade.clone();
                        let entidade = entidade.clone();
                        window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            e2.update(cx, |this, cx| {
                                let Some(i) = this.drag else { return };
                                let alvo = value_at(
                                    this.pos_along(e.position),
                                    this.min,
                                    this.max,
                                    this.track_length(),
                                    this.step,
                                );
                                this.set_thumb(i, alvo, cx);
                            });
                            window.refresh();
                        });
                        window.on_mouse_event(move |_e: &MouseUpEvent, phase, window, cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            entidade.update(cx, |this, cx| {
                                if this.drag.take().is_some() {
                                    cx.emit(SliderEvent::Commit(this.values.clone()));
                                    cx.notify();
                                }
                            });
                            window.refresh();
                        });
                        }
                    },
                )
                .absolute()
                .size(px(0.0)),
            );
        }

        // --- Control e Root ---------------------------------------------------------------------
        let controle = div()
            .flex()
            .when(vertical, |d| d.flex_col().h_full().min_h(px(MIN_LENGTH)))
            .when(!vertical, |d| d.w_full().min_w(px(MIN_LENGTH)))
            // `items_center` põe o trilho de 4px no meio da faixa que os polegares ocupam; sem isto
            // o trilho encostaria no topo e os polegares vazariam só pra baixo.
            .when(!vertical, |d| d.items_center())
            .when(vertical, |d| d.justify_center())
            .when(self.disabled, |d| d.opacity(DISABLED_OPACITY))
            .child(trilho);

        div()
            .id(("slider", self.id))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::teclado))
            .flex()
            .flex_col()
            // O polegar tem 16px e o trilho 4: ele sobra 6px pra cada lado. Sem este respiro, o
            // polegar seria cortado por qualquer pai com recorte, e encostaria nos vizinhos.
            .map(|d| {
                if vertical {
                    d.px(px(sobra)).h_full()
                } else {
                    d.py(px(sobra)).w_full()
                }
            })
            .child(controle)
    }
}

impl Slider {
    /// Um polegar: círculo branco com borda, bisel, sombra e — quando arrastado — 20% maior.
    fn render_thumb(
        &self,
        i: usize,
        comprimento: f32,
        focado: bool,
        p: &'static SliderPalette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let vertical = self.orientation.is_vertical();
        let arrastando_este = self.drag == Some(i);

        // `scale-120` por geometria: cresce e recua metade do que cresceu, o que equivale a escalar
        // em torno do centro. Ver o doc do módulo.
        let tamanho = if arrastando_este {
            THUMB * DRAG_SCALE
        } else {
            THUMB
        };
        let recuo = (tamanho - THUMB) / 2.0;

        let inicio = thumb_start(self.values[i], self.min, self.max, comprimento) - recuo;
        // Centra o polegar na espessura do trilho.
        let transversal = (TRACK - tamanho) / 2.0;
        let raio = tamanho / 2.0;

        // `[:has(*:focus-visible),[data-dragging]]:shadow-none` — focado ou arrastando, sem sombra.
        let com_sombra = !focado && !arrastando_este;

        let mut thumb = div()
            .id(("slider-thumb", (self.id << 8) | i as u64))
            .absolute()
            .w(px(tamanho))
            .h(px(tamanho))
            .rounded(px(raio))
            .bg(THUMB_FILL.hsla())
            .border(px(HAIRLINE))
            .border_color(p.thumb_border.hsla())
            .when(com_sombra, |d| {
                d.shadow(vec![gpui::BoxShadow {
                    color: p.shadow.hsla(),
                    offset: gpui::point(px(0.0), px(1.0)),
                    blur_radius: px(2.0),
                    spread_radius: px(0.0),
                }])
            })
            .map(|d| {
                if vertical {
                    d.bottom(px(inicio)).left(px(transversal))
                } else {
                    d.left(px(inicio)).top(px(transversal))
                }
            })
            // O bisel: preto 4% na BASE.
            //
            // A direção vem do sinal do deslocamento: `shadow-[0_1px_...]` empurra a sombra pra
            // baixo, então o filete visível é o de baixo. É a mesma leitura que o `crate::card` já
            // faz (`bevel_dir > 0` → `border_b`), e como aqui a referência NÃO dá variante escura, a
            // base vale nos dois temas. Uma primeira versão pôs no topo — que é o caso do escuro do
            // card (`0 -1px white/6%`), não deste.
            //
            // Cobre a border box, então usa o raio da superfície — ver `crate::card`.
            .child(
                div()
                    .absolute()
                    .inset(px(-HAIRLINE))
                    .rounded(px(raio + HAIRLINE))
                    // O lado sai do SINAL, e não de uma escolha escrita à mão: assim a constante é
                    // o que decide, e o teste que a verifica passa a valer de verdade.
                    .map(|d| {
                        if BEVEL_OFFSET_Y > 0.0 {
                            d.border_b(px(HAIRLINE))
                        } else {
                            d.border_t(px(HAIRLINE))
                        }
                    })
                    .border_color(THUMB_BEVEL.hsla()),
            );

        if focado {
            // `ring-[3px] ring-ring/24` — sem offset nenhum aqui, diferente do `switch`.
            thumb = thumb.child(
                div()
                    .absolute()
                    .inset(px(-RING))
                    .rounded(px(raio + RING))
                    .border(px(RING))
                    .border_color(p.ring.hsla()),
            );
        }

        if !self.disabled {
            thumb = thumb.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _e: &MouseDownEvent, window, cx| {
                    // `stop_propagation` pra a área de clique embaixo não recalcular o valor pela
                    // posição do ponteiro: agarrar o polegar não deve fazê-lo saltar.
                    cx.stop_propagation();
                    crate::focus_ring::pointer_used(window);
                    window.focus(&this.focus_handle);
                    this.focused_thumb = i;
                    this.drag = Some(i);
                    cx.notify();
                }),
            );
        }

        thumb
    }
}

// =================================================================================================
// A leitura do valor
// =================================================================================================

/// O `SliderValue` da referência: `flex justify-end text-sm`.
///
/// A formatação é de quem chama — casas decimais, unidade e separador são decisão de produto, e um
/// componente de biblioteca que escolhe isso por você atrapalha mais do que ajuda.
pub fn slider_value(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .justify_end()
        .text_size(px(14.0))
        .text_color(palette().text.hsla())
        .child(text.into())
}

/// Testes com janela de verdade. Arraste, clique e teclado passam pelo despacho de eventos do GPUI —
/// nada disto é verificável por aritmética, e é justamente onde um slider costuma quebrar.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{
        point, AppContext as _, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Harness {
        slider: Entity<Slider>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            // Largura fixa e generosa: o teste calcula posições a partir do trilho medido, mas uma
            // janela apertada faria o curso ser zero e os asserts passariam por vacuidade.
            div().w(px(400.0)).child(self.slider.clone())
        }
    }

    /// Abre a janela e devolve o slider, o log de eventos e o contexto visual.
    fn abrir(
        cx: &mut TestAppContext,
        montar: impl FnOnce(&mut Context<Slider>) -> Slider + 'static,
    ) -> (Entity<Slider>, Rc<RefCell<Vec<SliderEvent>>>, VisualTestContext) {
        let eventos: Rc<RefCell<Vec<SliderEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_w, cx| {
            let slider = cx.new(montar);
            cx.subscribe(&slider, move |_h, _s, ev: &SliderEvent, _cx| {
                capturados.borrow_mut().push(ev.clone());
            })
            .detach();
            Harness { slider }
        });
        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let slider = vcx.read(|cx| harness.read(cx).slider.clone());
        (slider, eventos, vcx)
    }

    /// O trilho medido — sem isto não há como converter valor em coordenada de tela.
    fn trilho(slider: &Entity<Slider>, vcx: &VisualTestContext) -> Bounds<Pixels> {
        vcx.read(|cx| slider.read(cx).track)
    }

    /// **Clicar no trilho move o valor, e continuar arrastando continua mudando.**
    ///
    /// Cobre o caminho mais arriscado do componente: os ouvintes de mouse são registrados na JANELA
    /// durante o paint, porque quem arrasta sai dos 16px do polegar no primeiro movimento. Se esse
    /// registro falhasse, o polegar grudaria no lugar do clique — e a captura de tela não mostraria
    /// nada de errado.
    #[gpui::test]
    fn arrastar_move_o_valor_e_emite_commit_no_fim(cx: &mut TestAppContext) {
        let (slider, eventos, mut vcx) = abrir(cx, |cx| Slider::new(0.0, cx).bounds(0.0, 100.0));

        let t = trilho(&slider, &vcx);
        assert!(
            f32::from(t.size.width) > THUMB * 2.0,
            "o trilho precisa ter curso, senão o teste não testa nada (deu {})",
            f32::from(t.size.width)
        );

        // Clica no meio: o polegar comanda pelo CENTRO, então o meio do trilho é ~50%.
        let meio = point(t.left() + t.size.width / 2.0, t.center().y);
        vcx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: meio,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.run_until_parked();

        let no_meio = vcx.read(|cx| slider.read(cx).value());
        assert!(
            (no_meio - 50.0).abs() <= 2.0,
            "clique no meio devia dar ~50 (deu {no_meio})"
        );
        assert!(
            vcx.read(|cx| slider.read(cx).drag.is_some()),
            "o clique no trilho já começa o arraste, pra dar pra ajustar sem soltar"
        );

        // Arrasta pra perto do fim, com o ponteiro FORA do polegar — é o que exige ouvinte de janela.
        vcx.simulate_event(MouseMoveEvent {
            position: point(t.right() - px(4.0), t.center().y),
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::default(),
        });
        vcx.run_until_parked();
        let no_fim = vcx.read(|cx| slider.read(cx).value());
        assert!(
            no_fim > 90.0,
            "arrastar pro fim devia subir bem (deu {no_fim})"
        );

        // Solta: sai do arraste e emite Commit uma vez.
        vcx.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: point(t.right() - px(4.0), t.center().y),
            modifiers: Modifiers::default(),
            click_count: 1,
        });
        vcx.run_until_parked();
        assert!(vcx.read(|cx| slider.read(cx).drag.is_none()), "soltou");

        let evs = eventos.borrow();
        assert!(
            evs.iter().any(|e| matches!(e, SliderEvent::Change(_))),
            "houve Change durante o arraste"
        );
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, SliderEvent::Commit(_)))
                .count(),
            1,
            "e exatamente um Commit, no fim"
        );
    }

    /// **O alvo de clique é a altura do POLEGAR, não a do trilho.** O trilho tem 4px; clicar 6px acima
    /// do centro tem que funcionar, senão o controle é impraticável com o mouse.
    #[gpui::test]
    fn o_alvo_de_clique_e_maior_que_o_trilho(cx: &mut TestAppContext) {
        let (slider, _evs, mut vcx) = abrir(cx, |cx| Slider::new(0.0, cx).bounds(0.0, 100.0));
        let t = trilho(&slider, &vcx);

        // 6px acima do centro do trilho: fora dos 4px, dentro dos 16 do polegar.
        let acima = point(t.left() + t.size.width / 2.0, t.center().y - px(6.0));
        vcx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: acima,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.run_until_parked();

        assert!(
            vcx.read(|cx| slider.read(cx).value()) > 40.0,
            "clique 6px fora do trilho tinha que valer"
        );
    }

    /// **O teclado anda no passo, e `Home`/`End` vão às pontas.** E o anel de foco só acende pela
    /// tecla — o clique foca mas não acende, que é o `focus-visible` da casa.
    #[gpui::test]
    fn teclado_anda_no_passo_e_acende_o_anel(cx: &mut TestAppContext) {
        let (slider, _evs, mut vcx) =
            abrir(cx, |cx| Slider::new(50.0, cx).bounds(0.0, 100.0).step(5.0));

        vcx.update(|window, cx| {
            let h = slider.read(cx).focus_handle.clone();
            window.focus(&h);
        });
        // Um clique antes, pra o rastreio de modalidade estar em "ponteiro".
        vcx.update(|window, _cx| crate::focus_ring::pointer_used(window));
        vcx.run_until_parked();
        assert!(!crate::focus_ring::visible(), "ponteiro não acende o anel");

        vcx.simulate_keystrokes("right");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| slider.read(cx).value()), 55.0, "andou um passo");
        assert!(crate::focus_ring::visible(), "a tecla acende o anel");

        vcx.simulate_keystrokes("pageup");
        vcx.run_until_parked();
        assert_eq!(
            vcx.read(|cx| slider.read(cx).value()),
            100.0,
            "PageUp anda 10 passos e apara no máximo"
        );

        vcx.simulate_keystrokes("home");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| slider.read(cx).value()), 0.0);

        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| slider.read(cx).value()), 100.0);
    }

    /// **Desabilitado ignora clique e tecla.** Não é só opacidade: o controle tem que ficar inerte.
    #[gpui::test]
    fn desabilitado_fica_inerte(cx: &mut TestAppContext) {
        let (slider, eventos, mut vcx) = abrir(cx, |cx| {
            Slider::new(30.0, cx).bounds(0.0, 100.0).disabled(true)
        });
        let t = trilho(&slider, &vcx);

        vcx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: point(t.right() - px(4.0), t.center().y),
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.run_until_parked();

        vcx.update(|window, cx| {
            let h = slider.read(cx).focus_handle.clone();
            window.focus(&h);
        });
        vcx.simulate_keystrokes("right");
        vcx.run_until_parked();

        assert_eq!(vcx.read(|cx| slider.read(cx).value()), 30.0, "não mexeu");
        assert!(eventos.borrow().is_empty(), "e não emitiu nada");
    }

    /// **Num controle de faixa, arrastar um polegar por cima do outro para no vizinho.** Sem isto a
    /// lista sairia da ordem e o indicador — que vai do menor ao maior — piscaria.
    #[gpui::test]
    fn na_faixa_um_polegar_para_no_outro(cx: &mut TestAppContext) {
        let (slider, _evs, mut vcx) =
            abrir(cx, |cx| Slider::range(vec![20.0, 60.0], cx).bounds(0.0, 100.0));
        let t = trilho(&slider, &vcx);

        // Agarra pelo trilho perto de 20 (move o polegar 0) e arrasta pro fim.
        let perto_de_20 = point(t.left() + t.size.width * 0.2, t.center().y);
        vcx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: perto_de_20,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseMoveEvent {
            position: point(t.right() - px(2.0), t.center().y),
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::default(),
        });
        vcx.run_until_parked();

        let vs = vcx.read(|cx| slider.read(cx).values().to_vec());
        assert_eq!(vs.len(), 2);
        assert!(vs[0] <= vs[1], "a ordem se mantém: {vs:?}");
        assert_eq!(vs[1], 60.0, "o vizinho não se moveu");
        assert_eq!(vs[0], 60.0, "e o arrastado parou nele");
    }
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// As medidas da referência.
    #[test]
    fn medidas_da_referencia() {
        assert_eq!(THUMB, 16.0, "sm:size-4");
        assert_eq!(TRACK, 4.0, "h-1");
        assert_eq!(RAIL_INSET, 2.0, "inset-x-0.5 e ms-0.5");
        assert_eq!(RING, 3.0, "ring-[3px]");
        assert_eq!(DRAG_SCALE, 1.2, "scale-120");
        assert_eq!(MIN_LENGTH, 176.0, "min-w-44");
        assert_eq!(DISABLED_OPACITY, 0.64, "opacity-64");
    }

    /// **O anel dobra de alfa no escuro** (`ring-ring/24` → `dark:ring-ring/48`), e o polegar é branco
    /// nos DOIS temas. Se alguém trocar o branco por `--background` "por coerência", o polegar
    /// desaparece no tema escuro.
    #[test]
    fn paleta_segue_a_referencia() {
        assert!(
            (SLIDER_DARK.ring.alpha() - 2.0 * SLIDER_LIGHT.ring.alpha()).abs() < 0.02,
            "o alfa do anel dobra no escuro"
        );
        assert_eq!(THUMB_FILL, Rgba8(0xffffffff), "bg-white nos dois temas");
        assert_eq!(
            SLIDER_DARK.thumb_border,
            Rgba8(0x141414ff),
            "dark:border-background"
        );
    }

    /// **O bisel do polegar NÃO segue o desvio de dobrar o alfa no escuro.** Aquele desvio existe
    /// porque um filete branco a 6% desaparece num fundo escuro; aqui o filete é preto sobre um
    /// polegar branco nos dois temas, então o valor da referência serve.
    ///
    /// E a DIREÇÃO é a base, não o topo: `shadow-[0_1px_...]` tem deslocamento POSITIVO, que empurra
    /// pra baixo. Este teste é aritmética sobre a regra, porque o erro que eu cometi foi justamente
    /// trocar o sinal — o topo é o caso do tema escuro do `card` (`0 -1px`), não deste.
    #[test]
    fn o_bisel_do_polegar_e_o_da_referencia() {
        assert!(
            (THUMB_BEVEL.alpha() - 10.0 / 255.0).abs() < 1e-6,
            "preto a 4% (alfa 10), não o dobro"
        );
        assert_eq!(
            BEVEL_OFFSET_Y, 1.0,
            "deslocamento positivo = pra baixo = filete na BASE"
        );
    }

    /// **`thumbAlignment="edge"`: o polegar nunca passa das pontas.** É a geometria que define o
    /// componente — com alinhamento por centro, no mínimo metade do polegar ficaria fora do trilho.
    #[test]
    fn o_polegar_fica_dentro_do_trilho_nos_extremos() {
        let l = 300.0;
        assert_eq!(thumb_start(0.0, 0.0, 100.0, l), 0.0, "no mínimo encosta em 0");
        assert_eq!(
            thumb_start(100.0, 0.0, 100.0, l),
            l - THUMB,
            "no máximo a borda DIREITA encosta no fim"
        );
        assert_eq!(thumb_start(50.0, 0.0, 100.0, l), (l - THUMB) / 2.0);

        // Trilho menor que o polegar: curso zero, e nada de posição negativa.
        assert_eq!(thumb_start(100.0, 0.0, 100.0, 10.0), 0.0);
    }

    /// **Posição e valor são inversos exatos.** Se não fossem, o polegar escaparia do ponteiro durante
    /// o arraste — o defeito clássico de slider.
    #[test]
    fn valor_e_posicao_sao_inversos() {
        let (min, max, l) = (0.0, 100.0, 300.0);
        for v in [0.0, 12.0, 50.0, 87.0, 100.0] {
            let inicio = thumb_start(v, min, max, l);
            // O ponteiro comanda o centro, então a posição equivalente é o início + meio polegar.
            let de_volta = value_at(inicio + THUMB / 2.0, min, max, l, 1.0);
            assert!(
                (de_volta - v).abs() < 0.51,
                "ida e volta de {v} deu {de_volta}"
            );
        }
    }

    /// O ponteiro fora do trilho apara nas pontas em vez de extrapolar.
    #[test]
    fn ponteiro_fora_do_trilho_apara() {
        let (min, max, l) = (0.0, 100.0, 300.0);
        assert_eq!(value_at(-500.0, min, max, l, 1.0), 0.0);
        assert_eq!(value_at(9999.0, min, max, l, 1.0), 100.0);
    }

    /// **A quantização é ancorada no MÍNIMO.** Com `min = 3` e `step = 5`, os válidos são 3, 8, 13 —
    /// quantizar contra o zero deixaria o próprio mínimo inalcançável.
    #[test]
    fn a_quantizacao_ancora_no_minimo() {
        assert_eq!(quantize(3.0, 3.0, 23.0, 5.0), 3.0);
        assert_eq!(quantize(4.0, 3.0, 23.0, 5.0), 3.0);
        assert_eq!(quantize(6.0, 3.0, 23.0, 5.0), 8.0);
        assert_eq!(quantize(23.0, 3.0, 23.0, 5.0), 23.0);
        // Fora da faixa apara; o passo nunca leva além do máximo.
        assert_eq!(quantize(99.0, 3.0, 23.0, 5.0), 23.0);
        // Passo 0 = contínuo.
        assert_eq!(quantize(4.7, 0.0, 10.0, 0.0), 4.7);
    }

    /// **Os polegares não trocam de ordem.** Sem aparar entre vizinhos, arrastar um por cima do outro
    /// inverteria a lista e o indicador (que vai do menor ao maior) piscaria.
    #[test]
    fn polegares_nao_se_atravessam() {
        let v = [20.0, 60.0];
        assert_eq!(clamp_between(&v, 0, 90.0, 0.0, 100.0), 60.0, "para no vizinho");
        assert_eq!(clamp_between(&v, 1, 5.0, 0.0, 100.0), 20.0);
        // As pontas se limitam na faixa.
        assert_eq!(clamp_between(&v, 0, -10.0, 0.0, 100.0), 0.0);
        assert_eq!(clamp_between(&v, 1, 150.0, 0.0, 100.0), 100.0);
    }

    /// O clique move o polegar mais próximo, com empate estável no de menor índice.
    #[test]
    fn o_clique_move_o_mais_proximo() {
        let v = [10.0, 50.0, 90.0];
        assert_eq!(nearest(&v, 12.0), 0);
        assert_eq!(nearest(&v, 45.0), 1);
        assert_eq!(nearest(&v, 100.0), 2);
        assert_eq!(nearest(&v, 30.0), 0, "empate resolve pro menor índice");
    }

    /// Faixa degenerada não divide por zero.
    #[test]
    fn faixa_degenerada_nao_estoura() {
        assert_eq!(fraction(5.0, 5.0, 5.0), 0.0);
        assert_eq!(thumb_start(5.0, 5.0, 5.0, 300.0), 0.0);
        assert_eq!(value_at(150.0, 5.0, 5.0, 300.0, 1.0), 5.0);
    }
}

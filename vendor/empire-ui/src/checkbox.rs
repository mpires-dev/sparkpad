//! `Checkbox` — o quadradinho booleano do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/checkbox.tsx`
//!
//! Complementa o [`crate::switch::Switch`] (o mesmo `bool`, em pílula) e o
//! [`crate::scrub_input::ScrubInput`] (o controle de *float*): aqui se edita **um bit**, com um
//! terceiro estado possível — **indeterminado**, o "alguns dos filhos estão marcados" das árvores de
//! seleção.
//!
//! # Anatomia
//!
//! ```text
//!    ┌────┐                ┌────┐                ┌────┐
//!    │    │ Rótulo         │ ✓  │ Rótulo         │ ─  │ Rótulo
//!    └────┘                └────┘                └────┘
//!     16px                  16px                  16px
//!   desmarcado             marcado             indeterminado
//!   borda --input        --primary cheio      borda --input + traço --foreground
//!   + bisel de 1px       (cobre a borda)      + bisel (o bisel NÃO some aqui)
//!   + shadow-xs/5        sem sombra           + shadow-xs/5
//! ```
//!
//! A caixa tem `size-4.5` (18px) com `sm:size-4` derrubando pra **16px**; como o `sm:` do Tailwind
//! (≥640px) sempre vale numa janela de desktop, o valor efetivo é 16. O ícone é `size-3.5` com
//! `sm:size-3` → **12px**. Mesma leitura que o [`crate::switch`] faz do `--thumb-size`.
//!
//! # Contrato
//!
//! É um `Entity` (view) próprio que mantém o seu estado e **emite** [`CheckboxEvent::Toggle`] com o
//! novo valor a cada interação. Quem usa só assina o evento (`cx.subscribe`) e grava onde quiser —
//! mesmo padrão desacoplado do `Switch`/`ScrubInput` (nada de callback acoplado).
//!
//! ```ignore
//! let cb = cx.new(|cx| Checkbox::new("Fit", false, cx));
//! cx.subscribe(&cb, |_this, _c, ev: &CheckboxEvent, _cx| println!("{ev:?}")).detach();
//! ```
//!
//! # O que o GPUI exigiu adaptar
//!
//! | coss                                              | aqui                                          |
//! |---------------------------------------------------|-----------------------------------------------|
//! | `before:shadow-[0_±1px_…]` (bisel)                | **borda** de 1px num overlay absoluto         |
//! | `focus-visible:ring-2 ring-offset-1`              | dois overlays absolutos concêntricos          |
//! | `focus-visible`                                   | [`crate::focus_ring`] (modalidade da sessão)  |
//! | `transition-shadow`                               | progresso por tempo + `request_animation_frame`|
//!
//! O bisel NÃO pode ser [`gpui::BoxShadow`]: o `Window::paint_shadows` do GPUI não recorta a sombra
//! pra fora do elemento como o CSS faz, e um overlay transparente com sombra viraria uma lavagem de
//! cor sobre a caixa inteira. Já custou defeitos visíveis no [`crate::input`] e no
//! [`crate::card`] — ver [`bevel`].
//!
//! # O que ficou de fora, e por quê
//!
//! - **`not-dark:bg-clip-padding`**: o GPUI pinta o fundo na border box, então no tema claro o
//!   `--input` da borda (preto a 10%) cai sobre o branco do fundo em vez de sobre a superfície de
//!   trás. Sobre uma página branca — o `--background` do coss no claro — o resultado é o MESMO
//!   pixel; a diferença só apareceria com o checkbox sobre uma superfície colorida. É a mesma
//!   omissão declarada no [`crate::slider`].
//! - **O rótulo não vem da referência.** O `checkbox.tsx` é só a caixa; o texto ao lado é um
//!   `<Label>` que o call site compõe. Os valores do rótulo aqui (`text-sm` = 14px, `--foreground`,
//!   `gap-2` = 8px) vêm da convenção do coss pra label de formulário, **não** deste arquivo — ver
//!   [`LABEL_SIZE`].
//! - **Não há variante de tamanho.** O `size-4.5 sm:size-4` da referência é um breakpoint, não uma
//!   prop: existe UM tamanho. Nenhum builder de tamanho foi inventado.
//! - **Não há estado de hover nem `:active`** no `checkbox.tsx` (diferente do `switch`, que tem
//!   `scale-x-110`). Nada foi adicionado "por coerência".

use crate::color::Rgba8;
use crate::theme;
use gpui::{
    div, point, px, svg, App, BoxShadow, Context, CursorStyle, Div, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use std::time::{Duration, Instant};

/// Evento emitido pelo [`Checkbox`] quando o usuário alterna o estado (clique ou espaço/enter com o
/// foco nele). Carrega o **novo valor** (já alternado), no mesmo espírito do
/// [`crate::switch::SwitchEvent`].
///
/// Um checkbox indeterminado que o usuário alterna vira **marcado** (a regra do HTML: `mixed` sobe
/// pra `true`), então o evento é `Toggle(true)` e o indeterminado é limpo — ou seja, receber
/// `Toggle(_)` já implica que o indeterminado saiu.
#[derive(Debug, Clone, Copy)]
pub enum CheckboxEvent {
    /// Novo estado do checkbox após a interação.
    Toggle(bool),
}

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Os utilitários Tailwind do `checkbox.tsx` resolvidos em número, com a paleta `neutral` do Tailwind
// expandida e os `color-mix` do tema já calculados. São os MESMOS tokens que o `crate::input`, o
// `crate::switch` e o `crate::slider` já resolveram — os valores batem com os de lá de propósito,
// porque `--primary`, `--input`, `--ring` e `--background` são tokens do TEMA, não do componente.
//
// Mesma disciplina de cor do resto do crate: TODO valor é `0xRRGGBBAA`, com o byte de alfa, sempre —
// ver [`crate::color`]. Misturar com os tokens de 6 dígitos de [`crate::theme`] desloca os canais e
// produz uma cor completamente diferente sem erro de compilação (`rgba(0xffffff)` é ciano). A única
// ponte é [`crate::color::opaque`].

/// Tokens visuais do checkbox, por tema.
///
/// `pub(crate)` porque o [`crate::radio_group`] usa **esta** paleta, e não uma cópia: as duas
/// referências do coss (`checkbox.tsx` e `radio-group.tsx`) declaram a MESMA lista de utilitários de
/// cor, caractere por caractere — só o raio e o indicador diferem. Duas tabelas com os mesmos quinze
/// valores divergiriam no primeiro refactor.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CheckboxPalette {
    /// `bg-background` — o fundo da caixa quando ela está `data-checked`.
    ///
    /// Marcado o indicador cobre a caixa inteira, então este valor só é observável de raspão; ele
    /// existe porque é o que a referência declara.
    pub(crate) bg: Rgba8,
    /// O fundo da caixa quando NÃO está `data-checked` (desmarcada ou indeterminada).
    ///
    /// No claro é `bg-background` (branco). No escuro a referência troca por
    /// `dark:not-data-checked:bg-input/32` — branco a 8% multiplicado por 32% ≈ **2,5%**, o mesmo
    /// fundo translúcido do campo do [`crate::input`], que faz a caixa "levantar" sobre o painel em
    /// vez de virar um buraco preto.
    pub(crate) bg_unchecked: Rgba8,
    /// `border-input` — a borda de 1px em repouso.
    pub(crate) border: Rgba8,
    /// `data-checked:bg-primary` — o preenchimento do indicador marcado.
    pub(crate) primary: Rgba8,
    /// `text-primary-foreground` — o check sobre o `--primary`.
    pub(crate) primary_fg: Rgba8,
    /// `--foreground` — o traço do indeterminado (`data-indeterminate:text-foreground`) **e** a cor
    /// do rótulo.
    pub(crate) foreground: Rgba8,
    /// `--ring` — cor do anel de foco (`focus-visible:ring-ring`).
    pub(crate) ring: Rgba8,
    /// `--ring-offset-background`, que no coss é o próprio `--background` — a coroa de 1px entre a
    /// caixa e o anel (`ring-offset-1 ring-offset-background`).
    pub(crate) ring_offset: Rgba8,
    /// Cor da sombra externa `shadow-xs/5` (preto a 5%).
    pub(crate) shadow: Rgba8,
    /// Fio de bisel de 1px. Desce no claro, sobe no escuro.
    pub(crate) bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro, `0 1px`), `-1` sobe (escuro, `0 -1px`). O SINAL do
    /// deslocamento da referência é a única coisa que decide de que lado o filete aparece.
    pub(crate) bevel_dir: f32,
    /// `aria-invalid:border-destructive/36` — borda inválida SEM foco.
    pub(crate) invalid_border: Rgba8,
    /// `focus-visible:aria-invalid:border-destructive/64` — borda inválida COM foco.
    pub(crate) invalid_border_focus: Rgba8,
    /// Anel de foco quando inválido: `focus-visible:aria-invalid:ring-destructive/48` no claro,
    /// `dark:aria-invalid:ring-destructive/24` no escuro.
    pub(crate) invalid_ring: Rgba8,
}

/// Tema **claro**.
pub(crate) const CHECKBOX_LIGHT: CheckboxPalette = CheckboxPalette {
    bg: Rgba8(0xffffffff),
    // No claro não existe `not-data-checked:bg-*`: os dois estados são `bg-background`.
    bg_unchecked: Rgba8(0xffffffff),
    border: Rgba8(0x0000001a),   // preto 10%
    primary: Rgba8(0x262626ff),  // neutral-800
    primary_fg: Rgba8(0xfafafaff), // neutral-50
    foreground: Rgba8(0x262626ff), // neutral-800
    ring: Rgba8(0xa3a3a3ff),     // neutral-400
    ring_offset: Rgba8(0xffffffff),
    shadow: Rgba8(0x0000000d), // preto 5%
    bevel: Rgba8(0x0000000a),  // preto 4%
    bevel_dir: 1.0,
    invalid_border: Rgba8(0xef44445c),       // red-500 a 36%
    invalid_border_focus: Rgba8(0xef4444a3), // red-500 a 64%
    invalid_ring: Rgba8(0xef44447a),         // red-500 a 48%
};

/// Tema **escuro**.
pub(crate) const CHECKBOX_DARK: CheckboxPalette = CheckboxPalette {
    // mix(neutral-950 96%, white) = #141414 — o mesmo `--background` do `crate::switch`.
    bg: Rgba8(0x141414ff),
    // `dark:not-data-checked:bg-input/32`: --input escuro é branco a 8%, e o /32 multiplica → ~2,5%.
    bg_unchecked: Rgba8(0xffffff07),
    border: Rgba8(0xffffff14),     // branco 8%
    primary: Rgba8(0xf5f5f5ff),    // neutral-100
    primary_fg: Rgba8(0x262626ff), // neutral-800
    foreground: Rgba8(0xf5f5f5ff), // neutral-100
    ring: Rgba8(0x737373ff),       // neutral-500
    ring_offset: Rgba8(0x141414ff),
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE do coss: o original usa branco a **6%** (alfa 0x0f). Aqui é o DOBRO —
    // alfa 0x1e ≈ 11,8% — por decisão de design: a 6% o filete é imperceptível no nosso fundo
    // escuro. É o MESMO desvio já aplicado no `crate::input`, `crate::card`, `crate::button`,
    // `crate::frame` e `crate::toast`, e vale aqui pela mesma razão: filete BRANCO sobre fundo
    // escuro. (Onde o filete é preto sobre superfície clara — o polegar do `crate::slider` — o
    // desvio NÃO vale.) NÃO "corrija" isto pra 0x0f achando que é erro de porte; se a intenção
    // mudar, mude junto o teste `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
    // destructive escuro = mix(red-500 90%, white) = #f15757.
    invalid_border: Rgba8(0xf157575c),       // 36%
    invalid_border_focus: Rgba8(0xf15757a3), // 64%
    invalid_ring: Rgba8(0xf157573d),         // 24% — o `dark:` derruba o /48 do claro
};

/// A paleta do checkbox no tema corrente.
pub(crate) fn palette() -> &'static CheckboxPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &CHECKBOX_DARK,
        theme::ThemeMode::Light => &CHECKBOX_LIGHT,
    }
}

// =================================================================================================
// Geometria — toda da referência
// =================================================================================================
//
// Os valores marcados `pub(crate)` são lidos também pelo [`crate::radio_group`]: o `radio-group.tsx`
// do coss declara a MESMA caixa (`size-4.5 sm:size-4`), a mesma borda, o mesmo `ring-2` com
// `ring-offset-1` e o mesmo `opacity-64` do `checkbox.tsx`. Um segundo `const BOX: f32 = 16.0` em
// outro arquivo é a definição de dois números que vão divergir. O que NÃO é compartilhado é o que
// realmente difere entre os dois: o raio (aqui 4px, lá `rounded-full`) e o indicador.

/// Lado da caixa — `size-4.5` com `sm:size-4`, e o `sm:` sempre vale em desktop → **16px**.
pub(crate) const BOX: f32 = 16.0;

/// Raio da caixa — `rounded-[.25rem]` = **4px**. Não é um `--radius-*` do coss: a referência crava
/// o valor.
const RADIUS: f32 = 4.0;

/// Espessura da borda e do bisel — `border` = 1px.
pub(crate) const BORDER: f32 = 1.0;

/// Lado do ícone — `size-3.5` com `sm:size-3` → **12px**.
const ICON: f32 = 12.0;

/// Espessura do anel de foco — `focus-visible:ring-2`.
pub(crate) const RING_WIDTH: f32 = 2.0;

/// Folga entre a caixa e o anel — `focus-visible:ring-offset-1`, pintada com
/// `ring-offset-background` (uma coroa da cor do fundo), igual ao [`crate::switch`].
pub(crate) const RING_OFFSET: f32 = 1.0;

/// `data-disabled:opacity-64` — o coss esmaece o CONJUNTO em vez de trocar cor por cor.
pub(crate) const DISABLED_OPACITY: f32 = 0.64;

/// Respiro entre a caixa e o rótulo.
///
/// **Não vem do `checkbox.tsx`** (que não tem rótulo): é o `gap-2` da linha de formulário do coss, e
/// por acaso o valor que este componente já usava. Ver [`LABEL_SIZE`].
pub(crate) const GAP: f32 = 8.0;

/// Corpo do rótulo — `text-sm`.
///
/// ⚠️ **Deduzido, não lido do `checkbox.tsx`.** A referência é só a caixa; o texto ao lado é um
/// `<Label>` composto pelo call site, e o `label.tsx` do coss é `text-sm font-medium`. 14px é o
/// `text-sm` do Tailwind, e é o mesmo corpo que o [`crate::input`], o [`crate::select`] e o
/// [`crate::slider`] já usam pro texto de formulário deste design system.
pub(crate) const LABEL_SIZE: f32 = 14.0;

/// Entrelinha do rótulo — o `--text-sm--line-height` do Tailwind (`calc(1.25/0.875)` em cima de
/// 14px) = **20px**.
///
/// Fixá-la é o que dá altura previsível à linha: sem isso a caixa de 16px e o texto disputam a
/// altura da fileira, e ela muda com a métrica da fonte carregada (é a mesma razão do
/// `.line_height()` do header do [`crate::frame`]).
pub(crate) const LABEL_LINE_HEIGHT: f32 = 20.0;

/// `transition-shadow` — a ÚNICA transição da referência. Ela é declarada na caixa e a propriedade é
/// `box-shadow`, o que no CSS cobre duas coisas de uma vez: a sombra `shadow-xs/5` **e** o anel de
/// foco (um `ring` do Tailwind é `box-shadow`). Sem `duration-*`, vale o default do Tailwind: 150ms.
///
/// O bisel NÃO entra: ele é `box-shadow` de um pseudo-elemento, e `transition` não é herdada por
/// pseudo-elemento — o filete aparece e desaparece instantâneo.
pub(crate) const TRANSITION: Duration = Duration::from_millis(150);

/// `cubic-bezier(0.4, 0, 0.2, 1)` — o `--default-transition-timing-function` do Tailwind, que é a
/// curva das classes `transition-*`, logo a curva do `transition-shadow`.
pub(crate) const EASE_TAILWIND: [f32; 4] = [0.4, 0.0, 0.2, 1.0];

/// Caminho do ícone de **check** (estado marcado).
///
/// A referência desenha o traço à mão: `M5.252 12.7 10.2 18.63 18.748 5.37` com `stroke-width: 3`
/// numa `viewBox` de 24. O ícone nativo do crate é `M5 12.5 l4.5 4.5 L19 6.5`, também com
/// `stroke-width: 3` e `viewBox` 24 — mesma espessura relativa (12,5% da viewBox) e vértices a
/// menos de 1px de distância, a 12px de render. O `iconoir/regular/check.svg` foi considerado e
/// REJEITADO: ele tem `stroke-width: 1.5`, ou seja **metade** do traço da referência.
const ICON_CHECK: &str = "icons/check.svg";

/// Caminho do ícone do estado **indeterminado**.
///
/// A referência desenha `M5.252 12h13.496` com `stroke-width: 3` numa `viewBox` de 24 — uma barra
/// que ocupa 56,2% da largura com 12,5% de espessura. O melhor disponível é o `icons/minus.svg`
/// nativo (`M4 8H12` numa `viewBox` de 16 com `stroke-width: 1.5` → 50% de comprimento e 9,4% de
/// espessura). O `iconoir/regular/minus.svg` foi considerado e REJEITADO: mesmos 50% de comprimento
/// mas só 6,25% de espessura, ou seja metade da nativa e um terço da referência. Não há `minus`
/// solid no Iconoir.
///
/// ⚠️ **Lacuna declarada:** a barra sai ~25% mais fina e ~11% mais curta que a da referência
/// (1,13px × 6,0px contra 1,5px × 6,75px, a 12px de render). Fechar isso exigiria um SVG novo em
/// `assets/icons/`, fora do alcance desta mudança.
const ICON_DASH: &str = "icons/minus.svg";

// =================================================================================================
// Overlays absolutos
// =================================================================================================

/// `(inset, raio)` de um overlay medido a partir da **border box** da caixa: `out` é quantos pixels
/// ele cresce pra fora dela (`0` = coincide com ela).
///
/// ⚠️ O `inset` do GPUI é relativo à **padding box** do pai (é a regra do CSS: o bloco contêiner de
/// um filho absoluto é a padding box do contêiner). Com a caixa tendo 1px de borda, coincidir com a
/// **border** box é `inset: -1px` — e é justamente por isso que o raio de um overlay que cobre a
/// borda é o da SUPERFÍCIE (`RADIUS`) e **não** `RADIUS - BORDER`. É também o que a referência faz:
/// o indicador é `-inset-px rounded-[.25rem]`, o mesmo raio da caixa.
///
/// Parte pura de [`overlay`], pra o teste medir a conversão em vez de confiar nela.
fn overlay_geom(out: f32) -> (f32, f32) {
    (-(BORDER + out), RADIUS + out)
}

/// Um overlay absoluto que coincide com a border box da caixa dilatada em `out` pixels.
fn overlay(out: f32) -> Div {
    let (inset, radius) = overlay_geom(out);
    div()
        .absolute()
        .top(px(inset))
        .left(px(inset))
        .right(px(inset))
        .bottom(px(inset))
        .rounded(px(radius))
}

/// O **fio de bisel** de 1px sobre a borda da caixa.
///
/// No coss é `before:absolute before:inset-0 before:rounded-[3px]` com
/// `box-shadow: 0 ±1px <cor>`: um pseudo-elemento transparente na padding box, cuja sombra só
/// aparece no 1px que ESCAPA — ou seja, o filete cai **sobre a borda**.
///
/// ⚠️ Aqui não pode ser [`gpui::BoxShadow`]. O `Window::paint_shadows` insere a sombra como um
/// retângulo arredondado **completo**, sem recortar a área do próprio elemento (o CSS recorta), e
/// num overlay transparente isso vira uma lavagem da cor sobre a caixa INTEIRA. Então o filete é
/// desenhado como o que ele é: uma **borda de 1px num único lado** de um overlay que cobre a border
/// box (`out = 0`).
///
/// A DIREÇÃO sai do SINAL do deslocamento da referência: `0 1px` (claro) empurra a sombra pra baixo
/// → o filete visível é o de BAIXO (`border_b`); `0 -1px` (escuro) → o de cima (`border_t`). É a
/// mesma regra do [`crate::card`].
fn bevel() -> Div {
    let p = palette();
    let o = overlay(0.0).border_color(p.bevel.hsla());
    if p.bevel_dir > 0.0 {
        o.border_b(px(BORDER))
    } else {
        o.border_t(px(BORDER))
    }
}

/// O **anel de foco** — `focus-visible:ring-2 ring-ring ring-offset-1 ring-offset-background`.
///
/// ⚠️ `ring` não existe no GPUI, e não dá pra fingir com sombra: o `spread_radius` dilata os limites
/// mas MANTÉM o raio, então a curvatura sai errada nas quinas, e num fundo translúcido (que é o caso
/// da caixa vazia no tema escuro) a sombra atravessa e tinge o componente.
///
/// No CSS um `ring` com offset são duas sombras concêntricas: 1px da cor do FUNDO colado no
/// elemento, e 2px da cor do anel por fora dela. Então são dois overlays, com bandas **adjacentes e
/// sem sobreposição**: a coroa vai de 0 a 1 fora da border box, o anel de 1 a 3.
///
/// `k` é a opacidade do conjunto — o anel entra e sai desvanecendo, porque no CSS ele é `box-shadow`
/// e a caixa declara `transition-shadow`.
///
/// ⚠️ A coroa é pintada com `--background`, literalmente como o `ring-offset-background` do
/// Tailwind. Se o checkbox estiver sobre uma superfície que NÃO é `--background` (um card, um
/// popover), aparece um fio de 1px da cor errada em volta dele — e é assim na referência também.
fn ring_overlays(k: f32, invalid: bool) -> [Div; 2] {
    let p = palette();
    let cor = if invalid { p.invalid_ring } else { p.ring };
    [
        overlay(RING_OFFSET)
            .border(px(BORDER))
            .border_color(p.ring_offset.scaled(k)),
        overlay(RING_OFFSET + RING_WIDTH)
            .border(px(RING_WIDTH))
            .border_color(cor.scaled(k)),
    ]
}

// =================================================================================================
// Tempo
// =================================================================================================
//
// ⚠️ `ease`/`elapsed_progress`/`progress` são gêmeos dos do `crate::switch`. A duplicação é
// deliberada e temporária: as funções de lá são privadas do módulo, e esta mudança não tem permissão
// pra tocar em outro arquivo. O lugar certo delas é um `crate::motion` compartilhado (`switch`,
// `input`, `button`, `toast` e este módulo usariam o mesmo) — extrair é um passo separado.
//
// Enquanto esse módulo não existe, `ease` e `progress` são `pub(crate)` e o [`crate::radio_group`]
// chama ESTAS — uma TERCEIRA cópia da mesma bisseção de Bézier seria pior que a segunda.

/// Avalia uma `cubic-bezier(x1, y1, x2, y2)` do CSS em `t`.
///
/// A curva do CSS é paramétrica: `x` e `y` são polinômios de Bézier no parâmetro `u`, e o que se
/// quer é `y` no `u` onde `x(u) == t`. Com `x1`/`x2` em `[0,1]` a curva `x(u)` é monótona, então uma
/// bisseção resolve — 24 passos dão precisão de ~6e-8, muito além do que um pixel percebe.
pub(crate) fn ease(curve: [f32; 4], t: f32) -> f32 {
    let [x1, y1, x2, y2] = curve;
    let t = t.clamp(0.0, 1.0);
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

/// Progresso linear `[0,1]` de uma transição a partir do **decorrido**. Parte pura (sem relógio) de
/// [`progress`], pra ser testável.
fn elapsed_progress(elapsed: Duration, dur: Duration) -> f32 {
    if dur.is_zero() {
        return 1.0;
    }
    (elapsed.as_secs_f32() / dur.as_secs_f32()).clamp(0.0, 1.0)
}

/// Progresso linear `[0,1]` de uma transição que começou em `since`.
///
/// `None` (nunca começou) devolve `1.0` — "já assentada". É o que o PRIMEIRO frame precisa: sem isso
/// todo checkbox da tela animaria a sombra ao abrir a janela.
pub(crate) fn progress(since: Option<Instant>, dur: Duration) -> f32 {
    match since {
        None => 1.0,
        Some(t) => elapsed_progress(t.elapsed(), dur),
    }
}

// =================================================================================================
// Estado visual
// =================================================================================================

/// Os três estados que a referência distingue, na forma em que ela os distingue: pelos atributos
/// `data-unchecked` / `data-checked` / `data-indeterminate` do primitivo.
///
/// O indeterminado **não** é `data-checked` — é o que faz o `data-indeterminate:text-foreground`
/// legível: se o indicador estivesse com `bg-primary`, um traço `--foreground` sobre ele seria
/// invisível (no tema claro os dois tokens são o MESMO neutral-800). Ou seja, indeterminado é a
/// caixa vazia com um traço escuro, e por isso ele mantém a sombra e o bisel que o marcado perde.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Visual {
    /// `data-unchecked` — caixa vazia.
    Unchecked,
    /// `data-checked` — caixa cheia de `--primary`, com o check.
    Checked,
    /// `data-indeterminate` — caixa vazia com o traço `--foreground`.
    Indeterminate,
}

/// Se a caixa está no estado de **repouso**, o único em que a sombra `shadow-xs/5` e o fio de bisel
/// aparecem.
///
/// É literalmente o par de condições da referência, que por acaso é o mesmo pros dois:
///
/// - `[[data-disabled],[data-checked],[aria-invalid]]:shadow-none`
/// - `not-data-disabled:not-data-checked:not-aria-invalid:before:shadow-[…]`
///
/// Note que **indeterminado não está em nenhuma das duas listas** — ele conserva sombra e bisel.
fn resting(disabled: bool, visual: Visual, invalid: bool) -> bool {
    !disabled && visual != Visual::Checked && !invalid
}

// =================================================================================================
// O componente
// =================================================================================================

/// Controle booleano com o visual do coss: caixa de 16px + rótulo opcional.
///
/// # Onde mora o estado de animação
///
/// Nos **campos deste struct**, e não numa tabela `thread_local` indexada por id como no
/// [`crate::button`] e no [`crate::input`]. A diferença é a natureza dos dois: eles são `RenderOnce`
/// (não têm onde guardar nada), e o `Checkbox` é um `Entity` com identidade própria — o relógio é
/// "quando o estado mudou", e o estado já mora aqui.
pub struct Checkbox {
    label: SharedString,
    checked: bool,
    indeterminate: bool,
    disabled: bool,
    invalid: bool,
    /// Id estável deste checkbox (pra `div().id(...)` único na árvore).
    id: u64,
    /// Handle de foco DESTE checkbox — é ele que decide se o anel aparece.
    ///
    /// `Option` só por causa dos testes: `gpui::FocusHandle::new` é `pub(crate)` no gpui, então um
    /// teste unitário (que não tem `App`) não consegue construir um. [`Checkbox::new`] sempre
    /// preenche.
    ///
    /// (Antes o [`Focusable`] daqui devolvia um `cx.focus_handle()` NOVO a cada chamada, ou seja um
    /// handle que não era o do checkbox e nunca podia estar focado.)
    focus_handle: Option<FocusHandle>,
    /// Se a caixa estava em [`resting`] no último render. `None` = ainda não renderizou, e o
    /// primeiro render nunca anima.
    seen_resting: Option<bool>,
    /// Instante da última troca do repouso — o relógio da sombra.
    shadow_at: Option<Instant>,
    /// Se o anel estava aceso no último render (`is_focused` **e** modalidade de teclado, ver
    /// [`crate::focus_ring`]). `None` = ainda não renderizou.
    seen_ring: Option<bool>,
    /// Instante da última troca do anel — o relógio dele.
    focus_at: Option<Instant>,
}

impl Checkbox {
    /// Cria um `Checkbox` com `label` e estado inicial `checked`.
    ///
    /// Rótulo **vazio** (`""`) rende só a caixa, que é exatamente o que o `checkbox.tsx` é — use
    /// isso quando o texto ao lado for montado por quem chama.
    pub fn new(label: impl Into<SharedString>, checked: bool, cx: &mut Context<Self>) -> Self {
        Self {
            label: label.into(),
            checked,
            indeterminate: false,
            disabled: false,
            invalid: false,
            id: cx.entity_id().as_u64(),
            focus_handle: Some(cx.focus_handle()),
            seen_resting: None,
            shadow_at: None,
            seen_ring: None,
            focus_at: None,
        }
    }

    /// Liga o estado **indeterminado** (builder, pra usar no `cx.new`): a caixa fica vazia com um
    /// traço `--foreground`, e o próximo toggle do usuário a leva pra MARCADA.
    ///
    /// Vence o `checked` enquanto estiver ligado — é a regra do `indeterminate` do HTML, onde os
    /// dois são independentes e o indeterminado é o que se vê.
    ///
    /// ```ignore
    /// let cb = cx.new(|cx| Checkbox::new("Todos", false, cx).indeterminate(true));
    /// ```
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// Desabilita (builder): a caixa para de responder, sai da ordem de tabulação e o conjunto
    /// esmaece pra 64%, com cursor de "não permitido".
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marca como **inválido** (builder) — o `aria-invalid` da referência: a borda vira
    /// `--destructive` a 36% (64% com o foco visível), o anel de foco troca de cor, e a sombra e o
    /// bisel somem.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Estado atual.
    pub fn checked(&self) -> bool {
        self.checked
    }

    /// Se está no estado indeterminado.
    pub fn is_indeterminate(&self) -> bool {
        self.indeterminate
    }

    /// Se está desabilitado.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Se está marcado como inválido.
    pub fn is_invalid(&self) -> bool {
        self.invalid
    }

    /// O rótulo atual (vazio = só a caixa).
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// Define o estado de fora (sincronização externa). **Não** emite `Toggle` — evita loops de
    /// feedback com quem assina o evento (mesma regra do `Switch::set_on`).
    ///
    /// Não mexe no indeterminado: quem quer limpar os dois chama os dois setters. Isso é de
    /// propósito — um controle que decide sozinho apagar o "mixed" ao receber um `checked` externo
    /// perde informação que o chamador ainda tinha.
    pub fn set_checked(&mut self, checked: bool, cx: &mut Context<Self>) {
        if self.checked == checked {
            return;
        }
        self.checked = checked;
        cx.notify();
    }

    /// Liga/desliga o indeterminado de fora. **Não** emite `Toggle`.
    pub fn set_indeterminate(&mut self, indeterminate: bool, cx: &mut Context<Self>) {
        if self.indeterminate == indeterminate {
            return;
        }
        self.indeterminate = indeterminate;
        cx.notify();
    }

    /// Habilita/desabilita em runtime.
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        cx.notify();
    }

    /// Marca/desmarca como inválido em runtime.
    pub fn set_invalid(&mut self, invalid: bool, cx: &mut Context<Self>) {
        if self.invalid == invalid {
            return;
        }
        self.invalid = invalid;
        cx.notify();
    }

    /// Define o rótulo de fora.
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        let label = label.into();
        if self.label == label {
            return;
        }
        self.label = label;
        cx.notify();
    }

    /// O estado visual corrente, na taxonomia da referência.
    fn visual(&self) -> Visual {
        if self.indeterminate {
            Visual::Indeterminate
        } else if self.checked {
            Visual::Checked
        } else {
            Visual::Unchecked
        }
    }

    /// Alterna o estado por interação do usuário e **emite** `Toggle(novo)`. Desabilitado, não faz
    /// nada.
    ///
    /// Indeterminado sobe pra **marcado** (a regra do `mixed` do HTML/ARIA) e o indeterminado é
    /// limpo: um terceiro estado que o usuário pudesse REENTRAR por clique não teria como sair de um
    /// ciclo de três com um controle de dois valores.
    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if self.indeterminate {
            self.indeterminate = false;
            self.checked = true;
        } else {
            self.checked = !self.checked;
        }
        cx.emit(CheckboxEvent::Toggle(self.checked));
        cx.notify();
    }

    /// O indicador — `absolute -inset-px rounded-[.25rem]`, ou seja um overlay que cobre a **border
    /// box** da caixa (`out = 0`), com o MESMO raio dela.
    ///
    /// `None` quando desmarcado (`data-unchecked:hidden`).
    ///
    /// Marcado: fundo `--primary` e check em `--primary-foreground`. Indeterminado: **sem** fundo e
    /// traço em `--foreground` — ver [`Visual`] pra por que o indeterminado não é `data-checked`.
    fn indicator(visual: Visual) -> Option<Div> {
        let p = palette();
        let (fill, icone, cor) = match visual {
            Visual::Unchecked => return None,
            Visual::Checked => (Some(p.primary), ICON_CHECK, p.primary_fg),
            Visual::Indeterminate => (None, ICON_DASH, p.foreground),
        };
        let mut el = overlay(0.0).flex().items_center().justify_center();
        if let Some(fill) = fill {
            el = el.bg(fill.hsla());
        }
        // O SVG é usado pelo GPUI como máscara de alfa e pintado com a `text_color` do elemento —
        // a cor declarada dentro do arquivo é irrelevante.
        Some(el.child(
            svg()
                .path(icone)
                .size(px(ICON))
                .flex_none()
                .text_color(cor.hsla()),
        ))
    }
}

impl EventEmitter<CheckboxEvent> for Checkbox {}

impl Render for Checkbox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette();
        let ativo = !self.disabled;
        let visual = self.visual();
        let em_repouso = resting(self.disabled, visual, self.invalid);

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

        // --- Relógios -------------------------------------------------------------------------
        // Nem o foco nem a modalidade chegam por callback: as duas são lidas a cada render. A troca
        // é detectada comparando com o último valor visto, e o PRIMEIRO render nunca anima.
        if self.seen_resting != Some(em_repouso) {
            let primeiro = self.seen_resting.is_none();
            self.seen_resting = Some(em_repouso);
            if !primeiro {
                self.shadow_at = Some(Instant::now());
            }
        }
        if self.seen_ring != Some(anel_aceso) {
            let primeiro = self.seen_ring.is_none();
            self.seen_ring = Some(anel_aceso);
            if !primeiro {
                self.focus_at = Some(Instant::now());
            }
        }

        let sombra_raw = progress(self.shadow_at, TRANSITION);
        let anel_raw = progress(self.focus_at, TRANSITION);
        // Cada progresso é "quanto andou desde a última troca"; virar "quanto do estado NOVO
        // aplicar" é uma inversão quando o destino é apagado.
        let sombra_k = {
            let e = ease(EASE_TAILWIND, sombra_raw);
            if em_repouso {
                e
            } else {
                1.0 - e
            }
        };
        let anel_k = {
            let e = ease(EASE_TAILWIND, anel_raw);
            if anel_aceso {
                e
            } else {
                1.0 - e
            }
        };

        // --- A caixa --------------------------------------------------------------------------
        //
        // `relative` porque indicador, bisel e anel são filhos ABSOLUTOS. Sem `overflow_hidden`: o
        // anel PRECISA sair da caixa, e um recorte o comeria (ver a armadilha no `crate::input`).
        let borda = if self.invalid {
            if anel_aceso {
                p.invalid_border_focus
            } else {
                p.invalid_border
            }
        } else {
            p.border
        };
        let fundo = if visual == Visual::Checked {
            p.bg
        } else {
            p.bg_unchecked
        };
        let mut caixa = div()
            .relative()
            .flex_none()
            .w(px(BOX))
            .h(px(BOX))
            .rounded(px(RADIUS))
            .border(px(BORDER))
            .border_color(borda.hsla())
            .bg(fundo.hsla());

        // `shadow-xs/5` = `0 1px 2px 0 rgb(0 0 0/.05)`.
        //
        // Aqui pode ser sombra de verdade e não overlay: ela é EXTERNA e a caixa tem fundo. No tema
        // claro o fundo é branco opaco, o caso seguro de sempre. No escuro o fundo da caixa vazia é
        // translúcido (~2,5% de branco), então a parte da sombra que cai atrás dela atravessa — 5%
        // de preto × 97,5% de transmissão sobre #141414 é imperceptível, e é o mesmo compromisso
        // que a moldura do `crate::input` já assume.
        if sombra_k > 0.0 {
            caixa = caixa.shadow(vec![BoxShadow {
                color: p.shadow.scaled(sombra_k),
                offset: point(px(0.0), px(1.0)),
                blur_radius: px(2.0),
                spread_radius: px(0.0),
            }]);
        }

        // O bisel é instantâneo (ver [`TRANSITION`]) e vem antes do indicador na ordem de pintura.
        if em_repouso {
            caixa = caixa.child(bevel());
        }
        caixa = caixa.children(Self::indicator(visual));
        // O anel vem por último: é absoluto, então a ordem só decide quem pinta em cima.
        if anel_k > 0.0 {
            caixa = caixa.children(ring_overlays(anel_k, self.invalid));
        }

        // --- A fileira ------------------------------------------------------------------------
        let mut row = div()
            .id(("checkbox", self.id))
            .flex()
            .items_center()
            .gap(px(GAP))
            .cursor(if ativo {
                // Desvio consciente: o primitivo da referência não declara `cursor-pointer`. Aqui
                // segue a convenção da casa, a mesma do `crate::button`, que põe a mãozinha em tudo
                // que é clicável.
                CursorStyle::PointingHand
            } else {
                // `data-disabled:cursor-not-allowed`.
                CursorStyle::OperationNotAllowed
            });

        if ativo {
            // Só um checkbox ATIVO entra na ordem de tabulação: rastrear o foco de um desabilitado o
            // deixaria alcançável por `tab` sem ter o que fazer ali, e sem anel (porque `focado` já
            // é falso) o usuário perderia o cursor de teclado.
            if let Some(h) = self.focus_handle.as_ref() {
                row = row.track_focus(h);
            }
            row = row
                // Área de clique generosa: a fileira INTEIRA (caixa + rótulo) alterna. Isto é nosso,
                // não da referência — que é só a caixa, sem rótulo pra clicar.
                .on_click(cx.listener(|this, _e, _window, cx| this.toggle(cx)))
                // Espaço e enter alternam com o foco na caixa — é o que dá sentido ao anel.
                .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                    if matches!(e.keystroke.key.as_str(), "space" | "enter") {
                        // Chegou aqui = o usuário está no teclado. O `focus_ring::init` já teria
                        // marcado isso; marcar de novo faz a tecla acender o anel mesmo num app que
                        // esqueceu de inicializar o crate.
                        crate::focus_ring::keyboard_used(window);
                        this.toggle(cx);
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_this, _e: &MouseDownEvent, window, _cx| {
                        // O mouse-down é ponteiro: apaga o anel (ver `crate::focus_ring`). Fica aqui
                        // e não no `on_click` porque é o evento mais cedo, e porque um press que
                        // termina fora do controle também mudou a modalidade — é o que o navegador
                        // faz.
                        crate::focus_ring::pointer_used(window);
                    }),
                );
        } else {
            // `data-disabled:opacity-64`: esmaece o conjunto — caixa, ícone e rótulo.
            row = row.opacity(DISABLED_OPACITY);
        }

        row = row.child(caixa);
        // Rótulo vazio = só a caixa, que é o que o `checkbox.tsx` é.
        if !self.label.is_empty() {
            row = row.child(
                div()
                    .text_size(px(LABEL_SIZE))
                    .line_height(px(LABEL_LINE_HEIGHT))
                    .text_color(p.foreground.hsla())
                    .child(self.label.clone()),
            );
        }

        // Enquanto houver transição em curso, pede o próximo frame. É o motor das duas animações
        // deste componente — não há elemento de animação, o progresso vem do tempo decorrido.
        if sombra_raw < 1.0 || anel_raw < 1.0 {
            window.request_animation_frame();
        }

        row
    }
}

impl Focusable for Checkbox {
    /// O handle DESTE checkbox — é o que faz o `.track_focus` e o anel de foco funcionarem.
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.focus_handle
            .clone()
            .unwrap_or_else(|| cx.focus_handle())
    }
}

#[cfg(test)]
// Travar o valor da referência é o PROPÓSITO destes testes: `assert_eq!(BOX, 16.0)` existe pra que
// mexer na constante quebre o teste, não pra calcular nada.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Um `Checkbox` sem `App` — o [`gpui::FocusHandle`] não é construível fora do gpui, e a lógica
    /// de estado não precisa dele.
    fn probe(checked: bool) -> Checkbox {
        Checkbox {
            label: SharedString::new_static("Fit"),
            checked,
            indeterminate: false,
            disabled: false,
            invalid: false,
            id: 0,
            focus_handle: None,
            seen_resting: None,
            shadow_at: None,
            seen_ring: None,
            focus_at: None,
        }
    }

    // --- Estado -------------------------------------------------------------------------------

    #[test]
    fn checked_reflete_inicial() {
        assert!(probe(true).checked());
        assert!(!probe(false).checked());
    }

    /// O toggle inverte — a regra antiga, preservada.
    #[test]
    fn toggle_inverte_o_estado() {
        let mut c = probe(false);
        c.checked = !c.checked;
        assert!(c.checked());
        c.checked = !c.checked;
        assert!(!c.checked());
    }

    /// **Os builders novos têm default igual ao comportamento antigo.** É o contrato desta mudança:
    /// um call site que só chama `new` tem exatamente o checkbox de antes.
    #[test]
    fn builders_novos_sao_opcionais_e_default_e_o_comportamento_antigo() {
        let c = probe(false);
        assert!(!c.is_indeterminate());
        assert!(!c.is_disabled());
        assert!(!c.is_invalid());

        assert!(probe(false).indeterminate(true).is_indeterminate());
        assert!(probe(false).disabled(true).is_disabled());
        assert!(probe(false).invalid(true).is_invalid());
        // E cada um volta atrás.
        assert!(!probe(false).indeterminate(true).indeterminate(false).is_indeterminate());
        assert!(!probe(false).disabled(true).disabled(false).is_disabled());
        assert!(!probe(false).invalid(true).invalid(false).is_invalid());
    }

    /// O indeterminado **vence** o `checked` na leitura visual, e é independente dele no estado —
    /// igual ao `indeterminate` do HTML.
    #[test]
    fn indeterminado_vence_o_checked_no_visual() {
        assert_eq!(probe(false).visual(), Visual::Unchecked);
        assert_eq!(probe(true).visual(), Visual::Checked);
        assert_eq!(probe(false).indeterminate(true).visual(), Visual::Indeterminate);
        assert_eq!(
            probe(true).indeterminate(true).visual(),
            Visual::Indeterminate,
            "indeterminado é o que se VÊ, mesmo com checked ligado"
        );
        // Mas o `checked` embaixo não foi apagado.
        assert!(probe(true).indeterminate(true).checked());
    }

    /// **Sombra e bisel só existem em repouso**, e o repouso é o mesmo par de condições da
    /// referência. O ponto que mais se erra: **indeterminado NÃO é `data-checked`**, então ele
    /// conserva os dois.
    #[test]
    fn repouso_e_a_tabela_da_referencia() {
        // (disabled, visual, invalid) -> repouso?
        let casos = [
            (false, Visual::Unchecked, false, true, "desmarcado: repouso"),
            (false, Visual::Indeterminate, false, true, "indeterminado: TAMBÉM repouso"),
            (false, Visual::Checked, false, false, "data-checked:shadow-none"),
            (true, Visual::Unchecked, false, false, "data-disabled:shadow-none"),
            (false, Visual::Unchecked, true, false, "aria-invalid:shadow-none"),
            (true, Visual::Checked, true, false, "tudo junto"),
        ];
        for (disabled, visual, invalid, esperado, quem) in casos {
            assert_eq!(resting(disabled, visual, invalid), esperado, "{quem}");
        }
    }

    // --- Geometria ----------------------------------------------------------------------------

    /// Os números da referência, resolvidos no breakpoint `sm:` (que numa janela de desktop sempre
    /// vale).
    #[test]
    fn geometria_da_referencia() {
        assert_eq!(BOX, 16.0, "size-4.5 com sm:size-4");
        assert_eq!(RADIUS, 4.0, "rounded-[.25rem]");
        assert_eq!(BORDER, 1.0, "border");
        assert_eq!(ICON, 12.0, "size-3.5 com sm:size-3");
        assert_eq!(RING_WIDTH, 2.0, "focus-visible:ring-2");
        assert_eq!(RING_OFFSET, 1.0, "focus-visible:ring-offset-1");
        assert_eq!(DISABLED_OPACITY, 0.64, "data-disabled:opacity-64");
    }

    /// O ícone cabe na caixa com folga simétrica — 12 dentro de 16 deixa 2px de cada lado, e é isso
    /// que o `items-center justify-center` da referência produz. Se alguém mexer num dos dois sem o
    /// outro, o check encosta na borda.
    #[test]
    fn icone_cabe_centrado_na_caixa() {
        let folga = (BOX - ICON) / 2.0;
        assert_eq!(folga, 2.0);
        assert!(folga >= BORDER, "o ícone não invade a borda");
    }

    /// **A conversão border-box → `inset` do GPUI, medida.**
    ///
    /// O `inset` é relativo à padding box do pai, então um overlay que coincide com a BORDER box
    /// está em `-BORDER` — e o raio dele é o da SUPERFÍCIE, não `RADIUS - BORDER`. Errar isto é o
    /// jeito clássico de o bisel virar uma segunda linha ao lado da borda em vez de cair sobre ela.
    #[test]
    fn overlay_converte_de_border_box_pro_inset_do_gpui() {
        assert_eq!(overlay_geom(0.0), (-1.0, RADIUS), "border box: inset -1, raio da superfície");
        assert_eq!(overlay_geom(RING_OFFSET), (-2.0, 5.0), "coroa");
        assert_eq!(overlay_geom(RING_OFFSET + RING_WIDTH), (-4.0, 7.0), "anel");
        // O raio cresce junto com a dilatação — é o que mantém as curvas concêntricas.
        for out in [0.0, 1.0, 3.0, 7.5] {
            let (inset, raio) = overlay_geom(out);
            assert_eq!(raio - RADIUS, out, "raio acompanha a dilatação (out {out})");
            assert_eq!(-inset - BORDER, out, "inset acompanha a dilatação (out {out})");
        }
    }

    /// As duas bandas do anel são **adjacentes**: a coroa da cor do fundo ocupa de 0 a 1 fora da
    /// border box, o anel de 1 a 3. Sobreposição pintaria a coroa por cima do anel; folga deixaria
    /// uma fresta.
    #[test]
    fn as_bandas_do_anel_sao_adjacentes() {
        let coroa = (0.0, RING_OFFSET);
        let anel = (RING_OFFSET, RING_OFFSET + RING_WIDTH);
        assert_eq!(coroa.1, anel.0, "sem fresta e sem sobreposição");
        assert_eq!(anel.1, 3.0, "o anel para 3px fora da caixa");
    }

    // --- Ícones -------------------------------------------------------------------------------

    /// **Os dois SVGs existem de verdade.** Um caminho inventado não dá erro em lugar nenhum: o
    /// GPUI simplesmente não pinta nada, e o estado marcado fica um quadrado cheio sem check.
    #[test]
    fn os_svgs_dos_dois_estados_existem() {
        for path in [ICON_CHECK, ICON_DASH] {
            let bytes = crate::assets::lookup(path)
                .unwrap_or_else(|| panic!("SVG ausente da tabela de assets: {path}"));
            assert!(!bytes.is_empty(), "SVG vazio: {path}");
        }
    }

    // --- Cores --------------------------------------------------------------------------------

    /// **A convenção de cor da paleta, decodificada de verdade.** Todo valor é `0xRRGGBBAA`; um de
    /// 6 dígitos esquecido aqui vira uma cor completamente diferente sem erro de compilação
    /// (`rgba(0xffffff)` é ciano) — foi o que aconteceu três vezes nesta base.
    #[test]
    fn paleta_decodifica_pras_cores_pretendidas() {
        for (nome, p) in [("claro", CHECKBOX_LIGHT), ("escuro", CHECKBOX_DARK)] {
            let neutro_opaco = |c: Rgba8, quem: &str| {
                let c: gpui::Rgba = c.hsla().into();
                assert_eq!(c.a, 1.0, "{nome}/{quem}: opaco");
                assert!(c.r > 0.05, "{nome}/{quem}: sem canal vermelho — leitura deslocada");
                assert!(
                    (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                    "{nome}/{quem}: é NEUTRO"
                );
            };
            neutro_opaco(p.bg, "background");
            neutro_opaco(p.primary, "primary");
            neutro_opaco(p.primary_fg, "primary-foreground");
            neutro_opaco(p.foreground, "foreground");
            // `--ring`: o sintoma do bug histórico era ele sair teal (r = 0), porque `0xa3a3a3`
            // lido como rgba perde o canal vermelho.
            neutro_opaco(p.ring, "ring");

            // Translúcidos: a borda `--input`, a sombra, o bisel e todo o vermelho de inválido.
            for (quem, c) in [
                ("border", p.border),
                ("shadow", p.shadow),
                ("bevel", p.bevel),
                ("invalid_border", p.invalid_border),
                ("invalid_border_focus", p.invalid_border_focus),
                ("invalid_ring", p.invalid_ring),
            ] {
                assert!(c.alpha() < 1.0, "{nome}/{quem}: é translúcido");
                assert!(c.alpha() > 0.0, "{nome}/{quem}: não é invisível");
            }

            // `ring-offset-background` É o `--background`, o mesmo `bg`. Um par diferente aqui
            // significaria que alguém inventou um token.
            assert_eq!(p.ring_offset, p.bg, "{nome}: a coroa é --background");

            // O vermelho de inválido é vermelho: r bem acima de g e b.
            let d: gpui::Rgba = p.invalid_border.hsla().into();
            assert!(d.r > d.g * 1.5 && d.r > d.b * 1.5, "{nome}: destructive é vermelho");
        }
    }

    /// O par contraste do estado marcado: `--primary` e `--primary-foreground` são opostos em cada
    /// tema, e os temas os INVERTEM. É o que garante que o check apareça sobre o preenchimento.
    #[test]
    fn temas_invertem_o_par_primary_e_seu_foreground() {
        let luma = |c: Rgba8| {
            let c: gpui::Rgba = c.hsla().into();
            c.r + c.g + c.b
        };
        assert!(
            luma(CHECKBOX_LIGHT.primary) < luma(CHECKBOX_LIGHT.primary_fg),
            "claro: caixa escura, check claro"
        );
        assert!(
            luma(CHECKBOX_DARK.primary) > luma(CHECKBOX_DARK.primary_fg),
            "escuro: caixa clara, check escuro"
        );
    }

    /// No tema **claro** o traço do indeterminado (`--foreground`) e o preenchimento do marcado
    /// (`--primary`) são o MESMO neutral-800 — e é essa coincidência que prova que o indeterminado
    /// **não** pode ter `bg-primary`: o traço ficaria invisível. Se alguém "unificar" os dois
    /// estados, este teste explica por que não.
    #[test]
    fn o_indeterminado_nao_pode_ter_fundo_primary() {
        assert_eq!(
            CHECKBOX_LIGHT.foreground, CHECKBOX_LIGHT.primary,
            "no claro --foreground == --primary; traço sobre preenchimento seria invisível"
        );
        // E por isso o indicador do indeterminado sai sem fundo — a caixa continua vazia.
        theme::set_theme(theme::ThemeMode::Light);
        assert!(
            Checkbox::indicator(Visual::Indeterminate).is_some(),
            "indeterminado TEM indicador"
        );
        assert!(
            Checkbox::indicator(Visual::Unchecked).is_none(),
            "data-unchecked:hidden"
        );
        theme::set_theme(theme::ThemeMode::Dark); // não deixa estado vazando pros outros testes
    }

    /// A alfa dos tokens de inválido, na tabela de bytes desta base: 36% = `0x5c`, 64% = `0xa3`,
    /// 48% = `0x7a` (claro) e 24% = `0x3d` (escuro).
    #[test]
    fn alfas_de_invalido_batem_com_a_referencia() {
        let byte = |c: Rgba8| c.0 & 0xff;
        assert_eq!(byte(CHECKBOX_LIGHT.invalid_border), 0x5c, "destructive/36");
        assert_eq!(byte(CHECKBOX_DARK.invalid_border), 0x5c, "destructive/36");
        assert_eq!(byte(CHECKBOX_LIGHT.invalid_border_focus), 0xa3, "destructive/64");
        assert_eq!(byte(CHECKBOX_DARK.invalid_border_focus), 0xa3, "destructive/64");
        assert_eq!(byte(CHECKBOX_LIGHT.invalid_ring), 0x7a, "ring-destructive/48");
        assert_eq!(
            byte(CHECKBOX_DARK.invalid_ring),
            0x3d,
            "dark:aria-invalid:ring-destructive/24 derruba o /48"
        );
    }

    /// O sentido do bisel vem do SINAL do deslocamento da referência: `0 1px` no claro (filete
    /// embaixo), `0 -1px` no escuro (filete em cima). Trocar isto não quebra nada que compile —
    /// então quebra aqui.
    #[test]
    fn sentido_do_bisel_segue_o_sinal_da_referencia() {
        assert!(CHECKBOX_LIGHT.bevel_dir > 0.0, "claro: shadow-[0_1px_…] → border_b");
        assert!(CHECKBOX_DARK.bevel_dir < 0.0, "escuro: shadow-[0_-1px_…] → border_t");
    }

    /// **O desvio consciente do bisel escuro.** A referência pede branco a 6% (`0x0f`); aqui é o
    /// dobro. Filete BRANCO sobre fundo escuro — o caso em que o desvio vale nesta base (no
    /// `crate::slider`, onde o filete é preto sobre um polegar branco, ele NÃO vale). Este teste
    /// existe pra que "corrigir" o valor pra 6% seja uma decisão, não um descuido.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        let claro = CHECKBOX_LIGHT.bevel.alpha();
        assert!((claro - 10.0 / 255.0).abs() < 1e-3, "claro: preto a 4% (0x0a), fiel");
        let escuro = CHECKBOX_DARK.bevel.alpha();
        assert!((escuro - 30.0 / 255.0).abs() < 1e-3, "escuro: branco a ~11,8% (0x1e)");
        assert!((escuro - 2.0 * 15.0 / 255.0).abs() < 1e-3, "é o DOBRO dos 6% da referência");
    }

    /// O fundo da caixa vazia: branco opaco no claro, translúcido no escuro
    /// (`dark:not-data-checked:bg-input/32`). Um fundo opaco no escuro faria a caixa virar um
    /// buraco em vez de levantar sobre o painel.
    #[test]
    fn caixa_vazia_e_translucida_so_no_escuro() {
        assert_eq!(
            CHECKBOX_LIGHT.bg_unchecked, CHECKBOX_LIGHT.bg,
            "no claro não há not-data-checked:bg-*"
        );
        assert!(
            CHECKBOX_DARK.bg_unchecked.alpha() < 0.05,
            "escuro: --input (8%) × /32 ≈ 2,5%"
        );
        assert_eq!(CHECKBOX_DARK.bg.alpha(), 1.0, "mas o `bg-background` é opaco");
    }

    // --- Tempo --------------------------------------------------------------------------------

    /// A única transição da referência, com a única duração dela.
    #[test]
    fn duracao_da_transicao() {
        assert_eq!(TRANSITION, Duration::from_millis(150), "transition-shadow, default 150ms");
    }

    /// O progresso é linear no decorrido e apara nas duas pontas.
    #[test]
    fn progresso_e_linear_e_apara() {
        let cento_e_cinquenta = Duration::from_millis(150);
        assert_eq!(elapsed_progress(Duration::ZERO, cento_e_cinquenta), 0.0);
        assert_eq!(elapsed_progress(Duration::from_millis(75), cento_e_cinquenta), 0.5);
        assert_eq!(elapsed_progress(cento_e_cinquenta, cento_e_cinquenta), 1.0);
        assert_eq!(
            elapsed_progress(Duration::from_secs(9), cento_e_cinquenta),
            1.0,
            "apara em 1"
        );
        assert_eq!(elapsed_progress(cento_e_cinquenta, Duration::ZERO), 1.0, "duração zero");
    }

    /// Sem relógio (`None`) a transição já está assentada — é o que faz o PRIMEIRO frame renderizar
    /// o estado final em vez de todo checkbox da tela animar ao abrir a janela.
    #[test]
    fn transicao_que_nunca_comecou_ja_esta_no_fim() {
        assert_eq!(progress(None, TRANSITION), 1.0);
    }

    /// A curva do Tailwind: passa por (0,0) e (1,1), é monótona e sai mais rápido que linear.
    #[test]
    fn curva_de_easing() {
        assert!(ease(EASE_TAILWIND, 0.0).abs() < 1e-4, "f(0) = 0");
        assert!((ease(EASE_TAILWIND, 1.0) - 1.0).abs() < 1e-4, "f(1) = 1");
        let mut anterior = -1.0;
        for i in 0..=50 {
            let y = ease(EASE_TAILWIND, i as f32 / 50.0);
            assert!(y >= anterior - 1e-5, "monótona em {i}: {y} < {anterior}");
            anterior = y;
        }
        assert!(ease(EASE_TAILWIND, 0.5) > 0.5, "ease-in-out do Tailwind já passou da metade");
        // Aparada fora de [0,1] — o decorrido pode passar de 1 por arredondamento.
        assert_eq!(ease(EASE_TAILWIND, -1.0), ease(EASE_TAILWIND, 0.0));
        assert_eq!(ease(EASE_TAILWIND, 3.0), ease(EASE_TAILWIND, 1.0));
    }
}

/// Testes que precisam de uma **janela de verdade**: os de cima cobrem geometria, cor e a máquina de
/// estados (aritmética pura), mas nada ali prova que o clique chega ao `toggle`, que a tecla chega ao
/// `on_key_down` ou que o `track_focus` está pendurado no elemento certo. Isso só se vê passando
/// eventos reais pelo despacho do GPUI — é o mesmo arranjo do [`crate::switch`].
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{
        canvas, AppContext as _, Bounds, Modifiers, Pixels, TestAppContext, VisualTestContext,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Um harness mínimo que ancora o checkbox no canto da janela. A geometria é FIXA (a caixa tem
    /// 16px no canto superior esquerdo), então o centro dela é conhecido sem medir nada.
    struct Harness {
        cb: gpui::Entity<Checkbox>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().flex().flex_col().child(self.cb.clone())
        }
    }

    /// O centro da CAIXA, em coordenadas da janela.
    ///
    /// A fileira é centrada pelo eixo transversal e o rótulo tem 20px de entrelinha, então a caixa
    /// de 16px fica 2px abaixo do topo — daí o `LABEL_LINE_HEIGHT / 2`, que é o centro da fileira e
    /// também o da caixa.
    fn centro_da_caixa() -> gpui::Point<Pixels> {
        point(px(BOX / 2.0), px(LABEL_LINE_HEIGHT / 2.0))
    }

    /// Um ponto sobre o RÓTULO, à direita da caixa e do respiro.
    fn sobre_o_rotulo() -> gpui::Point<Pixels> {
        point(px(BOX + GAP + 4.0), px(LABEL_LINE_HEIGHT / 2.0))
    }

    /// Abre a janela com um checkbox montado por `montar` e devolve a entidade, os eventos
    /// capturados e o contexto visual.
    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
        montar: impl FnOnce(Checkbox) -> Checkbox + 'static,
    ) -> (gpui::Entity<Checkbox>, Rc<RefCell<Vec<bool>>>, VisualTestContext) {
        let eventos: Rc<RefCell<Vec<bool>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_window, cx| {
            let cb = cx.new(|cx| montar(Checkbox::new("Rótulo", false, cx)));
            cx.subscribe(&cb, move |_this, _c, ev: &CheckboxEvent, _cx| match ev {
                CheckboxEvent::Toggle(v) => capturados.borrow_mut().push(*v),
            })
            .detach();
            Harness { cb }
        });

        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let cb = vcx.read(|cx| harness.read(cx).cb.clone());
        (cb, eventos, vcx)
    }

    /// **A medida que sustenta TODA a geometria de overlay deste componente.**
    ///
    /// [`overlay_geom`] assume que o `inset` de um filho absoluto no GPUI é relativo à **padding
    /// box** do pai (a regra do CSS), e é dessa assunção que saem os `-1` do indicador e do bisel e
    /// os raios de 5 e 7 do anel. Se ela for falsa, TODOS os quatro overlays saem 1px maiores e o
    /// bisel deixa de cair sobre a borda — um erro de 1px que não quebra nada que compile e que só
    /// se vê medindo pixels. Então mede-se aqui.
    ///
    /// Duas afirmações, nesta ordem: (1) o bloco contêiner de um filho absoluto é a padding box
    /// (`inset: 0` → 14×14 deslocado 1px, não 16×16 na origem); (2) o `overlay(0.0)` corrige isso e
    /// coincide EXATAMENTE com a border box da caixa.
    #[gpui::test]
    fn overlay_zero_coincide_com_a_border_box_da_caixa(cx: &mut TestAppContext) {
        #[derive(Default)]
        struct Medidas {
            /// Onde o GPUI põe um filho absoluto com `inset: 0`.
            bloco_conteiner: Option<Bounds<Pixels>>,
            /// Onde o nosso `overlay(0.0)` aterrissa.
            overlay: Option<Bounds<Pixels>>,
        }

        struct Medidor(Rc<RefCell<Medidas>>);

        impl Render for Medidor {
            fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                let (a, b) = (self.0.clone(), self.0.clone());
                // A mesma caixa do `render`: 16×16 com 1px de borda, ancorada na origem da janela.
                div().flex().flex_col().child(
                    div()
                        .relative()
                        .flex_none()
                        .w(px(BOX))
                        .h(px(BOX))
                        .border(px(BORDER))
                        .child(
                            canvas(
                                move |bounds, _w, _cx| a.borrow_mut().bloco_conteiner = Some(bounds),
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                        )
                        .child(overlay(0.0).child(
                            canvas(
                                move |bounds, _w, _cx| b.borrow_mut().overlay = Some(bounds),
                                |_, _, _, _| {},
                            )
                            .size_full(),
                        )),
                )
            }
        }

        let medidas = Rc::new(RefCell::new(Medidas::default()));
        let m = medidas.clone();
        let window = cx.add_window(move |_window, _cx| Medidor(m));
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let medidas = medidas.borrow();
        let bloco = medidas.bloco_conteiner.expect("o canvas do bloco contêiner pintou");
        let over = medidas.overlay.expect("o canvas do overlay pintou");

        // (1) O bloco contêiner é a PADDING box: a borda de 1px o encolhe e o desloca.
        assert_eq!(
            (bloco.origin.x, bloco.origin.y),
            (px(BORDER), px(BORDER)),
            "`inset: 0` começa DEPOIS da borda — é a padding box"
        );
        assert_eq!(
            (bloco.size.width, bloco.size.height),
            (px(BOX - 2.0 * BORDER), px(BOX - 2.0 * BORDER)),
            "e por isso mede 14×14, não 16×16"
        );

        // (2) O `overlay(0.0)` desfaz exatamente isso e volta pra border box da caixa.
        assert_eq!(
            (over.origin.x, over.origin.y),
            (px(0.0), px(0.0)),
            "o overlay começa na quina da caixa"
        );
        assert_eq!(
            (over.size.width, over.size.height),
            (px(BOX), px(BOX)),
            "e cobre os 16×16 inteiros, borda incluída"
        );
    }

    /// **O clique alterna e emite.** É o contrato público do componente, e o único jeito de provar
    /// que o `on_click` está pendurado na fileira é passar um clique de verdade.
    #[gpui::test]
    fn clique_alterna_e_emite(cx: &mut TestAppContext) {
        let (cb, eventos, mut vcx) = abrir(cx, |c| c);

        vcx.simulate_click(centro_da_caixa(), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert!(cb.read(cx).checked(), "o clique marcou"));
        assert_eq!(*eventos.borrow(), vec![true]);

        vcx.simulate_click(centro_da_caixa(), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert!(!cb.read(cx).checked(), "o segundo clique desmarcou"));
        assert_eq!(*eventos.borrow(), vec![true, false]);
    }

    /// **O RÓTULO também alterna.** A área de clique generosa é uma promessa do doc deste módulo; se
    /// alguém mover o `on_click` da fileira pra caixa, o rótulo para de responder e nada mais avisa.
    #[gpui::test]
    fn o_clique_no_rotulo_alterna(cx: &mut TestAppContext) {
        let (cb, eventos, mut vcx) = abrir(cx, |c| c);

        vcx.simulate_click(sobre_o_rotulo(), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert!(cb.read(cx).checked(), "clicar no texto marcou"));
        assert_eq!(*eventos.borrow(), vec![true]);
    }

    /// **Desabilitado não alterna e não emite** — e o `opacity-64` não é o que impede, é o guarda no
    /// `toggle` (mais o fato de os handlers nem serem pendurados). Um checkbox que esmaece mas
    /// continua respondendo é o pior dos dois mundos.
    #[gpui::test]
    fn desabilitado_ignora_o_clique(cx: &mut TestAppContext) {
        let (cb, eventos, mut vcx) = abrir(cx, |c| c.disabled(true));

        vcx.simulate_click(centro_da_caixa(), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert!(!cb.read(cx).checked(), "não alternou"));
        assert!(eventos.borrow().is_empty(), "não emitiu");
    }

    /// **Indeterminado sobe pra marcado num clique, e o indeterminado é limpo.** É a regra do
    /// `mixed` do HTML: o terceiro estado não é reentrante por interação.
    #[gpui::test]
    fn indeterminado_vira_marcado_no_clique(cx: &mut TestAppContext) {
        let (cb, eventos, mut vcx) = abrir(cx, |c| c.indeterminate(true));

        vcx.simulate_click(centro_da_caixa(), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| {
            let c = cb.read(cx);
            assert!(c.checked(), "subiu pra marcado");
            assert!(!c.is_indeterminate(), "e o indeterminado saiu");
        });
        assert_eq!(*eventos.borrow(), vec![true]);

        // Do marcado em diante é um controle de dois estados normal.
        vcx.simulate_click(centro_da_caixa(), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert!(!cb.read(cx).checked(), "e desmarca depois"));
        assert_eq!(*eventos.borrow(), vec![true, false]);
    }

    /// **Espaço e enter alternam com o checkbox focado.** É o que faz o anel de foco significar
    /// algo: sem tecla que alterne, um controle alcançável por `tab` seria um beco sem saída.
    ///
    /// O foco chega pelo clique (o `track_focus` faz o mouse-down focar), que é o jeito de chegar ao
    /// mesmo estado sem depender da ordem de tabulação do harness.
    #[gpui::test]
    fn espaco_e_enter_alternam_com_o_foco(cx: &mut TestAppContext) {
        let (cb, eventos, mut vcx) = abrir(cx, |c| c);

        vcx.simulate_click(centro_da_caixa(), Modifiers::default()); // marca e leva o foco
        vcx.run_until_parked();

        vcx.simulate_keystrokes("space");
        vcx.run_until_parked();
        vcx.read(|cx| assert!(!cb.read(cx).checked(), "espaço desmarcou"));

        vcx.simulate_keystrokes("enter");
        vcx.run_until_parked();
        vcx.read(|cx| assert!(cb.read(cx).checked(), "enter marcou de novo"));

        assert_eq!(*eventos.borrow(), vec![true, false, true], "cada interação emitiu");
    }

    /// **O clique NÃO acende o anel de foco; a tecla acende.**
    ///
    /// Este é o `focus-visible` da referência (ver [`crate::focus_ring`]). O `track_focus` faz o
    /// GPUI focar o checkbox no mouse-down, e o foco em si é desejável — é o que deixa o teclado
    /// continuar de onde o clique parou. O que não é desejável é o ANEL, e gatear só em
    /// `is_focused` acenderia um anel de 2px em todo clique (defeito já relatado nesta base).
    #[gpui::test]
    fn o_clique_nao_acende_o_anel_mas_a_tecla_acende(cx: &mut TestAppContext) {
        let (cb, _eventos, mut vcx) = abrir(cx, |c| c);

        vcx.simulate_click(centro_da_caixa(), Modifiers::default());
        vcx.run_until_parked();

        vcx.update(|window, cx| {
            assert!(
                !crate::focus_ring::visible(),
                "clicar não pode acender o anel (a referência é focus-visible)"
            );
            assert!(
                cb.read(cx)
                    .focus_handle
                    .as_ref()
                    .expect("o `new` sempre preenche")
                    .is_focused(window),
                "mas o foco VAI pro checkbox — é o que faz o teclado continuar dali"
            );
        });

        // Agora pelo teclado: o anel passa a valer.
        vcx.simulate_keystrokes("space");
        vcx.run_until_parked();
        assert!(crate::focus_ring::visible(), "a tecla acende o anel");

        // E um clique depois o apaga de novo — é a modalidade do ÚLTIMO input.
        vcx.simulate_click(centro_da_caixa(), Modifiers::default());
        vcx.run_until_parked();
        assert!(!crate::focus_ring::visible(), "voltar pro mouse apaga o anel");
    }

    /// **O indicador não intercepta o clique.** Ele cobre a caixa INTEIRA quando marcado e é pintado
    /// por cima; na referência é um filho sem hitbox própria. Se ele criasse uma, desmarcar seria
    /// impossível — e o meio dele é exatamente onde o usuário clica.
    #[gpui::test]
    fn o_indicador_nao_intercepta_o_clique(cx: &mut TestAppContext) {
        let (cb, _eventos, mut vcx) = abrir(cx, |c| c);
        vcx.read(|cx| assert!(!cb.read(cx).checked()));

        vcx.simulate_click(centro_da_caixa(), Modifiers::default());
        vcx.run_until_parked();
        vcx.simulate_click(centro_da_caixa(), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert!(!cb.read(cx).checked(), "o clique atravessou o indicador cheio"));
    }

    /// Um checkbox **sem rótulo** é só a caixa — e continua clicável. É o que o `checkbox.tsx` é, e
    /// o caso em que quem chama monta o próprio texto.
    #[gpui::test]
    fn sem_rotulo_a_caixa_sozinha_funciona(cx: &mut TestAppContext) {
        let eventos: Rc<RefCell<Vec<bool>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();
        let window = cx.add_window(move |_window, cx| {
            let cb = cx.new(|cx| Checkbox::new("", false, cx));
            cx.subscribe(&cb, move |_this, _c, ev: &CheckboxEvent, _cx| match ev {
                CheckboxEvent::Toggle(v) => capturados.borrow_mut().push(*v),
            })
            .detach();
            Harness { cb }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let cb = vcx.read(|cx| harness.read(cx).cb.clone());
        vcx.read(|cx| assert!(cb.read(cx).label().is_empty()));

        // Sem rótulo a fileira tem a altura da CAIXA (16), não a da entrelinha (20).
        vcx.simulate_click(point(px(BOX / 2.0), px(BOX / 2.0)), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert!(cb.read(cx).checked()));
        assert_eq!(*eventos.borrow(), vec![true]);
    }
}

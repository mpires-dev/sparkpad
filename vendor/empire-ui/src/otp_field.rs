//! `OTPField` — o **campo de código** (N caixas de um caractere) do `empire-ui`, com o visual do
//! design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/otp-field.tsx`
//!
//! # Anatomia
//!
//! ```text
//!   32   8   32       32   8       12       8   32
//! ┌────┐   ┌────┐   ┌────┐   ▁▁▁▁▁▁▁▁▁   ┌────┐
//! │ 4  │   │ 2  │   │ 9  │      ▔▔▔      │    │   ← `size-8` (32) · gap-2 (8) · separador 12×2
//! └────┘   └────┘   └────┘                └────┘
//!   ▲ rounded-lg (10) · border-input · bg-background · leading == altura da caixa
//! ```
//!
//! Como em todo porte do coss deste crate, os utilitários Tailwind já estão **resolvidos** para o
//! breakpoint `sm:` (≥640px), que numa janela de desktop vale SEMPRE: `size-9 sm:size-8` é **32px**
//! e `size-10 sm:size-9` (variante `lg`) é **36px**.
//!
//! # Contrato
//!
//! É um `Entity` (view) próprio, dono do código digitado, e **emite**
//! [`OTPFieldEvent::Change`] a cada mudança de valor e [`OTPFieldEvent::Complete`] quando as caixas
//! terminam de encher — o mesmo padrão desacoplado do [`crate::switch::Switch`] e do
//! [`crate::checkbox::Checkbox`].
//!
//! ```ignore
//! let otp = cx.new(|cx| OTPField::new(6, cx).groups([3, 3]));
//! cx.subscribe(&otp, |_this, _f, ev: &OTPFieldEvent, _cx| match ev {
//!     OTPFieldEvent::Change(v) => println!("parcial: {v}"),
//!     OTPFieldEvent::Complete(v) => println!("código: {v}"),
//! })
//! .detach();
//! ```
//!
//! # A decisão de arquitetura: UM estado, não N
//!
//! As caixas **não** são N [`gpui_component::input::InputState`] (um por caixa), como o
//! [`crate::input::Input`] usa. São desenho puro sobre **um** modelo: um vetor de dígitos mais um
//! caret. O motivo é que o que o núcleo de edição entrega — IME, clusters de grafema, seleção,
//! undo/redo, soft wrap — é justamente o que um campo de código **não** tem: cada caixa aceita
//! exatamente um dígito ASCII, não há seleção e não há composição. O que sobraria de custo é o que
//! importa aqui: com N estados, colar 6 dígitos e auto-avançar significa coordenar N entidades, N
//! handles de foco e N subscrições, e o auto-avanço passa a depender da ordem em que N campos
//! processam a tecla. Com um estado só, "digitar" é `Vec::insert` e "colar" é um `for`.
//!
//! O preço declarado: **não há IME** (irrelevante pra dígitos), **não há seleção** e **não há caret
//! piscando** — ver "O que ficou de fora".
//!
//! # O modelo de edição
//!
//! É um campo de texto de no máximo N caracteres, desenhado em N caixas. `digits` é o valor
//! (nunca com buracos no meio) e `caret` é a posição de inserção, em `0..=digits.len()`:
//!
//! - **digitar** insere no caret e avança (`auto-avanço`); com o código cheio, a tecla é ignorada;
//! - **backspace** apaga o dígito ANTES do caret e recua; **delete** apaga o de baixo dele;
//! - **setas / home / end** movem o caret, sem passar do último dígito digitado;
//! - **colar** escreve os dígitos do clipboard a partir do caret, **por cima** do que houver;
//! - **clicar** numa caixa põe o caret nela (aparado pelo fim do valor).
//!
//! A caixa realçada (a que recebe o anel de foco) é `min(caret, N-1)`: com o código cheio o caret
//! fica na posição virtual N e o realce continua na última caixa — o mesmo comportamento do
//! `input-otp`, que é o campo de código de referência na web.
//!
//! # O que o GPUI exigiu adaptar
//!
//! | coss                                             | aqui                                            |
//! |--------------------------------------------------|-------------------------------------------------|
//! | `before:shadow-[0_±1px_…]` (bisel)               | borda de 1px num overlay IRMÃO da caixa         |
//! | `focus-visible:ring-[3px]`                       | overlay 3px maior, com borda de 3px             |
//! | `Separator` (`separator.tsx`)                    | inline: um retângulo `rounded-full` de 12×2      |
//! | `has-disabled:opacity-64` no root                | [`gpui::Styled::opacity`] na linha inteira      |
//!
//! # O que ficou de fora (e por quê)
//!
//! - **Caret piscando**: não existe na referência (o `<input>` do navegador desenha o dele), e aqui
//!   seria animação por tempo com pedido de frame. Quem indica a caixa ativa é o anel de foco, que
//!   é o que a referência realça. Declarado, não esquecido.
//! - **`transition-shadow`**: o anel e a sombra de repouso trocam **instantaneamente**, sem o
//!   cross-fade de 150ms que o [`crate::input::Input`] faz. O anel aqui muda de caixa a cada tecla;
//!   um cross-fade por caixa custaria um relógio por caixa e piscaria durante a digitação.
//! - **`z-10` no foco**: no coss ele existe pra o anel de uma caixa passar por cima da vizinha. Com
//!   `gap-2` (8px) e anel de 3px as caixas nunca chegam a se tocar, então não há sobreposição pra
//!   ordenar. Ver [`tests::o_anel_nao_alcanca_a_caixa_vizinha`].
//! - **`not-dark:bg-clip-padding`**: `background-clip` não existe no GPUI (o fundo é pintado na
//!   border box). Só importaria se a borda fosse translúcida sobre um fundo diferente — e é
//!   exatamente o caso do tema claro, onde a borda é preto 10%: a borda do coss no claro fica
//!   ligeiramente mais escura aqui, porque o branco do fundo passa por baixo dela. É a mesma
//!   limitação já declarada no `Input`.
//! - **Letras**: o campo aceita **só dígitos ASCII**. Um código alfanumérico precisaria de uma
//!   política de charset (e de decidir maiúscula/minúscula); não foi pedido e não foi inventado.
//!
//! # `focus-visible` aqui NÃO é modal (e é de propósito)
//!
//! Ver [`OTPField::anel_aceso`]: num `<input>` de texto o navegador casa `:focus-visible` **sempre
//! que o campo tem foco**, inclusive por clique — é por isso que um campo clicado mostra anel. Este
//! componente segue essa regra (e o [`crate::input::Input`], que faz o mesmo), em vez de consultar
//! [`crate::focus_ring::visible`]. Sem isso, um campo clicado ficaria sem NENHUM indicador de qual
//! caixa está ativa, já que não há caret desenhado. A modalidade da sessão continua sendo
//! alimentada por este componente (`pointer_used`/`keyboard_used`), pros vizinhos que a usam.

use crate::color::Rgba8;
use crate::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, point, px, App, BoxShadow, Context, CursorStyle, Div, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, Keystroke, MouseButton, MouseDownEvent,
    ParentElement, Render, SharedString, Styled, Window,
};

// =================================================================================================
// Eventos
// =================================================================================================

/// Evento emitido pelo [`OTPField`].
///
/// `Complete` vem **depois** do `Change` que completou o código, e só na transição — reencher o
/// mesmo código depois de um backspace emite os dois de novo, mas ficar digitando num campo cheio
/// (que ignora a tecla) não emite nada.
#[derive(Debug, Clone)]
pub enum OTPFieldEvent {
    /// O valor mudou. Carrega o código parcial (0..N dígitos).
    Change(String),
    /// Todas as caixas foram preenchidas. Carrega o código completo (N dígitos).
    Complete(String),
}

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Os utilitários do `otp-field.tsx` resolvidos em número, com a paleta `neutral`/`red` do Tailwind
// expandida (`neutral-100 #f5f5f5`, `neutral-400 #a3a3a3`, `neutral-500 #737373`,
// `neutral-800 #262626`, `red-500 #ef4444`) e os `color-mix` do tema já calculados.
//
// Os valores batem com os do [`crate::input`] de propósito: `--input`, `--ring`, `--background`,
// `--foreground` e `--destructive` são tokens do TEMA, não do componente. A struct é própria (e não
// importada de lá) porque a do `Input` é privada e este módulo não mexe naquele arquivo — e porque
// o OTP não tem placeholder, addon nem contador, então metade dos campos de lá não teria dono.
//
// Mesma disciplina de cor do resto do crate: TODO valor é `0xRRGGBBAA`, com o byte de alfa,
// sempre — ver [`crate::color`]. Misturar com os tokens de 6 dígitos de [`crate::theme`] desloca os
// canais e produz outra cor sem erro de compilação (já custou três bugs visíveis nesta base).

/// Tokens visuais do campo de código, por tema.
#[derive(Clone, Copy, Debug)]
struct OtpPalette {
    /// Fundo da caixa. `bg-background` no claro; `dark:bg-input/32` no escuro (translúcido, então a
    /// caixa "levanta" sobre o painel).
    bg: Rgba8,
    /// Cor do dígito (`text-foreground`).
    text: Rgba8,
    /// Borda em repouso (`--input`). **É também a cor do separador** (`bg-input`) — um token só,
    /// uma fonte de verdade só.
    border: Rgba8,
    /// Borda com foco (`focus-visible:border-ring`).
    ring: Rgba8,
    /// Anel de 3px (`focus-visible:ring-ring/24`).
    ring_glow: Rgba8,
    /// Borda inválida SEM foco (`aria-invalid:border-destructive/36`).
    danger_border: Rgba8,
    /// Borda inválida COM foco (`aria-invalid:focus-visible:border-destructive/64`).
    danger_border_focus: Rgba8,
    /// Anel inválido (`destructive/16` no claro, `/24` no escuro).
    danger_glow: Rgba8,
    /// Sombra externa em repouso (`shadow-xs/5`).
    shadow: Rgba8,
    /// Fio de bisel de 1px em repouso (o `before:shadow-[0_±1px_…]`).
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro, filete embaixo), `-1` sobe (escuro, filete em cima).
    /// É o SINAL do deslocamento da referência, e é ele que escolhe o lado da borda em
    /// [`bevel_for`] — pra não haver como o sinal e o lado divergirem.
    bevel_dir: f32,
}

/// Tema **claro**.
const OTP_LIGHT: OtpPalette = OtpPalette {
    bg: Rgba8(0xffffffff),
    text: Rgba8(0x262626ff), // neutral-800
    border: Rgba8(0x0000001a), // black 10%
    ring: Rgba8(0xa3a3a3ff), // neutral-400
    ring_glow: Rgba8(0xa3a3a33d), // 24%
    danger_border: Rgba8(0xef44445c), // 36%
    danger_border_focus: Rgba8(0xef4444a3), // 64%
    danger_glow: Rgba8(0xef444429), // 16%
    shadow: Rgba8(0x0000000d), // black 5%
    bevel: Rgba8(0x0000000a), // black 4% — fiel à referência
    bevel_dir: 1.0,
};

/// Tema **escuro**.
const OTP_DARK: OtpPalette = OtpPalette {
    // `dark:bg-input/32`: o `--input` escuro é branco a 8%, e o `/32` do Tailwind o multiplica
    // (≈2,5%). Mesmo valor do `crate::input`.
    bg: Rgba8(0xffffff07),
    text: Rgba8(0xf5f5f5ff), // neutral-100
    border: Rgba8(0xffffff14), // white 8%
    ring: Rgba8(0x737373ff), // neutral-500
    ring_glow: Rgba8(0x7373733d),
    // destructive escuro = mix(red-500 90%, white) = #f15757.
    danger_border: Rgba8(0xf157575c),
    danger_border_focus: Rgba8(0xf15757a3),
    danger_glow: Rgba8(0xf157573d), // 24% no escuro
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE do coss, o mesmo que o `crate::input`, o `card`, o `button`, o `frame`, o
    // `toast`, o `checkbox` e o `menu` já fazem: a referência usa branco a **6%** (alfa 15); aqui é
    // o DOBRO (alfa 30 ≈ 11,8%), porque a 6% o filete é imperceptível no nosso fundo escuro. NÃO
    // "corrija" pra 0x0f achando que é erro de porte — ver `tests::bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
};

/// A paleta do campo no tema corrente.
fn paleta() -> &'static OtpPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &OTP_DARK,
        theme::ThemeMode::Light => &OTP_LIGHT,
    }
}

// =================================================================================================
// Geometria
// =================================================================================================

/// Respiro entre as caixas — `gap-2`. Vale também entre caixa e separador (é o mesmo `gap` do root).
const GAP: f32 = 8.0;

/// Raio da caixa — `rounded-lg` = `--radius` = `0.625rem` = **10px**.
const RADIUS: f32 = 10.0;

/// Espessura do anel de foco — `focus-visible:ring-[3px]`.
const RING_WIDTH: f32 = 3.0;

/// Largura do separador — `w-3`.
const SEP_W: f32 = 12.0;

/// Altura do separador — `h-0.5`.
const SEP_H: f32 = 2.0;

/// `has-disabled:opacity-64` — o coss esmaece o CONJUNTO em vez de trocar cor por cor.
const DISABLED_OPACITY: f32 = 0.64;

// =================================================================================================
// Tamanho
// =================================================================================================

/// Tamanho das caixas. A referência tem dois: o padrão e o `lg`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum OTPFieldSize {
    /// Padrão — `size-9 sm:size-8`, ou seja **32px** no desktop.
    #[default]
    Md,
    /// Grande — `size-10 sm:size-9`, ou seja **36px** no desktop.
    Lg,
}

impl OTPFieldSize {
    /// Lado da caixa, **borda inclusa** (o `size-*` do Tailwind é box-sizing `border-box`, e o
    /// `.size()` + `.border_1()` do GPUI também): 32 no padrão, 36 no `lg`.
    pub fn box_size(self) -> f32 {
        match self {
            OTPFieldSize::Md => 32.0,
            OTPFieldSize::Lg => 36.0,
        }
    }

    /// Corpo do dígito — `text-base sm:text-sm` (14) no padrão, `text-lg sm:text-base` (16) no `lg`.
    pub fn text_size(self) -> f32 {
        match self {
            OTPFieldSize::Md => 14.0,
            OTPFieldSize::Lg => 16.0,
        }
    }

    /// Entrelinha do dígito — e é **igual ao lado da caixa** por construção, porque é isso que a
    /// referência declara: `leading-9 sm:leading-8` numa caixa `size-9 sm:size-8` (32/32), e
    /// `leading-10 sm:leading-9` numa `size-10 sm:size-9` (36/36).
    ///
    /// ⚠️ Fixar a entrelinha é **obrigatório**: sem `.line_height(…)` o GPUI usa `relative(1.618)`
    /// (a razão de ouro), não o par do Tailwind — um `text-sm` sairia com 22,6px em vez de 20. Aqui
    /// a altura da caixa é fixa, então isso não mudaria o layout: mudaria a POSIÇÃO VERTICAL do
    /// dígito, que é pior porque não aparece em nenhuma medida de caixa.
    ///
    /// É a mesma técnica do [`crate::input`]: com a caixa de linha do tamanho da caixa, o glifo cai
    /// no centro sem depender de padding. Como a caixa mede 32 **com** as duas bordas, a área de
    /// conteúdo tem 30 e a caixa de linha de 32 sobra 1px pra cada lado — o `items_center` a
    /// centraliza, então o centro do glifo coincide com o centro da caixa. (Era a mesma situação no
    /// navegador: `leading-8` num `<input>` de content box 30px.)
    pub fn line_height(self) -> f32 {
        self.box_size()
    }
}

// =================================================================================================
// Desenho de uma caixa (funções livres, testáveis sem construir o componente)
// =================================================================================================

/// Cor da borda por estado, na ordem de especificidade que os seletores do coss impõem:
/// inválido+foco, inválido, foco, repouso.
///
/// `disabled` NÃO entra: no coss o campo desabilitado mantém as cores e o conjunto recebe
/// `opacity-64`. Um par "apagado" de cada token só criaria uma segunda fonte de verdade.
fn border_color_for(invalid: bool, focused: bool) -> Rgba8 {
    let p = paleta();
    match (invalid, focused) {
        (true, true) => p.danger_border_focus,
        (true, false) => p.danger_border,
        (false, true) => p.ring,
        (false, false) => p.border,
    }
}

/// A sombra externa da caixa — `shadow-xs/5`, ou seja `0 1px 2px black/5%`.
///
/// Ela existe **só em repouso**: o coss a apaga no foco (`focus-visible:shadow-none`), no inválido
/// (`aria-invalid:shadow-none`) e no desabilitado (`has-disabled:…:shadow-none`). É a metade fácil
/// de esquecer — uma caixa focada com sombra de repouso E anel fica com um contorno duplo.
///
/// Sombra EXTERNA é o único uso seguro de sombra nesta base: o `Window::paint_shadows` do GPUI não
/// recorta a sombra pra fora do elemento, mas sobre um fundo opaco ninguém vê. Nada de bisel/anel
/// por sombra — ver [`bevel_for`] e [`ring_overlay`].
fn shadow_stack_for(disabled: bool, focused: bool, invalid: bool) -> Vec<BoxShadow> {
    if disabled || focused || invalid {
        return Vec::new();
    }
    vec![BoxShadow {
        color: paleta().shadow.hsla(),
        offset: point(px(0.0), px(1.0)),
        blur_radius: px(2.0),
        spread_radius: px(0.0),
    }]
}

/// O **fio de bisel** de 1px sobreposto à caixa, ou `None` quando ele não deve aparecer.
///
/// ⚠️ **A técnica do coss não traduz pro GPUI.** Lá é um pseudo-elemento transparente sobre a
/// padding box com `box-shadow: 0 ±1px <cor>`: só o filete que ESCAPA fica visível, porque o CSS
/// nunca pinta a sombra embaixo da border box de quem a projeta. O `Window::paint_shadows` do GPUI
/// insere a sombra como um retângulo arredondado **completo**, sem recortar a área do próprio
/// elemento — num overlay transparente isso lava a caixa inteira de cor. Então o filete é desenhado
/// como o que ele é: uma **borda de 1px num único lado** de um overlay.
///
/// Dois detalhes que já custaram defeito visível nesta base:
///
/// 1. **O raio é o da SUPERFÍCIE (10), não `raio − 1`.** O `calc(--radius-lg - 1px)` da referência
///    descreve um pseudo-elemento na *padding* box; o nosso overlay é IRMÃO da caixa com `inset: 0`,
///    então coincide com a *border* box dela e usa o raio dela. (E sendo irmão ele não é recortado
///    por nenhum `overflow_hidden`.)
/// 2. **O lado sai do SINAL do deslocamento**, via [`OtpPalette::bevel_dir`]: `0 1px` (claro) empurra
///    pra baixo → o filete visível é o de **baixo**; `0 -1px` (escuro) → o de **cima**.
///
/// Some quando a caixa está focada, inválida ou desabilitada — é o
/// `not-focus-visible:not-aria-invalid:before:shadow-…` mais o `has-disabled:…:before:shadow-none!`.
fn bevel_for(disabled: bool, focused: bool, invalid: bool) -> Option<Div> {
    if disabled || focused || invalid {
        return None;
    }
    let p = paleta();
    let overlay = div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .rounded(px(RADIUS))
        .border_color(p.bevel.hsla());
    Some(if p.bevel_dir > 0.0 {
        overlay.border_b_1()
    } else {
        overlay.border_t_1()
    })
}

/// O anel de foco — `focus-visible:ring-[3px] ring-ring/24` (ou `ring-destructive/16|24` quando
/// inválido).
///
/// ⚠️ **Não é uma sombra**, pelos dois motivos que já causaram defeito no campo de texto: a sombra
/// do GPUI é pintada como um retângulo CHEIO atrás do elemento (e o fundo da caixa é translúcido no
/// tema escuro, então o anel atravessaria e tingiria a caixa), e o `spread_radius` dilata os limites
/// **mantendo o raio**, o que deixa as quinas com a curvatura errada.
///
/// Um `ring` do Tailwind é geometricamente um anel: a forma dilatada em 3px, com o raio externo
/// crescendo junto. Então é isso: um overlay 3px maior de cada lado, borda de 3px, raio `10 + 3`.
fn ring_overlay(invalid: bool) -> Div {
    let p = paleta();
    let cor = if invalid { p.danger_glow } else { p.ring_glow };
    div()
        .absolute()
        .top(px(-RING_WIDTH))
        .left(px(-RING_WIDTH))
        .right(px(-RING_WIDTH))
        .bottom(px(-RING_WIDTH))
        .rounded(px(RADIUS + RING_WIDTH))
        .border_3()
        .border_color(cor.hsla())
}

/// O separador entre grupos — `rounded-full bg-input`, 12×2.
///
/// ⚠️ Na referência ele vem do `separator.tsx` do coss, que **não foi portado** pra este crate. São
/// duas classes (`rounded-full bg-input` + a geometria `h-0.5 w-3`), então está inline aqui; um
/// `Separator` avulso continua não existindo no `empire-ui`.
fn separador() -> Div {
    div()
        .flex_none()
        .w(px(SEP_W))
        .h(px(SEP_H))
        .rounded_full()
        .bg(paleta().border.hsla())
}

// =================================================================================================
// Teclas
// =================================================================================================

/// O dígito que esta tecla produz, ou `None`.
///
/// Rejeita qualquer combinação com `cmd`/`ctrl`/`alt`/`fn`: sem isso, `cmd-1` (trocar de aba, num
/// app hospedeiro) digitaria um "1" no campo.
///
/// Lê `key_char` (o caractere que a tecla produziria) e cai pro `key` quando ele não vem — é o caso
/// do [`Keystroke::parse`], que os testes usam e que não preenche `key_char`.
fn digito(k: &Keystroke) -> Option<char> {
    let m = &k.modifiers;
    if m.platform || m.control || m.alt || m.function {
        return None;
    }
    let s = k.key_char.as_deref().unwrap_or(k.key.as_str());
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_digit() => Some(c),
        _ => None,
    }
}

/// Se esta tecla é o **colar** da plataforma: `cmd-v` no macOS, `ctrl-v` no resto.
///
/// É verificado à mão (e não por uma ação vinculada) porque o campo trata a tecla ele mesmo, sem
/// registrar bindings globais — uma lib não deve mexer no mapa de teclas de quem a usa. O efeito
/// colateral declarado: se o app hospedeiro vincular `cmd-v` a uma ação num contexto que envolva
/// este campo, a ação ganha e este handler não vê a tecla.
fn e_colar(k: &Keystroke) -> bool {
    if k.key != "v" {
        return false;
    }
    let m = &k.modifiers;
    if cfg!(target_os = "macos") {
        m.platform && !m.control && !m.alt
    } else {
        m.control && !m.platform && !m.alt
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// Campo de código: N caixas de um dígito, com auto-avanço, backspace, setas e colar.
///
/// Ver o doc do módulo pro modelo de edição e pras decisões de porte.
pub struct OTPField {
    /// Os dígitos já digitados, em ordem. Nunca tem buraco no meio e nunca passa de
    /// [`OTPField::boxes`].
    digits: Vec<char>,
    /// Posição de inserção, em `0..=digits.len()`. A caixa realçada é `min(caret, N-1)`.
    caret: usize,
    /// Quantas caixas em cada grupo; os grupos são separados por um [`separador`]. Nunca vazio, e
    /// nunca com um zero — é a ÚNICA fonte da contagem de caixas (`N` é a soma), pra não existir um
    /// `len` que discorde dos grupos.
    groups: Vec<usize>,
    size: OTPFieldSize,
    disabled: bool,
    invalid: bool,
    /// Id estável (pra `div().id(...)` único na árvore).
    id: u64,
    /// Handle de foco DESTE campo — um só pro conjunto, como num `<input>`: o campo é UMA parada de
    /// tabulação, e as setas navegam por dentro.
    ///
    /// `Option` só por causa dos testes unitários: `gpui::FocusHandle::new` é `pub(crate)` no gpui,
    /// então um teste sem `App` não consegue construir um. [`OTPField::new`] sempre preenche.
    focus_handle: Option<FocusHandle>,
}

impl OTPField {
    /// Cria um campo com `len` caixas, sem separador. `len` é elevado pra 1 se vier zero — um campo
    /// de código sem caixa nenhuma não tem representação visual nem comportamento.
    pub fn new(len: usize, cx: &mut Context<Self>) -> Self {
        Self {
            digits: Vec::new(),
            caret: 0,
            groups: vec![len.max(1)],
            size: OTPFieldSize::default(),
            disabled: false,
            invalid: false,
            id: cx.entity_id().as_u64(),
            focus_handle: Some(cx.focus_handle()),
        }
    }

    /// Divide as caixas em grupos separados por um filete — `groups([3, 3])` é o clássico `000-000`.
    ///
    /// **Os grupos são a fonte da contagem**: o total de caixas passa a ser a soma deles (aqui, 6),
    /// substituindo o `len` do [`Self::new`]. É por isso que não existe um estado inconsistente
    /// "6 caixas divididas em 3+4". Zeros são descartados; uma lista vazia (ou só de zeros) cai numa
    /// caixa.
    pub fn groups(mut self, groups: impl Into<Vec<usize>>) -> Self {
        let mut g: Vec<usize> = groups.into().into_iter().filter(|n| *n > 0).collect();
        if g.is_empty() {
            g.push(1);
        }
        self.groups = g;
        self.normalizar();
        self
    }

    /// Tamanho das caixas.
    pub fn size(mut self, size: OTPFieldSize) -> Self {
        self.size = size;
        self
    }

    /// Desabilita: o campo não recebe foco nem tecla, e o conjunto esmaece pra 64% (sem sombra e
    /// sem bisel).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marca o campo como **inválido**: bordas em `--destructive`, sem sombra e sem bisel.
    ///
    /// É o `aria-invalid` da referência, que lá vai em cada `<input>` — aqui vale pro conjunto,
    /// porque um código é válido ou inválido inteiro, nunca caixa por caixa.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// O código digitado (0..N dígitos, sem buracos).
    pub fn value(&self) -> String {
        self.digits.iter().collect()
    }

    /// Se todas as caixas estão preenchidas.
    pub fn is_complete(&self) -> bool {
        self.digits.len() == self.boxes()
    }

    /// Quantas caixas o campo tem (a soma dos grupos).
    ///
    /// Chama-se `boxes` e não `len` de propósito: `len` sem `is_empty` é apontamento de clippy, e um
    /// `is_empty` aqui seria ambíguo entre "sem caixas" (impossível) e "sem dígitos".
    pub fn boxes(&self) -> usize {
        self.groups.iter().sum()
    }

    /// Se está desabilitado.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Se está marcado como inválido.
    pub fn is_invalid(&self) -> bool {
        self.invalid
    }

    /// Define o valor de fora (sincronização externa: um código vindo de um deep link, um
    /// autofill). Descarta o que não é dígito, apara em N e põe o caret no fim.
    ///
    /// **Não** emite [`OTPFieldEvent`] — evita loops de feedback com quem assina, mesma regra do
    /// `Switch::set_on` e do `Checkbox::set_checked`.
    pub fn set_value(&mut self, value: impl AsRef<str>, cx: &mut Context<Self>) {
        self.digits = value
            .as_ref()
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(self.boxes())
            .collect();
        self.caret = self.digits.len();
        cx.notify();
    }

    /// Esvazia o campo e volta o caret pra primeira caixa. Não emite evento (ver [`Self::set_value`]).
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        if self.digits.is_empty() && self.caret == 0 {
            return;
        }
        self.digits.clear();
        self.caret = 0;
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

    /// Marca/desmarca como inválido em runtime — o caminho normal, já que a validade costuma vir
    /// da resposta do servidor.
    pub fn set_invalid(&mut self, invalid: bool, cx: &mut Context<Self>) {
        if self.invalid == invalid {
            return;
        }
        self.invalid = invalid;
        cx.notify();
    }

    // --- O modelo de edição -------------------------------------------------------------------
    //
    // Todas as funções abaixo são PURAS em relação ao GPUI (não tocam em `cx`, não notificam, não
    // emitem) e devolvem "mudou algo?". É o que as torna testáveis sem abrir janela, e é o que
    // mantém o handler de tecla com uma responsabilidade só: traduzir tecla em operação.

    /// Restaura os invariantes depois de uma mudança de grupos: nada de dígito além de N, nada de
    /// caret além do último dígito.
    fn normalizar(&mut self) {
        self.digits.truncate(self.boxes());
        self.caret = self.caret.min(self.digits.len());
    }

    /// A caixa realçada: `min(caret, N-1)`. Com o código cheio o caret está na posição virtual N e
    /// o realce fica na última caixa (comportamento do `input-otp`).
    fn caixa_ativa(&self) -> usize {
        self.caret.min(self.boxes().saturating_sub(1))
    }

    /// Digita um dígito no caret e **auto-avança**. Com o código cheio, ignora a tecla (em vez de
    /// sobrescrever a última caixa, que é o que aconteceria se o caret parasse em `N-1`).
    fn digitar(&mut self, c: char) -> bool {
        if self.digits.len() >= self.boxes() {
            return false;
        }
        self.digits.insert(self.caret, c);
        self.caret += 1;
        true
    }

    /// Backspace: apaga o dígito ANTES do caret e recua. No meio do código o resto desliza pra
    /// esquerda — é um campo de texto, não N caixas independentes, então não sobra buraco.
    fn apagar_atras(&mut self) -> bool {
        if self.caret == 0 {
            return false;
        }
        self.caret -= 1;
        self.digits.remove(self.caret);
        true
    }

    /// Delete: apaga o dígito debaixo do caret, sem mexer nele.
    fn apagar_frente(&mut self) -> bool {
        if self.caret >= self.digits.len() {
            return false;
        }
        self.digits.remove(self.caret);
        true
    }

    /// Move o caret pra `i`, aparado pelo fim do valor (não se navega pra dentro de caixa vazia
    /// que não seja a próxima a receber dígito — é o que impede um buraco no meio).
    fn ir_para(&mut self, i: usize) -> bool {
        let alvo = i.min(self.digits.len());
        if alvo == self.caret {
            return false;
        }
        self.caret = alvo;
        true
    }

    /// Cola: escreve os dígitos do texto a partir do caret, **por cima** do que houver, e avança o
    /// caret pro fim do que foi colado. Não-dígitos são descartados (então `"123-456"` cola 6
    /// dígitos), e o que passar da última caixa é cortado.
    ///
    /// Sobrescrever (e não inserir) é a escolha certa aqui: "colar distribui os dígitos pelas
    /// caixas" — e é o que faz colar um código novo sobre um código errado funcionar, em vez de não
    /// fazer nada por falta de espaço.
    fn colar(&mut self, texto: &str) -> bool {
        let espaco = self.boxes() - self.caret;
        let novos: Vec<char> = texto
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(espaco)
            .collect();
        if novos.is_empty() {
            return false;
        }
        for (k, c) in novos.iter().enumerate() {
            let i = self.caret + k;
            if i < self.digits.len() {
                self.digits[i] = *c;
            } else {
                self.digits.push(*c);
            }
        }
        self.caret += novos.len();
        true
    }

    // --- Interação ----------------------------------------------------------------------------

    /// Se o anel de foco está aceso.
    ///
    /// **Não consulta [`crate::focus_ring::visible`], e isso é deliberado.** A regra do
    /// `focus-visible` do navegador tem uma exceção pra campos de texto: um `<input>` casa
    /// `:focus-visible` sempre que tem foco, inclusive quando o foco veio de um clique. É por isso
    /// que clicar num campo de texto mostra anel e clicar num botão não. Como a referência é um
    /// `<input>`, é essa a regra fiel — e é a mesma que o [`crate::input::Input`] aplica.
    ///
    /// Aqui há um segundo motivo, mais forte: **não existe caret desenhado** (ver o doc do módulo).
    /// Se o anel dependesse da modalidade, um campo clicado ficaria sem nenhuma indicação de qual
    /// caixa recebe o próximo dígito.
    ///
    /// A modalidade da sessão continua sendo ALIMENTADA por este componente — `pointer_used` no
    /// mouse-down, `keyboard_used` na tecla —, pros componentes que a consomem.
    fn anel_aceso(&self, window: &Window) -> bool {
        !self.disabled
            && self
                .focus_handle
                .as_ref()
                .is_some_and(|h| h.is_focused(window))
    }

    /// Traduz uma tecla em operação de edição, e emite o que mudou.
    fn teclado(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        // Chegou tecla = o usuário está no teclado. O `focus_ring::init` já marcaria isso; marcar
        // aqui também faz funcionar num app que esqueceu de inicializar o crate.
        crate::focus_ring::keyboard_used(window);

        let antes = self.value();
        let completo_antes = self.is_complete();
        let k = &e.keystroke;

        let mudou = if e_colar(k) {
            match cx.read_from_clipboard().and_then(|item| item.text()) {
                Some(texto) => self.colar(&texto),
                None => false,
            }
        } else if let Some(c) = digito(k) {
            self.digitar(c)
        } else {
            match k.key.as_str() {
                "backspace" => self.apagar_atras(),
                "delete" => self.apagar_frente(),
                "left" => self.ir_para(self.caret.saturating_sub(1)),
                "right" => self.ir_para(self.caret + 1),
                "home" => self.ir_para(0),
                "end" => self.ir_para(self.digits.len()),
                _ => false,
            }
        };

        if !mudou {
            return;
        }
        // A tecla foi consumida: não deve também virar atalho de quem está por cima (setas de
        // navegação de lista, backspace de "voltar", etc).
        cx.stop_propagation();
        cx.notify();

        let agora = self.value();
        if agora != antes {
            cx.emit(OTPFieldEvent::Change(agora.clone()));
            if self.is_complete() && !completo_antes {
                cx.emit(OTPFieldEvent::Complete(agora));
            }
        }
    }

    /// Uma caixa: superfície + bisel + anel, num container `relative`.
    ///
    /// O bisel e o anel são **irmãos** da superfície, não filhos: o anel cresce 3px pra fora (um
    /// recorte o comeria) e o bisel tem que cair SOBRE a borda, não 1px pra dentro dela. Nenhum dos
    /// dois tem listener, então os cliques atravessam os dois e chegam na superfície.
    fn caixa(&self, i: usize, focado: bool, cx: &mut Context<Self>) -> Div {
        let s = self.size;
        let p = paleta();
        let aqui = focado && i == self.caixa_ativa();
        let digito = self.digits.get(i).copied();

        let superficie = div()
            .size_full()
            // O dígito é centrado pelo flex: `items_center` põe a caixa de linha (do tamanho da
            // caixa) no centro da área de conteúdo, e `justify_center` é o `text-center`.
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(RADIUS))
            .border_1()
            .border_color(border_color_for(self.invalid, aqui).hsla())
            .bg(p.bg.hsla())
            .text_size(px(s.text_size()))
            // Ver [`OTPFieldSize::line_height`]: sem isto o GPUI usa a razão de ouro e o dígito sai
            // do centro.
            .line_height(px(s.line_height()))
            .text_color(p.text.hsla())
            .shadow(shadow_stack_for(self.disabled, aqui, self.invalid))
            .when_some(digito, |d, c| {
                d.child(SharedString::from(c.to_string()))
            });

        let mut wrapper = div()
            .relative()
            .flex_none()
            .size(px(s.box_size()))
            .cursor(if self.disabled {
                CursorStyle::OperationNotAllowed
            } else {
                // O default do navegador pra `<input>` é o I-beam, e a referência não o troca.
                CursorStyle::IBeam
            })
            .child(superficie);

        if !self.disabled {
            wrapper = wrapper.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _e: &MouseDownEvent, window, cx| {
                    // Mouse-down é ponteiro: a modalidade da sessão passa a "ponteiro" (o anel DESTE
                    // campo não depende dela — ver `anel_aceso` —, mas o dos vizinhos depende).
                    crate::focus_ring::pointer_used(window);
                    if this.ir_para(i) {
                        cx.notify();
                    }
                }),
            );
        }

        if let Some(b) = bevel_for(self.disabled, aqui, self.invalid) {
            wrapper = wrapper.child(b);
        }
        if aqui {
            wrapper = wrapper.child(ring_overlay(self.invalid));
        }
        wrapper
    }
}

impl EventEmitter<OTPFieldEvent> for OTPField {}

impl Focusable for OTPField {
    /// O handle DESTE campo — é o que faz o `.track_focus`, o anel e as teclas funcionarem.
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.focus_handle
            .clone()
            .unwrap_or_else(|| cx.focus_handle())
    }
}

impl Render for OTPField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focado = self.anel_aceso(window);

        let mut root = div()
            .id(("otp-field", self.id))
            .flex()
            .items_center()
            .gap(px(GAP))
            // `has-disabled:opacity-64` — esmaece o conjunto inteiro (caixas, dígitos e separadores).
            .when(self.disabled, |d| d.opacity(DISABLED_OPACITY));

        if !self.disabled {
            // Só um campo ATIVO entra na ordem de tabulação: rastrear o foco de um desabilitado o
            // deixaria alcançável por `tab` sem ter o que fazer ali.
            if let Some(h) = self.focus_handle.as_ref() {
                root = root.track_focus(h);
            }
            root = root.on_key_down(cx.listener(Self::teclado));
        }

        // As caixas, com um separador entre grupos consecutivos.
        let mut i = 0usize;
        for (g, quantas) in self.groups.iter().enumerate() {
            if g > 0 {
                root = root.child(separador());
            }
            for _ in 0..*quantas {
                root = root.child(self.caixa(i, focado, cx));
                i += 1;
            }
        }
        root
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Um `OTPField` sem `App`: o [`gpui::FocusHandle`] não é construível fora do gpui, e o modelo
    /// de edição não precisa dele.
    fn probe(len: usize) -> OTPField {
        OTPField {
            digits: Vec::new(),
            caret: 0,
            groups: vec![len],
            size: OTPFieldSize::default(),
            disabled: false,
            invalid: false,
            id: 0,
            focus_handle: None,
        }
    }

    // --- Geometria ----------------------------------------------------------------------------

    /// **Os números da caixa, travados.** `size-9 sm:size-8` = 32 e `size-10 sm:size-9` = 36, porque
    /// o `sm:` do Tailwind (≥640px) vale sempre numa janela de desktop. Um porte que use o valor
    /// mobile sai 1 unidade (4px) maior em cada caixa.
    #[test]
    fn lado_da_caixa_e_o_valor_do_breakpoint_sm() {
        assert_eq!(OTPFieldSize::Md.box_size(), 32.0, "size-9 sm:size-8");
        assert_eq!(OTPFieldSize::Lg.box_size(), 36.0, "size-10 sm:size-9");
        assert_eq!(OTPFieldSize::default(), OTPFieldSize::Md);
    }

    /// **A entrelinha é IGUAL ao lado da caixa** — `leading-8` numa `size-8`, `leading-9` numa
    /// `size-9`. É o que centraliza o dígito, e é a armadilha nº 1 desta base: sem entrelinha
    /// explícita o GPUI usa `relative(1.618)` e o glifo sai do centro (um `text-sm` ocuparia 22,6px
    /// em vez dos 20 do par do Tailwind).
    ///
    /// O teste afirma a relação, não dois números soltos: é ela que a referência declara, e é ela
    /// que continua valendo se um dia aparecer um terceiro tamanho.
    #[test]
    fn entrelinha_e_o_lado_da_caixa() {
        for t in [OTPFieldSize::Md, OTPFieldSize::Lg] {
            assert_eq!(
                t.line_height(),
                t.box_size(),
                "leading-* == size-* é o que centraliza o dígito"
            );
            // E NÃO é a razão de ouro do GPUI nem o par do Tailwind pro corpo do texto: se alguém
            // trocar por um deles, o dígito desce/sobe sem que nenhuma medida de caixa mude.
            let razao_de_ouro = t.text_size() * 1.618_034;
            assert!(
                (t.line_height() - razao_de_ouro).abs() > 1.0,
                "a entrelinha não pode ser o default do GPUI ({razao_de_ouro})"
            );
        }
    }

    /// O corpo do texto cresce com o tamanho: `text-base sm:text-sm` (14) → `text-lg sm:text-base`
    /// (16). Diferente do [`crate::input`], onde ele é 14 nos três tamanhos — lá as variantes mudam
    /// só altura e respiro. Confundir os dois portes é fácil, então fica travado.
    #[test]
    fn corpo_do_texto_cresce_com_o_tamanho() {
        assert_eq!(OTPFieldSize::Md.text_size(), 14.0, "sm:text-sm");
        assert_eq!(OTPFieldSize::Lg.text_size(), 16.0, "sm:text-base");
    }

    /// Os números que não vêm do tamanho: `gap-2`, `rounded-lg`, `ring-[3px]`, `h-0.5 w-3`.
    /// 1 unidade de espaçamento do Tailwind = 4px.
    #[test]
    fn constantes_de_layout_batem_com_a_referencia() {
        assert_eq!(GAP, 8.0, "gap-2");
        assert_eq!(RADIUS, 10.0, "rounded-lg = --radius = 0.625rem");
        assert_eq!(RING_WIDTH, 3.0, "ring-[3px]");
        assert_eq!(SEP_W, 12.0, "w-3");
        assert_eq!(SEP_H, 2.0, "h-0.5");
        assert_eq!(DISABLED_OPACITY, 0.64, "has-disabled:opacity-64");
    }

    /// **O anel não alcança a caixa vizinha**, e é por isso que o `z-10` do foco não foi portado: o
    /// `z-10` existe na referência pra o anel de uma caixa passar POR CIMA da vizinha, e aqui nunca
    /// há sobreposição pra ordenar.
    #[test]
    fn o_anel_nao_alcanca_a_caixa_vizinha() {
        assert!(
            2.0 * RING_WIDTH < GAP,
            "os anéis de duas caixas adjacentes ({}px somados) têm que caber no gap de {GAP}px",
            2.0 * RING_WIDTH
        );
    }

    // --- Cor ----------------------------------------------------------------------------------

    /// Precedência da borda, na ordem que os seletores do coss impõem: inválido+foco é o estado mais
    /// específico, depois inválido, depois foco, depois repouso.
    #[test]
    fn precedencia_da_cor_de_borda() {
        theme::set_theme(theme::ThemeMode::Dark);
        let p = paleta();

        assert_eq!(border_color_for(true, true), p.danger_border_focus);
        assert_eq!(border_color_for(true, false), p.danger_border);
        assert_eq!(border_color_for(false, true), p.ring);
        assert_eq!(border_color_for(false, false), p.border);

        // Inválido vence o foco: a validade é informação mais importante que "onde está o caret".
        assert_ne!(border_color_for(true, true), p.ring);
    }

    /// **A sombra de repouso morre no foco E no inválido.** É o detalhe fácil de perder do
    /// `otp-field.tsx` (`focus-visible:shadow-none` + `aria-invalid:shadow-none`): sem isso a caixa
    /// focada fica com sombra de repouso E anel, um contorno duplo que não existe no original.
    #[test]
    fn sombra_so_existe_em_repouso() {
        theme::set_theme(theme::ThemeMode::Dark);

        let repouso = shadow_stack_for(false, false, false);
        assert_eq!(repouso.len(), 1, "repouso: shadow-xs/5");
        assert_eq!(repouso[0].offset.y, px(1.0), "shadow-xs desce 1px");
        assert_eq!(repouso[0].blur_radius, px(2.0));
        assert_eq!(repouso[0].spread_radius, px(0.0));

        assert!(shadow_stack_for(false, true, false).is_empty(), "focado: some");
        assert!(shadow_stack_for(false, false, true).is_empty(), "inválido: some");
        assert!(shadow_stack_for(true, false, false).is_empty(), "desabilitado: some");
    }

    /// **O bisel também só aparece em repouso** — é o
    /// `not-focus-visible:not-aria-invalid:before:shadow-…` mais o `has-disabled:…:before:shadow-none!`.
    /// A mesma regra da sombra, e o mesmo detalhe fácil de perder.
    #[test]
    fn bisel_so_aparece_em_repouso() {
        theme::set_theme(theme::ThemeMode::Dark);
        assert!(bevel_for(false, false, false).is_some(), "repouso: aparece");
        assert!(bevel_for(false, true, false).is_none(), "focado: some");
        assert!(bevel_for(false, false, true).is_none(), "inválido: some");
        assert!(bevel_for(true, false, false).is_none(), "desabilitado: some");
    }

    /// **Desvio consciente do coss, travado aqui.** O original usa branco a 6% no bisel escuro; esta
    /// base usa o DOBRO, porque a 6% o filete é imperceptível no nosso fundo (vale no `input`,
    /// `card`, `button`, `frame`, `toast`, `checkbox` e `menu`). O tema CLARO segue fiel (preto 4%),
    /// porque lá o problema não existe.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = OTP_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro segue fiel à referência (preto 4%); veio {claro}"
        );
        let escuro = OTP_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    /// **O SINAL do deslocamento dirige o lado do filete**, e os dois temas divergem: `0 1px` (claro)
    /// desce → filete embaixo; `0 -1px` (escuro) sobe → filete em cima. Eu já errei isso no
    /// `slider` por não olhar o sinal, então quem escolhe o lado em [`bevel_for`] é o próprio
    /// `bevel_dir` — não há como divergirem.
    #[test]
    fn o_sinal_do_bisel_troca_entre_os_temas() {
        assert!(OTP_LIGHT.bevel_dir > 0.0, "claro: 0 1px, filete embaixo");
        assert!(OTP_DARK.bevel_dir < 0.0, "escuro: 0 -1px, filete em cima");
    }

    /// **A paleta decodifica pras cores pretendidas.** Comparar número com número não pegaria nada;
    /// o bug histórico desta base é um valor de 6 dígitos indo pro `rgba`, que desloca os canais em
    /// silêncio (`rgba(0xffffff)` é ciano). Então aqui se decodifica e se afirma o que a cor DEVE
    /// ser perceptualmente.
    #[test]
    fn paleta_decodifica_pras_cores_pretendidas() {
        // Fundo do claro: branco OPACO. Fundo do escuro: quase transparente.
        let bg: gpui::Rgba = OTP_LIGHT.bg.hsla().into();
        assert_eq!((bg.r, bg.g, bg.b, bg.a), (1.0, 1.0, 1.0, 1.0));

        // Dígito: cinza neutro (quase preto no claro, quase branco no escuro), opaco.
        for (nome, c) in [("claro", OTP_LIGHT.text), ("escuro", OTP_DARK.text)] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: dígito opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: dígito é NEUTRO — se r≠g≠b o valor foi lido deslocado"
            );
        }

        // Anel: cinza neutro e opaco (o sintoma do bug era ele sair teal, com r=0).
        for (nome, ring) in [("claro", OTP_LIGHT.ring), ("escuro", OTP_DARK.ring)] {
            let c: gpui::Rgba = ring.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: anel opaco");
            assert!(c.r > 0.1, "{nome}: anel sem canal vermelho — leitura deslocada");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: anel é cinza NEUTRO"
            );
        }

        // Inválido: vermelho de verdade.
        for (nome, d) in [
            ("claro", OTP_LIGHT.danger_border_focus),
            ("escuro", OTP_DARK.danger_border_focus),
        ] {
            let c: gpui::Rgba = d.hsla().into();
            assert!(c.r > 0.7 && c.r > c.g + 0.3 && c.r > c.b + 0.3, "{nome}: é vermelho");
        }

        // O que DEVE ser translúcido continua translúcido.
        for (nome, c) in [
            ("borda claro", OTP_LIGHT.border),
            ("borda escuro", OTP_DARK.border),
            ("anel claro", OTP_LIGHT.ring_glow),
            ("anel escuro", OTP_DARK.ring_glow),
            ("bisel claro", OTP_LIGHT.bevel),
            ("bisel escuro", OTP_DARK.bevel),
            ("fundo escuro", OTP_DARK.bg),
            ("borda inválida", OTP_LIGHT.danger_border),
        ] {
            assert!(c.alpha() < 1.0, "{nome} tem que ser translúcido, veio {}", c.alpha());
        }
    }

    /// O anel inválido é mais forte no escuro (`/24`) que no claro (`/16`) — é a única assimetria de
    /// alfa entre os temas no `otp-field.tsx`, e generalizar a regra do claro pro escuro é o erro
    /// que este teste pega.
    #[test]
    fn o_anel_invalido_e_mais_forte_no_escuro() {
        assert!(
            OTP_DARK.danger_glow.alpha() > OTP_LIGHT.danger_glow.alpha(),
            "dark:aria-invalid:focus-visible:ring-destructive/24 contra /16 no claro"
        );
    }

    /// **O separador usa o MESMO token da borda** (`bg-input`). Um segundo campo de paleta com o
    /// mesmo valor seria uma segunda fonte de verdade esperando divergir.
    #[test]
    fn separador_usa_o_token_da_borda() {
        theme::set_theme(theme::ThemeMode::Light);
        assert_eq!(paleta().border, Rgba8(0x0000001a), "--input claro = preto 10%");
        theme::set_theme(theme::ThemeMode::Dark);
        assert_eq!(paleta().border, Rgba8(0xffffff14), "--input escuro = branco 8%");
    }

    // --- Contagem de caixas e grupos ------------------------------------------------------------

    /// Os grupos são a ÚNICA fonte da contagem de caixas, então "6 caixas divididas em 3+4" não é
    /// representável. E um grupo vazio (ou zero) não vira uma caixa fantasma.
    #[test]
    fn os_grupos_sao_a_fonte_da_contagem() {
        let mut f = probe(6);
        assert_eq!(f.boxes(), 6);

        f.groups = vec![3, 3];
        assert_eq!(f.boxes(), 6, "3+3");

        // A forma que o call site usa: um array literal.
        assert_eq!(probe(6).groups([3, 3]).boxes(), 6, "aceita array, não só Vec");

        // O builder normaliza: zeros saem, lista vazia cai numa caixa.
        let g = probe(4).groups(vec![2, 0, 2]);
        assert_eq!(g.groups, vec![2, 2]);
        assert_eq!(g.boxes(), 4);
        assert_eq!(probe(4).groups(Vec::<usize>::new()).boxes(), 1, "vazio → 1 caixa");
        assert_eq!(probe(4).groups(vec![0, 0]).boxes(), 1, "só zeros → 1 caixa");
    }

    /// Encurtar os grupos apara o valor e o caret — senão sobrariam dígitos sem caixa e um caret
    /// apontando pra fora.
    #[test]
    fn encurtar_os_grupos_apara_valor_e_caret() {
        let mut f = probe(6);
        for c in "123456".chars() {
            assert!(f.digitar(c));
        }
        assert_eq!(f.caret, 6);

        f.groups = vec![3];
        f.normalizar();
        assert_eq!(f.value(), "123");
        assert_eq!(f.caret, 3);
        assert!(f.is_complete());
    }

    // --- Modelo de edição -----------------------------------------------------------------------

    /// **Auto-avanço**: cada dígito escreve na caixa do caret e empurra o caret pra frente.
    #[test]
    fn digitar_avanca_a_caixa() {
        let mut f = probe(4);
        for (i, c) in "123".chars().enumerate() {
            assert!(f.digitar(c));
            assert_eq!(f.caret, i + 1, "o caret avançou junto");
        }
        assert_eq!(f.value(), "123");
        assert!(!f.is_complete());

        assert!(f.digitar('4'));
        assert!(f.is_complete());
        assert_eq!(f.caret, 4, "o caret para na posição VIRTUAL 4");
        assert_eq!(f.caixa_ativa(), 3, "e o realce fica na última caixa");
    }

    /// **Campo cheio ignora a tecla.** A alternativa (parar o caret em `N-1`) faria a última caixa
    /// ser sobrescrita a cada tecla, o que é pior: o usuário digita e vê o código mudar sem entender.
    #[test]
    fn campo_cheio_ignora_o_digito() {
        let mut f = probe(2);
        assert!(f.digitar('1'));
        assert!(f.digitar('2'));
        assert!(!f.digitar('3'), "não mudou nada");
        assert_eq!(f.value(), "12");
        assert_eq!(f.caret, 2);
    }

    /// **Backspace apaga o dígito ANTES do caret e recua**; no começo do código não faz nada.
    /// `delete` apaga o de baixo do caret sem mexer nele.
    #[test]
    fn backspace_recua_e_delete_nao() {
        let mut f = probe(4);
        for c in "1234".chars() {
            f.digitar(c);
        }

        assert!(f.apagar_atras());
        assert_eq!((f.value().as_str(), f.caret), ("123", 3));

        assert!(f.ir_para(1));
        assert!(f.apagar_frente());
        assert_eq!((f.value().as_str(), f.caret), ("13", 1), "delete não move o caret");

        assert!(f.apagar_atras());
        assert_eq!((f.value().as_str(), f.caret), ("3", 0));
        assert!(!f.apagar_atras(), "no começo, backspace não faz nada");
        assert!(f.ir_para(1));
        assert!(!f.apagar_frente(), "no fim do valor, delete não faz nada");
    }

    /// **Apagar no meio não deixa buraco**: o resto desliza. É a consequência de o campo ser um
    /// texto de N caracteres desenhado em N caixas, e não N caixas independentes — com buracos,
    /// `value()` mentiria (`"1_3"` viraria `"13"` ou `"1 3"`, os dois errados).
    #[test]
    fn apagar_no_meio_desliza_o_resto() {
        let mut f = probe(4);
        for c in "1234".chars() {
            f.digitar(c);
        }
        f.ir_para(2);
        assert!(f.apagar_atras()); // apaga o '2'
        assert_eq!(f.value(), "134", "o 3 e o 4 deslizaram");
        assert_eq!(f.caret, 1);
        // E digitar no lugar reconstrói sem buraco.
        assert!(f.digitar('9'));
        assert_eq!(f.value(), "1934");
    }

    /// **As setas não passam do último dígito digitado.** Deixar o caret pular pra uma caixa vazia à
    /// frente é o que criaria buraco (`"1__4"`), que o modelo não representa.
    #[test]
    fn as_setas_param_no_fim_do_valor() {
        let mut f = probe(6);
        for c in "12".chars() {
            f.digitar(c);
        }
        assert_eq!(f.caret, 2);

        assert!(!f.ir_para(3), "não passa do fim do valor");
        assert_eq!(f.caret, 2);

        assert!(f.ir_para(0), "home");
        assert!(!f.ir_para(0), "já estava lá: nada mudou");
        assert!(f.ir_para(usize::MAX), "end, aparado pelo fim do valor");
        assert_eq!(f.caret, 2);
    }

    /// **Colar distribui os dígitos pelas caixas**, descartando o que não é dígito e cortando o que
    /// não cabe.
    #[test]
    fn colar_distribui_os_digitos() {
        let mut f = probe(6);
        assert!(f.colar("123-456"));
        assert_eq!(f.value(), "123456", "o separador do texto colado é descartado");
        assert_eq!(f.caret, 6);
        assert!(f.is_complete());

        let mut g = probe(4);
        assert!(g.colar("9876543"));
        assert_eq!(g.value(), "9876", "o que passa da última caixa é cortado");

        let mut h = probe(4);
        assert!(!h.colar("sem dígito aqui"), "nada pra colar: nada muda");
        assert_eq!(h.value(), "");
    }

    /// **Colar SOBRESCREVE a partir do caret** (não insere). É o que faz colar o código certo sobre
    /// um código errado funcionar — inserindo, um campo cheio não teria espaço e o colar não faria
    /// nada, que é o comportamento irritante que este teste impede de voltar.
    #[test]
    fn colar_sobrescreve_a_partir_do_caret() {
        let mut f = probe(4);
        f.colar("1111");
        assert!(f.is_complete());

        f.ir_para(0);
        assert!(f.colar("2222"), "campo cheio, caret no começo: sobrescreve tudo");
        assert_eq!(f.value(), "2222");

        let mut g = probe(4);
        g.digitar('1');
        g.digitar('2');
        assert!(g.colar("99"));
        assert_eq!(g.value(), "1299", "colou nas duas caixas a partir do caret");
        assert_eq!(g.caret, 4);

        // Com o caret no fim de um campo cheio não sobra espaço — e aí sim não faz nada.
        assert!(!g.colar("7"));
    }

    /// A caixa realçada é `min(caret, N-1)`: nunca aponta pra fora, nem num campo cheio nem num
    /// campo de uma caixa.
    #[test]
    fn a_caixa_ativa_nunca_aponta_pra_fora() {
        let mut f = probe(3);
        assert_eq!(f.caixa_ativa(), 0);
        for c in "123".chars() {
            f.digitar(c);
        }
        assert_eq!(f.caret, 3);
        assert_eq!(f.caixa_ativa(), 2, "cheio: realce na última");

        let mut um = probe(1);
        um.digitar('7');
        assert_eq!(um.caixa_ativa(), 0);
    }

    // --- Teclas -------------------------------------------------------------------------------

    /// Só dígito ASCII entra, e **nenhum atalho digita**: `cmd-1` num app hospedeiro (trocar de aba)
    /// não pode virar um "1" no campo.
    #[test]
    fn so_digito_sem_modificador_e_aceito() {
        let k = |s: &str| Keystroke::parse(s).expect("keystroke válida");

        assert_eq!(digito(&k("1")), Some('1'));
        assert_eq!(digito(&k("0")), Some('0'));
        assert_eq!(digito(&k("a")), None, "letra não");
        assert_eq!(digito(&k("enter")), None);
        assert_eq!(digito(&k("cmd-1")), None, "atalho não digita");
        assert_eq!(digito(&k("ctrl-1")), None);
        assert_eq!(digito(&k("alt-1")), None);
        // `shift` sozinho não desqualifica (num teclado numérico shift-1 não produz dígito, mas o
        // `key_char` é quem manda quando ele vem).
        assert_eq!(digito(&k("shift-1")), Some('1'));
    }

    /// O colar é o da plataforma: `cmd-v` no macOS, `ctrl-v` no resto. `secondary-v` é exatamente
    /// essa distinção do lado do gpui, então serve de âncora nos dois.
    #[test]
    fn colar_e_o_atalho_da_plataforma() {
        let k = |s: &str| Keystroke::parse(s).expect("keystroke válida");

        assert!(e_colar(&k("secondary-v")), "cmd-v no mac, ctrl-v no resto");
        assert!(!e_colar(&k("v")), "v solto digita (nada, é letra), não cola");
        assert!(!e_colar(&k("secondary-c")), "outra tecla");
        if cfg!(target_os = "macos") {
            assert!(e_colar(&k("cmd-v")));
            assert!(!e_colar(&k("ctrl-v")), "no mac, ctrl-v não é colar");
        } else {
            assert!(e_colar(&k("ctrl-v")));
            assert!(!e_colar(&k("cmd-v")), "fora do mac, super-v não é colar");
        }
    }
}

/// Testes que abrem uma **janela de verdade** (`VisualTestContext`) e passam eventos reais.
///
/// Os testes acima cobrem a lógica (cor, geometria e o modelo de edição são funções puras), mas nada
/// ali prova que a tecla chega ao `on_key_down`, que o clique numa caixa move o caret, ou — o que
/// mais importa aqui — que **a caixa mede os pixels que a referência manda**. Isso só se vê medindo
/// o layout REAL: um `canvas` de sonda grava os bounds, e os cliques provam a geometria por
/// hit-testing (um clique no vão entre duas caixas não pode acertar nenhuma).
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{canvas, AppContext as _, Bounds, Modifiers, Pixels, TestAppContext, VisualTestContext};
    use std::cell::Cell;
    use std::rc::Rc;

    /// Onde a sonda escreve o retângulo medido.
    type Medida = Rc<Cell<Option<Bounds<Pixels>>>>;

    /// Um harness que ancora o campo no canto da janela e mede a caixa dele.
    ///
    /// O `wrapper` é `relative` e **encolhe** em volta do campo (é filho de um `flex_col` com
    /// `items_start`), então um `canvas` absoluto `inset: 0` ali mede exatamente a área que o campo
    /// ocupa — largura e altura de uma vez. É o mesmo truque de medida do [`crate::tabs`].
    struct Harness {
        f: gpui::Entity<OTPField>,
        medida: Medida,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let destino = self.medida.clone();
            div().flex().flex_col().items_start().child(
                div()
                    .relative()
                    .child(self.f.clone())
                    .child(
                        canvas(
                            move |bounds, _window, _cx| destino.set(Some(bounds)),
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    ),
            )
        }
    }

    /// Abre a janela com um campo e devolve a entidade, os eventos capturados, a medida e o contexto.
    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
        construir: impl FnOnce(&mut Context<OTPField>) -> OTPField + 'static,
    ) -> (
        gpui::Entity<OTPField>,
        Rc<std::cell::RefCell<Vec<OTPFieldEvent>>>,
        Medida,
        VisualTestContext,
    ) {
        let eventos: Rc<std::cell::RefCell<Vec<OTPFieldEvent>>> =
            Rc::new(std::cell::RefCell::new(Vec::new()));
        let capturados = eventos.clone();
        let medida: Medida = Rc::new(Cell::new(None));
        let sonda = medida.clone();

        let window = cx.add_window(move |_window, cx| {
            let f = cx.new(construir);
            cx.subscribe(&f, move |_this, _e, ev: &OTPFieldEvent, _cx| {
                capturados.borrow_mut().push(ev.clone());
            })
            .detach();
            Harness { f, medida: sonda }
        });

        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let f = vcx.read(|cx| harness.read(cx).f.clone());
        (f, eventos, medida, vcx)
    }

    /// O centro da caixa `i`, em coordenadas da janela: as caixas começam em 0 e cada uma ocupa
    /// `lado + GAP`. Só vale antes do primeiro separador (o que é suficiente: os testes de clique
    /// usam campos sem grupo).
    fn centro(i: usize, s: OTPFieldSize) -> gpui::Point<Pixels> {
        let lado = s.box_size();
        point(px(i as f32 * (lado + GAP) + lado / 2.0), px(lado / 2.0))
    }

    /// **Os pixels da caixa, medidos no layout real.** 4 caixas de 32 com `gap-2` fecham 152×32; em
    /// `lg` são 36 e fecham 168×36. Se alguém trocar o valor do breakpoint (`size-9` = 36 no padrão,
    /// que é o valor MOBILE), este teste aponta na hora.
    #[gpui::test]
    fn a_caixa_mede_o_que_a_referencia_manda(cx: &mut TestAppContext) {
        for (tamanho, lado) in [(OTPFieldSize::Md, 32.0_f32), (OTPFieldSize::Lg, 36.0_f32)] {
            let (_f, _ev, medida, _vcx) = abrir(cx, move |cx| OTPField::new(4, cx).size(tamanho));
            let b = medida.get().expect("a sonda mediu");
            assert_eq!(
                f32::from(b.size.height),
                lado,
                "{tamanho:?}: a altura da linha é o lado da caixa"
            );
            assert_eq!(
                f32::from(b.size.width),
                4.0 * lado + 3.0 * GAP,
                "{tamanho:?}: 4 caixas de {lado} com 3 gaps de {GAP}"
            );
        }
    }

    /// `new(0)` não é um campo sem caixa: o construtor eleva pra 1. Um campo de código com zero
    /// caixas não tem desenho nem comportamento, e todo o resto do módulo assume `N ≥ 1`
    /// (`caixa_ativa` faria `0 - 1`).
    #[gpui::test]
    fn nunca_zero_caixas(cx: &mut TestAppContext) {
        let (f, _ev, medida, _vcx) = abrir(cx, |cx| OTPField::new(0, cx));
        cx.read(|cx| {
            assert_eq!(f.read(cx).boxes(), 1);
            assert_eq!(f.read(cx).caixa_ativa(), 0);
        });
        let b = medida.get().expect("a sonda mediu");
        assert_eq!(f32::from(b.size.width), 32.0, "uma caixa, sem gap");
    }

    /// **O separador entre grupos mede 12px e come um `gap` de cada lado.** `[3,3]` = 6 caixas, 6
    /// gaps (são 7 filhos) e um filete de 12.
    #[gpui::test]
    fn o_separador_entra_na_largura(cx: &mut TestAppContext) {
        let (_f, _ev, medida, _vcx) = abrir(cx, |cx| OTPField::new(6, cx).groups(vec![3, 3]));
        let b = medida.get().expect("a sonda mediu");
        assert_eq!(
            f32::from(b.size.width),
            6.0 * 32.0 + 6.0 * GAP + SEP_W,
            "6 caixas + 6 gaps + o separador"
        );
        assert_eq!(f32::from(b.size.height), 32.0, "o separador não muda a altura");
    }

    /// **A tecla chega, o dígito entra e o campo avança** — e os eventos saem: um `Change` por
    /// dígito e um `Complete` só no último.
    #[gpui::test]
    fn digitar_pelo_teclado_preenche_e_emite(cx: &mut TestAppContext) {
        let (f, eventos, _m, mut vcx) = abrir(cx, |cx| OTPField::new(4, cx));

        // O clique leva o foco (o `track_focus` foca no mouse-down), que é o caminho que o usuário
        // usa antes de digitar.
        vcx.simulate_click(centro(0, OTPFieldSize::Md), Modifiers::default());
        vcx.run_until_parked();

        vcx.simulate_keystrokes("1 2 3");
        vcx.run_until_parked();
        vcx.read(|cx| {
            let f = f.read(cx);
            assert_eq!(f.value(), "123");
            assert_eq!(f.caixa_ativa(), 3, "o realce avançou pra 4ª caixa");
            assert!(!f.is_complete());
        });

        vcx.simulate_keystrokes("4");
        vcx.run_until_parked();
        vcx.read(|cx| assert!(f.read(cx).is_complete()));

        let ev = eventos.borrow();
        let mudancas: Vec<&str> = ev
            .iter()
            .filter_map(|e| match e {
                OTPFieldEvent::Change(v) => Some(v.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(mudancas, vec!["1", "12", "123", "1234"]);
        let completos: Vec<&str> = ev
            .iter()
            .filter_map(|e| match e {
                OTPFieldEvent::Complete(v) => Some(v.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(completos, vec!["1234"], "Complete só na transição");
    }

    /// **Backspace e setas pelo teclado de verdade.**
    #[gpui::test]
    fn backspace_e_setas_pelo_teclado(cx: &mut TestAppContext) {
        let (f, _ev, _m, mut vcx) = abrir(cx, |cx| OTPField::new(4, cx));
        vcx.simulate_click(centro(0, OTPFieldSize::Md), Modifiers::default());
        vcx.run_until_parked();

        vcx.simulate_keystrokes("1 2 3 4");
        vcx.simulate_keystrokes("backspace");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(f.read(cx).value(), "123"));

        vcx.simulate_keystrokes("left left");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(f.read(cx).caixa_ativa(), 1));

        vcx.simulate_keystrokes("home");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(f.read(cx).caixa_ativa(), 0));

        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(f.read(cx).caixa_ativa(), 3, "end vai pro fim do valor"));
    }

    /// **Colar do clipboard de verdade**, pelo atalho da plataforma.
    #[gpui::test]
    fn colar_do_clipboard_preenche_as_caixas(cx: &mut TestAppContext) {
        let (f, eventos, _m, mut vcx) = abrir(cx, |cx| OTPField::new(6, cx).groups(vec![3, 3]));
        vcx.simulate_click(centro(0, OTPFieldSize::Md), Modifiers::default());
        vcx.run_until_parked();

        vcx.update(|_window, cx| {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string("123-456".into()));
        });
        vcx.simulate_keystrokes("secondary-v");
        vcx.run_until_parked();

        vcx.read(|cx| {
            let f = f.read(cx);
            assert_eq!(f.value(), "123456");
            assert!(f.is_complete());
        });
        assert!(
            eventos
                .borrow()
                .iter()
                .any(|e| matches!(e, OTPFieldEvent::Complete(v) if v == "123456")),
            "colar completou o código e emitiu"
        );
    }

    /// **O clique numa caixa põe o caret nela — e o vão entre caixas não é clicável.**
    ///
    /// É o teste que prova a geometria por hit-testing: se a caixa medisse 36 em vez de 32, ou se o
    /// gap fosse 4, o clique em x=36 (o meio do vão) acertaria uma caixa e o caret se moveria.
    #[gpui::test]
    fn o_clique_escolhe_a_caixa_e_o_vao_nao_e_clicavel(cx: &mut TestAppContext) {
        let (f, _ev, _m, mut vcx) = abrir(cx, |cx| OTPField::new(4, cx));
        vcx.update(|_window, cx| f.update(cx, |f, cx| f.set_value("1234", cx)));
        vcx.run_until_parked();

        vcx.simulate_click(centro(2, OTPFieldSize::Md), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(f.read(cx).caret, 2, "clicou na 3ª caixa"));

        // O meio do vão entre a 1ª e a 2ª caixa: 32..40, ou seja x=36.
        vcx.simulate_click(point(px(36.0), px(16.0)), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(f.read(cx).caret, 2, "o vão não move o caret"));

        // E o clique numa caixa à frente do valor é aparado pelo fim dele.
        vcx.update(|_window, cx| f.update(cx, |f, cx| f.set_value("1", cx)));
        vcx.run_until_parked();
        vcx.simulate_click(centro(3, OTPFieldSize::Md), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(f.read(cx).caret, 1, "aparado pelo fim do valor"));
    }

    /// **Desabilitado não recebe foco, não digita e não emite** — e não é o `opacity-64` que impede,
    /// é o guarda no handler e a ausência de `track_focus`.
    #[gpui::test]
    fn desabilitado_ignora_clique_e_tecla(cx: &mut TestAppContext) {
        let (f, eventos, _m, mut vcx) = abrir(cx, |cx| OTPField::new(4, cx).disabled(true));

        vcx.simulate_click(centro(0, OTPFieldSize::Md), Modifiers::default());
        vcx.simulate_keystrokes("1 2 3");
        vcx.run_until_parked();

        vcx.update(|window, cx| {
            let f = f.read(cx);
            assert_eq!(f.value(), "", "nada entrou");
            assert!(
                !f.focus_handle
                    .as_ref()
                    .expect("o `new` sempre preenche")
                    .is_focused(window),
                "um campo desabilitado não recebe foco"
            );
            assert!(!f.anel_aceso(window), "e não acende anel");
        });
        assert!(eventos.borrow().is_empty(), "não emitiu");
    }

    /// **A hipótese que sustenta a centralização do dígito, verificada no layout real.**
    ///
    /// A caixa mede 32 **com** as duas bordas, então a área de conteúdo tem 30 — e a caixa de linha
    /// do dígito tem 32 (`leading-8`, ver [`OTPFieldSize::line_height`]). O dígito só fica no centro
    /// se o `items_center` **centrar** um filho que não cabe, sobrando 1px pra cada lado. Se ele
    /// encostasse no topo, o dígito sairia 1px abaixo do centro — e nenhuma medida de CAIXA mudaria,
    /// então nada mais pegaria isso. Daí um teste da hipótese, no espírito do [`crate::tabs`].
    ///
    /// Não usa o componente de propósito: o que está sendo afirmado é o comportamento do
    /// GPUI/taffy do qual o porte depende.
    #[gpui::test]
    fn caixa_de_linha_maior_que_o_conteudo_fica_centrada(cx: &mut TestAppContext) {
        struct H {
            medida: Medida,
        }
        impl Render for H {
            fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                let destino = self.medida.clone();
                div().flex().flex_col().items_start().child(
                    // A caixa: 32 com borda, área de conteúdo 30.
                    div()
                        .size(px(32.0))
                        .border_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        // O "dígito": um filho de 32 de altura, como a caixa de linha.
                        .child(div().relative().w(px(10.0)).h(px(32.0)).child(
                            canvas(
                                move |bounds, _window, _cx| destino.set(Some(bounds)),
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                        )),
                )
            }
        }

        let medida: Medida = Rc::new(Cell::new(None));
        let sonda = medida.clone();
        let window = cx.add_window(move |_window, _cx| H { medida: sonda });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let b = medida.get().expect("a sonda mediu");
        assert_eq!(
            f32::from(b.origin.y),
            0.0,
            "o filho de 32 numa área de conteúdo de 30 tem que sobrar 1px PRA CADA LADO \
             (encostado no topo daria 1.0, e o dígito sairia 1px baixo)"
        );
        assert_eq!(f32::from(b.size.height), 32.0, "e continua com 32 de altura");
    }

    /// **O clique ACENDE o anel deste campo** — a exceção de `focus-visible` pra campos de texto (ver
    /// [`OTPField::anel_aceso`]). Sem isso, um campo clicado não mostraria qual caixa está ativa,
    /// porque não há caret desenhado.
    ///
    /// O que continua valendo: o clique põe a **modalidade da sessão** em "ponteiro", pros
    /// componentes que a consomem (botões, abas, switch).
    #[gpui::test]
    fn o_clique_acende_o_anel_deste_campo_mas_apaga_a_modalidade(cx: &mut TestAppContext) {
        let (f, _ev, _m, mut vcx) = abrir(cx, |cx| OTPField::new(4, cx));

        vcx.simulate_click(centro(0, OTPFieldSize::Md), Modifiers::default());
        vcx.run_until_parked();

        vcx.update(|window, cx| {
            assert!(
                f.read(cx).anel_aceso(window),
                "campo de texto casa :focus-visible mesmo por clique"
            );
            assert!(
                !crate::focus_ring::visible(),
                "mas a modalidade da sessão virou ponteiro (é o que os botões consultam)"
            );
        });

        vcx.simulate_keystrokes("1");
        vcx.run_until_parked();
        assert!(crate::focus_ring::visible(), "a tecla devolve a modalidade de teclado");
    }
}

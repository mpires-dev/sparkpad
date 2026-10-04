//! `Dialog` — o **modal** do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/dialog.tsx`
//!
//! # Duas peças
//!
//! - [`Dialog`] — o **estado**, um [`gpui::Entity`]. Guarda se está aberto, o relógio da transição,
//!   o [`gpui::FocusHandle`] do popup (é dele que o `Escape` chega) e o [`gpui::ScrollHandle`] do
//!   painel. Emite [`DialogEvent`].
//! - [`dialog_layer`] — o **elemento montável**: um overlay absoluto que cobre a raiz da view
//!   hospedeira e desenha backdrop + viewport + popup. Existe porque o GPUI **não tem portal nem
//!   `position: fixed`** — o `Dialog.Portal`/`Dialog.Viewport` da referência se plantam na janela
//!   sozinhos, e aqui quem escolhe o lugar é a app.
//!
//! Não há [`gpui::Global`] envolvido: o estado é um `Entity` normal, então dá pra ter dois modais
//! independentes (ou nenhum) e testar sem tocar em estado de processo. É a mesma divisão do
//! [`crate::toast`], com uma diferença que vale entender: o [`crate::toast::ToastManager`] é view
//! (`Render`) porque o conteúdo dele é **dado** (título, descrição), que ele consegue guardar. O
//! conteúdo de um modal é **elemento** (`AnyElement`, de uso único), que não sobrevive a um frame —
//! então quem constrói o popup a cada frame é o chamador, e o [`Dialog`] só guarda estado.
//!
//! # Montagem
//!
//! ```ignore
//! struct Shell { dialog: Entity<Dialog> }
//!
//! impl Shell {
//!     fn new(cx: &mut Context<Self>) -> Self {
//!         Self { dialog: cx.new(Dialog::new) }
//!     }
//! }
//!
//! impl Render for Shell {
//!     fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!         let dialog = self.dialog.clone();
//!         div()
//!             .relative()      // ⚠️ obrigatório: o overlay é ABSOLUTO
//!             .size_full()
//!             .child(o_conteudo_da_app())
//!             .child(dialog_layer(                       // por ÚLTIMO: pinta em cima
//!                 &self.dialog,
//!                 DialogPopup::new()
//!                     .header(
//!                         DialogHeader::new()
//!                             .title("Publicar projeto")
//!                             .description("Isto deixa o vídeo visível pra qualquer pessoa."),
//!                     )
//!                     .panel(meu_formulario)
//!                     .footer(
//!                         DialogFooter::new()
//!                             .child(Button::new("cancelar", "Cancelar").on_click({
//!                                 let d = dialog.clone();
//!                                 move |_e, window, cx| {
//!                                     d.update(cx, |d, cx| d.close(window, cx));
//!                                 }
//!                             }))
//!                             .child(Button::new("publicar", "Publicar")),
//!                     ),
//!             ))
//!     }
//! }
//! ```
//!
//! E pra abrir, de qualquer handler que tenha `&mut Window`:
//!
//! ```ignore
//! self.dialog.update(cx, |d, cx| d.open(window, cx));
//! ```
//!
//! ## Por que os mutadores pedem `&mut Window`
//!
//! [`Dialog::open`] e [`Dialog::close`] recebem `&mut Window` por **dois** motivos, os dois
//! obrigatórios:
//!
//! 1. **Foco.** Abrir foca o popup (é assim que o `Escape` chega até ele) e fechar devolve o foco a
//!    quem o tinha. Focar exige janela.
//! 2. **Redesenho.** O popup é construído dentro do `render` da view HOSPEDEIRA. Um `cx.notify()` no
//!    [`Dialog`] só sujaria quem o **observa**, e a hospedeira não observa nada — o modal abriria e a
//!    tela não mudaria até o próximo frame provocado por outra coisa. O
//!    [`gpui::Window::refresh`] resolve sem exigir um `cx.observe` no call site (o mesmo caminho que
//!    o [`crate::focus_ring`] já usa).
//!
//! # O que o GPUI exigiu adaptar
//!
//! | coss                                        | aqui                                              |
//! |---------------------------------------------|---------------------------------------------------|
//! | `Dialog.Portal` + `fixed inset-0`           | [`dialog_layer`], montado pela app                |
//! | `grid grid-rows-[1fr_auto_3fr]`             | flex coluna com dois espaçadores de `flex-grow` 1 e 3 |
//! | `before:shadow-[0_1px_…]` (bisel)           | overlay em `inset:-1px` com `border_b_1`          |
//! | `transition-opacity duration-200`           | lerp por [`Instant::elapsed`] + `Styled::opacity` |
//! | `backdrop-blur-sm`                          | **nada** — ver as omissões                        |
//! | `sm:data-starting-style:scale-98`           | **nada** — ver as omissões                        |
//!
//! # Onde isto NÃO é a referência
//!
//! Tudo abaixo é decisão consciente, e não descuido.
//!
//! **Não implementado por limitação do GPUI**
//!
//! - **`backdrop-blur-sm` não tem equivalente no GPUI.** Não há desfoque de fundo em nenhuma API de
//!   `div`. O backdrop entrega o `bg-black/32` sozinho. Emular com camadas fica pior que a omissão.
//! - **`sm:data-starting-style:scale-98` (a escala de entrada/saída).** O GPUI não tem `transform`
//!   em `div` (a [`gpui::TransformationMatrix`] só serve pra `svg`/imagem), e a escala do CSS é de
//!   **pintura**: ela encolhe o texto junto, sem tocar no layout. Reproduzir por geometria significa
//!   encolher a CAIXA — e aí o texto reflui: num popup de 512px, 2% são 10px de largura, o que muda
//!   o ponto de quebra de qualquer linha que esteja perto dele e faz a altura pular durante os
//!   200ms. A alternativa (fixar o conteúdo na largura final e recortar a sobra num container
//!   interno) exige `overflow_hidden` no lugar onde o overlay de bisel vive, mais uma altura MEDIDA
//!   que não existe no primeiro frame da primeira abertura. Um modal com animação trêmula é pior que
//!   um modal que aparece direto, então aqui a entrada/saída é **só a opacidade** — que é fiel, e é
//!   200ms com a mesma curva `ease-in-out`.
//! - **`not-dark:bg-clip-padding`**: o GPUI pinta o fundo na border box. No tema claro a borda
//!   (`--border` = preto 8%, translúcida) fica sobre o branco do fundo em vez de sobre o backdrop —
//!   na prática a borda lê como `#ebebeb` fixo. Mesma diferença já documentada no [`crate::toast`].
//! - **`overscrollContain`** do `ScrollArea` da referência: o [`crate::ScrollArea`] não tem a opção.
//!
//! **Não implementado por escopo**
//!
//! - **Modais aninhados** (`--nested-dialogs`): o `opacity-[calc(1-var(--nested-dialogs))]` e o
//!   `sm:scale-[calc(1-0.1*…)]` que encolhem o modal de trás quando outro abre em cima. Não há API
//!   de aninhamento aqui; dois [`Dialog`] montados são independentes e não se enxergam.
//! - **`bottomStickOnMobile`** e todo o bloco `max-sm:`: nesta base `sm:` **sempre** vale (desktop),
//!   então o ramo de mobile é código morto.
//! - **`DialogTrigger`/`DialogClose`**: são `render`-props do base-ui que só encaminham um `onClick`.
//!   Aqui o gatilho é qualquer botão seu chamando [`Dialog::open`], e o fechar é
//!   [`Dialog::close`] — sem componente no meio.
//! - **Aprisionamento de foco** (o `Tab` circular do base-ui, que não deixa o foco sair do popup).
//!   Abrir FOCA o popup e fechar devolve o foco; o que falta é impedir o `Tab` de escapar, e isso
//!   depende de saber a ordem de tabulação da janela inteira.
//!
//! **Valores deduzidos**
//!
//! - **`font-heading`** do [`DialogTitle`](DialogHeader::title): é um token de FAMÍLIA de fonte, e
//!   não estava na tabela de tokens resolvidos. O título sai na fonte herdada, com o peso e o corpo
//!   certos (`font-semibold`, `text-xl`/`leading-none`). Se o coss aponta `--font-heading` pra outra
//!   família, o título está na fonte errada — é o único ponto do desenho que dependia de um token
//!   que eu não tinha.
//! - **`--popover-foreground`** também não estava na tabela. Uso `--foreground` (é o que o coss faz,
//!   e é o valor que o [`crate::toast`] já tinha resolvido pra `--popover-foreground`).

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
// Mesma disciplina de cor do toast, do card e do botão: TODO valor é `0xRRGGBBAA`, com o byte de
// alfa, SEMPRE — ver [`crate::color`]. Os tokens de [`crate::theme`] são `0xRRGGBB`, e misturar as
// duas convenções desloca os canais e produz uma cor completamente diferente sem erro de compilação
// (`rgba(0xffffff)` é lido como ciano; já custou três bugs visíveis nesta base).
//
// Todos os valores abaixo saíram da tabela de tokens do coss já resolvida do `globals.css` — não
// foram deduzidos de nome de classe.

/// Tokens visuais do modal, por tema.
#[derive(Clone, Copy, Debug)]
struct DialogPalette {
    /// `--popover` — fundo da superfície do popup.
    popover: Rgba8,
    /// `--popover-foreground` — cor do texto do popup. ⚠️ Não estava na tabela; é `--foreground`
    /// (ver o doc do módulo).
    popover_fg: Rgba8,
    /// `--muted-foreground` — a descrição do header.
    muted_fg: Rgba8,
    /// `--border` — a borda de 1px do popup e a linha de topo do footer.
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
const DIALOG_LIGHT: DialogPalette = DialogPalette {
    popover: Rgba8(0xffffffff),
    popover_fg: Rgba8(0x262626ff), // neutral-800
    // mix(neutral-500 90%, black) = #686868 — o mesmo do `crate::card` e do `crate::toast`.
    muted_fg: Rgba8(0x686868ff),
    border: Rgba8(0x00000014), // black 8%
    shadow: Rgba8(0x0000000d),  // black 5% (o `/5` do `shadow-lg/5`)
    bevel: Rgba8(0x0000000a),   // black 4% — o `--color-black/4%` do `before:shadow`
    bevel_dir: 1.0,
    // `--muted` é black 4% (alfa 10); o `/72` do Tailwind multiplica o alfa → 4% × 0,72 ≈ 2,9%
    // (alfa 7). O mesmo valor que o `crate::frame` já tinha resolvido pro seu `--muted/72`.
    footer_bg: Rgba8(0x00000007),
    backdrop: Rgba8(0x00000052), // black 32%
};

/// Tema **escuro**.
const DIALOG_DARK: DialogPalette = DialogPalette {
    // `--popover` escuro = mix(background 96%, white) = #1d1d1d.
    popover: Rgba8(0x1d1d1dff),
    popover_fg: Rgba8(0xf5f5f5ff), // neutral-100
    // mix(neutral-500 90%, white) = #818181.
    muted_fg: Rgba8(0x818181ff),
    border: Rgba8(0xffffff0f), // white 6%
    shadow: Rgba8(0x0000000d), // black 5% — a sombra não muda com o tema
    // ⚠️ DESVIO CONSCIENTE do coss, o MESMO já vigente no `Input`, `Card`, `Button`, `Frame` e
    // `Toast`: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO — alfa 30 ≈ 11,8% — porque a
    // 6% o filete é imperceptível no nosso fundo escuro. NÃO "corrija" isto pra 0x0f achando que é
    // erro de porte; se a intenção mudar, mude junto o teste `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
    footer_bg: Rgba8(0xffffff07), // white 4% × 72%
    // O backdrop é `bg-black/32` — preto literal, sem token de tema. Idêntico no claro e no escuro.
    backdrop: Rgba8(0x00000052),
};

/// O `--muted/72` — a lavagem que tinge uma faixa de "cromo" sobre a superfície do popup.
///
/// `pub(crate)` porque este token aparece **duas vezes na mesma superfície** e uma delas é de outro
/// módulo: aqui ele é o fundo da faixa de footer (`bg-muted/72`), e no `command.tsx` é o
/// `before:bg-muted/72` que tinge o corpo INTEIRO do `CommandDialogPopup` (é ele que faz o painel de
/// resultados "levantar" sobre a moldura). É a mesma classe, no mesmo papel, na mesma peça — e um
/// segundo `0x…07` calculado à mão no [`crate::command`] seria a segunda verdade pro mesmo pixel.
pub(crate) fn muted_wash() -> Rgba8 {
    palette().footer_bg
}

/// A paleta do modal no tema corrente.
fn palette() -> &'static DialogPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &DIALOG_DARK,
        theme::ThemeMode::Light => &DIALOG_LIGHT,
    }
}

// =================================================================================================
// Geometria (utilitários Tailwind da referência resolvidos em número)
// =================================================================================================

/// Raio da superfície — `rounded-2xl`. O coss **não** redefine `--radius-2xl`, então vale o default
/// do Tailwind: `1rem` = **16px**.
const RADIUS: f32 = 16.0;

/// Raio das quinas de BAIXO da faixa de footer — `rounded-b-[calc(var(--radius-2xl)-1px)]` = 15px.
///
/// A faixa tem fundo próprio (`bg-muted/72`) e é o último filho da superfície, que **não** recorta os
/// filhos (o `overflow_hidden` comeria o overlay de bisel — ver [`bevel_overlay`]). Sem este raio o
/// retângulo do fundo apareceria pra fora da curva da superfície. É `RADIUS − 1` porque a faixa vive
/// na padding box, uma borda pra dentro.
///
/// `pub(crate)` porque **todo** filho que encosta na curva da superfície precisa deste mesmo raio, e o
/// [`crate::command`] tem três: a lavagem `before:` do `CommandDialogPopup`, o rodapé dele e — quando
/// não há rodapé — a base do painel de resultados. As três classes da referência são o mesmo
/// `calc(var(--radius-2xl) - 1px)`.
pub(crate) const FOOTER_RADIUS: f32 = RADIUS - 1.0;

/// Largura máxima do popup — `max-w-lg` = `32rem` = **512px**.
const MAX_WIDTH: f32 = 512.0;

/// Respiro do viewport contra a borda da janela — `p-4`.
///
/// `pub(crate)` porque quem monta um popup com teto de altura PRÓPRIO precisa da mesma conta pra saber
/// quanto sobra: o [`crate::command`] tem o `max-h-105` do `CommandDialogPopup` e o clampa contra a
/// janela menos estes dois respiros. Com um segundo `16.0` do lado dele, mudar o respiro do viewport
/// deixaria o paladar de comandos 16px mais alto que o espaço que ele tem.
pub(crate) const VIEWPORT_PAD: f32 = 16.0;

/// Quanto do espaço livre fica ACIMA do popup — o `1fr` de `grid-rows-[1fr_auto_3fr]`.
const ROW_ABOVE: f32 = 1.0;

/// Quanto do espaço livre fica ABAIXO do popup — o `3fr`.
///
/// É isto que faz o modal **não** ser centralizado: com 1 acima e 3 abaixo, ele para a um quarto do
/// espaço livre, e não na metade.
const ROW_BELOW: f32 = 3.0;

/// Respiro padrão das fatias — `p-6`.
const PAD: f32 = 24.0;

/// Respiro apertado, no lado em que uma fatia encosta noutra — `pb-3`/`pt-3`.
const PAD_SNUG: f32 = 12.0;

/// Respiro mínimo do painel contra a fatia vizinha — `pt-1`/`pb-1`.
///
/// Some com o `PAD_SNUG` do vizinho: 12 + 4 = 16px entre o texto do header e o conteúdo do painel.
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
/// Sem isto a entrelinha default do GPUI (medida em ~1,65×) engordaria o header em vários pixels —
/// o mesmo achado que o [`crate::frame`] documenta pro `text-sm`.
const TITLE_LINE: f32 = TITLE_SIZE;

/// Corpo da descrição — `text-sm`.
const DESC_SIZE: f32 = 14.0;

/// Altura de linha da descrição — o par do `text-sm` do Tailwind é 14px/**20px**.
const DESC_LINE: f32 = 20.0;

/// Recuo do botão de fechar contra as bordas do popup — `absolute end-2 top-2`.
const CLOSE_INSET: f32 = 8.0;

/// Duração da transição de entrada e de saída — `duration-200`.
const TRANSITION: Duration = Duration::from_millis(200);

// =================================================================================================
// Curva de animação
// =================================================================================================

/// A curva da referência — `ease-in-out`, que no CSS é `cubic-bezier(.42,0,.58,1)`.
///
/// Está duplicada em relação ao [`crate::toast`] (que implementa a MESMA bissecção pra outra bezier)
/// porque lá ela é privada do módulo. São ~15 linhas; extrair pra um módulo comum seria mexer em
/// arquivo que não é meu.
fn ease_in_out(t: f32) -> f32 {
    cubic_bezier(0.42, 0.0, 0.58, 1.0, t)
}

/// Uma `cubic-bezier(x1,y1,x2,y2)` de CSS avaliada em `t` ∈ `[0,1]`.
///
/// Implementada de verdade (e não aproximada): a bezier do CSS é paramétrica em `u`, então achar `y`
/// pra um `t` dado exige inverter `x(u) = t`. Vinte passos de bissecção dão ~1e-6 no domínio, muito
/// além do que 60fps mostra, e sem o risco de divergência de um Newton perto das pontas.
///
/// `pub(crate)` porque o [`crate::popover`] precisa da MESMA bissecção pra outra curva (a `ease`
/// default do `animate-in`, que é `cubic-bezier(.25,.1,.25,1)`). Um terceiro clone destas ~15 linhas
/// seria a terceira verdade pra uma conta só — a promoção de visibilidade custa nada e evita isso.
pub(crate) fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
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

/// O relógio da opacidade — o `transition-opacity duration-200` da referência.
///
/// Está num tipo próprio (e não solto no [`Dialog`]) porque é a única parte da abertura que é
/// **pura**: o [`Dialog`] carrega [`FocusHandle`]s, que só existem com uma [`App`] viva, e isso
/// tornaria a máquina de estados intestável sem abrir janela.
#[derive(Clone, Copy, Debug, Default)]
struct Fade {
    /// O alvo: aberto ou fechado.
    open: bool,
    /// De que opacidade a transição corrente partiu, e quando. `None` = parado no alvo.
    anim: Option<(f32, Instant)>,
}

impl Fade {
    /// A opacidade de repouso do estado corrente.
    fn target(&self) -> f32 {
        if self.open {
            1.0
        } else {
            0.0
        }
    }

    /// A opacidade VISÍVEL agora.
    ///
    /// Não precisa de limpeza: passado o tempo, `ease_in_out` satura em 1 e a expressão devolve
    /// exatamente o alvo. É por isso que `anim` pode ficar `Some` pra sempre sem consequência.
    fn opacity(&self) -> f32 {
        match self.anim {
            None => self.target(),
            Some((from, start)) => {
                let t = start.elapsed().as_secs_f32() / TRANSITION.as_secs_f32();
                from + (self.target() - from) * ease_in_out(t)
            }
        }
    }

    /// Troca o alvo, partindo de onde o olho está vendo o modal agora. Devolve `false` se o alvo já
    /// era esse (e então nada acontece — nem evento, nem reinício de animação).
    ///
    /// Partir do valor VISÍVEL, e não de 0/1, é o que faz fechar-e-reabrir no meio da transição não
    /// dar um salto: a opacidade continua de onde estava.
    fn set(&mut self, open: bool) -> bool {
        if self.open == open {
            return false;
        }
        self.anim = Some((self.opacity(), Instant::now()));
        self.open = open;
        true
    }

    /// Se ainda há transição em curso — é o que decide pedir o próximo frame.
    fn animating(&self) -> bool {
        self.anim.is_some_and(|(_, start)| start.elapsed() < TRANSITION)
    }

    /// Se o modal deve estar MONTADO: aberto, ou ainda desvanecendo.
    ///
    /// Fechado e parado, isto é `false` e o [`dialog_layer`] não monta nada — nem pintura, nem
    /// hitbox. Um backdrop invisível que ainda engolisse cliques seria pior que não ter modal.
    fn mounted(&self) -> bool {
        self.open || self.opacity() > 0.0
    }

    /// Põe a transição no fim, sem esperar. Só em teste: o progresso vem de [`Instant`] (relógio de
    /// parede), que o executor de teste do GPUI não adianta.
    #[cfg(test)]
    fn settle(&mut self) {
        self.anim = None;
    }
}

// =================================================================================================
// Espaçamento entre as fatias
// =================================================================================================
//
// A referência resolve isto com seletores `:has()`, porque no CSS a única forma de uma fatia saber
// que existe outra é olhar o pai. Aqui o [`DialogPopup`] é DONO das três fatias, então é uma função
// pura da presença de cada uma — e é testável sem layout.

/// Os respiros que dependem de quais fatias existem.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Spacing {
    /// `p-6`, mas `pb-3` quando existe painel — `in-[…:has([data-slot=dialog-panel])]:pb-3`.
    header_pb: f32,
    /// `p-6`, mas `pt-1` quando existe header — `in-[…:has([data-slot=dialog-header])]:pt-1`.
    panel_pt: f32,
    /// `p-6`, mas `pb-1` quando existe footer SEM linha — `:has([data-slot=dialog-footer]:not(.border-t))`.
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
fn spacing(has_header: bool, has_panel: bool, footer: Option<DialogFooterVariant>) -> Spacing {
    let bare = footer == Some(DialogFooterVariant::Bare);
    Spacing {
        header_pb: if has_panel { PAD_SNUG } else { PAD },
        panel_pt: if has_header { PAD_HAIR } else { PAD },
        panel_pb: if bare { PAD_HAIR } else { PAD },
        footer_pt: match footer {
            // A faixa com linha e fundo é simétrica: `py-4`.
            Some(DialogFooterVariant::Default) => FOOTER_PAD_Y,
            // Sem faixa, o footer é só o respiro do popup: encosta no painel (`pt-3`) ou abre o
            // `pt-4` cheio.
            Some(DialogFooterVariant::Bare) if has_panel => PAD_SNUG,
            Some(DialogFooterVariant::Bare) => FOOTER_BARE_PAD_TOP,
            None => 0.0,
        },
        footer_pb: match footer {
            Some(DialogFooterVariant::Default) => FOOTER_PAD_Y,
            Some(DialogFooterVariant::Bare) => PAD,
            None => 0.0,
        },
    }
}

// =================================================================================================
// O estado
// =================================================================================================

/// Evento emitido pelo [`Dialog`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogEvent {
    /// O modal abriu.
    Opened,
    /// O modal começou a fechar — por [`Dialog::close`], por `Escape` ou por clique no backdrop.
    Closed,
}

/// O **estado** de um modal. Ver o doc do módulo.
pub struct Dialog {
    fade: Fade,
    /// O foco do popup. É por ele que o `Escape` chega: no GPUI tecla só vai pra quem tem foco.
    focus_handle: FocusHandle,
    /// O foco do botão de fechar — o [`crate::Button`] exige um handle de fora pra entrar na ordem
    /// de tabulação e acender o anel de `focus-visible`.
    close_focus: FocusHandle,
    /// Quem tinha o foco antes de abrir, pra devolver no fechamento.
    restore_focus: Option<FocusHandle>,
    /// A rolagem do painel. Precisa viver aqui: o [`crate::ScrollArea`] guarda a posição no handle,
    /// e um handle novo por frame zeraria a rolagem.
    scroll: ScrollHandle,
    /// Id estável pra compor os `ElementId` dos filhos.
    entity_id: u64,
    /// Só em teste: a caixa que o layout de verdade deu ao popup. Ver [`Dialog::probe`].
    #[cfg(test)]
    probe: Option<gpui::Bounds<gpui::Pixels>>,
}

impl Dialog {
    /// Um modal fechado.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            fade: Fade::default(),
            focus_handle: cx.focus_handle(),
            close_focus: cx.focus_handle(),
            restore_focus: None,
            scroll: ScrollHandle::new(),
            entity_id: cx.entity_id().as_u64(),
            #[cfg(test)]
            probe: None,
        }
    }

    /// Abre o modal, foca o popup e guarda o foco anterior. Idempotente.
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.fade.set(true) {
            return;
        }
        // Guardado ANTES de focar o popup, senão o "anterior" seria o próprio popup.
        self.restore_focus = window.focused(cx);
        window.focus(&self.focus_handle);
        cx.emit(DialogEvent::Opened);
        self.wake(window, cx);
    }

    /// Fecha o modal e devolve o foco a quem o tinha. Idempotente.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.fade.set(false) {
            return;
        }
        if let Some(handle) = self.restore_focus.take() {
            window.focus(&handle);
        }
        cx.emit(DialogEvent::Closed);
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

    /// Se o modal está aberto. Continua `false` durante a animação de SAÍDA — o modal já está indo
    /// embora, e quem pergunta quer saber a intenção, não a pintura.
    pub fn is_open(&self) -> bool {
        self.fade.open
    }

    /// O foco do popup — útil pra quem quer devolver o foco pra cá depois de um desvio.
    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    /// Marca a janela pra redesenhar.
    ///
    /// O `cx.notify()` sozinho não basta: ele suja quem OBSERVA este `Entity`, e a view hospedeira
    /// (que é quem constrói o popup) não observa nada. Ver o doc do módulo.
    fn wake(&self, window: &mut Window, cx: &mut Context<Self>) {
        cx.notify();
        window.refresh();
    }

    /// A caixa que o layout de verdade deu ao popup (a padding box: a superfície recuada 1px de
    /// cada lado pela borda).
    ///
    /// ⚠️ Só existe em teste. É a ÚNICA via de verificar, com layout real, que o popup para a um
    /// quarto do espaço livre e não na metade — a proporção sai de dois `flex-grow` num container
    /// que existe só em runtime, e um teste puro não a veria.
    #[cfg(test)]
    fn probe(&self) -> Option<gpui::Bounds<gpui::Pixels>> {
        self.probe
    }

    /// Põe a transição no fim. Só em teste — ver [`Fade::settle`].
    #[cfg(test)]
    fn settle(&mut self, cx: &mut Context<Self>) {
        self.fade.settle();
        cx.notify();
    }
}

impl EventEmitter<DialogEvent> for Dialog {}

impl Focusable for Dialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// =================================================================================================
// Header
// =================================================================================================

/// O **header** do popup: título e descrição, `flex flex-col gap-2 p-6`.
///
/// O respiro de baixo aperta pra `pb-3` quando o popup tem painel — quem decide é o
/// [`DialogPopup`], via [`spacing`].
#[derive(Default)]
pub struct DialogHeader {
    title: Option<SharedString>,
    description: Option<SharedString>,
}

impl DialogHeader {
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
    /// `.header(DialogHeader::new())` distraído somaria 48px de padding do nada **e** apertaria o
    /// `pt` do painel pra 4px, deixando o conteúdo colado no topo.
    fn is_empty(&self) -> bool {
        self.title.is_none() && self.description.is_none()
    }

    fn render(self, p: &DialogPalette, pad_bottom: f32) -> Div {
        let mut el = div()
            .flex()
            .flex_col()
            // Nem o header nem o footer encolhem quando o popup bate no `max-h-full`: quem cede é o
            // painel, que é o único que tem pra onde rolar.
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
pub enum DialogFooterVariant {
    /// Faixa com linha de topo e fundo `--muted/72` — o default da referência.
    #[default]
    Default,
    /// Só as ações, sem faixa (`variant="bare"`).
    Bare,
}

/// O **footer** do popup: as ações, alinhadas à direita.
///
/// A referência é `flex flex-col-reverse gap-2 px-6 sm:flex-row sm:justify-end`. Como `sm:` sempre
/// vale nesta base, o que sobra é **linha, alinhada à direita, na ordem declarada** — o
/// `flex-col-reverse` do mobile é código morto aqui.
#[derive(Default)]
pub struct DialogFooter {
    variant: DialogFooterVariant,
    children: Vec<AnyElement>,
}

impl DialogFooter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sem faixa: nem linha de topo, nem fundo (`variant="bare"`).
    pub fn bare(mut self) -> Self {
        self.variant = DialogFooterVariant::Bare;
        self
    }

    /// A variante, por valor.
    pub fn variant(mut self, variant: DialogFooterVariant) -> Self {
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

    fn render(self, p: &DialogPalette, sp: Spacing) -> Div {
        let mut el = div()
            .flex()
            .flex_none()
            .justify_end()
            .gap(px(GAP))
            .px(px(PAD))
            .pt(px(sp.footer_pt))
            .pb(px(sp.footer_pb));

        if self.variant == DialogFooterVariant::Default {
            el = el
                .border_t_1()
                .border_color(p.border.hsla())
                .bg(p.footer_bg.hsla())
                // Ver [`FOOTER_RADIUS`]: a superfície não recorta filhos, então a faixa arredonda
                // as próprias quinas de baixo.
                .rounded_bl(px(FOOTER_RADIUS))
                .rounded_br(px(FOOTER_RADIUS));
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

/// O **popup**: a superfície do modal, com as três fatias opcionais.
///
/// É construído pelo chamador a cada frame e passado pro [`dialog_layer`] — ele não guarda estado
/// (os `AnyElement` das ações e do painel são de uso único).
#[derive(Default)]
pub struct DialogPopup {
    header: Option<DialogHeader>,
    panel: Option<AnyElement>,
    panel_fade: bool,
    footer: Option<DialogFooter>,
    show_close: bool,
    max_width: f32,
    /// Conteúdo que **substitui** as três fatias — ver [`DialogPopup::bare`].
    bare: Option<AnyElement>,
}

impl DialogPopup {
    /// Um popup vazio, com o botão de fechar (o default da referência é `showCloseButton = true`).
    pub fn new() -> Self {
        Self {
            header: None,
            panel: None,
            // `scrollFade = true` é o default do `DialogPanel` da referência.
            panel_fade: true,
            footer: None,
            show_close: true,
            max_width: MAX_WIDTH,
            bare: None,
        }
    }

    /// Um popup **sem as fatias**: o conteúdo é o único filho da superfície, encostado na padding box.
    ///
    /// A superfície continua sendo esta — raio, borda, `bg-popover`, sombra, bisel, `occlude`, o foco
    /// de onde o `Escape` sai e o fade de 200ms — e é só o INTERIOR que passa a ser do chamador. O
    /// botão de fechar sai junto (`showCloseButton = false`), porque ele é `absolute` e cairia sobre a
    /// primeira linha do conteúdo.
    ///
    /// Existe pro [`crate::command`]: o `CommandDialogPopup` do coss é um `Dialog.Popup` cujo interior
    /// é um campo de busca **encostado no topo** e um painel de resultados **sangrado de borda a
    /// borda** — nenhum dos dois cabe no `px-6` que o [`DialogPopup::panel`] impõe, e nenhum dos dois
    /// justifica um segundo backdrop, um segundo `Escape` e um segundo salvamento de foco nesta base.
    ///
    /// ⚠️ **O conteúdo é responsável pelo próprio teto de altura.** A superfície é `max-h-full
    /// min-h-0` e **não** recorta os filhos (o `overflow_hidden` comeria o [`bevel_overlay`]), então um
    /// conteúdo mais alto que a janela pinta pra fora dela em vez de rolar. As fatias resolvem isso com
    /// a [`crate::ScrollArea`] do painel; quem usa `bare` resolve com o seu próprio teto (ver
    /// [`VIEWPORT_PAD`]).
    pub fn bare(content: impl IntoElement) -> Self {
        Self {
            bare: Some(content.into_any_element()),
            show_close: false,
            ..Self::new()
        }
    }

    /// O header (título e descrição).
    pub fn header(mut self, header: DialogHeader) -> Self {
        self.header = Some(header);
        self
    }

    /// O corpo do modal. Vai dentro de um [`crate::ScrollArea`], como na referência: se o popup
    /// bater no limite de altura da janela, é ESTE bloco que rola — o header e o footer ficam.
    pub fn panel(mut self, panel: impl IntoElement) -> Self {
        self.panel = Some(panel.into_any_element());
        self
    }

    /// Liga/desliga o fade nas bordas do painel (`scrollFade`). Default ligado.
    pub fn panel_scroll_fade(mut self, fade: bool) -> Self {
        self.panel_fade = fade;
        self
    }

    /// O footer (as ações).
    pub fn footer(mut self, footer: DialogFooter) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Mostra ou esconde o **X** do canto (`showCloseButton`). Default: mostra.
    pub fn show_close_button(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    /// Largura máxima do popup. Default [`MAX_WIDTH`] (`max-w-lg` = 512px). A largura efetiva é
    /// sempre `min(essa, largura da janela − 2 × p-4)`.
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = max_width;
        self
    }
}

/// O bisel de 1px sobreposto — o `before:shadow-[0_1px_…]` da referência.
///
/// ⚠️ As quatro coisas que já custaram bug visível nesta base (ver [`crate::card::Card::bevel`],
/// [`crate::frame`] e [`crate::toast`]):
///
/// 1. **É borda, não sombra.** O [`gpui::Window::paint_shadows`] não recorta a sombra pra fora do
///    elemento que a projeta (o CSS recorta), então o `before:box-shadow` viraria uma lavagem de cor
///    sobre o popup inteiro.
/// 2. **Cobre a BORDER box** (`inset: -1px`), porque no CSS a sombra do pseudo-elemento sai 1px pra
///    fora e cai SOBRE a borda, clareando-a.
/// 3. **O raio é o da SUPERFÍCIE**, não `raio − 1`. O `calc(var(--radius-2xl)-1px)` da referência é
///    o raio do pseudo-elemento na padding box; o nosso overlay está uma caixa pra FORA.
/// 4. **A direção sai do SINAL do deslocamento.** `0 1px` (claro) desenha na BASE; `0 -1px`
///    (escuro) desenha no TOPO.
fn bevel_overlay() -> Div {
    let p = palette();
    let overlay = div()
        .absolute()
        .top(px(-1.0))
        .left(px(-1.0))
        .right(px(-1.0))
        .bottom(px(-1.0))
        .rounded(px(RADIUS))
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

/// Um espaçador de grade: o `1fr`/`3fr` do `grid-rows-[1fr_auto_3fr]`.
///
/// `flex-basis: 0` + `flex-grow: n` distribui o espaço LIVRE na razão dos `n`, que é exatamente o que
/// `fr` faz. E `flex-shrink: 0` porque um `fr` também não vai abaixo de zero: quando o popup é mais
/// alto que a janela, quem cede é o popup (que tem `min-h-0`), não a grade.
fn spacer(grow: f32) -> Div {
    let mut d = div();
    let s = d.style();
    s.flex_grow = Some(grow);
    s.flex_shrink = Some(0.0);
    s.flex_basis = Some(px(0.0).into());
    d
}

// =================================================================================================
// O elemento montável
// =================================================================================================

/// O overlay que carrega o backdrop e o popup. **Monte-o como último filho de uma raiz
/// `relative()`** — ver o exemplo no doc do módulo.
///
/// Existe porque o GPUI não tem portal nem `position: fixed`. Quando o modal está fechado (e a
/// animação de saída já acabou) ele não monta nada: nem pintura, nem hitbox.
pub fn dialog_layer(dialog: &gpui::Entity<Dialog>, popup: DialogPopup) -> impl IntoElement {
    DialogLayer {
        dialog: dialog.clone(),
        popup,
    }
}

/// O elemento que [`dialog_layer`] devolve.
#[derive(IntoElement)]
struct DialogLayer {
    dialog: gpui::Entity<Dialog>,
    popup: DialogPopup,
}

/// O que o popup precisa do [`Dialog`], lido de uma vez.
///
/// Existe pra o empréstimo do `Entity` terminar antes de montar os elementos (os handlers precisam
/// de `&mut App`) — e pra [`DialogPopup::render`] não virar uma função de oito parâmetros.
struct Chrome {
    dialog: gpui::Entity<Dialog>,
    /// O foco do popup — é daqui que o `Escape` sai.
    focus: FocusHandle,
    /// O foco do botão de fechar.
    close_focus: FocusHandle,
    /// A posição de rolagem do painel.
    scroll: ScrollHandle,
    /// Id estável pros `ElementId` dos filhos.
    entity_id: u64,
}

impl RenderOnce for DialogLayer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette();

        // Tudo o que o estado tem a dizer, copiado de uma vez: o empréstimo morre aqui, e os
        // handlers abaixo podem tocar no `Entity` sem conflito.
        let (montado, animando, opacidade, chrome) = {
            let d = self.dialog.read(cx);
            (
                d.fade.mounted(),
                d.fade.animating(),
                d.fade.opacity(),
                Chrome {
                    dialog: self.dialog.clone(),
                    focus: d.focus_handle.clone(),
                    close_focus: d.close_focus.clone(),
                    scroll: d.scroll.clone(),
                    entity_id: d.entity_id,
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
            self.dialog.update(cx, |d, _| d.probe = None);
            return raiz.w_0().h_0();
        }

        // --- Backdrop -------------------------------------------------------------------------
        //
        // `fixed inset-0 bg-black/32 backdrop-blur-sm`. O blur não existe no GPUI (ver o doc do
        // módulo); o `bg-black/32` vai sozinho.
        //
        // `occlude()` é o que dá a MODALIDADE: sem ele o clique atravessaria o backdrop e chegaria
        // na app atrás (no GPUI o hit test acumula todos os hitboxes sob o cursor, não só o de cima).
        let fechar_por_backdrop = self.dialog.clone();
        let backdrop = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(p.backdrop.hsla())
            .opacity(opacidade)
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_e: &MouseDownEvent, window, cx| {
                fechar_por_backdrop.update(cx, |d, cx| d.close(window, cx));
            });

        // --- Viewport -------------------------------------------------------------------------
        //
        // `fixed inset-0 grid grid-rows-[1fr_auto_3fr] justify-items-center p-4`.
        //
        // ⚠️ Repare que isto NÃO é centralizar: com 1 acima e 3 abaixo, o popup para a um QUARTO do
        // espaço livre. `justify-items-center` (grid) é `align-items: center` numa coluna flex.
        let viewport = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .p(px(VIEWPORT_PAD))
            .child(spacer(ROW_ABOVE))
            .child(self.popup.render(p, opacidade, &chrome))
            .child(spacer(ROW_BELOW));

        // Enquanto a transição andar, pede o próximo frame — é o motor da animação, já que não há
        // elemento de animação e o progresso vem do tempo decorrido.
        //
        // ⚠️ `on_next_frame` + `refresh`, e NÃO `window.request_animation_frame()`: o
        // `request_animation_frame` notifica a view CORRENTE, e a view corrente aqui é quem quer que
        // esteja montando o layer — o popup é reconstruído pelo `render` dela, não por um `Entity`
        // nosso. Sujar a janela inteira não depende de acertar qual view é essa, e é o que o modal
        // precisa de qualquer forma (o backdrop cobre a janela toda). O `refresh` vai DENTRO do
        // callback porque, durante o desenho, ele é um no-op de propósito.
        if animando {
            window.on_next_frame(|window, _cx| window.refresh());
        }

        raiz.size_full().child(backdrop).child(viewport)
    }
}

impl DialogPopup {
    /// A superfície do popup, com as fatias que existirem.
    fn render(self, p: &DialogPalette, opacidade: f32, chrome: &Chrome) -> Div {
        let header = self.header.filter(|h| !h.is_empty());
        let footer = self.footer.filter(|f| !f.is_empty());
        let sp = spacing(
            header.is_some(),
            self.panel.is_some(),
            footer.as_ref().map(|f| f.variant),
        );

        // `relative row-start-2 flex max-h-full min-h-0 w-full min-w-0 max-w-lg flex-col rounded-2xl
        //  border bg-popover text-popover-foreground shadow-lg/5 outline-none`
        //
        // ⚠️ SEM `overflow_hidden`: ele recortaria o overlay de bisel, que vive em `inset:-1px`
        // (armadilha conhecida — ver [`bevel_overlay`]). É por isso que quem arredonda as quinas de
        // baixo é a faixa de footer, e não um recorte da superfície.
        let fechar_por_tecla = chrome.dialog.clone();
        let mut surface = div()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .min_w(px(0.0))
            .max_w(px(self.max_width))
            // `max-h-full` + `min-h-0`: o popup nunca passa da altura útil do viewport, e pode
            // encolher abaixo do conteúdo (é o que deixa o painel rolar em vez de vazar).
            .max_h(relative(1.0))
            .min_h(px(0.0))
            .rounded(px(RADIUS))
            .border_1()
            .border_color(p.border.hsla())
            .bg(p.popover.hsla())
            .text_color(p.popover_fg.hsla())
            .shadow(surface_shadows())
            .opacity(opacidade)
            // O foco vive AQUI porque é daqui que o `Escape` sai: no GPUI tecla só chega a quem tem
            // foco, e o `Dialog::open` foca este handle.
            //
            // E o popup NÃO ganha anel de foco: a referência tem `outline-none` justamente pra
            // suprimir o dela. Quem usa o `crate::focus_ring` aqui é só o botão de fechar (por
            // dentro do `crate::Button`, que já resolve o `focus-visible`).
            .track_focus(&chrome.focus)
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                if e.keystroke.key == "escape" {
                    fechar_por_tecla.update(cx, |d, cx| d.close(window, cx));
                }
            })
            // Sem isto, um clique NO popup também acertaria o hitbox do backdrop (que está atrás) e
            // fecharia o modal.
            .occlude();

        // `bare`: o conteúdo do chamador é o único filho, e as três fatias não existem (ver
        // [`DialogPopup::bare`]). O bisel e a sonda de teste, mais abaixo, continuam valendo — eles são
        // da SUPERFÍCIE, não das fatias.
        let bare = self.bare;
        let com_fatias = bare.is_none();
        if let Some(conteudo) = bare {
            surface = surface.child(conteudo);
        }

        if let Some(h) = header.filter(|_| com_fatias) {
            surface = surface.child(h.render(p, sp.header_pb));
        }

        if let Some(panel) = self.panel.filter(|_| com_fatias) {
            // O `<ScrollArea overscrollContain scrollFade>` da referência. O `min-h-0` é o que
            // permite ele ceder quando o popup bate no `max-h-full` — sem isso o mínimo automático
            // do flex seria a altura do conteúdo e o popup estouraria a janela.
            surface = surface.child(
                crate::ScrollArea::new(("dialog-panel", chrome.entity_id), &chrome.scroll)
                    .fade(self.panel_fade)
                    // O fade é um gradiente SOBREPOSTO, não uma máscara: a cor tem que ser a que
                    // está atrás do conteúdo, que aqui é `--popover` (e não o `--background` que o
                    // `ScrollArea` assume por default).
                    .fade_color(p.popover.hsla())
                    // O `rounded-[inherit]` do viewport da referência. Uma borda pra dentro do raio
                    // da superfície, que é onde o conteúdo do painel de fato vive.
                    .radius(RADIUS - 1.0)
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

        if let Some(f) = footer.filter(|_| com_fatias) {
            surface = surface.child(f.render(p, sp));
        }

        // O bisel ANTES do botão de fechar: os dois são absolutos, então a ordem aqui só decide
        // quem pinta em cima — e no CSS o `::before` é o primeiro filho, logo fica EMBAIXO de
        // qualquer filho posicionado que venha depois.
        surface = surface.child(bevel_overlay());

        if self.show_close {
            // `absolute end-2 top-2` + `<Button size="icon" variant="ghost">` com o `XIcon` do
            // lucide. O `ButtonSize::Icon` daqui é 32×32, igual ao `size-8` do coss; o ícone do
            // iconoir equivalente ao `XIcon` é o `xmark`.
            let fechar_por_botao = chrome.dialog.clone();
            surface = surface.child(
                div()
                    .absolute()
                    .top(px(CLOSE_INSET))
                    .right(px(CLOSE_INSET))
                    .child(
                        crate::Button::icon(
                            ("dialog-close", chrome.entity_id),
                            "iconoir/regular/xmark.svg",
                        )
                        .variant(crate::ButtonVariant::Ghost)
                        .focus(&chrome.close_focus)
                        .on_click(move |_e, window, cx| {
                            fechar_por_botao.update(cx, |d, cx| d.close(window, cx));
                        }),
                    ),
            );
        }

        // A sonda de teste é um `canvas` que só mede e não pinta. Mede a PADDING box (a superfície
        // menos a borda de 1px de cada lado), que é o que um filho absoluto em `inset:0` cobre.
        #[cfg(test)]
        {
            surface = surface.child(probe(&chrome.dialog));
        }

        surface
    }
}

/// Um `canvas` overlay que grava a caixa do popup e não pinta nada. Ver [`Dialog::probe`].
#[cfg(test)]
fn probe(dialog: &gpui::Entity<Dialog>) -> impl IntoElement {
    let dialog = dialog.clone();
    gpui::canvas(
        move |bounds: gpui::Bounds<gpui::Pixels>, _window, cx| {
            dialog.update(cx, |d, _| d.probe = Some(bounds));
        },
        |_, _, _, _| {},
    )
    // `inset:0` explícito: um absoluto de insets `auto` cairia na posição estática (dentro do
    // padding) e mediria menos. Mesmo truque do `crate::toast` e do `crate::tabs`.
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
    const PALETAS: [(&str, &DialogPalette); 2] =
        [("claro", &DIALOG_LIGHT), ("escuro", &DIALOG_DARK)];

    // --- Paleta -----------------------------------------------------------------------------

    /// **A convenção de cor, decodificada de verdade.**
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
        let claro: gpui::Rgba = DIALOG_LIGHT.popover.hsla().into();
        let escuro: gpui::Rgba = DIALOG_DARK.popover.hsla().into();
        assert_eq!((claro.r, claro.g, claro.b), (1.0, 1.0, 1.0));
        assert!(escuro.r < 0.2);

        // E o texto acompanha, invertido.
        let txt_claro: gpui::Rgba = DIALOG_LIGHT.popover_fg.hsla().into();
        let txt_escuro: gpui::Rgba = DIALOG_DARK.popover_fg.hsla().into();
        assert!(txt_claro.r < 0.2, "texto escuro sobre popover claro");
        assert!(txt_escuro.r > 0.8, "texto claro sobre popover escuro");
    }

    /// **O backdrop é `bg-black/32`, preto literal e IGUAL nos dois temas.**
    ///
    /// É a armadilha do arquivo: `bg-black/32` não é um token, então não muda com o tema. Tratar o
    /// backdrop como "o fundo do tema com alfa" daria um véu quase branco no tema claro.
    #[test]
    fn backdrop_e_preto_32_nos_dois_temas() {
        assert_eq!(DIALOG_LIGHT.backdrop, DIALOG_DARK.backdrop);
        let c: gpui::Rgba = DIALOG_LIGHT.backdrop.hsla().into();
        assert_eq!((c.r, c.g, c.b), (0.0, 0.0, 0.0), "preto puro");
        // 32% em 8 bits = 82 (0x52). A tabela de alfas desta base: 32% = 0x52.
        assert!(
            (c.a - 82.0 / 255.0).abs() < 1e-4,
            "alfa tem que ser 32%; veio {}",
            c.a
        );
    }

    /// **O fundo do footer é `--muted` com o alfa MULTIPLICADO por 72%**, e não `--muted` cheio.
    ///
    /// Um `bg-muted` sem o `/72` deixaria a faixa visivelmente mais forte que a linha de topo, e o
    /// footer leria como um bloco pintado em vez de uma faixa insinuada.
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
    /// `frame` e `toast`; este teste existe pra o desvio ser uma decisão registrada e não uma deriva.
    ///
    /// O tema CLARO segue fiel (preto 4%) — o desvio é só no escuro, onde o problema existia.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = DIALOG_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%); veio {claro}"
        );

        let escuro = DIALOG_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");

        // Ligados em `let` pra o clippy não tratar as asserções como constantes: o que interessa é o
        // SENTIDO — `0 1px` desce (base), `0 -1px` sobe (topo).
        let (dir_claro, dir_escuro) = (DIALOG_LIGHT.bevel_dir, DIALOG_DARK.bevel_dir);
        assert!(dir_claro > 0.0, "no claro o filete DESCE (border_b)");
        assert!(dir_escuro < 0.0, "no escuro o filete SOBE (border_t)");
    }

    // --- Geometria --------------------------------------------------------------------------

    /// **O raio do bisel é o da SUPERFÍCIE, e o da faixa de footer é `raio − 1`.**
    ///
    /// A diferença é a caixa em que cada um vive, e já enganou uma vez nesta base: o overlay de
    /// bisel cobre a BORDER box (`inset:-1px`), então acompanha o raio da borda; a faixa de footer
    /// vive na PADDING box, uma borda pra dentro, e por isso perde 1px de raio.
    #[test]
    fn raios_seguem_a_caixa_de_cada_um() {
        assert_eq!(RADIUS, 16.0, "rounded-2xl = default do Tailwind (o coss não redefine)");
        assert_eq!(
            FOOTER_RADIUS, 15.0,
            "rounded-b-[calc(var(--radius-2xl)-1px)]"
        );
        assert!(FOOTER_RADIUS < RADIUS);
    }

    /// Os respiros vêm todos da escala do Tailwind, e a relação entre eles é o que importa: a fatia
    /// que encosta noutra aperta, e o painel encosta com um fio.
    #[test]
    fn respiros_seguem_a_escala_do_tailwind() {
        assert_eq!(PAD, 24.0, "p-6");
        assert_eq!(PAD_SNUG, 12.0, "pb-3 / pt-3");
        assert_eq!(PAD_HAIR, 4.0, "pt-1 / pb-1");
        assert_eq!(FOOTER_PAD_Y, 16.0, "py-4");
        assert_eq!(GAP, 8.0, "gap-2");
        assert_eq!(VIEWPORT_PAD, 16.0, "p-4");
        assert_eq!(CLOSE_INSET, 8.0, "top-2 end-2");
        assert!(PAD_HAIR < PAD_SNUG && PAD_SNUG < PAD);

        // O par do `text-xl leading-none`: corpo e entrelinha IGUAIS. Se alguém tirar o
        // `line_height`, a entrelinha default do GPUI engorda o header (o mesmo achado do `frame`).
        assert_eq!(TITLE_SIZE, 20.0, "text-xl");
        assert_eq!(TITLE_LINE, TITLE_SIZE, "leading-none");
        // E o par do `text-sm`: 14/20.
        assert_eq!((DESC_SIZE, DESC_LINE), (14.0, 20.0));

        assert_eq!(MAX_WIDTH, 512.0, "max-w-lg = 32rem");
    }

    /// **O popup NÃO é centralizado.** O `grid-rows-[1fr_auto_3fr]` põe um quarto do espaço livre
    /// acima e três quartos abaixo — o modal para a 1/4 da altura, não na metade.
    ///
    /// Este é o teste da PROPORÇÃO; que o layout de verdade a respeita está em
    /// [`tests_de_janela::popup_para_a_um_quarto_do_espaco_livre`].
    #[test]
    fn a_grade_do_viewport_nao_centraliza() {
        let acima = ROW_ABOVE / (ROW_ABOVE + ROW_BELOW);
        assert!(
            (acima - 0.25).abs() < 1e-6,
            "um quarto acima; veio {acima}"
        );
        assert!(ROW_ABOVE < ROW_BELOW, "se igualar, o modal centraliza");
    }

    // --- Espaçamento entre fatias -----------------------------------------------------------

    /// **A cadeia de respiros do caso completo**: header + painel + footer `bare`.
    ///
    /// O que a referência produz, e o que dá sentido aos números: 12 do header + 4 do painel = 16px
    /// entre o texto e o conteúdo; 4 do painel + 12 do footer = 16px entre o conteúdo e as ações. Os
    /// dois `16` são o respiro real, e é por isso que o painel encosta com um fio em vez de somar
    /// dois `p-6`.
    #[test]
    fn header_painel_e_footer_bare_somam_16px_em_cada_junta() {
        let sp = spacing(true, true, Some(DialogFooterVariant::Bare));
        assert_eq!(sp.header_pb, PAD_SNUG);
        assert_eq!(sp.panel_pt, PAD_HAIR);
        assert_eq!(sp.panel_pb, PAD_HAIR);
        assert_eq!(sp.footer_pt, PAD_SNUG);
        assert_eq!(sp.footer_pb, PAD);

        assert_eq!(sp.header_pb + sp.panel_pt, 16.0, "header → painel");
        assert_eq!(sp.panel_pb + sp.footer_pt, 16.0, "painel → footer");
    }

    /// Com o footer DEFAULT (faixa com linha e fundo), o painel volta ao `pb-6`: a faixa tem fundo
    /// próprio, então o respiro pertence ao painel e não pode ser um fio de 4px — senão o conteúdo
    /// encostaria na linha separadora.
    #[test]
    fn footer_com_faixa_devolve_o_pb_cheio_ao_painel() {
        let sp = spacing(true, true, Some(DialogFooterVariant::Default));
        assert_eq!(sp.panel_pb, PAD, "o `:not(.border-t)` não casa");
        assert_eq!((sp.footer_pt, sp.footer_pb), (FOOTER_PAD_Y, FOOTER_PAD_Y), "py-4, simétrico");
    }

    /// Sem painel, o header volta ao `p-6` em todos os lados — o `pb-3` existe pra encostar num
    /// painel, e sem ele apertaria o header contra o footer sem motivo.
    #[test]
    fn sem_painel_o_header_nao_aperta() {
        let sp = spacing(true, false, Some(DialogFooterVariant::Default));
        assert_eq!(sp.header_pb, PAD);
        let sp = spacing(true, false, None);
        assert_eq!(sp.header_pb, PAD);
    }

    /// Sem header, o painel abre o `pt-6` cheio: o `pt-1` é o fio que encosta no header.
    #[test]
    fn sem_header_o_painel_abre_o_topo() {
        let sp = spacing(false, true, None);
        assert_eq!(sp.panel_pt, PAD);
        assert_eq!(sp.panel_pb, PAD, "e sem footer bare, o de baixo também");
    }

    /// O footer `bare` só encosta (`pt-3`) quando há painel; sozinho ele abre o `pt-4`.
    #[test]
    fn footer_bare_encosta_apenas_no_painel() {
        assert_eq!(
            spacing(true, true, Some(DialogFooterVariant::Bare)).footer_pt,
            PAD_SNUG
        );
        assert_eq!(
            spacing(true, false, Some(DialogFooterVariant::Bare)).footer_pt,
            FOOTER_BARE_PAD_TOP
        );
    }

    /// Sem footer, os respiros dele são zero — e nada mais muda. (O popup nem renderiza a fatia.)
    #[test]
    fn sem_footer_nao_ha_respiro_de_footer() {
        let sp = spacing(true, true, None);
        assert_eq!((sp.footer_pt, sp.footer_pb), (0.0, 0.0));
        assert_eq!(sp.panel_pb, PAD, "e o painel mantém o `pb-6`");
    }

    // --- Fatias vazias ----------------------------------------------------------------------

    /// Uma fatia sem nada dentro não vira fatia — senão um `.header(DialogHeader::new())` distraído
    /// somaria 48px de padding do nada **e** (pior) faria o painel achar que há header e apertar o
    /// `pt` pra 4px, colando o conteúdo no topo.
    #[test]
    fn fatias_vazias_sao_detectadas() {
        assert!(DialogHeader::new().is_empty());
        assert!(!DialogHeader::new().title("x").is_empty());
        assert!(!DialogHeader::new().description("x").is_empty());

        assert!(DialogFooter::new().is_empty());
        assert!(!DialogFooter::new().child(div()).is_empty());
        assert!(
            DialogFooter::new().bare().is_empty(),
            "a variante não enche o footer"
        );
    }

    /// Os defaults do popup são os da referência: `showCloseButton` e `scrollFade` ligados, largura
    /// `max-w-lg`.
    #[test]
    fn defaults_do_popup_seguem_a_referencia() {
        let popup = DialogPopup::new();
        assert!(popup.show_close, "showCloseButton = true");
        assert!(popup.panel_fade, "scrollFade = true");
        assert_eq!(popup.max_width, MAX_WIDTH);
        assert_eq!(
            DialogFooter::new().variant,
            DialogFooterVariant::Default,
            "variant = default"
        );
    }

    /// O ícone do X existe de verdade no iconoir embutido. Um caminho errado não dá erro de
    /// compilação — o glifo só some silenciosamente do botão.
    #[test]
    fn o_x_de_fechar_existe_no_iconoir() {
        assert!(crate::iconoir::has("iconoir/regular/xmark.svg"));
    }

    // --- Curva ------------------------------------------------------------------------------

    /// A bezier da referência é o `ease-in-out` do CSS: passa pelas pontas, é monótona, e é
    /// SIMÉTRICA — acelera e desacelera igual. Se alguém trocar por um `ease-out`, isto pega.
    #[test]
    fn a_curva_e_um_ease_in_out_simetrico() {
        assert!(ease_in_out(0.0).abs() < 1e-4);
        assert!((ease_in_out(1.0) - 1.0).abs() < 1e-4);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-3, "meio do caminho no meio do tempo");

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

    /// **Fechado e parado, o modal não está montado** — e é isso que impede um backdrop invisível de
    /// continuar engolindo cliques depois que o modal sai.
    #[test]
    fn fechado_e_parado_nao_monta_nada() {
        let f = Fade::default();
        assert!(!f.open);
        assert_eq!(f.opacity(), 0.0);
        assert!(!f.mounted());
        assert!(!f.animating());
    }

    /// Abrir sobe a opacidade de 0 a 1 em 200ms; fechar desce. E o modal continua montado durante a
    /// saída — é o que dá tempo de a saída ser vista.
    #[test]
    fn abrir_e_fechar_percorrem_a_opacidade() {
        let mut f = Fade::default();
        assert!(f.set(true), "abriu");
        assert!(f.mounted());
        assert!(f.animating());
        // No instante zero ainda está apagado, e o alvo é 1.
        assert!(f.opacity() < 0.05);
        assert_eq!(f.target(), 1.0);

        // Passados os 200ms, chega no alvo — sem precisar de limpeza do campo `anim`.
        f.anim = Some((0.0, ha(300)));
        assert_eq!(f.opacity(), 1.0);
        assert!(!f.animating());

        assert!(f.set(false), "fechou");
        assert!(f.animating());
        assert!(f.mounted(), "continua montado durante a saída");
        assert!(f.opacity() > 0.95, "a saída parte de onde estava");

        f.anim = Some((1.0, ha(300)));
        assert_eq!(f.opacity(), 0.0);
        assert!(!f.mounted(), "e aí desmonta");
    }

    /// **Trocar de alvo no meio do caminho parte de onde o olho está vendo**, e não de 0/1. Sem
    /// isso, fechar e reabrir depressa daria um salto de opacidade.
    #[test]
    fn inverter_no_meio_nao_da_salto() {
        let mut f = Fade::default();
        f.set(true);
        // No meio do percurso. A faixa é larga de propósito: o progresso vem do relógio de PAREDE, e
        // apertar isto em torno de `ease_in_out(0.5)` = 0,5 tornaria o teste sensível a alguns
        // milissegundos de agendamento. O que importa aqui é que não está nem apagado nem cheio.
        f.anim = Some((0.0, ha(100)));
        let meio = f.opacity();
        assert!(
            (0.1..0.9).contains(&meio),
            "a metade do percurso; veio {meio}"
        );

        // Esta comparação, sim, é exata: a inversão grava a opacidade visível como ponto de partida,
        // e no instante zero da nova transição é ela que sai. Um salto aqui significaria que a
        // inversão partiu de 1 (ou de 0) em vez de partir de onde o olho estava.
        f.set(false);
        let depois = f.opacity();
        assert!(
            (depois - meio).abs() < 0.01,
            "a inversão parte de {meio}; veio {depois}"
        );
        assert_eq!(f.target(), 0.0);
    }

    /// Pedir o alvo que já vale não faz nada: nem reinicia a animação, nem (no [`Dialog`]) emite
    /// evento. É o que torna `open`/`close` idempotentes.
    #[test]
    fn alvo_repetido_e_inerte() {
        let mut f = Fade::default();
        assert!(!f.set(false), "já estava fechado");
        assert!(f.anim.is_none(), "e não começou animação nenhuma");

        f.set(true);
        let antes = f.anim;
        assert!(!f.set(true), "já estava aberto");
        assert_eq!(f.anim.map(|(k, _)| k), antes.map(|(k, _)| k));
    }

    /// `settle` (só em teste) põe a transição no fim — é o que os testes de janela usam, porque o
    /// progresso vem do relógio de parede e o executor do GPUI não o adianta.
    #[test]
    fn settle_leva_ao_alvo() {
        let mut f = Fade::default();
        f.set(true);
        f.settle();
        assert_eq!(f.opacity(), 1.0);
        assert!(!f.animating());
    }

    /// A transição é a `duration-200` da referência.
    #[test]
    fn a_transicao_dura_200ms() {
        assert_eq!(TRANSITION, Duration::from_millis(200));
    }
}

/// Testes com janela de verdade. Posição, tamanho, `Escape` e clique passam pelo layout e pelo
/// despacho de eventos do GPUI — nada disto é verificável por aritmética, e é exatamente onde um
/// modal costuma quebrar.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{point, size, AppContext as _, Bounds, Entity, Modifiers, Pixels, Render, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Uma view hospedeira igualzinha à do doc do módulo: raiz `relative` e o layer por último.
    struct Harness {
        dialog: Entity<Dialog>,
        /// Se o popup mostra um painel alto (pra exercitar o `max-h-full`).
        alto: bool,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut popup = DialogPopup::new()
                .header(
                    DialogHeader::new()
                        .title("Publicar projeto")
                        .description("Isto deixa o vídeo visível pra qualquer pessoa."),
                )
                .footer(DialogFooter::new().child(div().w(px(80.0)).h(px(32.0))));
            popup = if self.alto {
                popup.panel(div().h(px(4000.0)))
            } else {
                popup.panel(div().h(px(120.0)))
            };
            div()
                .relative()
                .size_full()
                .child(div().size_full())
                .child(dialog_layer(&self.dialog, popup))
        }
    }

    /// O que uma janela de teste devolve.
    struct Aberta {
        dialog: Entity<Dialog>,
        eventos: Rc<RefCell<Vec<DialogEvent>>>,
        vcx: VisualTestContext,
        /// O tamanho REAL do viewport. Medido, e não o pedido: os testes de posição comparam pixel
        /// com pixel, e uma janela que não tivesse ficado do tamanho pedido faria as contas
        /// baterem por acidente (ou falharem por engano).
        viewport: gpui::Size<Pixels>,
    }

    /// Abre uma janela do tamanho pedido, com o layer montado como no doc do módulo.
    fn abrir(cx: &mut TestAppContext, largura: f32, altura: f32, alto: bool) -> Aberta {
        let eventos: Rc<RefCell<Vec<DialogEvent>>> = Rc::new(RefCell::new(Vec::new()));
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
                        let dialog = cx.new(Dialog::new);
                        cx.subscribe(&dialog, move |_h, _d, ev: &DialogEvent, _cx| {
                            capturados.borrow_mut().push(*ev);
                        })
                        .detach();
                        Harness { dialog, alto }
                    })
                },
            )
            .expect("abrir a janela de teste")
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let dialog = vcx.read(|cx| harness.read(cx).dialog.clone());
        let viewport = vcx.update(|window, _cx| window.viewport_size());
        Aberta {
            dialog,
            eventos,
            vcx,
            viewport,
        }
    }

    /// Põe a transição no fim e força um frame, pra o teste medir o estado de REPOUSO.
    ///
    /// O `refresh` é explícito porque o `settle` é só de teste e não passa pelo
    /// [`Dialog::wake`] — sem ele o frame novo não vem, já que a view hospedeira não observa o
    /// `Entity`.
    fn assentar(dialog: &Entity<Dialog>, vcx: &mut VisualTestContext) {
        vcx.update(|window, cx| {
            dialog.update(cx, |d, cx| d.settle(cx));
            window.refresh();
        });
        vcx.run_until_parked();
    }

    /// Abre o modal e assenta a transição.
    fn escancarar(dialog: &Entity<Dialog>, vcx: &mut VisualTestContext) {
        vcx.update(|window, cx| dialog.update(cx, |d, cx| d.open(window, cx)));
        vcx.run_until_parked();
        assentar(dialog, vcx);
    }

    /// A caixa da SUPERFÍCIE do popup — a sonda mede a padding box, que é a superfície menos a borda
    /// de 1px de cada lado.
    fn superficie(dialog: &Entity<Dialog>, vcx: &VisualTestContext) -> Bounds<Pixels> {
        let b = vcx
            .read(|cx| dialog.read(cx).probe())
            .expect("o popup tem que estar montado");
        Bounds {
            origin: point(b.origin.x - px(1.0), b.origin.y - px(1.0)),
            size: size(b.size.width + px(2.0), b.size.height + px(2.0)),
        }
    }

    /// **Fechado, o modal não existe no layout.** Se a sonda medisse algo, haveria um popup
    /// invisível (e um backdrop com hitbox) sobre a app.
    #[gpui::test]
    fn fechado_nao_monta_nada(cx: &mut TestAppContext) {
        let a = abrir(cx, 1000.0, 800.0, false);
        assert!(!a.vcx.read(|cx| a.dialog.read(cx).is_open()));
        assert!(
            a.vcx.read(|cx| a.dialog.read(cx).probe()).is_none(),
            "nada montado"
        );
        assert!(a.eventos.borrow().is_empty());
    }

    /// **O popup para a um quarto do espaço livre, e não na metade.**
    ///
    /// É o `grid-rows-[1fr_auto_3fr]` da referência, e é a única parte do desenho que um teste puro
    /// não vê: a proporção sai de dois `flex-grow` resolvidos pelo layout de verdade.
    #[gpui::test]
    fn popup_para_a_um_quarto_do_espaco_livre(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 800.0, false);
        escancarar(&a.dialog, &mut a.vcx);

        let s = superficie(&a.dialog, &a.vcx);
        let h = f32::from(s.size.height);
        let topo = f32::from(s.origin.y);

        // O espaço livre é a altura útil (janela − 2 × p-4) menos o popup; um quarto dele fica em
        // cima, depois do próprio p-4.
        let util = f32::from(a.viewport.height) - VIEWPORT_PAD * 2.0;
        let livre = util - h;
        let esperado = VIEWPORT_PAD + livre * (ROW_ABOVE / (ROW_ABOVE + ROW_BELOW));
        assert!(
            (topo - esperado).abs() < 1.0,
            "topo esperado {esperado:.1}, veio {topo:.1} (popup de {h:.1}px)"
        );

        // E o contraste que dá sentido ao teste: NÃO é o centro.
        let centro = VIEWPORT_PAD + livre / 2.0;
        assert!(
            (topo - centro).abs() > 20.0,
            "se isto falhar, o modal centralizou (centro seria {centro:.1})"
        );
    }

    /// A largura é `min(max-w-lg, janela − 2 × p-4)`, e o popup fica centrado na horizontal
    /// (`justify-items-center`). Os dois casos: janela larga (o `max-w` corta) e janela estreita (o
    /// `w-full` manda).
    #[gpui::test]
    fn largura_e_o_menor_entre_max_w_lg_e_a_janela(cx: &mut TestAppContext) {
        // Larga: o `max-w-lg` corta em 512.
        let mut a = abrir(cx, 1000.0, 800.0, false);
        escancarar(&a.dialog, &mut a.vcx);
        let s = superficie(&a.dialog, &a.vcx);
        assert!(
            (f32::from(s.size.width) - MAX_WIDTH).abs() < 1.0,
            "esperado {MAX_WIDTH}, veio {}",
            f32::from(s.size.width)
        );
        // Centrado: as sobras dos dois lados são iguais.
        let esquerda = f32::from(s.origin.x);
        let direita = f32::from(a.viewport.width - s.origin.x - s.size.width);
        assert!(
            (esquerda - direita).abs() < 1.0,
            "centrado: {esquerda:.1} à esquerda, {direita:.1} à direita"
        );
        assert!(esquerda >= VIEWPORT_PAD - 1.0, "respeita o p-4");
    }

    /// Numa janela mais estreita que `max-w-lg`, manda o `w-full`: a largura é a da janela menos o
    /// `p-4` dos dois lados.
    #[gpui::test]
    fn em_janela_estreita_manda_o_w_full(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 360.0, 800.0, false);
        escancarar(&a.dialog, &mut a.vcx);
        let s = superficie(&a.dialog, &a.vcx);
        let esperado = f32::from(a.viewport.width) - VIEWPORT_PAD * 2.0;
        assert!(
            esperado < MAX_WIDTH,
            "a janela precisa ser mais estreita que o max-w-lg pra o teste testar algo (deu {esperado})"
        );
        assert!(
            (f32::from(s.size.width) - esperado).abs() < 1.0,
            "esperado {esperado}, veio {}",
            f32::from(s.size.width)
        );
    }

    /// **A altura do popup, pixel por pixel.**
    ///
    /// É o teste que amarra TUDO o que um teste puro não vê de uma vez: as alturas de linha do
    /// título (`leading-none`) e da descrição (`text-sm` = 14/20), o `gap-2` entre as duas, a cadeia
    /// de respiros da [`spacing`], a linha de 1px do footer, a borda de 1px da superfície, e o fato
    /// de o [`crate::ScrollArea`] não somar altura nenhuma por conta própria.
    ///
    /// O conteúdo do [`Harness`] é: header com título e descrição, painel de 120px, footer default
    /// com uma ação de 32px de altura.
    #[gpui::test]
    fn a_altura_do_popup_bate_com_a_soma_dos_utilitarios(cx: &mut TestAppContext) {
        const PANEL_H: f32 = 120.0;
        const ACAO_H: f32 = 32.0;

        let mut a = abrir(cx, 1000.0, 800.0, false);
        escancarar(&a.dialog, &mut a.vcx);
        let s = superficie(&a.dialog, &a.vcx);

        let sp = spacing(true, true, Some(DialogFooterVariant::Default));
        // Header: `p-6` no topo, uma linha de título, o `gap-2`, uma linha de descrição, e o `pb-3`
        // porque existe painel.
        let header = PAD + TITLE_LINE + GAP + DESC_LINE + sp.header_pb;
        // Painel: o fio contra o header, o conteúdo, e o `pb-6` (o footer default tem linha).
        let painel = sp.panel_pt + PANEL_H + sp.panel_pb;
        // Footer: a linha de topo de 1px conta na altura (é border-box), mais `py-4` e a ação.
        let footer = 1.0 + sp.footer_pt + ACAO_H + sp.footer_pb;
        // E a borda de 1px da superfície, de cada lado.
        let esperado = header + painel + footer + 2.0;

        let veio = f32::from(s.size.height);
        assert!(
            (veio - esperado).abs() < 0.5,
            "altura esperada {esperado}, veio {veio} \
             (header {header} + painel {painel} + footer {footer} + 2 de borda)"
        );
    }

    /// **O `max-h-full` de verdade**: com um painel de 4000px, o popup não passa da altura útil da
    /// janela — quem cede é o painel (que rola), não o header nem o footer.
    #[gpui::test]
    fn popup_nao_passa_da_altura_util(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 600.0, true);
        escancarar(&a.dialog, &mut a.vcx);

        let s = superficie(&a.dialog, &a.vcx);
        let util = f32::from(a.viewport.height) - VIEWPORT_PAD * 2.0;
        assert!(
            f32::from(s.size.height) <= util + 1.0,
            "popup de {:.1}px numa altura útil de {util}",
            f32::from(s.size.height)
        );
        // E ele de fato ENCHEU a altura útil (senão o teste passaria por vacuidade).
        assert!(
            f32::from(s.size.height) > util - 2.0,
            "com 4000px de painel o popup tinha que encher os {util}px; veio {:.1}",
            f32::from(s.size.height)
        );
        // Encostando no limite, o `1fr` de cima colapsa e o popup fica no `p-4`.
        assert!((f32::from(s.origin.y) - VIEWPORT_PAD).abs() < 1.0);
    }

    /// **`Escape` fecha.** Depende de duas coisas que só a janela mostra: o `open` ter FOCADO o
    /// popup, e a tecla subir pelo caminho de foco até o `on_key_down` dele.
    #[gpui::test]
    fn escape_fecha(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 800.0, false);
        escancarar(&a.dialog, &mut a.vcx);
        assert!(a.vcx.read(|cx| a.dialog.read(cx).is_open()));

        a.vcx.simulate_keystrokes("escape");
        a.vcx.run_until_parked();

        assert!(!a.vcx.read(|cx| a.dialog.read(cx).is_open()), "fechou");
        assert_eq!(
            *a.eventos.borrow(),
            vec![DialogEvent::Opened, DialogEvent::Closed]
        );

        // E, acabada a saída, o layer DESMONTA. Um backdrop invisível que continuasse ali seguiria
        // engolindo todo clique da app — o pior defeito possível num modal fechado.
        assentar(&a.dialog, &mut a.vcx);
        assert!(
            a.vcx.read(|cx| a.dialog.read(cx).probe()).is_none(),
            "nada montado depois da saída"
        );

        // Outra tecla não fecha nada.
        escancarar(&a.dialog, &mut a.vcx);
        a.vcx.simulate_keystrokes("a");
        a.vcx.run_until_parked();
        assert!(
            a.vcx.read(|cx| a.dialog.read(cx).is_open()),
            "só o escape fecha"
        );
    }

    /// **Clicar no backdrop fecha; clicar NO popup não.**
    ///
    /// O segundo é o que exige o `occlude()` do popup: sem ele o clique também acertaria o hitbox do
    /// backdrop, que está atrás, e o modal fecharia ao usuário clicar no próprio conteúdo.
    #[gpui::test]
    fn clique_no_backdrop_fecha_e_no_popup_nao(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 800.0, false);
        escancarar(&a.dialog, &mut a.vcx);
        let s = superficie(&a.dialog, &a.vcx);

        // Dentro do popup: o centro dele.
        let dentro = point(
            s.origin.x + s.size.width / 2.0,
            s.origin.y + s.size.height / 2.0,
        );
        clicar(&mut a.vcx, dentro);
        assert!(
            a.vcx.read(|cx| a.dialog.read(cx).is_open()),
            "clique no popup NÃO fecha"
        );

        // Fora dele: bem no rodapé da janela, que é backdrop puro (o `3fr` de baixo).
        let rodape = point(a.viewport.width / 2.0, a.viewport.height - px(4.0));
        assert!(
            rodape.y > s.origin.y + s.size.height,
            "o ponto tem que estar ABAIXO do popup, senão o teste não testa nada"
        );
        clicar(&mut a.vcx, rodape);
        assert!(
            !a.vcx.read(|cx| a.dialog.read(cx).is_open()),
            "clique no backdrop fecha"
        );
    }

    /// Abrir devolve o foco a quem o tinha quando fecha — e, no meio, o foco está no popup (é o que
    /// faz o `Escape` funcionar).
    #[gpui::test]
    fn o_foco_vai_pro_popup_e_volta(cx: &mut TestAppContext) {
        let mut a = abrir(cx, 1000.0, 800.0, false);

        // Um handle qualquer focado antes de abrir.
        let anterior = a.vcx.update(|window, cx| {
            let h = cx.focus_handle();
            window.focus(&h);
            h
        });
        a.vcx.run_until_parked();

        escancarar(&a.dialog, &mut a.vcx);
        let popup = a.vcx.read(|cx| a.dialog.read(cx).focus_handle());
        assert!(
            a.vcx.update(|window, _cx| popup.is_focused(window)),
            "o popup fica focado enquanto aberto"
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

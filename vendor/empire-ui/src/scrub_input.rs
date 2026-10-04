//! `ScrubInput` — o **campo numérico** da casa (estilo Figma): digitar **e** arrastar.
//!
//! ```ignore
//! // uma vez, no `new` da sua view:
//! let opacidade = cx.new(|cx| {
//!     ScrubInput::new("Opacidade", ScrubIcon::Droplet, 100.0, 0.0, 100.0, 1.0, window, cx)
//!         .keyframeable()
//!         .flex_fill()
//! });
//! let _sub = cx.subscribe(&opacidade, |this, _campo, ev: &ScrubInputEvent, cx| match ev {
//!     ScrubInputEvent::Change(v) => this.aplicar_opacidade(*v, cx),
//!     ScrubInputEvent::ToggleKeyframe => this.alternar_keyframe(cx),
//! });
//!
//! // a cada render: `.child(self.opacidade.clone())`
//! ```
//!
//! # As três formas de mexer no número
//!
//! 1. **Digitar**: o número vive num [`crate::Input`] de verdade (cursor, seleção, IME, undo). O
//!    commit acontece no **Enter** e no **blur**; o texto é parseado, **clampado** a `[min, max]` e
//!    **snapado** ao `step`. `Escape` **cancela** e desfoca.
//! 2. **Arrastar pelo ícone** (o *scrub*): arrasto horizontal muda o valor ao vivo, proporcional ao
//!    `step` — direita aumenta, esquerda diminui.
//! 3. **De fora**, por [`ScrubInput::set_value`] / [`ScrubInput::set_mixed`] — sincronização, que de
//!    propósito **não** emite evento (senão quem assina realimentaria o próprio `set`).
//!
//! # Por que o valor não fica com quem usa
//!
//! O componente é um `Entity` e **emite** [`ScrubInputEvent`]; não recebe callback de escrita. É o
//! que permite um scrub emitir 40 `Change` por segundo sem que o dono precise de um caminho especial
//! pra isso — ele trata o evento igual, venha de tecla ou de arraste.
//!
//! # Por que a superfície NÃO é desenhada aqui
//!
//! Ela era: este arquivo montava fundo, borda e raio à mão em volta do campo nu do
//! `gpui-component`, com a paleta V1 do editor e uma borda de foco azul. Hoje a moldura inteira é
//! um [`crate::Input`] em [`InputSize::Sm`] — o mesmo campo de qualquer formulário da lib, com o
//! bisel, a sombra de repouso e o **anel de foco** da casa. Duas superfícies de campo na mesma app
//! divergem sempre, e a que ninguém olha é a que fica pra trás.
//!
//! # O que é superset da casa
//!
//! Este componente **não tem original no coss** — não existe "number field com arraste" lá. Então
//! não há "diferenças em relação ao original": há o que vem do design system e o que é nosso.
//!
//! **Do design system**, via [`crate::Input`] (não há uma linha de superfície neste arquivo):
//!
//! - a moldura: fundo (`--background` no claro, `bg-input/32` no escuro), borda `--input`, o fio de
//!   bisel de 1px, a sombra de repouso;
//! - o **anel de foco** (`--ring` + halo de 3px, com o cross-fade de 150ms). A borda de foco azul
//!   (`#3b82f6`) que este arquivo desenhava **saiu**: era do design V1 do editor e não existe no
//!   design system — foco em azul aqui e anel neutro no resto da app é a mesma app com duas
//!   linguagens de foco;
//! - a geometria de [`InputSize::Sm`] — altura externa 28, corpo 14, raio 10, respiro 9 — e não mais
//!   sete números avulsos declarados aqui;
//! - a cor do texto do valor, pintada pelo núcleo (ver `crate::input::apply_core_text_colors`);
//! - "clicar fora desfoca", que o [`crate::Input`] já traz e antes dependia do shell do editor.
//!
//! **Nosso**, sem original de onde portar:
//!
//! - o **scrub por arraste**: o ícone-alça é o `drag source` e o wrapper ouve o `DragMoveEvent` — o
//!   mesmo par que as divisórias de painel usam. O payload do arraste carrega o id do campo
//!   dono, pra dois `ScrubInput` lado a lado não se moverem juntos;
//! - o **diamante de keyframe** ([`ScrubInput::keyframeable`]), com área de clique própria, que
//!   emite [`ScrubInputEvent::ToggleKeyframe`] sem iniciar scrub nem focar o texto;
//! - os **19 ícones de canal** ([`ScrubIcon`]): o prefixo diz QUAL parâmetro é aquele campo (X, Y,
//!   raio de canto, espessura de borda…), e é também a alça de arraste. Um campo de formulário não
//!   tem esse papel duplo;
//! - a largura fixa de 150px (o padrão de inspector: rótulo à esquerda, caixa à direita),
//!   com [`ScrubInput::flex_fill`] pra quem está num grid de duas colunas;
//! - o estado **MISTO** (`"--"`) da multi-seleção.

use gpui::{
    div, prelude::FluentBuilder as _, px, svg, App, AppContext, Context, CursorStyle, DragMoveEvent,
    Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement,
    Pixels, Point, Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
};
use gpui_component::input::{Escape, InputEvent, InputState};

use crate::color::{opaque, Rgba8};
use crate::input::{single_line, Input, InputSize};
use crate::theme;

/// Ícone do [`ScrubInput`] — **representa o que aquele input altera** e também é a alça de scrub
/// (arrastar muda o valor). Cada variante aponta para um SVG embutido servido por
/// [`crate::assets::Assets`] (renderizado como máscara de alfa, pintado com a `text_color` do
/// elemento).
///
/// Use [`ScrubIcon::Scrub`] como padrão genérico (seta ↔) quando não houver um glifo específico pro
/// parâmetro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrubIcon {
    /// Brilho / glow — p.ex. *strength* do bloom (sol).
    Strength,
    /// Contraste / corte — p.ex. *threshold* (círculo meio-preenchido).
    Threshold,
    /// Raio / dispersão — p.ex. *radius* do bloom (anéis concêntricos).
    Radius,
    /// Aberração cromática — p.ex. *amount* do chromatic (moldura/lente, franja nas bordas).
    Chromatic,
    /// Curvatura de lente — p.ex. *distort* do lens distortion (diafragma/abertura).
    LensDistort,
    /// Dispersão de lente — p.ex. *dispersion* do lens distortion (foco/moldura com centro).
    LensDispersion,
    /// Genérico: seta de redimensionamento horizontal (↔).
    Scrub,
    /// Glifo de canal **X** (prefixo dos fields de Position/Rotation/Scale X).
    AxisX,
    /// Glifo de canal **Y**.
    AxisY,
    /// Glifo de canal **Z**.
    AxisZ,
    /// Glifo de **opacidade** (círculo de contraste) — prefixo do field Opacity.
    Opacity,
    /// Glifo de **raio de canto** — prefixo do field Corner Radius (bloco padrão).
    CornerRadius,
    /// Glifo de canto **superior-esquerdo** (TL) — prefixo do field de raio per-corner.
    CornerTL,
    /// Glifo de canto **superior-direito** (TR) — prefixo do field de raio per-corner.
    CornerTR,
    /// Glifo de canto **inferior-direito** (BR) — prefixo do field de raio per-corner.
    CornerBR,
    /// Glifo de canto **inferior-esquerdo** (BL) — prefixo do field de raio per-corner.
    CornerBL,
    /// Glifo de **espessura de borda** (border-width) — prefixo dos fields de largura da borda e do
    /// outline da forma.
    BorderWidth,
    /// Glifo de **gota** (droplet) — prefixo do field de OPACIDADE das cores (fill/border/outline).
    Droplet,
    /// Glifo de **offset de outline** (quadrado tracejado) — prefixo do field de offset do outline.
    OutlineOffset,
}

impl ScrubIcon {
    /// Caminho do SVG no asset bundle ([`crate::assets::Assets`]).
    ///
    /// ⚠️ Um caminho errado aqui **não dá erro**: a `AssetSource` devolve `None` e o glifo
    /// simplesmente não aparece. É o que o teste `todo_icone_de_canal_esta_embutido` cobre.
    fn path(self) -> SharedString {
        match self {
            ScrubIcon::Strength => "icons/strength.svg",
            ScrubIcon::Threshold => "icons/threshold.svg",
            ScrubIcon::Radius => "icons/radius.svg",
            ScrubIcon::Chromatic => "icons/chromatic.svg",
            ScrubIcon::LensDistort => "icons/lens_distort.svg",
            ScrubIcon::LensDispersion => "icons/lens_dispersion.svg",
            ScrubIcon::Scrub => "icons/scrub.svg",
            ScrubIcon::AxisX => "icons/glyph_x.svg",
            ScrubIcon::AxisY => "icons/glyph_y.svg",
            ScrubIcon::AxisZ => "icons/glyph_z.svg",
            ScrubIcon::Opacity => "icons/opacity.svg",
            ScrubIcon::CornerRadius => "icons/corner_radius.svg",
            ScrubIcon::CornerTL => "icons/corner_tl.svg",
            ScrubIcon::CornerTR => "icons/corner_tr.svg",
            ScrubIcon::CornerBR => "icons/corner_br.svg",
            ScrubIcon::CornerBL => "icons/corner_bl.svg",
            ScrubIcon::BorderWidth => "icons/border_width.svg",
            ScrubIcon::Droplet => "icons/droplet.svg",
            ScrubIcon::OutlineOffset => "icons/outline_offset.svg",
        }
        .into()
    }

    /// Todas as variantes, na ordem de declaração. Existe pros testes poderem varrer o catálogo —
    /// sem isto, um glifo novo entra sem nenhuma verificação.
    #[cfg(test)]
    const ALL: &'static [ScrubIcon] = &[
        ScrubIcon::Strength,
        ScrubIcon::Threshold,
        ScrubIcon::Radius,
        ScrubIcon::Chromatic,
        ScrubIcon::LensDistort,
        ScrubIcon::LensDispersion,
        ScrubIcon::Scrub,
        ScrubIcon::AxisX,
        ScrubIcon::AxisY,
        ScrubIcon::AxisZ,
        ScrubIcon::Opacity,
        ScrubIcon::CornerRadius,
        ScrubIcon::CornerTL,
        ScrubIcon::CornerTR,
        ScrubIcon::CornerBR,
        ScrubIcon::CornerBL,
        ScrubIcon::BorderWidth,
        ScrubIcon::Droplet,
        ScrubIcon::OutlineOffset,
    ];
}

/// Evento emitido pelo [`ScrubInput`] quando o valor muda (commit de digitação ou scrub ao vivo).
/// Carrega o **valor final** já clampado/snapado.
#[derive(Debug, Clone, Copy)]
pub enum ScrubInputEvent {
    /// Novo valor do controle.
    Change(f32),
    /// Clique no **diamante de keyframe** (sufixo) de um `ScrubInput` keyframe-able. O shell trata
    /// adicionando um keyframe no playhead pro canal deste input (mesmo caminho do cronômetro). Só
    /// é emitido quando o modo keyframe-able está ligado ([`ScrubInput::keyframeable`]).
    ToggleKeyframe,
}

/// Payload de arraste do scrub. Tipo **único** (não colide com o `DraggedEdge` das divisórias de
/// painel, que é outro tipo) — assim o `on_drag_move` do `ScrubInput` só reage ao próprio scrub e o
/// do `EditorShell` só reage às divisórias.
#[derive(Clone)]
struct ScrubDrag {
    /// Id do `ScrubInput` que iniciou o arraste (pra não cruzar entre campos).
    owner: u64,
}

impl Render for ScrubDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Drag "fantasma": sem chrome visível (igual aos handles do shell).
        gpui::Empty
    }
}

// =================================================================================================
// Paleta — só a tinta DESTE componente
// =================================================================================================
//
// A superfície (fundo, borda, bisel, anel) NÃO está aqui: ela é do `crate::Input`. O que sobra é a
// tinta das duas peças que só existem neste componente — o ícone-alça e o diamante de keyframe.
//
// Nenhuma cor é escrita como número aqui, de propósito: as duas vêm de onde o mesmo papel já mora —
// o token de ícone do tema e o `--foreground` do próprio campo (ver `scrub()`). Uma cor copiada é
// uma segunda fonte de verdade, e a que ninguém olha é a que fica pra trás quando o tema muda.

/// Tinta das peças que são só do [`ScrubInput`].
///
/// A alça e o diamante compartilham **uma** gramática de ênfase — discreto em repouso, cheio quando
/// ativo (alça sob o mouse; diamante com keyframes no playhead) — então é um par só de cores, e não
/// dois. Dois pares independentes divergiriam, e a divergência não ficaria bonita em nenhum tema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScrubPalette {
    /// Repouso: o ícone-alça e o diamante SEM keyframe.
    ///
    /// É o token de **ícone** do tema ([`theme::ICON`]) — o mesmo que o [`crate::Input`] usa nos
    /// ícones dentro do campo (ver `crate::input::Input::icon_button`). Um ícone dentro deste campo
    /// não tem por que ser de outro tom que um ícone dentro de qualquer outro.
    idle: gpui::Hsla,
    /// Ativo: a alça sob o mouse e o diamante COM keyframes no playhead.
    ///
    /// É o `--foreground` do próprio campo (`crate::input::field().text`) — a mesma tinta do número
    /// que está ao lado. "Ativo" aqui é ler igual ao valor; qualquer cor de destaque própria seria
    /// uma quinta cor de ênfase na app.
    active: Rgba8,
}

/// A paleta do scrub no tema corrente.
fn scrub() -> ScrubPalette {
    ScrubPalette {
        idle: opaque(theme::ICON()).scaled(IDLE_ICON_ALPHA),
        active: crate::input::field().text,
    }
}

// =================================================================================================
// Geometria — só o que é deste componente
// =================================================================================================

/// Tamanho do campo. `Sm` porque este é um controle de **inspector**: linhas densas, muitos campos
/// empilhados. Constante (e não parâmetro) porque um `ScrubInput` `Lg` num inspector não é uma
/// escolha que faça sentido oferecer — se um dia fizer, isto vira um builder.
const SIZE: InputSize = InputSize::Sm;

/// Sensibilidade do scrub: **pixels de arrasto por `step`**. ~3px por step dá um arrasto confortável
/// (nem rápido demais, nem lento demais), no espírito do Figma.
const PX_PER_STEP: f32 = 3.0;

/// Largura fixa da caixa numérica (padrão de inspector: rótulo à esquerda, campo de 150px à
/// direita numa linha `space-between`). [`ScrubInput::flex_fill`] troca isto pela coluna inteira.
const INPUT_WIDTH: f32 = 150.0;

/// Lado do **glifo** do ícone de canal.
///
/// 16px, como todo ícone dentro de um campo desta lib (é o `[&_svg]:size-4` que o
/// [`crate::select::Select`] também resolve em 16). Não é o `InputSize::icon()` (13 no `Sm`), que é
/// o alvo de um botãozinho de campo — aqui o glifo carrega significado (qual canal é este campo) e
/// precisa ser lido, não só apertado.
const ICON_SIZE: f32 = 16.0;

/// Lado da **caixa** do ícone: a alça de arraste. O glifo (16px) fica centralizado nela.
///
/// A caixa é maior que o glifo porque ela é o alvo do arraste, e um alvo do tamanho exato do desenho
/// obriga o usuário a acertar o traço. Cabe no miolo do campo (26px no `Sm`) — é o que o teste
/// `a_caixa_da_alca_cabe_no_miolo_do_campo` garante.
///
/// Eram 20 até a primeira revisão visual: com 20, a folga de 2px de cada lado do glifo somava aos
/// 9px de respiro do campo e punha a tinta a **13px** da borda, o que num chip de inspector lê como
/// ar demais (medido em janela, contra os 11,5px que o desenho V1 tinha). 18 mantém o alvo maior que
/// o desenho e devolve 1px de cada lado.
const ICON_BOX: f32 = 18.0;

/// **Respiro horizontal do campo** — sobrescreve o do [`InputSize::Sm`] (9px) via
/// [`crate::Input::pad_x`].
///
/// ⚠️ Desvio DECLARADO do design system, e o único deste componente. Os 9px do coss são pensados pra
/// campo de formulário; aqui o campo é um chip de 150px que carrega ícone-alça + 4 caracteres, e com
/// 9 a tinta do glifo caía a 13px da borda. Com 6, cai a **~9px** — mais apertado que os 11,5 do
/// desenho V1 que este componente tinha antes da refatoração, que é o alvo que a revisão visual pediu.
const FIELD_PAD_X: f32 = 6.0;

/// **Altura EXTERNA do campo** (com as duas bordas) — sobrescreve o do [`InputSize::Sm`] (28px) via
/// [`crate::Input::height`].
///
/// ⚠️ Desvio DECLARADO do design system. 24px é a densidade do inspector do Fennel: numa coluna com
/// 12 linhas de campo, os 4px a mais do `Sm` somam 48px de rolagem. O `Sm` do coss é o menor campo de
/// FORMULÁRIO; este é um chip de painel, que é outra coisa.
const FIELD_HEIGHT: f32 = 24.0;

/// **Corpo do número** — sobrescreve o do campo (14px nos três tamanhos) via
/// [`crate::Input::text_size`].
///
/// ⚠️ Desvio DECLARADO, pelo mesmo motivo da altura: 11px é o corpo de valor do inspector. Não é uma
/// escala do Tailwind (o `text-xs` é 12) — é a densidade que este painel pede, e está aqui pra ser
/// achado quando alguém perguntar por que este campo não usa o corpo do design system.
const VALUE_TEXT: f32 = 11.0;

/// **Raio da moldura** — sobrescreve o do campo (10px) via [`crate::Input::radius`].
///
/// ⚠️ Desvio DECLARADO. 10px é `rounded-lg`, o raio do campo do coss, e ele não muda com o tamanho;
/// numa moldura de 24px de altura isso deixa 5px de reta em cada lado e o campo lê como pílula. 8px é
/// `--radius-md`, um passo REAL da escala (não um número escolhido a olho), e devolve aresta ao chip.
const FIELD_RADIUS: f32 = 8.0;

/// **Fração da opacidade da tinta de repouso** do ícone-alça e do diamante sem keyframe.
///
/// O token de ícone do tema entra opaco; a alça é uma afordância secundária (o assunto do campo é o
/// NÚMERO), e cheia ela competia com ele. Metade é o que a revisão visual pediu, e o valor entra por
/// [`Rgba8::scaled`] em vez de um segundo byte de alfa na paleta — a cor continua vindo do token, e o
/// que é decisão deste componente é a FRAÇÃO.
const IDLE_ICON_ALPHA: f32 = 0.5;

/// Lado do **diamante de keyframe** — glifo e alvo de clique no mesmo número.
///
/// Aqui glifo e alvo coincidem de propósito: o diamante é um losango que já ocupa a diagonal inteira
/// da sua caixa, então uma caixa maior seria área morta em volta de um alvo que já é grande o
/// bastante.
const KF_DIAMOND: f32 = 16.0;

/// Controle numérico padrão (digitar + arrastar pelo ícone).
pub struct ScrubInput {
    /// Estado do campo de texto editável (o "digitar" de verdade).
    input: Entity<InputState>,
    label: &'static str,
    /// Ícone à esquerda — representa o que o input altera **e** é a alça de scrub.
    ///
    /// É um **caminho de asset**, não o enum: o [`ScrubIcon`] é só um atalho pros 19 glifos da casa
    /// (ver [`ScrubInput::icon_path`]). `None` = sem ícone, e aí **não há alça** — ver
    /// [`ScrubInput::without_icon`].
    icon: Option<SharedString>,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    /// **Valor MISTO** (multi-seleção estilo Figma): quando `true`, os elementos selecionados têm
    /// valores DIFERENTES neste canal → o campo mostra `"--"` em vez de um número. `value` segue
    /// sendo a base do primário (pra um scrub/commit partir dela e aplicar a TODOS). Limpo assim que
    /// o usuário digita/arrasta um valor concreto (vira uniforme) ou via [`Self::set_value`].
    mixed: bool,
    /// Id estável deste campo (pra casar o `ScrubDrag`).
    id: u64,
    /// Âncora do scrub: `(x inicial do mouse, valor inicial)`. `Some` enquanto arrasta. Capturada no
    /// início do drag pra o delta ser relativo ao ponto de pressão, não ao movimento incremental
    /// (ver [`scrubbed_value`]).
    scrub_anchor: Option<(Pixels, f32)>,
    /// Modo **keyframe-able**: quando `true`, o input ganha um **diamante de keyframe** como SUFIXO.
    /// Clicar nele emite [`ScrubInputEvent::ToggleKeyframe`] (o shell adiciona o kf no playhead).
    /// Ligado pelo bloco padrão do inspector ([`ScrubInput::keyframeable`]).
    keyframeable: bool,
    /// Estado visual do diamante de keyframe: `true` = o canal tem keyframes no playhead resolvido
    /// (diamante preenchido); `false` = vazio (contorno). Setável por render pelo shell
    /// ([`ScrubInput::set_kf_tracked`]).
    kf_tracked: bool,
    /// Se o campo estava focado no frame ANTERIOR — ver [`ScrubInput::selecionar_ao_focar`].
    estava_focado: bool,
    /// Largura do campo: por padrão [`INPUT_WIDTH`]; em `flex_fill` a coluna inteira
    /// ([`ScrubInput::flex_fill`]).
    fill_width: bool,
    _subscriptions: Vec<Subscription>,
}

impl ScrubInput {
    /// Cria um `ScrubInput`. `value` é o valor inicial; é clampado/snapado e escrito no campo de
    /// texto. Os limites/passo vêm de `min`/`max`/`step` (tipicamente de um `ParamDef`). `icon` é o
    /// glifo à esquerda que **representa o que este input altera** (e que também serve de alça de
    /// scrub).
    pub fn new(
        label: &'static str,
        icon: ScrubIcon,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let value = snap_clamp(value, min, max, step);
        let input = cx.new(|cx| single_line(window, cx).default_value(format_value(value, step)));

        // Commit da digitação: no Enter e no Blur, lê o texto, parseia e clampa/snapa; se válido,
        // atualiza o valor e re-emite o texto normalizado.
        let sub = cx.subscribe_in(
            &input,
            window,
            |this, _state, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => {
                    // Enter: aplica o valor e **sai do foco** (o anel apaga). `window.blur()` limpa
                    // o foco da janela; isso dispara um `InputEvent::Blur` em seguida, que
                    // re-commita — idempotente, porque o texto já foi normalizado (`changed ==
                    // false`, sem 2º emit).
                    this.commit_from_text(window, cx);
                    window.blur();
                }
                // Blur (clique fora / Tab / programático) encerra o foco. Quem desfoca no clique
                // fora é o próprio `crate::Input` (ver `crate::input::blur_on_outside_click`).
                InputEvent::Blur => this.commit_from_text(window, cx),
                // Ganhou foco → re-renderiza pro anel acender.
                InputEvent::Focus => cx.notify(),
                _ => {}
            },
        );

        // `entity_id` é único por entidade — bom id estável pro casamento do drag.
        let id = cx.entity_id().as_u64();

        Self {
            input,
            label,
            icon: Some(icon.path()),
            value,
            min,
            max,
            step,
            id,
            mixed: false,
            scrub_anchor: None,
            keyframeable: false,
            kf_tracked: false,
            estava_focado: false,
            fill_width: false,
            _subscriptions: vec![sub],
        }
    }

    /// Liga o modo **keyframe-able**: o input passa a renderizar um diamante de keyframe como
    /// sufixo (clicável → [`ScrubInputEvent::ToggleKeyframe`]). Builder (consome `self`), pra usar
    /// no `cx.new(|cx| ScrubInput::new(..).keyframeable())`.
    /// Troca o glifo por **qualquer ícone** servido pela [`crate::assets::Assets`] — o
    /// [`ScrubIcon`] é só o atalho pros 19 da casa.
    ///
    /// ```ignore
    /// ScrubInput::new("Ângulo", ScrubIcon::Scrub, 0.0, -180.0, 180.0, 1.0, window, cx)
    ///     .icon_path("iconoir/regular/angle-tool.svg")
    /// ```
    ///
    /// O que **não** é composável é a caixa: ela é a alça de arraste, então o componente é dono dela
    /// (tamanho, cursor, `on_drag`) e você entrega só o desenho. Um `impl IntoElement` livre aqui
    /// deixaria o usuário passar algo sem hitbox — e o scrub morreria sem aviso.
    pub fn icon_path(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }

    /// **Sem ícone**: o campo fica só com o número.
    ///
    /// ⚠️ **Isto desliga o scrub.** A alça de arraste É o ícone (é o que o [`crate::Input`] recebe
    /// como prefixo e o que carrega o `on_drag`), então sem ícone o controle vira um campo numérico
    /// de digitar — com o clamp, o snap e o commit no Enter/blur intactos. Use quando o rótulo da
    /// linha já disser o que o campo é e o arraste não fizer falta.
    pub fn without_icon(mut self) -> Self {
        self.icon = None;
        self
    }

    pub fn keyframeable(mut self) -> Self {
        self.keyframeable = true;
        self
    }

    /// Faz o input ocupar a **largura cheia** da coluna (`flex-1`) em vez da largura fixa de
    /// 150px (`INPUT_WIDTH`). Builder. Usado pelo grid de 2 colunas do bloco padrão.
    pub fn flex_fill(mut self) -> Self {
        self.fill_width = true;
        self
    }

    /// Atualiza o estado visual do diamante de keyframe (preenchido se o canal tem keyframes no
    /// playhead). Sincronização externa por render; não emite eventos.
    pub fn set_kf_tracked(&mut self, tracked: bool, cx: &mut Context<Self>) {
        if self.kf_tracked == tracked {
            return;
        }
        self.kf_tracked = tracked;
        cx.notify();
    }

    /// Valor atual (já clampado/snapado).
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Rótulo do controle. O `render` do `ScrubInput` desenha **só a caixa numérica**; o rótulo fica
    /// a cargo de quem o posiciona, numa linha `space-between` (rótulo à esquerda, campo à direita)
    /// — o padrão do inspector. Por isso o [`crate::Input`] é montado **sem** `.label()`.
    pub fn label(&self) -> &'static str {
        self.label
    }

    /// Define o valor de fora (clampa/snapa, sincroniza o texto e re-renderiza).
    /// **Não** emite `Change` — é uma sincronização externa, não uma edição do usuário (evita loops
    /// de feedback com quem assina o evento).
    pub fn set_value(&mut self, value: f32, window: &mut Window, cx: &mut Context<Self>) {
        let v = snap_clamp(value, self.min, self.max, self.step);
        // Sai do estado MISTO mesmo se o valor numérico não mudou (o "--" precisa virar número).
        if v == self.value && !self.mixed {
            return;
        }
        self.value = v;
        self.mixed = false;
        self.sync_text(window, cx);
        cx.notify();
    }

    /// **Valor MISTO** (multi-seleção): mostra `"--"`. `base` é o valor do primário (a partir do
    /// qual um scrub/commit aplica a TODOS). Não emite `Change` (sincronização externa). No-op se já
    /// misto com a mesma base.
    pub fn set_mixed(&mut self, base: f32, window: &mut Window, cx: &mut Context<Self>) {
        let v = snap_clamp(base, self.min, self.max, self.step);
        if self.mixed && v == self.value {
            return;
        }
        self.value = v;
        self.mixed = true;
        self.sync_text(window, cx);
        cx.notify();
    }

    /// Lê o texto digitado e — se for número válido — clampa/snapa, grava e emite `Change`. Sempre
    /// re-sincroniza o texto pro valor normalizado (ex.: "1.5" fora de range vira o clampado
    /// formatado). Ver [`parse_typed`] pra regra do parse.
    fn commit_from_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value();
        match parse_typed(&text, self.min, self.max, self.step) {
            Some(v) => {
                // Digitar um número concreto SAI do estado misto e emite Change (mesmo se == base,
                // pra o shell aplicar a TODOS os selecionados — "--" → valor uniforme).
                let changed = v != self.value || self.mixed;
                self.value = v;
                self.mixed = false;
                self.sync_text(window, cx);
                if changed {
                    cx.emit(ScrubInputEvent::Change(v));
                }
            }
            // Texto inválido: restaura o valor atual no campo.
            None => self.sync_text(window, cx),
        }
        cx.notify();
    }

    /// Esc: **cancela** a edição e sai do foco. Restaura o texto pro `value` atual (descarta o que
    /// foi digitado e não commitado) e então dá `blur` na janela.
    ///
    /// A ordem importa: re-sincronizamos o texto ANTES do `blur`, pois `blur` dispara
    /// `InputEvent::Blur` → `commit_from_text`; com o texto já igual ao valor atual, o commit é
    /// no-op (não emite `Change` espúrio nem reverte o cancelamento).
    fn cancel_and_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_text(window, cx);
        window.blur();
        cx.notify();
    }

    /// Escreve o texto do campo: `"--"` no estado MISTO, senão o `value` atual formatado.
    fn sync_text(&self, window: &mut Window, cx: &mut Context<Self>) {
        let text = display_text(self.value, self.step, self.mixed);
        self.input.update(cx, |state, cx| {
            state.set_value(text, window, cx);
        });
    }

    /// Aplica um passo de scrub a partir da âncora e da posição atual do mouse.
    fn scrub_to(&mut self, mouse_x: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        let Some((anchor_x, start_value)) = self.scrub_anchor else {
            return;
        };
        let v = scrubbed_value(
            f32::from(anchor_x),
            f32::from(mouse_x),
            start_value,
            self.min,
            self.max,
            self.step,
        );
        if v != self.value || self.mixed {
            self.value = v;
            self.mixed = false; // arrastar concretiza um valor → sai do misto (aplica a todos).
            self.sync_text(window, cx);
            cx.emit(ScrubInputEvent::Change(v));
            cx.notify();
        }
    }

    /// O ícone à esquerda do campo: **representa o parâmetro** (via [`ScrubIcon`]) e também é o
    /// **handle de arraste** (scrub).
    ///
    /// - Renderiza o SVG embutido (máscara de alfa pintada pela `text_color`).
    /// - Hover → cursor [`CursorStyle::ResizeLeftRight`] (o mesmo das divisórias de painel do
    ///   `editor_shell`, pra dar a dica de "arrastável").
    /// - `on_drag(ScrubDrag, …)` inicia o arraste (mesmo padrão das divisórias), com a âncora
    ///   capturada no `mouse_down` (antes do 1º move).
    fn scrub_icon(&self, path: SharedString, cx: &mut Context<Self>) -> impl IntoElement {
        let id = self.id;
        let p = scrub();
        div()
            .id("scrub-handle")
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .flex_none()
            // A caixa é o alvo do arraste; o glifo fica centralizado nela. O vão até o número é o
            // `gap` do conteúdo do `crate::Input` — não há margem própria aqui.
            .size(px(ICON_BOX))
            // O `gpui::svg()` pinta o SVG (como máscara de alfa) com a `text_color` do PRÓPRIO
            // elemento `svg` — ele NÃO herda a cor de texto do `div` pai (no paint, `Svg` lê
            // `style.text.color` do seu próprio `StyleRefinement`; se for `None`, não desenha nada e
            // não loga erro). Por isso a cor (e o realce no hover) vai DIRETO no `svg()`, com
            // `.flex_none()` + `.size()` pra garantir dimensões efetivas (não colapsa pra 0).
            .child(
                svg()
                    .path(path)
                    .size(px(ICON_SIZE))
                    .flex_none()
                    .text_color(p.idle)
                    .hover(|s| s.text_color(p.active.hsla())),
            )
            .cursor(CursorStyle::ResizeLeftRight)
            .on_drag(ScrubDrag { owner: id }, move |drag, _pos, _window, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            // Captura a âncora assim que o botão desce no ícone (antes do 1º move).
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, e: &gpui::MouseDownEvent, _window, cx| {
                    cx.stop_propagation();
                    this.scrub_anchor = Some((e.position.x, this.value));
                }),
            )
    }

    /// O **diamante de keyframe** como SUFIXO do input keyframe-able. Tem **área de clique própria**
    /// (`occlude` + `stop_propagation`), pra não atrapalhar o scrub/a digitação do valor. Clicar
    /// emite [`ScrubInputEvent::ToggleKeyframe`] (o shell adiciona o kf no playhead). Visual: glifo
    /// `kf_diamond` (contorno, tinta de repouso) quando NÃO tracked; `kf_diamond_filled` (tinta
    /// ativa) quando tracked.
    fn kf_diamond_suffix(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = scrub();
        let (path, color) = if self.kf_tracked {
            ("icons/kf_diamond_filled.svg", p.active.hsla())
        } else {
            ("icons/kf_diamond.svg", p.idle)
        };
        div()
            .id("kf-diamond")
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .flex_none()
            .size(px(KF_DIAMOND))
            .cursor(CursorStyle::PointingHand)
            .child(
                svg()
                    .path(path)
                    .size(px(KF_DIAMOND))
                    .flex_none()
                    .text_color(color)
                    .hover(|s| s.text_color(p.active.hsla())),
            )
            // Mouse-down próprio: para a propagação (não inicia scrub nem foca o texto) e emite o
            // toggle. `on_mouse_down` (não `on_click`) pra casar com o cronômetro do inspector e
            // disparar antes de qualquer blur/scrub.
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|_this, _e: &gpui::MouseDownEvent, _window, cx| {
                    cx.stop_propagation();
                    cx.emit(ScrubInputEvent::ToggleKeyframe);
                }),
            )
    }
    /// **Seleciona o número inteiro quando o campo ganha o foco.**
    ///
    /// É o comportamento DEFAULT deste componente, e não uma opção: quem clica num campo de inspector
    /// quer trocar o valor, não editar um dígito no meio de `0.65`. Num campo de texto comum o
    /// contrário é verdade (apagar o que estava lá é hostil), e é por isso que o [`crate::Input`] não
    /// faz isso sozinho — aqui é o mesmo comportamento que o [`crate::color_picker`] liga nos campos
    /// dele, com a diferença de que lá é decisão daquela tela e aqui é do componente.
    ///
    /// Também é o que faz o scrub e a digitação conviverem: depois de arrastar, o número está
    /// selecionado, então digitar substitui em vez de concatenar.
    ///
    /// # Por que no render, e não numa assinatura de `InputEvent::Focus`
    ///
    /// Mesma razão medida no [`crate::color_picker`]: no `TestAppContext` o `Focus` **nunca chega** —
    /// o `on_focus` do núcleo depende do caminho de foco, que é reconstruído no desenho, e naquele
    /// ambiente ele não fecha no campo. Comparar com o frame anterior custa um `is_focused` por render
    /// e é observável, o que deixa isto **testável**; a assinatura de evento não deixaria.
    ///
    /// A ordem sai de graça: o campo posiciona o cursor no próprio mouse-down, e como esta checagem
    /// roda no render seguinte, a seleção vem depois e sobrevive ao clique.
    fn selecionar_ao_focar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focado = gpui::Focusable::focus_handle(self.input.read(cx), cx).is_focused(window);
        if focado == self.estava_focado {
            return;
        }
        self.estava_focado = focado;
        if focado {
            self.input.update(cx, |st, cx| st.select_all_now(cx));
        }
    }
}
impl Focusable for ScrubInput {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.focus_handle(cx)
    }
}

impl EventEmitter<ScrubInputEvent> for ScrubInput {}

impl Render for ScrubInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Seleciona o número quando o campo acabou de ganhar o foco. No render de propósito — ver o
        // doc do método.
        self.selecionar_ao_focar(window, cx);
        let id = self.id;
        // O `render` desenha **só a caixa numérica**. Largura: por padrão fixa (padrão de
        // inspector); em `flex_fill` ocupa a coluna inteira, pro grid de 2 colunas do bloco padrão.
        let fill = self.fill_width;
        div()
            .id("scrub-input")
            .flex()
            .map(|d| {
                if fill {
                    d.flex_1().min_w(px(0.0))
                } else {
                    d.flex_none()
                }
            })
            // Esc: o `InputState` trata a tecla mas, no caso comum, faz `cx.propagate()` (não emite
            // evento próprio). A action `Escape` então sobe a árvore de dispatch (o campo é
            // descendente focado deste wrapper) e cai aqui → cancelamos a edição e damos blur.
            .on_action(cx.listener(|this, _: &Escape, window, cx| {
                this.cancel_and_blur(window, cx);
            }))
            // Clique em qualquer parte DESTE input não deve "borbulhar" até o `on_mouse_down` raiz
            // do `EditorShell` (que dá blur p/ clique-fora). Sem isto, clicar pra focar o campo
            // focaria e logo perderia o foco. O foco do texto é setado pelo `track_focus` do próprio
            // núcleo (sistema de foco do gpui), então parar a propagação aqui não impede o foco.
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|_, _: &gpui::MouseDownEvent, _window, cx| {
                    cx.stop_propagation();
                }),
            )
            // O scrub usa o mesmo par on_drag/DragMoveEvent dos handles do shell: o ícone inicia o
            // drag; o move chega aqui via `on_drag_move`.
            .on_drag_move(cx.listener(
                move |this, e: &DragMoveEvent<ScrubDrag>, window, cx| {
                    if e.drag(cx).owner != id {
                        return; // arraste de outro ScrubInput — ignora.
                    }
                    let pos: Point<Pixels> = e.event.position;
                    this.scrub_to(pos.x, window, cx);
                },
            ))
            // Fim do scrub: solta a âncora (qualquer mouse-up encerra).
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _e: &gpui::MouseUpEvent, _window, cx| {
                    if this.scrub_anchor.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            // A moldura é o campo da casa: `Input::new(&state)` sem `.label()` renderiza só a caixa,
            // e é ela que traz fundo, borda, bisel e o anel de foco. Nada de superfície aqui.
            .child(
                Input::new(&self.input)
                    .size(SIZE)
                    // As três exceções de densidade deste componente, todas declaradas nas
                    // constantes: respiro, altura e corpo do número.
                    .pad_x(FIELD_PAD_X)
                    .height(FIELD_HEIGHT)
                    .text_size(VALUE_TEXT)
                    .radius(FIELD_RADIUS)
                    // Sem ícone não há prefixo — e não há alça de arraste (ver `without_icon`).
                    .when_some(self.icon.clone(), |inp, path| {
                        inp.prefix(self.scrub_icon(path, cx))
                    })
                    // Diamante de keyframe como SUFIXO (só no modo keyframe-able). Área de clique
                    // própria → não atrapalha scrub/digitação do valor.
                    .when(self.keyframeable, |inp| inp.suffix(self.kf_diamond_suffix(cx)))
                    // Largura fixa quando não é `flex_fill`; no `flex_fill` o campo já ocupa o
                    // container (o default do `Input`).
                    .when(!fill, |inp| inp.width(px(INPUT_WIDTH))),
            )
    }
}

// =================================================================================================
// A matemática do campo — livre de `Window`/`Context`, e por isso testável
// =================================================================================================

/// Clampa a `[min, max]` e snapa ao múltiplo de `step` mais próximo **a partir de `min`**.
/// `step <= 0` desliga o snap (só clampa).
///
/// A âncora é `min`, e não zero: num campo `[0.05, 1.0]` com `step` `0.1`, os valores válidos são
/// `0.05, 0.15, …` — ancorar em zero daria `0.1, 0.2, …`, e o mínimo não seria alcançável.
fn snap_clamp(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let clamped = value.clamp(min, max);
    if step <= 0.0 {
        return clamped;
    }
    let snapped = min + ((clamped - min) / step).round() * step;
    snapped.clamp(min, max)
}

/// Quantos `step` um deslocamento horizontal de `dx` pixels vale. Ver [`PX_PER_STEP`].
fn steps_for_dx(dx: f32) -> f32 {
    (dx / PX_PER_STEP).round()
}

/// O valor de um arraste em curso, a partir da **âncora**.
///
/// `anchor_x`/`start_value` são o x do mouse e o valor no instante do `mouse_down`; `mouse_x` é o x
/// atual. O delta é sempre medido contra a âncora, e **não** acumulado move a move: acumulando, cada
/// arredondamento de `steps_for_dx` viraria erro permanente, e voltar o mouse ao ponto de partida
/// não devolveria o valor de partida.
fn scrubbed_value(
    anchor_x: f32,
    mouse_x: f32,
    start_value: f32,
    min: f32,
    max: f32,
    step: f32,
) -> f32 {
    // direita (dx > 0) aumenta; esquerda diminui.
    let steps = steps_for_dx(mouse_x - anchor_x);
    snap_clamp(start_value + steps * step, min, max, step)
}

/// Interpreta o texto digitado no campo: `Some(valor normalizado)` se parsear, `None` se não.
///
/// O `trim` existe porque colar um número traz espaço em volta com frequência, e recusar `" 12 "`
/// seria recusar um número que o usuário vê como válido. `None` (e não um fallback silencioso pro
/// mínimo) porque quem chama precisa distinguir "digitou outro número" de "digitou bobagem": no
/// segundo caso o valor antigo volta pro campo, sem emitir `Change`.
fn parse_typed(text: &str, min: f32, max: f32, step: f32) -> Option<f32> {
    Some(snap_clamp(parse_finite(text)?, min, max, step))
}

/// **Texto → número finito**, e nada mais: `trim`, parse, e a recusa de `inf`/`NaN`.
///
/// Está separado do [`parse_typed`] porque o clamp e o snap dele são decisões DESTE componente, e o
/// [`crate::number_field`] precisa da mesma leitura de texto com outras: lá a referência não snapa
/// (o `snapOnStep` nasce desligado) e o clamp pode ser dispensado pelo `allowOutOfRange`. O que os
/// dois compartilham é exatamente isto — e é o pedaço que carrega o peso.
///
/// O `trim` existe porque colar um número traz espaço em volta com frequência, e recusar `" 12 "`
/// seria recusar um número que o usuário vê como válido.
///
/// A recusa do não-finito não é preciosismo: `"inf"` e `"NaN"` parseiam como `f32` em Rust, e um
/// `NaN` atravessaria qualquer clamp — no [`snap_clamp`] toda comparação com ele é falsa, e no
/// `clamp_js` do [`crate::meter`] o `f32::min` devolveria o OUTRO operando, ou seja o máximo.
pub(crate) fn parse_finite(text: &str) -> Option<f32> {
    let parsed = text.trim().parse::<f32>().ok()?;
    if !parsed.is_finite() {
        return None;
    }
    Some(parsed)
}

/// Formata o valor pro campo de texto, com casas decimais derivadas do `step` (ex.: step 0.01 → 2
/// casas; step 0.005 → 3; step 1 → 0).
///
/// `pub(crate)` porque é a **única** gramática de número desta casa: o [`crate::number_field`] a
/// reusa em vez de trazer uma segunda (a referência dele pede `Intl.NumberFormat`, que é declarado
/// ausente lá). Dois campos numéricos na mesma app formatando `0.5` de dois jeitos é a divergência
/// que ninguém nota até estar na tela.
pub(crate) fn format_value(value: f32, step: f32) -> String {
    let decimals = decimals_for_step(step);
    format!("{value:.*}", decimals)
}

/// Texto exibido no campo: `"--"` no estado **MISTO** (multi-seleção com valores divergentes),
/// senão o `value` formatado pelo `step`.
fn display_text(value: f32, step: f32, mixed: bool) -> String {
    if mixed {
        "--".to_string()
    } else {
        format_value(value, step)
    }
}

/// Nº de casas decimais "naturais" pro `step` (até 4, evita ruído de ponto float).
///
/// Em cada passo multiplicamos por 10 e **arredondamos** antes de checar a parte fracionária — sem o
/// `round`, valores como `0.01_f32` carregam ruído de ponto flutuante e a fração nunca zera (caía
/// sempre no teto de 4 casas).
fn decimals_for_step(step: f32) -> usize {
    if step <= 0.0 {
        return 2;
    }
    let mut decimals = 0usize;
    let mut s = step as f64;
    while (s - s.round()).abs() > 1e-6 && decimals < 4 {
        s *= 10.0;
        decimals += 1;
    }
    decimals
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar a geometria declarada (a sensibilidade do scrub,
// o tamanho da alça) É o propósito destes testes. Mesma decisão nos outros módulos do crate.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// **O raio é menor que o do campo de formulário.** 10px (`rounded-lg`) numa moldura de 24px
    /// deixa 5px de reta em cada lado e lê como pílula; 8px é `--radius-md`, um passo real da escala.
    ///
    /// Literal de propósito, como a altura e o corpo: comparar com a própria constante não guarda nada.
    #[test]
    fn o_raio_e_menor_que_o_do_campo_de_formulario() {
        assert_eq!(FIELD_RADIUS, 8.0);
        assert!(
            FIELD_RADIUS < SIZE.radius(),
            "o chip tem aresta mais viva que o campo de formulário ({} vs {})",
            FIELD_RADIUS,
            SIZE.radius()
        );
        // E não pode virar pílula: numa moldura de 24, raio ≥ 12 arredondaria a altura inteira.
        assert!(FIELD_RADIUS < FIELD_HEIGHT / 2.0, "raio de pílula");
    }

    /// **A fração da tinta de repouso é metade.** Guardada como número porque foi pedido visual, e
    /// porque `scaled` aparar em `[0,1]` esconderia um valor absurdo.
    #[test]
    fn a_tinta_de_repouso_e_meia_opacidade() {
        assert_eq!(IDLE_ICON_ALPHA, 0.5);
        assert!(IDLE_ICON_ALPHA > 0.0, "tinta invisível não é afordância");
        assert!(IDLE_ICON_ALPHA < 1.0, "cheia, ela competiria com o número");
    }

    /// **O respiro é mais apertado que o do design system, e isso é declarado.**
    ///
    /// O teste guarda os dois números E a conta que os liga, porque o que a revisão visual pediu foi
    /// o RESULTADO (a tinta do glifo perto da borda), não o respiro em si: a distância da borda até
    /// a tinta é `borda + respiro + folga_da_caixa`, e mexer em qualquer um dos três muda o que se vê.
    #[test]
    fn o_respiro_e_menor_que_o_do_campo_de_formulario() {
        assert_eq!(FIELD_PAD_X, 6.0);
        assert!(
            FIELD_PAD_X < SIZE.pad_x(),
            "o chip de inspector é mais apertado que o campo de formulário ({} vs {})",
            FIELD_PAD_X,
            SIZE.pad_x()
        );
        // A conta do que se vê: 1px de borda + respiro + a folga de cada lado do glifo na caixa.
        let folga = (ICON_BOX - ICON_SIZE) / 2.0;
        let ate_a_caixa_do_glifo = 1.0 + FIELD_PAD_X + folga;
        assert_eq!(ate_a_caixa_do_glifo, 8.0, "1 + 6 + 1");
        // E é melhor que o desenho V1, que era o alvo: lá a tinta caía a 11,5px (medido em janela).
        assert!(
            ate_a_caixa_do_glifo < 11.5,
            "a refatoração não pode ficar mais folgada que o desenho que ela substituiu"
        );
    }

    /// **O ícone é composável: qualquer caminho serve.** O [`ScrubIcon`] é atalho, não cerca.
    #[test]
    fn o_icone_aceita_qualquer_caminho() {
        let padrao = ScrubIcon::Strength.path();
        assert_eq!(padrao.as_ref(), "icons/strength.svg", "o atalho da casa");
        // E o call site pode trocar por qualquer asset — inclusive um do Iconoir.
        let custom: SharedString = "iconoir/regular/angle-tool.svg".into();
        assert_ne!(custom, padrao);
    }

    /// **Sem ícone o campo existe — e o scrub não.** A alça É o ícone, então `without_icon` desliga o
    /// arraste. É consequência declarada, não bug: o teste existe pra que ninguém "conserte" a alça
    /// achando que ela sumiu por acidente.
    #[test]
    fn sem_icone_nao_ha_alca() {
        // A representação diz tudo: o ícone é um `Option` de caminho, e é ele que o prefixo consome.
        let com: Option<SharedString> = Some(ScrubIcon::Scrub.path());
        let sem: Option<SharedString> = None;
        assert!(com.is_some(), "com ícone há prefixo, e portanto alça");
        assert!(sem.is_none(), "sem ícone não há prefixo — nem alça");
        // O resto do controle não depende da alça: o clamp/snap do texto digitado é o mesmo.
        assert_eq!(snap_clamp(7.3, 0.0, 10.0, 1.0), 7.0);
    }

    /// **O corpo do número é 11px — a densidade do inspector, não a do design system.**
    ///
    /// Escrito em literal pelo mesmo motivo do teste de altura: comparar com a própria constante não
    /// guarda nada. Este teste existe porque uma mutação (11 → 14, ou seja voltar pro corpo do campo
    /// de formulário) não quebrava NENHUM teste — o valor estava declarado e desguardado.
    #[test]
    fn o_corpo_do_numero_e_de_onze_pixels() {
        assert_eq!(VALUE_TEXT, 11.0);
        assert!(
            VALUE_TEXT < SIZE.text_size(),
            "o chip é mais denso que o campo de formulário ({} vs {})",
            VALUE_TEXT,
            SIZE.text_size()
        );
        // E a altura acompanha: um corpo de 11 dentro de um miolo de 22 deixa folga pros dois lados.
        assert!(VALUE_TEXT < FIELD_HEIGHT - 2.0);
    }

    // --- A matemática do valor ------------------------------------------------------------------

    #[test]
    fn snap_clamp_respeita_limites_e_passo() {
        // clampa em cima/embaixo.
        assert_eq!(snap_clamp(5.0, 0.0, 2.0, 0.1), 2.0);
        assert_eq!(snap_clamp(-1.0, 0.0, 2.0, 0.1), 0.0);
        // snapa ao múltiplo de step mais próximo.
        assert!((snap_clamp(0.123, 0.0, 1.0, 0.01) - 0.12).abs() < 1e-5);
        assert!((snap_clamp(0.126, 0.0, 1.0, 0.01) - 0.13).abs() < 1e-5);
        // step 0 = só clampa.
        assert_eq!(snap_clamp(0.137, 0.0, 1.0, 0.0), 0.137);
    }

    /// O snap é ancorado no **mínimo**, não em zero: com `[0.05, 1.05]` e passo `0.1`, os válidos
    /// são `0.05, 0.15, …`. Ancorado em zero, o próprio mínimo não seria representável.
    #[test]
    fn snap_ancora_no_minimo_e_nao_no_zero() {
        assert!((snap_clamp(0.06, 0.05, 1.05, 0.1) - 0.05).abs() < 1e-5);
        assert!((snap_clamp(0.12, 0.05, 1.05, 0.1) - 0.15).abs() < 1e-5);
        // Se a âncora fosse zero, isto daria 0.1 — que não é alcançável nesta faixa.
        assert!((snap_clamp(0.09, 0.05, 1.05, 0.1) - 0.05).abs() < 1e-5);
    }

    /// O clamp vem **depois** do snap, e é por isso que ele aparece duas vezes na função.
    ///
    /// Faixa `[0, 1]` com passo `0.4`: o topo cai no MEIO de um passo (`1 / 0.4 = 2.5`), então o snap
    /// arredonda pra `1.2` — fora da faixa. Sem o segundo clamp, um campo `[0, 1]` entregaria `1.2`
    /// ao shader.
    #[test]
    fn snap_nunca_escapa_da_faixa() {
        let v = snap_clamp(1.0, 0.0, 1.0, 0.4);
        assert!(v <= 1.0, "snapou pra fora do max: {v}");
        assert!((v - 1.0).abs() < 1e-5, "o máximo tem que ser alcançável; veio {v}");
        // Por baixo não há como escapar: o snap é ancorado no mínimo, então o arredondamento nunca
        // desce dele. Fica afirmado pra a assimetria ser intencional e não sorte.
        assert_eq!(snap_clamp(-5.0, 0.0, 1.0, 0.4), 0.0);
    }

    #[test]
    fn decimais_derivam_do_step() {
        assert_eq!(decimals_for_step(0.01), 2);
        assert_eq!(decimals_for_step(0.005), 3);
        assert_eq!(decimals_for_step(1.0), 0);
        assert_eq!(decimals_for_step(0.1), 1);
    }

    /// Multi-seleção — `display_text`: estado MISTO mostra "--"; senão o valor formatado pelo step.
    #[test]
    fn display_text_misto_mostra_tracinho() {
        assert_eq!(display_text(12.0, 1.0, true), "--");
        assert_eq!(display_text(12.0, 1.0, false), "12");
        assert_eq!(display_text(0.5, 0.01, false), "0.50");
        // Misto ignora o valor (sempre "--").
        assert_eq!(display_text(999.0, 0.1, true), "--");
    }

    // --- O parse do texto digitado --------------------------------------------------------------

    /// O que o usuário digita é clampado e snapado igual ao que vem de fora — o campo não tem uma
    /// segunda régua pro teclado.
    #[test]
    fn texto_digitado_e_clampado_e_snapado() {
        assert_eq!(parse_typed("500", 0.0, 100.0, 1.0), Some(100.0));
        assert_eq!(parse_typed("-7", 0.0, 100.0, 1.0), Some(0.0));
        let v = parse_typed("0.126", 0.0, 1.0, 0.01).expect("número válido");
        assert!((v - 0.13).abs() < 1e-5, "veio {v}");
    }

    /// Espaço em volta é aceito: colar um número quase sempre traz espaço, e recusar `" 12 "` seria
    /// recusar um número que o usuário vê como válido.
    #[test]
    fn texto_com_espaco_em_volta_e_aceito() {
        assert_eq!(parse_typed("  12  ", 0.0, 100.0, 1.0), Some(12.0));
        assert_eq!(parse_typed("\t8\n", 0.0, 100.0, 1.0), Some(8.0));
    }

    /// Texto que não é número devolve `None` — é o que faz o campo restaurar o valor antigo em vez
    /// de emitir um `Change` com lixo.
    #[test]
    fn texto_invalido_nao_produz_valor() {
        for t in ["", "   ", "abc", "1.2.3", "12px", "--", "1,5"] {
            assert_eq!(parse_typed(t, 0.0, 100.0, 1.0), None, "aceitou {t:?}");
        }
    }

    /// `"inf"` e `"NaN"` parseiam como `f32` em Rust, e um NaN atravessaria o `clamp` e envenenaria
    /// o valor do campo (nenhuma comparação com NaN é verdadeira, então nem o guarda de mudança o
    /// pararia). São recusados no parse.
    #[test]
    fn infinito_e_nan_sao_recusados() {
        for t in ["inf", "-inf", "NaN", "infinity"] {
            assert_eq!(parse_typed(t, 0.0, 100.0, 1.0), None, "aceitou {t:?}");
        }
    }

    // --- O scrub por arraste --------------------------------------------------------------------

    /// **Três pixels de arraste valem um `step`** — a sensibilidade declarada em [`PX_PER_STEP`].
    #[test]
    fn tres_pixels_de_arraste_valem_um_passo() {
        assert_eq!(PX_PER_STEP, 3.0);
        assert_eq!(steps_for_dx(3.0), 1.0);
        assert_eq!(steps_for_dx(9.0), 3.0);
        assert_eq!(steps_for_dx(-6.0), -2.0);
        // Menos de meio passo não move nada (arredondamento pro passo mais próximo).
        assert_eq!(steps_for_dx(1.0), 0.0);
    }

    /// Direita aumenta, esquerda diminui — com o passo do campo, não com pixels.
    #[test]
    fn arrastar_para_a_direita_aumenta_e_para_a_esquerda_diminui() {
        let (min, max, step) = (0.0, 100.0, 1.0);
        // 30px à direita = 10 passos = +10.
        assert_eq!(scrubbed_value(100.0, 130.0, 50.0, min, max, step), 60.0);
        // 30px à esquerda = -10.
        assert_eq!(scrubbed_value(100.0, 70.0, 50.0, min, max, step), 40.0);
        // Parado na âncora = o valor de partida.
        assert_eq!(scrubbed_value(100.0, 100.0, 50.0, min, max, step), 50.0);
    }

    /// O arraste é medido contra a **âncora**, não acumulado move a move. Duas consequências
    /// testadas: a posição absoluta da âncora é irrelevante (só o delta conta), e voltar o mouse pro
    /// x inicial devolve **exatamente** o valor inicial.
    #[test]
    fn arraste_e_relativo_a_ancora() {
        let (min, max, step) = (0.0, 100.0, 1.0);
        // Mesmo delta (+12px = +4 passos) a partir de âncoras bem distantes → mesmo valor.
        let perto = scrubbed_value(10.0, 22.0, 20.0, min, max, step);
        let longe = scrubbed_value(900.0, 912.0, 20.0, min, max, step);
        assert_eq!(perto, longe);
        assert_eq!(perto, 24.0);
        // Ida e volta: o valor volta ao de partida, sem erro acumulado de arredondamento.
        let ida = scrubbed_value(200.0, 204.0, 7.0, min, max, step);
        assert_ne!(ida, 7.0, "andou");
        assert_eq!(scrubbed_value(200.0, 200.0, 7.0, min, max, step), 7.0);
    }

    /// O passo do arraste é o `step` do campo — não 1, não pixels. Com `step` `0.05`, 6px (2 passos)
    /// valem `0.1`.
    #[test]
    fn o_arraste_anda_em_passos_do_campo() {
        let v = scrubbed_value(0.0, 6.0, 0.5, 0.0, 1.0, 0.05);
        assert!((v - 0.6).abs() < 1e-5, "veio {v}");
        // E o resultado continua sendo múltiplo do passo (snap).
        let v = scrubbed_value(0.0, 7.0, 0.5, 0.0, 1.0, 0.05);
        assert!((v / 0.05 - (v / 0.05).round()).abs() < 1e-3, "fora do passo: {v}");
    }

    /// Um arraste longo para em `min`/`max` — não há como arrastar pra fora da faixa.
    #[test]
    fn arraste_respeita_os_limites() {
        let (min, max, step) = (0.0, 10.0, 1.0);
        assert_eq!(scrubbed_value(0.0, 3000.0, 5.0, min, max, step), max);
        assert_eq!(scrubbed_value(3000.0, 0.0, 5.0, min, max, step), min);
    }

    // --- A geometria que vem do design system ---------------------------------------------------

    /// O campo é o [`InputSize::Sm`] da casa: 26px de miolo + as duas bordas = **28** de altura
    /// externa, corpo **14**, raio **10**.
    ///
    /// Estes números não são escolha deste módulo — são do design system, e o teste existe pra
    /// registrar QUAIS são (o componente media 26 de altura e 12 de corpo antes da migração; se
    /// alguém trocar o tamanho aqui, é este teste que conta a diferença).
    #[test]
    fn a_moldura_e_o_tamanho_pequeno_do_campo_da_casa() {
        assert_eq!(SIZE, InputSize::Sm);
        assert_eq!(SIZE.content_height(), 26.0);
        assert_eq!(SIZE.height(), 28.0, "miolo + as duas bordas de 1px");
        assert_eq!(SIZE.text_size(), 14.0, "o corpo do campo, igual nos três tamanhos");
        assert_eq!(SIZE.radius(), 10.0, "`rounded-lg`, e não mais o RADIUS_FIELD de 7");
    }

    /// A caixa da alça cabe no miolo do campo.
    ///
    /// Se não couber, o excedente é **recortado em silêncio**: a moldura do [`crate::Input`] tem
    /// altura fixa e `overflow_hidden`, então uma alça grande demais não estica nada — ela some pelas
    /// beiradas, e o `tests_de_janela` continua medindo os mesmos 28px. Este é o único lugar onde
    /// esse limite é verificado.
    #[test]
    fn a_caixa_da_alca_cabe_no_miolo_do_campo() {
        // ⚠️ O miolo é o do FIELD_HEIGHT deste componente (22), **não** o do `InputSize::Sm` (26).
        // Este teste comparava com o do `Sm` e ficou obsoleto quando a altura virou 24: passaria com
        // uma alça de 24, que estouraria o campo. É a diferença entre travar o número e travar o
        // ENCAIXE.
        let miolo = FIELD_HEIGHT - 2.0;
        assert_eq!(miolo, 22.0);
        assert!(
            ICON_BOX <= miolo,
            "alça de {ICON_BOX} num miolo de {miolo}"
        );
        assert!(KF_DIAMOND <= miolo, "diamante de {KF_DIAMOND} num miolo de {miolo}");
    }

    /// O alvo de arraste é **maior** que o desenho: um alvo do tamanho exato do glifo obrigaria o
    /// usuário a acertar o traço pra começar um scrub.
    #[test]
    fn o_alvo_da_alca_e_maior_que_o_glifo() {
        assert!(ICON_BOX > ICON_SIZE, "{ICON_BOX} não é maior que {ICON_SIZE}");
    }

    // --- A tinta -------------------------------------------------------------------------------

    /// Repouso e ativo têm que ser cores **distintas** nos dois temas: é a única diferença entre
    /// "este canal tem keyframe" e "não tem", e entre a alça em repouso e sob o mouse.
    ///
    /// E a de repouso é **metade** da opacidade do token de ícone: a alça é afordância secundária (o
    /// assunto do campo é o número), e cheia ela competia com ele. O teste guarda a FRAÇÃO, não um
    /// segundo valor de cor — se o token do tema mudar, a metade acompanha.
    #[test]
    fn a_tinta_distingue_repouso_de_ativo_nos_dois_temas() {
        for modo in [theme::ThemeMode::Dark, theme::ThemeMode::Light] {
            theme::set_theme(modo);
            let p = scrub();
            assert_ne!(p.idle, p.active.hsla(), "tinta indistinguível no tema {modo:?}");
            // A de repouso é meia opacidade do token; a ativa é opaca.
            assert!(
                (p.idle.a - 0.5).abs() < 0.01,
                "repouso deveria ser meia opacidade, veio {}",
                p.idle.a
            );
            assert_eq!(p.active.alpha(), 1.0, "a ativa é opaca");
            // E o RGB da de repouso continua sendo o do token — o que muda é só o alfa.
            let token = opaque(theme::ICON()).hsla();
            assert!(
                (p.idle.h - token.h).abs() < 1e-6 && (p.idle.l - token.l).abs() < 1e-6,
                "o repouso tem que ser o TOKEN a meia opacidade, não outra cor"
            );
        }
        theme::set_theme(theme::ThemeMode::Dark); // não deixa estado vazando pros outros testes
    }

    /// A tinta ativa é o `--foreground` do **próprio campo**, lido de `crate::input` — não uma cópia
    /// do número. Se as duas divergirem, o diamante aceso deixa de ler igual ao número ao lado.
    #[test]
    fn a_tinta_ativa_e_o_foreground_do_campo() {
        for modo in [theme::ThemeMode::Dark, theme::ThemeMode::Light] {
            theme::set_theme(modo);
            assert_eq!(scrub().active, crate::input::field().text);
        }
        theme::set_theme(theme::ThemeMode::Dark);
    }

    // --- O catálogo de ícones -------------------------------------------------------------------

    /// Todo [`ScrubIcon`] aponta pra um SVG que está **embutido**. Um caminho errado não dá erro: a
    /// `AssetSource` devolve `None` e o glifo desaparece em silêncio — exatamente o tipo de bug que
    /// só aparece na tela, e só no campo que ninguém abriu ainda.
    #[test]
    fn todo_icone_de_canal_esta_embutido() {
        for icon in ScrubIcon::ALL {
            let path = icon.path();
            assert!(
                crate::assets::ICONS.iter().any(|(p, _)| *p == path.as_ref()),
                "{icon:?} aponta pra {path}, que não está embutido"
            );
        }
        // Os dois glifos do diamante não vêm do enum — entram pelo `kf_diamond_suffix`.
        for path in ["icons/kf_diamond.svg", "icons/kf_diamond_filled.svg"] {
            assert!(
                crate::assets::ICONS.iter().any(|(p, _)| *p == path),
                "{path} não está embutido"
            );
        }
    }

    /// Cada variante tem um glifo **próprio**. Duas variantes com o mesmo caminho é copy-paste: o
    /// campo mostra o ícone do canal vizinho, e nada quebra.
    #[test]
    fn cada_icone_de_canal_tem_glifo_proprio() {
        let mut vistos: Vec<SharedString> = Vec::new();
        for icon in ScrubIcon::ALL {
            let path = icon.path();
            assert!(!vistos.contains(&path), "{icon:?} repete o glifo {path}");
            vistos.push(path);
        }
        assert_eq!(vistos.len(), 19, "o catálogo tem 19 canais");
    }
}

/// Testes de **janela**: a geometria como ela sai no layout, e não como está declarada.
///
/// A altura do campo é a soma de coisas que moram em lugares diferentes — o miolo do
/// [`InputSize::Sm`], as duas bordas da moldura, a caixa da alça, o diamante — e um teste de
/// constante contra constante não vê quando uma delas passa a esticar a moldura. Este mede.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{canvas, Bounds, TestAppContext, VisualTestContext};
    use std::cell::Cell;
    use std::rc::Rc;

    /// Largura da janela do harness — folgada, pra o campo de 150px nunca disputar espaço.
    const LARGURA: f32 = 400.0;

    /// O medidor: o campo e, LOGO ABAIXO dele numa coluna sem `gap`, uma sonda de altura zero. A
    /// origem `y` da sonda é, portanto, a altura total que o campo ocupou.
    struct Medidor {
        campo: Entity<ScrubInput>,
        sonda: Rc<Cell<Option<Bounds<Pixels>>>>,
    }

    impl Render for Medidor {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            let sonda = self.sonda.clone();
            div()
                .flex()
                .flex_col()
                .w(px(LARGURA))
                .child(self.campo.clone())
                .child(
                    canvas(move |b, _w, _cx| sonda.set(Some(b)), |_, _, _, _| {})
                        .w_full()
                        .h(px(0.0)),
                )
        }
    }

    /// Abre uma janela com um `ScrubInput` e devolve a altura que ele ocupou, em px lógicos.
    fn altura_medida(cx: &mut TestAppContext, keyframeable: bool) -> f32 {
        let sonda: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let s = sonda.clone();
        let window = cx.add_window(move |window, cx| {
            let campo = cx.new(|cx| {
                let c = ScrubInput::new(
                    "Opacidade",
                    ScrubIcon::Droplet,
                    50.0,
                    0.0,
                    100.0,
                    1.0,
                    window,
                    cx,
                );
                if keyframeable {
                    c.keyframeable()
                } else {
                    c
                }
            });
            Medidor { campo, sonda: s }
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        f32::from(sonda.get().expect("a sonda mediu no prepaint").origin.y)
    }

    /// **O campo mede 28px de altura** — a externa do [`InputSize::Sm`] (miolo 26 + as duas bordas
    /// de 1px). Antes da migração eram 26: um miolo de 24 desenhado à mão + 2 de borda.
    ///
    /// O 28 está **literal** de propósito. A primeira versão deste teste comparava o medido com
    /// `SIZE.height()`, e isso é tautológico: trocar o tamanho do controle move os dois lados juntos
    /// e o teste passa calado (verificado — com `InputSize::Md` ele media 32 e aprovava). Com o
    /// número escrito, mudar a densidade do inspector obriga a mudar esta linha.
    ///
    /// O modo keyframe-able entra no laço porque o diamante é um sufixo de 16px. Note que um sufixo
    /// maior que o miolo **não** esticaria a moldura — a altura dela é fixa, e o excedente seria
    /// recortado em silêncio pelo `overflow_hidden` do campo. Quem guarda esse limite é
    /// [`super::tests::a_caixa_da_alca_cabe_no_miolo_do_campo`]; aqui só se afirma que ligar o
    /// diamante não muda a altura da linha.
    #[gpui::test]
    fn a_altura_medida_do_campo_e_de_24px(cx: &mut TestAppContext) {
        /// A altura que o inspector tem por linha de campo numérico — a especificação do painel, não
        /// a do `InputSize::Sm` (que dá 28).
        ///
        /// ⚠️ **Literal de propósito.** Escrever `FIELD_HEIGHT` aqui deixa o teste tautológico: mutar a
        /// constante mudaria o código E a expectativa juntos, e ele passaria calado com qualquer
        /// altura. Já aconteceu duas vezes neste mesmo teste — a primeira com `SIZE.height()`.
        const ALTURA: f32 = 24.0;

        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        for keyframeable in [false, true] {
            let medida = altura_medida(cx, keyframeable);
            assert!(
                (medida - ALTURA).abs() < 0.5,
                "com keyframeable={keyframeable} o campo mediu {medida}px, esperado {ALTURA}px"
            );
        }
    }





    /// **Focar o campo seleciona o número inteiro.**
    ///
    /// Não dá pra ler a seleção de fora (o `selected_range` do núcleo é `pub(super)`), então o teste
    /// afirma o COMPORTAMENTO, que é o que interessa: com tudo selecionado, digitar um dígito
    /// SUBSTITUI o valor. Sem a seleção, o dígito entraria no cursor e o campo diria `500` ou `050`
    /// em vez de `5`.
    ///
    /// **Clica**, em vez de chamar `focus()`: o caminho de foco é reconstruído no desenho, e um
    /// `focus()` deixa `is_focused` verdadeiro sem produzir esse caminho. O clique também prova a
    /// ORDEM — o campo posiciona o cursor no próprio mouse-down, e a seleção tem que sobreviver a isso.
    #[gpui::test]
    fn focar_o_campo_seleciona_o_numero(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });

        let campo: Rc<Cell<Option<Entity<ScrubInput>>>> = Rc::new(Cell::new(None));
        let c = campo.clone();
        let sonda: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let s = sonda.clone();
        let window = cx.add_window(move |window, cx| {
            let e = cx.new(|cx| {
                ScrubInput::new("Opacidade", ScrubIcon::Droplet, 50.0, 0.0, 100.0, 1.0, window, cx)
            });
            c.set(Some(e.clone()));
            // ⚠️ Envolto em `Root`: o clique atravessa o núcleo do `gpui-component`, que faz `unwrap`
            // nas camadas de popover/modal dele (`root.rs:268`). Sem o `Root` o teste estoura ali, e o
            // pânico não diz que o que falta é isto. Os outros testes de janela deste módulo não
            // precisam porque só MEDEM, sem interagir.
            gpui_component::Root::new(cx.new(|_| Medidor { campo: e, sonda: s }), window, cx)
        });
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let e = campo.take().expect("o campo foi construído");
        let estado = vcx.update(|_w, cx| e.read(cx).input.clone());
        assert_eq!(vcx.update(|_w, cx| estado.read(cx).value().to_string()), "50");

        // Clica no MEIO da caixa, à direita da alça (a alça inicia arraste, não digitação).
        //
        // ⚠️ A sonda deste harness é um canvas de altura ZERO colocado DEPOIS do campo: o `origin.y`
        // dela é a ALTURA do campo, não a posição dele. Usar `origin.y + height/2` punha o clique na
        // borda de baixo, fora da caixa — e o teste falhava dizendo que a seleção não aconteceu, quando
        // o que não acontecia era o clique.
        let altura = f32::from(sonda.get().expect("a sonda mediu").origin.y);
        assert!((altura - FIELD_HEIGHT).abs() < 0.5, "a sonda mediu {altura}");
        // O campo começa em (0,0) e mede INPUT_WIDTH de largura; 100 está dentro dele e à direita da
        // alça (que ocupa até ~25 com o respiro).
        let x = 100.0;
        let y = altura / 2.0;
        vcx.simulate_click(gpui::point(px(x), px(y)), gpui::Modifiers::default());
        vcx.run_until_parked();

        vcx.simulate_input("5");
        vcx.run_until_parked();
        let depois = vcx.update(|_w, cx| estado.read(cx).value().to_string());
        assert_eq!(
            depois, "5",
            "o dígito devia SUBSTITUIR a seleção; veio {depois:?}, ou seja a seleção não aconteceu"
        );
    }
}

//! `Group` — costura vários controles numa peça só.
//!
//! Porte do `group.tsx` do coss. É o componente que junta botão+botão, campo+botão, rótulo+campo,
//! select+campo, campo+campo — sem que cada um desses pares precise de um componente próprio.
//!
//! # O que "costurar" quer dizer
//!
//! No coss a costura é feita por seletores de irmão, que mexem nos FILHOS:
//!
//! ```text
//! *:data-slot:has-[~[data-slot]]:rounded-e-none   → quem tem vizinho depois perde o raio de fim
//! *:data-slot:has-[~[data-slot]]:border-e-0       → …e a borda de fim
//! *:[[data-slot]~[data-slot]]:rounded-s-none      → quem vem depois de alguém perde o raio de início
//! *:[[data-slot]~[data-slot]]:border-s-0          → …e a borda de início
//! ```
//!
//! Ou seja: **entre dois filhos não sobra borda nenhuma**. A linha que se vê na emenda é sempre o
//! separador — e é por isso que o doc do coss diz, em letras tantas, que o `GroupSeparator` é
//! *required between all controls*. Aqui ele é **automático** (ver [`Group::seamless`] pra desligar):
//! um separador obrigatório que se pode esquecer é um defeito esperando pra acontecer.
//!
//! O GPUI não tem seletor de irmão, então a costura é explícita: o `Group` calcula um [`Join`] por
//! filho (tem vizinho antes? depois?) e passa pra ele. Cada componente costurável sabe aplicar o
//! `Join` na própria geometria — ver [`Join::rounded`], [`Join::borders`] e [`Join::overlay`].
//!
//! # Group não é InputGroup
//!
//! Duas coisas parecidas que resolvem problemas diferentes, e o coss tem as duas:
//!
//! | | superfícies | borda | anel de foco | Tab |
//! |---|---|---|---|---|
//! | [`crate::Input`] com `prefix`/`suffix`/`addon_*` | **uma** | uma | envolve o conjunto | 1 parada |
//! | `Group` | **várias** | uma por filho | um por filho | uma parada por filho |
//!
//! "Botão no fim do campo" tem as duas leituras: DENTRO do campo é
//! [`crate::Input::icon_button`] no `suffix`; COLADO ao campo é `Group`. Escolha pelo que o conjunto
//! é: um controle só, ou vários controles vizinhos.
//!
//! # Aninhamento é como se cria espaço
//!
//! `has-[>[data-slot=group]]:gap-2` — um Group que contém outro Group ganha 8px de vão e **para de
//! costurar**. É o idioma do coss pra separar blocos (paginação, "campo + botão de enviar"), e é
//! por isso que os exemplos dele têm Groups internos com um único filho.
//!
//! ```ignore
//! Group::new()                                  // externo: só o vão de 8px
//!     .child(Group::new()                       // bloco 1: costurado
//!         .child(Button::new("um", "1"))
//!         .child(Button::new("dois", "2")))
//!     .child(Group::new()                       // bloco 2: costurado
//!         .child(Button::icon("prev", "icons/chevron_left.svg")))
//! ```
//!
//! # O que ficou de fora, e por quê
//!
//! - **O anel de foco não passa por cima do vizinho.** No coss `*:focus-visible:z-1` levanta o filho
//!   focado, e o anel dele desenha SOBRE os vizinhos. O GPUI 0.2 não tem `z-index`, e a ordem de
//!   pintura é a ordem dos filhos. Então aqui o anel para na emenda (ver o `seam` de
//!   [`Join::overlay`]) e quem fecha o contorno naquele lado é o separador, que vira a cor do anel
//!   quando o vizinho está focado — que é exatamente o que o coss faz pros CAMPOS
//!   (`has-[+[data-slot=input-control]:focus-within]:bg-ring`). A diferença sobra só pros botões.
//! - **O deslocamento de 1px do separador** (`translate-x-px` quando o vizinho está focado).
//!   Precisaria de margem negativa, que já colapsou layout nesta base (ver `crate::tabs`), e o
//!   ganho é meio pixel.

use crate::color::Rgba8;
use crate::theme;
use gpui::{
    div, px, App, Div, ElementId, FocusHandle, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window,
};

// =================================================================================================
// Orientação e costura
// =================================================================================================

/// Em que direção o grupo empilha os filhos.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Orientation {
    /// Lado a lado. `start` = esquerda, `end` = direita.
    #[default]
    Horizontal,
    /// Um sobre o outro. `start` = topo, `end` = base.
    Vertical,
}

/// **Como um filho encosta nos vizinhos.** É o que o [`Group`] passa pra cada um.
///
/// `start`/`end` são lógicos, não geográficos: no horizontal são esquerda/direita, no vertical são
/// topo/base. Um mesmo `Join` serve pras duas orientações porque a geometria é a mesma girada.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Join {
    /// Tem vizinho **antes**: perde raio e borda desse lado.
    pub start: bool,
    /// Tem vizinho **depois**: idem.
    pub end: bool,
    /// Em que eixo a costura acontece.
    pub orientation: Orientation,
}

/// Quanto um overlay avança PRA DENTRO do vizinho, na emenda, pra não deixar meia-linha de fresta.
///
/// É o `before:-end-[0.5px]` / `before:-start-[0.5px]` do coss. Sem isso, o bisel de dois filhos
/// vizinhos pode não se encontrar quando a emenda cai numa fração de pixel de dispositivo.
const SEAM_BLEED: f32 = 0.5;

impl Join {
    /// Um filho **solto**: mantém raio e bordas nos quatro lados. É o valor de quem não está num
    /// grupo, e o `Default`.
    pub const NONE: Join = Join {
        start: false,
        end: false,
        orientation: Orientation::Horizontal,
    };

    /// Se este filho encosta em alguém.
    pub fn is_joined(self) -> bool {
        self.start || self.end
    }

    /// Aplica o raio **canto por canto** numa superfície de raio `radius`: os cantos do lado da
    /// emenda vão a zero.
    ///
    /// Genérico em [`Styled`] porque as raízes dos componentes não são todas `Div`: a do
    /// [`crate::Button`] é `Stateful<Div>`, porque ela tem `id` e estado de hover.
    pub fn rounded<T: Styled>(self, d: T, radius: f32) -> T {
        let r = px(radius);
        let flat = px(0.0);
        let (inicio, fim) = (
            if self.start { flat } else { r },
            if self.end { flat } else { r },
        );
        match self.orientation {
            Orientation::Horizontal => d
                .rounded_tl(inicio)
                .rounded_bl(inicio)
                .rounded_tr(fim)
                .rounded_br(fim),
            Orientation::Vertical => d
                .rounded_tl(inicio)
                .rounded_tr(inicio)
                .rounded_bl(fim)
                .rounded_br(fim),
        }
    }

    /// Liga as bordas de 1px **lado por lado**: o lado da emenda fica SEM borda.
    ///
    /// Substitui o `.border_1()` de quem entra num grupo. A cor continua vindo de um
    /// `.border_color(…)` do chamador — lado com largura zero não pinta.
    pub fn borders<T: Styled>(self, d: T) -> T {
        let (transversal, inicio, fim) = match self.orientation {
            Orientation::Horizontal => (d.border_t_1().border_b_1(), Lado::Esquerda, Lado::Direita),
            Orientation::Vertical => (d.border_l_1().border_r_1(), Lado::Topo, Lado::Base),
        };
        let d = if self.start {
            transversal
        } else {
            inicio.borda(transversal)
        };
        if self.end {
            d
        } else {
            fim.borda(d)
        }
    }

    /// Um **overlay absoluto** (bisel, realce, anel) ciente da emenda.
    ///
    /// `inset` é o recuo dos lados livres — negativo cresce pra fora, e é assim que estes overlays
    /// cobrem a *border box* (ver `crate::button::bevel_overlay`). Na emenda vale `seam`, porque ali
    /// não existe borda pra cobrir; os dois valores que fazem sentido são:
    ///
    /// - `-SEAM_BLEED` pro **bisel e o realce**: atravessam meio pixel pra fechar a linha com a do
    ///   vizinho, como o `before:-end-[0.5px]` do coss.
    /// - `0.0` pro **anel de foco**: encosta na emenda e para. Sem `z-index` no GPUI ele não teria
    ///   como aparecer por cima do vizinho de qualquer forma (ver o doc do módulo).
    pub fn overlay(self, inset: f32, seam: f32, radius: f32) -> Div {
        let (inicio, fim) = (
            if self.start { seam } else { inset },
            if self.end { seam } else { inset },
        );
        let d = div().absolute();
        let d = match self.orientation {
            Orientation::Horizontal => d
                .top(px(inset))
                .bottom(px(inset))
                .left(px(inicio))
                .right(px(fim)),
            Orientation::Vertical => d
                .left(px(inset))
                .right(px(inset))
                .top(px(inicio))
                .bottom(px(fim)),
        };
        self.rounded(d, radius)
    }

    /// O overlay de um **bisel ou realce**: atravessa a emenda por meio pixel.
    pub fn bevel_overlay(self, inset: f32, radius: f32) -> Div {
        self.overlay(inset, -SEAM_BLEED, radius)
    }

    /// O overlay do **anel de foco**: encosta na emenda e para.
    pub fn ring_overlay(self, inset: f32, radius: f32) -> Div {
        self.overlay(inset, 0.0, radius)
    }
}

/// Qual lado de uma `Div` ligar — existe só pra [`Join::borders`] não repetir quatro `if`.
#[derive(Clone, Copy)]
enum Lado {
    Esquerda,
    Direita,
    Topo,
    Base,
}

impl Lado {
    fn borda<T: Styled>(self, d: T) -> T {
        match self {
            Lado::Esquerda => d.border_l_1(),
            Lado::Direita => d.border_r_1(),
            Lado::Topo => d.border_t_1(),
            Lado::Base => d.border_b_1(),
        }
    }
}

// =================================================================================================
// Paleta
// =================================================================================================

/// Tokens do grupo. Convenção da casa: `0xRRGGBBAA` (ver [`crate::color`]).
struct GroupPalette {
    /// `--input` — o separador, e a borda do [`GroupText`].
    input: Rgba8,
    /// O separador quando um vizinho está sob o mouse ou pressionado. No claro é igual ao repouso
    /// (o coss só tem a regra `dark:`); no escuro clareia.
    input_hover: Rgba8,
    /// O separador quando um vizinho está **LIGADO** — o terceiro degrau, e o mais claro dos três.
    ///
    /// Existe pro [`crate::toggle_group::ToggleGroup`]: em base-ui `data-pressed` num toggle
    /// significa SELECIONADO, e a regra
    /// `dark:*:[[data-slot=separator]:has(+[data-slot=toggle][data-pressed])]:before:bg-input`
    /// sobe o `::before` pra `--input` CHEIO (contra o `/64` do hover). No claro é igual ao repouso,
    /// pela mesma razão que [`Self::input_hover`]: a regra do coss é `dark:`.
    input_on: Rgba8,
    /// `--ring` — o separador quando o CAMPO vizinho está focado, pra o contorno do foco não abrir
    /// um buraco na emenda.
    ring: Rgba8,
    /// `--primary` a 72% — o separador de [`SeparatorTone::OnPrimary`].
    primary_72: Rgba8,
    /// Fundo do [`GroupText`]: `--muted` no claro, `--input`/64 no escuro.
    text_bg: Rgba8,
    /// `--muted-foreground` — o texto do [`GroupText`].
    text_fg: Rgba8,
    /// Sombra externa do [`GroupText`] (`shadow-xs/5`).
    shadow: Rgba8,
    /// Fio de bisel do [`GroupText`].
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro), `-1` sobe (escuro). Ver a armadilha nº 4 da casa — é o
    /// SINAL que escolhe o lado, não um segundo campo que poderia divergir dele.
    bevel_dir: f32,
}

const GROUP_LIGHT: GroupPalette = GroupPalette {
    input: Rgba8(0x0000001a),       // black 10%
    input_hover: Rgba8(0x0000001a), // igual: o realce de hover do separador é só do tema escuro
    input_on: Rgba8(0x0000001a),    // idem — o degrau de "vizinho ligado" também é só do escuro
    ring: Rgba8(0xa3a3a3ff),       // neutral-400
    primary_72: Rgba8(0x262626b8),  // neutral-800 a 72%
    text_bg: Rgba8(0x0000000a),    // --muted = black 4%
    text_fg: Rgba8(0x686868ff),    // mix(neutral-500 90%, black)
    shadow: Rgba8(0x0000000d),     // black 5%
    bevel: Rgba8(0x0000000f),      // black 6%
    bevel_dir: 1.0,
};

const GROUP_DARK: GroupPalette = GroupPalette {
    // `bg-input` (branco 8%) com o `dark:before:bg-input/32` (branco 2,56%) por cima: as duas
    // camadas compõem ~10,4%. Uma camada só, com o alfa já composto — o `::before` do coss existe
    // pra ter o que recolorir no hover, e aqui esse papel é do `input_hover`.
    input: Rgba8(0xffffff1a), // ~10,2%
    // No hover/pressed o `::before` sobe pra `input/64` (branco 5,12%) → ~12,7% composto.
    input_hover: Rgba8(0xffffff20), // ~12,5%
    // E com o vizinho LIGADO o `::before` vai a `input` cheio (branco 8%) → ~15,4% composto.
    input_on: Rgba8(0xffffff27), // ~15,3%
    ring: Rgba8(0x737373ff),        // neutral-500
    primary_72: Rgba8(0xf5f5f5b8),  // neutral-100 a 72%
    // `dark:bg-input/64`: --input escuro é branco 8%, e o /64 o multiplica → ~5,1%.
    text_bg: Rgba8(0xffffff0d),
    text_fg: Rgba8(0x818181ff), // mix(neutral-500 90%, white)
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE, o mesmo do resto da lib: a referência usa branco a 6% (alfa 15) e aqui
    // é o DOBRO, porque a 6% o filete é imperceptível no nosso fundo escuro. Ver a armadilha nº 5.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
};

fn palette() -> &'static GroupPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &GROUP_DARK,
        theme::ThemeMode::Light => &GROUP_LIGHT,
    }
}

/// Raio das superfícies do grupo — `rounded-lg` = `--radius-lg` = **10px**.
const RADIUS: f32 = 10.0;

/// Espessura do separador — `w-px`/`h-px`.
///
/// `pub(crate)` porque o [`crate::toggle_group::ToggleGroup`] reusa o [`separador`] daqui e trava
/// esta espessura contra a referência num teste dele — o valor é um só, e o teste tem que olhar
/// justamente este.
pub(crate) const SEPARATOR: f32 = 1.0;

/// Vão entre blocos quando o grupo contém outro grupo — `gap-2`.
const NESTED_GAP: f32 = 8.0;

/// Respiro horizontal do [`GroupText`] — `px-[calc(--spacing(3)-1px)]`, o `-1px` descontando a
/// borda.
const TEXT_PAD_X: f32 = 11.0;

/// Corpo do texto do [`GroupText`] — `sm:text-sm`, com a entrelinha do par (armadilha nº 1: sem
/// isso o GPUI usaria a razão de ouro e a peça sairia ~2,6px mais alta que a referência).
const TEXT_SIZE: f32 = 14.0;
const TEXT_LINE_HEIGHT: f32 = 20.0;

/// Lado do ícone do [`GroupText`] — `sm:size-4`.
const TEXT_ICON: f32 = 16.0;

/// O `[&_svg]:-mx-0.5` da referência, reproduzido como GEOMETRIA e não como margem negativa
/// (armadilha nº 7: `.mx(px(-2.0))` já colapsou o layout das abas). O ícone puxa 2px do respiro de
/// início e 2px do vão até o texto.
const TEXT_ICON_PULL: f32 = 2.0;

/// Vão entre ícone e texto no [`GroupText`] — `gap-2` menos o puxão do ícone.
const TEXT_GAP: f32 = 8.0;

// =================================================================================================
// Separador
// =================================================================================================

/// De que cor o separador se pinta.
///
/// São os dois casos que a referência mostra, e não uma cor livre: a escolha depende do que está dos
/// dois lados, então ela é semântica.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SeparatorTone {
    /// `--input`. O padrão — serve pra tudo que tem superfície própria: `Outline`, campos, selects,
    /// [`GroupText`].
    #[default]
    Input,
    /// `--primary` a 72%. Pra grupos de botões **preenchidos** (`Primary`), onde o `--input` sumiria
    /// contra o fundo escuro do botão. É o `<GroupSeparator className="bg-primary/72" />` dos
    /// exemplos `p-group-6` e `p-group-13`.
    OnPrimary,
}

impl SeparatorTone {
    fn color(self, realce: f32, ligado: bool) -> Rgba8 {
        let p = palette();
        match self {
            // Um vizinho LIGADO é estado estável, e o degrau dele é o mais claro dos três — então
            // ele não interpola nem cede ao hover: quem está ligado continua ligado com o ponteiro
            // em cima.
            SeparatorTone::Input if ligado => p.input_on,
            // O realce de hover interpola entre os dois alfas de `--input`. No claro os dois são
            // iguais, então a interpolação é um no-op — de propósito: a regra do coss é `dark:`.
            SeparatorTone::Input => {
                if realce <= 0.0 {
                    p.input
                } else if realce >= 1.0 {
                    p.input_hover
                } else {
                    Rgba8(mistura_alfa(p.input, p.input_hover, realce))
                }
            }
            SeparatorTone::OnPrimary => p.primary_72,
        }
    }
}

/// Interpola só o ALFA entre duas cores de mesmo RGB, devolvendo `0xRRGGBBAA`.
///
/// As duas pontas de `--input` são a mesma cor com alfas diferentes, então interpolar o alfa é
/// exatamente o cross-fade — e sai um `Rgba8`, que é o que a paleta fala. (`crate::color::lerp`
/// devolve `Hsla` e serve pro caso geral, em que o RGB também muda.)
fn mistura_alfa(a: Rgba8, b: Rgba8, t: f32) -> u32 {
    let (rgb, alfa_a, alfa_b) = (a.0 & 0xffffff00, a.0 & 0xff, b.0 & 0xff);
    let alfa = alfa_a as f32 + (alfa_b as f32 - alfa_a as f32) * t.clamp(0.0, 1.0);
    rgb | (alfa.round() as u32).min(0xff)
}

/// **De que cor o separador se pinta**, dado o tom e o estado dos vizinhos.
///
/// Separado do desenho porque é a única decisão do separador — e a única coisa nele que dá pra
/// afirmar num teste (não se lê a cor de volta de uma `Div`).
///
/// A precedência dos três estados, do mais forte pro mais fraco:
///
/// 1. **`foco`** — um vizinho focado e sob o mouse ao mesmo tempo mostra o anel, que é a informação
///    mais forte das duas (e é ela que fecha o contorno do foco na emenda);
/// 2. **`ligado`** — estado estável de um toggle vizinho (`data-pressed` em base-ui é
///    SELECIONADO, não "mouse apertado"). No coss é o degrau mais claro dos dois de `--input`, e a
///    ordem de precedência daqui é a mesma ordem de brilho de lá;
/// 3. **`realce`** — hover/pressão do ponteiro, que interpola.
///
/// `pub(crate)` porque o [`crate::toggle_group::ToggleGroup`] é construído AO LADO do [`Group`] (ver
/// o doc daquele módulo) e reusa esta regra em vez de manter uma segunda cópia dela.
pub(crate) fn cor_do_separador(
    tone: SeparatorTone,
    realce: f32,
    ligado: bool,
    foco: bool,
) -> Rgba8 {
    if foco {
        palette().ring
    } else {
        tone.color(realce, ligado)
    }
}

/// O separador: 1px atravessado no eixo do grupo.
///
/// `foco` acende a cor do anel — é o `has-[+[data-slot=input-control]:focus-within]:bg-ring` do
/// coss, e é o que fecha o contorno do foco na emenda, já que o anel do filho para ali.
///
/// `pub(crate)` pelo mesmo motivo de [`cor_do_separador`].
pub(crate) fn separador(
    orientation: Orientation,
    tone: SeparatorTone,
    realce: f32,
    ligado: bool,
    foco: bool,
) -> Div {
    let cor = cor_do_separador(tone, realce, ligado, foco);
    let d = div().flex_none().bg(cor.hsla());
    match orientation {
        // Sem altura declarada: o `align-items` default do flex é `stretch`, então ele acompanha o
        // filho mais alto — é o `self-stretch` do `Separator` do coss.
        Orientation::Horizontal => d.w(px(SEPARATOR)),
        Orientation::Vertical => d.h(px(SEPARATOR)),
    }
}

// =================================================================================================
// O que pode entrar num grupo
// =================================================================================================

/// **O que um filho de [`Group`] precisa saber fazer.**
///
/// A costura tem que chegar em quem desenha a superfície, e cada componente da lib se monta de um
/// jeito diferente — uns são [`RenderOnce`] (o `Join` entra num campo antes de renderizar), outros
/// são `Entity` com estado próprio (o `Join` é gravado na entidade, que se redesenha depois). Este
/// trait é onde essa diferença fica escondida; ver as implementações no fim deste módulo.
pub trait GroupChild {
    /// O elemento, já sabendo como encostar nos vizinhos.
    fn into_joined(self: Box<Self>, join: Join, window: &mut Window, cx: &mut App)
        -> gpui::AnyElement;

    /// O foco deste filho **quando ele é um campo** — o separador vizinho usa isso pra virar a cor
    /// do anel. `None` (o default) = não é campo, e aí o separador não reage.
    fn field_focus(&self, _cx: &App) -> Option<FocusHandle> {
        None
    }

    /// O id deste filho **quando ele é um botão** — o separador usa pra saber se o vizinho está sob
    /// o mouse ou pressionado. `None` (o default) = o separador não reage.
    fn button_id(&self) -> Option<ElementId> {
        None
    }

    /// Se este filho é ele mesmo um [`Group`]. Muda tudo: o pai passa a espaçar em vez de costurar
    /// (ver o doc do módulo).
    fn is_group(&self) -> bool {
        false
    }
}

// =================================================================================================
// Group
// =================================================================================================

/// Agrupa controles numa peça só. Ver o doc do módulo.
///
/// É um **elemento de render** ([`RenderOnce`]): construa a cada frame.
///
/// ```ignore
/// Group::new()
///     .child(Button::new("files", "Files").variant(ButtonVariant::Outline))
///     .child(Button::new("media", "Media").variant(ButtonVariant::Outline))
/// ```
#[derive(IntoElement, Default)]
pub struct Group {
    orientation: Orientation,
    children: Vec<Box<dyn GroupChild>>,
    separators: bool,
    tone: SeparatorTone,
    full_width: bool,
}

impl Group {
    pub fn new() -> Self {
        Self {
            orientation: Orientation::Horizontal,
            children: Vec::new(),
            // Automático por decisão nossa: no coss é manual e "obrigatório entre todos os
            // controles", o que é a definição de coisa que não devia ser manual.
            separators: true,
            tone: SeparatorTone::default(),
            full_width: false,
        }
    }

    /// Empilha na vertical em vez de lado a lado.
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Acrescenta um filho. A costura dele é calculada pela POSIÇÃO — não há nada pra ajustar aqui.
    pub fn child(mut self, child: impl GroupChild + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// **Sem separadores**: os controles encostam sem linha entre eles.
    ///
    /// Raro e deliberado. A costura continua (raios e bordas da emenda somem), então o conjunto vira
    /// uma superfície contínua sem divisão — o que só faz sentido quando os filhos já se distinguem
    /// por outra coisa, como fundos diferentes.
    pub fn seamless(mut self) -> Self {
        self.separators = false;
        self
    }

    /// O grupo **ocupa a largura disponível** em vez de medir o conteúdo, e os filhos elásticos
    /// dividem o espaço entre si.
    ///
    /// O default é o `w-fit` da referência: o grupo mede o conteúdo. Isto é pro caso oposto, em que a
    /// largura vem de fora e os filhos se acomodam nela — três campos numéricos dividindo uma linha
    /// em três, por exemplo. Só faz sentido com filhos que sabem crescer: um [`crate::Button`] é
    /// `flex_none`, então ele não estica e sobra vão no fim.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// De que cor os separadores se pintam. Ver [`SeparatorTone`].
    pub fn separator_tone(mut self, tone: SeparatorTone) -> Self {
        self.tone = tone;
        self
    }
}

impl RenderOnce for Group {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let orientation = self.orientation;
        // Um grupo que contém grupos ESPAÇA em vez de costurar (ver o doc do módulo).
        let aninhado = self.children.iter().any(|c| c.is_group());

        // O que o separador precisa saber dos vizinhos, lido ANTES de consumir os filhos (o
        // `into_joined` toma posse deles).
        let vizinhos: Vec<(bool, f32)> = self
            .children
            .iter()
            .map(|c| {
                let focado = c
                    .field_focus(cx)
                    .is_some_and(|h| h.is_focused(window));
                let realce = c.button_id().map_or(0.0, crate::button::interaction_amount);
                (focado, realce)
            })
            .collect();

        let total = self.children.len();
        let mut raiz = div().flex();
        raiz = if self.full_width {
            raiz.w_full()
        } else {
            raiz.flex_none()
        };
        raiz = match orientation {
            Orientation::Horizontal => raiz,
            Orientation::Vertical => raiz.flex_col(),
        };
        if aninhado {
            raiz = raiz.gap(px(NESTED_GAP));
        }

        for (i, filho) in self.children.into_iter().enumerate() {
            if self.separators && !aninhado && i > 0 {
                // O separador reage aos DOIS lados: basta um vizinho focado pra ele virar anel, e
                // ele acompanha o realce mais forte entre os dois.
                let (foco_a, realce_a) = vizinhos[i - 1];
                let (foco_b, realce_b) = vizinhos[i];
                raiz = raiz.child(separador(
                    orientation,
                    self.tone,
                    realce_a.max(realce_b),
                    // O degrau de "vizinho LIGADO" é sempre `false` aqui: o `Group` genérico não
                    // sabe o que é estar ligado — quem sabe é o [`crate::toggle_group::ToggleGroup`],
                    // que é o único componente do coss com a regra `[data-pressed]` no separador.
                    false,
                    foco_a || foco_b,
                ));
            }
            let join = if aninhado {
                Join::NONE
            } else {
                Join {
                    // Todo filho do meio é costurado dos dois lados. O separador entre eles conta
                    // como vizinho no coss (ele também é um `[data-slot]`), e o resultado é o
                    // mesmo: a emenda perde borda e raio.
                    start: i > 0,
                    end: i + 1 < total,
                    orientation,
                }
            };
            raiz = raiz.child(filho.into_joined(join, window, cx));
        }

        // **O `w-fit` da referência, que o GPUI não tem.** Sem largura declarada, um `div` de flex
        // ESTICA no eixo transversal do pai — e num grupo vertical isso é visível: o separador
        // (que não declara largura, pra acompanhar o filho mais largo) atravessava a linha inteira.
        //
        // Uma linha em volta resolve pelo eixo: dentro dela o grupo é um item de flex no eixo
        // horizontal, e com `flex_none` o tamanho dele nesse eixo passa a ser o do CONTEÚDO. No
        // grupo horizontal isto é um no-op; no vertical é o que faz o separador medir o botão.
        let mut fora = div().flex();
        if self.full_width {
            fora = fora.w_full();
        }
        fora.child(raiz)
    }
}

/// Um grupo dentro de outro: o pai só espaça, e o filho costura os netos.
impl GroupChild for Group {
    fn into_joined(
        self: Box<Self>,
        _join: Join,
        _window: &mut Window,
        _cx: &mut App,
    ) -> gpui::AnyElement {
        // O `Join` é ignorado de propósito: no coss ele TAMBÉM cai no grupo aninhado
        // (`*:data-slot:…` casa com `data-slot=group`), mas como o grupo não tem borda nem raio
        // próprios, achatá-lo é um no-op. Quem tem geometria são os netos, e deles cuida o
        // `render` deste grupo.
        (*self).into_any_element()
    }

    fn is_group(&self) -> bool {
        true
    }
}

// =================================================================================================
// GroupText
// =================================================================================================

/// Um bloco de **texto** dentro de um grupo: `https://`, `.com`, `R$`, um rótulo.
///
/// Tem superfície própria (fundo `--muted`, borda `--input`, bisel), ao contrário do texto que vive
/// DENTRO de um campo — ver a tabela no doc do módulo.
///
/// **Não declara altura**: acompanha o filho mais alto do grupo, pelo `stretch` do flex. É o que a
/// referência faz (o `GroupText` do coss não tem nenhuma classe de altura) e é o que faz ele casar
/// com qualquer tamanho de botão ou campo sem precisar de variantes de tamanho.
#[derive(IntoElement)]
pub struct GroupText {
    text: SharedString,
    icon: Option<SharedString>,
    join: Join,
}

impl GroupText {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            join: Join::NONE,
        }
    }

    /// Um ícone antes do texto.
    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }
}

impl RenderOnce for GroupText {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = palette();
        let join = self.join;
        // O ícone puxa 2px pra si (ver `TEXT_ICON_PULL`): sai do respiro de início e do vão.
        let (pad_start, gap) = if self.icon.is_some() {
            (TEXT_PAD_X - TEXT_ICON_PULL, TEXT_GAP - TEXT_ICON_PULL)
        } else {
            (TEXT_PAD_X, TEXT_GAP)
        };

        let mut superficie = div()
            .relative()
            .flex()
            .items_center()
            .flex_none()
            .gap(px(gap))
            .pl(px(pad_start))
            .pr(px(TEXT_PAD_X))
            .bg(p.text_bg.hsla())
            .text_size(px(TEXT_SIZE))
            .line_height(px(TEXT_LINE_HEIGHT))
            .text_color(p.text_fg.hsla())
            .border_color(p.input.hsla())
            .shadow(vec![gpui::BoxShadow {
                color: p.shadow.hsla(),
                offset: gpui::point(px(0.0), px(1.0)),
                blur_radius: px(2.0),
                spread_radius: px(0.0),
            }]);
        superficie = join.borders(superficie);
        superficie = join.rounded(superficie, RADIUS);

        if let Some(icon) = self.icon {
            superficie = superficie.child(
                gpui::svg()
                    .path(icon)
                    .size(px(TEXT_ICON))
                    .flex_none()
                    .text_color(p.text_fg.hsla()),
            );
        }
        superficie = superficie.child(self.text);

        // O bisel é FILHO absoluto: o `GroupText` não tem `overflow_hidden`, então não precisa do
        // arranjo de irmão que o campo de texto exige (armadilha nº 6).
        let bisel = join
            .bevel_overlay(-1.0, RADIUS)
            .border_color(p.bevel.hsla());
        superficie.child(if p.bevel_dir > 0.0 {
            bisel.border_b_1()
        } else {
            bisel.border_t_1()
        })
    }
}

impl GroupChild for GroupText {
    fn into_joined(
        mut self: Box<Self>,
        join: Join,
        _window: &mut Window,
        _cx: &mut App,
    ) -> gpui::AnyElement {
        self.join = join;
        (*self).into_any_element()
    }
}

// =================================================================================================
// Quem pode ser costurado
// =================================================================================================
//
// São três formas de receber a costura, e a diferença não é estética — vem de como cada componente
// existe no GPUI:
//
// - **`RenderOnce`** (`Button`, `Input`, `GroupText`): são construídos a cada frame, então o grupo
//   grava o `Join` num campo antes de pedir o elemento.
// - **`Entity`** (`Select`): se renderiza sozinha, quando o GPUI quiser. O grupo não tem como
//   embrulhar esse render — então grava o `Join` NA ENTIDADE. Vale porque o `render` da entidade só
//   roda depois, na montagem da árvore, e já lê o valor novo; e sem `notify`, porque não é uma
//   mudança de estado do usuário, é a mesma informação que o grupo recalcula todo frame.
// - **`Group`**: ver a implementação acima — o pai espaça em vez de costurar.

impl GroupChild for crate::Button {
    fn into_joined(
        mut self: Box<Self>,
        join: Join,
        _window: &mut Window,
        _cx: &mut App,
    ) -> gpui::AnyElement {
        self.join = join;
        (*self).into_any_element()
    }

    fn button_id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
}

impl GroupChild for crate::Input {
    fn into_joined(
        mut self: Box<Self>,
        join: Join,
        _window: &mut Window,
        _cx: &mut App,
    ) -> gpui::AnyElement {
        self.join = join;
        (*self).into_any_element()
    }

    fn field_focus(&self, cx: &App) -> Option<FocusHandle> {
        Some(gpui::Focusable::focus_handle(self.state.read(cx), cx))
    }
}

impl GroupChild for gpui::Entity<crate::Select> {
    fn into_joined(
        self: Box<Self>,
        join: Join,
        _window: &mut Window,
        cx: &mut App,
    ) -> gpui::AnyElement {
        self.update(cx, |select, _cx| select.join = join);
        (*self).into_any_element()
    }

    fn field_focus(&self, cx: &App) -> Option<FocusHandle> {
        Some(gpui::Focusable::focus_handle(self.read(cx), cx))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::assertions_on_constants)]
    use super::*;

    /// A costura é **posicional**: primeiro filho aberto no início, último aberto no fim, e todo o
    /// meio fechado dos dois lados. Um grupo de um filho só não costura nada.
    #[test]
    fn a_costura_sai_da_posicao() {
        let junta = |i: usize, total: usize| Join {
            start: i > 0,
            end: i + 1 < total,
            orientation: Orientation::Horizontal,
        };

        // Um filho só: solto.
        assert_eq!(junta(0, 1), Join::NONE);
        assert!(!junta(0, 1).is_joined());

        // Dois: cada um costurado de um lado.
        assert!(!junta(0, 2).start && junta(0, 2).end);
        assert!(junta(1, 2).start && !junta(1, 2).end);

        // Três: o do meio costurado dos dois.
        assert!(junta(1, 3).start && junta(1, 3).end);
    }

    /// **O lado da emenda vai a zero, e só ele.** Este teste existe porque `start`/`end` são
    /// lógicos: no vertical eles são topo/base, e trocar os eixos é o erro fácil de cometer aqui.
    #[test]
    fn o_raio_zera_no_lado_certo_nas_duas_orientacoes() {
        // Não dá pra ler o raio de volta de uma `Div`, então o teste afirma a REGRA que o
        // `rounded` implementa, com os cantos nomeados como o código os nomeia.
        let cantos = |j: Join| -> [bool; 4] {
            // (tl, tr, br, bl) achatados?
            match j.orientation {
                Orientation::Horizontal => [j.start, j.end, j.end, j.start],
                Orientation::Vertical => [j.start, j.start, j.end, j.end],
            }
        };

        let h = Join {
            start: true,
            end: false,
            orientation: Orientation::Horizontal,
        };
        assert_eq!(
            cantos(h),
            [true, false, false, true],
            "no horizontal, `start` é o lado ESQUERDO (tl+bl)"
        );

        let v = Join {
            start: true,
            end: false,
            orientation: Orientation::Vertical,
        };
        assert_eq!(
            cantos(v),
            [true, true, false, false],
            "no vertical, `start` é o TOPO (tl+tr)"
        );
    }

    /// O bisel atravessa a emenda; o anel para nela. Se os dois usassem o mesmo recuo, ou o bisel
    /// deixaria uma fresta de meio pixel, ou o anel entraria por baixo do vizinho pra nada.
    #[test]
    fn bisel_e_anel_tratam_a_emenda_de_formas_opostas() {
        assert!(SEAM_BLEED > 0.0, "o bisel avança pra dentro do vizinho");
        assert_eq!(SEAM_BLEED, 0.5, "é o `-end-[0.5px]` da referência");
    }

    /// O alfa do separador interpola; o RGB não se mexe. Se `mistura_alfa` tocasse no RGB, o
    /// separador mudaria de COR no hover em vez de só clarear.
    #[test]
    fn mistura_alfa_so_mexe_no_alfa() {
        let (a, b) = (Rgba8(0xffffff1a), Rgba8(0xffffff20));
        assert_eq!(mistura_alfa(a, b, 0.0), a.0, "t=0 é a ponta A");
        assert_eq!(mistura_alfa(a, b, 1.0), b.0, "t=1 é a ponta B");
        assert_eq!(mistura_alfa(a, b, 0.5) & 0xffffff00, 0xffffff00, "RGB intacto");
        let meio = mistura_alfa(a, b, 0.5) & 0xff;
        assert!(meio > 0x1a && meio < 0x20, "o alfa do meio fica entre os dois: {meio:#x}");
    }

    /// No tema CLARO o separador não reage ao hover — a regra do coss é `dark:`. Este teste trava
    /// isso, porque "os dois alfas são iguais" parece um erro de copiar e colar na paleta.
    #[test]
    fn o_realce_do_separador_e_so_do_tema_escuro() {
        assert_eq!(
            GROUP_LIGHT.input.0, GROUP_LIGHT.input_hover.0,
            "claro: mesma cor nas duas pontas, de propósito"
        );
        assert!(
            GROUP_DARK.input_hover.0 & 0xff > GROUP_DARK.input.0 & 0xff,
            "escuro: o hover CLAREIA o separador"
        );
    }

    /// O bisel escuro é o dobro da referência — o mesmo desvio do resto da lib. Se alguém
    /// "corrigir" a paleta pra 6%, este teste cai junto com os irmãos dele em `input`/`button`.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        assert_eq!(GROUP_LIGHT.bevel.0 & 0xff, 0x0f, "claro: preto a 6%");
        assert_eq!(
            GROUP_DARK.bevel.0 & 0xff,
            0x1e,
            "escuro: branco a ~12%, o dobro dos 6% do coss"
        );
        assert_eq!(GROUP_LIGHT.bevel_dir, 1.0, "claro: filete EMBAIXO");
        assert_eq!(GROUP_DARK.bevel_dir, -1.0, "escuro: filete EM CIMA");
    }

    /// A entrelinha do `GroupText` é a do par do Tailwind, não a razão de ouro do GPUI — a
    /// armadilha nº 1, que já custou 6px de altura no `Frame` e nos itens do `Menu`.
    #[test]
    fn a_entrelinha_do_texto_e_a_do_par_tailwind() {
        assert_eq!((TEXT_SIZE, TEXT_LINE_HEIGHT), (14.0, 20.0), "text-sm");
        assert!(
            TEXT_LINE_HEIGHT < TEXT_SIZE * 1.618_034,
            "se fosse a razão de ouro, a peça sairia mais alta que a referência"
        );
    }

    /// O ícone puxa do respiro E do vão, em partes iguais — é a geometria que substitui o
    /// `-mx-0.5`. Somados, respiro+vão do caso com ícone têm que dar 4px menos que sem ícone.
    #[test]
    fn o_icone_puxa_dos_dois_lados_em_vez_de_usar_margem_negativa() {
        let sem = TEXT_PAD_X + TEXT_GAP;
        let com = (TEXT_PAD_X - TEXT_ICON_PULL) + (TEXT_GAP - TEXT_ICON_PULL);
        assert_eq!(sem - com, 2.0 * TEXT_ICON_PULL);
        assert_eq!(TEXT_ICON_PULL, 2.0, "`-mx-0.5` = 2px de cada lado");
    }

    /// **O foco vence o hover.** Um vizinho focado E sob o mouse tem que mostrar o anel: é a
    /// informação mais forte das duas, e é ela que fecha o contorno do foco na emenda.
    #[test]
    fn no_separador_o_foco_vence_o_realce() {
        theme::set_theme(theme::ThemeMode::Dark);
        let p = palette();

        assert_eq!(
            cor_do_separador(SeparatorTone::Input, 0.0, false, false).0,
            p.input.0,
            "repouso: --input"
        );
        assert_eq!(
            cor_do_separador(SeparatorTone::Input, 1.0, false, false).0,
            p.input_hover.0,
            "vizinho em hover: clareia"
        );
        assert_eq!(
            cor_do_separador(SeparatorTone::Input, 1.0, false, true).0,
            p.ring.0,
            "focado E em hover: manda o foco"
        );
        assert_eq!(
            cor_do_separador(SeparatorTone::OnPrimary, 0.0, false, false).0,
            p.primary_72.0,
            "em botões preenchidos o tom é outro"
        );
        assert_eq!(
            cor_do_separador(SeparatorTone::OnPrimary, 0.0, false, true).0,
            p.ring.0,
            "…mas o foco continua vencendo"
        );
    }

    /// **O vizinho LIGADO é o degrau mais claro, e ele não cede ao hover.** É o que separa a regra
    /// `[data-pressed]` do coss da regra `:hover`: as duas mexem no mesmo `::before`, mas a de
    /// pressionado leva `--input` cheio e a de hover só `/64`.
    #[test]
    fn o_degrau_de_ligado_vence_o_hover_e_e_o_mais_claro() {
        theme::set_theme(theme::ThemeMode::Dark);
        let p = palette();
        let alfa = |c: Rgba8| c.0 & 0xff;

        assert_eq!(
            cor_do_separador(SeparatorTone::Input, 0.0, true, false).0,
            p.input_on.0,
            "vizinho ligado: --input cheio por cima"
        );
        assert_eq!(
            cor_do_separador(SeparatorTone::Input, 1.0, true, false).0,
            p.input_on.0,
            "ligado E em hover continua ligado — o hover não apaga o estado"
        );
        assert!(
            alfa(p.input) < alfa(p.input_hover) && alfa(p.input_hover) < alfa(p.input_on),
            "os três degraus em ordem de brilho: repouso < hover < ligado"
        );
        assert_eq!(
            cor_do_separador(SeparatorTone::Input, 0.0, true, true).0,
            p.ring.0,
            "…mas o foco continua vencendo os três"
        );

        // No claro os três degraus são a MESMA cor: as regras do coss são todas `dark:`.
        theme::set_theme(theme::ThemeMode::Light);
        let claro = palette();
        assert_eq!(claro.input.0, claro.input_on.0, "claro: ligado não clareia");
    }
}

/// **A plumbagem que faz o separador reagir** — o que o `Group` consegue descobrir sobre cada filho
/// antes de consumi-lo.
///
/// Os dois testes aqui parecem óbvios e não são: são eles que garantem que o separador tem de onde
/// tirar o estado do vizinho. Se `field_focus` devolvesse `None` pra um campo, ou o `button_id` não
/// casasse com o id que o botão registra na tabela de hover, o separador simplesmente nunca mudaria
/// de cor — e um defeito desses não quebra teste nenhum, só some da tela.
#[cfg(test)]
mod tests_de_plumbagem {
    use super::*;
    use gpui::{AppContext, TestAppContext};

    /// Um campo entrega o foco DELE — o mesmo handle que o `InputState` usa, não um novo.
    #[gpui::test]
    fn o_campo_entrega_o_proprio_foco(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let window = cx.add_window(|window, cx| {
            let estado = cx.new(|cx| crate::input::single_line(window, cx));
            let campo = crate::Input::new(&estado);

            let pelo_trait = campo
                .field_focus(cx)
                .expect("um campo é um campo: tem que devolver Some");
            let do_estado = gpui::Focusable::focus_handle(estado.read(cx), cx);
            assert_eq!(
                pelo_trait, do_estado,
                "tem que ser o MESMO handle: um handle novo nunca estaria focado, e o separador \
                 nunca acenderia"
            );
            gpui::Empty
        });
        let _ = window;
    }

    /// Um botão entrega o id com que ele se registra na tabela de hover, e **não** se declara campo
    /// (senão o separador tentaria ler foco de quem não tem).
    #[test]
    fn o_botao_entrega_o_id_e_nao_se_declara_campo() {
        let botao = crate::Button::new("g-teste", "Files");
        assert_eq!(
            botao.button_id(),
            Some(gpui::ElementId::from("g-teste")),
            "o id tem que ser o mesmo que o botão usa no `div().id(...)` — é a chave da tabela de \
             hover que o grupo consulta"
        );
        assert!(!botao.is_group());
    }

    /// Um grupo aninhado se declara grupo — é o que faz o pai espaçar em vez de costurar.
    #[test]
    fn um_grupo_se_declara_grupo() {
        assert!(Group::new().is_group());
        assert!(!GroupText::new("https://").is_group());
        assert_eq!(GroupText::new("https://").button_id(), None);
    }
}

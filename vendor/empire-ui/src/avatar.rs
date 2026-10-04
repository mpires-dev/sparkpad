//! `Avatar` — o **retrato redondo** de uma pessoa. Porte do `avatar.tsx` do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/avatar.tsx`
//!
//! ```ignore
//! // O caso normal: foto, com as iniciais por baixo enquanto ela carrega (ou se falhar).
//! Avatar::image("https://exemplo.com/matheus.png").with_initials("MP")
//!
//! // Sem foto nenhuma: só as iniciais.
//! Avatar::initials("MP")
//!
//! // Sem nome pra abreviar: um glifo genérico.
//! Avatar::icon("iconoir/regular/user.svg").size(AvatarSize::Lg)
//! ```
//!
//! # As três peças do original, numa só
//!
//! O coss expõe `Avatar` (raiz) + `AvatarImage` + `AvatarFallback`, e o call site compõe as três. Mas
//! a composição é SEMPRE a mesma — a raiz envolve a imagem, e o fallback é o que aparece quando a
//! imagem não está lá. Aqui isso é um componente só, com duas fontes de conteúdo: a imagem entra pelo
//! construtor [`Avatar::image`], e o fallback por
//! [`Avatar::with_initials`]/[`Avatar::with_icon`]. É a mesma decisão do [`crate::card`], do
//! [`crate::frame`] e do [`crate::empty`]: quem é dono das fatias garante a regra, em vez de confiar
//! na ordem em que o call site declarou os filhos.
//!
//! | daqui | no original |
//! |---|---|
//! | [`Avatar`] | `Avatar` + `AvatarImage` + `AvatarFallback` |
//! | [`Avatar::with_initials`] / [`Avatar::with_icon`] | os `children` do `AvatarFallback` |
//!
//! # A troca imagem↔fallback, que não está no arquivo
//!
//! O comportamento vem do `Avatar` do **base-ui**, e não do CSS: a imagem tem quatro estados —
//! `idle`, `loading`, `loaded`, `error` — e **só aparece em `loaded`**. Nos outros três quem aparece é
//! o fallback, que ainda tem um `delay` (em ms, **default 0**) pra não piscar as iniciais num carregamento
//! instantâneo. O coss não passa `delay`, então na referência o fallback aparece **na hora**.
//!
//! No GPUI o que existe é o [`gpui::img`], e ele resolve os quatro estados em três (leia o
//! `gpui-0.2.2/src/elements/img.rs`): `use_data` devolve `None` (idle **e** loading, indistinguíveis),
//! `Some(Err(..))` (error) ou `Some(Ok(..))` (loaded). Colapsar idle com loading é inofensivo aqui —
//! os dois mostram o fallback.
//!
//! O `img` tem dois ganchos pra isso, e **nenhum dos dois serve**:
//!
//! - `with_fallback(..)` cobre só o ramo de ERRO.
//! - `with_loading(..)` cobre o ramo de carregamento, mas (1) só depois de `gpui::LOADING_DELAY`, que
//!   é **200ms fixos** e que ninguém de fora pode mudar, e (2) só num `img` com `.id(..)`, porque o
//!   contador `started_loading` vive no estado de elemento e o ramo inteiro é guardado por
//!   `if let Some(state)` — num `img` sem id, `with_loading` é um **no-op silencioso**.
//!
//! Ou seja: pelo caminho dos ganchos, o mais perto que se chega do `delay: 0` da referência é 200ms de
//! buraco. Então o fallback aqui **não** é um ramo alternativo: ele é um **irmão em fluxo, POR BAIXO**
//! da imagem, que é absoluta e cobre a caixa inteira. A imagem carregada pinta em cima e o esconde; a
//! que falhou não pinta nada e o deixa à vista. O resultado é o `delay: 0` da referência, de graça, sem
//! estado de elemento e sem id. O preço está declarado abaixo.
//!
//! # ⚠️ O círculo, o `overflow-hidden` e o ContentMask RETANGULAR
//!
//! O `overflow-hidden` da raiz é o que faz a foto virar círculo no original. No GPUI ele é um
//! [`gpui::ContentMask`], que é um [`gpui::Bounds`] — **retangular**. Ele recorta pela CAIXA, nunca
//! pelo raio. Um avatar circular com foto por dentro é exatamente o caso onde isso morde.
//!
//! O que resolve é outra coisa, e ela existe: o `Window::paint_image` recebe `corner_radii` — tirados
//! do estilo **da própria imagem**, aparados em `min(w,h)/2`. Então o recorte redondo não vem do pai:
//! **cada filho carrega o seu**. A imagem leva `rounded_full`, o fallback também, e a raiz leva
//! `overflow_hidden` só pela metade retangular do serviço (sem ele, uma foto em `Cover` mais larga que
//! a caixa vazaria pra fora dos 32px em vez de ser aparada).
//!
//! Onde isso **não** fecha: o `corner_radii` é aplicado ao retângulo do `object-fit`, não à caixa do
//! elemento. Com fonte quadrada — o caso normal de uma foto de perfil — `Cover` devolve exatamente a
//! caixa, e o círculo é perfeito. Com fonte 2:1 ele devolve 64×32 começando 16px à esquerda, o aparo
//! dá raio 16 nessa pista de atletismo, e a janela visível de 32×32 fica com os **cantos quadrados**.
//! É isso que o teste `cover_perde_o_circulo_quando_a_fonte_nao_e_quadrada` tranca em número, e é a
//! razão de existir o [`AvatarFit::Fill`].
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`inline-flex` e `align-middle`**: os dois descrevem um elemento de nível **inline**, alinhado
//!   pela linha de base do texto em volta. O GPUI não tem fluxo inline — todo elemento é de bloco. Na
//!   prática: no original um avatar pode cair no meio de um parágrafo; aqui ele é irmão flex do texto,
//!   e o alinhamento vertical é do container (`items-center`).
//! - **`select-none`**: não há seleção de texto de conteúdo de controle nesta base.
//! - **`data-slot`**: atributo de DOM, usado no original pra estilizar de fora. Sem DOM, sem atributo.
//! - **`onLoadingStatusChange`** do base-ui: o [`gpui::img`] lê o estado de carga DENTRO do
//!   `Element::request_layout` e não o expõe a quem o construiu. Não há onde pendurar o callback sem
//!   duplicar a máquina de asset do GPUI.
//!
//! **Resolvido em número**
//!
//! - **`overflow-hidden`** → metade retangular no pai, metade redonda em cada filho. Ver a seção do
//!   ContentMask acima, e o número da fonte 2:1 que sobra de fora.
//! - **`rounded-full`** → `rounded_full()`, que é `px(9999)` aparado pelo GPUI em `min(w,h)/2` na hora
//!   de pintar. Um raio só serve os três tamanhos, sem constante por tamanho: em 32px ele resolve pra
//!   16, que é o círculo exato.
//! - **`text-xs`** → o PAR do Tailwind, `(12, 16)`. Fixar a entrelinha é obrigatório: o default do
//!   GPUI é `relative(1.618_034)`, que sobre 12px dá **19,4**. Ver [`AvatarSize::text`].
//! - **`bg-background`** → `#ffffff` no claro, `mix(neutral-950 96%, white)` = `#141414` no escuro.
//!   **`bg-muted`** → preto 4% no claro, branco 4% no escuro. Ver [`AVATAR_LIGHT`]/[`AVATAR_DARK`].
//! - Não há **nenhum** modificador `/N` neste componente — o `--muted` já nasce com alfa no
//!   `globals.css` (`--alpha(black / 4%)`), e nada o multiplica depois. Por isso o
//!   [`crate::color::Rgba8::scaled`] não aparece aqui: usá-lo seria inventar um alfa que a referência
//!   não pede.
//!
//! **Desvio consciente**
//!
//! - **O fallback fica POR BAIXO da imagem, não no lugar dela.** É o que compra o `delay: 0` da
//!   referência (ver a seção da troca acima). O preço: com uma imagem **parcialmente transparente**
//!   (um PNG com alfa), as iniciais aparecem ATRÁS dela, onde o original mostraria o
//!   `--background` da raiz. Para foto de perfil — JPEG/PNG opaco — não há diferença observável.
//! - **A imagem é `absolute` + `size-full`**, e não `size-full` em fluxo como no original. A caixa é a
//!   mesma (32×32, a raiz não tem respiro); a mudança existe só pra empilhar a imagem sobre o fallback.
//!
//! **Superset consciente**
//!
//! - **[`AvatarSize`]**: o coss só traz `size-8`. Os outros dois passos são nossos —
//!   `Sm` = `size-6` (24) e `Lg` = `size-10` (40), ambos na escala de espaçamento do Tailwind. O passo
//!   de TIPO só muda no `Lg` (pra `text-sm`, 14/20); `Sm` e `Default` ficam em `text-xs`, porque o
//!   Tailwind não tem passo abaixo dele. Só o [`AvatarSize::Default`] veio da referência.
//! - **[`AvatarFit::Fill`]**: a saída de emergência pra fonte não quadrada — garante o círculo ao custo
//!   de esticar a imagem. O default segue fiel ao `object-cover` do original.
//! - **[`Avatar::icon`]**: o `AvatarFallback` do original aceita `children` quaisquer, e o glifo
//!   genérico é o segundo uso mais comum depois das iniciais. Aqui ele é um caso nomeado, com o ícone
//!   a metade da caixa (ver [`AvatarSize::icon_size`]).
//! - **A raiz declara `--foreground`**: o original o HERDA do ancestral, e o GPUI não tem `inherit`.
//!   Mesma decisão, e mesma razão, do [`crate::empty`].
//!
//! **Sem cobertura de teste — declarado**
//!
//! - A **ORDEM DE PINTURA** dos dois filhos (fallback primeiro, foto depois) não tem teste. Trocar a
//!   ordem faria o fallback pintar POR CIMA de uma foto que carregou, e a suíte inteira continua
//!   verde: o GPUI não expõe a árvore montada nem a cena pintada pra um teste, e o
//!   `VisualTestContext` só mede bounds — que são iguais nas duas ordens. O que existe pra guardar
//!   isso é: (a) o teste `a_foto_fica_fora_do_fluxo_e_preenche_a_caixa`, que pega o irmão do problema
//!   (a foto sair do `absolute`), e (b) a captura de janela, que é como esta lacuna foi conferida na
//!   integração. Mesma classe de limite do recorte redondo, logo abaixo.
//! - O **recorte redondo** de cada filho é propriedade de PINTURA (o raio vai pro `paint_image` e pro
//!   shader de quad), invisível a teste de unidade. Tirar o `rounded_full` do fallback não quebra
//!   nada — só aparece na tela.
//!
//! **Ausente
//!
//! - O **`delay` do `AvatarFallback`** como knob. O nosso é fixo no default da referência (0ms), que é
//!   o que o coss usa; quem quiser 300ms não tem por onde pedir.
//! - **`data-starting-style` / `data-ending-style`** do base-ui: os ganchos de animação de entrada e
//!   saída da imagem. O coss não os usa — não há classe de transição no arquivo — então não há
//!   aparência a perder, só a extensibilidade.
//! - **Elemento arbitrário como fallback**: aqui são os dois casos concretos (iniciais e ícone SVG).
//! - **O re-export do `AvatarPrimitive`**: escape hatch de React.

use gpui::{
    div, px, App, Div, ImageSource, IntoElement, ObjectFit, ParentElement, RenderOnce, SharedString,
    Styled, StyledImage as _, Window,
};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Convenção da casa: TODO valor é `0xRRGGBBAA`, com o byte de alfa, SEMPRE (ver [`crate::color`]).
// Misturar com os tokens de 6 dígitos de [`crate::theme`] desloca os canais e produz outra cor sem
// erro de compilação — já custou três bugs visíveis nesta base. A única ponte é
// [`crate::color::opaque`].

/// Tokens visuais do avatar, por tema. São **três**, e o componente inteiro sai deles.
#[derive(Clone, Copy, Debug)]
struct AvatarPalette {
    /// `--background` — o fundo da RAIZ (`bg-background`).
    ///
    /// Ele aparece em dois momentos: no vão entre a foto e o círculo, quando o `object-fit` não
    /// preenche a caixa (`Contain` não é oferecido aqui, mas `Cover` de uma fonte degenerada pode
    /// deixar sobra), e como o que está atrás de um fallback translúcido — ver [`Self::muted`].
    background: Rgba8,
    /// `--muted` — o fundo do FALLBACK (`bg-muted`).
    ///
    /// ⚠️ É **translúcido** (4%): ele não substitui o fundo da raiz, ele se compõe sobre ele. É essa
    /// composição que faz o disco das iniciais ler como um cinza levíssimo em vez de um bloco chapado
    /// — e é por isso que o teste `fallback_se_compoe_sobre_o_fundo_da_raiz` guarda o alfa dos dois.
    muted: Rgba8,
    /// `--foreground` — a cor das iniciais e do ícone.
    ///
    /// **Superset**: o original HERDA esta cor do ancestral, e o GPUI não tem `inherit` (ver o doc do
    /// módulo). O valor é o que o original herdaria em qualquer superfície do coss.
    fg: Rgba8,
}

/// Tema **claro**.
const AVATAR_LIGHT: AvatarPalette = AvatarPalette {
    background: Rgba8(0xffffffff), // --background = --color-white
    muted: Rgba8(0x0000000a),      // --alpha(black / 4%)
    fg: Rgba8(0x262626ff),         // neutral-800
};

/// Tema **escuro**.
const AVATAR_DARK: AvatarPalette = AvatarPalette {
    // `--background` escuro = mix(neutral-950 96%, white) = 10·0,96 + 255·0,04 ≈ 20 = 0x14.
    background: Rgba8(0x141414ff),
    muted: Rgba8(0xffffff0a), // --alpha(white / 4%)
    fg: Rgba8(0xf5f5f5ff),    // neutral-100
};

/// A paleta do avatar no tema corrente.
fn palette() -> &'static AvatarPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &AVATAR_DARK,
        theme::ThemeMode::Light => &AVATAR_LIGHT,
    }
}

// --- Geometria -----------------------------------------------------------------------------------

/// Corpo do `text-xs` — o passo de tipo da referência.
const TEXT_XS: f32 = 12.0;

/// Entrelinha do `text-xs` — o PAR do Tailwind: 12 de fonte, **16** de linha.
///
/// ⚠️ Sem fixar, o GPUI usa `relative(1.618_034)` e a linha sai com **19,4px**. Num avatar isso não
/// muda a ALTURA (ela é fixa em [`AvatarSize::box_size`]), mas engorda a caixa de linha dentro do
/// círculo e desloca as iniciais do centro ótico. É a armadilha nº 1 da casa — a mesma que o
/// [`crate::toggle`] e o [`crate::empty`] declaram.
const LINE_XS: f32 = 16.0;

/// Corpo do `text-sm` — só o [`AvatarSize::Lg`] o usa, e ele é **superset** (ver o doc do módulo).
const TEXT_SM: f32 = 14.0;

/// Entrelinha do `text-sm` — o PAR do Tailwind: 14 de fonte, **20** de linha.
const LINE_SM: f32 = 20.0;

/// Quanto da caixa o ícone do fallback ocupa.
///
/// Meia caixa. É uma regra só, e ela cai **em cima** dos passos do Tailwind nos três tamanhos:
/// 24→12 (`size-3`), 32→16 (`size-4`), 40→20 (`size-5`). Uma tabela de três valores poderia divergir
/// da escala; uma razão não.
const ICON_RATIO: f32 = 0.5;

// =================================================================================================
// Tamanho
// =================================================================================================

/// Tamanho do avatar.
///
/// ⚠️ **Superset.** O coss só traz `size-8` — que é o [`AvatarSize::Default`]. Ver o doc do módulo
/// pra de onde saem os outros dois.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AvatarSize {
    /// 24px — `size-6`. O avatar de linha de lista.
    Sm,
    /// **32px — `size-8`, o tamanho da referência.**
    #[default]
    Default,
    /// 40px — `size-10`. O avatar de cabeçalho de perfil.
    Lg,
}

impl AvatarSize {
    /// Todos os tamanhos, na ordem de tamanho. É a lista ÚNICA — quem varre tamanhos (teste,
    /// storybook) usa esta, pra um tamanho novo não passar batido.
    pub const ALL: [AvatarSize; 3] = [AvatarSize::Sm, AvatarSize::Default, AvatarSize::Lg];

    /// O lado da caixa — `size-6` / `size-8` / `size-10`.
    ///
    /// É o lado E o diâmetro: o avatar é um quadrado com `rounded-full`.
    pub fn box_size(self) -> f32 {
        match self {
            AvatarSize::Sm => 24.0,
            AvatarSize::Default => 32.0,
            AvatarSize::Lg => 40.0,
        }
    }

    /// O passo de tipo, como **par** `(corpo, entrelinha)`.
    ///
    /// Um par, e não dois métodos, porque os dois números do Tailwind andam juntos: separados, dá pra
    /// trocar o corpo e esquecer a linha, e aí a razão de ouro do GPUI volta pela porta de trás. Ver
    /// [`LINE_XS`].
    pub fn text(self) -> (f32, f32) {
        match self {
            // O Tailwind não tem passo abaixo de `text-xs`, então o `Sm` fica no mesmo — 16 de linha
            // ainda cabe folgado nos 24px da caixa.
            AvatarSize::Sm | AvatarSize::Default => (TEXT_XS, LINE_XS),
            AvatarSize::Lg => (TEXT_SM, LINE_SM),
        }
    }

    /// O lado do ícone de [`Avatar::icon`] — metade da caixa (ver [`ICON_RATIO`]).
    pub fn icon_size(self) -> f32 {
        self.box_size() * ICON_RATIO
    }
}

// =================================================================================================
// Ajuste da imagem
// =================================================================================================

/// Como a foto preenche o círculo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AvatarFit {
    /// **`object-cover` — o da referência.** Preenche a caixa mantendo a proporção, aparando a sobra.
    ///
    /// Com fonte quadrada (o normal numa foto de perfil) o resultado é exato e o círculo é perfeito.
    /// Com fonte não quadrada, a sobra é aparada pelo `overflow_hidden` da raiz — que é RETANGULAR, e
    /// por isso os cantos saem quadrados. Ver a seção do ContentMask no doc do módulo.
    #[default]
    Cover,
    /// **Superset**: estica a imagem até a caixa, sem manter a proporção.
    ///
    /// A saída de emergência pro caso acima: o retângulo da imagem passa a ser a caixa, então o
    /// `rounded_full` dela vira o círculo exato. Custa distorção — use quando a fonte não é quadrada
    /// e o círculo importa mais que a proporção.
    Fill,
}

impl AvatarFit {
    /// Todos os ajustes. Lista única, mesma razão do [`AvatarSize::ALL`].
    pub const ALL: [AvatarFit; 2] = [AvatarFit::Cover, AvatarFit::Fill];

    /// O [`ObjectFit`] do GPUI correspondente.
    ///
    /// A tradução vive num lugar só, e o enum daqui não é o do GPUI de propósito: o [`ObjectFit`] tem
    /// cinco variantes, e três delas (`Contain`, `ScaleDown`, `None`) deixam vão dentro do círculo —
    /// não são escolhas de avatar. Além disso ele não deriva `Copy`, `PartialEq` nem `Default`, que é
    /// o que este enum precisa ser pra caber num [`RenderOnce`] e num teste.
    fn object_fit(self) -> ObjectFit {
        match self {
            AvatarFit::Cover => ObjectFit::Cover,
            AvatarFit::Fill => ObjectFit::Fill,
        }
    }
}

// =================================================================================================
// Fallback
// =================================================================================================

/// O que o fallback mostra. Os `children` do `AvatarFallback` do original, nos dois casos concretos
/// que esta lib oferece (ver "Ausente" no doc do módulo).
#[derive(Clone, Debug, PartialEq, Eq)]
enum FallbackKind {
    /// As iniciais, em `text-xs` medium.
    Initials(SharedString),
    /// Um ícone SVG, a meia caixa, servido pela [`crate::assets::Assets`].
    Icon(SharedString),
}

/// O disco do fallback — `flex size-full items-center justify-center rounded-full bg-muted`.
///
/// O `rounded_full` dele não é decorativo: é ele que arredonda o disco, porque o `overflow_hidden` da
/// raiz recorta pela caixa e não pelo raio (ver o doc do módulo). Aqui o `size-full` é o da
/// referência — o disco cobre a raiz inteira, que não tem respiro.
fn fallback_element(kind: FallbackKind, size: AvatarSize, p: &AvatarPalette) -> Div {
    let base = div()
        .flex()
        .items_center()
        .justify_center()
        .size_full()
        .rounded_full()
        .bg(p.muted.hsla());
    match kind {
        FallbackKind::Initials(texto) => base.child(texto),
        FallbackKind::Icon(path) => base.child(
            gpui::svg()
                .path(path)
                .size(px(size.icon_size()))
                .flex_none()
                .text_color(p.fg.hsla()),
        ),
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// O **retrato redondo** de uma pessoa, com o visual do coss. Ver o doc do módulo.
///
/// É um **elemento de render** ([`RenderOnce`]): construa a cada frame. O estado de carga da imagem
/// não é dele — é do cache de asset do GPUI, que sobrevive entre frames e avisa a janela quando a
/// imagem chega (`Window::use_asset`). Por isso o avatar **não precisa de [`gpui::ElementId`]**:
/// diferente do que o `with_loading` do `img` exigiria, aqui não há estado de elemento em jogo.
#[derive(IntoElement, Default)]
pub struct Avatar {
    /// A foto. `None` = só o fallback.
    image: Option<ImageSource>,
    /// O que mostrar enquanto a foto carrega, se ela falhar, ou se não há foto. `None` = o círculo
    /// vazio de `--background`, que é o que o original faz com uma raiz sem filhos.
    fallback: Option<FallbackKind>,
    size: AvatarSize,
    fit: AvatarFit,
}

impl Avatar {
    /// Um avatar **vazio**: só o círculo de `--background`.
    ///
    /// É o `<Avatar />` sem filhos do original. Serve de placeholder de carregamento de uma lista
    /// (skeleton) — e é a base pra encadear [`Self::with_initials`] ou [`Self::with_icon`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Um avatar com **foto**. Encadeie [`Self::with_initials`] pra dar a ele o que mostrar enquanto
    /// a foto não está lá — que é o uso canônico:
    ///
    /// ```ignore
    /// Avatar::image("https://exemplo.com/matheus.png").with_initials("MP")
    /// ```
    ///
    /// A fonte segue as conversões do [`gpui::ImageSource`], e a distinção importa: um texto que
    /// parseia como URI vira `Resource::Uri` (baixado por HTTP); qualquer outro vira
    /// `Resource::Embedded`, ou seja um caminho servido pela [`crate::assets::Assets`] — a MESMA
    /// `AssetSource` dos ícones. Sem ela registrada no bootstrap, uma foto embutida falha
    /// SILENCIOSAMENTE e o que fica é o fallback.
    pub fn image(source: impl Into<ImageSource>) -> Self {
        Self {
            image: Some(source.into()),
            ..Self::default()
        }
    }

    /// Um avatar de **iniciais**, sem foto.
    ///
    /// O texto sai como veio: abreviar um nome é decisão de domínio (quantas letras, se maiúsculas, o
    /// que fazer com nome de uma palavra só) e o original também não a toma.
    pub fn initials(text: impl Into<SharedString>) -> Self {
        Self {
            fallback: Some(FallbackKind::Initials(text.into())),
            ..Self::default()
        }
    }

    /// Um avatar de **ícone**, sem foto — o glifo genérico de quem não tem nome pra abreviar.
    ///
    /// O caminho é servido pela [`crate::assets::Assets`] (ex.: `"iconoir/regular/user.svg"`). Sem
    /// essa `AssetSource` registrada no bootstrap, o ícone some SILENCIOSAMENTE e sobra o disco.
    pub fn icon(path: impl Into<SharedString>) -> Self {
        Self {
            fallback: Some(FallbackKind::Icon(path.into())),
            ..Self::default()
        }
    }

    /// As **iniciais** por baixo da foto. Substitui o fallback anterior, se houver.
    pub fn with_initials(mut self, text: impl Into<SharedString>) -> Self {
        self.fallback = Some(FallbackKind::Initials(text.into()));
        self
    }

    /// O **ícone** por baixo da foto. Substitui o fallback anterior, se houver.
    pub fn with_icon(mut self, path: impl Into<SharedString>) -> Self {
        self.fallback = Some(FallbackKind::Icon(path.into()));
        self
    }

    /// O tamanho. O default é o `size-8` da referência.
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Como a foto preenche o círculo. O default é o `object-cover` da referência.
    pub fn fit(mut self, fit: AvatarFit) -> Self {
        self.fit = fit;
        self
    }
}

/// A **foto**, fora do fluxo e empilhada sobre o fallback.
///
/// Função separada (e não um encadeamento dentro do `render`) por um motivo só: assim o teste
/// `a_foto_fica_fora_do_fluxo_e_preenche_a_caixa` consegue LER o estilo dela. Duas mutações minhas
/// tinham sobrevivido aos 12 testes originais — tirar o `absolute` (o fallback passaria a ficar ao
/// LADO da foto, cada um com metade da caixa) e trocar a ordem dos dois filhos. A primeira este teste
/// pega; a segunda é ordem de PINTURA, que nenhum teste desta base alcança — ver o doc do módulo.
fn photo_element(source: gpui::ImageSource, fit: AvatarFit) -> gpui::Img {
    gpui::img(source)
        // `absolute` + `size-full` em vez do `size-full` em fluxo do original: mesma caixa, mas
        // empilhada sobre o fallback (desvio declarado no doc do módulo).
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        // O recorte redondo da FOTO. Não vem do pai — ver o doc do módulo.
        .rounded_full()
        .object_fit(fit.object_fit())
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = palette();
        let s = self.size;
        let (corpo, entrelinha) = s.text();

        let mut el = div()
            // `relative` porque a imagem é filha ABSOLUTA — é o que a empilha sobre o fallback.
            .relative()
            .flex()
            // `shrink-0`: um avatar não encolhe numa linha apertada. Ele é um retrato, não um espaço.
            .flex_none()
            .items_center()
            .justify_center()
            // A metade RETANGULAR do `overflow-hidden`: apara o que uma foto em `Cover` derrama pra
            // fora dos 32px. A metade redonda é de cada filho (ver o doc do módulo).
            .overflow_hidden()
            .size(px(s.box_size()))
            .rounded_full()
            .bg(p.background.hsla())
            .text_size(px(corpo))
            .line_height(px(entrelinha))
            .font_weight(gpui::FontWeight::MEDIUM)
            // Superset declarado: o GPUI não tem `inherit`.
            .text_color(p.fg.hsla());

        // O fallback PRIMEIRO: sem z-index no GPUI, a ordem dos filhos é a ordem de pintura, e é ela
        // que põe a foto em cima. Ver "A troca imagem↔fallback" no doc do módulo.
        if let Some(kind) = self.fallback {
            el = el.child(fallback_element(kind, s, p));
        }
        if let Some(source) = self.image {
            el = el.child(photo_element(source, self.fit));
        }
        el
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável ("this assertion has a constant
// value"), presumindo que quem escreveu quis testar algo variável. Aqui é o contrário: travar o
// valor que veio da referência É o propósito destes testes — eles são a documentação executável de
// quanto mede cada coisa. Mesma decisão em todos os módulos portados do coss.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::color::opaque;
    use gpui::{point, size, Bounds, DevicePixels, Pixels};

    /// **O tamanho da referência é 32×32**, e é o default.
    ///
    /// Os outros dois são superset (ver o doc do módulo), e o que este teste guarda deles é que estão
    /// na escala de espaçamento do Tailwind (múltiplos de 4) e em ordem — um passo fora da escala
    /// seria um número inventado sem par no design system.
    #[test]
    fn a_caixa_da_referencia_e_o_size_8() {
        assert_eq!(AvatarSize::default(), AvatarSize::Default, "o default é o da referência");
        assert_eq!(AvatarSize::Default.box_size(), 32.0, "size-8 = 32px");

        let esperado = [
            (AvatarSize::Sm, 24.0),      // size-6
            (AvatarSize::Default, 32.0), // size-8 — a referência
            (AvatarSize::Lg, 40.0),      // size-10
        ];
        for (size, lado) in esperado {
            assert_eq!(size.box_size(), lado, "{size:?}: lado");
            assert_eq!(
                size.box_size() % 4.0,
                0.0,
                "{size:?}: {} não está na escala do Tailwind (1 unidade = 4px)",
                size.box_size()
            );
        }
        assert_eq!(AvatarSize::ALL.len(), 3, "a lista única cobre os três");
        // Em ordem estrita: `ALL` é a ordem de tamanho, e o storybook conta com isso.
        for par in AvatarSize::ALL.windows(2) {
            assert!(
                par[0].box_size() < par[1].box_size(),
                "{:?} tem que ser menor que {:?}",
                par[0],
                par[1]
            );
        }
    }

    /// **A armadilha da entrelinha, travada em número.** Todo `text-*` do Tailwind é um PAR
    /// `(corpo, linha)`; sem declarar a linha, o GPUI usa `relative(1.618_034)`.
    ///
    /// O teste guarda os pares E mostra o número que sairia sem fixar.
    #[test]
    fn entrelinha_e_o_par_do_tailwind() {
        assert_eq!(AvatarSize::Default.text(), (12.0, 16.0), "o par do text-xs — a referência");
        assert_eq!(AvatarSize::Sm.text(), (12.0, 16.0), "o Sm fica no text-xs");
        assert_eq!(AvatarSize::Lg.text(), (14.0, 20.0), "o par do text-sm");

        // O que aconteceria sem fixar, em cada passo.
        const RAZAO_DE_OURO: f32 = 1.618_034;
        for size in AvatarSize::ALL {
            let (corpo, linha) = size.text();
            let solta = corpo * RAZAO_DE_OURO;
            assert!(
                solta > linha + 2.0,
                "{size:?}: sem fixar, a linha sairia {solta:.1}px em vez de {linha}"
            );
            // A entrelinha tem que CABER na caixa: uma linha maior que o círculo transbordaria o
            // texto, e aí o `overflow_hidden` cortaria as iniciais pela metade.
            assert!(
                linha < size.box_size(),
                "{size:?}: entrelinha {linha} não cabe na caixa de {}",
                size.box_size()
            );
        }
        // O número da referência, explícito: 12 × φ = 19,4, e não 16.
        assert!((TEXT_XS * RAZAO_DE_OURO - 19.4).abs() < 0.05);
    }

    /// O ícone do fallback é **meia caixa**, e a razão cai em cima dos passos do Tailwind nos três
    /// tamanhos: `size-3` / `size-4` / `size-5`.
    ///
    /// Os valores estão escritos à mão de propósito — comparar com `box_size() * ICON_RATIO` seria
    /// conferir a conta contra ela mesma.
    #[test]
    fn icone_do_fallback_e_meia_caixa() {
        let esperado = [
            (AvatarSize::Sm, 12.0),      // size-3
            (AvatarSize::Default, 16.0), // size-4
            (AvatarSize::Lg, 20.0),      // size-5
        ];
        for (size, lado) in esperado {
            assert_eq!(size.icon_size(), lado, "{size:?}: lado do ícone");
            assert_eq!(size.icon_size() % 4.0, 0.0, "{size:?}: fora da escala do Tailwind");
            // Sobra respiro em volta: um ícone que enche a caixa encostaria na aresta do círculo,
            // onde o recorte redondo o cortaria.
            assert!(size.icon_size() * 2.0 <= size.box_size());
        }
        assert_eq!(ICON_RATIO, 0.5);
    }

    /// Os tokens do tema entram por [`opaque`], que é a única ponte entre os `0xRRGGBB` de
    /// [`crate::theme`] e os `0xRRGGBBAA` daqui.
    #[test]
    fn tokens_opacos_entram_por_opaque() {
        assert_eq!(AVATAR_LIGHT.background, opaque(0xffffff), "--background claro = white");
        // `--background` escuro = mix(neutral-950 96%, white) = 10·0,96 + 255·0,04 ≈ 20 = 0x14.
        assert_eq!(AVATAR_DARK.background, opaque(0x141414));
        assert_eq!(AVATAR_LIGHT.fg, opaque(0x262626), "neutral-800");
        assert_eq!(AVATAR_DARK.fg, opaque(0xf5f5f5), "neutral-100");
    }

    /// As paletas decodificam pras cores pretendidas — o teste que pega a inversão de canais que a
    /// convenção `0xRRGGBBAA` existe pra evitar (um `0x0a000000` no lugar do `0x0000000a` compila e
    /// dá preto quase opaco em vez de preto a 4%).
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        for (nome, cor) in [("muted claro", AVATAR_LIGHT.muted), ("muted escuro", AVATAR_DARK.muted)]
        {
            assert!(
                (cor.alpha() - 0.04).abs() < 0.01,
                "{nome}: --muted é 4% de alfa, veio {:.3}",
                cor.alpha()
            );
        }
        // O RGB de cada um: preto no claro, branco no escuro.
        assert_eq!(AVATAR_LIGHT.muted.0 & 0xffffff00, 0x00000000, "--alpha(black / 4%)");
        assert_eq!(AVATAR_DARK.muted.0 & 0xffffff00, 0xffffff00, "--alpha(white / 4%)");

        // E o contraste que faz as iniciais serem legíveis: o texto é quase preto sobre fundo quase
        // branco no claro, e o inverso no escuro. Se as duas paletas fossem iguais (o erro de copiar
        // uma pra outra), este par de comparações não fecharia.
        let luz = |c: Rgba8| c.0 >> 24;
        assert!(luz(AVATAR_LIGHT.fg) < luz(AVATAR_LIGHT.background), "claro: texto escuro");
        assert!(luz(AVATAR_DARK.fg) > luz(AVATAR_DARK.background), "escuro: texto claro");
    }

    /// **O `bg-muted` do fallback se COMPÕE sobre o `bg-background` da raiz, não o substitui.**
    ///
    /// É a razão de ele ser um disco de cinza levíssimo e não um bloco chapado — e a razão de o
    /// `--muted` ser translúcido. Se alguém "resolver" o token pra uma cor opaca (achando que 4% de
    /// preto sobre branco é `#f5f5f5`), o avatar deixa de funcionar sobre qualquer superfície que não
    /// seja `--background` — dentro de um [`crate::card`], por exemplo.
    #[test]
    fn fallback_se_compoe_sobre_o_fundo_da_raiz() {
        for (nome, p) in [("claro", &AVATAR_LIGHT), ("escuro", &AVATAR_DARK)] {
            assert!(
                p.muted.alpha() < 1.0,
                "{nome}: --muted tem que ser translúcido, veio alfa {:.3}",
                p.muted.alpha()
            );
            assert_eq!(
                p.background.alpha(),
                1.0,
                "{nome}: --background é a superfície, e ela é opaca"
            );
        }
    }

    // --- O círculo e o `object-fit` ---------------------------------------------------------------

    /// A caixa de um avatar no tamanho da referência, na origem.
    fn caixa(tam: AvatarSize) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(tam.box_size()), px(tam.box_size())),
        }
    }

    /// **Com fonte quadrada, `Cover` devolve EXATAMENTE a caixa** — e é isso que faz o
    /// `rounded_full` da imagem virar um círculo perfeito, já que o `paint_image` aplica o
    /// `corner_radii` ao retângulo do `object-fit`, não à caixa do elemento.
    ///
    /// O teste chama a função do PRÓPRIO GPUI (`ObjectFit::get_bounds`) através do nosso
    /// [`AvatarFit`], então ele cobre a tradução e o comportamento de verdade — não uma réplica da
    /// conta.
    #[test]
    fn cover_de_fonte_quadrada_da_o_circulo_exato() {
        for tam in AvatarSize::ALL {
            let b = caixa(tam);
            // Uma foto de perfil típica: quadrada, e maior que a caixa.
            let fonte = size(DevicePixels(256), DevicePixels(256));
            let r = AvatarFit::Cover.object_fit().get_bounds(b, fonte);
            assert_eq!(r, b, "{tam:?}: fonte quadrada em Cover tem que dar a caixa inteira");
            // E o raio aparado (`min(w,h)/2`) é metade do lado: o círculo.
            let raio = f32::min(f32::from(r.size.width), f32::from(r.size.height)) / 2.0;
            assert_eq!(raio, tam.box_size() / 2.0, "{tam:?}: raio de círculo");
        }
    }

    /// ⚠️ **O limite declarado: com fonte NÃO quadrada, `Cover` perde o círculo.**
    ///
    /// O `corner_radii` do `paint_image` vale pro retângulo do `object-fit`. Numa fonte 2:1 dentro de
    /// 32×32, `Cover` devolve 64×32 começando 16px à esquerda: o aparo dá raio 16 nessa pista de
    /// atletismo, os cantos redondos ficam FORA da caixa, e o `overflow_hidden` da raiz — que é um
    /// ContentMask retangular — corta um quadrado. Ver a seção do ContentMask no doc do módulo.
    ///
    /// Este teste é a prova de que o `AvatarFit::Fill` não é enfeite: ele é a única saída.
    #[test]
    fn cover_perde_o_circulo_quando_a_fonte_nao_e_quadrada() {
        let b = caixa(AvatarSize::Default);
        let fonte = size(DevicePixels(128), DevicePixels(64)); // 2:1

        let cover = AvatarFit::Cover.object_fit().get_bounds(b, fonte);
        assert_eq!(f32::from(cover.size.width), 64.0, "Cover dobra a largura pra cobrir a altura");
        assert_eq!(f32::from(cover.size.height), 32.0);
        assert_eq!(f32::from(cover.origin.x), -16.0, "e centra, sobrando 16px de cada lado");
        assert_ne!(cover, b, "o retângulo da imagem NÃO é a caixa — é aí que o círculo se perde");

        // O `Fill` é a saída: o retângulo da imagem passa a ser a caixa, e o `rounded_full` dela
        // volta a ser o círculo exato.
        let fill = AvatarFit::Fill.object_fit().get_bounds(b, fonte);
        assert_eq!(fill, b, "Fill devolve a caixa, custe a proporção");

        // E o default segue FIEL ao `object-cover` do original — a saída existe, mas não é o padrão.
        assert_eq!(Avatar::new().fit, AvatarFit::Cover, "o default é o da referência");
        assert_eq!(AvatarFit::ALL.len(), 2);
    }

    // --- A troca imagem↔fallback -----------------------------------------------------------------

    /// **Foto e fallback coexistem** — é o que permite a foto pintar EM CIMA do fallback, e o que dá
    /// o `delay: 0` do base-ui sem estado de elemento (ver o doc do módulo).
    ///
    /// Um `with_initials` que zerasse a imagem (ou um construtor que zerasse o fallback) devolveria o
    /// comportamento de ramos alternativos, e aí o buraco de carregamento voltaria.
    #[test]
    fn foto_e_fallback_coexistem() {
        let a = Avatar::image("foto.png").with_initials("MP");
        assert!(a.image.is_some(), "a foto continua lá depois do with_initials");
        assert_eq!(a.fallback, Some(FallbackKind::Initials("MP".into())));

        let b = Avatar::image("foto.png").with_icon("iconoir/regular/user.svg");
        assert!(b.image.is_some());
        assert_eq!(b.fallback, Some(FallbackKind::Icon("iconoir/regular/user.svg".into())));
    }

    /// Cada construtor monta só o que o nome diz — um `Avatar::initials` com imagem embutida
    /// carregaria uma foto que ninguém pediu.
    #[test]
    fn construtores_montam_so_o_que_o_nome_diz() {
        let vazio = Avatar::new();
        assert!(vazio.image.is_none() && vazio.fallback.is_none());

        let so_foto = Avatar::image("foto.png");
        assert!(so_foto.image.is_some());
        assert!(so_foto.fallback.is_none(), "foto sem fallback é o `<AvatarImage>` sozinho");

        let so_iniciais = Avatar::initials("MP");
        assert!(so_iniciais.image.is_none(), "iniciais não carregam foto");
        assert_eq!(so_iniciais.fallback, Some(FallbackKind::Initials("MP".into())));

        let so_icone = Avatar::icon("u.svg");
        assert!(so_icone.image.is_none());
        assert_eq!(so_icone.fallback, Some(FallbackKind::Icon("u.svg".into())));

        // O último fallback declarado vence — os dois `with_*` escrevem no MESMO lugar, então não há
        // como acabar com iniciais e ícone empilhados dentro do disco.
        let trocado = Avatar::initials("MP").with_icon("u.svg");
        assert_eq!(trocado.fallback, Some(FallbackKind::Icon("u.svg".into())));
    }

    /// **O default do componente é o do original**: 32px, `object-cover`, sem foto e sem fallback.
    #[test]
    fn defaults_batem_com_a_referencia() {
        let a = Avatar::default();
        assert_eq!(a.size, AvatarSize::Default);
        assert_eq!(a.fit, AvatarFit::Cover);
        assert!(a.image.is_none());
        assert!(a.fallback.is_none());
    }

    /// **O número que justifica NÃO usar o `with_loading` do [`gpui::img`].**
    ///
    /// O `delay` do `AvatarFallback` do base-ui tem default **0**, e o coss não o troca — o fallback da
    /// referência aparece na hora. O caminho dos ganchos do `img` custaria `gpui::LOADING_DELAY` de
    /// buraco, e esse número não é configurável por quem chama. É por isso que o fallback aqui é irmão
    /// por baixo da foto, e não um ramo alternativo.
    ///
    /// O teste é uma sentinela de VERSÃO: se um upgrade do gpui zerar o `LOADING_DELAY` (ou o tornar
    /// ajustável), ele falha e a decisão do módulo volta pra mesa em vez de virar folclore.
    #[test]
    fn o_atraso_do_img_do_gpui_nao_serve_pro_delay_zero_da_referencia() {
        const DELAY_DA_REFERENCIA_MS: u64 = 0;
        let gpui_ms = gpui::LOADING_DELAY.as_millis();
        assert_eq!(gpui_ms, 200, "o LOADING_DELAY do gpui 0.2.2");
        assert!(
            gpui_ms > u128::from(DELAY_DA_REFERENCIA_MS),
            "se o gpui chegar a {DELAY_DA_REFERENCIA_MS}ms, o with_loading passa a servir"
        );
    }

    /// **A foto fica FORA DO FLUXO e preenche a caixa.** É o mecanismo em que o componente inteiro se
    /// apoia: a troca imagem↔fallback aqui não é estado, é EMPILHAMENTO — o fallback fica embaixo, em
    /// fluxo, e a foto por cima, absoluta. Uma foto que carrega esconde o fallback; uma que falha não
    /// pinta nada e o deixa visível. É assim que o `delay: 0` da referência sai de graça (ver o doc do
    /// módulo).
    ///
    /// Este teste existe porque a asserção estava FALTANDO: tirar o `.absolute()` passava nos 12
    /// testes originais, e o resultado seria o fallback ao LADO da foto, cada um com metade dos 32px.
    #[test]
    fn a_foto_fica_fora_do_fluxo_e_preenche_a_caixa() {
        use gpui::{Length, Position};
        let mut el = photo_element(gpui::ImageSource::from("x.png"), AvatarFit::Cover);
        let style = el.style().clone();
        assert_eq!(
            style.position,
            Some(Position::Absolute),
            "sem `absolute` a foto entra no fluxo e divide a caixa com o fallback"
        );
        let cheio = Some(Length::Definite(gpui::relative(1.0)));
        assert_eq!(style.size.width, cheio, "size-full na largura");
        assert_eq!(style.size.height, cheio, "size-full na altura");
        // E o recorte redondo é DELA, não do pai: o `overflow_hidden` do GPUI é retangular.
        assert!(style.corner_radii.is_some(), "a foto carrega o próprio raio");
    }
}

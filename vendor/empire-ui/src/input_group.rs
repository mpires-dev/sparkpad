//! `InputGroup` — **a superfície do campo virada wrapper**, com addons nas quatro posições.
//!
//! Porte do `input-group.tsx` do coss. É o componente que resolve o que o `prefix`/`suffix` do
//! [`crate::Input`] não resolve: peças acima e abaixo do campo, dentro da MESMA moldura.
//!
//! ```ignore
//! // busca com ícone à esquerda e atalho à direita
//! InputGroup::new(&self.busca)
//!     .addon(InputGroupAddon::inline_start().icon("iconoir/regular/search.svg"))
//!     .addon(InputGroupAddon::inline_end().kbd("⌘K"))
//!
//! // textarea com uma barra de ferramentas embaixo, na mesma moldura
//! InputGroup::textarea(&self.mensagem, 3)
//!     .addon(
//!         InputGroupAddon::block_end()
//!             .divider()
//!             .text("Markdown")
//!             .control(Button::new("enviar", "Enviar")),
//!     )
//! ```
//!
//! # A ideia: uma superfície, quatro lugares pra pendurar coisa
//!
//! O `input.tsx` e o `textarea.tsx` do coss têm a moldura no **wrapper** do campo, e o `<input>` de
//! dentro é só o miolo. O `input-group.tsx` faz exatamente a mesma moldura num wrapper MAIOR e manda o
//! campo entrar `unstyled` — as classes da superfície (`rounded-lg border border-input bg-background`,
//! o bisel do `before:`, e as regras de `has-focus-visible`/`has-aria-invalid`/`has-disabled`) são
//! **as mesmas**, caractere por caractere.
//!
//! Então aqui **não existe paleta nova nem geometria nova**: a cor da borda vem de
//! [`crate::input::border_color_for`], o fundo de [`crate::input::field`], a sombra de
//! [`crate::input::shadow_stack_for`], o bisel de [`crate::input::bevel_for`], o anel de
//! [`crate::input::ring_overlay`] e o raio de [`crate::input::FIELD_RADIUS`]. Duplicar qualquer um
//! desses números criaria uma segunda fonte de verdade pra superfície do campo — o erro que esta base
//! já pagou três vezes (ver [`crate::color`]).
//!
//! # Isto não é um [`crate::group::Group`]
//!
//! O doc de módulo do [`crate::group`] tem a seção **"Group não é InputGroup"** com a tabela que
//! separa os dois; ela vale como está e este módulo é o outro lado dela:
//!
//! | | superfícies | borda | anel de foco | Tab |
//! |---|---|---|---|---|
//! | `InputGroup` | **uma** | uma | envolve o conjunto | 1 parada |
//! | [`crate::group::Group`] | **várias** | uma por filho | um por filho | uma por filho |
//!
//! Um addon **não é um filho costurado**: ele não tem borda, não tem fundo, não tem raio e não recebe
//! foco. Se este porte começar a desenhar borda por addon, ou a costurar emendas com
//! [`crate::group::Join`], ele virou um `Group` e está errado.
//!
//! # Clicar no grupo foca o campo — e o clique num controle não
//!
//! O original resolve isso sniffando o DOM: o `onMouseDown` do addon roda
//! `target.closest("button, a, input, select, textarea, [role=button], …")` e desiste se achar algo.
//! No GPUI não há DOM pra interrogar, então a informação vira **estrutura**: um item entra no addon
//! como conteúdo inerte ([`InputGroupAddon::text`], [`InputGroupAddon::icon`],
//! [`InputGroupAddon::badge`], [`InputGroupAddon::kbd`], [`InputGroupAddon::child`]) ou como
//! **controle** ([`InputGroupAddon::control`]).
//!
//! O handler de foco fica no addon; cada controle é embrulhado num elemento que faz
//! `cx.stop_propagation()` no mouse-down. Como a fase de bolha do GPUI percorre os ouvintes em ordem
//! INVERSA de pintura (`Window::dispatch_mouse_event`), o controle é visitado antes do addon que o
//! contém — então o clique num controle nunca chega ao handler de foco, e o clique no respiro/no vão
//! do addon chega. É a mesma semântica do `closest`, sem sniffar nada.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`order-first` / `order-last`**: o GPUI não tem `order`. A ordem passa a ser a de construção da
//!   árvore, resolvida por [`particionar`] — os addons de bloco de início primeiro, depois a linha
//!   (inline-start · campo · inline-end), depois os de bloco de fim.
//! - **`truncate` no texto do addon**: o GPUI 0.2.2 não trunca texto sozinho. Um rótulo comprido
//!   estoura em vez de virar `…`.
//! - **`select-none`, `outline-none`, `transition-shadow`**: não há seleção de texto de rótulo, nem
//!   contorno de foco nativo, nem transição declarativa. O suavizado do anel de foco existe e vem de
//!   graça do campo hospedado, que faz o cross-fade à mão (ver [`crate::input`]).
//! - **`has-autofill:bg-foreground/4`**: não existe autofill de navegador num app nativo.
//! - **`max-sm:min-h-23.5`** e tudo que é `max-sm:`: uma janela de desktop está sempre acima do
//!   breakpoint `sm:` (≥640px). Valem as variantes `sm:`, e é por isso que o texto é 14 e não 16 e o
//!   ícone 16 e não 18 — mesma decisão do [`crate::Input`] e do [`crate::toggle`].
//!
//! **Resolvido em número**
//!
//! - `px-[calc(--spacing(3)-1px)]` = **11**, e `[[data-size=sm]+&]:px-[calc(--spacing(2.5)-1px)]` =
//!   **9**. São os MESMOS números do respiro do campo, então vêm de
//!   [`crate::input::InputSize::pad_x`] em vez de constantes novas.
//! - Os puxões do original entram como **geometria, não como margem**: margem negativa já colapsou o
//!   layout das abas nesta base (ver `crate::tabs`), e a regra da casa é que não se usa. Um
//!   `-ms-2` ao lado de um `ps-11` é, geometricamente, um respiro de 3px — então é 3px que se
//!   escreve. Ver [`pull`] e [`addon_lead_pad`]; é o mesmo tratamento que o
//!   [`crate::group::GroupText`] dá ao `-mx-0.5` do ícone dele.
//! - `**:[input]:ps-2` / `pe-2` (e `ps-1.5` no `Sm`) = **8** e **6**: o respiro do campo ENCOLHE do
//!   lado onde há addon inline, senão o texto ficaria 11px longe do ícone. Ver [`field_pad`].
//! - `**:[textarea]:py-[calc(--spacing(3)-1px)]` = **11**: a textarea já traz 6 de
//!   respiro do próprio tamanho (3/5/7), e o grupo completa o que falta (8/6/4)
//!   ([`textarea_pad_y_complement`]) — um número só, derivado, em vez de dois que podem divergir.
//! - `min-h-20.5` (82px) da textarea dentro do grupo: aqui a altura vem do `rows` de
//!   [`InputGroup::textarea`], que é o que de fato se especifica ("três linhas"), e 82px são ≈2,3
//!   linhas. Um `min_h` por cima do `rows` só criaria um vão morto e inclicável no fim do campo.
//! - `[&>kbd]:rounded-[calc(var(--radius)-5px)]` (5px): fica pra quando o crate tiver um `Kbd`. O
//!   raio é do `<kbd>`, não do addon — declarar aqui seria constante morta.
//!
//! **Desvio consciente**
//!
//! - O fio de bisel do tema escuro é o DOBRO da referência (branco ~11,8% em vez de 6%). Não é
//!   decisão deste módulo: ele reusa [`crate::input::bevel_for`], e o desvio já vale em 7 módulos —
//!   ver o teste `bisel_escuro_e_o_dobro_da_referencia` em [`crate::input`].
//! - **O encolhimento do respiro vale também pra textarea.** No original
//!   `has-data-[align=inline-start]:**:[input]:ps-2` alcança só o `input`, e uma textarea com ícone à
//!   esquerda mantém os 11px. Aqui a regra é uma só: um ícone com 11px de vão até o texto abre um
//!   buraco visível, e não há razão de desenho pra a textarea ser diferente.
//! - **A validade é um `bool`** ([`InputGroup::invalid`]), não um [`crate::input::Validity`]. O grupo
//!   é só a superfície: a MENSAGEM de erro pertence à moldura de formulário (label/hint/contador do
//!   [`crate::Input`]), que fica FORA do grupo. Um `Validity` aqui carregaria uma mensagem que este
//!   componente não tem onde mostrar.
//! - **`InputGroupText` não virou um tipo.** No original ele é um `<span>` que empresta ao conteúdo o
//!   `flex items-center gap-2` e a cor `--muted-foreground` — e o addon já é `flex items-center
//!   gap-2`, então o que sobrava dele era a cor. Virou [`InputGroupAddon::text`]. Dois tipos pro mesmo
//!   pixel seriam duas formas de escrever a mesma coisa.
//! - **`**:[textarea]:resize-none` não é escrito aqui, e vale.** A alça de redimensionar da textarea
//!   desta lib (um superset consciente — ver [`crate::input`]) é peça da MOLDURA do campo, e o grupo
//!   hospeda o campo em [`crate::Input::unstyled`], sem moldura. Então ela não aparece aqui sem que este
//!   módulo diga nada — e é exatamente o que o `resize-none` do original pede, no mesmo lugar.
//!   Travado em `sem_moldura_nao_ha_alca`, em `tests/textarea_resize_handle.rs`.
//!
//! **Superset consciente**
//!
//! - **Misturar addon de bloco com addon inline funciona.** No original, um addon de bloco põe o grupo
//!   em `flex-col` e os addons inline viram linhas soltas na coluna — degenera. Aqui a linha do meio é
//!   um container próprio, então `[bloco de cima] / [ícone · campo · botão] / [bloco de baixo]` é o que
//!   se vê. Ver [`particionar`].
//! - **[`InputGroupAddon::divider`]** é o `[.border-t]:pt-…` / `[.border-b]:pb-…` do original virado
//!   opção nomeada: lá você liga a divisória escrevendo `className="border-t"` no addon e a regra do
//!   grupo reage; aqui a intenção é declarada e o respiro extra vem junto, sem depender de uma classe
//!   solta que se pode esquecer.
//!
//! **Ausente**
//!
//! - `*:[[data-slot=input-control]]:contents` — o `display: contents` que faz o wrapper do campo
//!   desaparecer do layout. Aqui o campo hospedado é um filho flex normal, com
//!   [`crate::Input::pad_x`] zerado e o respiro no wrapper do grupo, que é o que aquele truque
//!   existia pra permitir.
//! - `**:[textarea_button]:rounded-[calc(var(--radius-md)-1px)]` (7px) — raio de botão dentro da
//!   textarea; é o [`crate::Button`] que decide o raio dele.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, CursorStyle, Div, Entity, Focusable as _, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
};
use gpui_component::input::InputState;

use crate::color::Rgba8;
use crate::group::Join;
use crate::input::{self, Input, InputSize};

// =================================================================================================
// Números (utilitários do coss resolvidos)
// =================================================================================================

/// `gap-2` — o vão entre os itens de um addon.
const ADDON_GAP: f32 = 8.0;

/// `sm:text-sm` — o corpo do texto do grupo.
const TEXT_SIZE: f32 = 14.0;

/// A entrelinha que **acompanha** `text-sm` no Tailwind: 14/20.
///
/// ⚠️ Tem que ser declarada. O default do GPUI é `relative(1.618034)`, que num corpo de 14 daria
/// 22,65 — 2,65px a mais por linha, e num addon de bloco isso empurra a barra inteira. Travado em
/// [`tests::entrelinha_do_grupo_e_o_par_do_tailwind`].
const TEXT_LINE_HEIGHT: f32 = 20.0;

/// `sm:size-4` — o lado dos ícones do addon (a classe base é `size-4.5`/18, que não vale no desktop).
const ICON_SIZE: f32 = 16.0;

/// `not-has-[button]:**:[svg:not([class*='opacity-'])]:opacity-80` — num addon SEM controle os ícones
/// esmaecem. Com controle ficam cheios, pra o ícone do botão não parecer desligado.
const ICON_OPACITY: f32 = 0.8;

/// `**:[input]:ps-2` / `pe-2` — o respiro do campo do lado onde há addon inline.
const FIELD_INLINE_PAD: f32 = 8.0;

/// `[[data-size=sm]_input]:ps-1.5` — o mesmo, no `Sm`.
const FIELD_INLINE_PAD_SM: f32 = 6.0;

/// `**:[input]:pt-1.5` / `pb-1.5` — o respiro do campo contra a borda do grupo, do lado onde NÃO há
/// addon de bloco.
const FIELD_BLOCK_PAD: f32 = 6.0;

/// `**:[textarea]:py-[calc(--spacing(3)-1px)]` — o respiro vertical da textarea DENTRO do grupo.
///
/// É maior que o do campo solto (3/5/7 por tamanho) porque o original declara os
/// dois separadamente: `py-[calc(--spacing(1.5)-1px)]` no `textarea.tsx` e
/// `py-[calc(--spacing(3)-1px)]` no `input-group.tsx`. Quem completa a diferença é
/// [`textarea_pad_y_complement`].
const TEXTAREA_PAD_Y: f32 = 11.0;

/// O raio da moldura do grupo — `rounded-lg`, o **mesmo do campo**.
///
/// É função (e não um `const` novo com um 10 dentro) justamente pra o acoplamento ser verificável:
/// [`tests::superficie_e_a_do_campo_sem_copia`] compara este valor com o
/// [`crate::input::FIELD_RADIUS`], então um raio próprio aqui derruba o teste em vez de passar batido.
pub fn frame_radius() -> f32 {
    input::FIELD_RADIUS
}

/// `--muted-foreground` **cheio** — a cor do texto e dos ícones do addon (`text-muted-foreground`).
///
/// É DERIVADO do `placeholder` da paleta do campo, que é o mesmo token a 72% (`/72` no coss). Repetir
/// `#686868`/`#818181` aqui criaria uma segunda fonte de verdade pro `--muted-foreground` do campo, e
/// é exatamente esse tipo de cópia que já produziu três bugs de cor nesta base (ver [`crate::color`]).
/// O `| 0xff` sobe o alfa pra cheio sem tocar em nenhum canal de cor.
fn muted_fg() -> Rgba8 {
    Rgba8((input::field().placeholder.0 & 0xffff_ff00) | 0xff)
}

/// Quanto falta pro respiro vertical da textarea chegar aos 11px do original, já que o campo
/// hospedado traz o respiro do PRÓPRIO tamanho por conta própria.
///
/// Derivado (e não um número escrito) pra que mexer no respiro do campo solto não descole
/// silenciosamente o do grupo. O `max(0)` existe porque respiro negativo é proibido nesta base: se um
/// dia o campo passar dos 11, o grupo não acrescenta nada em vez de subtrair.
///
/// ⚠️ **Depende do tamanho, e isto foi um acerto de integração.** Este porte foi escrito contra uma
/// constante única (`input::TEXTAREA_PAD_Y`, 6px) que o porte do Textarea — feito em paralelo, no
/// mesmo crate — substituiu por um respiro **por tamanho** (3/5/7, os `py` que a referência declara
/// pra `sm`/default/`lg`). Com a base variando, o complemento varia: **8 / 6 / 4**. A soma continua
/// batendo nos 11 da referência nos três tamanhos, e é isso que o teste afirma.
fn textarea_pad_y_complement(size: InputSize) -> f32 {
    (TEXTAREA_PAD_Y - size.textarea_pad_y()).max(0.0)
}

// =================================================================================================
// Addon
// =================================================================================================

/// Onde o addon fica na moldura. São as quatro variantes do `inputGroupAddonVariants` do original.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddonAlign {
    /// Colado à esquerda do campo, na mesma linha (`order-first ps-*`).
    InlineStart,
    /// Colado à direita do campo, na mesma linha (`order-last pe-*`).
    InlineEnd,
    /// Uma LINHA de largura cheia **acima** do campo (`order-first w-full px-* pt-*`).
    BlockStart,
    /// Uma LINHA de largura cheia **abaixo** do campo (`order-last w-full px-* pb-*`).
    BlockEnd,
}

impl AddonAlign {
    /// Se este alinhamento é uma linha de largura cheia — é ele que põe o grupo em coluna.
    pub fn is_block(self) -> bool {
        matches!(self, AddonAlign::BlockStart | AddonAlign::BlockEnd)
    }
}

/// O que um item do addon **é** — e é isso que decide o puxão dele e se ele rouba o clique.
///
/// A distinção existe porque o original a faz por seletor de DOM (`has-[>button]`,
/// `has-[>:last-child[data-slot=badge]]`, `has-[>kbd:last-child]`, `[&_svg]`) e no GPUI não há DOM pra
/// interrogar. Ver a seção de clique no doc do módulo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddonItemKind {
    /// Texto inerte — nenhum puxão.
    Text,
    /// Ícone inerte, 16px (`[&_svg]:-mx-0.5`).
    Icon,
    /// Um badge (`has-[>:last-child[data-slot=badge]]:-ms-1.5`).
    Badge,
    /// Uma dica de atalho (`has-[>kbd:last-child]:ms-[-0.35rem]`).
    Kbd,
    /// Um controle interativo — botão, select, link (`has-[>button]:-ms-2`). **Não** rouba o clique
    /// pro campo.
    Control,
}

/// O puxão declarado pra cada tipo de item, em px — os valores negativos do original com o sinal
/// tirado, porque aqui eles SAEM do respiro em vez de virar margem.
///
/// `-ms-2` = 8, `-ms-1.5` = 6, `ms-[-0.35rem]` = 5,6 (0,35 × 16), `-mx-0.5` = 2.
pub fn pull(kind: AddonItemKind) -> f32 {
    match kind {
        AddonItemKind::Text => 0.0,
        AddonItemKind::Icon => 2.0,
        AddonItemKind::Badge => 6.0,
        AddonItemKind::Kbd => 5.6,
        AddonItemKind::Control => 8.0,
    }
}

/// O respiro do addon **do lado que encosta na borda do grupo**, já com o puxão descontado.
///
/// A base é [`InputSize::pad_x`] (11, ou 9 no `Sm`) — os mesmos números do respiro do campo, porque no
/// original é literalmente a mesma expressão `calc(--spacing(3)-1px)`.
///
/// # Qual puxão vale, quando há mais de um item
///
/// - Um **controle em qualquer posição** vence: é a leitura do `has-[>button]`, que no original é
///   "algum filho é botão", e um botão de 24px encostando na borda é o caso em que o vão sobra mais.
/// - Sem controle, vale o item que **encosta na borda** (o primeiro num addon de início, o último num
///   de fim). Com um filho só — o caso real de quase todo addon — as três leituras do original
///   (`>button`, `:last-child`, `&_svg`) coincidem.
///
/// Addon de BLOCO não tem puxão: o `cva` do original não declara nenhum pras variantes de bloco, e faz
/// sentido — ali o respiro é o da linha inteira, não o de uma peça encostada na borda.
pub fn addon_lead_pad(size: InputSize, align: AddonAlign, edge: Option<AddonItemKind>) -> f32 {
    if align.is_block() {
        return size.pad_x();
    }
    size.pad_x() - pull(edge.unwrap_or(AddonItemKind::Text))
}

/// O vão entre os itens do addon. Encolhe pelo puxão do ícone quando há ícone, exatamente como o
/// [`crate::group::GroupText`] faz com o `-mx-0.5` do ícone dele.
pub fn addon_gap(has_icon: bool) -> f32 {
    if has_icon {
        ADDON_GAP - pull(AddonItemKind::Icon)
    } else {
        ADDON_GAP
    }
}

/// A opacidade dos ícones inertes do addon.
pub fn icon_opacity(has_control: bool) -> f32 {
    if has_control {
        1.0
    } else {
        ICON_OPACITY
    }
}

/// Um item de addon: o elemento e o que ele é.
struct AddonItem {
    kind: AddonItemKind,
    el: AnyElement,
}

/// Uma peça pendurada na moldura do [`InputGroup`] — o `InputGroupAddon` do original.
///
/// Não tem borda, fundo, raio nem foco próprios: é conteúdo DENTRO da superfície do campo. Se você
/// precisa de um controle com moldura própria colado ao campo, o componente é o
/// [`crate::group::Group`] (ver a seção "Group não é InputGroup" no doc dele).
pub struct InputGroupAddon {
    align: AddonAlign,
    items: Vec<AddonItem>,
    divider: bool,
}

impl InputGroupAddon {
    fn with(align: AddonAlign) -> Self {
        Self {
            align,
            items: Vec::new(),
            divider: false,
        }
    }

    /// Addon colado à **esquerda** do campo, na mesma linha.
    pub fn inline_start() -> Self {
        Self::with(AddonAlign::InlineStart)
    }

    /// Addon colado à **direita** do campo, na mesma linha.
    pub fn inline_end() -> Self {
        Self::with(AddonAlign::InlineEnd)
    }

    /// Uma linha de largura cheia **acima** do campo, na mesma moldura.
    pub fn block_start() -> Self {
        Self::with(AddonAlign::BlockStart)
    }

    /// Uma linha de largura cheia **abaixo** do campo, na mesma moldura.
    pub fn block_end() -> Self {
        Self::with(AddonAlign::BlockEnd)
    }

    /// Texto inerte, na cor `--muted-foreground` — é o `InputGroupText` do original.
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        let el = div()
            .flex_none()
            .text_color(muted_fg().hsla())
            .child(text.into())
            .into_any_element();
        self.items.push(AddonItem {
            kind: AddonItemKind::Text,
            el,
        });
        self
    }

    /// Ícone inerte de 16px, na cor `--muted-foreground`.
    ///
    /// O SVG é montado aqui (e não recebido pronto) porque o tamanho, a cor e a opacidade dele são
    /// regras do addon — recebê-lo pronto deixaria cada call site repetir os três números.
    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        let el = gpui::svg()
            .path(path.into())
            .size(px(ICON_SIZE))
            .flex_none()
            .text_color(muted_fg().hsla())
            .into_any_element();
        self.items.push(AddonItem {
            kind: AddonItemKind::Icon,
            el,
        });
        self
    }

    /// Um badge inerte (ver [`crate::badge`]) — puxa 6px do respiro da borda.
    pub fn badge(mut self, badge: impl IntoElement) -> Self {
        self.items.push(AddonItem {
            kind: AddonItemKind::Badge,
            el: badge.into_any_element(),
        });
        self
    }

    /// Uma dica de atalho de teclado, inerte — puxa 5,6px do respiro da borda.
    pub fn kbd(mut self, kbd: impl IntoElement) -> Self {
        self.items.push(AddonItem {
            kind: AddonItemKind::Kbd,
            el: kbd.into_any_element(),
        });
        self
    }

    /// Conteúdo inerte genérico: clicar nele **foca o campo**, como em qualquer parte do addon.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.items.push(AddonItem {
            kind: AddonItemKind::Text,
            el: child.into_any_element(),
        });
        self
    }

    /// Um **controle interativo** (botão, select, link): clicar nele NÃO foca o campo.
    ///
    /// É a informação que o original obtém sniffando o DOM (`target.closest("button, a, …")`) e que
    /// aqui é declarada — ver a seção de clique no doc do módulo. Um controle passado por
    /// [`Self::child`] continua funcionando, mas o clique nele também moveria o cursor pro campo.
    pub fn control(mut self, control: impl IntoElement) -> Self {
        self.items.push(AddonItem {
            kind: AddonItemKind::Control,
            el: control.into_any_element(),
        });
        self
    }

    /// Liga a **divisória** entre este addon de bloco e o campo, com o respiro que o original dá
    /// junto (`[.border-t]:pt-*` / `[.border-b]:pb-*`).
    ///
    /// Só faz efeito num addon de bloco: numa linha inline, a divisória seria um traço vertical que o
    /// original não desenha.
    pub fn divider(mut self) -> Self {
        self.divider = true;
        self
    }

    /// O item que encosta na borda do grupo: o primeiro num addon de início, o último num de fim.
    fn edge_kind(&self) -> Option<AddonItemKind> {
        if self.items.iter().any(|i| i.kind == AddonItemKind::Control) {
            return Some(AddonItemKind::Control);
        }
        match self.align {
            AddonAlign::InlineEnd => self.items.last(),
            _ => self.items.first(),
        }
        .map(|i| i.kind)
    }

    /// Monta o addon. `focus` é o campo que o clique no respiro deve focar.
    fn render(self, size: InputSize, focus: gpui::FocusHandle) -> Div {
        let align = self.align;
        let base = size.pad_x();
        let lead = addon_lead_pad(size, align, self.edge_kind());
        let has_control = self
            .items
            .iter()
            .any(|i| i.kind == AddonItemKind::Control);
        let has_icon = self.items.iter().any(|i| i.kind == AddonItemKind::Icon);
        let opacity = icon_opacity(has_control);

        let mut d = div()
            .flex()
            .items_center()
            .gap(px(addon_gap(has_icon)))
            // `cursor-text` na base do `cva`: o addon faz parte do campo, não é uma barra à parte.
            .cursor(CursorStyle::IBeam)
            // Clicar no addon foca o campo. Um controle dentro dele interrompe este handler antes
            // (ver a seção de clique no doc do módulo).
            .on_mouse_down(MouseButton::Left, move |_, window, _cx| {
                // A guarda é o `!parent?.querySelector("input:focus")` do original: focar de novo um
                // campo já focado zeraria a seleção que o usuário acabou de fazer.
                if !focus.is_focused(window) {
                    focus.focus(window);
                }
            });

        d = match align {
            // `justify-center` da base do `cva`.
            AddonAlign::InlineStart => d.justify_center().flex_none().pl(px(lead)),
            AddonAlign::InlineEnd => d.justify_center().flex_none().pr(px(lead)),
            // `w-full justify-start`: a linha de bloco ocupa a moldura inteira e alinha à esquerda.
            AddonAlign::BlockStart => d
                .w_full()
                .justify_start()
                .px(px(base))
                .pt(px(base))
                .when(self.divider, |d| {
                    d.border_b_1()
                        .border_color(crate::theme::border_divider())
                        .pb(px(base))
                }),
            AddonAlign::BlockEnd => d
                .w_full()
                .justify_start()
                .px(px(base))
                .pb(px(base))
                .when(self.divider, |d| {
                    d.border_t_1()
                        .border_color(crate::theme::border_divider())
                        .pt(px(base))
                }),
        };

        for item in self.items {
            d = match item.kind {
                // Um controle é embrulhado num elemento que corta a propagação do mouse-down: é o
                // que impede o clique nele de virar "foca o campo".
                AddonItemKind::Control => d.child(
                    div()
                        .flex()
                        .items_center()
                        .flex_none()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(item.el),
                ),
                // O ícone esmaece quando o addon não tem controle.
                AddonItemKind::Icon => d.child(
                    div()
                        .flex()
                        .items_center()
                        .flex_none()
                        .opacity(opacity)
                        .child(item.el),
                ),
                _ => d.child(item.el),
            };
        }
        d
    }
}

/// Em que ordem os addons entram na árvore, resolvendo os `order-first`/`order-last` do original.
///
/// Devolve os índices em quatro faixas — bloco de início, inline de início, inline de fim, bloco de
/// fim — cada uma na ordem em que foram declarados. É função pura de propósito: a ordem é a única
/// coisa que o GPUI não faz por conta própria aqui, e é o que mais fácil se quebra sem alguém notar.
pub fn particionar(aligns: &[AddonAlign]) -> [Vec<usize>; 4] {
    let mut out = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for (i, a) in aligns.iter().enumerate() {
        let faixa = match a {
            AddonAlign::BlockStart => 0,
            AddonAlign::InlineStart => 1,
            AddonAlign::InlineEnd => 2,
            AddonAlign::BlockEnd => 3,
        };
        out[faixa].push(i);
    }
    out
}

// =================================================================================================
// Respiro do campo hospedado
// =================================================================================================

/// O respiro do campo hospedado, nos quatro lados.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FieldPad {
    /// Esquerda.
    pub start: f32,
    /// Direita.
    pub end: f32,
    /// Topo.
    pub top: f32,
    /// Base.
    pub bottom: f32,
}

/// O respiro do campo dentro do grupo.
///
/// Horizontal: o normal do campo (11, ou 9 no `Sm`) quando aquele lado está livre, e **8** (6 no `Sm`)
/// quando há addon inline ali — o addon já trouxe o respiro contra a borda, e repetir os 11 abriria um
/// vão entre o ícone e o texto.
///
/// Vertical: o original põe `pt-1.5` quando existe addon de bloco de FIM e `pb-1.5` quando existe um
/// de INÍCIO — ou seja, o respiro entra do lado **oposto** ao addon, que é justamente o lado onde o
/// campo encosta na borda do grupo sem nada no meio.
pub fn field_pad(
    size: InputSize,
    has_inline_start: bool,
    has_inline_end: bool,
    has_block_start: bool,
    has_block_end: bool,
) -> FieldPad {
    let junto = if size == InputSize::Sm {
        FIELD_INLINE_PAD_SM
    } else {
        FIELD_INLINE_PAD
    };
    FieldPad {
        start: if has_inline_start { junto } else { size.pad_x() },
        end: if has_inline_end { junto } else { size.pad_x() },
        top: if has_block_end { FIELD_BLOCK_PAD } else { 0.0 },
        bottom: if has_block_start { FIELD_BLOCK_PAD } else { 0.0 },
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// A superfície do campo com addons dentro. Ver o doc do módulo.
///
/// É um **elemento de render** (não uma entidade), como o [`crate::Input`]: construa a cada frame
/// apontando pro [`InputState`] que guarda o texto.
#[derive(IntoElement)]
pub struct InputGroup {
    state: Entity<InputState>,
    size: InputSize,
    disabled: bool,
    invalid: bool,
    rows: Option<usize>,
    addons: Vec<InputGroupAddon>,
}

impl InputGroup {
    /// Grupo com um campo de **uma linha** dentro.
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            size: InputSize::default(),
            disabled: false,
            invalid: false,
            rows: None,
            addons: Vec::new(),
        }
    }

    /// Grupo com uma **textarea** de `rows` linhas dentro.
    ///
    /// Use o MESMO `rows` do [`crate::input::multi_line`] que criou o estado — é a mesma armadilha
    /// que o [`crate::Input::rows`] documenta.
    pub fn textarea(state: &Entity<InputState>, rows: usize) -> Self {
        let mut this = Self::new(state);
        this.rows = Some(rows);
        this
    }

    /// Tamanho do grupo. Vale pra moldura, pro campo e pro respiro dos addons de uma vez — é o
    /// `data-size` do original, que os addons leem por seletor de irmão.
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Desabilita o conjunto: o campo não recebe foco nem edição, e a moldura toda esmaece.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marca a moldura como **inválida** (`aria-invalid`): a borda vira `--destructive` e o anel de
    /// foco vem na cor de erro.
    ///
    /// É um `bool` e não um [`crate::input::Validity`] de propósito — ver o doc do módulo.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Pendura um addon. A posição vem do próprio addon; a ordem de chamada só desempata addons na
    /// mesma posição.
    pub fn addon(mut self, addon: InputGroupAddon) -> Self {
        self.addons.push(addon);
        self
    }
}

impl RenderOnce for InputGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let s = self.size;
        let focus_handle = self.state.read(cx).focus_handle(cx);
        let focused = focus_handle.is_focused(window);

        let aligns: Vec<AddonAlign> = self.addons.iter().map(|a| a.align).collect();
        let [bloco_inicio, inline_inicio, inline_fim, bloco_fim] = particionar(&aligns);
        let coluna = !bloco_inicio.is_empty() || !bloco_fim.is_empty();
        let pad = field_pad(
            s,
            !inline_inicio.is_empty(),
            !inline_fim.is_empty(),
            !bloco_inicio.is_empty(),
            !bloco_fim.is_empty(),
        );

        // Os addons são consumidos por índice, então viram `Option` pra poder sair do vetor sem
        // reordenar nada.
        let mut addons: Vec<Option<InputGroupAddon>> = self.addons.into_iter().map(Some).collect();
        let mut render_faixa = |indices: &[usize]| -> Vec<Div> {
            indices
                .iter()
                .filter_map(|&i| addons[i].take())
                .map(|a| a.render(s, focus_handle.clone()))
                .collect()
        };
        let el_bloco_inicio = render_faixa(&bloco_inicio);
        let el_inline_inicio = render_faixa(&inline_inicio);
        let el_inline_fim = render_faixa(&inline_fim);
        let el_bloco_fim = render_faixa(&bloco_fim);

        // --- O campo hospedado, sem moldura ---------------------------------------------------
        //
        // `pad_x(0)`: o respiro horizontal passa a ser do wrapper, porque no grupo ele é ASSIMÉTRICO
        // (encolhe só do lado onde há addon) e o `pad_x` do campo é um valor só pros dois lados.
        // É também o que o `*:[[data-slot=input-control]]:contents` do original consegue por outro
        // caminho — ver "Ausente" no doc do módulo.
        //
        // A validade NÃO desce pro campo: quem desenha a borda de erro aqui é o grupo, e um
        // `Validity::Error` no campo abriria a linha de mensagem dele DENTRO da moldura.
        let mut campo = Input::new(&self.state)
            .unstyled()
            .size(s)
            .pad_x(0.0)
            .disabled(self.disabled);
        campo = match self.rows {
            Some(rows) => campo.rows(rows),
            // A altura do campo é o MIOLO do grupo: a moldura (e as duas bordas de 1px que somam os
            // 32px do `Md`) é do grupo agora, então o campo mede `content_height`, não `height`.
            None => campo.height(s.content_height()),
        };

        let mut caixa_campo = div()
            .flex_1()
            .min_w(px(0.0))
            .pl(px(pad.start))
            .pr(px(pad.end))
            .when(pad.top > 0.0, |d| d.pt(px(pad.top)))
            .when(pad.bottom > 0.0, |d| d.pb(px(pad.bottom)));
        // O que falta pros 11px de respiro vertical da textarea no grupo.
        if self.rows.is_some() {
            let extra = textarea_pad_y_complement(self.size);
            caixa_campo = caixa_campo.pt(px(pad.top + extra)).pb(px(pad.bottom + extra));
        }
        let caixa_campo = caixa_campo.child(campo);

        // --- A moldura ------------------------------------------------------------------------
        //
        // Toda a superfície vem do `crate::input`: cor de borda, fundo, sombra, bisel, anel e raio.
        // Ver o doc do módulo — nada disso é reescrito aqui.
        let mut frame = div()
            .relative()
            .flex()
            .w_full()
            .overflow_hidden()
            .cursor(CursorStyle::IBeam)
            .text_size(px(TEXT_SIZE))
            // ⚠️ Sem isto o GPUI usaria `relative(1.618034)` — ver [`TEXT_LINE_HEIGHT`].
            .line_height(px(TEXT_LINE_HEIGHT))
            .bg(input::field().bg.hsla())
            .border_1()
            .border_color(input::border_color_for(self.invalid, focused).hsla())
            .rounded(px(frame_radius()));

        if coluna {
            // `has-data-[align=block-*]:flex-col` + `has-data-[align=block-*]:h-auto`: a altura vem
            // do conteúdo (não se declara altura nenhuma). O alinhamento cruzado fica no default
            // (esticar), que é o que dá largura cheia às linhas sem cada uma pedir.
            frame = frame.flex_col();
        } else if self.rows.is_some() {
            // `has-[textarea]:h-auto`: quem manda na altura é o `rows` do campo hospedado.
            frame = frame.items_start();
        } else {
            // Sem addon de bloco e sem textarea, a moldura é uma linha da altura do tamanho (32 no
            // `Md`), com o miolo de 30 dentro das duas bordas.
            frame = frame.items_center().h(px(s.height()));
        }

        frame = frame.children(el_bloco_inicio);
        if coluna {
            // A linha do meio é um container próprio — é o que faz misturar bloco com inline
            // funcionar (ver "Superset consciente" no doc do módulo).
            frame = frame.child(
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .children(el_inline_inicio)
                    .child(caixa_campo)
                    .children(el_inline_fim),
            );
        } else {
            frame = frame
                .children(el_inline_inicio)
                .child(caixa_campo)
                .children(el_inline_fim);
        }
        frame = frame.children(el_bloco_fim);

        frame = frame.shadow(input::shadow_stack_for(
            self.disabled,
            self.invalid,
            0.0,
            if focused { 0.0 } else { 1.0 },
        ));

        // --- Bisel e anel, irmãos da moldura --------------------------------------------------
        //
        // Irmãos e não filhos pelo mesmo motivo do campo solto: a moldura tem `overflow_hidden` e
        // recortaria um overlay que cobre a borda ou cresce 3px pra fora (armadilha nº 6 da casa).
        // ⚠️ O campo hospedado chama `focus_transition` com o MESMO `EntityId` no mesmo frame (é o
        // cross-fade do anel dele). Não é conflito: a primeira chamada registra a virada e devolve
        // `Some`, a segunda cai no ramo "ainda dentro da janela" e devolve `Some` também — as duas
        // leem o mesmo estado. E o anel do campo não entra na árvore, porque ele é `unstyled`.
        let transicao = input::focus_transition(self.state.entity_id(), focused);
        let mostra_anel = input::ring_visible(self.disabled, focused, transicao == Some(false));

        let mut wrap = div()
            .relative()
            .w_full()
            // `has-disabled:opacity-64` — o original esmaece o CONJUNTO. O campo hospedado não
            // esmaece a si mesmo (é o que [`crate::Input::unstyled`] desliga), então não há
            // multiplicação de opacidade.
            .when(self.disabled, |d| d.opacity(0.64))
            .on_mouse_down_out(input::blur_on_outside_click(focus_handle.clone()))
            .child(frame);

        if let Some(bisel) = input::bevel_for(self.disabled, focused, self.invalid, Join::NONE) {
            wrap = wrap.child(bisel);
        }
        if mostra_anel {
            let anel = input::ring_overlay(self.invalid, Join::NONE);
            wrap = match transicao {
                None => wrap.child(anel),
                Some(entrando) => wrap.child(gpui::AnimationExt::with_animation(
                    anel,
                    gpui::ElementId::NamedInteger(
                        if entrando {
                            "group-ring-in".into()
                        } else {
                            "group-ring-out".into()
                        },
                        self.state.entity_id().as_u64(),
                    ),
                    gpui::Animation::new(input::FOCUS_TRANSITION).with_easing(gpui::ease_in_out),
                    move |el, delta| el.opacity(if entrando { delta } else { 1.0 - delta }),
                )),
            };
        }
        wrap
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o número que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `input.rs`, `frame.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Os respiros do addon são **11 e 9** — `calc(--spacing(3)-1px)` e
    /// `calc(--spacing(2.5)-1px)` — e vêm do MESMO lugar que o respiro do campo, porque no original é
    /// literalmente a mesma expressão. Se alguém escrever um 11 novo aqui, este teste continua
    /// passando; o que ele trava é o número.
    #[test]
    fn respiro_do_addon_e_11_e_9_por_tamanho() {
        let sem_puxao = Some(AddonItemKind::Text);
        for align in [AddonAlign::InlineStart, AddonAlign::InlineEnd] {
            assert_eq!(addon_lead_pad(InputSize::Md, align, sem_puxao), 11.0);
            assert_eq!(addon_lead_pad(InputSize::Lg, align, sem_puxao), 11.0);
            assert_eq!(addon_lead_pad(InputSize::Sm, align, sem_puxao), 9.0);
        }
        // A base é a do campo, não uma constante nova.
        assert_eq!(InputSize::Md.pad_x(), 11.0);
        assert_eq!(InputSize::Sm.pad_x(), 9.0);
    }

    /// **Os puxões do original entram como geometria.** Cada `-ms-*` sai do respiro; nenhum resultado
    /// é negativo, porque margem negativa é proibida nesta base.
    #[test]
    fn puxoes_saem_do_respiro_e_nunca_ficam_negativos() {
        assert_eq!(pull(AddonItemKind::Text), 0.0);
        assert_eq!(pull(AddonItemKind::Icon), 2.0, "-mx-0.5");
        assert_eq!(pull(AddonItemKind::Badge), 6.0, "-ms-1.5");
        assert_eq!(pull(AddonItemKind::Kbd), 5.6, "ms-[-0.35rem] = 0,35 × 16");
        assert_eq!(pull(AddonItemKind::Control), 8.0, "-ms-2");

        // Md: 11 menos o puxão.
        let md = |k| addon_lead_pad(InputSize::Md, AddonAlign::InlineStart, Some(k));
        assert_eq!(md(AddonItemKind::Text), 11.0);
        assert_eq!(md(AddonItemKind::Icon), 9.0);
        assert_eq!(md(AddonItemKind::Badge), 5.0);
        assert!((md(AddonItemKind::Kbd) - 5.4).abs() < 1e-6);
        assert_eq!(md(AddonItemKind::Control), 3.0);

        // Sm: 9 menos o puxão — o caso mais apertado de todos, e ainda positivo.
        let sm = |k| addon_lead_pad(InputSize::Sm, AddonAlign::InlineEnd, Some(k));
        assert_eq!(sm(AddonItemKind::Control), 1.0);
        for k in [
            AddonItemKind::Text,
            AddonItemKind::Icon,
            AddonItemKind::Badge,
            AddonItemKind::Kbd,
            AddonItemKind::Control,
        ] {
            assert!(sm(k) > 0.0, "{k:?}: respiro negativo é proibido nesta base");
        }
    }

    /// Addon de **bloco** não tem puxão nenhum: o `cva` do original não declara `-ms-*`/`-me-*` pras
    /// variantes de bloco, e ali o respiro é o da linha inteira.
    #[test]
    fn addon_de_bloco_nao_tem_puxao() {
        for align in [AddonAlign::BlockStart, AddonAlign::BlockEnd] {
            for k in [
                AddonItemKind::Text,
                AddonItemKind::Icon,
                AddonItemKind::Badge,
                AddonItemKind::Kbd,
                AddonItemKind::Control,
            ] {
                assert_eq!(
                    addon_lead_pad(InputSize::Md, align, Some(k)),
                    InputSize::Md.pad_x(),
                    "{align:?} + {k:?}: bloco mantém o respiro cheio"
                );
            }
        }
        assert!(AddonAlign::BlockStart.is_block() && AddonAlign::BlockEnd.is_block());
        assert!(!AddonAlign::InlineStart.is_block() && !AddonAlign::InlineEnd.is_block());
    }

    /// O vão entre itens é **8**, e encolhe pro puxão do ícone quando há ícone — o mesmo tratamento
    /// que o [`crate::group::GroupText`] dá ao `-mx-0.5` dele.
    #[test]
    fn vao_do_addon_encolhe_com_icone() {
        assert_eq!(addon_gap(false), 8.0, "gap-2");
        assert_eq!(addon_gap(true), 6.0, "8 − 2 do -mx-0.5");
        assert_eq!(addon_gap(true), ADDON_GAP - pull(AddonItemKind::Icon));
    }

    /// **A armadilha da entrelinha.** `text-sm` no Tailwind é o par 14/20; o default do GPUI é
    /// `relative(1.618034)`, que daria 22,65 e empurraria cada linha de addon 2,65px pra baixo.
    #[test]
    fn entrelinha_do_grupo_e_o_par_do_tailwind() {
        assert_eq!(TEXT_SIZE, 14.0, "sm:text-sm");
        assert_eq!(TEXT_LINE_HEIGHT, 20.0, "o par de text-sm é 14/20");

        // O que sairia sem fixar: o default do GPUI.
        const GPUI_DEFAULT: f32 = 1.618_034;
        let solto = TEXT_SIZE * GPUI_DEFAULT;
        assert!(
            (solto - 22.65).abs() < 0.01,
            "sem fixar, a entrelinha seria {solto}"
        );
        assert!(
            solto > TEXT_LINE_HEIGHT + 2.0,
            "a diferença é grande o bastante pra ser vista: {solto} contra {TEXT_LINE_HEIGHT}"
        );
    }

    /// O ícone do addon é **16px** (o `sm:size-4`, não o `size-4.5` da classe base) e esmaece a 80%
    /// só quando o addon **não** tem controle.
    #[test]
    fn icone_do_addon_e_16px_e_esmaece_sem_controle() {
        assert_eq!(ICON_SIZE, 16.0, "sm:size-4; a base 4.5/18 não vale no desktop");
        assert_eq!(icon_opacity(false), 0.8, "not-has-[button]:…:opacity-80");
        assert_eq!(icon_opacity(true), 1.0, "com controle, o ícone fica cheio");
    }

    /// O respiro do campo **encolhe** do lado onde há addon inline (11/9 → 8/6) e fica cheio do lado
    /// livre. Sem isso o texto ficaria 11px longe do ícone.
    #[test]
    fn respiro_do_campo_encolhe_do_lado_com_addon() {
        let so_inicio = field_pad(InputSize::Md, true, false, false, false);
        assert_eq!(so_inicio.start, 8.0, "ps-2");
        assert_eq!(so_inicio.end, 11.0, "lado livre mantém o respiro do campo");

        let so_fim = field_pad(InputSize::Md, false, true, false, false);
        assert_eq!(so_fim.start, 11.0);
        assert_eq!(so_fim.end, 8.0, "pe-2");

        let sm = field_pad(InputSize::Sm, true, true, false, false);
        assert_eq!((sm.start, sm.end), (6.0, 6.0), "[data-size=sm] ps-1.5/pe-1.5");

        let livre = field_pad(InputSize::Lg, false, false, false, false);
        assert_eq!((livre.start, livre.end), (11.0, 11.0));
    }

    /// O respiro vertical do campo entra do lado **oposto** ao addon de bloco: é o lado em que o campo
    /// encosta na borda do grupo sem nada no meio. Contra-intuitivo lendo os seletores do original
    /// (`block-end` → `pt-1.5`), então fica travado aqui.
    #[test]
    fn respiro_vertical_do_campo_fica_no_lado_sem_addon() {
        let acima = field_pad(InputSize::Md, false, false, true, false);
        assert_eq!(acima.bottom, 6.0, "addon em cima → respiro embaixo (pb-1.5)");
        assert_eq!(acima.top, 0.0, "em cima quem respira é o addon (pt-11)");

        let abaixo = field_pad(InputSize::Md, false, false, false, true);
        assert_eq!(abaixo.top, 6.0, "addon embaixo → respiro em cima (pt-1.5)");
        assert_eq!(abaixo.bottom, 0.0);

        // Com os dois, o original aplica as duas regras.
        let ambos = field_pad(InputSize::Md, false, false, true, true);
        assert_eq!((ambos.top, ambos.bottom), (6.0, 6.0));

        // Sem addon de bloco não há respiro vertical: a altura do campo já centraliza o texto.
        let nenhum = field_pad(InputSize::Md, true, true, false, false);
        assert_eq!((nenhum.top, nenhum.bottom), (0.0, 0.0));
    }

    /// A ordem dos addons é EXPLÍCITA (o GPUI não tem `order`): bloco de início, inline de início,
    /// inline de fim, bloco de fim — cada faixa na ordem de declaração.
    #[test]
    fn ordem_dos_addons_e_explicita() {
        let aligns = [
            AddonAlign::BlockEnd,     // 0
            AddonAlign::InlineEnd,    // 1
            AddonAlign::BlockStart,   // 2
            AddonAlign::InlineStart,  // 3
            AddonAlign::InlineStart,  // 4
        ];
        let [bi, ii, ifim, bf] = particionar(&aligns);
        assert_eq!(bi, vec![2], "order-first do bloco");
        assert_eq!(ii, vec![3, 4], "dois inline-start, na ordem de declaração");
        assert_eq!(ifim, vec![1]);
        assert_eq!(bf, vec![0], "order-last do bloco");

        // Sem addon nenhum, nada em faixa nenhuma — e o grupo continua sendo uma linha.
        let [a, b, c, d] = particionar(&[]);
        assert!(a.is_empty() && b.is_empty() && c.is_empty() && d.is_empty());

        // Todo índice aparece EXATAMENTE uma vez: um addon perdido na partição sumiria da tela sem
        // nenhum erro.
        let [a, b, c, d] = particionar(&aligns);
        let mut todos: Vec<usize> = [a, b, c, d].concat();
        todos.sort_unstable();
        assert_eq!(todos, (0..aligns.len()).collect::<Vec<_>>());
    }

    /// Só um addon de **bloco** põe o grupo em coluna — é o `has-data-[align=block-*]:flex-col`.
    #[test]
    fn addon_de_bloco_poe_o_grupo_em_coluna() {
        let coluna = |aligns: &[AddonAlign]| {
            let [bi, _, _, bf] = particionar(aligns);
            !bi.is_empty() || !bf.is_empty()
        };
        assert!(!coluna(&[]));
        assert!(!coluna(&[AddonAlign::InlineStart, AddonAlign::InlineEnd]));
        assert!(coluna(&[AddonAlign::BlockStart]));
        assert!(coluna(&[AddonAlign::BlockEnd]));
        assert!(coluna(&[AddonAlign::InlineStart, AddonAlign::BlockEnd]));
    }

    /// O respiro vertical da textarea no grupo é **11** — os 6 que o campo já traz mais o complemento
    /// que o grupo acrescenta. O complemento é DERIVADO: mexer no respiro do campo solto não descola o
    /// do grupo, e nunca vira respiro negativo.
    #[test]
    fn respiro_vertical_da_textarea_no_grupo_fecha_em_11() {
        assert_eq!(TEXTAREA_PAD_Y, 11.0, "**:[textarea]:py-[calc(--spacing(3)-1px)]");
        // ⚠️ A soma tem que bater nos TRÊS tamanhos: a base do campo varia (3/5/7, os `py` que a
        // referência declara por tamanho) e o complemento varia com ela (8/6/4). Um `assert` num
        // tamanho só passaria mesmo se a função ignorasse o argumento — foi assim que este teste
        // nasceu, contra uma base única de 6px que o porte do Textarea substituiu.
        for size in [InputSize::Sm, InputSize::Md, InputSize::Lg] {
            let base = size.textarea_pad_y();
            let extra = textarea_pad_y_complement(size);
            assert_eq!(
                base + extra,
                TEXTAREA_PAD_Y,
                "{size:?}: base {base} + complemento {extra} não fecha {TEXTAREA_PAD_Y}"
            );
            assert!(extra >= 0.0, "{size:?}: respiro negativo é proibido nesta base");
        }
        // E os três complementos são DIFERENTES entre si — se fossem iguais, a função estaria
        // ignorando o tamanho e a soma só fecharia por coincidência num deles.
        let (a, b, c) = (
            textarea_pad_y_complement(InputSize::Sm),
            textarea_pad_y_complement(InputSize::Md),
            textarea_pad_y_complement(InputSize::Lg),
        );
        assert!(a != b && b != c, "complementos {a}/{b}/{c} não acompanham o tamanho");
    }

    /// **A superfície é a do campo, não uma cópia dela.** Nem cor nem raio novos: o raio é o
    /// `FIELD_RADIUS` do campo, a cor da borda é a mesma função de precedência, e o
    /// `--muted-foreground` do addon é o token do placeholder com o alfa cheio.
    #[test]
    fn superficie_e_a_do_campo_sem_copia() {
        crate::theme::set_theme(crate::theme::ThemeMode::Light);

        // Raio: `rounded-lg` = 10, e o do grupo É o do campo — não uma cópia com o mesmo número.
        assert_eq!(input::FIELD_RADIUS, 10.0);
        assert_eq!(
            frame_radius(),
            input::FIELD_RADIUS,
            "o raio do grupo tem que VIR do campo; um 10 próprio aqui é a segunda fonte de verdade"
        );

        // A cor da borda segue a MESMA precedência do campo (inválido+foco > inválido > foco >
        // repouso), porque é a mesma função — os quatro estados dão quatro cores diferentes.
        let f = input::field();
        let cores = [
            input::border_color_for(false, false),
            input::border_color_for(false, true),
            input::border_color_for(true, false),
            input::border_color_for(true, true),
        ];
        for (i, a) in cores.iter().enumerate() {
            for b in &cores[i + 1..] {
                assert_ne!(a, b, "os quatro estados da borda do grupo são distintos");
            }
        }

        // O texto do addon é o `--muted-foreground` CHEIO, e o placeholder é o MESMO token a 72%:
        // mesmos canais de cor, alfas diferentes. Se alguém escrever um `#686868` novo aqui, os
        // canais podem divergir sem ninguém notar.
        let texto: gpui::Rgba = muted_fg().hsla().into();
        let ph: gpui::Rgba = f.placeholder.hsla().into();
        assert_eq!(texto.a, 1.0, "o texto do addon é opaco");
        assert!(ph.a < 1.0, "o placeholder é o mesmo token a 72%");
        for (nome, a, b) in [("r", texto.r, ph.r), ("g", texto.g, ph.g), ("b", texto.b, ph.b)] {
            assert!(
                (a - b).abs() < 1e-6,
                "canal {nome}: o texto do addon TEM que ser o mesmo token do placeholder ({a} vs {b})"
            );
        }
        // E decodifica pro cinza que o coss resolve no tema claro (#686868), não pra um ciano —
        // a assinatura do bug histórico de convenção de cor.
        assert!(texto.r > 0.1, "canal vermelho presente");
        assert!(
            (texto.r - texto.g).abs() < 0.01 && (texto.g - texto.b).abs() < 0.01,
            "--muted-foreground é NEUTRO"
        );
    }

    /// A altura da moldura é a do tamanho (32 no `Md`) e o campo hospedado mede o MIOLO (30): as duas
    /// bordas de 1px agora são do grupo. Se o campo mantivesse a altura externa, o grupo sairia 2px
    /// alto em todo tamanho.
    #[test]
    fn moldura_mede_a_altura_externa_e_o_campo_o_miolo() {
        for t in [InputSize::Sm, InputSize::Md, InputSize::Lg] {
            assert_eq!(t.height(), t.content_height() + 2.0);
        }
        assert_eq!(InputSize::Md.height(), 32.0);
        assert_eq!(InputSize::Md.content_height(), 30.0);
    }
}

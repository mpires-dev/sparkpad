//! `Kbd` — a **tecla** do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `packages/ui/src/components/kbd.tsx`
//!
//! # O que é
//!
//! A pastilha que mostra um atalho de teclado: o `⌘` de "⌘K pra abrir a busca", o `Esc` do rodapé
//! de um diálogo, o `⇧` de uma dica de ferramenta. É o componente mais simples do catálogo — uma
//! caixa de estilo com fundo `--muted`, sem variante, sem tamanho, sem estado e **sem interação**.
//!
//! O original são duas funções: a tecla ([`Kbd`]) e o agrupador ([`KbdGroup`]), que só existe pra
//! pôr `gap-1` entre várias teclas.
//!
//! ```text
//!   ┌──────┐ ┌──────┐        ┌──────────┐
//!   │  ⌘   │ │  K   │        │ ◇  Enter │
//!   └──────┘ └──────┘        └──────────┘
//!     20px  4px  20px          ← .icon_before(…) · rótulo
//!            ▲ o `gap-1` do KbdGroup
//! ```
//!
//! # Uso
//!
//! ```ignore
//! // Uma tecla só.
//! kbd::Kbd::new("Esc")
//!
//! // Um atalho: o agrupador aceita filhos livres.
//! kbd::KbdGroup::new()
//!     .child(kbd::Kbd::new("⌘"))
//!     .child(kbd::Kbd::new("K"))
//!
//! // Tecla de ícone só (nasce quadrada, 20×20).
//! kbd::Kbd::icon("iconoir/regular/arrow-up.svg")
//!
//! // Ícone + rótulo, separados pelo `gap-1` interno.
//! kbd::Kbd::new("Enter").icon_before("iconoir/regular/enter-key.svg")
//! ```
//!
//! # O `kbd.rs` do fork NÃO serve — veredito
//!
//! ⚠️ `vendor/gpui-component/src/kbd.rs` existe e tem um tipo chamado `Kbd`. **Não é este
//! componente.** Ele é um formatador de [`gpui::Keystroke`] casado com uma pastilha cujo contrato
//! visual diverge em quase todo número:
//!
//! | | `vendor/gpui-component` | `kbd.tsx` (daqui) |
//! |---|---|---|
//! | altura | `py-0.5` — dirigida pelo conteúdo | `h-5` = **20px fixos** |
//! | piso de largura | `min_w_5` = 20px | `min-w-5` = 20px ✓ |
//! | respiro | `px_1` = 4px | `px-1` = 4px ✓ |
//! | fundo | `--background` (superfície opaca) | `--muted` (véu de 4%) |
//! | borda | `border_1` em `--border` | **nenhuma** |
//! | raio | `rounded_sm` = `--radius-sm` = 6px | `rounded-[.25rem]` = **4px** |
//! | entrelinha | `relative(1.0)` = 12px | par do `text-xs` = **16px** |
//! | peso | herdado | `font-medium` = **500** |
//! | ícone / gap | não tem | `size-3` / `gap-1` |
//! | quebra de linha | `whitespace_normal` | nenhuma (é `inline-flex`) |
//! | cor do texto | `muted_foreground` ✓ | `--muted-foreground` ✓ |
//!
//! Coincidem **três** valores de onze. Reusar aquilo seria trocar o pixel-perfect por um vizinho
//! parecido: fundo de superfície com borda no lugar do véu sem borda, raio 6 no lugar de 4, e o
//! texto com entrelinha 12 num miolo de 20. Além disso ele é `Styled`, lê `cx.theme()` do crate
//! vendorizado em vez do nosso [`crate::theme`], e é justamente pela porta do `Styled` que cada
//! call site remendava o tamanho por conta própria.
//!
//! **Serve de referência pra uma coisa só**, e ela não faz parte deste port: a tabela de glifos de
//! plataforma do `Kbd::format` (`⌘⌥⇧⌃`, `⌫`, `⎋`, `Ctrl+…` no Windows). O `kbd.tsx` não formata
//! tecla nenhuma — ele recebe filhos e não sabe o que é um atalho. Ver "Ausente", abaixo.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`inline-flex`** — no DOM o `<kbd>` é inline-LEVEL: ele senta no meio de uma frase ("aperte
//!   ⌘K pra abrir"). O GPUI não tem contexto de formatação inline; toda caixa é de bloco. Uma
//!   tecla "dentro de um texto" aqui é uma linha flex com o texto quebrado em volta dela. É a
//!   única diferença deste módulo que muda o que dá pra montar, e não só como fica.
//! - **`pointer-events-none`** — um `div` do GPUI sem `.id()` e sem nenhum listener não recebe
//!   ponteiro. A classe é redundante aqui, não omitida. (É o que garante que a tecla desenhada
//!   sobre um item de menu não engula o clique do item.)
//! - **`select-none`** — não existe seleção de texto num `div` do GPUI; só o [`crate::input`] tem
//!   seleção. Redundante.
//! - **`font-sans`** — no DOM isso é um **reset**: o UA stylesheet dá `font-family: monospace` ao
//!   `<kbd>`, e o original o traz de volta pra fonte do corpo. Sem UA stylesheet não há o que
//!   resetar — e é por isso que aqui **não** existe nenhuma família monoespaçada. Nenhum
//!   componente deste crate declara família (ver o aviso em [`crate::theme::FONT_INTER`]), então a
//!   tecla sai na fonte da janela, que é exatamente o que o `font-sans` pede.
//! - **`data-slot="kbd"` / `data-slot="kbd-group"`** — atributo de DOM, usado no original pra
//!   estilizar de fora. Sem DOM, sem atributo.
//! - **`[class*='size-']` no seletor do ícone** — o `:not(…)` existe pra o call site poder trocar o
//!   lado do ícone por classe. Sem classes, o lado é o do slot e ponto.
//!
//! **Resolvido em número** (`--spacing` = 4px; sem nenhum ramo `sm:` — o `kbd.tsx` é uma classe
//! base só, diferente do `badge.tsx`, onde tudo tem par de breakpoint)
//!
//! - `h-5` e `min-w-5` = 5 × 4 = **20px** → [`SIDE`].
//! - `px-1` = 1 × 4 = **4px** de cada lado → [`PAD_X`].
//! - `gap-1` = 1 × 4 = **4px** → [`GAP`] (serve os DOIS `gap-1` do original: o da tecla e o do
//!   agrupador).
//! - `rounded-[.25rem]` = 0,25 × 16 = **4px** → [`RADIUS`]. ⚠️ **Não** é `--radius-sm` (6px): é
//!   valor arbitrário, o mesmo caso do `BadgeSize::Sm`.
//! - `text-xs` = 0,75rem = **12px**, e o par de entrelinha que o Tailwind v4 anexa a ele é
//!   `calc(1 / 0.75)` = 1,333… → 12 × 4/3 = **16px** (= `1rem`) → [`TEXT_SIZE`] e [`LINE_HEIGHT`].
//! - `size-3` = 3 × 4 = **12px** → [`ICON_SIZE`].
//! - `font-medium` = `--font-weight-medium` = **500** → [`gpui::FontWeight::MEDIUM`].
//! - `bg-muted` = `--alpha(black / 4%)` no claro e `--alpha(white / 4%)` no escuro = `0x0000000a` /
//!   `0xffffff0a` (4% × 255 = 10,2 → 10 = `0x0a`).
//! - `text-muted-foreground` = `mix(neutral-500 90%, black)` = `#686868` no claro e
//!   `mix(neutral-500 90%, white)` = `#818181` no escuro. Os mesmos valores que o
//!   [`crate::tabs`], o [`crate::dialog`] e o [`crate::empty`] já resolveram.
//! - **Nenhum modificador `/N` neste componente.** O `--muted` já nasce com alfa e nada o
//!   multiplica, então `Rgba8::scaled` (ver [`crate::color`]) não aparece aqui — é o mesmo caso do
//!   [`crate::avatar`].
//!
//! **Desvio consciente**
//!
//! - **Filhos arbitrários → slots explícitos.** O original é `React.ComponentProps<"kbd">`: aceita
//!   qualquer coisa dentro. Aqui a tecla tem [`Kbd::new`] (rótulo), [`Kbd::icon`],
//!   [`Kbd::icon_before`] e [`Kbd::icon_after`], pelo mesmo motivo do [`crate::badge`]: a regra
//!   `[&_svg…]:size-3` é uma regra sobre DESCENDENTES, e sem CSS quem tem que aplicar os 12px é o
//!   componente. Um `ParentElement` aqui deixaria todo ícone sair no tamanho errado, em silêncio.
//! - **Sem `Styled`/`className`.** O original aceita sobrescrita de classe; não expor isso é
//!   deliberado — era por essa porta que o `Kbd` do fork era remendado caso a caso.
//! - **O [`KbdGroup`] MANTÉM filhos livres** ([`gpui::ParentElement`]): ali o original não estiliza
//!   descendente nenhum (`inline-flex items-center gap-1` e nada mais), então não há nada pro
//!   componente aplicar e nada a perder.
//!
//! **Superset consciente**
//!
//! - Nenhum. Toda a API pública daqui aceita **menos** do que o original, não mais.
//!
//! **Ausente**
//!
//! - **O formatador de atalho.** Não existe `Kbd::from(Keystroke)` aqui, porque não existe no
//!   `kbd.tsx`: no coss quem sabe que `cmd-k` se escreve `⌘K` é o call site. Quem tem a tabela de
//!   glifos é `vendor/gpui-component/src/kbd.rs`, e ela continua lá — se uma app precisar, o
//!   caminho é `Kbd::new(gpui_component::Kbd::format(&stroke))`, e **não** re-derivar a tabela
//!   (`⌫` pra `delete` no macOS, a ordem `⌃⌥⇧⌘`, `Page Down` com espaço, o `+` como divisor no
//!   Windows — é fácil errar meia dúzia desses de cabeça).
//! - **Variante, tamanho, estado.** Não há `cva` no original: nenhuma variante, nenhum tamanho,
//!   nenhum `hover:`, `active:`, `disabled:` ou `focus-visible:` — e portanto **nenhum anel de
//!   foco**. A tecla é decorativa (`pointer-events-none`) e não é focável.
//! - **Sombra, realce interno e bisel.** A classe base não tem **nenhum** utilitário de sombra —
//!   nem `shadow-xs`, nem `inset-shadow-*`, nem `before:shadow-*`. É a **mesma decisão já declarada
//!   no [`crate::badge`]**, e a consequência é a mesma: como não há fio de bisel, não há a calibração
//!   de "branco a 6% vs. o dobro no escuro" pra fazer aqui. Se alguém acrescentar um bisel por
//!   analogia com o [`crate::input`] ou o [`crate::card`], saiu do original.
//! - **Borda.** Também não há. É por isso que o respiro é `px-1` **inteiro** (4px), e não o
//!   `px-[calc(--spacing(1)-1px)]` = 3px do [`crate::badge`]: lá o -1px desconta a borda pra o
//!   recuo total cair na escala de spacing; aqui não há borda a descontar, e os 20px de [`SIDE`]
//!   são 20px de miolo.
//! - **`opacity-80` no ícone.** ⚠️ O [`crate::badge`] tem
//!   `[&_svg:not([class*='opacity-'])]:opacity-80`; o `kbd.tsx` **não**. O ícone da tecla sai na
//!   força cheia do `--muted-foreground`. Copiar o alívio do selo por analogia deixaria a seta de
//!   um `⇧` mais apagada que o `K` ao lado dela.
//!
//! # Sem cobertura de teste — declarado
//!
//! A divisão é limpa e vale enunciá-la, porque ela decide o que um teste daqui pode prometer: **o
//! que vira LAYOUT é medido na janela; o que vira só ESTILO PINTADO não é medido em lugar nenhum.**
//!
//! - **Coberto na janela** (o layout): [`SIDE`] como altura e como piso de largura, [`PAD_X`],
//!   [`GAP`] nos dois lugares (dentro da tecla e no [`KbdGroup`]), [`ICON_SIZE`] e o `items-center`
//!   do agrupador. Apagar qualquer uma dessas chamadas do `render` faz um teste de janela falhar —
//!   conferido por mutação, uma a uma.
//! - **Só travado como constante** (o estilo pintado): [`RADIUS`], [`TEXT_SIZE`], [`LINE_HEIGHT`],
//!   o `font-medium` e as duas cores. Os testes puros garantem que o NÚMERO é o da referência, mas
//!   **nada garante que a chamada correspondente está no `render`** — o GPUI não expõe o pixel que
//!   pintou, nem o raio da quina, nem a métrica do texto. Conferido por mutação: apagar
//!   `.rounded(…)`, `.text_size(…)`, `.line_height(…)`, `.font_weight(…)` ou `.bg(…)` do `render`
//!   **não** faz nenhum teste falhar. Não há como dar dente a isso sem captura de tela, e ela não
//!   existe nesta base.
//! - A entrelinha é o caso mais desconfortável dessa segunda lista, porque a caixa tem altura
//!   **fixa** (`h-5`): diferente do [`crate::empty`], nem a altura medida denuncia uma entrelinha
//!   solta. Por isso o número vem com o contrafactual da razão de ouro escrito dentro do teste
//!   (`tests::entrelinha_e_geometria_nesta_caixa`) — é o mais perto de um dente que dá pra chegar
//!   aqui.
//! - **`pointer-events-none`.** Não há listener pendurado, então não há evento a simular que
//!   provasse a ausência de reação.
//! - **`KbdGroup` com muitos filhos.** O original é `inline-flex` sem `flex-wrap`: não há quebra
//!   nem overflow com comportamento próprio a medir.

use gpui::{
    div, px, AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Duas cores, e nada mais: o `kbd.tsx` usa `bg-muted` e `text-muted-foreground`, ponto.
//
// Mesma disciplina de cor do resto do crate: TODO valor é `0xRRGGBBAA`, com o byte de alfa, sempre —
// ver [`crate::color`]. Misturar com os tokens de 6 dígitos de [`crate::theme`] desloca os canais e
// produz uma cor completamente diferente, sem erro de compilação.

/// Tokens visuais da tecla, por tema.
#[derive(Clone, Copy, Debug)]
struct KbdPalette {
    /// `--muted` — o fundo da pastilha (`bg-muted`).
    ///
    /// É **translúcido de propósito**: 4% de preto/branco se compõe sobre a superfície em que a
    /// tecla estiver (um item de menu, o rodapé de um diálogo, o miolo de um tooltip). Resolver isso
    /// pra uma cor opaca deixaria a tecla com um retângulo chapado por baixo em toda superfície que
    /// não for exatamente `--background`.
    muted: Rgba8,
    /// `--muted-foreground` — a cor do texto e do ícone (`text-muted-foreground`, e o
    /// `currentColor` que o ícone herda).
    muted_fg: Rgba8,
}

/// Tema **claro**.
const KBD_LIGHT: KbdPalette = KbdPalette {
    muted: Rgba8(0x0000000a), // --alpha(black / 4%)
    // muted-foreground = mix(neutral-500 90%, black) = #686868
    muted_fg: Rgba8(0x686868ff),
};

/// Tema **escuro**.
const KBD_DARK: KbdPalette = KbdPalette {
    muted: Rgba8(0xffffff0a), // --alpha(white / 4%)
    // muted-foreground = mix(neutral-500 90%, white) = #818181
    muted_fg: Rgba8(0x818181ff),
};

/// A paleta da tecla no tema corrente.
fn kbd() -> &'static KbdPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &KBD_DARK,
        theme::ThemeMode::Light => &KBD_LIGHT,
    }
}

// =================================================================================================
// Geometria
// =================================================================================================

/// Altura TOTAL **e** piso de largura — `h-5 min-w-5` = 5 × 4px = **20px**.
///
/// É um número só porque no original é uma decisão só: a tecla nasce **quadrada** e cresce apenas
/// na horizontal, quando o rótulo pede. Duas constantes com o mesmo 20 seriam duas fontes de verdade
/// pro mesmo `5` da referência — a mesma decisão que o [`crate::badge::BadgeSize::height`] toma.
///
/// A altura é total e **não tem borda a descontar** (o `kbd.tsx` não tem borda nenhuma): 20px aqui
/// são 20px de miolo, e é isso que faz a entrelinha de 16 caber com 2px de respiro em cima e embaixo
/// sem nenhum `py-*` no original.
pub const SIDE: f32 = 20.0;

/// Respiro horizontal interno — `px-1` = **4px** de cada lado.
///
/// O `px-1` é inteiro, não o `calc(--spacing(1)-1px)` = 3px do [`crate::badge`]: lá o -1px desconta
/// a borda, e aqui não há borda.
pub const PAD_X: f32 = 4.0;

/// Espaço entre as peças — `gap-1` = **4px**.
///
/// Serve os **dois** `gap-1` do original: o de dentro da tecla (ícone ↔ rótulo) e o do [`KbdGroup`]
/// (tecla ↔ tecla). São a mesma declaração na referência, então é a mesma constante aqui.
pub const GAP: f32 = 4.0;

/// Raio — `rounded-[.25rem]` = 0,25 × 16 = **4px**.
///
/// ⚠️ **Não** é `rounded-sm`. O token `--radius-sm` do coss é **6px**, e a referência escolhe um
/// valor arbitrário menor de propósito — numa caixa de 20px, 6px de raio já começa a ler como
/// pílula. É o mesmo desvio que o `BadgeSize::Sm` faz, e é o número em que o `Kbd` do fork
/// (`rounded_sm`) erra.
pub const RADIUS: f32 = 4.0;

/// Corpo do texto — `text-xs` = 0,75rem = **12px**.
pub const TEXT_SIZE: f32 = 12.0;

/// Entrelinha — o par que o Tailwind v4 anexa ao `text-xs`: `calc(1 / 0.75)` = 1,333…, ou seja
/// 12 × 4/3 = **16px** (o mesmo que `1rem`).
///
/// **Numa caixa deste tamanho a entrelinha é geometria.** Sem declará-la, o GPUI usa o default
/// `relative(1.618_034)` (a razão de ouro, em `gpui/src/geometry.rs`) e a linha sai com
/// 12 × 1,618 = 19,42px dentro de um miolo de 20 — sobram 0,29px de cada lado em vez de 2, e o
/// glifo encosta nas arestas. Como o `h-5` é fixo, a caixa continua medindo 20 e **nada** no código
/// nem na medida denuncia o defeito: só a tela. Travado em
/// `tests::entrelinha_e_geometria_nesta_caixa`.
pub const LINE_HEIGHT: f32 = 16.0;

/// Lado do ícone — `[&_svg:not([class*='size-'])]:size-3` = 3 × 4px = **12px**.
///
/// Exatamente o miolo horizontal do piso de largura (`SIDE − 2 × PAD_X`): uma tecla de ícone só
/// nasce quadrada com o ícone encostando no respiro dos dois lados, sem folga nenhuma. Ver
/// `tests::geometria_e_a_classe_base_da_referencia`.
pub const ICON_SIZE: f32 = 12.0;

// =================================================================================================
// A tecla
// =================================================================================================

/// A pastilha de atalho de teclado, com o visual do coss. Ver o doc do módulo.
///
/// É um **elemento de render** (`RenderOnce`): construa a cada frame. Não tem id, não tem estado e
/// não recebe ponteiro — é decoração (`pointer-events-none` no original).
#[derive(IntoElement)]
pub struct Kbd {
    label: Option<SharedString>,
    icon_before: Option<SharedString>,
    icon_after: Option<SharedString>,
}

impl Kbd {
    /// Uma tecla com **rótulo**: `Kbd::new("Esc")`, `Kbd::new("⌘")`, `Kbd::new("Enter")`.
    ///
    /// Quem escolhe o glifo é o call site, como no original — o `kbd.tsx` não sabe o que é um
    /// atalho. Ver "Ausente" no doc do módulo se você quer formatar um [`gpui::Keystroke`].
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: Some(label.into()),
            icon_before: None,
            icon_after: None,
        }
    }

    /// Uma tecla de **ícone só**, sem rótulo — a seta de um `⇧`, o `⌫` de um backspace. Nasce
    /// quadrada em 20×20 (ver [`ICON_SIZE`]).
    ///
    /// O caminho é servido pela [`crate::assets::Assets`] (ex.:
    /// `"iconoir/regular/arrow-up.svg"`). Sem essa `AssetSource` registrada no bootstrap, o ícone
    /// some SILENCIOSAMENTE — mas a caixa continua medindo 20×20, porque o layout não depende do
    /// asset ter carregado.
    pub fn icon(path: impl Into<SharedString>) -> Self {
        Self {
            label: None,
            icon_before: Some(path.into()),
            icon_after: None,
        }
    }

    /// Ícone **antes** do rótulo, separado dele pelo [`GAP`]. 12px.
    pub fn icon_before(mut self, path: impl Into<SharedString>) -> Self {
        self.icon_before = Some(path.into());
        self
    }

    /// Ícone **depois** do rótulo. 12px.
    pub fn icon_after(mut self, path: impl Into<SharedString>) -> Self {
        self.icon_after = Some(path.into());
        self
    }

    /// Um ícone da tecla, no tamanho e na cor certos.
    ///
    /// A cor é o `currentColor` do original resolvido à mão: o `<svg>` do `kbd.tsx` não tem cor
    /// própria e herda o `text-muted-foreground` da pastilha.
    ///
    /// **Sem `opacity-80`** — ver "Ausente" no doc do módulo. E sem `flex_none`: o `size-3` do
    /// original também não fixa `flex-shrink`, e no piso de largura não há pressão de compressão
    /// pra isso importar.
    fn render_icon(path: SharedString) -> impl IntoElement {
        gpui::svg()
            .path(path)
            .size(px(ICON_SIZE))
            .text_color(kbd().muted_fg.hsla())
    }
}

impl RenderOnce for Kbd {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = kbd();

        let mut el = div()
            // `inline-flex items-center justify-center gap-1`. O `inline-` não tem equivalente (ver
            // "Não reproduzível"); o resto é literal.
            .flex()
            .items_center()
            .justify_center()
            .gap(px(GAP))
            // `h-5 min-w-5`: altura fixa, largura só cresce.
            .h(px(SIDE))
            .min_w(px(SIDE))
            .px(px(PAD_X))
            .rounded(px(RADIUS))
            // `bg-muted` — véu de 4%, que se compõe sobre a superfície de baixo.
            .bg(p.muted.hsla())
            .text_size(px(TEXT_SIZE))
            // Obrigatório: sem isto o GPUI cai na razão de ouro e o glifo encosta nas arestas de uma
            // caixa que continua medindo 20. Ver [`LINE_HEIGHT`].
            .line_height(px(LINE_HEIGHT))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(p.muted_fg.hsla());

        if let Some(path) = self.icon_before {
            el = el.child(Self::render_icon(path));
        }
        if let Some(label) = self.label {
            el = el.child(label);
        }
        if let Some(path) = self.icon_after {
            el = el.child(Self::render_icon(path));
        }

        el
    }
}

// =================================================================================================
// O agrupador
// =================================================================================================

/// O agrupador de teclas — `inline-flex items-center gap-1`, e **nada mais**.
///
/// É o `KbdGroup` do original: existe só pra pôr o [`GAP`] de 4px entre as teclas de um atalho
/// (`⌘` + `K`). Não tem fundo, borda, respiro nem cor própria.
///
/// Diferente do [`Kbd`], aceita **filhos livres** ([`gpui::ParentElement`]) — o original não
/// estiliza descendente nenhum aqui, então não há regra de CSS pro componente aplicar por conta
/// própria. Normalmente os filhos são [`Kbd`], mas um separador de texto entre eles ("ou") também
/// é caso do original.
#[derive(IntoElement, Default)]
pub struct KbdGroup {
    children: Vec<AnyElement>,
}

impl KbdGroup {
    /// Um agrupador vazio; encha com `.child(…)` / `.children(…)`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for KbdGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for KbdGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(GAP))
            .children(self.children)
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `badge.rs`, `button.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// **A convenção de cor da paleta, decodificada de verdade.**
    ///
    /// Os dois valores de [`KbdPalette`] são `0xRRGGBBAA` e vão pro `rgba`. Um valor de 6 dígitos
    /// esquecido ali vira uma cor completamente diferente sem erro de compilação —
    /// `rgba(0x686868)` é lido como `0x00686868`, quase transparente. Isso já custou três bugs
    /// visíveis nesta base.
    ///
    /// Então, em vez de comparar número com número (que não pegaria nada), decodifica e afirma o que
    /// a cor DEVE ser perceptualmente.
    #[test]
    fn paleta_decodifica_pras_cores_pretendidas() {
        // O fundo é um VÉU: translúcido nos dois temas. Se ele vier opaco, a tecla passa a apagar o
        // que estiver atrás dela (um item de menu, o miolo de um tooltip) em vez de se compor.
        for (nome, c) in [("muted claro", KBD_LIGHT.muted), ("muted escuro", KBD_DARK.muted)] {
            let a = c.alpha();
            assert!(a < 1.0, "{nome}: tem que ser translúcido, veio alfa {a:.3}");
            // 4% × 255 = 10,2 → 0x0a. A margem cobre o arredondamento do byte, não uma escolha.
            assert!(
                (a - 0.04).abs() < 0.005,
                "{nome}: --alpha(… / 4%), veio {a:.3}"
            );
        }

        // E o véu troca de LADO entre os temas: preto no claro, branco no escuro. Se os dois forem
        // preto, a tecla desaparece no tema escuro (véu preto sobre fundo quase preto).
        assert_eq!(
            KBD_LIGHT.muted.0 & 0xffffff00,
            0x00000000,
            "claro: --alpha(black / 4%)"
        );
        assert_eq!(
            KBD_DARK.muted.0 & 0xffffff00,
            0xffffff00,
            "escuro: --alpha(white / 4%)"
        );

        // O texto: opaco e NEUTRO nos dois temas — se sair colorido, o valor foi lido deslocado.
        for (nome, c) in [
            ("muted-fg claro", KBD_LIGHT.muted_fg),
            ("muted-fg escuro", KBD_DARK.muted_fg),
        ] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: NEUTRO — se r≠g≠b, o valor foi lido deslocado"
            );
            // Meio-tom de verdade: nem quase-preto (viraria o `--foreground`) nem quase-branco.
            assert!(
                c.r > 0.3 && c.r < 0.6,
                "{nome}: é um meio-tom, veio {:.3}",
                c.r
            );
        }

        // E ele CLAREIA no tema escuro (#686868 → #818181), porque o que está atrás dele inverteu.
        // Repetir o valor do claro no escuro é o erro que deixa a tecla ilegível sobre `#141414`.
        let claro: gpui::Rgba = KBD_LIGHT.muted_fg.hsla().into();
        let escuro: gpui::Rgba = KBD_DARK.muted_fg.hsla().into();
        assert!(
            escuro.r > claro.r,
            "o texto tem que clarear no escuro: claro {:.3}, escuro {:.3}",
            claro.r,
            escuro.r
        );
    }

    /// A paleta segue o tema corrente, e não o contrário. Se os braços do `match` estiverem
    /// trocados, a tecla sai com véu branco no tema claro (invisível) e preto no escuro (idem).
    #[test]
    fn a_paleta_segue_o_tema_corrente() {
        theme::set_theme(theme::ThemeMode::Light);
        assert_eq!(kbd().muted, Rgba8(0x0000000a), "claro: véu PRETO");
        assert_eq!(kbd().muted_fg, Rgba8(0x686868ff));

        theme::set_theme(theme::ThemeMode::Dark);
        assert_eq!(kbd().muted, Rgba8(0xffffff0a), "escuro: véu BRANCO");
        assert_eq!(kbd().muted_fg, Rgba8(0x818181ff));

        theme::set_theme(theme::ThemeMode::Dark); // não deixa estado vazando pros outros testes
    }

    /// **A geometria da classe base do `kbd.tsx`, travada nos literais.**
    ///
    /// O `kbd.tsx` não tem `cva`: não há variante nem tamanho, então esta é a geometria INTEIRA do
    /// componente. Cada número está aqui como literal (e não como o nome da constante que ele
    /// deveria proteger) justamente pra uma edição na constante fazer o teste falhar.
    ///
    /// Os dois números que mais convidam ao erro:
    ///
    /// - **o raio é 4, não 6.** `rounded-[.25rem]` é valor arbitrário; `--radius-sm` é 6px e é o que
    ///   o `Kbd` do `vendor/gpui-component` usa.
    /// - **o respiro é 4 inteiro, não 3.** O [`crate::badge`] usa `calc(--spacing(1)-1px)` = 3px pra
    ///   descontar a borda; aqui não há borda.
    #[test]
    fn geometria_e_a_classe_base_da_referencia() {
        assert_eq!(SIDE, 20.0, "`h-5 min-w-5` = 5 × 4px");
        assert_eq!(PAD_X, 4.0, "`px-1` INTEIRO — não há borda a descontar");
        assert_eq!(GAP, 4.0, "`gap-1`");
        assert_eq!(RADIUS, 4.0, "`rounded-[.25rem]` — NÃO o --radius-sm de 6px");
        assert_eq!(TEXT_SIZE, 12.0, "`text-xs` = .75rem");
        assert_eq!(ICON_SIZE, 12.0, "`size-3` = 3 × 4px");

        // O raio nunca chega a metade do lado: aí seria uma pílula, e o `rounded-full` do Tailwind é
        // que estaria na referência. É a distância que o 6px do `--radius-sm` começaria a comer.
        assert!(RADIUS < SIDE / 2.0, "virou pílula");
        assert_ne!(RADIUS, 6.0, "--radius-sm é 6 e não é isto aqui");

        // **O miolo horizontal do piso de largura é EXATAMENTE o lado do ícone.** Uma tecla de
        // ícone só nasce 20×20 com o ícone encostando no respiro dos dois lados, sem folga: é a
        // conta que o teste de janela `um_kbd_de_um_icone_nasce_quadrado` mede na tela. Se alguém
        // "arredondar" o respiro pra 3 (por analogia com o selo) ou o ícone pra 14, a tecla de
        // ícone deixa de ser quadrada.
        assert_eq!(SIDE - 2.0 * PAD_X, 12.0, "o miolo é 20 − 4 − 4");
        assert_eq!(
            SIDE - 2.0 * PAD_X,
            ICON_SIZE,
            "o ícone preenche o miolo exatamente"
        );
    }

    /// **A entrelinha é geometria nesta caixa** — e é o único número do componente que NENHUMA
    /// medida denuncia.
    ///
    /// `text-xs` no Tailwind v4 traz o par `calc(1 / 0.75)`: 12 × 4/3 = 16px. Dentro de um miolo de
    /// 20px isso dá 2px de respiro em cima e embaixo, que é o que o `items-center` centra — e é por
    /// isso que o original não tem nenhum `py-*`.
    ///
    /// Se a entrelinha ficar solta, o GPUI usa `relative(1.618_034)` e a linha vai a 19,42px: sobram
    /// 0,29px de cada lado, o glifo encosta nas arestas, e como o `h-5` é FIXO a caixa continua
    /// medindo 20×20 na medição. Nada quebra; só fica errado na tela.
    #[test]
    fn entrelinha_e_geometria_nesta_caixa() {
        assert_eq!(LINE_HEIGHT, 16.0, "`text-xs` → calc(1 / .75) × 12px");
        // A conta do par do Tailwind, escrita: 1 / 0.75 = 4/3.
        assert!(
            (LINE_HEIGHT - TEXT_SIZE * 4.0 / 3.0).abs() < 1e-6,
            "o par do text-xs é calc(1 / 0.75), ou seja 4/3"
        );

        // 20 − 16 = 4, ou seja 2px em cima e 2px embaixo. É o respiro vertical que o original NÃO
        // declara com `py-*`, porque a entrelinha já o produz.
        assert_eq!(SIDE - LINE_HEIGHT, 4.0, "2px de respiro de cada lado");
        assert!(
            LINE_HEIGHT > TEXT_SIZE,
            "entrelinha menor que o corpo do texto corta os descendentes"
        );
        assert!(LINE_HEIGHT < SIDE, "a linha tem que caber no miolo");

        // O CONTRAFACTUAL: o default do GPUI (`gpui/src/geometry.rs`) é a razão de ouro. Ele CABE na
        // caixa — é o que torna o defeito invisível pra qualquer medida — mas encosta.
        let default_do_gpui = TEXT_SIZE * 1.618_034;
        assert!(
            (default_do_gpui - 19.416_408).abs() < 1e-3,
            "12 × 1,618034 = 19,42, veio {default_do_gpui}"
        );
        assert!(
            default_do_gpui > LINE_HEIGHT,
            "o default é MAIOR que o par do Tailwind"
        );
        assert!(
            SIDE - default_do_gpui < 1.0,
            "e sobra menos de 1px no total: o glifo encosta nas arestas"
        );
    }

    /// Os slots não se atropelam. Um `icon_before` que gravasse no campo do `icon_after` (ou
    /// vice-versa) compila e só aparece na tela, com o ícone do lado errado do rótulo.
    #[test]
    fn slots_nao_se_atropelam() {
        let t = Kbd::new("Enter")
            .icon_before("iconoir/regular/enter-key.svg")
            .icon_after("iconoir/regular/arrow-right.svg");
        assert_eq!(t.label, Some(SharedString::from("Enter")));
        assert_eq!(
            t.icon_before,
            Some(SharedString::from("iconoir/regular/enter-key.svg"))
        );
        assert_eq!(
            t.icon_after,
            Some(SharedString::from("iconoir/regular/arrow-right.svg"))
        );

        // Uma tecla de rótulo só não tem ícone nenhum.
        let r = Kbd::new("Esc");
        assert!(r.icon_before.is_none() && r.icon_after.is_none());

        // E uma tecla de ícone só não tem rótulo — o piso de largura é que a mantém quadrada. O
        // ícone entra ANTES (é o único), não depois.
        let i = Kbd::icon("iconoir/regular/arrow-up.svg");
        assert!(i.label.is_none());
        assert_eq!(
            i.icon_before,
            Some(SharedString::from("iconoir/regular/arrow-up.svg"))
        );
        assert!(i.icon_after.is_none());

        // O agrupador nasce vazio, pelos dois caminhos.
        assert!(KbdGroup::new().children.is_empty());
        assert!(KbdGroup::default().children.is_empty());
    }
}

/// Os testes que precisam de uma **janela de verdade**.
///
/// Os de cima são aritmética: eles provam que as constantes são as da referência, e não que elas
/// chegaram ao elemento. Num componente cuja geometria inteira são sete números, é aqui que se vê se
/// o `min-w-5` virou `w-5`, se o `px-1` foi aplicado nos dois eixos, ou se o `gap-1` do [`KbdGroup`]
/// ficou no `Kbd` por engano.
///
/// A medição é indireta de propósito: o [`Kbd`] não aceita filhos livres (é o desvio declarado no
/// doc do módulo), então não dá pra plantar uma sonda dentro dele. O que dá é plantar **marcas de
/// tamanho fixo** como irmãs dentro do [`KbdGroup`] e ler onde elas foram parar — a posição delas é
/// função da largura e da altura da tecla no meio.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{canvas, Bounds, Context, Pixels, Render, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Lado da marca. Menor que a tecla de propósito: é a diferença entre os dois que expõe o
    /// `items-center` do agrupador.
    const MARCA: f32 = 8.0;

    /// Qual tecla o medidor monta. Não dá pra guardar um [`Kbd`] no medidor — ele é `RenderOnce` e
    /// o `render` da view roda mais de uma vez.
    #[derive(Clone, Copy)]
    enum Caso {
        /// Tecla de **ícone só**: a largura é determinística, sem depender de métrica de glifo
        /// nenhuma (12 de ícone + 4 + 4 de respiro = exatamente o piso de 20).
        Icone,
        /// Tecla de **uma letra**: o conteúdo é mais estreito que o piso, então quem decide a
        /// largura é o `min-w-5`. É o único caso que prova que o piso chegou ao elemento — no caso
        /// do ícone o conteúdo bate EXATAMENTE nos 20, e um `min-w` esquecido passaria batido.
        Letra,
        /// Tecla com um **rótulo** de várias letras: aqui só a desigualdade é afirmável.
        Rotulo,
        /// **Dois ícones**, um antes e um depois. É o caso que passa do piso de largura sem
        /// envolver nenhum glifo: 4 + 12 + 4 + 12 + 4 = 36. É o único que expõe o `px-1` (nos casos
        /// que param no piso o respiro fica escondido atrás do `min-w`) e o `gap-1` de DENTRO da
        /// tecla (as outras teclas têm um filho só, e sem dois filhos não há gap).
        DoisIcones,
    }

    #[derive(Default)]
    struct Medidas {
        /// A marca ANTES da tecla, dentro do agrupador.
        antes: Option<Bounds<Pixels>>,
        /// A marca DEPOIS da tecla.
        depois: Option<Bounds<Pixels>>,
    }

    /// Uma marca: uma caixa de lado fixo com um `canvas` que só reporta onde ela ficou.
    fn marca(
        medidas: &Rc<RefCell<Medidas>>,
        campo: fn(&mut Medidas, Bounds<Pixels>),
    ) -> impl IntoElement {
        let m = medidas.clone();
        div().w(px(MARCA)).h(px(MARCA)).child(
            canvas(
                move |bounds, _w, _cx| campo(&mut m.borrow_mut(), bounds),
                |_, _, _, _| {},
            )
            .size_full(),
        )
    }

    struct Medidor {
        medidas: Rc<RefCell<Medidas>>,
        caso: Caso,
    }

    impl Render for Medidor {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let m = self.medidas.clone();
            let miolo = match self.caso {
                Caso::Icone => Kbd::icon("iconoir/regular/command.svg"),
                Caso::Letra => Kbd::new("K"),
                Caso::Rotulo => Kbd::new("Shift"),
                Caso::DoisIcones => Kbd::icon("iconoir/regular/command.svg")
                    .icon_after("iconoir/regular/arrow-up.svg"),
            };

            // `flex_col` + `items_start` encolhe o agrupador nos DOIS eixos: no eixo transversal é o
            // `items_start` que evita o esticão, e no principal ele não tem `flex-grow`. Se o
            // agrupador fosse a raiz, ele mediria a janela inteira e o `items-center` centraria as
            // marcas na altura da JANELA em vez da altura da tecla — que é justamente o que se quer
            // medir. E, sendo o primeiro filho sem respiro nenhum, ele nasce em (0, 0).
            div().flex().flex_col().items_start().child(
                KbdGroup::new()
                    .child(marca(&m, |md, b| md.antes = Some(b)))
                    .child(miolo)
                    .child(marca(&m, |md, b| md.depois = Some(b))),
            )
        }
    }

    /// Abre a janela, deixa o layout assentar e devolve as medidas.
    fn medir(cx: &mut TestAppContext, caso: Caso) -> Medidas {
        let medidas = Rc::new(RefCell::new(Medidas::default()));
        let m = medidas.clone();
        let window = cx.add_window(move |_window, _cx| Medidor { medidas: m, caso });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let out = medidas.borrow();
        Medidas {
            antes: out.antes,
            depois: out.depois,
        }
    }

    /// **Uma tecla de ícone nasce 20×20 — medido na tela.**
    ///
    /// Com uma marca de 8px de cada lado da tecla dentro do agrupador, e o agrupador em (0, 0):
    ///
    /// - a marca de antes começa em `x = 0`;
    /// - a de depois começa em `8 + 4 + largura_da_tecla + 4`, e com a tecla em 20 isso dá **36**;
    /// - as duas ficam em `y = (20 − 8) / 2 = 6`, o que prova de uma vez a **altura** de 20 e o
    ///   `items-center` do agrupador.
    ///
    /// O caso do ícone é o único totalmente determinístico: 12 de ícone + 4 + 4 de respiro fecham
    /// exatamente o piso de 20, sem métrica de fonte no meio. Ele trava `SIDE`, `PAD_X`,
    /// `ICON_SIZE` e `GAP` num número só — mexer em qualquer um dos quatro tira o 36 do lugar.
    #[gpui::test]
    fn um_kbd_de_um_icone_nasce_quadrado(cx: &mut TestAppContext) {
        let m = medir(cx, Caso::Icone);
        let antes = m.antes.expect("a marca de antes pintou");
        let depois = m.depois.expect("a marca de depois pintou");

        assert_eq!(antes.origin.x, px(0.0), "o agrupador nasce na origem");
        assert_eq!(
            (antes.size.width, antes.size.height),
            (px(8.0), px(8.0)),
            "a marca mede o que pediu"
        );

        // 8 (marca) + 4 (gap) + 20 (tecla) + 4 (gap) = 36.
        assert_eq!(
            depois.origin.x,
            px(36.0),
            "a tecla de ícone tem que medir 20 de largura: 8 + 4 + 20 + 4 = 36"
        );

        // (20 − 8) / 2 = 6 — a altura da tecla, lida pelo `items-center` do agrupador.
        assert_eq!(
            antes.origin.y,
            px(6.0),
            "as marcas centram contra uma tecla de 20px de altura"
        );
        assert_eq!(depois.origin.y, antes.origin.y, "as duas na mesma linha");
    }

    /// **O `min-w-5` é o que segura uma tecla de uma letra em 20px de largura.**
    ///
    /// Um `K` a 12px mede uns 8px: sem o piso, a tecla sairia com 8 + 4 + 4 = 16 e a marca de depois
    /// começaria em 32, não em 36. É o único caso que prova o `min-w` — na tecla de ícone o conteúdo
    /// bate exatamente nos 20 e um `min_w` esquecido não mudaria nada na medida.
    ///
    /// A premissa (o glifo é mais estreito que os 12px de miolo) está afirmada junto: se a fonte da
    /// janela de teste mudar pra uma em que o `K` passe de 12px, é este `assert` que explica o
    /// motivo em vez de o número sair torto sem justificativa.
    #[gpui::test]
    fn o_piso_de_largura_segura_uma_tecla_de_uma_letra(cx: &mut TestAppContext) {
        let letra = medir(cx, Caso::Letra);
        let antes = letra.antes.expect("a marca de antes pintou");
        let depois = letra.depois.expect("a marca de depois pintou");

        assert_eq!(antes.origin.x, px(0.0));
        // 8 (marca) + 4 (gap) + 20 (piso) + 4 (gap) = 36 — o MESMO número da tecla de ícone, porque
        // as duas param no piso.
        assert_eq!(
            depois.origin.x,
            px(36.0),
            "o `min-w-5` tem que segurar a tecla em 20: 8 + 4 + 20 + 4 = 36"
        );

        // A premissa: o conteúdo é mais estreito que o piso. Se não fosse, o teste acima estaria
        // medindo o glifo em vez do piso.
        let icone = medir(cx, Caso::Icone);
        let depois_icone = icone.depois.expect("a marca de depois pintou");
        assert_eq!(
            depois.origin.x, depois_icone.origin.x,
            "as duas param no piso, então medem igual"
        );
    }

    /// **O respiro `px-1` e o `gap-1` de DENTRO da tecla, medidos — e só este caso os vê.**
    ///
    /// Duas coisas ficam invisíveis nos outros testes de janela:
    ///
    /// - **o respiro.** Numa tecla que para no piso de largura, o `px-1` está escondido atrás do
    ///   `min-w-5`: tirar os 4px de cada lado não muda a largura medida, porque o piso a segura em
    ///   20 de qualquer jeito. Só passando do piso é que o respiro aparece na conta.
    /// - **o `gap` interno.** As teclas dos outros casos têm um filho só, e um filho só não produz
    ///   gap nenhum.
    ///
    /// Dois ícones resolvem os dois de uma vez e **sem nenhuma métrica de glifo no meio**:
    /// 4 + 12 + 4 + 12 + 4 = **36** de largura, logo a marca de depois começa em
    /// 8 + 4 + 36 + 4 = **52**.
    #[gpui::test]
    fn o_respiro_e_o_gap_interno_aparecem_com_dois_icones(cx: &mut TestAppContext) {
        let m = medir(cx, Caso::DoisIcones);
        let antes = m.antes.expect("a marca de antes pintou");
        let depois = m.depois.expect("a marca de depois pintou");

        assert_eq!(antes.origin.x, px(0.0));
        assert_eq!(
            depois.origin.x,
            px(52.0),
            "a tecla mede 4 + 12 + 4 + 12 + 4 = 36, então a marca cai em 8 + 4 + 36 + 4 = 52"
        );

        // E a altura não mudou: dois ícones de 12 continuam dentro dos 20 do `h-5`.
        assert_eq!(antes.origin.y, px(6.0), "`h-5` não se mexe");
    }

    /// **O rótulo alarga a tecla, e só isso**: a altura continua 20 e o `gap` continua 4.
    ///
    /// É o `h-5` sendo altura FIXA, e não um `min-h`: um rótulo de cinco letras a 12px passa dos
    /// 20px de largura, mas nada nele pode empurrar a caixa pra baixo. Só a desigualdade é afirmada
    /// na horizontal — a largura exata dependeria da métrica do glifo, que não é contrato deste
    /// componente.
    #[gpui::test]
    fn o_rotulo_alarga_mas_a_altura_nao_muda(cx: &mut TestAppContext) {
        let m = medir(cx, Caso::Rotulo);
        let antes = m.antes.expect("a marca de antes pintou");
        let depois = m.depois.expect("a marca de depois pintou");

        assert_eq!(antes.origin.x, px(0.0));
        assert!(
            depois.origin.x > px(36.0),
            "\"Shift\" a 12px passa do piso de 20; veio {:?}",
            depois.origin.x
        );

        // A altura NÃO mudou: as marcas continuam em y = 6.
        assert_eq!(
            antes.origin.y,
            px(6.0),
            "`h-5` é altura fixa — o rótulo não engorda a caixa"
        );
        assert_eq!(depois.origin.y, px(6.0));
    }
}

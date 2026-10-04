//! `Empty` — o **estado vazio** do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/empty.tsx`
//!
//! # O que é
//!
//! O bloco centrado que ocupa o lugar de uma lista que não tem nada dentro: uma **mídia** (ícone
//! numa cartelinha, ou uma imagem/avatar), um **título**, uma **descrição** e um bloco de
//! **conteúdo** (normalmente os botões de ação).
//!
//! ```ignore
//! Empty::new()
//!     .header(
//!         EmptyHeader::new()
//!             .media(EmptyMedia::icon("iconoir/regular/folder.svg"))
//!             .title("Nenhum projeto")
//!             .description("Crie um projeto pra começar."),
//!     )
//!     .content(EmptyContent::new().child(Button::new("novo", "Novo projeto")))
//! ```
//!
//! # Dirigido por CONTEÚDO, do começo ao fim
//!
//! Não existe uma única altura fixa neste componente: a caixa mede exatamente o que os textos
//! medirem. Por isso **toda** entrelinha aqui é explícita. O default do GPUI é
//! `relative(1.618_034)` (a razão de ouro, em `gpui/src/geometry.rs`), e o Tailwind usa pares
//! fixos: `text-sm` é 14/**20**, `text-xl` é 20/**28**. Sem fixar, o título sairia com 32,4px em vez
//! de 28 e a descrição com 22,7 em vez de 20 — o bloco inteiro ~13% mais alto, sem nada no código
//! parecendo errado. É o que o teste `tests_de_janela::altura_e_a_soma_das_partes` tranca, medindo
//! a janela de verdade: 328px, não ~335.
//!
//! # As quatro peças
//!
//! Como no [`crate::card`] e no [`crate::frame`], cada peça é **dona** das suas fatias, em vez de
//! ser um monte de primitivas soltas que o call site empilha na ordem certa. Isso importa aqui por
//! um motivo concreto: a regra `[[data-slot=empty-title]+&]:mt-1` do original é uma **adjacência de
//! CSS** (a descrição ganha 4px de topo só quando vem logo depois do título). Com o header dono do
//! título e da descrição, a mesma regra é uma condição local e determinística — não depende da
//! ordem em que os filhos foram declarados.
//!
//! | daqui | no original |
//! |---|---|
//! | [`Empty`] | `Empty` |
//! | [`EmptyHeader`] | `EmptyHeader` + `EmptyTitle` + `EmptyDescription` |
//! | [`EmptyMedia`] | `EmptyMedia` (com o `cva` `emptyMediaVariants`) |
//! | [`EmptyContent`] | `EmptyContent` |
//!
//! # O leque é DESENHO, não layout
//!
//! A variante [`EmptyMediaVariant::Icon`] são **três** cartelinhas: a da frente, e duas decorativas
//! (`aria-hidden` no original) atrás dela, giradas ∓10° em `scale-84` com origem no canto de baixo —
//! o baralho aberto. O GPUI não gira `div`, então as duas de trás são **caminho vetorial**
//! ([`gpui::PathBuilder`] + [`gpui::Window::paint_path`]) dentro de um [`gpui::canvas`], com a
//! transformação do CSS resolvida à mão em [`fan_point`]. Detalhes e o preço da troca estão no
//! comentário de [`paint_fan`].
//!
//! O número que mostra que a conta está certa é este: girar uma caixa de `36 × 0,84` em 10° em torno
//! do canto de baixo levanta o canto oposto `30,24 · (sen 10° + cos 10°) = 35,03`px — exatamente a
//! altura da cartelinha da frente menos o `bottom-px`. Ou seja o desenho do coss é afinado pra que a
//! ponta das cartelinhas de trás encoste na aresta de cima da mídia, e é isso que o teste
//! `a_ponta_do_leque_encosta_no_topo_da_midia` tranca — errar a ordem da transformação ou a origem
//! do giro não cai nesse número por acaso.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`text-balance`** (no [`Empty`] e no [`EmptyContent`]): o GPUI não tem balanceamento de
//!   linhas. A quebra é a gulosa comum. Afeta só ONDE a linha quebra, não a altura: um texto que
//!   ocupa duas linhas ocupa duas linhas nos dois casos.
//! - **`not-dark:bg-clip-padding`** na cartelinha: o GPUI pinta o fundo na border box. Só importa
//!   porque a borda (`--border`) é translúcida — no tema claro ela lê sobre o branco do fundo em vez
//!   de sobre o que está atrás. Mesma diferença já documentada no [`crate::dialog`] e no
//!   [`crate::toast`].
//! - **`[&>a]:underline [&>a]:underline-offset-4 [&>a:hover]:text-primary`** da descrição: o
//!   original aceita um `<a>` inline no meio do texto. Aqui a descrição é uma
//!   [`gpui::SharedString`], não uma árvore — não há texto rico com trechos interativos. Quem
//!   precisa de link no estado vazio põe um [`crate::Button`] com
//!   [`crate::ButtonVariant::Link`](crate::ButtonVariant) no [`EmptyContent`].
//! - **`data-slot` / `data-variant`**: atributos de DOM, usados no original pra estilizar de fora
//!   (é como a adjacência do `mt-1` funciona). Sem DOM, sem atributo — a informação virou decisão
//!   local (ver acima).
//! - **`[&_svg]:pointer-events-none`**: um [`gpui::svg`] não recebe ponteiro nesta base de qualquer
//!   forma; a classe é redundante aqui, não omitida.
//!
//! **Valores deduzidos** (não estavam na tabela de tokens resolvidos)
//!
//! - **`--shadow-sm`** do `shadow-sm/5` da cartelinha. A tabela do coss cobre cor e raio, não
//!   sombra, então usei o default do Tailwind v4 — `0 1px 3px 0 black/10, 0 1px 2px -1px black/10`
//!   — com o `/5` trocando o alfa das duas por 5%. É a MESMA fonte de onde o [`crate::dialog`]
//!   resolveu o `--shadow-lg` dele. Se o coss redefine `--shadow-sm` no `globals.css`, este é o
//!   valor a conferir. Ver [`icon_shadows`].
//! - **`font-heading`** do título: token de FAMÍLIA de fonte, ausente da tabela. O título sai na
//!   fonte herdada, com corpo e peso certos (`text-xl`, `font-semibold`). Mesma lacuna, e mesma
//!   escolha, do [`crate::dialog`].
//!
//! **Escolha de breakpoint**
//!
//! - O respiro vertical do [`Empty`] é `py-12 md:py-20`. Nesta base o ramo de desktop é o que vale
//!   (a mesma regra do `sm:` que o [`crate::dialog`] já aplica), então [`PAD_Y`] é o **`md:py-20`**
//!   = 80px, e o `py-12` = 48px é o ramo estreito — código morto aqui. ⚠️ Vale reparar que `md:` é
//!   consulta de **viewport** (≥768px), não de container: numa janela de desktop ele vale mesmo que
//!   o `Empty` esteja dentro de um painel de 300px. Numa janela mais estreita que 768px o original
//!   cairia pra 48px e nós não — é o único ponto onde a fidelidade depende do tamanho da janela.
//!
//! **Superset consciente**
//!
//! - O original não põe cor de texto no [`Empty`]; ele herda o `--foreground` do ancestral. O GPUI
//!   não tem `inherit`, e um estado vazio montado sobre um container que não declarou cor sairia na
//!   cor default da janela. Então a raiz declara `--foreground`, que é o valor que o original
//!   herdaria em qualquer superfície do coss. Título e descrição declaram a sua por cima, como no
//!   original.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, point, px, AnyElement, App, Bounds, Div, IntoElement, ParentElement, PathBuilder, Pixels,
    Point, RenderOnce, SharedString, Styled, Window,
};

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Mesma disciplina de cor do card, da moldura e do modal: TODO valor é `0xRRGGBBAA`, com o byte de
// alfa, SEMPRE — ver [`crate::color`]. Os tokens de [`crate::theme`] são `0xRRGGBB`, e misturar as
// duas convenções desloca os canais e produz outra cor sem erro de compilação (`rgba(0xffffff)` é
// lido como ciano; já custou três bugs visíveis nesta base). A única ponte é [`crate::color::opaque`].
//
// Todos os valores abaixo saíram da tabela de tokens do coss já resolvida do `globals.css` — não
// foram deduzidos de nome de classe.

/// Tokens visuais do estado vazio, por tema.
#[derive(Clone, Copy, Debug)]
struct EmptyPalette {
    /// `--foreground` — a cor do título (e a que a raiz declara pros filhos).
    fg: Rgba8,
    /// `--muted-foreground` — a descrição.
    muted_fg: Rgba8,
    /// `--card` — o fundo da cartelinha da variante [`EmptyMediaVariant::Icon`].
    card: Rgba8,
    /// `--border` — a borda de 1px da cartelinha.
    border: Rgba8,
    /// `shadow-sm/5` — a cor das duas sombras da cartelinha.
    shadow: Rgba8,
    /// Fio de bisel de 1px da cartelinha. Desce no claro, sobe no escuro.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (`0 1px`, base), `-1` sobe (`0 -1px`, topo).
    bevel_dir: f32,
}

/// Tema **claro**.
const EMPTY_LIGHT: EmptyPalette = EmptyPalette {
    fg: Rgba8(0x262626ff), // neutral-800
    // mix(neutral-500 90%, black) = #686868
    muted_fg: Rgba8(0x686868ff),
    card: Rgba8(0xffffffff),
    border: Rgba8(0x00000014), // black 8%
    shadow: Rgba8(0x0000000d), // black 5% (o `/5` do `shadow-sm/5`)
    bevel: Rgba8(0x0000000a),  // black 4% — o `--color-black/4%` do `before:shadow`
    bevel_dir: 1.0,
};

/// Tema **escuro**.
const EMPTY_DARK: EmptyPalette = EmptyPalette {
    fg: Rgba8(0xf5f5f5ff), // neutral-100
    // mix(neutral-500 90%, white) = #818181
    muted_fg: Rgba8(0x818181ff),
    // `--card` escuro = mix(background 98%, white), com background = mix(neutral-950 96%, white).
    card: Rgba8(0x191919ff),
    border: Rgba8(0xffffff0f), // white 6%
    shadow: Rgba8(0x0000000d), // black 5% — a sombra não muda de cor entre os temas
    // ⚠️ DESVIO CONSCIENTE do coss, o MESMO já vigente no `Input`, no `Card`, no `Frame` e no
    // `Button`: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO — alfa 30 ≈ 11,8% — porque
    // a 6% o filete é imperceptível no nosso fundo escuro. NÃO "corrija" isto pra 0x0f achando que é
    // erro de porte; se a intenção mudar, mude junto o teste `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
};

/// A paleta do estado vazio no tema corrente.
fn palette() -> &'static EmptyPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &EMPTY_DARK,
        theme::ThemeMode::Light => &EMPTY_LIGHT,
    }
}

// --- Geometria ------------------------------------------------------------------------------------

/// Respiro horizontal da raiz — `px-6`.
const PAD_X: f32 = 24.0;

/// Respiro vertical da raiz — o `md:py-20` de `py-12 md:py-20`.
///
/// Ver "Escolha de breakpoint" no doc do módulo: nesta base vale o ramo de desktop, e o `py-12`
/// (48px) é código morto. É de longe o maior número do componente — 160px dos ~328 de um estado
/// vazio típico são estes dois respiros.
const PAD_Y: f32 = 80.0;

/// Espaço entre o header e o conteúdo — `gap-6`.
const GAP: f32 = 24.0;

/// Largura máxima do header e do conteúdo — `max-w-sm` = `24rem`.
///
/// É o que faz a descrição quebrar em linhas curtas e centradas em vez de atravessar um painel
/// largo inteiro.
const MAX_W: f32 = 384.0;

/// Espaço entre a mídia e o que vem depois dela — `mb-6` no wrapper da mídia.
///
/// **Incondicional**, como no original: é margem do wrapper, não adjacência. Um header que só tem
/// mídia carrega 24px de sobra embaixo — é o comportamento do original, e não uma omissão daqui.
const MEDIA_MB: f32 = 24.0;

/// Lado da cartelinha da variante [`EmptyMediaVariant::Icon`] — `size-9`.
const ICON_BOX: f32 = 36.0;

/// Raio da cartelinha — `rounded-md`, que no coss é `calc(var(--radius) - 2px)` = `10 − 2` = **8px**.
const ICON_RADIUS: f32 = 8.0;

/// Corpo do ícone dentro da cartelinha — `size-4.5` (o
/// `[&_svg:not([class*='size-'])]:size-4.5` do original).
const ICON_SIZE: f32 = 18.0;

/// Redução das duas cartelinhas de trás do leque — `scale-84`.
const FAN_SCALE: f32 = 0.84;

/// Giro de cada cartelinha de trás — `-rotate-10` na da esquerda, `rotate-10` na da direita.
const FAN_ROT: f32 = 10.0;

/// Deslocamento horizontal de cada cartelinha de trás — `-translate-x-0.5` / `translate-x-0.5`, que
/// na escala de espaçamento do Tailwind é `0.125rem` = **2px**.
const FAN_DX: f32 = 2.0;

/// Quanto a cartelinha de trás sobe do rodapé da mídia — `bottom-px`.
const FAN_BOTTOM: f32 = 1.0;

/// Corpo do título — `text-xl`.
const TITLE_SIZE: f32 = 20.0;

/// Entrelinha do título — o PAR do `text-xl` no Tailwind: 20px de fonte, **28** de linha.
///
/// Ver "Dirigido por conteúdo" no doc do módulo. Sem isto o título mede 32,4px (a razão de ouro do
/// GPUI sobre 20) e o bloco inteiro engorda.
const TITLE_LINE: f32 = 28.0;

/// Corpo da descrição e do conteúdo — `text-sm`.
const TEXT_SIZE: f32 = 14.0;

/// Entrelinha da descrição e do conteúdo — o PAR do `text-sm`: 14px de fonte, **20** de linha.
const TEXT_LINE: f32 = 20.0;

/// Respiro de topo da descrição **quando ela vem logo depois do título** — `mt-1`.
///
/// No original é a adjacência `[[data-slot=empty-title]+&]:mt-1`; aqui é condição local (ver o doc
/// do módulo). Uma descrição sozinha, sem título, não ganha nada.
const DESC_MT: f32 = 4.0;

/// Espaço entre os filhos do [`EmptyContent`] — `gap-4`.
const CONTENT_GAP: f32 = 16.0;

/// As duas sombras do `shadow-sm/5` da cartelinha.
///
/// ⚠️ **Valor deduzido.** O `--shadow-sm` do Tailwind v4 é
/// `0 1px 3px 0 black/10, 0 1px 2px -1px black/10`, e o modificador `/5` troca o alfa das duas por
/// 5%. A tabela de tokens do coss cobre cor e raio, não sombra — ver o doc do módulo.
///
/// É sombra **externa** sobre fundo **opaco** (`bg-card`), que é o caso em que a técnica do GPUI
/// funciona: o `Window::paint_shadows` não recorta a sombra pra fora do elemento que a projeta, e
/// ali ninguém vê. Pro filete de 1px o caminho é outro — ver [`EmptyMedia::bevel`].
fn icon_shadows() -> Vec<gpui::BoxShadow> {
    let cor = palette().shadow.hsla();
    vec![
        gpui::BoxShadow {
            color: cor,
            offset: gpui::point(px(0.0), px(1.0)),
            blur_radius: px(3.0),
            spread_radius: px(0.0),
        },
        gpui::BoxShadow {
            color: cor,
            offset: gpui::point(px(0.0), px(1.0)),
            blur_radius: px(2.0),
            spread_radius: px(-1.0),
        },
    ]
}

// --- O leque -------------------------------------------------------------------------------------
//
// As duas cartelinhas decorativas atrás da da frente — o efeito de baralho aberto. No original são
// dois `<div aria-hidden>` com a MESMA classe da cartelinha da frente mais
// `absolute bottom-px origin-bottom-{left,right} ∓translate-x-0.5 ∓rotate-10 scale-84 shadow-none`.
//
// **Por que isto é desenho e não `div`:** o GPUI não gira `div`. A `TransformationMatrix` só vale
// pra `svg` e imagem, e não existe primitiva de quad girado — foi por isso que esta variante ficou
// sem o leque na primeira passada. O caminho que EXISTE é o [`gpui::PathBuilder`], que tem `fill` e
// `stroke` e sai pela [`Window::paint_path`]; a mesma dupla que os contornos de gizmo do
// `pve-ui::editor_shell` já usam nesta base. Ou seja: **não foi preciso mexer no fork.**
//
// O preço de trocar `div` por path é que tudo que o `div` dava de graça passa a ser conta nossa:
// fundo, borda por dentro da border box e o filete de bisel são TRÊS passes de path (ver
// [`paint_fan`]), e o `paint_path` não desenha sombra — o que aqui é de graça, porque as
// cartelinhas de trás são `shadow-none`.

/// Onde um ponto da cartelinha de trás do leque cai, em coordenadas da mídia.
///
/// `(u, v)` é o ponto na caixa de 36×36 **antes** da transformação, com `(0,0)` no canto de cima à
/// esquerda dela; `direita` escolhe qual das duas cartelinhas.
///
/// É o `origin-bottom-* / translate-x / rotate / scale` do original numa conta só, na ordem em que o
/// Tailwind v4 compõe o `transform` — `translate rotate scale`, e função aplica da direita pra
/// esquerda, ou seja **escala primeiro, gira depois, desloca por último**. As duas primeiras em
/// torno da `transform-origin`, que é o canto de BAIXO (esquerdo ou direito) da caixa.
///
/// Esta é a ÚNICA implementação da transformação: o [`paint_fan`] manda os pontos do contorno por
/// aqui em vez de usar o `PathBuilder::rotate`/`scale`, justamente pra que a conta que os testes
/// checam seja a mesma que pinta.
fn fan_point(direita: bool, u: f32, v: f32) -> Point<Pixels> {
    let (ox, sinal) = if direita { (ICON_BOX, 1.0) } else { (0.0, -1.0) };
    // 1. Relativo à origem do giro.
    let (x, y) = (u - ox, v - ICON_BOX);
    // 2. `scale-84`. Por ser em torno da origem, encolhe junto o raio e a borda — é por isso que
    //    quem chama passa raio e espessura já multiplicados por [`FAN_SCALE`].
    let (x, y) = (x * FAN_SCALE, y * FAN_SCALE);
    // 3. `∓rotate-10`. Ângulo positivo gira no sentido do relógio, igual ao CSS: aqui o y cresce
    //    pra baixo, então a matriz de rotação padrão já sai no sentido certo.
    let (sin, cos) = (sinal * FAN_ROT).to_radians().sin_cos();
    let (x, y) = (x * cos - y * sin, x * sin + y * cos);
    // 4. `∓translate-x-0.5`, e a volta pra origem — que o `bottom-px` deixa 1px acima do rodapé.
    point(px(x + ox + sinal * FAN_DX), px(y + ICON_BOX - FAN_BOTTOM))
}

/// Qual aresta do contorno emitir — o bisel é UM lado, não a volta inteira.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FanEdge {
    /// A volta fechada: é o que o fundo e a borda usam.
    All,
    /// Só a aresta de baixo, com os dois cantos — o bisel do tema claro (`0 1px`).
    Bottom,
    /// Só a de cima, com os dois cantos — o bisel do tema escuro (`0 -1px`).
    Top,
}

/// Emite no `pb` o contorno de uma cartelinha de trás do leque, já girado e no lugar.
///
/// - `origem`: o canto de cima à esquerda da mídia, em coordenadas de janela.
/// - `inset`: quanto recuar pra dentro, em px de ANTES da escala. `0` é a border box (o contorno de
///   fora); `0.5` é onde um traço de 1px cobre exatamente o 1px de dentro dela, que é onde o CSS
///   põe a borda e o bisel.
fn fan_contour(
    pb: &mut PathBuilder,
    direita: bool,
    origem: Point<Pixels>,
    inset: f32,
    edge: FanEdge,
) {
    let p = |u: f32, v: f32| origem + fan_point(direita, u, v);
    // Extremos e raio do contorno recuado. O raio do arco vai pro `arc_to` já escalado, porque lá
    // ele é medido no espaço final; o giro não mexe nele — círculo girado é círculo.
    let (a, b) = (inset, ICON_BOX - inset);
    let r = ICON_RADIUS - inset;
    let raio = point(px(r * FAN_SCALE), px(r * FAN_SCALE));
    // Sentido horário, e por isso `sweep = true` nos quatro arcos. O giro é rígido (não espelha),
    // então o sentido vale igual nas duas cartelinhas.
    let arco =
        |pb: &mut PathBuilder, to: Point<Pixels>| pb.arc_to(raio, px(0.0), false, true, to);
    match edge {
        FanEdge::All => {
            pb.move_to(p(a + r, a));
            pb.line_to(p(b - r, a));
            arco(pb, p(b, a + r));
            pb.line_to(p(b, b - r));
            arco(pb, p(b - r, b));
            pb.line_to(p(a + r, b));
            arco(pb, p(a, b - r));
            pb.line_to(p(a, a + r));
            arco(pb, p(a + r, a));
            pb.close();
        }
        FanEdge::Bottom => {
            pb.move_to(p(b, b - r));
            arco(pb, p(b - r, b));
            pb.line_to(p(a + r, b));
            arco(pb, p(a, b - r));
        }
        FanEdge::Top => {
            pb.move_to(p(a, a + r));
            arco(pb, p(a + r, a));
            pb.line_to(p(b - r, a));
            arco(pb, p(b, a + r));
        }
    }
}

/// Pinta as duas cartelinhas de trás do leque. `origem` é o canto de cima à esquerda da mídia.
///
/// Três passes por cartelinha, porque é o que substitui o que um `div` daria de graça:
///
/// 1. **fundo** — o contorno de fora inteiro, preenchido com `--card`. Cobre a border box, como o
///    `background-clip: border-box` do CSS (o `not-dark:bg-clip-padding` é a mesma omissão já
///    declarada no doc do módulo pra cartelinha da frente).
/// 2. **borda** — traço de 1px POR DENTRO da border box, como o CSS: contorno recuado meio pixel e
///    traço de um. Os dois já encolhidos pelo `scale-84`, que no original encolhe a borda também.
/// 3. **bisel** — o mesmo filete de 1px da cartelinha da frente (ver [`EmptyMedia::bevel`]), na
///    aresta que o SINAL do deslocamento escolhe: base no claro, topo no escuro.
///
/// ⚠️ **Onde o filete difere do da cartelinha da frente:** ali ele é `border_b_1` num overlay, e o
/// shader de quad do GPUI afina o filete ao longo do arco do canto; aqui o traço tem espessura
/// constante nos dois cantos. É diferença de sub-pixel a 4% de alfa, e no tema **claro** ela é
/// invisível de qualquer jeito — a aresta de baixo das duas cartelinhas de trás fica INTEIRA atrás
/// da cartelinha da frente. No escuro aparecem os 6,2px de aresta de cima que sobram pra fora.
fn paint_fan(origem: Point<Pixels>, window: &mut Window) {
    let p = palette();
    let bisel = if p.bevel_dir > 0.0 {
        FanEdge::Bottom
    } else {
        FanEdge::Top
    };
    // A da esquerda primeiro, na ordem do original: sem z-index, quem pinta depois fica em cima, e
    // no DOM a da direita vem depois. As duas se cruzam só atrás da cartelinha da frente.
    for direita in [false, true] {
        // O `build` só falha em caminho degenerado (mesma nota do `pve-ui::editor_shell`), e nenhum
        // destes é — as constantes garantem `ICON_RADIUS < ICON_BOX / 2`. Se falhasse, a cartelinha
        // some em vez de derrubar o frame.
        for (mut pb, cor, edge, inset) in [
            (PathBuilder::fill(), p.card, FanEdge::All, 0.0),
            (PathBuilder::stroke(px(FAN_SCALE)), p.border, FanEdge::All, 0.5),
            (PathBuilder::stroke(px(FAN_SCALE)), p.bevel, bisel, 0.5),
        ] {
            fan_contour(&mut pb, direita, origem, inset, edge);
            if let Ok(path) = pb.build() {
                window.paint_path(path, cor.hsla());
            }
        }
    }
}

// =================================================================================================
// EmptyMedia
// =================================================================================================

/// Como a mídia do estado vazio se apresenta — o `cva` `emptyMediaVariants` do original.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EmptyMediaVariant {
    /// `bg-transparent`: a mídia é o que você puser dentro, sem moldura nenhuma. É a variante pra
    /// imagem, ilustração ou pilha de avatares.
    #[default]
    Default,
    /// A **cartelinha**: 36×36, raio 8, borda, `bg-card`, sombra e bisel, com o ícone a 18px no
    /// centro — e o **leque** de duas cartelinhas giradas atrás dela (ver o doc do módulo).
    ///
    /// O leque passa 6,2px pra fora de cada lado sem entrar na medida da mídia, igual ao original:
    /// quem alinha a mídia pela caixa alinha pelos 36px da cartelinha da frente.
    Icon,
}

/// A **mídia** do estado vazio: o que aparece acima do título.
///
/// O wrapper carrega 24px de respiro embaixo ([`MEDIA_MB`]) em qualquer variante.
#[derive(Default)]
pub struct EmptyMedia {
    variant: EmptyMediaVariant,
    /// Caminho de um ícone SVG, servido pela [`crate::assets::Assets`].
    icon: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl EmptyMedia {
    /// Mídia da variante [`EmptyMediaVariant::Default`] — sem moldura. Ponha o conteúdo com
    /// [`Self::child`].
    pub fn new() -> Self {
        Self::default()
    }

    /// A **cartelinha com ícone**: já vem na variante [`EmptyMediaVariant::Icon`] com o ícone
    /// dentro, a 18px e na cor `--foreground`.
    ///
    /// O caminho é servido pela [`crate::assets::Assets`] (ex.:
    /// `"iconoir/regular/folder.svg"`). Sem essa `AssetSource` registrada no bootstrap, o ícone
    /// some SILENCIOSAMENTE e sobra a cartelinha vazia.
    pub fn icon(path: impl Into<SharedString>) -> Self {
        Self {
            variant: EmptyMediaVariant::Icon,
            icon: Some(path.into()),
            children: Vec::new(),
        }
    }

    /// Troca a variante — pra montar a cartelinha em volta de um elemento seu em vez de um ícone
    /// SVG, ou pra tirar a moldura de uma mídia criada com [`Self::icon`].
    pub fn variant(mut self, variant: EmptyMediaVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Adiciona um elemento à mídia. A ordem de chamada é a ordem na tela, depois do ícone.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Se a mídia não tem nada pra mostrar — e aí o [`EmptyHeader`] não a renderiza, pra um
    /// `.media(EmptyMedia::new())` distraído não empurrar o título 24px pra baixo do nada.
    ///
    /// A variante [`EmptyMediaVariant::Icon`] **nunca** é vazia: a cartelinha é desenho próprio e
    /// aparece mesmo sem ícone dentro (é o que o original faz).
    fn is_empty(&self) -> bool {
        self.variant == EmptyMediaVariant::Default
            && self.icon.is_none()
            && self.children.is_empty()
    }

    /// O bisel de 1px sobreposto da cartelinha.
    ///
    /// ⚠️ Duas coisas que já custaram bug visível nesta base (ver `crate::input::bevel_for`,
    /// [`crate::card`] e [`crate::frame`]):
    ///
    /// 1. **É borda, não sombra.** O `Window::paint_shadows` do GPUI não recorta a sombra pra fora
    ///    do elemento que a projeta (diferente do CSS), então o `before:box-shadow` do coss viraria
    ///    uma lavagem de cor sobre a cartelinha inteira.
    /// 2. **Cobre a BORDER box, não a padding box.** O `before:rounded-[calc(var(--radius-md)-1px)]`
    ///    do original descreve um pseudo-elemento na **padding** box, e é de lá que sai o `− 1`. O
    ///    nosso overlay fica em `inset: -1px`, ou seja na **border** box, então o raio dele é o da
    ///    superfície ([`ICON_RADIUS`]) — é o que o deixa concêntrico com a borda que ele clareia.
    ///
    /// E o LADO do filete sai do **sinal** do deslocamento, não de uma escolha paralela:
    /// `shadow-[0_1px_…]` empurra pra baixo → borda de baixo; `0 -1px` → borda de topo. Com o
    /// sinal dirigindo o lado, não há como os dois divergirem.
    fn bevel() -> Div {
        let p = palette();
        let overlay = div()
            .absolute()
            .top(px(-1.0))
            .left(px(-1.0))
            .right(px(-1.0))
            .bottom(px(-1.0))
            .rounded(px(ICON_RADIUS))
            .border_color(p.bevel.hsla());
        if p.bevel_dir > 0.0 {
            overlay.border_b_1()
        } else {
            overlay.border_t_1()
        }
    }

    fn render(self) -> Div {
        let p = palette();
        let cartelinha = self.variant == EmptyMediaVariant::Icon;

        // `flex shrink-0 items-center justify-center` é a base das DUAS variantes.
        let mut inner = div().flex().flex_none().items_center().justify_center();
        if cartelinha {
            inner = inner
                .relative()
                .size(px(ICON_BOX))
                .rounded(px(ICON_RADIUS))
                .border_1()
                .border_color(p.border.hsla())
                .bg(p.card.hsla())
                .text_color(p.fg.hsla())
                .shadow(icon_shadows());
        }

        if let Some(path) = self.icon {
            inner = inner.child(
                gpui::svg()
                    .path(path)
                    .size(px(ICON_SIZE))
                    .flex_none()
                    .text_color(p.fg.hsla()),
            );
        }
        for c in self.children {
            inner = inner.child(c);
        }
        // O bisel por último: é absoluto, então a ordem só decide quem pinta em cima. E a
        // cartelinha NÃO tem `overflow_hidden` — ele recortaria justamente o 1px que o overlay
        // projeta pra fora (armadilha conhecida, ver [`Self::bevel`]).
        if cartelinha {
            inner = inner.child(Self::bevel());
        }

        div()
            .relative()
            .mb(px(MEDIA_MB))
            // O leque ANTES da cartelinha da frente: sem z-index no GPUI, a ordem dos filhos é a
            // ordem de pintura, e é ela que põe as duas de trás atrás. O canvas é absoluto, então
            // não mede nada — como os `absolute` do original, o leque não muda o tamanho da mídia
            // (ele passa 6,2px pra fora de cada lado, e é assim que o original também faz).
            .when(cartelinha, |w| {
                w.child(
                    gpui::canvas(
                        |_, _, _| (),
                        |bounds: Bounds<Pixels>, _, window, _| paint_fan(bounds.origin, window),
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size(px(ICON_BOX)),
                )
            })
            .child(inner)
    }
}

// =================================================================================================
// EmptyHeader
// =================================================================================================

/// O **header** do estado vazio: mídia, título e descrição, centrados e presos a
/// [`MAX_W`] de largura.
///
/// No original são três componentes (`EmptyHeader` + `EmptyTitle` + `EmptyDescription`); aqui o
/// header é dono dos dois textos, o que transforma a adjacência do `mt-1` numa decisão local. Ver o
/// doc do módulo.
#[derive(Default)]
pub struct EmptyHeader {
    media: Option<EmptyMedia>,
    title: Option<SharedString>,
    description: Option<SharedString>,
}

impl EmptyHeader {
    pub fn new() -> Self {
        Self::default()
    }

    /// A mídia acima do título.
    pub fn media(mut self, media: EmptyMedia) -> Self {
        self.media = Some(media);
        self
    }

    /// O título — `text-xl` (20/28), semibold, na cor `--foreground`.
    ///
    /// ⚠️ A referência também pede `font-heading`, uma FAMÍLIA de fonte que não estava nos tokens
    /// resolvidos. Aqui o título sai na fonte herdada. Ver o doc do módulo.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// A descrição — `text-sm` (14/20), na cor `--muted-foreground`. Ganha 4px de respiro no topo
    /// **se** houver título ([`DESC_MT`]).
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Se o header não tem nada pra mostrar (evita renderizar uma caixa vazia).
    fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.media.as_ref().is_none_or(EmptyMedia::is_empty)
    }

    fn render(self) -> Div {
        let p = palette();
        let tem_titulo = self.title.is_some();

        // `flex max-w-sm flex-col items-center text-center` — sem `gap`: o respiro entre as linhas
        // vem da entrelinha do texto e do `mb-6` da mídia, exatamente como no original.
        let mut el = div()
            .flex()
            .flex_col()
            .items_center()
            .max_w(px(MAX_W))
            .text_center();

        if let Some(m) = self.media {
            if !m.is_empty() {
                el = el.child(m.render());
            }
        }
        if let Some(t) = self.title {
            el = el.child(
                div()
                    .text_size(px(TITLE_SIZE))
                    .line_height(px(TITLE_LINE))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(p.fg.hsla())
                    .child(t),
            );
        }
        if let Some(d) = self.description {
            el = el.child(
                div()
                    // A adjacência `[[data-slot=empty-title]+&]:mt-1` do original, como condição
                    // local: 4px de respiro SÓ quando existe título logo acima.
                    .when(tem_titulo, |d| d.mt(px(DESC_MT)))
                    .text_size(px(TEXT_SIZE))
                    .line_height(px(TEXT_LINE))
                    .text_color(p.muted_fg.hsla())
                    .child(d),
            );
        }
        el
    }
}

// =================================================================================================
// EmptyContent
// =================================================================================================

/// O bloco de **conteúdo** do estado vazio: normalmente os botões de ação, empilhados e centrados.
///
/// `flex w-full min-w-0 max-w-sm flex-col items-center gap-4 text-sm` — mesma largura máxima do
/// [`EmptyHeader`], pra os dois blocos lerem como uma coluna só.
#[derive(Default)]
pub struct EmptyContent {
    children: Vec<AnyElement>,
}

impl EmptyContent {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adiciona um elemento. A ordem de chamada é a ordem na tela.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Se não há nada pra mostrar — e aí o [`Empty`] não renderiza o bloco, pra um
    /// `.content(EmptyContent::new())` distraído não somar 24px de `gap` do nada.
    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    fn render(self) -> Div {
        let mut el = div()
            .flex()
            .flex_col()
            .items_center()
            .w_full()
            // `min-w-0`: sem ele, um filho largo (um botão com rótulo comprido) empurraria a coluna
            // além do `max-w-sm` em vez de quebrar.
            .min_w(px(0.0))
            .max_w(px(MAX_W))
            .gap(px(CONTENT_GAP))
            .text_size(px(TEXT_SIZE))
            .line_height(px(TEXT_LINE));
        for c in self.children {
            el = el.child(c);
        }
        el
    }
}

// =================================================================================================
// Empty
// =================================================================================================

/// O **estado vazio**: o bloco centrado que ocupa o lugar de uma lista sem nada dentro. Ver o doc
/// do módulo.
///
/// A raiz é `flex min-w-0 flex-1 flex-col items-center justify-center gap-6 px-6 py-20` — sem
/// superfície própria: nem fundo, nem borda, nem sombra. Ela se planta no espaço que o container
/// der (`flex-1`) e centra o conteúdo nos dois eixos.
#[derive(IntoElement, Default)]
pub struct Empty {
    header: Option<EmptyHeader>,
    content: Option<EmptyContent>,
}

impl Empty {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mídia, título e descrição.
    pub fn header(mut self, header: EmptyHeader) -> Self {
        self.header = Some(header);
        self
    }

    /// O bloco de ações abaixo do header.
    pub fn content(mut self, content: EmptyContent) -> Self {
        self.content = Some(content);
        self
    }
}

impl RenderOnce for Empty {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let p = palette();

        let mut el = div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            // `min-w-0 flex-1`: a raiz OCUPA o espaço que o container der (numa linha, a largura;
            // numa coluna, a altura) e pode encolher abaixo do conteúdo. Sem `w_full`, como no
            // original: em container de bloco a largura já é a cheia, e em container flex quem a dá
            // é o `stretch`/`flex-1`.
            .min_w(px(0.0))
            .flex_1()
            .gap(px(GAP))
            .px(px(PAD_X))
            .py(px(PAD_Y))
            .text_center()
            // Superset declarado (ver o doc do módulo): o GPUI não tem `inherit`.
            .text_color(p.fg.hsla());

        if let Some(h) = self.header {
            if !h.is_empty() {
                el = el.child(h.render());
            }
        }
        if let Some(c) = self.content {
            if !c.is_empty() {
                el = el.child(c.render());
            }
        }
        el
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável ("this assertion has a constant
// value"), presumindo que quem escreveu quis testar algo variável. Aqui é o contrário: travar o
// valor que veio da referência É o propósito destes testes — eles são a documentação executável de
// quanto mede cada coisa. Mesma decisão em `card.rs`, `frame.rs` e `dialog.rs`.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::color::opaque;

    /// **A armadilha da entrelinha, travada em número.** Este componente não tem uma única altura
    /// fixa: cada `text-*` é um par `(fonte, linha)` do Tailwind, e sem fixar a linha o GPUI usa a
    /// razão de ouro. O teste guarda os dois pares E a diferença que eles evitam.
    #[test]
    fn entrelinhas_sao_o_par_do_tailwind() {
        assert_eq!((TITLE_SIZE, TITLE_LINE), (20.0, 28.0), "o par do text-xl");
        assert_eq!((TEXT_SIZE, TEXT_LINE), (14.0, 20.0), "o par do text-sm");

        // O que aconteceria sem fixar: `relative(1.618_034)`, o default do GPUI.
        const RAZAO_DE_OURO: f32 = 1.618_034;
        let titulo_solto = TITLE_SIZE * RAZAO_DE_OURO;
        let desc_solta = TEXT_SIZE * RAZAO_DE_OURO;
        assert!(
            titulo_solto > TITLE_LINE + 4.0,
            "o título sairia {titulo_solto:.1}px em vez de {TITLE_LINE}"
        );
        assert!(
            desc_solta > TEXT_LINE + 2.0,
            "a descrição sairia {desc_solta:.1}px em vez de {TEXT_LINE}"
        );
    }

    /// A altura do header é a **soma das partes**, e é assim que ela é medida na janela em
    /// `tests_de_janela::altura_e_a_soma_das_partes`. Aqui fica a aritmética explícita, pra quem
    /// mudar uma constante ver de onde vem o número.
    #[test]
    fn altura_do_header_com_cartelinha_titulo_e_descricao() {
        let header = ICON_BOX + MEDIA_MB + TITLE_LINE + DESC_MT + TEXT_LINE;
        assert_eq!(header, 112.0, "36 + 24 + 28 + 4 + 20");
    }

    // --- O leque ---------------------------------------------------------------------------------

    /// Os quatro cantos de uma cartelinha de trás, na ordem cima-esquerda, cima-direita,
    /// baixo-direita, baixo-esquerda. São os cantos da CAIXA (sem o arredondamento), que é o que a
    /// transformação do CSS move.
    fn cantos_do_leque(direita: bool) -> [(f32, f32); 4] {
        [(0.0, 0.0), (ICON_BOX, 0.0), (ICON_BOX, ICON_BOX), (0.0, ICON_BOX)].map(|(u, v)| {
            let p = fan_point(direita, u, v);
            (f32::from(p.x), f32::from(p.y))
        })
    }

    /// Os números do `cva` do original, em `scale-84 ∓rotate-10 ∓translate-x-0.5 bottom-px`.
    #[test]
    fn leque_tem_os_valores_da_referencia() {
        assert_eq!(FAN_SCALE, 0.84, "scale-84");
        assert_eq!(FAN_ROT, 10.0, "∓rotate-10");
        assert_eq!(FAN_DX, 2.0, "∓translate-x-0.5 = 0.125rem");
        assert_eq!(FAN_BOTTOM, 1.0, "bottom-px");
    }

    /// **O número que prova que a transformação está certa.** Girar a caixa de `36 · 0,84` em 10° em
    /// torno do canto de BAIXO levanta o canto oposto `30,24 · (sen 10° + cos 10°) = 35,03`px — que é
    /// exatamente a altura da mídia menos o `bottom-px`. Ou seja: o desenho do coss é afinado pra que
    /// a ponta das cartelinhas de trás encoste na aresta de cima da cartelinha da frente.
    ///
    /// É isso que torna este teste discriminante em vez de tautológico: ele não confere uma conta
    /// contra ela mesma, confere contra um alinhamento que só fecha se a ORDEM da transformação
    /// (escala, giro, deslocamento) e a ORIGEM do giro (o canto de baixo) estiverem as duas certas.
    /// Trocar `FAN_ROT` por 12, `FAN_SCALE` por 0,8 ou a origem pro centro tira o canto do topo.
    #[test]
    fn a_ponta_do_leque_encosta_no_topo_da_midia() {
        for direita in [false, true] {
            // A ponta é o canto de cima do lado OPOSTO à origem do giro.
            let (_, y) = cantos_do_leque(direita)[if direita { 0 } else { 1 }];
            assert!(
                y.abs() < 0.05,
                "{}: a ponta caiu em y={y:.3}, e a aresta de cima da mídia é y=0",
                if direita { "direita" } else { "esquerda" },
            );
        }
    }

    /// O `bottom-px`: o canto que é a origem do giro é o único que a transformação não move, e ele
    /// tem que cair 1px acima do rodapé da mídia.
    #[test]
    fn a_origem_do_giro_e_o_canto_de_baixo_um_pixel_acima_do_rodape() {
        let esquerda = cantos_do_leque(false)[3]; // baixo-esquerda
        let direita = cantos_do_leque(true)[2]; // baixo-direita
        assert_eq!(esquerda.1, ICON_BOX - FAN_BOTTOM);
        assert_eq!(direita.1, ICON_BOX - FAN_BOTTOM);
        // O `∓translate-x-0.5` é o ÚNICO deslocamento horizontal que sobra na origem.
        assert_eq!(esquerda.0, -FAN_DX);
        assert_eq!(direita.0, ICON_BOX + FAN_DX);
    }

    /// **É o GIRO que faz o leque existir, não o deslocamento de 2px.** Sem girar, uma cartelinha de
    /// `scale-84` mede 30,24 contra 36 e ficaria INTEIRA atrás da da frente, sobrando 2px de apara —
    /// foi esse o argumento que deixou o leque de fora na primeira passada deste módulo.
    ///
    /// O teste guarda os DOIS números, porque eles não são o mesmo: o canto da CAIXA vai a 7,25px pra
    /// fora, mas o canto é arredondado, e o ponto mais externo do contorno é o do arco de cima — 6,19.
    /// É o segundo que aparece na tela, e ele fecha com a medição da janela: 48,0px de largura total
    /// contra os 36 da cartelinha da frente (o resto do 48,37 teórico é o AA da borda a 6% de alfa,
    /// abaixo do limiar de brilho da medição).
    #[test]
    fn o_giro_e_o_que_faz_o_leque_aparecer() {
        let sobra_sem_girar = FAN_DX; // a caixa encolhida cabe inteira; sobra só o translate

        let canto = -cantos_do_leque(false)
            .iter()
            .map(|c| c.0)
            .fold(f32::INFINITY, f32::min);
        assert!(
            (canto - 7.25).abs() < 0.05,
            "o canto da caixa sobra {canto:.2}px pra fora, esperado ~7,25"
        );

        // O extremo VISÍVEL: o arco de cima-esquerda tem centro no ponto `(raio, raio)` da caixa, e o
        // giro é rígido, então o raio dele na tela é o mesmo `ICON_RADIUS · FAN_SCALE`.
        let centro = fan_point(false, ICON_RADIUS, ICON_RADIUS);
        let visivel = -(f32::from(centro.x) - ICON_RADIUS * FAN_SCALE);
        assert!(
            (visivel - 6.19).abs() < 0.05,
            "o contorno sobra {visivel:.2}px pra fora, esperado ~6,19"
        );

        assert!(visivel < canto, "o arredondamento só pode encurtar a sobra");
        assert!(visivel > 3.0 * sobra_sem_girar);
    }

    /// As duas cartelinhas são espelho uma da outra em torno do meio da mídia — é o que
    /// `origin-bottom-left`/`right` com sinais opostos de giro e de deslocamento significa. Um sinal
    /// trocado em qualquer um dos três passos quebra a simetria.
    #[test]
    fn o_leque_e_simetrico() {
        for (u, v) in [(0.0, 0.0), (ICON_BOX, 0.0), (12.0, 30.0), (8.0, 36.0)] {
            let e = fan_point(false, u, v);
            let d = fan_point(true, ICON_BOX - u, v);
            let (ex, ey) = (f32::from(e.x), f32::from(e.y));
            let (dx, dy) = (f32::from(d.x), f32::from(d.y));
            assert!(
                (ex + dx - ICON_BOX).abs() < 1e-3 && (ey - dy).abs() < 1e-3,
                "({u},{v}): esquerda ({ex:.3},{ey:.3}) não espelha direita ({dx:.3},{dy:.3})"
            );
        }
    }

    /// O leque não passa do rodapé da mídia: ele é `bottom-px`, e o giro só levanta pontos. Se
    /// passasse, o `mb-6` da mídia encurtaria na prática e o título subiria por cima do desenho.
    #[test]
    fn o_leque_nao_desce_abaixo_da_midia() {
        for direita in [false, true] {
            for (_, y) in cantos_do_leque(direita) {
                assert!(
                    y <= ICON_BOX - FAN_BOTTOM + 1e-3,
                    "canto em y={y:.2} passa do rodapé da caixa da cartelinha de trás"
                );
            }
        }
    }

    /// A aresta do bisel do leque segue o **sinal** do deslocamento do tema, igual à da cartelinha da
    /// frente (ver [`EmptyMedia::bevel`]): `0 1px` desce → base; `0 -1px` sobe → topo. Com o sinal
    /// dirigindo o lado, não há como as duas divergirem.
    #[test]
    fn o_bisel_do_leque_segue_o_sinal_do_tema() {
        let aresta = |dir: f32| {
            if dir > 0.0 {
                FanEdge::Bottom
            } else {
                FanEdge::Top
            }
        };
        assert!(aresta(EMPTY_LIGHT.bevel_dir) == FanEdge::Bottom);
        assert!(aresta(EMPTY_DARK.bevel_dir) == FanEdge::Top);
    }

    /// Os respiros e as larguras, na escala do Tailwind (1 unidade = 4px).
    #[test]
    fn respiros_seguem_a_escala_do_tailwind() {
        assert_eq!(PAD_X, 24.0, "px-6");
        assert_eq!(PAD_Y, 80.0, "md:py-20 — o ramo de desktop; py-12 seria 48");
        assert_eq!(GAP, 24.0, "gap-6");
        assert_eq!(MEDIA_MB, 24.0, "mb-6");
        assert_eq!(CONTENT_GAP, 16.0, "gap-4");
        assert_eq!(DESC_MT, 4.0, "mt-1");
        assert_eq!(MAX_W, 384.0, "max-w-sm = 24rem");

        // O respiro vertical é MAIOR que o horizontal: é o que faz o bloco flutuar no meio do painel
        // em vez de parecer um cabeçalho encostado no topo.
        assert!(PAD_Y > PAD_X);
    }

    /// A cartelinha: 36 de lado, raio 8, ícone de 18 — e o ícone tem que caber com folga nos dois
    /// eixos, senão ele encosta na borda e o `justify-center` não tem o que centrar.
    #[test]
    fn cartelinha_tem_a_geometria_do_size_9() {
        assert_eq!(ICON_BOX, 36.0, "size-9");
        assert_eq!(ICON_RADIUS, 8.0, "rounded-md = calc(--radius - 2px) = 10 - 2");
        assert_eq!(ICON_SIZE, 18.0, "size-4.5");
        assert!(
            ICON_SIZE + 2.0 * 1.0 < ICON_BOX,
            "o ícone de {ICON_SIZE} tem que caber na caixa de {ICON_BOX} com a borda de 1px"
        );
        // O raio é MENOR que meio lado: com 18 a cartelinha viraria um círculo.
        assert!(ICON_RADIUS < ICON_BOX / 2.0);
    }

    /// **O raio do overlay de bisel acompanha a BORDER box** — e é por isso que ele NÃO é
    /// `ICON_RADIUS - 1`. O `calc(--radius-md - 1px)` do original vale pra um pseudo-elemento na
    /// padding box; o nosso vive em `inset: -1px`. Mesma escolha do [`crate::card`] e do
    /// [`crate::frame`], e a armadilha que já enganou uma vez nesta base.
    #[test]
    fn raio_do_bisel_e_o_da_superficie() {
        // Se alguém trocar o raio do overlay pelo `− 1` da referência, esta é a diferença.
        let raio_da_padding_box = ICON_RADIUS - 1.0;
        assert_eq!(raio_da_padding_box, 7.0, "é o número do original, na OUTRA caixa");
        assert_ne!(
            raio_da_padding_box, ICON_RADIUS,
            "o overlay usa {ICON_RADIUS}, não {raio_da_padding_box}"
        );
    }

    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, a mesma decisão já vigente
    /// no `Input`, no `Card`, no `Frame` e no `Button`, porque a 6% o filete é imperceptível no
    /// nosso fundo. Este teste existe pra o desvio ser uma decisão registrada e não uma deriva: se
    /// alguém "corrigir" pra 6% achando que é erro de porte, ele falha e aponta pra cá.
    ///
    /// O tema CLARO segue fiel (preto 4%) — o filete ali é PRETO sobre superfície clara, e nesse
    /// caso o valor da referência funciona.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = EMPTY_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%); veio {claro}"
        );

        let escuro = EMPTY_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    /// **A direção do bisel sai do sinal**, e os dois temas apontam pra lados opostos: escurece a
    /// base no claro, clareia o topo no escuro. Se os dois apontassem pro mesmo lado, um dos temas
    /// ficaria com o relevo invertido.
    #[test]
    fn bisel_troca_de_lado_entre_os_temas() {
        assert!(EMPTY_LIGHT.bevel_dir > 0.0, "claro: `0 1px` desce → borda de baixo");
        assert!(EMPTY_DARK.bevel_dir < 0.0, "escuro: `0 -1px` sobe → borda de topo");
    }

    /// As duas paletas decodificam com os canais no lugar e com a intenção perceptual certa. É o
    /// teste que pega a confusão de convenção `0xRRGGBB` vs `0xRRGGBBAA` — que já custou três bugs
    /// visíveis nesta base, todos sem erro de compilação.
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        // Fundo da cartelinha: branco opaco no claro, quase-preto opaco no escuro, os dois NEUTROS.
        for (nome, c) in [("claro", EMPTY_LIGHT.card), ("escuro", EMPTY_DARK.card)] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: fundo da cartelinha é OPACO");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: fundo neutro — se r≠g≠b, o valor foi lido deslocado"
            );
        }
        let card_claro: gpui::Rgba = EMPTY_LIGHT.card.hsla().into();
        let card_escuro: gpui::Rgba = EMPTY_DARK.card.hsla().into();
        assert_eq!((card_claro.r, card_claro.g, card_claro.b), (1.0, 1.0, 1.0));
        assert!(card_escuro.r < 0.2, "cartelinha do tema escuro é escura");

        // Texto: quase preto no claro, quase branco no escuro, os dois neutros e opacos. E o
        // secundário fica ENTRE o texto e o fundo — se saísse igual ao título, a descrição perderia
        // a hierarquia.
        for (nome, c) in [("claro", EMPTY_LIGHT.fg), ("escuro", EMPTY_DARK.fg)] {
            let c: gpui::Rgba = c.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: texto opaco");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: texto neutro"
            );
        }
        let fg_claro: gpui::Rgba = EMPTY_LIGHT.fg.hsla().into();
        let fg_escuro: gpui::Rgba = EMPTY_DARK.fg.hsla().into();
        assert!(fg_claro.r < 0.2, "texto do tema claro é escuro");
        assert!(fg_escuro.r > 0.8, "texto do tema escuro é claro");

        let muted_claro: gpui::Rgba = EMPTY_LIGHT.muted_fg.hsla().into();
        let muted_escuro: gpui::Rgba = EMPTY_DARK.muted_fg.hsla().into();
        assert_eq!((muted_claro.a, muted_escuro.a), (1.0, 1.0));
        assert!(
            muted_claro.r > fg_claro.r && muted_claro.r < card_claro.r,
            "no claro o secundário fica entre o texto e o fundo"
        );
        assert!(
            muted_escuro.r < fg_escuro.r && muted_escuro.r > card_escuro.r,
            "no escuro o secundário fica entre o texto e o fundo"
        );

        // Borda, sombra e bisel são TRANSLÚCIDOS — é isso que os deixa funcionar sobre qualquer
        // fundo.
        for (nome, c) in [
            ("borda claro", EMPTY_LIGHT.border),
            ("borda escuro", EMPTY_DARK.border),
            ("sombra claro", EMPTY_LIGHT.shadow),
            ("sombra escuro", EMPTY_DARK.shadow),
            ("bisel claro", EMPTY_LIGHT.bevel),
            ("bisel escuro", EMPTY_DARK.bevel),
        ] {
            assert!(c.alpha() < 1.0, "{nome} tem que ser translúcido");
            assert!(c.alpha() > 0.0, "{nome} não pode ser invisível");
        }
    }

    /// As duas sombras do `shadow-sm/5`, com a geometria do Tailwind v4 — ⚠️ **valor deduzido**, ver
    /// o doc de [`icon_shadows`]. A segunda tem `spread` NEGATIVO: é ela que aperta a sombra contra
    /// a base da cartelinha em vez de espalhá-la.
    #[test]
    fn sombra_da_cartelinha_e_o_shadow_sm() {
        theme::set_theme(theme::ThemeMode::Light);
        let sombras = icon_shadows();
        assert_eq!(sombras.len(), 2, "`--shadow-sm` do Tailwind v4 tem DUAS camadas");

        assert_eq!(sombras[0].offset.y, px(1.0), "as duas descem 1px");
        assert_eq!(sombras[0].blur_radius, px(3.0));
        assert_eq!(sombras[0].spread_radius, px(0.0));

        assert_eq!(sombras[1].offset.y, px(1.0));
        assert_eq!(sombras[1].blur_radius, px(2.0));
        assert_eq!(sombras[1].spread_radius, px(-1.0), "spread NEGATIVO");

        for s in &sombras {
            assert_eq!(s.offset.x, px(0.0), "nenhuma desloca na horizontal");
            assert!(s.color.a < 1.0, "o `/5` deixa as duas a 5%");
        }
    }

    /// Peça vazia não vira caixa. Sem isto, um `.media(EmptyMedia::new())` empurraria o título 24px
    /// pra baixo do nada, e um `.content(EmptyContent::new())` somaria 24px de `gap`.
    #[test]
    fn pecas_vazias_sao_detectadas() {
        assert!(EmptyMedia::new().is_empty());
        assert!(!EmptyMedia::new().child(div()).is_empty());
        assert!(!EmptyMedia::icon("x.svg").is_empty());
        // A cartelinha é desenho PRÓPRIO: ela aparece mesmo sem nada dentro, como no original.
        assert!(!EmptyMedia::new()
            .variant(EmptyMediaVariant::Icon)
            .is_empty());
        // E `variant(Default)` desfaz a cartelinha de um `icon()`, mas o ícone continua lá.
        assert!(!EmptyMedia::icon("x.svg")
            .variant(EmptyMediaVariant::Default)
            .is_empty());

        assert!(EmptyContent::new().is_empty());
        assert!(!EmptyContent::new().child(div()).is_empty());

        assert!(EmptyHeader::new().is_empty());
        assert!(!EmptyHeader::new().title("x").is_empty());
        assert!(!EmptyHeader::new().description("x").is_empty());
        assert!(!EmptyHeader::new().media(EmptyMedia::icon("x.svg")).is_empty());
        // Um header cuja única peça é uma mídia VAZIA também é vazio — senão ele renderizaria uma
        // caixa de 24px de `mb-6` e nada mais.
        assert!(EmptyHeader::new().media(EmptyMedia::new()).is_empty());
    }

    /// A variante default é a **sem moldura** — é o que o `defaultVariants` do `cva` diz.
    #[test]
    fn variante_default_e_a_sem_moldura() {
        assert_eq!(EmptyMediaVariant::default(), EmptyMediaVariant::Default);
        assert_eq!(EmptyMedia::new().variant, EmptyMediaVariant::Default);
        assert_eq!(EmptyMedia::icon("x.svg").variant, EmptyMediaVariant::Icon);
    }

    /// `opaque` é a única ponte entre as convenções de cor, e a paleta daqui já nasce em
    /// `0xRRGGBBAA` — então nenhum token de 6 dígitos do tema entra sem passar por ele.
    #[test]
    fn tokens_do_tema_entram_por_opaque() {
        theme::set_theme(theme::ThemeMode::Dark);
        let elevado = opaque(theme::TEXT_MUTED());
        assert_eq!(elevado.alpha(), 1.0);
    }
}

/// Os testes que precisam de uma JANELA de verdade.
///
/// Num componente cuja altura é 100% dirigida por conteúdo, é o **único** lugar onde a aritmética
/// das constantes é conferida contra a MEDIDA. Um teste puro não vê entrelinha: ele vê o número que
/// eu escrevi na constante. O que engordou o header do `Frame` em 6px e os itens do `Menu` em 2,6px
/// só apareceu aqui.
#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests_de_janela {
    use super::*;
    use gpui::{canvas, Bounds, Context, Pixels, Render, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Largura da janela do harness. Bem acima do [`MAX_W`] de propósito: é o que prova que o header
    /// para nos 384px em vez de esparramar, e que o bloco fica CENTRADO na sobra.
    const LARGURA: f32 = 600.0;

    /// Altura do filho que faz o papel do botão de ação no [`EmptyContent`].
    const ACAO_H: f32 = 32.0;

    #[derive(Default)]
    struct Medidas {
        /// A caixa do [`Empty`] inteiro.
        raiz: Option<Bounds<Pixels>>,
        /// A PADDING box da cartelinha (a superfície recuada 1px de cada lado pela borda).
        cartelinha: Option<Bounds<Pixels>>,
        /// A caixa do filho do [`EmptyContent`] que faz o papel do botão de ação.
        acao: Option<Bounds<Pixels>>,
    }

    /// Uma sonda: um `canvas` que só mede e não pinta nada.
    fn sonda(
        medidas: &Rc<RefCell<Medidas>>,
        campo: fn(&mut Medidas, Bounds<Pixels>),
    ) -> impl IntoElement {
        let m = medidas.clone();
        canvas(
            move |bounds, _w, _cx| campo(&mut m.borrow_mut(), bounds),
            |_, _, _, _| {},
        )
        .size_full()
    }

    /// O harness: um container de BLOCO (o default do GPUI) com largura fixa. De bloco de propósito
    /// — num container flex de coluna o `flex-1` da raiz a esticaria até a altura da janela, e a
    /// altura medida deixaria de ser a do conteúdo, que é justamente o que se quer medir.
    struct Medidor(Rc<RefCell<Medidas>>);

    impl Render for Medidor {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let m = self.0.clone();
            div().w(px(LARGURA)).child(
                div()
                    .relative()
                    // A sonda da raiz é ABSOLUTA e mede o wrapper, cuja altura é a do `Empty` (é o
                    // único filho em fluxo).
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .child(sonda(&m, |md, b| md.raiz = Some(b))),
                    )
                    .child(
                        Empty::new()
                            .header(
                                EmptyHeader::new()
                                    .media(
                                        EmptyMedia::new()
                                            .variant(EmptyMediaVariant::Icon)
                                            // A sonda entra COMO o ícone: a cartelinha tem tamanho
                                            // fixo, então ela não mexe no layout.
                                            .child(sonda(&m, |md, b| md.cartelinha = Some(b))),
                                    )
                                    .title("Nenhum projeto")
                                    .description("Crie um projeto pra começar."),
                            )
                            .content(
                                EmptyContent::new().child(
                                    div()
                                        .relative()
                                        .w_full()
                                        .h(px(ACAO_H))
                                        .child(sonda(&m, |md, b| md.acao = Some(b))),
                                ),
                            ),
                    ),
            )
        }
    }

    /// Abre a janela, deixa o layout assentar e devolve as medidas.
    fn medir(cx: &mut TestAppContext) -> Medidas {
        let medidas = Rc::new(RefCell::new(Medidas::default()));
        let m = medidas.clone();
        let window = cx.add_window(move |_window, _cx| Medidor(m));
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let out = medidas.borrow();
        Medidas {
            raiz: out.raiz,
            cartelinha: out.cartelinha,
            acao: out.acao,
        }
    }

    /// **A altura é a soma das partes — e é aqui que a entrelinha é pega.**
    ///
    /// `py-20` × 2 + header + `gap-6` + a ação:
    /// `80 + (36 + 24 + 28 + 4 + 20) + 24 + 32 + 80` = **328**.
    ///
    /// Sem `.line_height()` no título e na descrição, o GPUI usa `relative(1.618_034)` e as duas
    /// linhas somam ~55 em vez de 48 — a caixa sai com ~335. É exatamente o defeito que engordou o
    /// header do `Frame` e os itens do `Menu`, e num componente sem nenhuma altura fixa ele
    /// contamina TUDO.
    #[gpui::test]
    fn altura_e_a_soma_das_partes(cx: &mut TestAppContext) {
        let medidas = medir(cx);
        let raiz = medidas.raiz.expect("a sonda da raiz pintou");

        let header = ICON_BOX + MEDIA_MB + TITLE_LINE + DESC_MT + TEXT_LINE;
        let esperado = 2.0 * PAD_Y + header + GAP + ACAO_H;
        assert_eq!(esperado, 328.0, "a aritmética que o número mede");
        assert_eq!(
            raiz.size.height,
            px(esperado),
            "a altura MEDIDA tem que ser a soma das partes — se veio maior, alguma entrelinha \
             ficou solta na razão de ouro do GPUI"
        );
        assert_eq!(raiz.size.width, px(LARGURA), "a raiz ocupa o container");
    }

    /// **A cartelinha mede 36×36 e fica centrada**, com o topo a `py-20` da borda de cima.
    ///
    /// A sonda mede a PADDING box (a borda de 1px a encolhe e a desloca — é a regra do bloco
    /// contêiner de filho absoluto/`size-full`, medida no `checkbox`), então 34×34 aqui prova 36×36
    /// de border box.
    ///
    /// **E é este teste que garante que o leque não mede.** As duas cartelinhas de trás passam 6,2px
    /// pra fora de cada lado, e o desenho delas vive num [`gpui::canvas`] absoluto; se ele entrasse no
    /// fluxo, a mídia mediria mais que 36 e o `items-center` centraria pelo desenho em vez de pela
    /// cartelinha da frente — que é o que o original faz com `position: absolute`.
    #[gpui::test]
    fn cartelinha_mede_36_e_fica_centrada(cx: &mut TestAppContext) {
        let medidas = medir(cx);
        let raiz = medidas.raiz.expect("a sonda da raiz pintou");
        let cart = medidas.cartelinha.expect("a sonda da cartelinha pintou");

        assert_eq!(
            (cart.size.width, cart.size.height),
            (px(ICON_BOX - 2.0), px(ICON_BOX - 2.0)),
            "a padding box é 34×34, ou seja a border box é {ICON_BOX}×{ICON_BOX}"
        );

        // O topo: `py-20` da borda de cima da raiz, mais o 1px de borda da cartelinha.
        assert_eq!(
            cart.origin.y,
            raiz.origin.y + px(PAD_Y + 1.0),
            "a mídia é a primeira coisa depois do respiro de topo"
        );

        // E o centro dela coincide com o centro da raiz: é o `items-center` da coluna.
        let centro_cart = cart.origin.x + cart.size.width / 2.0;
        let centro_raiz = raiz.origin.x + raiz.size.width / 2.0;
        assert!(
            (centro_cart - centro_raiz).abs() < px(0.5),
            "cartelinha centrada em {centro_cart:?}, raiz em {centro_raiz:?}"
        );
    }

    /// **O conteúdo para no `max-w-sm`** e não acompanha a largura do container.
    ///
    /// É o que mantém as ações na mesma coluna estreita do header em vez de um botão `w-full`
    /// atravessando um painel de 600px.
    #[gpui::test]
    fn conteudo_para_no_max_w_sm(cx: &mut TestAppContext) {
        let medidas = medir(cx);
        let raiz = medidas.raiz.expect("a sonda da raiz pintou");
        let acao = medidas.acao.expect("a sonda da ação pintou");

        assert_eq!(
            acao.size.width,
            px(MAX_W),
            "o `w-full` do filho para no `max-w-sm` do EmptyContent"
        );
        assert!(
            acao.size.width < raiz.size.width - px(2.0 * PAD_X),
            "e sobra espaço dentro do respiro horizontal"
        );

        // A ação é a ÚLTIMA coisa antes do respiro de baixo.
        assert_eq!(
            acao.origin.y + px(ACAO_H) + px(PAD_Y),
            raiz.origin.y + raiz.size.height,
            "o `py-20` de baixo fecha a caixa"
        );
    }
}

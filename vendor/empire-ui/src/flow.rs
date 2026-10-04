//! `flow` — o diagrama de nós do [kumo], com layout automático, arestas e ramos paralelos.
//!
//! [kumo]: https://github.com/cloudflare/kumo/tree/main/packages/kumo/src/components/flow
//!
//! Um fluxo se declara como uma lista de peças, e o componente resolve o resto: onde cada nó cai,
//! quem liga em quem, e o desenho de cada seta.
//!
//! ```ignore
//! Flow::new()
//!     .node(FlowNode::new().child("Requisição"))
//!     .parallel(
//!         FlowParallel::new()
//!             .node(FlowNode::new().child("Cache"))
//!             .node(FlowNode::new().child("Origem")),
//!     )
//!     .node(FlowNode::new().child("Resposta"))
//! ```
//!
//! Isso desenha o **leque**: a requisição abre nas duas pontas e as duas fecham na resposta. É o
//! desenho que o componente existe pra mostrar, e a regra que o gera está em
//! [`crate::flow_layout`], junto com toda a aritmética.
//!
//! # A divisão com o `flow_layout`
//!
//! Este módulo **não decide posição nem aresta nem curva**. Isso é tudo do
//! [`crate::flow_layout`], que é puro e testado sem janela. Aqui ficam só as três coisas que
//! precisam de janela: medir os nós, montar os `div`, e pintar os caminhos. Se você está
//! escrevendo aritmética neste arquivo, ela provavelmente está no lugar errado.
//!
//! # A medição — a decisão central do porte
//!
//! O kumo mede o DOM (`getBoundingClientRect` + `ResizeObserver`) porque o conteúdo de um nó é
//! arbitrário e o tamanho dele não se sabe de antemão. No GPUI não há DOM, e havia três saídas:
//!
//! 1. **Exigir tamanho declarado** por quem monta o diagrama. Custo: o componente deixa de ser o
//!    do kumo — cada nó passa a carregar dois números que o conteúdo dele já sabe, e um rótulo mais
//!    longo que a largura declarada é cortado em silêncio.
//! 2. **Sondar com [`gpui::canvas`]** (o jeito que o resto desta lib mede) e posicionar no frame
//!    seguinte. Custo: **um frame errado** por mudança de tamanho.
//! 3. **Medir de verdade dentro do próprio frame**, com um [`gpui::Element`] à mão.
//!
//! Escolhemos (3), e o número que sustenta a escolha é **zero frames errados**: o diagrama sai
//! correto no primeiro. Travado pelo teste `diagrama_sai_correto_no_primeiro_frame`, que confere as
//! posições **e** que a view renderizou exatamente uma vez.
//!
//! A (2) foi tentada primeiro e **não funciona**, por um motivo que vale registrar: pra a medida
//! virar posição é preciso um frame novo, e o GPUI **recusa agendar frame de dentro de um desenho**.
//! [`gpui::Window::refresh`] é `if not_drawing()` por dentro; `App::notify` chega em
//! `invalidate_view`, que só marca a janela suja `if draw_phase == None`; e
//! `Window::request_animation_frame` enfileira em `next_frame_callbacks`, drenados só pelo laço de
//! frame da plataforma — que não existe numa janela de teste. **Os três falham em silêncio**, sem
//! erro e sem aviso, e o sintoma é o diagrama empilhado em (0, 0) pra sempre: a medida é gravada e
//! o frame que a usaria nunca vem.
//!
//! ## Como as três fases do kumo caem num frame só
//!
//! É a ordem natural do [`gpui::Element`], e cada fase da spec do kumo cai numa fase do GPUI:
//!
//! | fase da spec | fase do GPUI | o que acontece |
//! |---|---|---|
//! | medição | `request_layout` | cada nó é medido por [`gpui::AnyElement::layout_as_root`], que roda um layout independente e devolve o tamanho natural dele |
//! | layout | `request_layout` | com os tamanhos em mão, [`crate::flow_layout`] deriva posições e o retângulo — que é o tamanho que o diagrama devolve pro pai |
//! | render | `prepaint` + `paint` | cada nó é solto na posição dele por [`gpui::AnyElement::prepaint_at`], e as arestas são pintadas por cima |
//!
//! Ou seja: **não há estado entre frames**. Nada de sonda guardada, nada de mapa indexado por id,
//! nada pra invalidar. Medida nova entra no mesmo frame em que o conteúdo muda, o que também
//! substitui o `ResizeObserver` do original sem precisar de observador nenhum.
//!
//! ## O caso das âncoras, que é o único que precisa das duas passadas
//!
//! Uma [`FlowNode::anchor`] desloca a ponta da seta pro meio dela, então o número que ela produz é
//! a posição de um filho **por dentro** do nó — que o `layout_as_root` calcula mas não expõe.
//!
//! A saída é que **posição de nó não depende de âncora**: [`crate::flow_layout::compute_positions`]
//! só olha largura e altura. Só o endereço das pontas da seta depende, e seta é pintada no `paint`.
//! Então as âncoras se medem por sonda ([`gpui::canvas`]) durante o `prepaint` dos nós, e são lidas
//! no `paint` do diagrama — **mais tarde no mesmo frame**. A caixa onde elas escrevem nasce e morre
//! num render (ver [`Anchors`]); nada atravessa frame.
//!
//! # A ordem de pintura
//!
//! **Não existe `z-index` no GPUI**: quem pinta por cima é quem pinta por último. O `paint` deste
//! elemento pinta **todos os nós e depois as arestas**, então as arestas passam **na frente** dos
//! nós. É a ordem do kumo (lá o `<svg>` das conexões vem depois da lista de nós no DOM), e é a
//! ordem que se quer: a ponta da seta encosta na borda do nó de destino, e atrás dela ficariam a
//! sombra e o bisel do nó — que a comeriam pela metade. Numa aresta longa que passe por cima de um
//! nó do meio, o traço aparece sobre ele; é assim no original também.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! ## Não reproduzível
//!
//! - **`ResizeObserver`.** Não existe no GPUI, e aqui não faz falta nenhuma: a medição acontece no
//!   `request_layout`, que roda a cada frame, então "o conteúdo mudou de tamanho" e "o diagrama se
//!   reorganizou" são o mesmo frame. O original precisa do observador porque mede fora do ciclo de
//!   layout; nós medimos dentro.
//! - **A árvore vinda da hierarquia de componentes.** O kumo descobre a estrutura em tempo de
//!   render, com um contexto de "descendentes" (`use-children.tsx`) que cada `Flow.Node` usa pra
//!   se registrar no pai, e ids do `useId()` do React. Aqui a árvore é **explícita**: quem monta
//!   aninha [`FlowParallel`] e [`FlowList`] à mão, e o id de cada nó é a posição dele na ordem de
//!   declaração. Mesma árvore, sem as 225 linhas de plumbing de contexto — e sem a corrida de
//!   batelada de estado que o `node.tsx` do kumo tem um comentário longo explicando como evitou.
//! - **`<marker>` / SVG.** Não há SVG. Os caminhos saem do [`gpui::PathBuilder`] pela
//!   [`gpui::Window::paint_path`], e a ponta da seta é geometria — ver
//!   [`crate::flow_layout::arrowhead`]. O `Q` do SVG vira [`gpui::PathBuilder::curve_to`], que é
//!   quadrática também: um pra um, sem amostragem em polilinha.
//! - **`data-node-id` / `data-testid` / `aria-hidden`.** Atributos de DOM. Sem DOM, sem
//!   atributos, e sem árvore de acessibilidade no gpui 0.2.2 pra pendurar o equivalente.
//!
//! ## Resolvido em número
//!
//! - **`rounded-md`** → **8px**, que é o `--radius-md` (`--radius` − 2 = 10 − 2) dos NOSSOS
//!   tokens. O default do Tailwind seria 6. Ver [`NODE_RADIUS`].
//! - **`px-3 py-2`** → 12px e 8px. Respiro é o que decide o tamanho do nó, e tamanho é layout,
//!   então estes dois seguem o kumo.
//! - **`strokeWidth="2"`** → [`STROKE`] = 2. Idem: espessura de traço é a geometria da seta.
//! - **`opacity-40`** do conector desativado → [`DISABLED_ALPHA`] = 0.4, aplicado pelo
//!   [`crate::color::Rgba8::scaled`] na cor do traço (não há passe de opacidade em `paint_path`).
//! - **`shadow`** (o `shadow-sm` do Tailwind v4) → o par de camadas de [`node_shadows`], com o
//!   nosso token de sombra a 5%. É o mesmo `shadow-sm/5` que o [`crate::card`] e o
//!   [`crate::empty`] já usam.
//! - **`bg-kumo-base` / `ring-kumo-line` / `text-kumo-placeholder`** → os nossos `--card`,
//!   `--border` e `--muted-foreground`/72%. Ver [`FLOW_LIGHT`]/[`FLOW_DARK`] e a próxima seção.
//!
//! ## Desvio consciente
//!
//! - **A paleta inteira é a nossa, não a do kumo.** O kumo é um design system diferente do coss,
//!   que esta lib porta. Estrutura, layout, algoritmo e comportamento são do kumo; cor, raio,
//!   borda, sombra e tipografia saem dos nossos tokens, pra que este componente pareça irmão dos
//!   outros 38 da lib e não um enxerto. Em particular:
//!   - o `ring` de 1px do kumo virou **borda** de 1px (o `ring` do Tailwind é sombra, e o
//!     `paint_shadows` do GPUI não recorta sombra pra fora do elemento — o mesmo motivo que já
//!     vale no [`crate::card`] e no [`crate::empty`]);
//!   - o nó ganhou o **bisel** de 1px da casa, que o kumo não tem. É a assinatura de superfície
//!     desta lib: sem ele o nó não parece irmão do card nem da moldura. Ver [`FlowNode::bevel`].
//! - **O bisel escuro é o DOBRO da referência do coss** (branco a ~11,8% em vez de 6%), a mesma
//!   decisão já vigente em `Input`, `Card`, `Frame`, `Button` e mais oito componentes. Travado
//!   por `bisel_escuro_e_o_dobro_da_referencia`.
//! - **O nó declara `text-sm` (14/20).** O nó do kumo não declara corpo de texto nenhum, ele
//!   herda da página. Herdar aqui significa cair no `relative(1.618_034)` do GPUI, que sobre 14px
//!   dá 22,7 de entrelinha e engorda todo nó de uma linha em 3px — ou seja, herdar mudaria o
//!   layout. Declarar o par é a regra da casa exatamente por isso.
//!
//! ## Superset consciente
//!
//! - **[`Flow::gaps`]** expõe `column_gap`/`row_gap` na API pública. No kumo eles são o segundo
//!   argumento de `computePositions`, que o `diagram.tsx` chama sem passar nada — ou seja, não há
//!   como mudá-los de fora. Expor custa um método e é o que permite uma história provar que os
//!   64/48 vêm de [`crate::flow_layout::Gaps`] e não estão embutidos na conta.
//!
//! ## Ausente
//!
//! - **Pan e as barras de rolagem fantasma.** O `diagram.tsx` arrasta o diagrama com
//!   `framer-motion` (`onPanStart`/`onPan`/`onPanEnd`, cursor `grab`) e desenha duas barras de
//!   rolagem que só aparecem no hover, com o polegar dimensionado por `useTransform`. São ~200
//!   das 614 linhas do arquivo e são um **componente de rolagem**, não um diagrama: quem quer
//!   isso envolve o [`Flow`] num [`crate::scroll_area`], que já é o nosso componente pra isso e
//!   já tem a barra que se esconde. Não reimplementar aqui é o que evita duas barras de rolagem
//!   diferentes na mesma lib.
//! - **`render` prop.** No kumo, `<Flow.Node render={<custom/>}/>` troca o elemento inteiro. O
//!   equivalente aqui é [`FlowNode::bare`], que tira a superfície e deixa o conteúdo cru — o que
//!   cobre o caso real (desenhar o próprio nó) sem precisar de `cloneElement`.
//! - **`Connector::is_bottom` e `single = false` na API pública.** O
//!   [`crate::flow_layout::rounded_path`] implementa os dois (são do `createRoundedPath`), mas o
//!   `FlowConnectors` do kumo também nunca os usa: ele passa `single: true` e deixa `isBottom`
//!   fora. Ficam disponíveis no módulo puro, sem porta na API do componente.
//! - **`disabled` no `Flow.Parallel`.** O tipo `NodeData` do kumo tem o campo, e nada o lê.
//!
//! ## Sem cobertura de teste — declarado
//!
//! - **O que foi pintado.** O GPUI não deixa um teste ler a cena, então nenhum teste prova que o
//!   traço saiu na cor certa, na espessura certa, ou que a ponta da seta apareceu. O que guarda
//!   isso é: a geometria toda testada por valor no [`crate::flow_layout`] (42 testes), o teste
//!   das constantes e o da paleta contra literais, e a história no storybook. Trocar
//!   [`STROKE`] por 3 não quebra a suíte.
//! - **Cor, peso e entrelinha do texto do nó.** Mesma limitação: são propriedade de estilo. O
//!   teste de constantes trava o par (14, 20) contra os literais, e o teste de janela trava a
//!   ALTURA do nó, que é o número que a entrelinha move — mas remover o `.text_color(..)` não
//!   quebra nada.
//! - **O bisel e a sombra.** Invisíveis a teste. A paleta e o lado do filete são testados; que o
//!   `div` do bisel esteja de fato pendurado no nó, não.
//! - **A âncora chegando na seta.** [`anchor_offset`] é testada por valor, e
//!   `no_com_ancora_monta_e_empilha_o_conteudo_na_ordem` executa o caminho inteiro (as duas sondas,
//!   o [`AnchorKind::Both`] que pendura duas no mesmo nó, e a disciplina de [`RefCell`] entre o
//!   `prepaint` que escreve e o `paint` que lê). O que **não** se testa é o último elo: que a seta
//!   pintada encostou na âncora. Isso é pintura, e pintura não se lê.
//! - **A escolha de [`gpui::AvailableSpace`] na medição.** Trocar `MaxContent` por `MinContent`
//!   **não quebra teste nenhum** — verificado por mutação. O caminho de texto do GPUI mede a linha
//!   inteira nos dois casos, então a diferença não se manifesta aqui. `MaxContent` fica porque é o
//!   que descreve a intenção ("o tamanho natural do conteúdo"), não porque um teste o exija.
//! - **O `bare` visualmente.** `no_cru_nao_tem_respiro_nem_borda` prova que a variante crua não
//!   ocupa respiro nem borda (que é o que afeta o layout). Que ela também não pinte fundo, sombra
//!   nem bisel é pintura, e não se lê.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    canvas, div, point, px, size, AnyElement, App, AvailableSpace, Bounds, Div, Element, ElementId,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, ParentElement, PathBuilder, Pixels,
    Point, Style, Styled, Window,
};

use crate::color::Rgba8;
use crate::flow_layout::{
    arrowhead, arrowhead_outline, compute_connectors, compute_diagram_rect, compute_edges,
    compute_positions, Connector, FlowAlign, FlowOrientation, FlowState, Gaps, NodeId, NodeMeasure,
    ParallelAlign, PathCommand, PathOptions, Position, TreeNode,
};
use crate::theme;

// =================================================================================================
// Paleta
// =================================================================================================

// Mesma disciplina de cor do card, da moldura e do estado vazio: TODO valor é `0xRRGGBBAA`, com o
// byte de alfa, SEMPRE — ver [`crate::color`]. Os tokens de [`crate::theme`] são `0xRRGGBB`, e
// misturar as duas convenções desloca os canais e produz outra cor sem erro de compilação. A única
// ponte é [`crate::color::opaque`].
//
// O kumo tem os tokens dele (`kumo-base`, `kumo-line`, `kumo-placeholder`); aqui valem os nossos
// equivalentes do coss, pela decisão declarada no doc do módulo.

/// Tokens visuais do diagrama, por tema.
#[derive(Clone, Copy, Debug)]
struct FlowPalette {
    /// `--card`, no lugar do `bg-kumo-base` — o fundo do nó.
    bg: Rgba8,
    /// `--border`, no lugar do `ring-kumo-line` — a borda de 1px do nó.
    border: Rgba8,
    /// `--foreground` — o texto do nó.
    text: Rgba8,
    /// `shadow-sm/5` — a cor das duas camadas de sombra do nó.
    shadow: Rgba8,
    /// Fio de bisel de 1px do nó. Desce no claro, sobe no escuro.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (`0 1px`, base), `-1` sobe (`0 -1px`, topo).
    bevel_dir: f32,
    /// `--muted-foreground`/72%, no lugar do `text-kumo-placeholder` — a cor do traço e da ponta
    /// de cada aresta.
    connector: Rgba8,
}

/// Tema **claro**.
const FLOW_LIGHT: FlowPalette = FlowPalette {
    bg: Rgba8(0xffffffff),
    border: Rgba8(0x00000014), // black 8%
    text: Rgba8(0x262626ff),   // neutral-800
    shadow: Rgba8(0x0000000d), // black 5% (o `/5` do `shadow-sm/5`)
    bevel: Rgba8(0x0000000a),  // black 4%
    bevel_dir: 1.0,
    // mix(neutral-500 90%, black) = #686868, com o `/72` do Tailwind no alfa.
    connector: Rgba8(0x686868b8),
};

/// Tema **escuro**.
const FLOW_DARK: FlowPalette = FlowPalette {
    // `--card` escuro = mix(background 98%, white), com background = mix(neutral-950 96%, white).
    bg: Rgba8(0x191919ff),
    border: Rgba8(0xffffff0f), // white 6%
    text: Rgba8(0xf5f5f5ff),   // neutral-100
    shadow: Rgba8(0x0000000d), // black 5% — a sombra não muda de cor entre os temas
    // ⚠️ DESVIO CONSCIENTE do coss, o MESMO já vigente no `Input`, no `Card`, no `Frame`, no
    // `Button` e no `Empty`: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO — alfa 30 ≈
    // 11,8% — porque a 6% o filete é imperceptível no nosso fundo escuro. NÃO "corrija" isto pra
    // 0x0f achando que é erro de porte; se a intenção mudar, mude junto o teste
    // `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
    // mix(neutral-500 90%, white) = #818181, com o `/72` no alfa.
    connector: Rgba8(0x818181b8),
};

/// A paleta do diagrama no tema corrente.
fn palette() -> &'static FlowPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &FLOW_DARK,
        theme::ThemeMode::Light => &FLOW_LIGHT,
    }
}

// =================================================================================================
// Constantes
// =================================================================================================

// 1 unidade Tailwind = 4px; `--radius` = 10px, `--radius-md` = 8px.

/// Raio da superfície do nó — `rounded-md` = `--radius-md` = 10 − 2.
///
/// O default do Tailwind pra `rounded-md` é 6; sob os NOSSOS tokens dá 8. Ver o doc do módulo.
const NODE_RADIUS: f32 = 8.0;

/// Respiro horizontal do nó — `px-3`.
const NODE_PX: f32 = 12.0;

/// Respiro vertical do nó — `py-2`.
const NODE_PY: f32 = 8.0;

/// Espessura da borda do nó — o `ring` de 1px do original, virado borda.
const NODE_BORDER: f32 = 1.0;

/// Corpo do texto do nó — `text-sm`.
const TEXT_SM: f32 = 14.0;

/// Entrelinha do texto do nó — o PAR do `text-sm` no Tailwind: 14px de fonte, **20** de linha.
///
/// Sem isto o GPUI aplica `relative(1.618_034)`, que sobre 14px dá 22,7 — e como a altura do nó é
/// o que alimenta o layout, herdar a entrelinha mudaria a posição de todo mundo.
const TEXT_SM_LINE: f32 = 20.0;

/// Espessura do traço de uma aresta — o `strokeWidth="2"` do original.
const STROKE: f32 = 2.0;

/// Alfa do conector cujos nós estão desativados — o `opacity-40` do original.
const DISABLED_ALPHA: f32 = 0.4;

/// As duas sombras do `shadow-sm/5` do nó.
///
/// ⚠️ **Valor deduzido.** O `--shadow-sm` do Tailwind v4 é
/// `0 1px 3px 0 black/10, 0 1px 2px -1px black/10`, e o `/5` troca o alfa das duas por 5%. O
/// `shadow` do nó do kumo é justamente o `shadow-sm`. É o mesmo par do [`crate::card`] e do
/// [`crate::empty`].
///
/// Sombra **externa** sobre fundo **opaco**, que é o caso em que a técnica do GPUI funciona: o
/// `paint_shadows` não recorta a sombra pra fora do elemento que a projeta, e aqui ninguém vê.
fn node_shadows() -> Vec<gpui::BoxShadow> {
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

// =================================================================================================
// Medição
// =================================================================================================

/// O que as sondas de âncora de UM nó viram, em coordenadas de janela.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct AnchorProbe {
    /// A caixa da âncora de **saída** ([`AnchorKind::Start`] ou [`AnchorKind::Both`]).
    start: Option<Bounds<Pixels>>,
    /// A caixa da âncora de **entrada** ([`AnchorKind::End`] ou [`AnchorKind::Both`]).
    end: Option<Bounds<Pixels>>,
}

/// Onde as sondas de âncora escrevem, indexado pela ordem de declaração dos nós.
///
/// **Vive um frame.** Nasce no `request_layout`, é escrita no `prepaint` de cada nó e lida no
/// `paint` do diagrama. Não é cache: se sobrevivesse ao frame, seria estado a invalidar, e é
/// justamente não ter estado que faz o diagrama sair correto no primeiro frame.
type Anchors = Rc<RefCell<Vec<AnchorProbe>>>;

/// Monta a medida de um nó a partir do tamanho que o layout devolveu.
///
/// As âncoras entram depois, no `paint` — ver [`anchor_offset`].
fn measure_of(width: f32, height: f32, disabled: bool) -> NodeMeasure {
    NodeMeasure {
        width,
        height,
        disabled,
        start_anchor_offset: None,
        end_anchor_offset: None,
    }
}

/// O deslocamento de uma âncora: o **meio** dela, medido a partir do topo do nó.
///
/// É o mesmo número do kumo (`anchorRect.top - nodeRect.top + anchorRect.height / 2`), com as duas
/// caixas em coordenadas de janela. Sem âncora, `None` — e aí o [`crate::flow_layout`] usa o centro
/// do nó, que é o padrão.
fn anchor_offset(anchor: Option<Bounds<Pixels>>, node_top: f32) -> Option<f32> {
    let anchor = anchor?;
    Some(f32::from(anchor.origin.y) + f32::from(anchor.size.height) / 2.0 - node_top)
}

/// Uma sonda de âncora: um [`gpui::canvas`] que só mede e não pinta nada.
///
/// O `inset: 0` explícito é de propósito: sem ele, um absoluto de insets `auto` cai na posição
/// estática (dentro do respiro) e a medida sai deslocada. Mesmo truque do [`crate::tabs`], do
/// [`crate::toast`] e do [`crate::dialog`].
fn sonda_de_ancora(
    anchors: Anchors,
    index: usize,
    write: fn(&mut AnchorProbe, Bounds<Pixels>),
) -> impl IntoElement {
    canvas(
        move |bounds, _window, _cx| {
            if let Some(probe) = anchors.borrow_mut().get_mut(index) {
                write(probe, bounds);
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

// =================================================================================================
// A API pública
// =================================================================================================

/// Qual ponta uma [`FlowNode::anchor`] governa.
///
/// O `type` do `Flow.Anchor` do kumo. Por padrão a seta liga no **centro** do nó; uma âncora troca
/// esse ponto pelo meio dela.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnchorKind {
    /// As duas pontas: a seta que sai e a que entra. O padrão.
    #[default]
    Both,
    /// Só a seta que **sai** do nó. A que entra continua no centro.
    Start,
    /// Só a seta que **entra** no nó. A que sai continua no centro.
    End,
}

/// Uma peça de um fluxo, na ordem em que foi declarada.
///
/// Enum privado, e não trait: as três variantes são fechadas (é a árvore da spec do kumo) e
/// nenhuma delas precisa ser interrogada pelo pai como os filhos do [`crate::group`]. Mesma
/// escolha, e mesma razão, do `FrameSlot` do [`crate::frame`].
enum FlowItem {
    Node(FlowNode),
    Parallel(FlowParallel),
    List(FlowList),
}

/// Um filho de [`FlowNode`], que pode ser conteúdo comum ou uma âncora.
///
/// Guardar as âncoras **na ordem** entre os outros filhos é o que permite a âncora ser o rodapé do
/// nó, como no exemplo da spec.
enum FlowNodeChild {
    Plain(AnyElement),
    Anchor {
        kind: AnchorKind,
        content: AnyElement,
    },
}

/// Um nó do diagrama: uma superfície com conteúdo livre.
///
/// O tamanho **não** se declara — é medido. Ver "A medição" no doc do módulo.
#[derive(Default)]
pub struct FlowNode {
    children: Vec<FlowNodeChild>,
    disabled: bool,
    bare: bool,
}

impl FlowNode {
    pub fn new() -> Self {
        Self::default()
    }

    /// Acrescenta conteúdo. A ordem de chamada é a ordem na tela.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children
            .push(FlowNodeChild::Plain(child.into_any_element()));
        self
    }

    /// Acrescenta conteúdo que também é o **ponto de ligação** das setas.
    ///
    /// Sem âncora, a seta liga no centro do nó. Com [`AnchorKind::Start`] no rodapé e nada no
    /// cabeçalho, a seta entra pelo meio do nó e sai pelo rodapé — o desenho do exemplo da spec.
    pub fn anchor(mut self, kind: AnchorKind, content: impl IntoElement) -> Self {
        self.children.push(FlowNodeChild::Anchor {
            kind,
            content: content.into_any_element(),
        });
        self
    }

    /// Acinzenta as arestas que tocam este nó.
    ///
    /// Note que o nó em si **não** muda: é assim no kumo (`opacity-40` vai no conector, não no nó).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Tira a superfície e deixa o conteúdo cru — sem fundo, borda, sombra, bisel nem respiro.
    ///
    /// É o equivalente do `render` prop do kumo: quem quer desenhar o próprio nó ainda ganha o
    /// posicionamento e as arestas de graça.
    pub fn bare(mut self) -> Self {
        self.bare = true;
        self
    }

    /// O bisel de 1px sobreposto do nó.
    ///
    /// ⚠️ Duas coisas que já custaram bug visível nesta base (ver [`crate::card`],
    /// [`crate::frame`] e [`crate::empty`]):
    ///
    /// 1. **É borda, não sombra.** O `Window::paint_shadows` do GPUI não recorta a sombra pra fora
    ///    do elemento que a projeta (diferente do CSS), então um `box-shadow` de filete viraria uma
    ///    lavagem de cor sobre o nó inteiro.
    /// 2. **Cobre a BORDER box.** O overlay fica em `inset: -1px`, ou seja na border box, então o
    ///    raio dele é o da superfície ([`NODE_RADIUS`]) — é o que o deixa concêntrico com a borda
    ///    que ele clareia.
    ///
    /// E o LADO do filete sai do **sinal** do deslocamento, não de uma escolha paralela: um
    /// `0 1px` empurra pra baixo → borda de baixo; `0 -1px` → borda de topo. Com o sinal dirigindo
    /// o lado, não há como os dois divergirem.
    fn bevel() -> Div {
        let p = palette();
        let overlay = div()
            .absolute()
            .top(px(-NODE_BORDER))
            .left(px(-NODE_BORDER))
            .right(px(-NODE_BORDER))
            .bottom(px(-NODE_BORDER))
            .rounded(px(NODE_RADIUS))
            .border_color(p.bevel.hsla());
        if p.bevel_dir > 0.0 {
            overlay.border_b_1()
        } else {
            overlay.border_t_1()
        }
    }

    /// Monta o nó.
    ///
    /// Não recebe posição: quem posiciona é o `prepaint` do [`Flow`], que solta este elemento no
    /// lugar com [`gpui::AnyElement::prepaint_at`]. Aqui o nó só precisa saber medir a si mesmo.
    fn render(self, index: usize, anchors: Anchors) -> AnyElement {
        let p = palette();

        let mut surface = div().relative().flex().flex_col();
        if !self.bare {
            surface = surface
                .px(px(NODE_PX))
                .py(px(NODE_PY))
                .rounded(px(NODE_RADIUS))
                .bg(p.bg.hsla())
                .border(px(NODE_BORDER))
                .border_color(p.border.hsla())
                .shadow(node_shadows())
                .text_size(px(TEXT_SM))
                .line_height(px(TEXT_SM_LINE))
                .text_color(p.text.hsla());
        }

        for child in self.children {
            surface = match child {
                FlowNodeChild::Plain(content) => surface.child(content),
                FlowNodeChild::Anchor { kind, content } => {
                    // A âncora se mede por sonda, e o meio da caixa dela é onde a seta encosta.
                    let mut anchor = div().relative().child(content);
                    if matches!(kind, AnchorKind::Start | AnchorKind::Both) {
                        anchor = anchor.child(sonda_de_ancora(
                            anchors.clone(),
                            index,
                            |probe, bounds| probe.start = Some(bounds),
                        ));
                    }
                    if matches!(kind, AnchorKind::End | AnchorKind::Both) {
                        anchor = anchor.child(sonda_de_ancora(
                            anchors.clone(),
                            index,
                            |probe, bounds| probe.end = Some(bounds),
                        ));
                    }
                    surface.child(anchor)
                }
            };
        }

        // O bisel por último: é absoluto, então a ordem só decide quem pinta em cima. E a
        // superfície NÃO tem `overflow_hidden` — ele recortaria justamente o 1px que o overlay
        // projeta pra fora.
        if !self.bare {
            surface = surface.child(Self::bevel());
        }

        surface.into_any_element()
    }
}

/// Um grupo de ramos **paralelos**: os filhos abrem lado a lado, e o vizinho de fora liga em todos.
///
/// É o que produz o leque. Dois grupos paralelos adjacentes **não** se interligam — ver as regras
/// em [`crate::flow_layout`].
#[derive(Default)]
pub struct FlowParallel {
    items: Vec<FlowItem>,
    align: ParallelAlign,
}

impl FlowParallel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Um ramo de um nó só. A ordem de chamada é a ordem na tela.
    pub fn node(mut self, node: FlowNode) -> Self {
        self.items.push(FlowItem::Node(node));
        self
    }

    /// Um ramo com vários nós em sequência.
    ///
    /// O vizinho de fora liga só no **primeiro** e no **último** da lista.
    pub fn list(mut self, list: FlowList) -> Self {
        self.items.push(FlowItem::List(list));
        self
    }

    /// Um grupo paralelo dentro deste. O leque achata: o vizinho de fora liga em todas as folhas.
    pub fn parallel(mut self, parallel: FlowParallel) -> Self {
        self.items.push(FlowItem::Parallel(parallel));
        self
    }

    /// Encosta os ramos na **saída** em vez da entrada — o ramo curto desloca pra frente.
    pub fn align_end(mut self) -> Self {
        self.align = ParallelAlign::End;
        self
    }
}

/// Uma sequência de peças aninhada dentro de um [`FlowParallel`].
///
/// Existe pelo que ela faz nas **arestas**: um vizinho de fora liga só na primeira e na última
/// peça dela, não no meio.
#[derive(Default)]
pub struct FlowList {
    items: Vec<FlowItem>,
}

impl FlowList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Acrescenta um nó. A ordem de chamada é a ordem na tela.
    pub fn node(mut self, node: FlowNode) -> Self {
        self.items.push(FlowItem::Node(node));
        self
    }

    /// Acrescenta um grupo paralelo.
    pub fn parallel(mut self, parallel: FlowParallel) -> Self {
        self.items.push(FlowItem::Parallel(parallel));
        self
    }

    /// Acrescenta outra lista.
    pub fn list(mut self, list: FlowList) -> Self {
        self.items.push(FlowItem::List(list));
        self
    }
}

/// O diagrama de fluxo. Ver o doc do módulo.
///
/// Construa a cada frame: não há estado guardado entre um e outro.
///
/// ```ignore
/// Flow::new()
///     .node(FlowNode::new().child("A"))
///     .parallel(
///         FlowParallel::new()
///             .node(FlowNode::new().child("B1"))
///             .node(FlowNode::new().child("B2")),
///     )
///     .node(FlowNode::new().child("C"))
/// ```
#[derive(Default)]
pub struct Flow {
    items: Vec<FlowItem>,
    orientation: FlowOrientation,
    align: FlowAlign,
    gaps: Gaps,
}

impl Flow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Acrescenta um nó na sequência principal. A ordem de chamada é a ordem na tela.
    pub fn node(mut self, node: FlowNode) -> Self {
        self.items.push(FlowItem::Node(node));
        self
    }

    /// Abre um leque: os ramos ficam lado a lado, e os vizinhos ligam em todos eles.
    pub fn parallel(mut self, parallel: FlowParallel) -> Self {
        self.items.push(FlowItem::Parallel(parallel));
        self
    }

    /// Aninha uma sequência. Na raiz isto é quase sempre desnecessário — o diagrama já **é** uma
    /// lista; serve pra agrupar peças cujas arestas devem entrar só pelas pontas.
    pub fn list(mut self, list: FlowList) -> Self {
        self.items.push(FlowItem::List(list));
        self
    }

    /// Vira o eixo: a sequência desce e os ramos abrem pra direita.
    ///
    /// As setas passam a sair pela **base** do nó e entrar pelo **topo** — e aí as âncoras não
    /// participam, porque são deslocamentos verticais. É assim no kumo.
    pub fn vertical(mut self) -> Self {
        self.orientation = FlowOrientation::Vertical;
        self
    }

    /// Centraliza cada peça da sequência na faixa da mais alta, em vez de encostar todas no topo.
    pub fn align_center(mut self) -> Self {
        self.align = FlowAlign::Center;
        self
    }

    /// Troca os espaçamentos. O padrão é o do `flow-layout.ts` do kumo: 64 na sequência, 16
    /// entre ramos (a spec dele diz 48 — ver a declaração no doc de `flow_layout`).
    pub fn gaps(mut self, column_gap: f32, row_gap: f32) -> Self {
        self.gaps = Gaps {
            column_gap,
            row_gap,
        };
        self
    }
}

/// Achata a árvore declarada em [`TreeNode`] + a lista de nós a renderizar.
///
/// O [`NodeId`] de um nó é a **posição dele na ordem de declaração** — que é o que substitui o
/// `useId()` do React. Puro, e por isso testável sem janela.
fn flatten(items: Vec<FlowItem>, out: &mut Vec<FlowNode>) -> Vec<TreeNode> {
    items
        .into_iter()
        .map(|item| match item {
            FlowItem::Node(node) => {
                let id = NodeId(out.len());
                out.push(node);
                TreeNode::Node { id }
            }
            FlowItem::List(list) => TreeNode::List {
                children: flatten(list.items, out),
            },
            FlowItem::Parallel(parallel) => TreeNode::Parallel {
                children: flatten(parallel.items, out),
                align: parallel.align,
            },
        })
        .collect()
}

// =================================================================================================
// O elemento
// =================================================================================================

/// O que o `request_layout` descobriu e as fases seguintes consomem.
///
/// Fica atrás de um [`Rc`]/[`RefCell`] porque o fechamento de medida do
/// [`gpui::Window::request_measured_layout`] é `FnMut` e precisa dos mesmos elementos que o
/// `prepaint` e o `paint` vão usar depois.
///
/// É **detalhe de implementação**, público só porque o Rust exige que o tipo associado
/// `Element::RequestLayoutState` seja tão visível quanto o `impl`. Nada aqui é API.
pub struct Medido {
    /// Os elementos dos nós, na ordem de declaração — o índice é o [`NodeId`].
    elements: Vec<AnyElement>,
    /// A medida de cada nó, já com o `disabled`.
    measures: Vec<NodeMeasure>,
    /// A árvore, guardada pra o `paint` poder recalcular as arestas com as âncoras.
    tree: TreeNode,
    /// Onde cada nó vai, relativo à origem do diagrama.
    positions: Vec<Option<Position>>,
    /// Onde as sondas de âncora escrevem.
    anchors: Anchors,
}

impl IntoElement for Flow {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Flow {
    type RequestLayoutState = Rc<RefCell<Medido>>;
    type PrepaintState = ();

    /// Sem id: este elemento **não guarda estado entre frames**, então não há nada pra uma
    /// `GlobalElementId` endereçar. Ver "A medição" no doc do módulo.
    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    /// As fases de **medição** e de **layout** da spec do kumo, as duas aqui.
    ///
    /// ⚠️ A medição roda **direto neste método**, e não dentro de um fechamento de
    /// [`gpui::Window::request_measured_layout`]. Não é estilo: o fechamento de medida é chamado de
    /// dentro do `compute_layout`, que faz `layout_engine.take()` — e aí qualquer
    /// [`gpui::AnyElement::layout_as_root`] lá dentro estoura num `unwrap()` de `Option::None` no
    /// `window.rs`, sem mensagem que diga o motivo. Medir antes e pedir um layout de tamanho fixo é
    /// o que o `virtual_list` do `gpui-component` também faz, pela mesma razão.
    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // A árvore, e os nós na ordem de declaração.
        let mut nodes = Vec::new();
        let tree = TreeNode::List {
            children: flatten(std::mem::take(&mut self.items), &mut nodes),
        };

        let anchors: Anchors = Rc::new(RefCell::new(vec![AnchorProbe::default(); nodes.len()]));
        let disabled: Vec<bool> = nodes.iter().map(|node| node.disabled).collect();
        let mut elements: Vec<AnyElement> = nodes
            .into_iter()
            .enumerate()
            .map(|(index, node)| node.render(index, anchors.clone()))
            .collect();

        // **A medição.** Cada nó é um layout independente, medido no espaço que ele quiser —
        // `MaxContent` nos dois eixos, pra o texto sair numa linha e a caixa sair do conteúdo. É
        // isto que substitui o `getBoundingClientRect` do original.
        let espaco = size(AvailableSpace::MaxContent, AvailableSpace::MaxContent);
        let measures: Vec<NodeMeasure> = elements
            .iter_mut()
            .zip(&disabled)
            .map(|(element, &disabled)| {
                let medida = element.layout_as_root(espaco, window, cx);
                measure_of(f32::from(medida.width), f32::from(medida.height), disabled)
            })
            .collect();

        // **O layout.** Derivado, nunca guardado — é o ponto que a spec do kumo repete.
        let state = FlowState {
            nodes: measures.clone(),
            tree: tree.clone(),
            align: self.align,
            orientation: self.orientation,
        };
        let positions = compute_positions(&state, self.gaps);
        let rect = compute_diagram_rect(&positions, &state);

        // O tamanho do diagrama é o retângulo do conteúdo — no MESMO frame, então o pai já recebe a
        // medida certa. Os nós não entram como filhos de layout: eles já foram medidos como raízes
        // e são soltos no lugar pelo `prepaint`.
        let mut style = Style::default();
        style.size.width = px(rect.width).into();
        style.size.height = px(rect.height).into();
        let layout_id = window.request_layout(style, [], cx);

        (
            layout_id,
            Rc::new(RefCell::new(Medido {
                elements,
                measures,
                tree,
                positions,
                anchors,
            })),
        )
    }

    /// Solta cada nó na posição que o layout lhe deu — e é aqui que as sondas de âncora medem.
    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        medido: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let mut m = medido.borrow_mut();
        let Medido {
            elements,
            positions,
            ..
        } = &mut *m;

        for (element, position) in elements.iter_mut().zip(positions.iter()) {
            let Some(position) = position else { continue };
            element.prepaint_at(
                bounds.origin + point(px(position.x), px(position.y)),
                window,
                cx,
            );
        }
    }

    /// Os nós, e as arestas **por cima** deles. Ver "A ordem de pintura" no doc do módulo.
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        medido: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let mut m = medido.borrow_mut();

        // As âncoras já foram medidas (as sondas rodaram no `prepaint` acima), então só agora as
        // pontas das setas têm endereço definitivo.
        let sondado = m.anchors.borrow().clone();
        let mut measures = m.measures.clone();
        for (index, probe) in sondado.iter().enumerate() {
            let (Some(Some(position)), Some(measure)) =
                (m.positions.get(index), measures.get_mut(index))
            else {
                continue;
            };
            let node_top = f32::from(bounds.origin.y) + position.y;
            measure.start_anchor_offset = anchor_offset(probe.start, node_top);
            measure.end_anchor_offset = anchor_offset(probe.end, node_top);
        }

        let state = FlowState {
            nodes: measures,
            tree: m.tree.clone(),
            align: self.align,
            orientation: self.orientation,
        };
        let edges = compute_edges(&state);
        let connectors = compute_connectors(&edges, &m.positions, &state, self.orientation);

        // Os nós primeiro.
        for element in m.elements.iter_mut() {
            element.paint(window, cx);
        }

        // E as arestas depois, ou seja em cima.
        paint_connectors(&connectors, self.orientation, bounds.origin, window);
    }
}

// =================================================================================================
// A pintura das arestas
// =================================================================================================

/// Emite no `pb` um caminho do [`crate::flow_layout`], deslocado pra coordenadas de janela.
///
/// O `QuadTo` vira [`gpui::PathBuilder::curve_to`], que é quadrática também — nota o `to` **antes**
/// do controle, ao contrário do `Q` do SVG.
fn emit(pb: &mut PathBuilder, commands: &[PathCommand], origin: Point<Pixels>) {
    let at = |x: f32, y: f32| origin + point(px(x), px(y));
    for command in commands {
        match *command {
            PathCommand::MoveTo { x, y } => pb.move_to(at(x, y)),
            PathCommand::LineTo { x, y } => pb.line_to(at(x, y)),
            PathCommand::QuadTo { cx, cy, x, y } => pb.curve_to(at(x, y), at(cx, cy)),
        }
    }
}

/// Pinta o traço e a ponta de cada aresta.
///
/// Um caminho degenerado (o diagrama vazio, ou dois nós exatamente sobrepostos) não constrói, e o
/// `build` devolve `Err` — que é ignorado de propósito: não há o que desenhar.
fn paint_connectors(
    connectors: &[Connector],
    orientation: FlowOrientation,
    origin: Point<Pixels>,
    window: &mut Window,
) {
    let options = PathOptions {
        orientation,
        ..Default::default()
    };
    let cor = palette().connector;

    for connector in connectors {
        let commands = crate::flow_layout::rounded_path(connector, options);
        // O `paint_path` não tem passe de opacidade: o `opacity-40` do conector desativado entra
        // no alfa da cor.
        let alpha = if connector.disabled {
            DISABLED_ALPHA
        } else {
            1.0
        };

        let mut traco = PathBuilder::stroke(px(STROKE));
        emit(&mut traco, &commands, origin);
        if let Ok(path) = traco.build() {
            window.paint_path(path, cor.scaled(alpha));
        }

        // A ponta, girada pela direção do último segmento.
        if let Some(head) = arrowhead(&commands) {
            let mut gota = PathBuilder::fill();
            emit(&mut gota, &arrowhead_outline(&head), origin);
            gota.close();
            if let Ok(path) = gota.build() {
                window.paint_path(path, cor.scaled(alpha));
            }
        }
    }
}

// =================================================================================================
// Testes
// =================================================================================================

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod testes {
    use super::*;

    // --- A árvore que o builder produz ----------------------------------------------------------

    /// Achata só a árvore, descartando os nós — o que os testes de estrutura querem ver.
    fn arvore(flow: Flow) -> TreeNode {
        let mut nodes = Vec::new();
        TreeNode::List {
            children: flatten(flow.items, &mut nodes),
        }
    }

    fn quantos_nos(flow: Flow) -> usize {
        let mut nodes = Vec::new();
        flatten(flow.items, &mut nodes);
        nodes.len()
    }

    #[test]
    fn diagrama_vazio_vira_lista_vazia() {
        assert_eq!(
            arvore(Flow::new()),
            TreeNode::List {
                children: Vec::new()
            }
        );
    }

    #[test]
    fn nos_recebem_id_na_ordem_de_declaracao() {
        let flow = Flow::new()
            .node(FlowNode::new().child("A"))
            .parallel(
                FlowParallel::new()
                    .node(FlowNode::new().child("B1"))
                    .node(FlowNode::new().child("B2")),
            )
            .node(FlowNode::new().child("C"));

        assert_eq!(
            arvore(flow),
            TreeNode::List {
                children: vec![
                    TreeNode::Node { id: NodeId(0) },
                    TreeNode::Parallel {
                        children: vec![
                            TreeNode::Node { id: NodeId(1) },
                            TreeNode::Node { id: NodeId(2) },
                        ],
                        align: ParallelAlign::Start,
                    },
                    TreeNode::Node { id: NodeId(3) },
                ],
            }
        );
    }

    #[test]
    fn lista_dentro_de_paralelo_vira_lista_dentro_de_paralelo() {
        // O aninhamento do exemplo mais completo da spec do kumo.
        let flow = Flow::new().node(FlowNode::new()).parallel(
            FlowParallel::new()
                .list(FlowList::new().node(FlowNode::new()).node(FlowNode::new()))
                .node(FlowNode::new()),
        );

        assert_eq!(
            arvore(flow),
            TreeNode::List {
                children: vec![
                    TreeNode::Node { id: NodeId(0) },
                    TreeNode::Parallel {
                        children: vec![
                            TreeNode::List {
                                children: vec![
                                    TreeNode::Node { id: NodeId(1) },
                                    TreeNode::Node { id: NodeId(2) },
                                ],
                            },
                            TreeNode::Node { id: NodeId(3) },
                        ],
                        align: ParallelAlign::Start,
                    },
                ],
            }
        );
    }

    #[test]
    fn align_end_chega_na_arvore() {
        // O `align` tem que atravessar o builder até o `TreeNode`, senão o layout nunca o vê.
        let flow = Flow::new().parallel(FlowParallel::new().node(FlowNode::new()).align_end());

        let TreeNode::List { children } = arvore(flow) else {
            panic!("a raiz é uma lista");
        };
        assert_eq!(
            children[0],
            TreeNode::Parallel {
                children: vec![TreeNode::Node { id: NodeId(0) }],
                align: ParallelAlign::End,
            }
        );
    }

    #[test]
    fn contagem_de_nos_ignora_os_grupos() {
        // Grupos não são nós: eles não têm medida, não recebem id e não aparecem na tela.
        let flow = Flow::new()
            .node(FlowNode::new())
            .parallel(
                FlowParallel::new()
                    .parallel(FlowParallel::new().node(FlowNode::new()))
                    .list(FlowList::new().node(FlowNode::new()).node(FlowNode::new()))
                    .node(FlowNode::new()),
            )
            .list(FlowList::new().node(FlowNode::new()));

        assert_eq!(quantos_nos(flow), 6);
    }

    #[test]
    fn orientacao_e_alinhamento_sao_opcionais_e_default_horizontal() {
        let padrao = Flow::new();
        assert_eq!(padrao.orientation, FlowOrientation::Horizontal);
        assert_eq!(padrao.align, FlowAlign::Start);
        assert_eq!(padrao.gaps, Gaps::default());

        let virado = Flow::new().vertical().align_center().gaps(10.0, 4.0);
        assert_eq!(virado.orientation, FlowOrientation::Vertical);
        assert_eq!(virado.align, FlowAlign::Center);
        assert_eq!(
            virado.gaps,
            Gaps {
                column_gap: 10.0,
                row_gap: 4.0
            }
        );
    }

    // --- A derivação da medida ------------------------------------------------------------------

    fn caixa(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(x), px(y)),
            size: size(px(w), px(h)),
        }
    }

    #[test]
    fn medida_carrega_tamanho_e_desativado_e_deixa_ancora_pra_depois() {
        // As âncoras entram só no `paint`, quando as sondas já mediram — ver o doc do módulo.
        let m = measure_of(64.0, 38.0, true);

        assert_eq!(m.width, 64.0);
        assert_eq!(m.height, 38.0);
        assert!(m.disabled);
        assert_eq!(m.start_anchor_offset, None);
        assert_eq!(m.end_anchor_offset, None);
    }

    #[test]
    fn deslocamento_da_ancora_e_o_meio_dela_medido_do_topo_do_no() {
        // Nó com o topo em y = 500; âncora de 20 de altura começando em y = 570. O meio dela é
        // 580, ou seja 80 abaixo do topo do nó.
        let offset = anchor_offset(Some(caixa(312.0, 570.0, 40.0, 20.0)), 500.0);

        assert_eq!(offset, Some(80.0));
    }

    #[test]
    fn deslocamento_da_ancora_e_relativo_ao_no_nao_a_janela() {
        // A MESMA âncora, com o diagrama rolado 1000px pra baixo, dá o MESMO deslocamento. É o que
        // faz a seta encostar no lugar certo independente de onde o diagrama está na tela.
        let perto = anchor_offset(Some(caixa(0.0, 570.0, 40.0, 20.0)), 500.0);
        let longe = anchor_offset(Some(caixa(0.0, 1570.0, 40.0, 20.0)), 1500.0);

        assert_eq!(perto, longe);
        assert_eq!(perto, Some(80.0));
    }

    #[test]
    fn sem_ancora_nao_ha_deslocamento() {
        // `None` é o que faz o `flow_layout` cair no centro do nó, que é o padrão da spec.
        assert_eq!(anchor_offset(None, 500.0), None);
    }

    // --- As constantes -------------------------------------------------------------------------

    #[test]
    fn raio_do_no_e_o_radius_md_dos_nossos_tokens() {
        // `rounded-md` = `--radius-md` = `--radius` − 2 = 10 − 2. O default do Tailwind seria 6:
        // este número é o desvio declarado, não o do original.
        assert_eq!(NODE_RADIUS, 8.0);
        assert!(
            NODE_RADIUS != 6.0,
            "6 é o `rounded-md` do Tailwind puro, não o dos nossos tokens"
        );
    }

    #[test]
    fn respiro_e_borda_do_no_seguem_o_original() {
        // `px-3 py-2`, com 1 unidade = 4px. Respiro é tamanho, e tamanho é layout.
        assert_eq!(NODE_PX, 12.0);
        assert_eq!(NODE_PY, 8.0);
        // O `ring` de 1px do original, virado borda — e a borda entra na caixa do nó, ou seja no
        // layout. Ver o desvio declarado no doc do módulo.
        assert_eq!(NODE_BORDER, 1.0);
    }

    #[test]
    fn texto_do_no_declara_o_par_do_tailwind() {
        // `text-sm` = (14, 20). Literais de propósito: comparar com as próprias constantes seria
        // tautológico.
        assert_eq!(TEXT_SM, 14.0);
        assert_eq!(TEXT_SM_LINE, 20.0);

        // E a prova de que declarar importa: herdando, o GPUI aplicaria a razão de ouro.
        const RAZAO_DE_OURO: f32 = 1.618_034;
        let herdada = TEXT_SM * RAZAO_DE_OURO;
        assert!(
            herdada - TEXT_SM_LINE > 2.0,
            "herdar daria {herdada:.1} de entrelinha contra os {TEXT_SM_LINE} da referência"
        );
    }

    #[test]
    fn traco_e_alfa_do_desativado_seguem_o_original() {
        // `strokeWidth="2"` e `opacity-40`.
        assert_eq!(STROKE, 2.0);
        assert_eq!(DISABLED_ALPHA, 0.4);
    }

    // --- A paleta ------------------------------------------------------------------------------

    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // **Desvio consciente do coss, travado aqui.** O original usa branco a 6% no bisel escuro;
        // nós usamos o DOBRO, a mesma decisão já vigente no `Input`, no `Card`, no `Frame`, no
        // `Button` e no `Empty`. Se alguém "corrigir" pra 6% achando que é erro de porte, este
        // teste falha e aponta pra cá.
        //
        // O tema CLARO segue fiel (preto 4%) — ali o filete é PRETO sobre superfície clara, e nesse
        // caso o valor da referência funciona.
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = FLOW_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%); veio {claro}"
        );

        let escuro = FLOW_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    #[test]
    fn bisel_troca_de_lado_entre_os_temas() {
        assert!(
            FLOW_LIGHT.bevel_dir > 0.0,
            "claro: `0 1px` desce → borda de baixo"
        );
        assert!(
            FLOW_DARK.bevel_dir < 0.0,
            "escuro: `0 -1px` sobe → borda de topo"
        );
    }

    #[test]
    fn fundo_do_no_e_opaco_nos_dois_temas() {
        // A sombra do GPUI não é recortada pra fora de quem a projeta: só funciona sob fundo
        // opaco. Um `bg` translúcido aqui deixaria a sombra aparecer por dentro do nó.
        assert_eq!(FLOW_LIGHT.bg.alpha(), 1.0);
        assert_eq!(FLOW_DARK.bg.alpha(), 1.0);
    }

    #[test]
    fn conector_e_translucido_como_o_placeholder() {
        // `text-kumo-placeholder` virou o nosso `--muted-foreground`/72%. O `/72` é o que faz o
        // traço não competir com o texto do nó.
        let esperado = 184.0 / 255.0; // 0xb8
        assert!((FLOW_LIGHT.connector.alpha() - esperado).abs() < 1e-4);
        assert!((FLOW_DARK.connector.alpha() - esperado).abs() < 1e-4);
    }

    #[test]
    fn paleta_acompanha_o_tema() {
        theme::set_theme(theme::ThemeMode::Dark);
        assert_eq!(palette().bg, FLOW_DARK.bg);
        theme::set_theme(theme::ThemeMode::Light);
        assert_eq!(palette().bg, FLOW_LIGHT.bg);
    }

    #[test]
    fn tokens_do_tema_entram_por_opaque() {
        // `opaque` é a única ponte entre as convenções de cor, e a paleta daqui já nasce em
        // `0xRRGGBBAA` — então nenhum token de 6 dígitos do tema entra sem passar por ele.
        theme::set_theme(theme::ThemeMode::Dark);
        assert_eq!(crate::color::opaque(theme::TEXT_MUTED()).alpha(), 1.0);
    }
}

// =================================================================================================
// Testes com janela
// =================================================================================================

/// Testes com janela de verdade — os que provam o que aritmética nenhuma prova.
///
/// Os três aqui existem por motivos diferentes:
///
/// - o do **primeiro frame** porque medir dentro do ciclo de layout é a decisão central do porte, e
///   o que a sustenta é um número que só a janela dá: quantos renders até o diagrama estar certo.
///   Com a sonda em duas passadas (a abordagem que o resto da lib usa) este teste falha, e foi ele
///   que pegou isso;
/// - o do **leque** porque a simetria dos ramos é o desenho que o componente existe pra mostrar, e
///   com medida de verdade — não com os 40×40 redondos da spec — ela depende de a largura do grupo
///   ser a do ramo mais largo;
/// - o do **texto** porque o tamanho de um nó de texto é o único que nenhum teste puro alcança: ele
///   sai do sistema de fontes, e é ele que decide se `MaxContent` foi o espaço de medida certo.
#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod testes_de_janela {
    use super::*;
    use gpui::{Context, Render, TestAppContext, VisualTestContext};
    use std::cell::Cell;

    /// Uma sonda de teste: grava os bounds que o layout lhe deu.
    type Sonda = Rc<Cell<Option<Bounds<Pixels>>>>;

    fn nova_sonda() -> Sonda {
        Rc::new(Cell::new(None))
    }

    fn lido(s: &Sonda) -> Bounds<Pixels> {
        s.get().expect("a sonda mediu")
    }

    fn x_de(s: &Sonda) -> f32 {
        f32::from(lido(s).origin.x)
    }

    fn y_de(s: &Sonda) -> f32 {
        f32::from(lido(s).origin.y)
    }

    /// Conteúdo de tamanho conhecido, pra a caixa do nó ser uma conta e não um chute.
    fn conteudo(largura: f32, altura: f32, s: &Sonda) -> impl IntoElement {
        let s = s.clone();
        div().w(px(largura)).h(px(altura)).relative().child(
            canvas(move |bounds, _w, _cx| s.set(Some(bounds)), |_, _, _, _| {})
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
        )
    }

    /// A caixa de um nó cujo conteúdo mede `largura` × `altura`: respiro e borda dos dois lados.
    ///
    /// ⚠️ **Literais de propósito.** Escrever `2.0 * NODE_PX + 2.0 * NODE_BORDER` aqui deixaria os
    /// testes de janela tautológicos: mudar a constante mudaria o código E a expectativa junto, e
    /// nenhum deles falharia. Os números são `px-3 py-2` (12 e 8) mais 1px de borda, em dobro por
    /// serem dois lados. Se a referência mudar, este é o segundo lugar a mudar.
    fn caixa_do_no(largura: f32, altura: f32) -> (f32, f32) {
        (largura + 24.0 + 2.0, altura + 16.0 + 2.0)
    }

    // --- O primeiro frame ----------------------------------------------------------------------

    struct MedidorDeSequencia {
        a: Sonda,
        b: Sonda,
        renders: Rc<Cell<usize>>,
    }

    impl Render for MedidorDeSequencia {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            self.renders.set(self.renders.get() + 1);
            div().child(
                Flow::new()
                    .node(FlowNode::new().child(conteudo(50.0, 30.0, &self.a)))
                    .node(FlowNode::new().child(conteudo(70.0, 30.0, &self.b))),
            )
        }
    }

    #[gpui::test]
    fn diagrama_sai_correto_no_primeiro_frame(cx: &mut TestAppContext) {
        let (a, b) = (nova_sonda(), nova_sonda());
        let renders = Rc::new(Cell::new(0));
        let (sa, sb, sr) = (a.clone(), b.clone(), renders.clone());
        let window = cx.add_window(move |_w, _cx| MedidorDeSequencia {
            a: sa,
            b: sb,
            renders: sr,
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        // A distância entre os CONTEÚDOS cancela o respiro dos dois nós, então o que sobra é a
        // largura do nó A mais o `column_gap`.
        let distancia = x_de(&b) - x_de(&a);
        let (largura_a, _) = caixa_do_no(50.0, 30.0);
        let esperado = largura_a + Gaps::default().column_gap;

        assert!(
            (distancia - esperado).abs() < 0.5,
            "os conteúdos ficaram a {distancia} e a conta pede {esperado}"
        );

        // **O número que sustenta a escolha de arquitetura.** Medindo dentro do ciclo de layout, um
        // render basta. Com sonda em duas passadas seriam dois — e como o GPUI recusa agendar frame
        // de dentro de um desenho, na prática o diagrama nunca saía do lugar. Ver o doc do módulo.
        assert_eq!(
            renders.get(),
            1,
            "o diagrama tem que sair certo no PRIMEIRO frame"
        );

        // E a prova de que a distância veio da medida, não do gap: sem medir, os dois nós ficariam
        // a um gap um do outro.
        assert!(
            esperado - Gaps::default().column_gap > 30.0,
            "sem a medição a distância cairia pra {} — é essa diferença que o teste pega",
            Gaps::default().column_gap
        );
        // Os dois na mesma linha: a lista horizontal não move ninguém no eixo cruzado.
        assert!(
            (y_de(&a) - y_de(&b)).abs() < 0.5,
            "a lista horizontal alinha os dois pelo topo"
        );
    }

    // --- O leque -------------------------------------------------------------------------------

    struct MedidorDeLeque {
        a: Sonda,
        b1: Sonda,
        b2: Sonda,
        c: Sonda,
    }

    impl Render for MedidorDeLeque {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            div().child(
                Flow::new()
                    .node(FlowNode::new().child(conteudo(40.0, 20.0, &self.a)))
                    .parallel(
                        FlowParallel::new()
                            // Larguras DESIGUAIS de propósito: com ramos iguais, um bug de "a
                            // largura do grupo é a do último ramo" passaria batido.
                            .node(FlowNode::new().child(conteudo(60.0, 20.0, &self.b1)))
                            .node(FlowNode::new().child(conteudo(140.0, 20.0, &self.b2))),
                    )
                    .node(FlowNode::new().child(conteudo(40.0, 20.0, &self.c))),
            )
        }
    }

    #[gpui::test]
    fn leque_alinha_os_ramos_e_desloca_pelo_mais_largo(cx: &mut TestAppContext) {
        let (a, b1, b2, c) = (nova_sonda(), nova_sonda(), nova_sonda(), nova_sonda());
        let (sa, s1, s2, sc) = (a.clone(), b1.clone(), b2.clone(), c.clone());
        let window = cx.add_window(move |_w, _cx| MedidorDeLeque {
            a: sa,
            b1: s1,
            b2: s2,
            c: sc,
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        // 1. Os dois ramos partem da MESMA coluna. É o que faz o leque sair simétrico.
        assert!(
            (x_de(&b1) - x_de(&b2)).abs() < 0.5,
            "os ramos deviam partir da mesma coluna; vieram em {} e {}",
            x_de(&b1),
            x_de(&b2)
        );

        // 2. Os dois ramos estão separados no eixo vertical por altura + `row_gap`.
        let (_, altura_ramo) = caixa_do_no(60.0, 20.0);
        let dy = y_de(&b2) - y_de(&b1);
        let dy_esperado = altura_ramo + Gaps::default().row_gap;
        assert!(
            (dy - dy_esperado).abs() < 0.5,
            "os ramos ficaram a {dy} um do outro e a conta pede {dy_esperado}"
        );

        // 3. O nó DEPOIS do leque desloca pela largura do ramo MAIS LARGO, não do último nem do
        //    primeiro. É a regra que mantém a simetria quando os ramos são desiguais.
        let (largura_a, _) = caixa_do_no(40.0, 20.0);
        let (largura_larga, _) = caixa_do_no(140.0, 20.0);
        let (largura_curta, _) = caixa_do_no(60.0, 20.0);
        let gap = Gaps::default().column_gap;
        let esperado = largura_a + gap + largura_larga + gap;
        let obtido = x_de(&c) - x_de(&a);
        assert!(
            (obtido - esperado).abs() < 0.5,
            "o nó de saída ficou a {obtido} do de entrada e a conta pede {esperado}"
        );
        // A prova de que o teste distingue: se o grupo tivesse deslocado pelo ramo CURTO, o número
        // seria bem outro.
        let se_fosse_o_curto = largura_a + gap + largura_curta + gap;
        assert!(
            (esperado - se_fosse_o_curto).abs() > 30.0,
            "os dois candidatos precisam estar longe pro teste distinguir"
        );
        // E o leque abre À FRENTE da entrada, não sobre ela.
        assert!(
            x_de(&b1) > x_de(&a) + largura_a,
            "o leque abre à frente da entrada"
        );
    }

    // --- O tamanho que o diagrama devolve pro pai -----------------------------------------------

    struct MedidorDoDiagrama {
        direita: Sonda,
        abaixo: Sonda,
    }

    impl Render for MedidorDoDiagrama {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            let marco = |s: &Sonda| {
                let s = s.clone();
                canvas(move |bounds, _w, _cx| s.set(Some(bounds)), |_, _, _, _| {})
                    .w(px(1.0))
                    .h(px(1.0))
            };
            // Irmãos colados no diagrama: a origem deles é a soma de tudo que ele ocupou. Medir o
            // diagrama por dentro não serve — os nós são internos e absolutos.
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .child(
                            Flow::new()
                                .node(FlowNode::new().child(conteudo(40.0, 20.0, &nova_sonda())))
                                .parallel(
                                    FlowParallel::new()
                                        .node(
                                            FlowNode::new()
                                                .child(conteudo(40.0, 20.0, &nova_sonda())),
                                        )
                                        .node(
                                            FlowNode::new()
                                                .child(conteudo(40.0, 20.0, &nova_sonda())),
                                        ),
                                ),
                        )
                        .child(marco(&self.direita)),
                )
                .child(marco(&self.abaixo))
        }
    }

    #[gpui::test]
    fn diagrama_ocupa_no_pai_o_retangulo_do_conteudo(cx: &mut TestAppContext) {
        // O diagrama tem que declarar o PRÓPRIO tamanho no mesmo frame, senão um irmão depois dele
        // se sobrepõe ao desenho. É o que o `style.size` do `request_layout` faz, e é a metade da
        // decisão de arquitetura que nenhum outro teste toca: os testes de posição leem os nós, que
        // são posicionados pelo `prepaint` e sairiam certos mesmo com o diagrama medindo 0×0.
        let (direita, abaixo) = (nova_sonda(), nova_sonda());
        let (sd, sb) = (direita.clone(), abaixo.clone());
        let window = cx.add_window(move |_w, _cx| MedidorDoDiagrama {
            direita: sd,
            abaixo: sb,
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        // O diagrama é: nó de entrada, e um leque de dois ramos. Tudo com conteúdo de 40×20.
        let (largura_no, altura_no) = caixa_do_no(40.0, 20.0);
        let gaps = Gaps::default();
        // Largura: entrada + gap + o leque (a largura de um ramo).
        let largura_esperada = largura_no + gaps.column_gap + largura_no;
        // Altura: o ramo de baixo é quem vai mais longe — não é a soma de tudo.
        let altura_esperada = altura_no + gaps.row_gap + altura_no;

        assert!(
            (x_de(&direita) - largura_esperada).abs() < 0.5,
            "o irmão à direita começou em {} e o diagrama devia medir {largura_esperada}",
            x_de(&direita)
        );
        assert!(
            (y_de(&abaixo) - altura_esperada).abs() < 0.5,
            "o irmão abaixo começou em {} e o diagrama devia medir {altura_esperada} de altura",
            y_de(&abaixo)
        );
        // E a prova de que o teste morde: um diagrama que não declarasse tamanho mediria 0×0, e os
        // dois irmãos cairiam na origem.
        // O limiar é 50 e não 100 porque o `row_gap` do kumo é 16 (o do `flow-layout.ts`, ver a
        // declaração no doc de `flow_layout`): dois nós de 20px de conteúdo mais 16 de vão não
        // passam de 100. Cinquenta continua cumprindo o papel — distinguir de zero com folga.
        assert!(
            largura_esperada > 50.0 && altura_esperada > 50.0,
            "os dois números precisam estar longe de zero pro teste distinguir"
        );
    }

    // --- O nó cru ------------------------------------------------------------------------------

    struct MedidorDeNoCru {
        cru: Sonda,
        vizinho: Sonda,
    }

    impl Render for MedidorDeNoCru {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            div().child(
                Flow::new()
                    .node(FlowNode::new().bare().child(conteudo(50.0, 30.0, &self.cru)))
                    .node(FlowNode::new().child(conteudo(50.0, 30.0, &self.vizinho))),
            )
        }
    }

    #[gpui::test]
    fn no_cru_nao_tem_respiro_nem_borda(cx: &mut TestAppContext) {
        // `FlowNode::bare` é o nosso equivalente do `render` prop do kumo, e o que se pode medir
        // dele é justo o que importa pro layout: sem superfície, a caixa do nó É a do conteúdo — e
        // por isso o vizinho desloca 26px menos que deslocaria com a superfície.
        let (cru, vizinho) = (nova_sonda(), nova_sonda());
        let (sc, sv) = (cru.clone(), vizinho.clone());
        let window = cx.add_window(move |_w, _cx| MedidorDeNoCru {
            cru: sc,
            vizinho: sv,
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let distancia = x_de(&vizinho) - x_de(&cru);
        let gap = Gaps::default().column_gap;
        // Aqui o respiro NÃO se cancela entre os dois, como nos outros testes: o nó cru não tem, e
        // o vizinho tem. Então a conta é a largura do cru (só o conteúdo, 50) + o gap + o recuo do
        // conteúdo do vizinho dentro da superfície dele (12 de `px-3` + 1 de borda).
        let recuo_do_vizinho = 12.0 + 1.0;
        let esperado = 50.0 + gap + recuo_do_vizinho;
        assert!(
            (distancia - esperado).abs() < 0.5,
            "o vizinho ficou a {distancia} e a conta do nó cru pede {esperado}"
        );

        // E o contraste que dá dente ao teste: com superfície, a caixa do MESMO conteúdo é maior.
        let (com_superficie, _) = caixa_do_no(50.0, 30.0);
        assert!(
            com_superficie - 50.0 > 20.0,
            "a superfície acrescenta {} — é essa diferença que o teste pega",
            com_superficie - 50.0
        );
    }

    // --- As âncoras ----------------------------------------------------------------------------

    struct MedidorDeAncoras {
        cabeca: Sonda,
        pe: Sonda,
        vizinho: Sonda,
    }

    impl Render for MedidorDeAncoras {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            div().child(
                Flow::new()
                    .node(FlowNode::new().child(conteudo(30.0, 20.0, &self.vizinho)))
                    // Um nó alto com as duas âncoras: a de ENTRADA no cabeçalho e a de SAÍDA no pé
                    // — o desenho do segundo exemplo de âncora da spec.
                    .node(
                        FlowNode::new()
                            .anchor(AnchorKind::End, conteudo(80.0, 20.0, &self.cabeca))
                            .child(div().w(px(80.0)).h(px(60.0)))
                            .anchor(AnchorKind::Start, conteudo(80.0, 20.0, &self.pe)),
                    )
                    // E um terceiro com a âncora que governa as duas pontas, pra o ramo
                    // `AnchorKind::Both` (duas sondas no mesmo nó) também rodar.
                    .node(FlowNode::new().anchor(AnchorKind::Both, "fim")),
            )
        }
    }

    #[gpui::test]
    fn no_com_ancora_monta_e_empilha_o_conteudo_na_ordem(cx: &mut TestAppContext) {
        // O que este teste garante é o CAMINHO da âncora, não o efeito dela na seta (esse é
        // pintura, e pintura não se lê — está declarado em "Sem cobertura" no doc do módulo). Sem
        // ele, todo o ramo de `FlowNode::anchor` — as duas sondas, o `Both` que pendura duas no
        // mesmo nó, e a disciplina de `RefCell` entre o `prepaint` que escreve e o `paint` que lê —
        // nunca era executado por teste nenhum.
        let (cabeca, pe, vizinho) = (nova_sonda(), nova_sonda(), nova_sonda());
        let (sc, sp, sv) = (cabeca.clone(), pe.clone(), vizinho.clone());
        let window = cx.add_window(move |_w, _cx| MedidorDeAncoras {
            cabeca: sc,
            pe: sp,
            vizinho: sv,
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        // 1. As âncoras são conteúdo: elas empilham na ordem declarada, com os 60px do corpo entre
        //    as duas.
        let dy = y_de(&pe) - y_de(&cabeca);
        assert!(
            (dy - 80.0).abs() < 0.5,
            "cabeçalho e pé ficaram a {dy}: devia ser os 20 do cabeçalho mais os 60 do corpo"
        );

        // 2. E o nó com âncoras foi medido como qualquer outro — o vizinho deslocou pela largura
        //    dele, prova de que pendurar uma sonda dentro do nó não mexeu na medição.
        let (largura_vizinho, _) = caixa_do_no(30.0, 20.0);
        let distancia = x_de(&cabeca) - x_de(&vizinho);
        assert!(
            (distancia - (largura_vizinho + Gaps::default().column_gap)).abs() < 0.5,
            "o nó de âncoras ficou a {distancia} do vizinho, e a conta pede {}",
            largura_vizinho + Gaps::default().column_gap
        );

        // 3. O deslocamento que a âncora do pé produz aponta pra BAIXO do centro do nó — que é o
        //    ponto de existir uma âncora. A conta é a mesma do `paint`: o meio da caixa da âncora,
        //    medido do topo do nó.
        let topo_do_no = y_de(&cabeca) - NODE_PY - NODE_BORDER;
        let offset_do_pe = anchor_offset(Some(lido(&pe)), topo_do_no).expect("o pé foi medido");
        let altura_do_no = 20.0 + 60.0 + 20.0 + 2.0 * NODE_PY + 2.0 * NODE_BORDER;
        assert!(
            offset_do_pe > altura_do_no / 2.0,
            "a âncora do pé devia sair abaixo do centro ({}), e deu {offset_do_pe}",
            altura_do_no / 2.0
        );
    }

    // --- O nó de texto -------------------------------------------------------------------------

    struct MedidorDeTexto {
        curto: Sonda,
        longo: Sonda,
    }

    /// O rótulo longo do teste: 36 caracteres, 7 palavras.
    const ROTULO_LONGO: &str = "Uma etapa com nome bem mais comprido";

    impl Render for MedidorDeTexto {
        fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            // A sonda cobre o CONTEÚDO do nó, então o que ela mede é a caixa do texto — que é o
            // número onde a quebra de linha aparece.
            let envolve = |texto: &'static str, s: &Sonda| {
                let s = s.clone();
                div().relative().child(texto).child(
                    canvas(move |bounds, _w, _cx| s.set(Some(bounds)), |_, _, _, _| {})
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                )
            };
            div().child(
                Flow::new()
                    .node(FlowNode::new().child(envolve("Ok", &self.curto)))
                    .node(FlowNode::new().child(envolve(ROTULO_LONGO, &self.longo))),
            )
        }
    }

    #[gpui::test]
    fn no_de_texto_e_medido_sem_quebrar_a_linha(cx: &mut TestAppContext) {
        // O único tamanho que teste puro nenhum alcança: o de texto, que sai do sistema de fontes.
        //
        // ⚠️ Este teste NÃO prova a escolha de `AvailableSpace::MaxContent`: trocar por
        // `MinContent` o mantém verde, porque o caminho de texto do GPUI mede a linha inteira nos
        // dois casos. Está declarado em "Sem cobertura" no doc do módulo. O que ele prova é que a
        // medida saiu do CONTEÚDO — um nó de texto não tem tamanho declarado em lugar nenhum, e é
        // ele que decide onde o vizinho cai.
        let (curto, longo) = (nova_sonda(), nova_sonda());
        let (sc, sl) = (curto.clone(), longo.clone());
        let window = cx.add_window(move |_w, _cx| MedidorDeTexto {
            curto: sc,
            longo: sl,
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let largura_longa = f32::from(lido(&longo).size.width);
        let altura_longa = f32::from(lido(&longo).size.height);
        let largura_curta = f32::from(lido(&curto).size.width);

        // 1. **Uma linha só.** Quebrado em 7 palavras, o texto teria pelo menos o dobro desta
        //    altura.
        assert!(
            altura_longa < 1.5 * TEXT_SM_LINE,
            "o rótulo de {} caracteres mediu {altura_longa} de altura — a entrelinha é \
             {TEXT_SM_LINE}, então ele quebrou em mais de uma linha",
            ROTULO_LONGO.len()
        );

        // 2. E numa linha, ele é bem mais largo que a palavra mais longa dele. Com `MinContent` a
        //    largura cairia pra ~essa palavra, então este número é o outro lado da mesma prova.
        let maior_palavra = ROTULO_LONGO.split(' ').map(str::len).max().unwrap_or(0);
        assert!(
            largura_longa > 3.0 * maior_palavra as f32,
            "o rótulo inteiro numa linha mediu {largura_longa}, perto da maior palavra dele \
             ({maior_palavra} caracteres) — sinal de que quebrou"
        );

        // 3. O texto curto mede menos que o longo: o nó realmente acompanha o conteúdo, e não um
        //    tamanho fixo.
        assert!(
            largura_curta * 2.0 < largura_longa,
            "os dois nós mediram {largura_curta} e {largura_longa} — o tamanho não veio do conteúdo"
        );

        // 4. E o deslocamento do vizinho usou a largura MEDIDA, não só o gap.
        let distancia = x_de(&longo) - x_de(&curto);
        let gap = Gaps::default().column_gap;
        assert!(
            (distancia - (largura_curta + 2.0 * NODE_PX + 2.0 * NODE_BORDER + gap)).abs() < 0.5,
            "o vizinho ficou a {distancia}, que não é a largura medida do nó curto mais o gap"
        );
    }
}

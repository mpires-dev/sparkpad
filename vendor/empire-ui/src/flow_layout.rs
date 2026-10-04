//! `flow_layout` — a **matemática** do diagrama de fluxo, em funções puras.
//!
//! Este módulo é o porte do `flow-layout.ts` do [kumo] (design system da Cloudflare) mais a
//! parte geométrica do `connectors.tsx`. **Não depende de `gpui`**: entra `f32`, sai `f32`.
//! Toda decisão de posição, aresta e caminho vive aqui, então dá pra testar o componente
//! inteiro sem abrir janela. Quem desenha é [`crate::flow`].
//!
//! [kumo]: https://github.com/cloudflare/kumo/tree/main/packages/kumo/src/components/flow
//!
//! # As três fases
//!
//! O kumo divide o Flow em **medição** (lê o DOM), **layout** (deriva posições e arestas) e
//! **render** (desenha as arestas em SVG). Este módulo é a fase 2 inteira e a geometria da
//! fase 3; a fase 1 é do [`crate::flow`], que mede com `canvas`.
//!
//! O ponto que o kumo faz questão de repetir na spec: posições e arestas são **derivadas**, não
//! guardadas. Só as medidas são estado. Aqui é igual — [`compute_positions`] e
//! [`compute_edges`] são funções, não campos.
//!
//! # A árvore
//!
//! [`TreeNode`] é a estrutura que a medição produz: uma [`TreeNode::List`] encadeia os filhos em
//! sequência, uma [`TreeNode::Parallel`] os abre em ramos, e uma [`TreeNode::Node`] é a folha
//! que carrega um [`NodeId`]. O diagrama todo é uma lista na raiz.
//!
//! # As arestas
//!
//! Quatro regras, na ordem em que a spec as apresenta:
//!
//! 1. Nós adjacentes numa lista ligam do anterior pro seguinte (`A → B`).
//! 2. Um nó vizinho de um grupo paralelo liga em **todos** os filhos dele — o leque. Entrando:
//!    `A → B1`, `A → B2`. Saindo: `B1 → C`, `B2 → C`.
//! 3. Dois grupos paralelos **adjacentes não se interligam**. É a regra que mais surpreende:
//!    `A → B1`, `A → B2`, `C1 → D`, `C2 → D`, e **nada** entre os `B` e os `C`.
//! 4. Uma lista aninhada expõe pra fora só o **primeiro** (entrada) e o **último** (saída) nó.
//!
//! As regras 2 e 4 são o mesmo mecanismo visto de dois lados: [`entry_ids`] e [`exit_ids`]
//! descem a árvore devolvendo os pontos de contato de uma subárvore — todos os filhos, no
//! paralelo; só as pontas, na lista.
//!
//! # As posições
//!
//! Lista anda no eixo principal separada por `column_gap`; paralelo abre no eixo cruzado
//! separado por `row_gap`. Um grupo paralelo ocupa a **largura do filho mais largo**, e o
//! próximo nó da lista desloca por essa largura inteira mais o `column_gap` — é isso que
//! mantém o leque simétrico quando os ramos têm tamanhos diferentes. Origem em (0, 0), no
//! canto superior esquerdo, x crescendo pra direita e y pra baixo.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! ## Não reproduzível
//!
//! - **`Record<string, T>` com ids gerados pelo React.** O kumo identifica nó por string
//!   (`useId()`). Aqui o id é um [`NodeId`] denso (índice) atribuído por quem monta a árvore, e
//!   os mapas viram `Vec` indexada. Motivo: sem React não existe `useId`, e índice denso dá
//!   iteração determinística — que é o que faz os testes deste módulo serem estáveis.
//!   Consequência observável: um id fora do alcance de `nodes` não recebe posição, enquanto no
//!   kumo receberia uma entrada no `Record`. O retângulo do diagrama ignora esse id nos dois,
//!   e um id sem medida não tem nó pra desenhar, então o comportamento visível é o mesmo.
//! - **`markerEnd` do SVG.** O kumo declara a ponta da seta uma vez num `<marker>` e o
//!   navegador a repete em cada caminho, girada pela direção final (`orient="auto"`). Não há
//!   `<marker>` aqui: [`arrowhead`] extrai o ponto e o ângulo do último segmento **que tem
//!   direção** — que é o que `orient="auto"` faz — e [`arrowhead_outline`] devolve o contorno já
//!   transformado. O desenho da gota é o mesmo, ponto a ponto. O "que tem direção" não é zelo: num
//!   vão apertado a aparagem do cotovelo deixa o último segmento com comprimento zero, e ler a
//!   direção dele apontaria toda seta pra direita.
//!
//! ## Resolvido em número
//!
//! - **`row_gap` = 16, e a fonte do kumo se contradiz aqui.** A spec
//!   (`2026-04-17-flow-layout.md`) declara `rowGap = 48`; o `flow-layout.ts` que está no repo
//!   hoje traz `{ columnGap = 64, rowGap = 16 }`. Os dois números são do kumo.
//!
//!   Vale o **16**, porque é o que o componente FAZ: a spec é documento de projeto, e o código é
//!   o que roda e o que alguém vê na tela. Portar a spec por cima do código seria portar uma
//!   intenção, não o componente. O 48 fica registrado aqui — se um dia o kumo alinhar os dois, é
//!   deste parágrafo que sai a resposta de qual mudou. Configurável via [`Gaps`] de todo modo.
//! - **Ordem de pintura por `disabled`.** O kumo ordena os conectores no componente de render
//!   (desativado primeiro, pra ficar embaixo). [`compute_connectors`] faz a mesma ordenação —
//!   estável — na saída, porque ordem de pintura é dado, e dado se testa. Resultado idêntico.
//!
//! ## Desvio consciente
//!
//! Os dois desvios abaixo servem **um invariante que o kumo não garante**: *nenhum segmento de
//! nenhuma aresta entra na caixa de nenhum nó*. É a propriedade que decide se o diagrama parece
//! certo — as arestas são pintadas **depois** dos nós, então uma linha que fura uma caixa aparece
//! por cima dela e o desenho fica ilegível. Está travado pelos oito testes
//! `conector_nao_invade_no_*`, que rodam vãos apertados, ramos desiguais, dois e três ramos, e os
//! dois eixos.
//!
//! - **Os três deslocamentos do cotovelo são aparados pelo vão disponível** — ver [`fit_elbow`].
//!   O kumo usa constantes absolutas (perna a 32 da chegada, cantos de 8, 8 de folga pra seta), e
//!   elas só cabem em vão largo: com `column_gap` de 24, a perna cai **atrás da borda do nó de
//!   saída** e o caminho anda pra trás por dentro da caixa antes de curvar. Medido: com nós de 70
//!   de largura, o caminho saía de `x = 70` e voltava até `x = 54`.
//!
//!   **Isto não muda o desenho padrão.** Com o `column_gap` de 64, o `mid_offset` de 32 do kumo já
//!   é exatamente metade do vão — a constante dele é um meio-vão escrito à mão. Então pra vão de 64
//!   ou mais os três valores saem intactos e o caminho é idêntico ao do original; só vão menor
//!   muda, que é justo onde o original furava. Travado por `apara_nao_mexe_em_nada_no_vao_padrao`,
//!   e mais: **todos os testes de caminho herdados passaram sem uma expectativa alterada.**
//!
//! - **A perna do cotovelo respeita os ramos vizinhos** — ver [`free_main_span`]. Num grupo
//!   paralelo de ramos **desiguais**, a aresta que sai do ramo mais curto atravessa a faixa do ramo
//!   mais largo; se a perna ficar antes da borda dele, o traço corta a caixa dele em dois. O
//!   `createRoundedPath` do kumo só conhece as duas pontas da aresta e não tem como saber disso,
//!   então quem calcula o vão livre é o [`compute_connectors`], que vê todos os nós. Medido num
//!   leque de três ramos (90, 150 e 110) com `gaps(24.0, 8.0)`: a perna ia pra `x = 236`, dentro do
//!   segundo ramo, que ia até 244.
//!
//! Um terceiro desvio, menor, veio de brinde: os deslocamentos agora são aplicados **com o sinal da
//! direção de viagem** nos dois eixos. O kumo escreve `x2 - arrowhead_offset` no caso reto e os
//! cantos do eixo vertical sem sinal, o que faz uma aresta que sobe (ou que vai da direita pra
//! esquerda) passar do destino e voltar por dentro do nó. As arestas do componente sempre viajam
//! pra frente, então nenhum desenho muda — mas é o que deixa o invariante valer nas quatro
//! direções em vez de só em duas.
//!
//! O que **não** mudou, porque seria a saída errada: os vãos. [`Gaps`] continua aceitando
//! qualquer número sem aparar. Aparar ali sobrescreveria em silêncio o que quem monta o diagrama
//! pediu de propósito; e não é preciso, porque com o cotovelo aparado **qualquer** vão desenha
//! legível. Ramos colados num `row_gap` de 8 ficam apertados, e ficam apertados porque foi isso que
//! se pediu.
//!
//! ## Superset consciente
//!
//! - **[`PathCommand`] em vez de string SVG.** `createRoundedPath` devolve `"M 0 0 L ..."`.
//!   Aqui devolve uma lista de comandos tipados. Não é enfeite: é o que permite testar o
//!   caminho por valor em vez de por comparação de string, e é o que o `PathBuilder` do GPUI
//!   consome.
//!
//! ## Ausente
//!
//! - **Nada da fase de render que não seja geometria.** Cor, espessura, opacidade do
//!   desativado e a superfície do nó são do [`crate::flow`], porque saem dos nossos tokens.
//! - **Pan e barras de rolagem.** O `diagram.tsx` do kumo arrasta o diagrama com `framer-motion`
//!   e desenha duas barras de rolagem fantasma. Nada disso é layout, então nada disso está
//!   aqui; o [`crate::flow`] declara o que fez com isso.

// =============================================================================
// Tipos
// =============================================================================

/// Identificador de um nó do diagrama.
///
/// Denso e atribuído por quem monta a árvore: o primeiro nó é `NodeId(0)`, o segundo
/// `NodeId(1)`, e assim por diante. É o índice em [`FlowState::nodes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub usize);

/// A árvore que a fase de medição produz.
#[derive(Clone, Debug, PartialEq)]
pub enum TreeNode {
    /// Folha: um nó de verdade, com medida em [`FlowState::nodes`].
    Node { id: NodeId },
    /// Sequência: filhos encadeados no eixo principal.
    List { children: Vec<TreeNode> },
    /// Ramos: filhos abertos no eixo cruzado, todos ligados aos vizinhos da lista.
    Parallel {
        children: Vec<TreeNode>,
        align: ParallelAlign,
    },
}

/// Alinhamento dos ramos de um grupo paralelo no eixo principal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParallelAlign {
    /// Ramos alinhados pelo começo (esquerda, no fluxo horizontal). O padrão.
    #[default]
    Start,
    /// Ramos alinhados pelo fim — o ramo curto encosta na saída, não na entrada.
    End,
}

/// Alinhamento dos itens de uma lista no eixo **cruzado**.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlowAlign {
    /// Todos os itens na mesma borda (topo, no fluxo horizontal). O padrão.
    #[default]
    Start,
    /// Cada item centralizado na faixa do item mais alto.
    Center,
}

/// Eixo principal do diagrama.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlowOrientation {
    /// Listas andam pra direita, ramos empilham pra baixo. O padrão.
    #[default]
    Horizontal,
    /// Listas andam pra baixo, ramos abrem pra direita.
    Vertical,
}

/// O que a fase de medição descobriu sobre um nó.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeMeasure {
    pub width: f32,
    pub height: f32,
    /// Acinzenta os conectores que tocam este nó.
    pub disabled: bool,
    /// Deslocamento, a partir da borda superior do nó, do ponto de onde a seta **sai**.
    /// `None` = centro do nó. É o `startAnchorOffset` do kumo.
    pub start_anchor_offset: Option<f32>,
    /// Deslocamento, a partir da borda superior do nó, do ponto onde a seta **entra**.
    /// `None` = centro do nó. É o `endAnchorOffset` do kumo.
    pub end_anchor_offset: Option<f32>,
}

/// O estado do diagrama: medidas + árvore + as duas opções de alinhamento.
#[derive(Clone, Debug)]
pub struct FlowState {
    /// Medidas indexadas por [`NodeId`].
    pub nodes: Vec<NodeMeasure>,
    pub tree: TreeNode,
    pub align: FlowAlign,
    pub orientation: FlowOrientation,
}

impl Default for FlowState {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            tree: TreeNode::List {
                children: Vec::new(),
            },
            align: FlowAlign::default(),
            orientation: FlowOrientation::default(),
        }
    }
}

/// Os dois espaçamentos do diagrama.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gaps {
    /// Espaço entre itens de uma lista, no eixo principal.
    pub column_gap: f32,
    /// Espaço entre ramos de um grupo paralelo, no eixo cruzado.
    pub row_gap: f32,
}

impl Default for Gaps {
    /// Os números do `flow-layout.ts` do kumo: 64 no eixo principal, 16 entre ramos.
    fn default() -> Self {
        Self {
            column_gap: 64.,
            row_gap: 16.,
        }
    }
}

/// Uma aresta: do nó de origem pro nó de destino.
pub type Edge = (NodeId, NodeId);

/// Canto superior esquerdo de um nó, relativo à origem do diagrama.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// Largura × altura de uma subárvore (ou do diagrama inteiro).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DiagramRect {
    pub width: f32,
    pub height: f32,
}

/// Uma aresta já resolvida em dois pontos, pronta pra virar caminho.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Connector {
    pub from: NodeId,
    pub to: NodeId,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    /// Algum dos dois nós está desativado.
    pub disabled: bool,
    /// Curva as **duas** pontas (S suave). [`compute_connectors`] sempre liga.
    pub single: bool,
    /// Junta a curva na ponta de chegada em vez da de saída.
    pub is_bottom: bool,
    /// Quanto do eixo principal, contado a partir da CHEGADA, está livre de nós.
    ///
    /// `None` = a distância inteira entre as duas pontas, que é o caso comum. Fica menor quando um
    /// ramo mais largo do mesmo grupo paralelo avança no corredor por onde esta aresta passa: aí a
    /// perna do cotovelo tem que ficar **depois** dele, senão o traço atravessa a caixa dele. Quem
    /// calcula é o [`compute_connectors`], que é quem conhece todos os nós.
    pub free_main_span: Option<f32>,
}

/// Um comando de caminho, no subconjunto que o kumo usa: `M`, `L` e `Q`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo { x: f32, y: f32 },
    LineTo { x: f32, y: f32 },
    QuadTo { cx: f32, cy: f32, x: f32, y: f32 },
}

impl PathCommand {
    /// O ponto onde o comando termina.
    pub fn end(&self) -> (f32, f32) {
        match *self {
            PathCommand::MoveTo { x, y } | PathCommand::LineTo { x, y } => (x, y),
            PathCommand::QuadTo { x, y, .. } => (x, y),
        }
    }
}

/// Ajustes geométricos do caminho de uma aresta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathOptions {
    /// Raio máximo dos dois cantos. Cai pra metade da distância no eixo cruzado quando o
    /// espaço é curto.
    pub corner_radius: f32,
    /// Onde fica o trecho do meio (a "perna" do S), medido a partir da ponta.
    pub mid_offset: f32,
    /// Quanto o caminho para **antes** do ponto de chegada, pra abrir espaço pra ponta da seta.
    pub arrowhead_offset: f32,
    pub orientation: FlowOrientation,
}

impl Default for PathOptions {
    /// Os padrões do `createRoundedPath` do kumo.
    fn default() -> Self {
        Self {
            corner_radius: 8.,
            mid_offset: 32.,
            arrowhead_offset: 8.,
            orientation: FlowOrientation::Horizontal,
        }
    }
}

/// Onde e com que giro desenhar a ponta da seta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arrowhead {
    /// Ponto de encaixe: o fim do caminho. A gota cresce daqui pra frente.
    pub x: f32,
    pub y: f32,
    /// Giro, em radianos, tirado da direção do último segmento.
    pub angle: f32,
}

/// Distância máxima, no eixo cruzado, em que a aresta sai reta em vez de curvar.
///
/// O `FLAT_THRESHOLD` do kumo.
pub const FLAT_THRESHOLD: f32 = 2.;

// =============================================================================
// compute_edges
// =============================================================================

/// Deriva as arestas da árvore, pelas quatro regras da spec.
///
/// Puro: não olha medida nenhuma, só estrutura.
pub fn compute_edges(state: &FlowState) -> Vec<Edge> {
    let mut edges = Vec::new();
    collect_edges(&state.tree, &mut edges);
    edges
}

/// Pontos por onde uma seta **entra** nesta subárvore.
///
/// Folha: ela mesma. Paralelo: todos os filhos (o leque da regra 2). Lista: só o primeiro
/// filho (a regra 4).
fn entry_ids(node: &TreeNode) -> Vec<NodeId> {
    match node {
        TreeNode::Node { id } => vec![*id],
        TreeNode::Parallel { children, .. } => children.iter().flat_map(entry_ids).collect(),
        TreeNode::List { children } => children.first().map(entry_ids).unwrap_or_default(),
    }
}

/// Pontos por onde uma seta **sai** desta subárvore.
///
/// Espelho de [`entry_ids`]: na lista, o último filho em vez do primeiro.
fn exit_ids(node: &TreeNode) -> Vec<NodeId> {
    match node {
        TreeNode::Node { id } => vec![*id],
        TreeNode::Parallel { children, .. } => children.iter().flat_map(exit_ids).collect(),
        TreeNode::List { children } => children.last().map(exit_ids).unwrap_or_default(),
    }
}

/// Desce a árvore juntando arestas.
///
/// Folha não gera nada. Paralelo desce nos filhos **sem** ligá-los entre si — ramos paralelos
/// são paralelos justamente por não se encadearem. Lista desce nos filhos e depois costura os
/// pares vizinhos.
fn collect_edges(node: &TreeNode, edges: &mut Vec<Edge>) {
    match node {
        TreeNode::Node { .. } => {}
        TreeNode::Parallel { children, .. } => {
            for child in children {
                collect_edges(child, edges);
            }
        }
        TreeNode::List { children } => {
            for child in children {
                collect_edges(child, edges);
            }

            for pair in children.windows(2) {
                let (current, next) = (&pair[0], &pair[1]);

                // Regra 3: dois grupos paralelos adjacentes não se interligam.
                if matches!(current, TreeNode::Parallel { .. })
                    && matches!(next, TreeNode::Parallel { .. })
                {
                    continue;
                }

                for from in exit_ids(current) {
                    for to in entry_ids(next) {
                        edges.push((from, to));
                    }
                }
            }
        }
    }
}

// =============================================================================
// compute_positions
// =============================================================================

/// Deriva a posição absoluta de cada nó.
///
/// Devolve um `Vec` do tamanho de [`FlowState::nodes`]: `Some` pra todo nó que aparece na
/// árvore, `None` pros que não aparecem. Um [`NodeId`] da árvore que caia fora de `nodes`
/// conta como tamanho zero e não recebe posição.
///
/// Puro: não olha o DOM porque não existe DOM.
pub fn compute_positions(state: &FlowState, gaps: Gaps) -> Vec<Option<Position>> {
    let mut out = vec![None; state.nodes.len()];
    let layout = Layout { state, gaps };
    layout.run(&state.tree, 0., 0., &mut out);
    out
}

/// Largura × altura de uma subárvore.
type Size = (f32, f32);

struct Layout<'a> {
    state: &'a FlowState,
    gaps: Gaps,
}

impl Layout<'_> {
    /// Posiciona uma subárvore com o canto superior esquerdo em (`origin_x`, `origin_y`),
    /// escrevendo em `out`, e devolve o tamanho que ela ocupou.
    fn run(
        &self,
        node: &TreeNode,
        origin_x: f32,
        origin_y: f32,
        out: &mut Vec<Option<Position>>,
    ) -> Size {
        match node {
            TreeNode::Node { id } => {
                let measured = self.state.nodes.get(id.0).copied().unwrap_or_default();
                if let Some(slot) = out.get_mut(id.0) {
                    *slot = Some(Position {
                        x: origin_x,
                        y: origin_y,
                    });
                }
                (measured.width, measured.height)
            }
            TreeNode::List { children } => self.list(children, origin_x, origin_y, out),
            TreeNode::Parallel { children, align } => {
                self.parallel(children, *align, origin_x, origin_y, out)
            }
        }
    }

    /// Mede os filhos sem sujar o resultado — a primeira das duas passadas.
    ///
    /// É o `layout(child, 0, 0, {})` do kumo: posiciona num mapa descartável só pra saber o
    /// tamanho de cada filho antes de decidir onde encaixá-los.
    fn measure(&self, children: &[TreeNode]) -> Vec<Size> {
        let mut scratch = vec![None; self.state.nodes.len()];
        children
            .iter()
            .map(|child| self.run(child, 0., 0., &mut scratch))
            .collect()
    }

    fn list(
        &self,
        children: &[TreeNode],
        origin_x: f32,
        origin_y: f32,
        out: &mut Vec<Option<Position>>,
    ) -> Size {
        let gap = self.gaps.column_gap;
        let vertical = self.state.orientation == FlowOrientation::Vertical;
        let centered = self.state.align == FlowAlign::Center;

        match (vertical, centered) {
            // Lista vertical centralizada: todos na faixa do filho mais largo.
            (true, true) => {
                let sizes = self.measure(children);
                let column_width = max_of(sizes.iter().map(|s| s.0));

                let mut cursor_y = origin_y;
                for (i, child) in children.iter().enumerate() {
                    let child_x = origin_x + (column_width - sizes[i].0) / 2.;
                    self.run(child, child_x, cursor_y, out);
                    cursor_y += sizes[i].1;
                    if i < children.len() - 1 {
                        cursor_y += gap;
                    }
                }

                (column_width, cursor_y - origin_y)
            }
            // Lista vertical: de cima pra baixo, todos na mesma coluna.
            (true, false) => {
                let mut cursor_y = origin_y;
                let mut total_width: f32 = 0.;

                for (i, child) in children.iter().enumerate() {
                    let (width, height) = self.run(child, origin_x, cursor_y, out);
                    cursor_y += height;
                    if i < children.len() - 1 {
                        cursor_y += gap;
                    }
                    total_width = total_width.max(width);
                }

                (total_width, cursor_y - origin_y)
            }
            // Lista horizontal centralizada: todos na faixa do filho mais alto.
            (false, true) => {
                let sizes = self.measure(children);
                let row_height = max_of(sizes.iter().map(|s| s.1));

                let mut cursor_x = origin_x;
                for (i, child) in children.iter().enumerate() {
                    let child_y = origin_y + (row_height - sizes[i].1) / 2.;
                    self.run(child, cursor_x, child_y, out);
                    cursor_x += sizes[i].0;
                    if i < children.len() - 1 {
                        cursor_x += gap;
                    }
                }

                (cursor_x - origin_x, row_height)
            }
            // Lista horizontal: da esquerda pra direita, todos na mesma linha. O padrão.
            (false, false) => {
                let mut cursor_x = origin_x;
                let mut total_height: f32 = 0.;

                for (i, child) in children.iter().enumerate() {
                    let (width, height) = self.run(child, cursor_x, origin_y, out);
                    cursor_x += width;
                    if i < children.len() - 1 {
                        cursor_x += gap;
                    }
                    total_height = total_height.max(height);
                }

                (cursor_x - origin_x, total_height)
            }
        }
    }

    fn parallel(
        &self,
        children: &[TreeNode],
        align: ParallelAlign,
        origin_x: f32,
        origin_y: f32,
        out: &mut Vec<Option<Position>>,
    ) -> Size {
        let gap = self.gaps.row_gap;
        let vertical = self.state.orientation == FlowOrientation::Vertical;
        let at_end = align == ParallelAlign::End;

        match (vertical, at_end) {
            // Paralelo vertical alinhado pelo fim: ramos encostam na borda de baixo.
            (true, true) => {
                let sizes = self.measure(children);
                let max_height = max_of(sizes.iter().map(|s| s.1));

                let mut cursor_x = origin_x;
                for (i, child) in children.iter().enumerate() {
                    let child_y = origin_y + max_height - sizes[i].1;
                    self.run(child, cursor_x, child_y, out);
                    cursor_x += sizes[i].0;
                    if i < children.len() - 1 {
                        cursor_x += gap;
                    }
                }

                (cursor_x - origin_x, max_height)
            }
            // Paralelo vertical: ramos abrem pra direita.
            (true, false) => {
                let mut cursor_x = origin_x;
                let mut max_height: f32 = 0.;

                for (i, child) in children.iter().enumerate() {
                    let (width, height) = self.run(child, cursor_x, origin_y, out);
                    max_height = max_height.max(height);
                    cursor_x += width;
                    if i < children.len() - 1 {
                        cursor_x += gap;
                    }
                }

                (cursor_x - origin_x, max_height)
            }
            // Paralelo horizontal alinhado pelo fim: ramos encostam na borda direita.
            (false, true) => {
                let sizes = self.measure(children);
                let max_width = max_of(sizes.iter().map(|s| s.0));

                let mut cursor_y = origin_y;
                for (i, child) in children.iter().enumerate() {
                    let child_x = origin_x + max_width - sizes[i].0;
                    self.run(child, child_x, cursor_y, out);
                    cursor_y += sizes[i].1;
                    if i < children.len() - 1 {
                        cursor_y += gap;
                    }
                }

                (max_width, cursor_y - origin_y)
            }
            // Paralelo horizontal: ramos empilham pra baixo, e o grupo ocupa a largura do
            // filho mais largo. O padrão. É esta linha que mantém o leque simétrico.
            (false, false) => {
                let mut cursor_y = origin_y;
                let mut max_width: f32 = 0.;

                for (i, child) in children.iter().enumerate() {
                    let (width, height) = self.run(child, origin_x, cursor_y, out);
                    max_width = max_width.max(width);
                    cursor_y += height;
                    if i < children.len() - 1 {
                        cursor_y += gap;
                    }
                }

                (max_width, cursor_y - origin_y)
            }
        }
    }
}

/// Maior valor da sequência, ou zero se ela estiver vazia.
fn max_of(values: impl Iterator<Item = f32>) -> f32 {
    values.fold(0., f32::max)
}

// =============================================================================
// compute_diagram_rect
// =============================================================================

/// Retângulo que envolve o diagrama inteiro.
///
/// Largura = maior `x + largura`; altura = maior `y + altura`. Note que **não** é a soma dos
/// tamanhos: com ramos de tamanhos desiguais, quem manda é a borda que foi mais longe.
pub fn compute_diagram_rect(positions: &[Option<Position>], state: &FlowState) -> DiagramRect {
    let mut rect = DiagramRect::default();

    for (index, position) in positions.iter().enumerate() {
        let Some(position) = position else { continue };
        let Some(node) = state.nodes.get(index) else {
            continue;
        };
        rect.width = rect.width.max(position.x + node.width);
        rect.height = rect.height.max(position.y + node.height);
    }

    rect
}

// =============================================================================
// compute_connectors
// =============================================================================

/// Resolve cada aresta em dois pontos concretos.
///
/// No fluxo horizontal a seta sai da borda **direita** do nó de origem e entra pela borda
/// **esquerda** do de destino; o `y` é o centro do nó, ou a âncora, quando houver. No fluxo
/// vertical sai por **baixo** e entra por **cima**, no centro horizontal — e aí as âncoras
/// **não** participam, porque são deslocamentos verticais (é assim no kumo).
///
/// Arestas cujos nós não têm posição são descartadas em silêncio: é o frame em que a medição
/// ainda não chegou.
///
/// A saída vem ordenada com os desativados na frente, pra que sejam pintados **embaixo** dos
/// ativos.
pub fn compute_connectors(
    edges: &[Edge],
    positions: &[Option<Position>],
    state: &FlowState,
    orientation: FlowOrientation,
) -> Vec<Connector> {
    let mut connectors: Vec<Connector> = Vec::with_capacity(edges.len());

    for &(from, to) in edges {
        let (Some(Some(from_pos)), Some(Some(to_pos))) =
            (positions.get(from.0), positions.get(to.0))
        else {
            continue;
        };
        let (Some(from_node), Some(to_node)) = (state.nodes.get(from.0), state.nodes.get(to.0))
        else {
            continue;
        };

        let (x1, y1, x2, y2) = match orientation {
            FlowOrientation::Vertical => (
                from_pos.x + from_node.width / 2.,
                from_pos.y + from_node.height,
                to_pos.x + to_node.width / 2.,
                to_pos.y,
            ),
            FlowOrientation::Horizontal => (
                from_pos.x + from_node.width,
                from_pos.y
                    + from_node
                        .start_anchor_offset
                        .unwrap_or(from_node.height / 2.),
                to_pos.x,
                to_pos.y + to_node.end_anchor_offset.unwrap_or(to_node.height / 2.),
            ),
        };

        connectors.push(Connector {
            from,
            to,
            x1,
            y1,
            x2,
            y2,
            disabled: from_node.disabled || to_node.disabled,
            single: true,
            is_bottom: false,
            free_main_span: Some(free_main_span(
                orientation,
                (x1, y1),
                (x2, y2),
                positions,
                state,
            )),
        });
    }

    // Ordenação estável: desativado primeiro, o resto na ordem das arestas.
    connectors.sort_by_key(|connector| !connector.disabled);
    connectors
}

/// Quanto tanto do eixo principal está livre de nós entre a saída e a chegada.
///
/// # Por que isto precisa existir
///
/// A perna do cotovelo é um segmento reto no eixo **cruzado**: ela atravessa toda a faixa entre as
/// duas pontas da aresta. Num grupo paralelo de ramos **desiguais**, a aresta que sai do ramo mais
/// curto pra o nó de saída tem que passar por cima da faixa do ramo mais **largo** — e se a perna
/// ficar antes da borda direita dele, o traço corta a caixa dele em dois.
///
/// Medido num leque de três ramos (90, 150 e 110 de largura) com `gaps(24.0, 8.0)`: a aresta do
/// terceiro ramo pro nó de saída punha a perna em `x = 236`, dentro do segundo ramo, que ia de 94 a
/// **244**.
///
/// O [`rounded_path`] só conhece as duas pontas da aresta, então não tem como saber disso. Quem
/// sabe é esta função, que vê todos os nós — e o número que ela devolve é o vão que o
/// [`fit_elbow`] tem de verdade pra trabalhar.
///
/// # A regra
///
/// Um nó **atrapalha** quando (a) a faixa dele no eixo cruzado se sobrepõe à da aresta, e (b) a
/// caixa dele invade o corredor **aberto** entre as duas pontas. Tocar as pontas não conta: o nó de
/// saída encosta na origem da aresta e o de chegada no destino dela, e nenhum dos dois atrapalha.
/// O resultado é a menor distância entre a chegada e a borda de um atrapalhador.
fn free_main_span(
    orientation: FlowOrientation,
    from: (f32, f32),
    to: (f32, f32),
    positions: &[Option<Position>],
    state: &FlowState,
) -> f32 {
    /// Folga pra "encostar" não contar como "atrapalhar".
    const EPSILON: f32 = 0.01;

    let horizontal = orientation == FlowOrientation::Horizontal;
    // (principal, cruzado) das duas pontas.
    let (a1, c1) = if horizontal { from } else { (from.1, from.0) };
    let (a2, c2) = if horizontal { to } else { (to.1, to.0) };

    let sign = if a2 > a1 { 1. } else { -1. };
    let (cross_lo, cross_hi) = (c1.min(c2), c1.max(c2));

    let mut livre = (a2 - a1).abs();

    for (index, position) in positions.iter().enumerate() {
        let (Some(position), Some(node)) = (position, state.nodes.get(index)) else {
            continue;
        };
        // A caixa do nó, nos eixos da aresta.
        let (a_lo, a_hi, c_lo, c_hi) = if horizontal {
            (
                position.x,
                position.x + node.width,
                position.y,
                position.y + node.height,
            )
        } else {
            (
                position.y,
                position.y + node.height,
                position.x,
                position.x + node.width,
            )
        };

        // A faixa cruzada tem que se sobrepor de verdade: só atrapalha quem está na altura por onde
        // a perna passa. Encostar não conta.
        if c_hi <= cross_lo + EPSILON || c_lo >= cross_hi - EPSILON {
            continue;
        }

        // A borda do nó no sentido da viagem, e o que sobra dela até a chegada.
        //
        // Estas duas guardas são também o teste de "está no corredor": `restante >= 0` descarta
        // quem já passou da chegada (o próprio nó de destino, entre outros) e `restante < livre`
        // descarta quem está antes da saída (o nó de origem). Uma condição escrita à parte pro
        // corredor seria código morto — a mutação confirmou, ao não matar teste nenhum ao removê-la.
        let borda = if sign > 0. { a_hi } else { a_lo };
        let restante = (a2 - borda) * sign;
        if restante >= 0. && restante < livre {
            livre = restante;
        }
    }

    livre
}

// =============================================================================
// rounded_path
// =============================================================================

/// Os três deslocamentos do cotovelo, já aparados pelo vão que existe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Elbow {
    /// Raio dos dois cantos.
    pub corner_radius: f32,
    /// A que distância da chegada fica a perna do S.
    pub mid_offset: f32,
    /// Quanto o caminho para antes da chegada, pra abrir espaço pra ponta da seta.
    pub arrowhead_offset: f32,
}

/// Apara os três deslocamentos do cotovelo pra que ele **caiba no vão** entre os dois nós.
///
/// # O problema que isto resolve
///
/// O `createRoundedPath` do kumo usa três constantes absolutas: perna a **32** da chegada, cantos
/// de **8**, e **8** de folga pra ponta da seta. No eixo principal, o cotovelo consome
/// `2 × raio + folga` = 24, e a perna precisa de 32 de espaço atrás da chegada. Com o vão padrão de
/// 64 sobra de tudo — mas com um vão de 24 a perna cai **atrás da borda do nó de saída**, e o
/// caminho sai andando pra trás por dentro da caixa antes de curvar. Como as arestas são pintadas
/// **depois** dos nós, a linha aparece atravessando o nó, e o desenho fica ilegível.
///
/// Medido na história `gaps(24.0, 8.0)`: com nós de 70 de largura, o caminho saía de `x = 70` (a
/// borda) e voltava até `x = 54`, ou seja **16px dentro** do nó.
///
/// # A regra
///
/// Os três deslocamentos passam a sair do vão em vez de serem absolutos:
///
/// 1. O raio segue a regra do kumo (metade da distância no eixo **cruzado**) e ganha um segundo
///    teto: junto com a folga da seta, os dois cantos têm que caber no eixo **principal**. Quando
///    não cabem, raio e folga encolhem **na mesma proporção**, o que mantém a forma do cotovelo em
///    vez de achatar só uma parte dele.
/// 2. A perna nunca fica a mais de **metade do vão** da chegada — ou seja, no aperto ela vai pro
///    MEIO do vão, que é o único lugar onde os dois cantos têm o mesmo espaço dos dois lados.
/// 3. E nunca tão perto da chegada que a aproximação final ande pra trás, nem tão perto da saída
///    que o primeiro canto fure a borda: a posição dela é presa em `[raio + folga, vão − raio −
///    folga]`.
///
/// # Por que isto não muda o desenho padrão
///
/// Com o `column_gap` de 64, **`mid_offset = 32` já é exatamente metade do vão** — a constante do
/// kumo é um meio-vão escrito à mão. Então pra qualquer vão de 64 ou mais os três valores saem
/// intactos (32, 8, 8) e o caminho é **idêntico** ao do original, byte a byte. Só vão menor que 64
/// muda, que é justo onde o original furava. Travado por
/// `apara_nao_mexe_em_nada_no_vao_padrao`.
fn fit_elbow(
    main: f32,
    cross: f32,
    max_corner_radius: f32,
    mid_offset: f32,
    arrowhead_offset: f32,
) -> Elbow {
    // Regra 1a, a do kumo: o raio não passa da metade da distância no eixo cruzado, pra a curva
    // não estourar quando o desnível é pequeno.
    let raio_cruzado = max_corner_radius.min((cross / 2.).abs());

    // Regra 1b, nova: os dois cantos mais a folga da seta têm que caber no eixo principal. Quando
    // não cabem, encolhem juntos, na mesma proporção.
    let preciso = 2. * raio_cruzado + arrowhead_offset;
    // `main` nunca é negativo e aqui `preciso > main`, então a razão cai sempre em [0, 1).
    let escala = if preciso > main && preciso > 0. {
        main / preciso
    } else {
        1.
    };
    let corner_radius = raio_cruzado * escala;
    let arrowhead_offset = arrowhead_offset * escala;

    // Regras 2 e 3: a perna vai pro meio do vão quando aperta, e nunca chega tão perto da chegada
    // que a aproximação final ande pra trás.
    //
    // O teto (`perna + raio <= vão`, pra o primeiro canto não furar a borda de saída) **não precisa
    // ser escrito**: `perna` já é no máximo `vão / 2`, e quando `vão / 2` é menor que o piso quem
    // vale é o piso, que a regra 1b garante caber. Escrever o teto seria código morto — foi a
    // mutação que mostrou isso, ao não conseguir matar teste nenhum ao removê-lo.
    let piso = corner_radius + arrowhead_offset;
    let mid_offset = mid_offset.min(main / 2.).max(piso);

    Elbow {
        corner_radius,
        mid_offset,
        arrowhead_offset,
    }
}

/// Monta o caminho de uma aresta: reta quando dá, senão um S com dois cantos arredondados.
///
/// Porte do `createRoundedPath`, com **os três deslocamentos aparados pelo vão disponível** — ver
/// [`fit_elbow`], que é a diferença consciente em relação ao original e a razão de o invariante
/// "conector nenhum invade nó nenhum" valer pra qualquer [`Gaps`].
///
/// Uma assimetria do original foi **corrigida** de propósito: aqui os deslocamentos são todos
/// aplicados **com o sinal da direção de viagem**, nos dois eixos. O kumo escreve `x2 -
/// arrowhead_offset` no caso reto e `horizontal_y ± corner_radius` no eixo vertical sem sinal
/// nenhum, o que faz uma aresta que sobe (ou que vai da direita pra esquerda) passar do destino e
/// voltar por dentro do nó. Como as arestas do componente sempre viajam pra frente, isso não muda
/// desenho nenhum na prática — mas é o que deixa o invariante valer nas quatro direções em vez de
/// só em duas.
pub fn rounded_path(connector: &Connector, options: PathOptions) -> Vec<PathCommand> {
    let Connector {
        x1,
        y1,
        x2,
        y2,
        single,
        is_bottom,
        free_main_span,
        ..
    } = *connector;
    let PathOptions {
        corner_radius: max_corner_radius,
        mid_offset,
        arrowhead_offset,
        orientation,
    } = options;

    let horizontal = orientation == FlowOrientation::Horizontal;
    // O eixo PRINCIPAL é onde a aresta viaja; o CRUZADO é onde ela se desloca.
    let (main, cross) = if horizontal {
        (x2 - x1, y2 - y1)
    } else {
        (y2 - y1, x2 - x1)
    };

    let horizontal_sign = if x2 > x1 { 1. } else { -1. };
    let vertical_sign = if y2 > y1 { 1. } else { -1. };
    // O sinal da viagem: `+1` pra frente no eixo principal.
    let main_sign = if horizontal {
        horizontal_sign
    } else {
        vertical_sign
    };

    // O vão de verdade: a distância entre as pontas, ou menos, se um ramo mais largo avança no
    // corredor — ver [`free_main_span`].
    let livre = free_main_span.unwrap_or(main.abs()).clamp(0., main.abs());
    let Elbow {
        corner_radius,
        mid_offset,
        arrowhead_offset,
    } = fit_elbow(livre, cross, max_corner_radius, mid_offset, arrowhead_offset);

    let mut commands = vec![PathCommand::MoveTo { x: x1, y: y1 }];

    if horizontal {
        if cross.abs() <= FLAT_THRESHOLD {
            commands.push(PathCommand::LineTo {
                x: x2 - main_sign * arrowhead_offset,
                y: y2,
            });
            return commands;
        }

        // Onde fica a perna vertical do S. Os deslocamentos vão com o sinal da viagem — ver o doc
        // desta função.
        let vertical_x = if single || is_bottom {
            x2 - main_sign * mid_offset
        } else {
            x1 + main_sign * mid_offset
        };
        let first_horizontal_end = vertical_x - horizontal_sign * corner_radius;
        let vertical_start = y1 + vertical_sign * corner_radius;
        let vertical_end = y2 - vertical_sign * corner_radius;
        let second_horizontal_start = vertical_x + horizontal_sign * corner_radius;
        let path_end_x = x2 - horizontal_sign * arrowhead_offset;

        if is_bottom {
            commands.push(PathCommand::LineTo {
                x: first_horizontal_end,
                y: y1,
            });
            commands.push(PathCommand::QuadTo {
                cx: vertical_x,
                cy: y1,
                x: vertical_x,
                y: vertical_start,
            });
            if single {
                commands.push(PathCommand::LineTo {
                    x: vertical_x,
                    y: vertical_end,
                });
                commands.push(PathCommand::QuadTo {
                    cx: vertical_x,
                    cy: y2,
                    x: second_horizontal_start,
                    y: y2,
                });
            } else {
                commands.push(PathCommand::LineTo {
                    x: vertical_x,
                    y: y2,
                });
            }
        } else {
            if single {
                commands.push(PathCommand::LineTo {
                    x: first_horizontal_end,
                    y: y1,
                });
                commands.push(PathCommand::QuadTo {
                    cx: vertical_x,
                    cy: y1,
                    x: vertical_x,
                    y: vertical_start,
                });
            } else {
                commands.push(PathCommand::LineTo {
                    x: vertical_x,
                    y: y1,
                });
            }
            commands.push(PathCommand::LineTo {
                x: vertical_x,
                y: vertical_end,
            });
            commands.push(PathCommand::QuadTo {
                cx: vertical_x,
                cy: y2,
                x: second_horizontal_start,
                y: y2,
            });
        }

        commands.push(PathCommand::LineTo {
            x: path_end_x,
            y: y2,
        });
        return commands;
    }

    if cross.abs() <= FLAT_THRESHOLD {
        commands.push(PathCommand::LineTo {
            x: x2,
            y: y2 - main_sign * arrowhead_offset,
        });
        return commands;
    }

    // Onde fica a perna horizontal do S.
    let horizontal_y = if single || is_bottom {
        y2 - main_sign * mid_offset
    } else {
        y1 + main_sign * mid_offset
    };
    // Aqui os dois cantos também vão com o sinal da viagem. O kumo os escreve sem sinal, o que faz
    // uma aresta que SOBE curvar pro lado errado — ver o doc desta função.
    let first_vertical_end = horizontal_y - main_sign * corner_radius;
    let horizontal_start = x1 + horizontal_sign * corner_radius;
    let horizontal_end = x2 - horizontal_sign * corner_radius;
    let second_vertical_start = horizontal_y + main_sign * corner_radius;
    let path_end_y = y2 - vertical_sign * arrowhead_offset;

    if is_bottom {
        commands.push(PathCommand::LineTo {
            x: x1,
            y: first_vertical_end,
        });
        commands.push(PathCommand::QuadTo {
            cx: x1,
            cy: horizontal_y,
            x: horizontal_start,
            y: horizontal_y,
        });
        if single {
            commands.push(PathCommand::LineTo {
                x: horizontal_end,
                y: horizontal_y,
            });
            commands.push(PathCommand::QuadTo {
                cx: x2,
                cy: horizontal_y,
                x: x2,
                y: second_vertical_start,
            });
        } else {
            commands.push(PathCommand::LineTo {
                x: x2,
                y: horizontal_y,
            });
        }
    } else {
        if single {
            commands.push(PathCommand::LineTo {
                x: x1,
                y: first_vertical_end,
            });
            commands.push(PathCommand::QuadTo {
                cx: x1,
                cy: horizontal_y,
                x: horizontal_start,
                y: horizontal_y,
            });
        } else {
            commands.push(PathCommand::LineTo {
                x: x1,
                y: horizontal_y,
            });
        }
        commands.push(PathCommand::LineTo {
            x: horizontal_end,
            y: horizontal_y,
        });
        commands.push(PathCommand::QuadTo {
            cx: x2,
            cy: horizontal_y,
            x: x2,
            y: second_vertical_start,
        });
    }

    commands.push(PathCommand::LineTo {
        x: x2,
        y: path_end_y,
    });
    commands
}

// =============================================================================
// arrowhead
// =============================================================================

/// Onde encaixar a ponta da seta, e com que giro.
///
/// Faz o que o `orient="auto"` do `<marker>` faz: pega o **último segmento** do caminho e usa
/// a direção dele. Devolve `None` pra caminho com menos de dois comandos.
///
/// Quando o último segmento tem comprimento zero, o ângulo cai em 0 (aponta pra direita) —
/// não há direção pra ler.
pub fn arrowhead(commands: &[PathCommand]) -> Option<Arrowhead> {
    let last = commands.last()?;
    let (x, y) = last.end();
    if commands.len() < 2 {
        return None;
    }

    // Anda de trás pra frente até achar um segmento com direção.
    //
    // ⚠️ Não é zelo: num vão apertado, o [`fit_elbow`] pode aparar a aproximação final até ela ter
    // comprimento ZERO — a perna do S encosta no ponto onde a seta começa. Aí o último segmento não
    // tem direção, e ler o ângulo dele daria 0 (apontando pra direita) em qualquer aresta,
    // inclusive nas verticais. Quem manda então é o último segmento que de fato andou.
    for i in (1..commands.len()).rev() {
        let (from_x, from_y) = match commands[i] {
            // A direção de uma quadrática no fim é a do controle pro ponto final.
            PathCommand::QuadTo { cx, cy, .. } => (cx, cy),
            _ => commands[i - 1].end(),
        };
        let (to_x, to_y) = commands[i].end();
        let (dx, dy) = (to_x - from_x, to_y - from_y);
        if dx != 0. || dy != 0. {
            return Some(Arrowhead {
                x,
                y,
                angle: dy.atan2(dx),
            });
        }
    }

    // Caminho inteiramente degenerado (dois nós exatamente sobrepostos): sem direção pra ler.
    Some(Arrowhead { x, y, angle: 0. })
}

/// A gota da ponta da seta, já girada e posicionada.
///
/// Mesmos pontos do `<path>` dentro do `<marker>` do kumo, numa caixa 8×8 cujo ponto de
/// encaixe é `(0, 4)` — a metade da borda esquerda, como o `refX="0" refY="4"` manda. A ponta
/// afiada cai a 8px à frente do fim do caminho, que é a borda do nó de destino.
pub fn arrowhead_outline(head: &Arrowhead) -> Vec<PathCommand> {
    /// `M 0,1.5 Q 0,0 1.5,0 Q 3.5,1 5.8,3.2 Q 6.5,4 5.8,4.8 Q 3.5,7 1.5,8 Q 0,8 0,6.5 Z`
    const OUTLINE: [PathCommand; 6] = [
        PathCommand::MoveTo { x: 0., y: 1.5 },
        PathCommand::QuadTo {
            cx: 0.,
            cy: 0.,
            x: 1.5,
            y: 0.,
        },
        PathCommand::QuadTo {
            cx: 3.5,
            cy: 1.,
            x: 5.8,
            y: 3.2,
        },
        PathCommand::QuadTo {
            cx: 6.5,
            cy: 4.,
            x: 5.8,
            y: 4.8,
        },
        PathCommand::QuadTo {
            cx: 3.5,
            cy: 7.,
            x: 1.5,
            y: 8.,
        },
        PathCommand::QuadTo {
            cx: 0.,
            cy: 8.,
            x: 0.,
            y: 6.5,
        },
    ];

    let (sin, cos) = head.angle.sin_cos();
    // Leva o ponto de encaixe do marcador pra origem, gira, e solta no fim do caminho.
    let place = |x: f32, y: f32| {
        let (x, y) = (x, y - 4.);
        (head.x + x * cos - y * sin, head.y + x * sin + y * cos)
    };

    OUTLINE
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo { x, y } => {
                let (x, y) = place(x, y);
                PathCommand::MoveTo { x, y }
            }
            PathCommand::LineTo { x, y } => {
                let (x, y) = place(x, y);
                PathCommand::LineTo { x, y }
            }
            PathCommand::QuadTo { cx, cy, x, y } => {
                let (cx, cy) = place(cx, cy);
                let (x, y) = place(x, y);
                PathCommand::QuadTo { cx, cy, x, y }
            }
        })
        .collect()
}

// =============================================================================
// Testes
// =============================================================================

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod testes {
    use super::*;

    /// Quanto duas coordenadas podem discordar e ainda serem "a mesma".
    const EPSILON: f32 = 0.001;

    fn no(id: usize) -> TreeNode {
        TreeNode::Node { id: NodeId(id) }
    }

    fn lista(children: Vec<TreeNode>) -> TreeNode {
        TreeNode::List { children }
    }

    fn paralelo(children: Vec<TreeNode>) -> TreeNode {
        TreeNode::Parallel {
            children,
            align: ParallelAlign::Start,
        }
    }

    fn medida(width: f32, height: f32) -> NodeMeasure {
        NodeMeasure {
            width,
            height,
            ..Default::default()
        }
    }

    fn estado(nodes: Vec<NodeMeasure>, tree: TreeNode) -> FlowState {
        FlowState {
            nodes,
            tree,
            ..Default::default()
        }
    }

    /// Quadrado 40×40, o tamanho que a spec usa em todos os exemplos.
    fn quadrados(quantidade: usize) -> Vec<NodeMeasure> {
        vec![medida(40., 40.); quantidade]
    }

    fn posicao(positions: &[Option<Position>], id: usize) -> Position {
        positions[id].expect("o nó devia ter posição")
    }

    #[track_caller]
    fn perto(obtido: f32, esperado: f32) {
        assert!(
            (obtido - esperado).abs() < EPSILON,
            "esperava {esperado}, veio {obtido}"
        );
    }

    // -------------------------------------------------------------------------
    // Arestas
    // -------------------------------------------------------------------------

    #[test]
    fn diagrama_vazio_nao_gera_aresta_nem_posicao_nem_retangulo() {
        let state = estado(Vec::new(), lista(Vec::new()));
        let positions = compute_positions(&state, Gaps::default());

        assert_eq!(compute_edges(&state), Vec::<Edge>::new());
        assert_eq!(positions, Vec::<Option<Position>>::new());
        assert_eq!(
            compute_diagram_rect(&positions, &state),
            DiagramRect {
                width: 0.,
                height: 0.
            }
        );
    }

    #[test]
    fn um_no_so_nao_tem_aresta_e_fica_na_origem() {
        let state = estado(quadrados(1), lista(vec![no(0)]));
        let positions = compute_positions(&state, Gaps::default());

        assert_eq!(compute_edges(&state), Vec::<Edge>::new());
        assert_eq!(posicao(&positions, 0), Position { x: 0., y: 0. });
        assert_eq!(
            compute_diagram_rect(&positions, &state),
            DiagramRect {
                width: 40.,
                height: 40.
            }
        );
    }

    #[test]
    fn lista_encadeia_linearmente() {
        let state = estado(quadrados(3), lista(vec![no(0), no(1), no(2)]));

        assert_eq!(
            compute_edges(&state),
            vec![(NodeId(0), NodeId(1)), (NodeId(1), NodeId(2))]
        );
    }

    #[test]
    fn no_vizinho_de_paralelo_faz_leque_na_entrada_e_na_saida() {
        // A → paralelo(B1, B2) → C. É a regra 2 da spec, e o desenho que o componente existe
        // pra mostrar.
        let state = estado(
            quadrados(4),
            lista(vec![no(0), paralelo(vec![no(1), no(2)]), no(3)]),
        );

        let edges = compute_edges(&state);

        assert_eq!(edges.len(), 4);
        assert!(edges.contains(&(NodeId(0), NodeId(1))));
        assert!(edges.contains(&(NodeId(0), NodeId(2))));
        assert!(edges.contains(&(NodeId(1), NodeId(3))));
        assert!(edges.contains(&(NodeId(2), NodeId(3))));
    }

    #[test]
    fn paralelo_com_um_filho_so_vira_encadeamento_simples() {
        let state = estado(quadrados(3), lista(vec![no(0), paralelo(vec![no(1)]), no(2)]));

        assert_eq!(
            compute_edges(&state),
            vec![(NodeId(0), NodeId(1)), (NodeId(1), NodeId(2))]
        );
    }

    #[test]
    fn paralelos_adjacentes_nao_se_interligam() {
        // Regra 3: A → B1, A → B2, C1 → D, C2 → D, e NADA entre os B e os C.
        let state = estado(
            quadrados(6),
            lista(vec![
                no(0),
                paralelo(vec![no(1), no(2)]),
                paralelo(vec![no(3), no(4)]),
                no(5),
            ]),
        );

        let edges = compute_edges(&state);

        assert_eq!(edges.len(), 4, "arestas encontradas: {edges:?}");
        assert!(edges.contains(&(NodeId(0), NodeId(1))));
        assert!(edges.contains(&(NodeId(0), NodeId(2))));
        assert!(edges.contains(&(NodeId(3), NodeId(5))));
        assert!(edges.contains(&(NodeId(4), NodeId(5))));
        for aresta in [
            (NodeId(1), NodeId(3)),
            (NodeId(1), NodeId(4)),
            (NodeId(2), NodeId(3)),
            (NodeId(2), NodeId(4)),
        ] {
            assert!(!edges.contains(&aresta), "{aresta:?} não devia existir");
        }
    }

    #[test]
    fn lista_aninhada_liga_so_no_primeiro_e_no_ultimo() {
        // Regra 4, com o exemplo exato da spec:
        // A → paralelo(lista(B1, B2), C1) → D  ⇒  A→B1, A→C1, B1→B2, B2→D, C1→D.
        let state = estado(
            quadrados(5),
            lista(vec![
                no(0),
                paralelo(vec![lista(vec![no(1), no(2)]), no(3)]),
                no(4),
            ]),
        );

        let edges = compute_edges(&state);

        assert_eq!(edges.len(), 5, "arestas encontradas: {edges:?}");
        for aresta in [
            (NodeId(0), NodeId(1)),
            (NodeId(0), NodeId(3)),
            (NodeId(1), NodeId(2)),
            (NodeId(2), NodeId(4)),
            (NodeId(3), NodeId(4)),
        ] {
            assert!(edges.contains(&aresta), "faltou {aresta:?}");
        }
        // O nó de fora NÃO alcança o meio da lista aninhada.
        assert!(!edges.contains(&(NodeId(0), NodeId(2))));
        assert!(!edges.contains(&(NodeId(1), NodeId(4))));
    }

    #[test]
    fn paralelo_dentro_de_paralelo_achata_o_leque() {
        // Um paralelo aninhado não cria nível de indireção nas arestas: A liga nos três ramos.
        let state = estado(
            quadrados(5),
            lista(vec![
                no(0),
                paralelo(vec![paralelo(vec![no(1), no(2)]), no(3)]),
                no(4),
            ]),
        );

        let edges = compute_edges(&state);

        assert_eq!(edges.len(), 6, "arestas encontradas: {edges:?}");
        for aresta in [
            (NodeId(0), NodeId(1)),
            (NodeId(0), NodeId(2)),
            (NodeId(0), NodeId(3)),
            (NodeId(1), NodeId(4)),
            (NodeId(2), NodeId(4)),
            (NodeId(3), NodeId(4)),
        ] {
            assert!(edges.contains(&aresta), "faltou {aresta:?}");
        }
    }

    #[test]
    fn lista_aninhada_vazia_nao_gera_aresta_pendurada() {
        let state = estado(quadrados(2), lista(vec![no(0), lista(Vec::new()), no(1)]));

        // Sem pontos de contato no meio, não há como costurar — e não há pânico.
        assert_eq!(compute_edges(&state), Vec::<Edge>::new());
    }

    // -------------------------------------------------------------------------
    // Posições
    // -------------------------------------------------------------------------

    #[test]
    fn lista_anda_no_eixo_horizontal_somando_column_gap() {
        // Exemplo da spec: A=40, B=60, C=40 ⇒ 0, 104, 228.
        let state = estado(
            vec![medida(40., 40.), medida(60., 40.), medida(40., 40.)],
            lista(vec![no(0), no(1), no(2)]),
        );

        let positions = compute_positions(&state, Gaps::default());

        assert_eq!(posicao(&positions, 0), Position { x: 0., y: 0. });
        assert_eq!(posicao(&positions, 1), Position { x: 104., y: 0. });
        assert_eq!(posicao(&positions, 2), Position { x: 228., y: 0. });
    }

    #[test]
    fn paralelo_empilha_no_eixo_vertical_somando_row_gap() {
        // Exemplo da spec, com o row_gap do código: B1 e B2 na mesma coluna, B2 em y = 40 + 16.
        let state = estado(
            vec![
                medida(40., 40.),
                medida(60., 40.),
                medida(60., 40.),
                medida(40., 40.),
            ],
            lista(vec![no(0), paralelo(vec![no(1), no(2)]), no(3)]),
        );

        let positions = compute_positions(&state, Gaps::default());

        assert_eq!(posicao(&positions, 1), Position { x: 104., y: 0. });
        assert_eq!(posicao(&positions, 2), Position { x: 104., y: 56. });
        // x igual nos dois ramos é o que faz o leque sair simétrico.
        perto(posicao(&positions, 1).x, posicao(&positions, 2).x);
    }

    #[test]
    fn paralelo_ocupa_a_largura_do_filho_mais_largo() {
        // Exemplo da spec: B1=60, B2=100 ⇒ o C seguinte desloca por 100, não por 60.
        let state = estado(
            vec![
                medida(40., 40.),
                medida(60., 40.),
                medida(100., 40.),
                medida(40., 40.),
            ],
            lista(vec![no(0), paralelo(vec![no(1), no(2)]), no(3)]),
        );

        let positions = compute_positions(&state, Gaps::default());

        assert_eq!(posicao(&positions, 3), Position { x: 268., y: 0. });
    }

    #[test]
    fn lista_aninhada_em_paralelo_posiciona_como_na_spec() {
        // O exemplo mais completo da spec: A=40, B1=60, B2=100, C1=40, D=40 com
        // lista(A, paralelo(lista(B1, B2), C1), D) ⇒ A=0, B1=104, B2=228, C1=(104,88), D=392.
        let state = estado(
            vec![
                medida(40., 40.),
                medida(60., 40.),
                medida(100., 40.),
                medida(40., 40.),
                medida(40., 40.),
            ],
            lista(vec![
                no(0),
                paralelo(vec![lista(vec![no(1), no(2)]), no(3)]),
                no(4),
            ]),
        );

        let positions = compute_positions(&state, Gaps::default());

        assert_eq!(posicao(&positions, 0), Position { x: 0., y: 0. });
        assert_eq!(posicao(&positions, 1), Position { x: 104., y: 0. });
        assert_eq!(posicao(&positions, 2), Position { x: 228., y: 0. });
        assert_eq!(posicao(&positions, 3), Position { x: 104., y: 56. });
        // 40 + 64 + 60 + 64 + 100 + 64: a lista aninhada (224 de largura) manda no
        // deslocamento, não o C1 (40).
        assert_eq!(posicao(&positions, 4), Position { x: 392., y: 0. });
    }

    #[test]
    fn no_de_largura_zero_nao_consome_espaco_mas_ainda_gasta_o_gap() {
        let state = estado(
            vec![medida(0., 40.), medida(40., 40.)],
            lista(vec![no(0), no(1)]),
        );

        let positions = compute_positions(&state, Gaps::default());

        assert_eq!(posicao(&positions, 0), Position { x: 0., y: 0. });
        // Largura zero + column_gap: o vizinho anda 64, não 104.
        assert_eq!(posicao(&positions, 1), Position { x: 64., y: 0. });
    }

    #[test]
    fn medida_ausente_conta_como_tamanho_zero() {
        // Um id que não existe em `nodes` (frame antes da medição) não estoura nem desloca. Ele
        // vai no MEIO de propósito: no fim, um tamanho errado não moveria ninguém e o teste não
        // morderia.
        let state = estado(quadrados(2), lista(vec![no(0), no(9), no(1)]));

        let positions = compute_positions(&state, Gaps::default());

        assert_eq!(positions.len(), 2, "o id de fora não ganha vaga");
        assert_eq!(posicao(&positions, 0), Position { x: 0., y: 0. });
        // 40 + 64 + **0** + 64: o nó sem medida gasta os dois gaps e nenhuma largura.
        assert_eq!(posicao(&positions, 1), Position { x: 168., y: 0. });
    }

    #[test]
    fn gaps_sao_configuraveis() {
        // Provando que os 64/48 vêm de `Gaps` e não estão embutidos na aritmética.
        let state = estado(
            quadrados(3),
            lista(vec![no(0), paralelo(vec![no(1), no(2)])]),
        );

        let positions = compute_positions(
            &state,
            Gaps {
                column_gap: 10.,
                row_gap: 4.,
            },
        );

        assert_eq!(posicao(&positions, 1), Position { x: 50., y: 0. });
        assert_eq!(posicao(&positions, 2), Position { x: 50., y: 44. });
    }

    #[test]
    fn lista_horizontal_centralizada_centra_na_faixa_do_mais_alto() {
        let state = FlowState {
            nodes: vec![medida(40., 20.), medida(40., 100.)],
            tree: lista(vec![no(0), no(1)]),
            align: FlowAlign::Center,
            orientation: FlowOrientation::Horizontal,
        };

        let positions = compute_positions(&state, Gaps::default());

        // (100 - 20) / 2 = 40.
        assert_eq!(posicao(&positions, 0), Position { x: 0., y: 40. });
        assert_eq!(posicao(&positions, 1), Position { x: 104., y: 0. });
    }

    #[test]
    fn paralelo_alinhado_pelo_fim_encosta_os_ramos_na_direita() {
        let state = estado(
            vec![medida(40., 40.), medida(100., 40.)],
            TreeNode::Parallel {
                children: vec![no(0), no(1)],
                align: ParallelAlign::End,
            },
        );

        let positions = compute_positions(&state, Gaps::default());

        // O ramo curto desloca 100 - 40 = 60 pra encostar na saída; o largo fica na origem, um
        // row_gap abaixo (40 de altura + 16).
        assert_eq!(posicao(&positions, 0), Position { x: 60., y: 0. });
        assert_eq!(posicao(&positions, 1), Position { x: 0., y: 56. });
    }

    #[test]
    fn orientacao_vertical_troca_os_eixos() {
        let state = FlowState {
            nodes: quadrados(3),
            tree: lista(vec![no(0), paralelo(vec![no(1), no(2)])]),
            align: FlowAlign::Start,
            orientation: FlowOrientation::Vertical,
        };

        let positions = compute_positions(&state, Gaps::default());

        // Lista desce (column_gap no y), ramos abrem pra direita (row_gap no x).
        assert_eq!(posicao(&positions, 1), Position { x: 0., y: 104. });
        assert_eq!(posicao(&positions, 2), Position { x: 56., y: 104. });
    }

    // -------------------------------------------------------------------------
    // Retângulo do diagrama
    // -------------------------------------------------------------------------

    #[test]
    fn retangulo_do_diagrama_pega_a_borda_que_foi_mais_longe() {
        // Tamanhos desiguais: o nó mais largo NÃO é o que define a altura, e vice-versa.
        let state = estado(
            vec![medida(40., 200.), medida(300., 30.), medida(20., 20.)],
            lista(vec![no(0), paralelo(vec![no(1), no(2)])]),
        );

        let positions = compute_positions(&state, Gaps::default());
        let rect = compute_diagram_rect(&positions, &state);

        // Largura: o nó 1 começa em 40 + 64 = 104 e tem 300 ⇒ 404.
        perto(rect.width, 404.);
        // Altura: o nó 0 sozinho vai a 200; o ramo 2 vai a 30 + 16 + 20 = 66. Ganha o 200.
        perto(rect.height, 200.);
    }

    #[test]
    fn retangulo_ignora_no_sem_posicao() {
        // Nó medido mas fora da árvore não infla o retângulo.
        let state = estado(
            vec![medida(40., 40.), medida(1000., 1000.)],
            lista(vec![no(0)]),
        );

        let positions = compute_positions(&state, Gaps::default());

        assert_eq!(
            compute_diagram_rect(&positions, &state),
            DiagramRect {
                width: 40.,
                height: 40.
            }
        );
    }

    // -------------------------------------------------------------------------
    // Conectores
    // -------------------------------------------------------------------------

    #[test]
    fn conector_horizontal_sai_da_direita_e_entra_na_esquerda_no_meio_da_altura() {
        let state = estado(
            vec![medida(40., 40.), medida(40., 80.)],
            lista(vec![no(0), no(1)]),
        );
        let positions = compute_positions(&state, Gaps::default());
        let edges = compute_edges(&state);

        let connectors =
            compute_connectors(&edges, &positions, &state, FlowOrientation::Horizontal);

        assert_eq!(connectors.len(), 1);
        let c = connectors[0];
        perto(c.x1, 40.); // borda direita da origem
        perto(c.y1, 20.); // meio da altura da origem
        perto(c.x2, 104.); // borda esquerda do destino
        perto(c.y2, 40.); // meio da altura do destino
    }

    #[test]
    fn ancoras_deslocam_so_a_ponta_que_lhes_cabe() {
        // Âncora de saída no nó 0 e de entrada no nó 1: cada uma manda na sua ponta.
        let mut nodes = vec![medida(40., 100.), medida(40., 100.)];
        nodes[0].start_anchor_offset = Some(10.);
        nodes[1].end_anchor_offset = Some(90.);
        let state = estado(nodes, lista(vec![no(0), no(1)]));
        let positions = compute_positions(&state, Gaps::default());
        let edges = compute_edges(&state);

        let connectors =
            compute_connectors(&edges, &positions, &state, FlowOrientation::Horizontal);

        perto(connectors[0].y1, 10.);
        perto(connectors[0].y2, 90.);
    }

    #[test]
    fn ancora_de_entrada_nao_mexe_na_saida_do_mesmo_no() {
        // `type = "end"` no nó do meio: a seta que ENTRA usa a âncora, a que SAI usa o centro.
        let mut nodes = vec![medida(40., 100.), medida(40., 100.), medida(40., 100.)];
        nodes[1].end_anchor_offset = Some(5.);
        let state = estado(nodes, lista(vec![no(0), no(1), no(2)]));
        let positions = compute_positions(&state, Gaps::default());
        let edges = compute_edges(&state);

        let connectors =
            compute_connectors(&edges, &positions, &state, FlowOrientation::Horizontal);

        let entrando = connectors.iter().find(|c| c.to == NodeId(1)).unwrap();
        let saindo = connectors.iter().find(|c| c.from == NodeId(1)).unwrap();
        perto(entrando.y2, 5.);
        perto(saindo.y1, 50.);
    }

    #[test]
    fn conector_vertical_sai_de_baixo_e_entra_por_cima_ignorando_ancora() {
        let mut nodes = vec![medida(40., 40.), medida(60., 40.)];
        nodes[0].start_anchor_offset = Some(0.);
        nodes[1].end_anchor_offset = Some(0.);
        let state = FlowState {
            nodes,
            tree: lista(vec![no(0), no(1)]),
            align: FlowAlign::Start,
            orientation: FlowOrientation::Vertical,
        };
        let positions = compute_positions(&state, Gaps::default());
        let edges = compute_edges(&state);

        let connectors = compute_connectors(&edges, &positions, &state, FlowOrientation::Vertical);

        let c = connectors[0];
        perto(c.x1, 20.); // meio da largura da origem
        perto(c.y1, 40.); // borda de baixo da origem — âncora não entra na conta
        perto(c.x2, 30.); // meio da largura do destino
        perto(c.y2, 104.); // borda de cima do destino
    }

    #[test]
    fn conector_desativado_vem_antes_pra_ser_pintado_embaixo() {
        let mut nodes = quadrados(3);
        nodes[2].disabled = true;
        let state = estado(nodes, lista(vec![no(0), paralelo(vec![no(1), no(2)])]));
        let positions = compute_positions(&state, Gaps::default());
        let edges = compute_edges(&state);
        // Na ordem das arestas, o desativado (0 → 2) vem depois do ativo (0 → 1).
        assert_eq!(edges[1], (NodeId(0), NodeId(2)));

        let connectors =
            compute_connectors(&edges, &positions, &state, FlowOrientation::Horizontal);

        assert!(connectors[0].disabled, "o desativado devia vir primeiro");
        assert_eq!(connectors[0].to, NodeId(2));
        assert!(!connectors[1].disabled);
    }

    #[test]
    fn conector_para_no_fora_da_arvore_e_descartado() {
        // Aresta montada à mão apontando pra um nó MEDIDO mas que não está na árvore: ele tem
        // medida e não tem posição. É o caso que só a guarda de posição pega.
        let state = estado(quadrados(2), lista(vec![no(0)]));
        let positions = compute_positions(&state, Gaps::default());
        assert_eq!(positions[1], None, "o nó 1 não está na árvore");

        let connectors = compute_connectors(
            &[(NodeId(0), NodeId(1))],
            &positions,
            &state,
            FlowOrientation::Horizontal,
        );

        assert!(connectors.is_empty());
    }

    #[test]
    fn conector_sem_medida_ainda_e_descartado() {
        // Frame antes da medição: a aresta aponta pra um id sem medida.
        let state = estado(quadrados(1), lista(vec![no(0), no(3)]));
        let positions = compute_positions(&state, Gaps::default());
        let edges = compute_edges(&state);
        assert_eq!(edges.len(), 1);

        let connectors =
            compute_connectors(&edges, &positions, &state, FlowOrientation::Horizontal);

        assert!(connectors.is_empty());
    }

    // -------------------------------------------------------------------------
    // O invariante: conector nenhum invade nó nenhum
    // -------------------------------------------------------------------------

    /// Meio pixel: encostar na borda de um nó é permitido, atravessar não.
    const TOLERANCIA: f32 = 0.5;

    /// Em quantos pontos cada segmento é amostrado. 64 pega qualquer travessia que valha algo
    /// numa caixa de dezenas de pixels.
    const AMOSTRAS: usize = 64;

    /// O ponto está DENTRO da caixa, com folga maior que a tolerância nos quatro lados?
    ///
    /// Encostar (ou correr rente) não conta: só conta furar.
    fn dentro(ponto: (f32, f32), position: Position, node: &NodeMeasure) -> bool {
        let (x, y) = ponto;
        x > position.x + TOLERANCIA
            && x < position.x + node.width - TOLERANCIA
            && y > position.y + TOLERANCIA
            && y < position.y + node.height - TOLERANCIA
    }

    /// Os pontos de um caminho, segmento por segmento, com as quadráticas amostradas.
    fn amostrar(commands: &[PathCommand]) -> Vec<(f32, f32)> {
        let mut pontos = Vec::new();
        let mut atual = (0., 0.);

        for command in commands {
            match *command {
                PathCommand::MoveTo { x, y } => {
                    atual = (x, y);
                    pontos.push(atual);
                }
                PathCommand::LineTo { x, y } => {
                    for i in 0..=AMOSTRAS {
                        let t = i as f32 / AMOSTRAS as f32;
                        pontos.push((atual.0 + (x - atual.0) * t, atual.1 + (y - atual.1) * t));
                    }
                    atual = (x, y);
                }
                PathCommand::QuadTo { cx, cy, x, y } => {
                    for i in 0..=AMOSTRAS {
                        let t = i as f32 / AMOSTRAS as f32;
                        let u = 1. - t;
                        pontos.push((
                            u * u * atual.0 + 2. * u * t * cx + t * t * x,
                            u * u * atual.1 + 2. * u * t * cy + t * t * y,
                        ));
                    }
                    atual = (x, y);
                }
            }
        }

        pontos
    }

    /// **O invariante do desenho**: nenhum segmento de nenhum caminho entra na caixa de nenhum nó.
    ///
    /// É a propriedade que decide se o diagrama parece certo. As pontas do conector encostam nas
    /// bordas dos dois nós que ele liga — isso é o desenho funcionando — mas atravessar uma caixa
    /// faz a linha aparecer por cima do nó, porque a ordem de pintura é nós primeiro e arestas
    /// depois.
    ///
    /// Devolve `Err` com o diagnóstico do primeiro furo: qual aresta, qual nó, e onde.
    fn nenhum_conector_invade_no(state: &FlowState, gaps: Gaps) -> Result<(), String> {
        let positions = compute_positions(state, gaps);
        let edges = compute_edges(state);
        let connectors = compute_connectors(&edges, &positions, state, state.orientation);
        let options = PathOptions {
            orientation: state.orientation,
            ..Default::default()
        };

        for connector in &connectors {
            let commands = rounded_path(connector, options);
            for ponto in amostrar(&commands) {
                for (index, position) in positions.iter().enumerate() {
                    let (Some(position), Some(node)) = (position, state.nodes.get(index)) else {
                        continue;
                    };
                    if dentro(ponto, *position, node) {
                        return Err(format!(
                            "a aresta {:?} → {:?} entra no nó {index} \
                             (caixa x {:.1}..{:.1}, y {:.1}..{:.1}) no ponto ({:.1}, {:.1}); \
                             o caminho é {commands:?}",
                            connector.from,
                            connector.to,
                            position.x,
                            position.x + node.width,
                            position.y,
                            position.y + node.height,
                            ponto.0,
                            ponto.1,
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// O diagrama da história que o usuário apontou: um nó, um leque de N ramos, e um nó de saída.
    ///
    /// Alturas de 48, como as caixas de verdade da história; larguras desiguais de propósito.
    fn leque(larguras: &[f32], orientation: FlowOrientation) -> FlowState {
        let mut nodes = vec![medida(70., 48.)];
        nodes.extend(larguras.iter().map(|&w| medida(w, 48.)));
        nodes.push(medida(70., 48.));

        let ramos = (0..larguras.len()).map(|i| no(i + 1)).collect();
        let saida = larguras.len() + 1;

        FlowState {
            nodes,
            tree: lista(vec![no(0), paralelo(ramos), no(saida)]),
            align: FlowAlign::Start,
            orientation,
        }
    }

    #[track_caller]
    fn invariante(state: &FlowState, gaps: Gaps) {
        if let Err(erro) = nenhum_conector_invade_no(state, gaps) {
            panic!("invariante violado: {erro}");
        }
    }

    #[test]
    fn conector_nao_invade_no_com_os_vaos_padrao() {
        // A linha de base: com 64/16 o desenho é o do kumo, e ele não invade.
        invariante(
            &leque(&[90., 150.], FlowOrientation::Horizontal),
            Gaps::default(),
        );
    }

    #[test]
    fn conector_nao_invade_no_com_o_vao_da_historia() {
        // `gaps(24.0, 8.0)` — exatamente a terceira história de "Vertical, centralizado, e vãos
        // configuráveis", que é onde o usuário viu a linha atravessando a caixa.
        invariante(
            &leque(&[90., 150.], FlowOrientation::Horizontal),
            Gaps {
                column_gap: 24.,
                row_gap: 8.,
            },
        );
    }

    #[test]
    fn conector_nao_invade_no_com_vao_muito_apertado() {
        invariante(
            &leque(&[90., 150.], FlowOrientation::Horizontal),
            Gaps {
                column_gap: 8.,
                row_gap: 4.,
            },
        );
    }

    #[test]
    fn conector_nao_invade_no_com_vao_zero() {
        // O caso degenerado: nós colados. Não há espaço pra seta nenhuma, e o que se exige é só
        // que a linha não fure ninguém.
        invariante(
            &leque(&[90., 150.], FlowOrientation::Horizontal),
            Gaps {
                column_gap: 0.,
                row_gap: 0.,
            },
        );
    }

    #[test]
    fn conector_nao_invade_no_com_tres_ramos_apertados() {
        // Três ramos: o do meio é o que o leque tem mais chance de atravessar, porque a aresta que
        // vai pro de baixo passa na altura dele.
        invariante(
            &leque(&[90., 150., 110.], FlowOrientation::Horizontal),
            Gaps {
                column_gap: 24.,
                row_gap: 8.,
            },
        );
    }

    #[test]
    fn conector_nao_invade_no_no_eixo_vertical() {
        for gaps in [
            Gaps::default(),
            Gaps {
                column_gap: 24.,
                row_gap: 8.,
            },
            Gaps {
                column_gap: 8.,
                row_gap: 4.,
            },
        ] {
            invariante(&leque(&[90., 150.], FlowOrientation::Vertical), gaps);
        }
    }

    #[test]
    fn conector_nao_invade_no_com_align_center() {
        // A segunda história de "Vertical, centralizado, e vãos configuráveis": centralizar move os
        // nós no eixo cruzado, o que muda a faixa por onde cada perna passa.
        for gaps in [
            Gaps::default(),
            Gaps {
                column_gap: 24.,
                row_gap: 8.,
            },
        ] {
            let mut state = leque(&[90., 150.], FlowOrientation::Horizontal);
            state.align = FlowAlign::Center;
            invariante(&state, gaps);
        }
    }

    #[test]
    fn conector_nao_invade_no_em_lista_longa_apertada() {
        // Sem paralelo nenhum: quatro nós em fila, com vão curto. A aresta aqui é quase reta, e o
        // que se testa é a aproximação da ponta não voltar pra dentro do nó de saída.
        let state = FlowState {
            nodes: vec![medida(70., 48.); 4],
            tree: lista(vec![no(0), no(1), no(2), no(3)]),
            align: FlowAlign::Start,
            orientation: FlowOrientation::Horizontal,
        };

        invariante(
            &state,
            Gaps {
                column_gap: 6.,
                row_gap: 6.,
            },
        );
    }

    // -------------------------------------------------------------------------
    // A simetria do leque vertical
    // -------------------------------------------------------------------------

    /// O leque vertical da história: um nó em cima, dois ramos abrindo pra direita.
    ///
    /// Larguras plausíveis de nó de texto — 46 pra "um", 56 pra "dois", 60 pra "três" — porque a
    /// assimetria só aparece com larguras de verdade.
    fn leque_vertical(align: FlowAlign) -> FlowState {
        FlowState {
            nodes: vec![medida(46., 48.), medida(56., 48.), medida(60., 48.)],
            tree: lista(vec![no(0), paralelo(vec![no(1), no(2)])]),
            align,
            orientation: FlowOrientation::Vertical,
        }
    }

    /// O centro de um nó no eixo horizontal — de onde a seta sai, no fluxo vertical.
    fn centro_x(positions: &[Option<Position>], state: &FlowState, id: usize) -> f32 {
        posicao(positions, id).x + state.nodes[id].width / 2.
    }

    #[test]
    fn leque_vertical_alinhado_a_esquerda_poe_o_pai_sobre_o_primeiro_ramo() {
        // ⚠️ **Isto não é defeito, é o padrão do kumo** — e é feio neste caso. O
        // `FlowAlign::Start` encosta toda a lista na borda esquerda, e o grupo paralelo também
        // começa nela, então o pai fica exatamente em cima do primeiro ramo: uma aresta sai quase
        // reta e a outra dá uma volta grande. Quem quer o leque simétrico usa [`FlowAlign::Center`]
        // — que é justo o motivo de o kumo ter a opção. Ver o teste seguinte.
        let state = leque_vertical(FlowAlign::Start);
        let positions = compute_positions(&state, Gaps::default());

        // Pai e primeiro ramo partem os dois de x = 0.
        assert_eq!(posicao(&positions, 0).x, 0.);
        assert_eq!(posicao(&positions, 1).x, 0.);

        // O grupo inteiro mede 56 + 16 + 60 = 132, então o centro dele é 66. O pai está a 23.
        let centro_do_grupo = (56. + 16. + 60.) / 2.;
        perto(centro_do_grupo, 66.);
        let desvio = (centro_x(&positions, &state, 0) - centro_do_grupo).abs();
        assert!(
            desvio > 40.,
            "o pai devia estar BEM fora do centro do leque (é o padrão); desviou só {desvio}"
        );
        // E está praticamente sobre o primeiro ramo.
        assert!(
            (centro_x(&positions, &state, 0) - centro_x(&positions, &state, 1)).abs() < 10.,
            "o pai fica sobre o primeiro ramo"
        );
    }

    #[test]
    fn align_center_centra_o_pai_sobre_o_leque_inteiro() {
        // Com `align_center()` o pai vai pro meio da largura do grupo, e as duas arestas saem
        // espelhadas. É esta a configuração que mostra o leque vertical direito.
        let state = leque_vertical(FlowAlign::Center);
        let positions = compute_positions(&state, Gaps::default());

        // (132 − 46) / 2 = 43.
        assert_eq!(posicao(&positions, 0).x, 43.);

        let centro_do_grupo = (56. + 16. + 60.) / 2.;
        perto(centro_x(&positions, &state, 0), centro_do_grupo);

        // E as duas arestas saem pra lados opostos, com sobras parecidas — o que "simétrico"
        // significa aqui, já que os dois ramos têm larguras diferentes (56 e 60).
        let edges = compute_edges(&state);
        let connectors =
            compute_connectors(&edges, &positions, &state, FlowOrientation::Vertical);
        let dx: Vec<f32> = connectors.iter().map(|c| c.x2 - c.x1).collect();
        assert!(
            dx[0] < 0. && dx[1] > 0.,
            "uma aresta vai pra esquerda e a outra pra direita; deu {dx:?}"
        );
        assert!(
            (dx[0].abs() - dx[1].abs()).abs() < 5.,
            "as duas sobras deviam ser quase iguais; deram {dx:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Caminho
    // -------------------------------------------------------------------------

    fn conector(x1: f32, y1: f32, x2: f32, y2: f32) -> Connector {
        Connector {
            from: NodeId(0),
            to: NodeId(1),
            x1,
            y1,
            x2,
            y2,
            disabled: false,
            single: true,
            is_bottom: false,
            // `None` = nenhum ramo largo atravancando o corredor, que é o caso destes testes de
            // caminho: eles olham UMA aresta isolada.
            free_main_span: None,
        }
    }

    #[test]
    fn aresta_quase_reta_sai_reta_e_para_antes_da_seta() {
        // Desnível de 2 é exatamente o limite: ainda é reta.
        let commands = rounded_path(&conector(0., 0., 100., 2.), PathOptions::default());

        assert_eq!(
            commands,
            vec![
                PathCommand::MoveTo { x: 0., y: 0. },
                PathCommand::LineTo { x: 92., y: 2. },
            ]
        );
    }

    #[test]
    fn desnivel_acima_do_limite_curva() {
        // Um pixel a mais que o limite e o caminho ganha os dois cantos.
        let reta = rounded_path(&conector(0., 0., 100., 2.), PathOptions::default());
        let curva = rounded_path(&conector(0., 0., 100., 3.), PathOptions::default());

        assert_eq!(reta.len(), 2);
        assert_eq!(curva.len(), 6);
    }

    #[test]
    fn caminho_curvo_horizontal_tem_a_perna_junto_da_chegada() {
        // single = true ⇒ a perna vertical fica a mid_offset da ponta de chegada.
        let commands = rounded_path(&conector(0., 0., 100., 40.), PathOptions::default());

        assert_eq!(
            commands,
            vec![
                PathCommand::MoveTo { x: 0., y: 0. },
                // 100 - 32 = 68 é a perna; para 8 antes dela.
                PathCommand::LineTo { x: 60., y: 0. },
                PathCommand::QuadTo {
                    cx: 68.,
                    cy: 0.,
                    x: 68.,
                    y: 8.
                },
                PathCommand::LineTo { x: 68., y: 32. },
                PathCommand::QuadTo {
                    cx: 68.,
                    cy: 40.,
                    x: 76.,
                    y: 40.
                },
                PathCommand::LineTo { x: 92., y: 40. },
            ]
        );
    }

    #[test]
    fn raio_do_canto_cai_pra_metade_do_vao_quando_o_espaco_e_curto() {
        // Desnível de 6 ⇒ raio 3, não os 8 padrão.
        let commands = rounded_path(&conector(0., 0., 100., 6.), PathOptions::default());

        assert_eq!(commands[1], PathCommand::LineTo { x: 65., y: 0. });
        assert_eq!(
            commands[2],
            PathCommand::QuadTo {
                cx: 68.,
                cy: 0.,
                x: 68.,
                y: 3.
            }
        );
    }

    #[test]
    fn aresta_que_sobe_espelha_o_sinal_vertical() {
        let commands = rounded_path(&conector(0., 40., 100., 0.), PathOptions::default());

        // Subindo, o primeiro canto entra em y = 40 - 8 e o segundo sai em y = 0 + 8.
        assert_eq!(
            commands[2],
            PathCommand::QuadTo {
                cx: 68.,
                cy: 40.,
                x: 68.,
                y: 32.
            }
        );
        assert_eq!(commands[3], PathCommand::LineTo { x: 68., y: 8. });
    }

    #[test]
    fn caminho_vertical_curva_no_outro_eixo() {
        let commands = rounded_path(
            &conector(0., 0., 40., 100.),
            PathOptions {
                orientation: FlowOrientation::Vertical,
                ..Default::default()
            },
        );

        assert_eq!(
            commands,
            vec![
                PathCommand::MoveTo { x: 0., y: 0. },
                PathCommand::LineTo { x: 0., y: 60. },
                PathCommand::QuadTo {
                    cx: 0.,
                    cy: 68.,
                    x: 8.,
                    y: 68.
                },
                PathCommand::LineTo { x: 32., y: 68. },
                PathCommand::QuadTo {
                    cx: 40.,
                    cy: 68.,
                    x: 40.,
                    y: 76.
                },
                PathCommand::LineTo { x: 40., y: 92. },
            ]
        );
    }

    #[test]
    fn caminho_vertical_quase_alinhado_sai_reto() {
        let commands = rounded_path(
            &conector(10., 0., 11., 100.),
            PathOptions {
                orientation: FlowOrientation::Vertical,
                ..Default::default()
            },
        );

        assert_eq!(
            commands,
            vec![
                PathCommand::MoveTo { x: 10., y: 0. },
                PathCommand::LineTo { x: 11., y: 92. },
            ]
        );
    }

    #[test]
    fn junta_na_saida_muda_onde_a_perna_fica() {
        // single = false + is_bottom = false ⇒ a perna vai pro lado da SAÍDA (x1 + mid).
        let mut c = conector(0., 0., 100., 40.);
        c.single = false;
        let commands = rounded_path(&c, PathOptions::default());

        assert_eq!(commands[1], PathCommand::LineTo { x: 32., y: 0. });
    }

    #[test]
    fn is_bottom_inverte_qual_ponta_curva() {
        let mut c = conector(0., 0., 100., 40.);
        c.single = false;
        c.is_bottom = true;
        let commands = rounded_path(&c, PathOptions::default());

        // Com is_bottom, a curva acontece logo na saída e o trecho reto fecha na chegada.
        assert_eq!(commands[1], PathCommand::LineTo { x: 60., y: 0. });
        assert_eq!(commands[3], PathCommand::LineTo { x: 68., y: 40. });
    }

    // -------------------------------------------------------------------------
    // A aparagem do cotovelo
    // -------------------------------------------------------------------------

    #[test]
    fn apara_nao_mexe_em_nada_no_vao_padrao() {
        // **A propriedade que torna a aparagem segura.** Com o `column_gap` de 64, o `mid_offset`
        // de 32 do kumo JÁ É metade do vão — a constante dele é um meio-vão escrito à mão. Então
        // pra qualquer vão de 64 ou mais os três deslocamentos saem intactos e o caminho é
        // idêntico ao do original.
        for main in [64., 100., 400.] {
            let elbow = fit_elbow(main, 56., 8., 32., 8.);
            assert_eq!(
                elbow,
                Elbow {
                    corner_radius: 8.,
                    mid_offset: 32.,
                    arrowhead_offset: 8.,
                },
                "com vão de {main} nada devia ser aparado"
            );
        }
    }

    #[test]
    fn apara_leva_a_perna_pro_meio_do_vao_quando_aperta() {
        // Vão de 24 — o da história. A perna do kumo iria a 32 da chegada, ou seja 8 ATRÁS da
        // borda de saída; aparada, ela para no meio do que existe.
        let elbow = fit_elbow(24., 56., 8., 32., 8.);

        assert!(
            elbow.mid_offset < 32.,
            "a perna tinha que recuar; ficou em {}",
            elbow.mid_offset
        );
        // As duas condições que fazem o caminho andar sempre PRA FRENTE no eixo principal, que é o
        // que impede tanto o recuo pra dentro do nó de saída quanto a seta apontando pra trás:
        //
        // - o primeiro canto não pode furar a borda de saída ⇒ `perna + raio <= vão`;
        // - a aproximação final não pode andar pra trás ⇒ `perna >= raio + folga`.
        assert!(
            elbow.mid_offset + elbow.corner_radius <= 24. + EPSILON,
            "o primeiro canto furou a borda de saída"
        );
        assert!(
            elbow.mid_offset >= elbow.corner_radius + elbow.arrowhead_offset - EPSILON,
            "a aproximação final andaria pra trás"
        );
        // Com vão de 24 e os padrões 8/8, as duas condições se fecham num único valor possível.
        perto(elbow.mid_offset, 16.);
    }

    /// O caminho anda sempre PRA FRENTE no eixo principal, e nunca passa do destino.
    ///
    /// São as duas propriedades observáveis por trás da aparagem e do sinal da viagem: recuar é o
    /// que punha o traço dentro do nó de saída, e passar do destino é o que fazia a seta aterrissar
    /// além da borda de chegada.
    #[track_caller]
    fn caminho_progride(commands: &[PathCommand], a1: f32, a2: f32, principal_x: bool, rotulo: &str) {
        let sign = if a2 > a1 { 1. } else { -1. };
        let mut anterior = f32::NEG_INFINITY;

        for ponto in amostrar(commands) {
            let a = if principal_x { ponto.0 } else { ponto.1 };
            // Progresso medido no sentido da viagem.
            let progresso = (a - a1) * sign;
            assert!(
                progresso >= anterior - EPSILON,
                "{rotulo}: o caminho recuou (progresso {anterior} → {progresso}): {commands:?}"
            );
            anterior = progresso;
            // E nunca passa da chegada.
            let sobra = (a2 - a) * sign;
            assert!(
                sobra >= -EPSILON,
                "{rotulo}: o caminho passou do destino em {}: {commands:?}",
                -sobra
            );
        }
    }

    #[test]
    fn caminho_apertado_anda_sempre_pra_frente() {
        for vao in [0., 4., 8., 16., 24., 40., 48., 64., 200.] {
            let mut c = conector(0., 0., vao, 56.);
            c.free_main_span = Some(vao);
            let commands = rounded_path(&c, PathOptions::default());
            caminho_progride(&commands, 0., vao, true, &format!("vão {vao}"));
        }
    }

    #[test]
    fn caminho_progride_nas_quatro_direcoes() {
        // ⚠️ É este teste que o **sinal da viagem** existe pra satisfazer. O kumo escreve
        // `x2 - arrowhead_offset` no caso reto e os cantos do eixo vertical sem sinal, então uma
        // aresta que vai pra trás (ou que sobe) passa do destino e volta curvando pro lado errado.
        // As arestas do componente sempre viajam pra frente, então nada disso aparece no desenho de
        // hoje — mas sem este teste o sinal é código que nada exerce.
        let horizontal = PathOptions::default();
        let vertical = PathOptions {
            orientation: FlowOrientation::Vertical,
            ..Default::default()
        };

        // Eixo horizontal: pra direita e pra ESQUERDA, com e sem desnível.
        for (x1, x2, cross) in [
            (0., 200., 56.),
            (200., 0., 56.),
            (0., 200., 0.),
            (200., 0., 0.),
        ] {
            let c = conector(x1, 0., x2, cross);
            let commands = rounded_path(&c, horizontal);
            caminho_progride(&commands, x1, x2, true, &format!("horizontal {x1}→{x2}"));
        }

        // Eixo vertical: pra baixo e pra CIMA.
        for (y1, y2, cross) in [
            (0., 200., 56.),
            (200., 0., 56.),
            (0., 200., 0.),
            (200., 0., 0.),
        ] {
            let c = conector(0., y1, cross, y2);
            let commands = rounded_path(&c, vertical);
            caminho_progride(&commands, y1, y2, false, &format!("vertical {y1}→{y2}"));
        }
    }

    #[test]
    fn apara_poe_a_perna_exatamente_no_meio_do_vao_intermediario() {
        // Vão de 48: cabe o cotovelo inteiro (24), mas os 32 do kumo passariam do meio. A regra 2
        // manda a perna pro MEIO — o único lugar em que os dois cantos têm o mesmo espaço dos dois
        // lados. Sem esta faixa de vão testada, o `.min(vão / 2)` não morde: em vão apertado o piso
        // já dá o mesmo número, e em vão largo o 32 não é aparado.
        let elbow = fit_elbow(48., 56., 8., 32., 8.);

        perto(elbow.mid_offset, 24.);
        assert!(
            elbow.mid_offset < 32.,
            "os 32 absolutos do kumo passariam do meio do vão de 48"
        );
        // E o piso não é quem está mandando aqui — se fosse, o teste não provaria a regra 2.
        assert!(
            elbow.mid_offset > elbow.corner_radius + elbow.arrowhead_offset,
            "quem devia mandar neste vão é a metade, não o piso"
        );
    }

    #[test]
    fn apara_encolhe_canto_e_folga_juntos_no_aperto_extremo() {
        // Vão de 8: os 24 que o cotovelo precisa (2 cantos + folga da seta) não cabem, então os
        // dois encolhem NA MESMA PROPORÇÃO — 8/24 = um terço. Encolher só um deles achataria a
        // curva de um lado e deixaria o desenho torto.
        let elbow = fit_elbow(8., 56., 8., 32., 8.);

        perto(elbow.corner_radius, 8. / 3.);
        perto(elbow.arrowhead_offset, 8. / 3.);
        // A proporção entre os dois é a mesma de antes de encolher.
        perto(elbow.corner_radius / elbow.arrowhead_offset, 1.);
        // E tudo cabe no vão.
        assert!(2. * elbow.corner_radius + elbow.arrowhead_offset <= 8. + EPSILON);
    }

    #[test]
    fn apara_zera_tudo_no_vao_zero() {
        // Nós colados: não há espaço pra cotovelo nenhum, e o caminho degenera numa reta na borda
        // compartilhada. É o que faz o invariante valer até no caso degenerado.
        let elbow = fit_elbow(0., 56., 8., 32., 8.);

        assert_eq!(elbow.corner_radius, 0.);
        assert_eq!(elbow.arrowhead_offset, 0.);
        assert_eq!(elbow.mid_offset, 0.);
    }

    #[test]
    fn apara_mantem_a_regra_do_raio_pelo_eixo_cruzado() {
        // A regra 1a é a do kumo e continua valendo: desnível de 6 ⇒ raio 3, mesmo com vão de
        // sobra. A aparagem só ACRESCENTA tetos, não substitui o que já havia.
        let elbow = fit_elbow(400., 6., 8., 32., 8.);

        perto(elbow.corner_radius, 3.);
        perto(elbow.arrowhead_offset, 8.);
    }

    #[test]
    fn vao_livre_encurta_quando_um_ramo_mais_largo_atravanca() {
        // O leque de três ramos desiguais: a aresta do ramo CURTO pro nó de saída tem que passar
        // por cima da faixa do ramo LARGO, e a perna dela precisa ficar depois da borda dele.
        let state = leque(&[90., 150., 110.], FlowOrientation::Horizontal);
        let gaps = Gaps {
            column_gap: 24.,
            row_gap: 8.,
        };
        let positions = compute_positions(&state, gaps);
        let edges = compute_edges(&state);
        let connectors =
            compute_connectors(&edges, &positions, &state, FlowOrientation::Horizontal);

        // O terceiro ramo (nó 3) sai em x = 204; o nó de saída (4) começa em 268: 64 de distância.
        // Mas o segundo ramo (nó 2) avança até 244, então só 24 estão livres.
        let atravancada = connectors
            .iter()
            .find(|c| c.from == NodeId(3) && c.to == NodeId(4))
            .expect("a aresta do terceiro ramo pra saída");
        perto(atravancada.x2 - atravancada.x1, 64.);
        assert_eq!(atravancada.free_main_span, Some(24.));

        // E uma aresta que não tem ninguém no caminho fica com o vão inteiro.
        let livre = connectors
            .iter()
            .find(|c| c.from == NodeId(0) && c.to == NodeId(1))
            .expect("a aresta de entrada pro primeiro ramo");
        assert_eq!(livre.free_main_span, Some(livre.x2 - livre.x1));
    }

    #[test]
    fn vao_livre_ignora_no_fora_da_faixa_cruzada() {
        // Um nó plantado no corredor, mas numa altura que a perna do cotovelo não cruza: ele NÃO
        // pode encurtar o vão. Se encurtasse, a perna seria puxada pra chegada sem motivo e o
        // desenho ficaria torto em diagramas onde nada atravanca.
        //
        // Posições montadas à mão de propósito: a geometria que separa os dois casos é específica
        // demais pra sair de um `compute_positions`.
        let state = estado(
            vec![medida(70., 48.), medida(70., 48.), medida(100., 48.)],
            lista(vec![no(0), no(1)]),
        );
        // Saída em x 0..70; chegada em x 200..270; o terceiro nó em x 100..200 — dentro do corredor.
        let chegada = Some(Position { x: 200., y: 0. });
        let saida = Some(Position { x: 0., y: 0. });
        let aresta = ((70., 24.), (200., 100.));

        // Longe da faixa da aresta (y 300..348, contra a faixa 24..100): não atrapalha.
        let longe = vec![saida, chegada, Some(Position { x: 100., y: 300. })];
        perto(
            free_main_span(
                FlowOrientation::Horizontal,
                aresta.0,
                aresta.1,
                &longe,
                &state,
            ),
            130.,
        );

        // O MESMO nó, agora na faixa da aresta (y 50..98): atrapalha, e o vão cai pra 200 − 200.
        let dentro = vec![saida, chegada, Some(Position { x: 100., y: 50. })];
        perto(
            free_main_span(
                FlowOrientation::Horizontal,
                aresta.0,
                aresta.1,
                &dentro,
                &state,
            ),
            0.,
        );
    }

    #[test]
    fn vao_livre_ignora_os_dois_nos_da_propria_aresta() {
        // O nó de saída encosta na origem da aresta e o de chegada no destino dela. Se algum dos
        // dois contasse como atravancador, o vão livre sairia zero (ou negativo) em TODA aresta e
        // o cotovelo nunca se desenharia.
        let state = estado(
            vec![medida(70., 48.), medida(70., 48.)],
            lista(vec![no(0), no(1)]),
        );
        let positions = compute_positions(&state, Gaps::default());

        let livre = free_main_span(
            FlowOrientation::Horizontal,
            (70., 24.),
            (134., 24.),
            &positions,
            &state,
        );

        perto(livre, 64.);
    }

    // -------------------------------------------------------------------------
    // Ponta da seta
    // -------------------------------------------------------------------------

    #[test]
    fn ponta_da_seta_aponta_na_direcao_do_ultimo_segmento() {
        let direita = rounded_path(&conector(0., 0., 100., 40.), PathOptions::default());
        let head = arrowhead(&direita).expect("caminho com dois comandos tem ponta");

        perto(head.x, 92.);
        perto(head.y, 40.);
        perto(head.angle, 0.);
    }

    #[test]
    fn ponta_da_seta_gira_no_caminho_vertical() {
        let baixo = rounded_path(
            &conector(0., 0., 40., 100.),
            PathOptions {
                orientation: FlowOrientation::Vertical,
                ..Default::default()
            },
        );
        let head = arrowhead(&baixo).expect("caminho com dois comandos tem ponta");

        perto(head.x, 40.);
        perto(head.y, 92.);
        perto(head.angle, std::f32::consts::FRAC_PI_2);
    }

    #[test]
    fn caminho_curto_demais_nao_tem_ponta() {
        assert_eq!(arrowhead(&[]), None);
        assert_eq!(arrowhead(&[PathCommand::MoveTo { x: 0., y: 0. }]), None);
    }

    #[test]
    fn ponta_da_seta_ignora_segmento_final_de_comprimento_zero() {
        // ⚠️ Isto acontece de verdade, não é caso inventado: num vão apertado a aparagem do
        // cotovelo encosta a perna no ponto onde a seta começa, e o último segmento fica com
        // comprimento ZERO. Lendo a direção dele, o ângulo sairia 0 (apontando pra direita) em
        // qualquer aresta — inclusive nas que descem.
        let descendo = vec![
            PathCommand::MoveTo { x: 0., y: 0. },
            PathCommand::LineTo { x: 0., y: 100. },
            // O segmento morto: termina onde já estava.
            PathCommand::LineTo { x: 0., y: 100. },
        ];

        let head = arrowhead(&descendo).expect("tem ponta");

        perto(head.x, 0.);
        perto(head.y, 100.);
        perto(head.angle, std::f32::consts::FRAC_PI_2);
        assert!(
            (head.angle - 0.).abs() > 1.,
            "ler o segmento morto daria 0 e a seta apontaria pro lado errado"
        );
    }

    #[test]
    fn ponta_da_seta_do_caminho_apertado_de_verdade_aponta_pra_frente() {
        // O caso completo, montado pelo próprio `rounded_path`: vão de 24, onde a aproximação final
        // some. A seta ainda tem que apontar pra frente (ângulo 0 no eixo horizontal).
        let mut c = conector(0., 0., 24., 56.);
        c.free_main_span = Some(24.);
        let commands = rounded_path(&c, PathOptions::default());

        // Confirmando que o último segmento é de fato morto — é o que o teste existe pra cobrir.
        let n = commands.len();
        assert_eq!(
            commands[n - 1].end(),
            commands[n - 2].end(),
            "o vão de 24 devia produzir um segmento final de comprimento zero: {commands:?}"
        );

        let head = arrowhead(&commands).expect("tem ponta");
        perto(head.angle, 0.);
    }

    #[test]
    fn gota_da_seta_encaixa_o_bico_oito_pixels_adiante() {
        // Sem giro, o ponto de encaixe é a metade da borda esquerda da caixa 8×8, então a
        // gota nasce em y - 4 e o bico fica ~6.5px à frente (a curva do meio).
        let head = Arrowhead {
            x: 100.,
            y: 50.,
            angle: 0.,
        };

        let outline = arrowhead_outline(&head);

        assert_eq!(outline.len(), 6);
        assert_eq!(outline[0], PathCommand::MoveTo { x: 100., y: 47.5 });
        let bico = outline[3];
        let PathCommand::QuadTo { cx, cy, .. } = bico else {
            panic!("o terceiro comando devia ser o bico");
        };
        perto(cx, 106.5);
        perto(cy, 50.);
    }

    #[test]
    fn gota_da_seta_gira_com_o_angulo() {
        // A 90°, o que era "pra frente" em x passa a ser "pra baixo" em y.
        let head = arrowhead_outline(&Arrowhead {
            x: 100.,
            y: 50.,
            angle: std::f32::consts::FRAC_PI_2,
        });

        let PathCommand::QuadTo { cx, cy, .. } = head[3] else {
            panic!("o terceiro comando devia ser o bico");
        };
        perto(cx, 100.);
        perto(cy, 56.5);
    }
}

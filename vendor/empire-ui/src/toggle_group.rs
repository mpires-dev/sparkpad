//! `ToggleGroup` — o **botão segmentado**: vários [`crate::Toggle`] numa peça só, com seleção única
//! ou múltipla. Porte do `toggle-group.tsx` do coss.
//!
//! ```ignore
//! let alinhamento = cx.new(|cx| {
//!     ToggleGroup::new(
//!         vec![
//!             ToggleGroupItem::icon("iconoir/regular/align-left.svg"),
//!             ToggleGroupItem::icon("iconoir/regular/align-center.svg"),
//!             ToggleGroupItem::icon("iconoir/regular/align-right.svg"),
//!         ],
//!         cx,
//!     )
//!     .variant(ToggleVariant::Outline)   // costurado, com separador
//!     .mode(ToggleGroupMode::Single)     // acender um APAGA os irmãos
//!     .with_selected(&[0])
//! });
//!
//! cx.subscribe(&alinhamento, |_this, _grupo, e: &ToggleGroupEvent, _cx| match e {
//!     ToggleGroupEvent::Change { item, selected } => println!("clicou {item}, ligados {selected:?}"),
//! })
//! .detach();
//! ```
//!
//! # As duas variantes são dois componentes diferentes
//!
//! O `cva` do toggle tem duas variantes, e o grupo trata cada uma de um jeito **oposto**:
//!
//! | | [`ToggleVariant::Default`] | [`ToggleVariant::Outline`] |
//! |---|---|---|
//! | vão entre itens | **2px** (`gap-0.5`) | zero |
//! | raio e bordas | cada item mantém os seus | a emenda perde os dois (costura) |
//! | separador | não existe | 1px entre cada par |
//! | bisel | não tem (é da `Outline`) | ver "o filete no vertical" |
//!
//! Ou seja: a `Default` é uma FILEIRA de toggles soltos, e a `Outline` é uma PEÇA só. É a diferença
//! entre a barra de ferramentas (negrito/itálico/sublinhado) e o seletor de alinhamento.
//!
//! # Ao lado do [`crate::Group`], não sobre ele
//!
//! O [`crate::Group`] já dá costura por posição, separador automático e o `Join` por filho — e o
//! [`crate::Toggle`] já implementa [`crate::group::GroupChild`], então `Group::new().child(toggle)`
//! **funciona hoje**. Mesmo assim o `ToggleGroup` é construído ao lado dele, e não em cima. O motivo
//! é que **nenhuma** das quatro coisas que este componente precisa está lá — e três delas só fazem
//! sentido pra toggles, então pôr no `Group` seria alargar o componente genérico pra um caso só:
//!
//! 1. **A variante `Default`** quer vão de 2px, sem costura e sem separador. O `Group` não tem vão
//!    (o `gap-2` dele é pra grupo ANINHADO) e o [`crate::Group::seamless`] tira o separador mas
//!    mantém a costura — o oposto do que se quer aqui.
//! 2. **O filete do vertical** é apagado por POSIÇÃO (ver abaixo). O `Group` não tem como falar isso
//!    com um filho: o `Join` diz onde estão as emendas, não o que fazer com o bisel.
//! 3. **A seleção** é estado, e o `Group` é [`gpui::RenderOnce`] — ele não pode ser dono de nada. Um
//!    grupo de seleção única precisa apagar os irmãos, e isso exige um dono só.
//! 4. **O separador reage a LIGADO**, um estado que o `Group` não conhece (ver
//!    [`crate::group::cor_do_separador`]).
//!
//! O que o `Group` tem de reusável não está no container, está nas peças — e são essas que este
//! módulo usa, sem uma segunda cópia de nada: [`crate::group::Join`] (raio, bordas e o sangramento de
//! meio pixel na emenda), [`crate::group::Orientation`], e o par
//! [`crate::group::separador`]/[`crate::group::cor_do_separador`], que é onde vive a regra de clarear
//! o separador conforme o estado do vizinho.
//!
//! # O filete do bisel no vertical
//!
//! No grupo horizontal o bisel de cada item continua a linha do vizinho e o conjunto lê como uma
//! peça só. No **vertical** isso não vale: o filete de um item do meio cairia DENTRO da emenda, em
//! cima do separador — duas linhas de 1px encostadas. A referência resolve escondendo o filete de
//! quase todos:
//!
//! ```text
//! *:data-[slot=toggle]:not-last:before:hidden   → esconde em todos menos o último
//! dark:*:last:before:hidden                     → …no escuro, esconde no último
//! dark:*:first:before:block                     → …e mostra no primeiro
//! ```
//!
//! Traduzido: **o filete sobrevive só na ponta do grupo pra onde o bisel aponta.** No claro ele desce
//! (`0 1px`), então a ponta é a de BAIXO — o último item. No escuro ele sobe (`0 -1px`), então é a de
//! CIMA — o primeiro. É o sinal de `bevel_dir` que escolhe, e não uma tabela de tema: ver
//! [`bevel_visible`] e [`crate::toggle::bevel_dir`]. Se um dia o bisel do claro passar a subir, a
//! regra acompanha sozinha.
//!
//! # Os itens são dado puro, e o grupo é o dono
//!
//! [`ToggleGroupItem`] não é um elemento nem uma `Entity`: é o que o item MOSTRA (rótulo, ícone, se
//! dá pra clicar). O estado ligado/desligado mora no [`ToggleGroup`], num vetor paralelo. Três razões,
//! na ordem de força:
//!
//! 1. **A seleção única é uma regra do CONJUNTO.** Acender um item apaga os irmãos; com o estado
//!    dentro de cada item, essa regra seria uma varredura escrevendo em N donos, e qualquer caminho
//!    que esquecesse a varredura deixaria dois itens acesos.
//! 2. **O [`crate::Toggle`] é controlado por contrato** (ver "Controlado, sempre" no doc dele). Ele é
//!    [`gpui::RenderOnce`] e recebe `pressed` de fora justamente pra não haver um segundo lugar
//!    guardando a mesma verdade.
//! 3. É o mesmo arranjo do [`crate::tabs::TabsTab`] — dado puro dentro de uma `Entity` que emite
//!    evento —, e o do [`crate::Switch`] pro caso de um só.
//!
//! O que É por item e mora no grupo são os três vetores paralelos aos itens: o estado, o
//! [`gpui::ElementId`] e o [`gpui::FocusHandle`]. Eles nascem e morrem junto com a lista (ver
//! [`ToggleGroup::set_items`]) — a mesma disciplina dos `focus_handles`/`tab_bounds` do
//! [`crate::tabs`].
//!
//! # Teclado: roving tabindex, e a seta que NÃO ativa
//!
//! O grupo tem **UMA** parada de `Tab`, e as **setas** movem o foco entre os itens. É o que o
//! `ToggleGroupPrimitive` do base-ui é — um *composite* —, e não uma aproximação: o GPUI 0.2.2 tem
//! [`gpui::FocusHandle::tab_stop`], e a ordem de `Tab` do [`gpui::Window::focus_next`] só enxerga
//! handles com essa marca. Então a cada frame **exatamente um** dos handles do grupo é marcado como
//! parada (ver [`ToggleGroup::mark_tab_stops`]) e os outros são desmarcados — o roving tabindex do
//! HTML, no mecanismo do GPUI.
//!
//! | tecla | efeito |
//! |---|---|
//! | `→` / `←` (grupo horizontal) | próximo / anterior item habilitado, **dando a volta** |
//! | `↓` / `↑` (grupo vertical) | idem, no eixo do grupo |
//! | `Home` / `End` | primeiro / último habilitado |
//! | `Enter` / `Space` | **alterna** o item focado |
//!
//! As quatro sub-decisões, cada uma com a linha do primitivo que a sustenta
//! (`packages/react/src/toggle-group/ToggleGroup.tsx` e o composite que ele monta,
//! `internals/composite/root/useCompositeRoot.ts`):
//!
//! - **A seta move o FOCO, não a seleção.** No composite a seta só mexe no `highlightedIndex` e chama
//!   `.focus()` no elemento; quem alterna é o `<button>` do item, por `Enter`/`Space` ou clique. É a
//!   diferença que importa em relação ao [`crate::radio_group`], onde a seta **marca** (ativação
//!   automática do rádio nativo): um grupo de rádios tem uma escolha, e um grupo de toggles é uma
//!   barra de ferramentas — em [`ToggleGroupMode::Multiple`] vários itens ficam ligados ao mesmo
//!   tempo, e uma seta que ativasse ligaria um item a cada passo do foco, o que nenhuma barra de
//!   ferramentas faz. Em [`ToggleGroupMode::Single`] seria pior ainda: atravessar o grupo com a seta
//!   deixaria a seleção no último item visitado, sem o usuário ter escolhido nada.
//! - **Só o eixo do grupo anda.** O primitivo passa o `orientation` (default `horizontal`) ao
//!   composite, e lá `isForwardKey` é `orientation !== 'vertical' && ArrowRight` **ou**
//!   `orientation !== 'horizontal' && ArrowDown`: num grupo horizontal o `↓` não é tratado, e num
//!   vertical o `→` não é. Ver [`arrow_dir`]. Isto **difere** do [`crate::radio_group`], onde as
//!   quatro setas fazem a mesma coisa — e a razão é a referência de cada um: o `radio-group.tsx` crava
//!   `flex-col` (existe uma direção só, e o rádio nativo responde às quatro), enquanto aqui o eixo é
//!   uma prop e o primitivo filtra as teclas por ela.
//! - **Dá a volta nas pontas.** O `loopFocus` do primitivo é `true` por default ("whether to loop
//!   keyboard focus back to the first item when the end of the list is reached while using the arrow
//!   keys").
//! - **`Home`/`End` existem**, e vão pras pontas habilitadas: o `ToggleGroup` do base-ui passa
//!   `enableHomeAndEndKeys` fixo em `true`. Não é superset — é a referência.
//!
//! **Qual item é a parada de `Tab`**: o **último que teve foco**, e o primeiro habilitado enquanto
//! ninguém teve. É o `highlightedIndex` do composite, que começa em 0 e é reescrito pelo `onFocus` de
//! cada item — com o mesmo cuidado que o primitivo documenta pro caso do item travado ("a natively
//! disabled element is removed from the tab order, and an aria-disabled one should not be the entry
//! point. Move the tab stop to the first enabled item"). Daqui saem duas coisas: o campo
//! [`ToggleGroup::last_focused`] (não existe "o selecionado" aqui — em `Multiple` são vários) e o
//! reuso do [`crate::radio_group::tab_stop_index`], que é essa mesma pergunta com o argumento
//! significando outra coisa. Grupo com **nenhum** item habilitado sai inteiro da ordem de tabulação.
//!
//! ## O que veio do [`crate::radio_group`], e o que não
//!
//! [`crate::radio_group::step_index`] (a seta, com volta e pulando travado),
//! [`crate::radio_group::edge_index`] (`Home`/`End`) e [`crate::radio_group::tab_stop_index`] (a
//! parada) são **os mesmos** aqui e lá — a geometria do movimento não depende do que o movimento
//! significa. Reusar por promoção de visibilidade é o que impede as duas cópias divergirem no primeiro
//! refactor (o `step_index` do [`crate::tabs`] é a terceira cópia, e o lugar certo dos três é um
//! módulo de navegação compartilhado; extrair é um passo separado, e `tabs.rs` não é deste escopo).
//!
//! O que **não** deu pra compartilhar é o [`enabled_bits`]: o do rádio recebe `&[RadioGroupItem]` e
//! compõe com um `disabled` de GRUPO, que este componente não tem (aqui só o item se desabilita). São
//! quatro linhas com tipos diferentes, e forçar um genérico esconderia essa diferença de contrato em
//! vez de declará-la — a mesma leitura que o [`crate::meter`] fez sobre o [`crate::slider`] em "O que
//! NÃO é compartilhado".
//!
//! ## Um detalhe medido: item desabilitado não tem foco rastreado
//!
//! O [`crate::Toggle`] só pendura `track_focus` quando o item está **habilitado**, e o `tab_stop` do
//! GPUI só vale pra handle que foi inserido na tabela de paradas por um `track_focus` — a inserção
//! copia o `tab_stop` do handle naquele instante (`TabStopMap::insert`). Ou seja: **marcar o handle de
//! um item travado não faz nada**, o handle não está na tabela. Está medido em
//! [`tests_de_janela::marcar_tab_stop_sem_track_focus_nao_entra_na_ordem`], e é uma segunda camada de
//! defesa e não a primeira: o [`crate::radio_group::tab_stop_index`] nunca escolhe um item travado. As
//! duas juntas são o que faz um grupo inteiramente desabilitado sair do `Tab`.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`*:focus-visible:z-10`**: o GPUI 0.2 não tem `z-index`, e a ordem de pintura é a ordem dos
//!   filhos. O [`crate::group`] já declara exatamente esta limitação ("o anel de foco não passa por
//!   cima do vizinho… o anel para na emenda e quem fecha o contorno naquele lado é o separador"), e
//!   aqui é igual: o anel do item focado para na emenda. A diferença sobra pros itens do MEIO de um
//!   grupo `Outline`, que ficam com o anel aberto nos dois lados. No coss o `z-10` existe pra que o
//!   anel do item focado cubra o vizinho — e é justamente o que não dá pra fazer sem `z-index`.
//! - **O separador não vira a cor do anel** quando o vizinho está focado. Não é omissão: a regra
//!   `has-[+[data-slot=input-control]:focus-within]:bg-ring` do coss é dos CAMPOS, e o
//!   `toggle-group.tsx` não tem nada equivalente. O parâmetro existe em
//!   [`crate::group::separador`] e este módulo passa `false` de propósito.
//! - **`*:pointer-coarse:after:min-w-auto` / `min-h-auto`**: o grupo desliga o alvo de 44px que o
//!   toggle põe em ponteiro GROSSO (dedo). `pointer-coarse` é consulta de mídia e numa janela de
//!   desktop com mouse ela nunca vale — o [`crate::toggle`] já declarou o alvo como ramo morto, e
//!   desligar um ramo morto é um no-op.
//! - **`pointer-events-none` do separador**: o separador daqui é uma `div` sem `id` e sem listener,
//!   então o GPUI não cria hitbox pra ela — ela já não intercepta ponteiro. Não há o que desligar.
//!
//! ⚠️ **O `roving tabindex` MOROU nesta lista, e a justificativa estava errada.** Ela dizia que
//! reproduzi-lo "exigiria roubar o `Tab` dos filhos, e o [`crate::Toggle`] é quem monta o
//! `track_focus` — a decisão não é do grupo". Os dois elos estavam furados: (1) o GPUI **tem** o
//! mecanismo, [`gpui::FocusHandle::tab_stop`], e não é preciso interceptar `Tab` nenhum — basta marcar
//! um handle por frame; (2) o `track_focus` é montado pelo `Toggle`, mas o **handle é do grupo** (ele
//! cria os [`ToggleGroup::focus_handles`] e passa cada um ao filho por `Toggle::focus`), e o
//! `tab_stop` grava no `FocusRef` COMPARTILHADO por todos os clones daquele handle — então o grupo
//! decide sem tocar na API do filho. O registro fica aqui de propósito: a declaração foi escrita
//! **antes** de alguém usar o `tab_stop` nesta base, o [`crate::radio_group`] foi o primeiro a usá-lo
//! (e já anotou lá que esta seção estava desatualizada), e uma "impossibilidade" declarada é
//! exatamente o tipo de erro que se propaga por cópia. O comportamento de hoje está em "Teclado:
//! roving tabindex, e a seta que NÃO ativa", acima.
//!
//! E a mesma declaração errava também na descrição do que ELA fazia: dizia que "cada item habilitado
//! entra na ordem de tabulação (`Tab` anda item a item)". **Medido** ao remover o
//! [`ToggleGroup::mark_tab_stops`] e rodar os testes de janela: sem marca nenhuma, o
//! [`gpui::Window::focus_next`] não alcança item nenhum do grupo. O `FocusHandle` do GPUI nasce com
//! `tab_stop: false` e a tabela de paradas do frame copia esse valor no `track_focus` — então o grupo
//! não custava N paradas de `Tab`, ele **não tinha nenhuma**: era operável só por ponteiro (e por
//! tecla depois de um clique). O defeito de acessibilidade era pior do que a própria declaração dizia.
//!
//! **Resolvido em número**
//!
//! - `gap-0.5` da variante `Default` = **2px** (a unidade do `--spacing` é 4px). Ver [`GAP`].
//! - O separador tem **1px** no eixo, atravessado (`w-px`/`h-px` + `self-stretch`), e o valor é o
//!   [`crate::group::SEPARATOR`] — o mesmo do `Group`, não uma cópia.
//! - O alfa do separador no escuro é a COMPOSIÇÃO das duas camadas do original (`bg-input` mais o
//!   `dark:before:bg-input/32` por cima), com o `/N` já multiplicando o alfa que `--input` tem: os
//!   três degraus dão ~10,4%, ~12,7% e ~15,4%. A conta está em [`crate::group`] (nos campos
//!   `input`/`input_hover`/`input_on`) e o teste que a trava está aqui, em
//!   [`tests::o_alfa_do_separador_compoe_as_duas_camadas`].
//! - `variant`/`size` chegam nos itens porque o grupo os passa (é o `ToggleGroupContext` do
//!   original). E o item **não pode divergir**: no coss o `context.variant || variant` faz o valor do
//!   contexto vencer sempre — o `variant` do item é código morto lá. Aqui isso é tipo:
//!   [`ToggleGroupItem`] não tem campo de variante nem de tamanho.
//!
//! **Desvio consciente**
//!
//! - **A variante `Default` vertical empilha.** No original o `flex-col` está SÓ no ramo da `Outline`
//!   (ver a expressão condicional do `className`), então um grupo `default` com
//!   `orientation="vertical"` sai em linha, com o `orientation` valendo só pro teclado. É um
//!   esquecimento da referência, não uma intenção: as duas variantes aqui empilham.
//! - **O separador é automático**, como no [`crate::Group`]: o coss exporta um
//!   `ToggleGroupSeparator` pra quem usa colocar à mão entre os itens (e as regras `dark:` do grupo
//!   assumem que ele está lá). Um separador obrigatório que se pode esquecer é um defeito esperando
//!   pra acontecer — mesma decisão, mesma razão. A orientação dele também sai do grupo, em vez de ser
//!   um parâmetro que dá pra errar (no original o default é `vertical`, e num grupo vertical quem usa
//!   tem que lembrar de passar `horizontal`).
//!
//! **Superset consciente**
//!
//! - **[`ToggleGroupMode`]** nomeia o que no base-ui é um booleano (`toggleMultiple`). Duas variantes
//!   nomeadas em vez de `bool` porque no call site `ToggleGroupMode::Single` diz o que acontece e
//!   `false` não diz nada.
//! - **`Enter`/`Space`** alternam o item focado. O primitivo do base-ui faz isso via `<button>`
//!   nativo; no GPUI não existe ativação por teclado de graça, então ela é explícita — sem isso, um
//!   item alcançável por `Tab` seria um beco sem saída (a mesma razão que fez o [`crate::Switch`]
//!   tratar as duas teclas).
//!
//! # Sem cobertura de teste — declarado
//!
//! - **A reatribuição do handle no [`ToggleGroup::mark_tab_stops`]** sobrevive à mutação, e por quê
//!   está no doc dela: o que muda o comportamento é a escrita no `FocusRef` compartilhado.
//! - **A pintura**: o anel de foco do item, o esmaecimento do travado e a cor do separador são do
//!   [`crate::toggle`] e do [`crate::group`], e são pixel. O que dá pra travar em número está travado
//!   (ver [`tests::o_alfa_do_separador_compoe_as_duas_camadas`]).
//! - **O guarda `if self.items[i].disabled` do [`ToggleGroup::on_key`]** é defesa em profundidade e não
//!   é falsificável: o item travado não recebe `track_focus`, então ele não pode ser o item focado. Ele
//!   fica pelo dia em que o [`crate::Toggle`] mudar isso — é a mesma decisão que o
//!   [`crate::radio_group`] declara pro gate `if habilitado` do `item_row` dele.
//! - **Um grupo de teclado dentro de outro foco de janela**: que o `Tab` SAIA do grupo pro próximo
//!   controle da tela (e não fique preso) é propriedade da tabela de paradas do GPUI, que é global. Os
//!   testes daqui medem que o grupo tem uma parada só; que a de fora venha depois é do
//!   [`gpui::Window::focus_next`], e o harness de um grupo sozinho não tem "de fora".

use gpui::{
    div, px, Context, ElementId, EventEmitter, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, SharedString, Styled, Window,
};

use crate::group::{separador, Join, Orientation, SeparatorTone};
use crate::toggle::{Toggle, ToggleSize, ToggleVariant};

// =================================================================================================
// Geometria
// =================================================================================================
//
// Cor NÃO aparece neste módulo, e é de propósito: a dos itens é do `crate::toggle`, a do separador é
// do `crate::group`. Uma terceira paleta aqui seria uma terceira fonte de verdade pro mesmo
// `--input`.

/// `gap-0.5` — o vão entre os itens na variante [`ToggleVariant::Default`], em px.
///
/// A unidade do `--spacing` do Tailwind é 4px, então `0.5` são **2px**. É o mesmo número do
/// `LIST_GAP` das abas, e por coincidência de desenho, não por dependência.
const GAP: f32 = 2.0;

/// O vão entre os itens, por variante.
///
/// Só a `Default` tem: na `Outline` os itens são costurados (vão zero), e o que os separa é o
/// separador de 1px.
fn gap(variant: ToggleVariant) -> f32 {
    match variant {
        ToggleVariant::Default => GAP,
        ToggleVariant::Outline => 0.0,
    }
}

/// Se esta variante põe **separador** entre os itens.
///
/// É a mesma pergunta que [`gap`] faz, com a resposta invertida — e as duas existem separadas porque
/// é assim que a referência as escreve (um ramo do ternário é `gap-0.5`, o outro é a costura). Se um
/// dia aparecer uma terceira variante, ela vai ter que responder às duas.
fn separators(variant: ToggleVariant) -> bool {
    match variant {
        ToggleVariant::Default => false,
        ToggleVariant::Outline => true,
    }
}

/// **Como o item `i` de `total` encosta nos vizinhos.**
///
/// Na `Default` ninguém encosta em ninguém: cada item mantém os quatro cantos e os quatro lados, e o
/// vão de [`GAP`] os separa. Na `Outline` é a costura posicional do [`crate::Group`] — o primeiro
/// aberto no início, o último aberto no fim, o meio fechado dos dois lados.
fn join_for(
    i: usize,
    total: usize,
    variant: ToggleVariant,
    orientation: Orientation,
) -> Join {
    if !separators(variant) {
        return Join::NONE;
    }
    Join {
        start: i > 0,
        end: i + 1 < total,
        orientation,
    }
}

/// **Se o item `i` de `total` mostra o fio de bisel.**
///
/// No horizontal, sempre: o filete de cada item continua o do vizinho e o conjunto vira uma linha
/// só (é pra isso que existe o sangramento de meio pixel do [`Join::bevel_overlay`]).
///
/// No vertical, só na PONTA do grupo pra onde o bisel aponta — ver "o filete do bisel no vertical" no
/// doc do módulo. `bevel_dir > 0` desce, então a ponta é o ÚLTIMO item; `< 0` sobe, e é o PRIMEIRO.
/// A regra sai do sinal, e não de "claro/escuro": num grupo de um item só ele é primeiro e último,
/// então o filete aparece nos dois casos, como na referência.
///
/// A variante não entra na conta porque não precisa: a [`ToggleVariant::Default`] não tem bisel
/// nenhum (ver `variant_style` no [`crate::toggle`]), então esconder o dela é um no-op.
fn bevel_visible(i: usize, total: usize, orientation: Orientation, bevel_dir: f32) -> bool {
    match orientation {
        Orientation::Horizontal => true,
        Orientation::Vertical if bevel_dir > 0.0 => i + 1 == total,
        Orientation::Vertical => i == 0,
    }
}

// =================================================================================================
// Seleção
// =================================================================================================

/// Quantos itens podem estar ligados ao mesmo tempo.
///
/// É o `toggleMultiple` do `ToggleGroupPrimitive`, com nome em vez de booleano (ver "Superset
/// consciente" no doc do módulo). O default é [`Self::Single`], o mesmo do base-ui.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToggleGroupMode {
    /// **Um só.** Acender um item apaga os irmãos; clicar no que já está aceso o apaga, e o grupo
    /// fica vazio.
    ///
    /// O grupo poder ficar vazio é o comportamento do base-ui, e não um esquecimento: um seletor de
    /// alinhamento que não deixa desmarcar é um botão de rádio, e aí o componente seria outro.
    #[default]
    Single,
    /// **Quantos quiserem.** Cada item é independente — é o negrito/itálico/sublinhado.
    Multiple,
}

/// Aplica um toque no item `idx` sobre o estado `pressed`, devolvendo o estado NOVO.
///
/// `None` = índice fora da lista, e aí nada muda. Quando o índice é válido a mudança é certa: o bit
/// do item clicado sempre inverte (é o que faz clicar no item aceso de um grupo `Single` apagá-lo).
///
/// É o miolo puro da seleção, e o único lugar onde o modo importa — por isso ele é livre e testável
/// sem `Context` nem GPU, no mesmo espírito do `apply_choice` do [`crate::tabs`].
fn apply_toggle(pressed: &[bool], mode: ToggleGroupMode, idx: usize) -> Option<Vec<bool>> {
    let atual = *pressed.get(idx)?;
    let mut novo = match mode {
        ToggleGroupMode::Multiple => pressed.to_vec(),
        // **Aqui está a seleção única.** Sem esta linha o grupo viraria múltiplo em silêncio: o
        // resto do código não tem nenhum outro lugar que apague um irmão.
        ToggleGroupMode::Single => vec![false; pressed.len()],
    };
    novo[idx] = !atual;
    Some(novo)
}

/// Monta o vetor de estado a partir de uma lista de índices ligados.
///
/// Índices fora da lista são ignorados (a lista de itens é a verdade sobre o tamanho). No modo
/// [`ToggleGroupMode::Single`] só o PRIMEIRO índice válido sobrevive: pedir dois num grupo de seleção
/// única é dado inválido, e a alternativa (aceitar os dois) produziria um grupo em estado que nenhum
/// clique consegue alcançar.
fn pressed_bits(len: usize, on: &[usize], mode: ToggleGroupMode) -> Vec<bool> {
    let mut bits = vec![false; len];
    for &i in on {
        if i < len {
            bits[i] = true;
            if mode == ToggleGroupMode::Single {
                break;
            }
        }
    }
    bits
}

/// Os índices ligados, em ordem crescente — a forma pública do estado.
fn selected_indices(pressed: &[bool]) -> Vec<usize> {
    pressed
        .iter()
        .enumerate()
        .filter_map(|(i, on)| on.then_some(i))
        .collect()
}

// =================================================================================================
// Teclado — o roving tabindex e o movimento do foco
// =================================================================================================
//
// As três funções que a navegação usa (`step_index`, `edge_index` e `tab_stop_index`) NÃO estão aqui:
// são as do `crate::radio_group`, reusadas por promoção de visibilidade. Ver "O que veio do
// radio_group, e o que não" no doc do módulo.

/// **Quem está habilitado**, item por item — a máscara que a seta e a parada de `Tab` consultam.
///
/// Diferente do gêmeo do [`crate::radio_group`], não há `disabled` de GRUPO pra compor: aqui só o
/// item se desabilita. É por isso que esta função não é compartilhada — ver o doc do módulo.
fn enabled_bits(items: &[ToggleGroupItem]) -> Vec<bool> {
    items.iter().map(|it| !it.disabled).collect()
}

/// **O sentido que uma seta anda**, ou `None` quando ela não é do eixo do grupo.
///
/// Só o eixo do grupo navega, e isso é a referência e não economia: no composite do base-ui o
/// `isForwardKey` é `orientation !== 'vertical' && ArrowRight` **ou** `orientation !== 'horizontal' &&
/// ArrowDown`, ou seja num grupo horizontal o `↓`/`↑` não é tratado e num vertical o `→`/`←` não é.
///
/// É função pura porque a alternativa é um `match` dentro do `on_key`, onde trocar o eixo por engano
/// (ou fazer as quatro setas andarem, como o [`crate::radio_group`] faz com razão) não quebraria nada
/// que compile.
fn arrow_dir(key: &str, orientation: Orientation) -> Option<isize> {
    match (orientation, key) {
        (Orientation::Horizontal, "right") => Some(1),
        (Orientation::Horizontal, "left") => Some(-1),
        (Orientation::Vertical, "down") => Some(1),
        (Orientation::Vertical, "up") => Some(-1),
        _ => None,
    }
}

/// **Pra onde o FOCO vai** com a tecla `key`, partindo do item `focado`. `None` = a tecla não move
/// foco, ou não há pra onde ir.
///
/// Note o que esta função **não** devolve: nada sobre seleção. É aqui que a diferença em relação ao
/// [`crate::radio_group`] fica observável — lá o alvo da seta vai pro `choose`, aqui ele vai só pro
/// `window.focus`.
///
/// `focado = None` (o foco não está em nenhum item) faz a seta entrar pela ponta de onde ela vem, que
/// é o comportamento do [`crate::radio_group::step_index`]. No composite do base-ui esse estado não
/// existe (há sempre um `highlightedIndex`), e aqui ele é inalcançável pela interface — a tecla só
/// chega à raiz do grupo quando o foco está num descendente dela.
fn focus_target(
    key: &str,
    orientation: Orientation,
    focado: Option<usize>,
    enabled: &[bool],
) -> Option<usize> {
    if let Some(dir) = arrow_dir(key, orientation) {
        return crate::radio_group::step_index(focado, enabled, dir);
    }
    match key {
        "home" => crate::radio_group::edge_index(enabled, false),
        "end" => crate::radio_group::edge_index(enabled, true),
        _ => None,
    }
}

// =================================================================================================
// Um item
// =================================================================================================

/// A **definição** de um item: o que ele mostra e se dá pra clicar.
///
/// Não é um elemento nem uma `Entity` — ver "os itens são dado puro" no doc do módulo. Variante e
/// tamanho não estão aqui de propósito: eles vêm do grupo, e o item não pode divergir.
#[derive(Clone, Debug)]
pub struct ToggleGroupItem {
    label: Option<SharedString>,
    icon: Option<SharedString>,
    disabled: bool,
}

impl ToggleGroupItem {
    /// Um item com **rótulo**.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: Some(label.into()),
            icon: None,
            disabled: false,
        }
    }

    /// Um item só de **ícone** — quadrado, pelo `min-w` do [`crate::ToggleSize`].
    ///
    /// O caminho é servido pela [`crate::assets::Assets`] (ex.:
    /// `"iconoir/regular/align-left.svg"`). Sem essa `AssetSource` registrada no bootstrap, o ícone
    /// some SILENCIOSAMENTE.
    pub fn icon(path: impl Into<SharedString>) -> Self {
        Self {
            label: None,
            icon: Some(path.into()),
            disabled: false,
        }
    }

    /// Um ícone **antes do rótulo**.
    pub fn with_icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }

    /// Desabilita o item: ele para de responder a clique e hover, sai da ordem de tabulação e
    /// esmaece pra 64%.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// O rótulo, se houver.
    pub fn label(&self) -> Option<&SharedString> {
        self.label.as_ref()
    }

    /// O caminho do ícone, se houver.
    pub fn icon_path(&self) -> Option<&SharedString> {
        self.icon.as_ref()
    }

    /// Se o item está desabilitado.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// Evento emitido quando o usuário mexe no grupo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleGroupEvent {
    /// Alguém foi tocado. Carrega as duas coisas que quem assina pode querer:
    ///
    /// - `item`: o índice que o usuário TOCOU;
    /// - `selected`: os índices ligados DEPOIS do toque, em ordem crescente.
    ///
    /// Os dois porque num grupo [`ToggleGroupMode::Single`] o toque muda o estado de dois itens (o
    /// que acendeu e o que apagou), então "o índice tocado" não descreve o resultado — e "o
    /// resultado" não diz o que o usuário fez.
    Change {
        /// O índice tocado.
        item: usize,
        /// Os índices ligados depois do toque.
        selected: Vec<usize>,
    },
}

/// Botão segmentado com o visual do coss. Ver o doc do módulo.
pub struct ToggleGroup {
    /// Os itens, na ordem dos índices.
    items: Vec<ToggleGroupItem>,
    /// Ligado/desligado por item. **Sempre do tamanho de `items`** — ver [`Self::set_items`].
    pressed: Vec<bool>,
    /// O id de cada item, pré-montado.
    ///
    /// Ele é a chave da tabela de hover do [`crate::button`] (que é global por [`ElementId`], não por
    /// caminho na árvore), então carrega o id da ENTIDADE: dois grupos com itens de mesmo índice não
    /// podem clarear o separador um do outro. Pré-montado porque o `render` roda a cada frame e
    /// formatar N strings por frame é lixo que dá pra não produzir.
    item_ids: Vec<ElementId>,
    /// Um handle de foco por item — é o que liga o anel e, pelo [`gpui::FocusHandle::tab_stop`], o
    /// que põe **um** item na ordem de tabulação. O grupo é o dono deles e passa cada um ao filho por
    /// `Toggle::focus`; ver "Teclado" no doc do módulo.
    focus_handles: Vec<FocusHandle>,
    /// **O último item que teve o foco** — e, por isso, a parada de `Tab` do grupo.
    ///
    /// É o `highlightedIndex` do composite do base-ui. Não dá pra derivar da seleção como o
    /// [`crate::radio_group`] faz: em [`ToggleGroupMode::Multiple`] não existe "o selecionado", e em
    /// `Single` a seleção pode estar vazia. Fica `None` até alguém focar (e aí a parada é o primeiro
    /// item habilitado), e **não** é apagado quando o foco sai do grupo: voltar por `Tab` devolve o
    /// usuário ao item onde ele estava, que é o que o composite faz.
    ///
    /// Tem **um escritor só**: a sincronização no [`Render::render`], que lê o foco de verdade da
    /// janela. É de propósito — o foco chega por clique, por `Tab` e por seta, e só o render vê os
    /// três. Ver o comentário lá.
    last_focused: Option<usize>,
    mode: ToggleGroupMode,
    variant: ToggleVariant,
    size: ToggleSize,
    orientation: Orientation,
    /// Id estável da entidade, pro `div().id(..)` da raiz e pros ids dos itens.
    id: u64,
}

impl ToggleGroup {
    /// Um grupo com `items`, nenhum ligado.
    pub fn new(items: Vec<ToggleGroupItem>, cx: &mut Context<Self>) -> Self {
        let id = cx.entity_id().as_u64();
        Self {
            pressed: vec![false; items.len()],
            item_ids: item_ids(items.len(), id),
            focus_handles: items.iter().map(|_| cx.focus_handle()).collect(),
            last_focused: None,
            items,
            mode: ToggleGroupMode::default(),
            variant: ToggleVariant::default(),
            size: ToggleSize::default(),
            orientation: Orientation::default(),
            id,
        }
    }

    /// Seleção única ou múltipla. Ver [`ToggleGroupMode`].
    pub fn mode(mut self, mode: ToggleGroupMode) -> Self {
        self.mode = mode;
        self
    }

    /// A variante dos itens — o grupo a propaga, e o item não pode divergir.
    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    /// O tamanho dos itens — idem.
    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    /// Empilha os itens em coluna em vez de lado a lado.
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Liga os itens de `on`. No modo [`ToggleGroupMode::Single`] só o primeiro índice válido vale
    /// (ver [`pressed_bits`]).
    pub fn with_selected(mut self, on: &[usize]) -> Self {
        self.pressed = pressed_bits(self.items.len(), on, self.mode);
        self
    }

    /// Os itens, na ordem dos índices.
    pub fn items(&self) -> &[ToggleGroupItem] {
        &self.items
    }

    /// Os índices ligados, em ordem crescente.
    pub fn selected(&self) -> Vec<usize> {
        selected_indices(&self.pressed)
    }

    /// Se o item `i` está ligado (`false` pra índice fora da lista).
    pub fn is_selected(&self, i: usize) -> bool {
        self.pressed.get(i).copied().unwrap_or(false)
    }

    /// Liga exatamente os itens de `on` (sincronização de fora). **Não** emite — evita loop de
    /// feedback com quem assina, mesma regra do [`crate::tabs::Tabs::set_selected`].
    pub fn set_selected(&mut self, on: &[usize], cx: &mut Context<Self>) {
        self.pressed = pressed_bits(self.items.len(), on, self.mode);
        cx.notify();
    }

    /// Troca a lista de itens. Sincronização de fora, **não** emite.
    ///
    /// Os três vetores paralelos (estado, ids, handles) são refeitos junto: eles são POR ITEM, e um
    /// deles fora de sincronia com a lista é um `panic` de índice esperando um render. Perder a
    /// seleção é o preço de honestidade — os índices antigos não descrevem mais nada, e é a mesma
    /// razão pela qual o [`Self::last_focused`] volta pra `None`: a parada de `Tab` do grupo novo é o
    /// primeiro item habilitado dele.
    pub fn set_items(&mut self, items: Vec<ToggleGroupItem>, cx: &mut Context<Self>) {
        self.pressed = vec![false; items.len()];
        self.item_ids = item_ids(items.len(), self.id);
        self.focus_handles = items.iter().map(|_| cx.focus_handle()).collect();
        self.last_focused = None;
        self.items = items;
        cx.notify();
    }

    /// Põe o foco de teclado na **parada de `Tab`** do grupo — o último item que teve foco, ou o
    /// primeiro habilitado. No-op num grupo sem nenhum item habilitado.
    ///
    /// Existe pela mesma razão do gêmeo no [`crate::radio_group`]: com roving tabindex, quem chama não
    /// tem como saber qual dos N handles é a entrada do grupo.
    pub fn focus(&self, window: &mut Window) {
        let enabled = enabled_bits(&self.items);
        if let Some(i) = crate::radio_group::tab_stop_index(self.last_focused, &enabled) {
            if let Some(h) = self.focus_handles.get(i) {
                h.focus(window);
            }
        }
    }

    /// Toca o item `idx` por interação: aplica a regra do modo e **emite**
    /// [`ToggleGroupEvent::Change`].
    fn choose(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(novo) = apply_toggle(&self.pressed, self.mode, idx) else {
            return;
        };
        self.pressed = novo;
        let selected = self.selected();
        cx.emit(ToggleGroupEvent::Change { item: idx, selected });
        cx.notify();
    }

    /// O índice do item que está com o foco de teclado, se algum.
    fn focused_item(&self, window: &Window) -> Option<usize> {
        self.focus_handles.iter().position(|h| h.is_focused(window))
    }

    /// **O roving tabindex**: marca `tab_stop(true)` no handle de `parada` e `false` em todos os
    /// outros.
    ///
    /// O `tab_stop` grava no `FocusRef` que todos os clones daquele handle compartilham, e o
    /// [`gpui::Window::focus_next`] só enxerga handles marcados — então marcar UM por frame é o que dá
    /// uma parada de `Tab` pro grupo inteiro. `parada = None` (nenhum item habilitado) deixa o grupo
    /// inteiro fora da ordem de tabulação.
    ///
    /// Dois detalhes que a implementação depende:
    ///
    /// - o valor é escrito **sempre**, e não só quando é `true`: senão o item que já foi parada
    ///   continuaria sendo depois de o foco sair dele, e o grupo teria duas;
    /// - o handle guardado é substituído pelo que o `tab_stop` devolve. Esta parte **não é falsificável
    ///   por teste, e está medido**: a escrita no `FocusRef` já basta pro comportamento (o
    ///   `Toggle::focus` clona, e o `Clone` do [`gpui::FocusHandle`] relê o `FocusRef`), então trocar a
    ///   reatribuição por um `let _ =` não quebra teste nenhum. Ela fica porque um campo local mentindo
    ///   sobre o estado compartilhado é o tipo de coisa que morde quem for ler
    ///   `focus_handles[i].tab_stop` depois.
    fn mark_tab_stops(&mut self, parada: Option<usize>) {
        for i in 0..self.focus_handles.len() {
            let marcado = parada == Some(i);
            let handle = self.focus_handles[i].clone().tab_stop(marcado);
            self.focus_handles[i] = handle;
        }
    }

    /// Teclado: as **setas** (mais `Home`/`End`) movem o foco, e `Enter`/`Space` alternam o item
    /// FOCADO. Ver a tabela em "Teclado" no doc do módulo.
    ///
    /// A seta **não** alterna nada — é a diferença em relação ao [`crate::radio_group`], e ela está
    /// declarada com a linha do primitivo que a sustenta no doc do módulo. `Enter`/`Space` existem
    /// porque no GPUI não há ativação por teclado de graça: sem elas, um item alcançado por `Tab` seria
    /// um beco sem saída.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if matches!(key, "enter" | "space") {
            let Some(i) = self.focused_item(window) else {
                return;
            };
            // O item desabilitado nem recebe `track_focus` (o [`crate::Toggle`] só o pendura quando
            // está habilitado), então este guarda é redundante — e fica, porque o dia em que o toggle
            // mudar isso, é aqui que um item inerte passaria a responder à tecla.
            if self.items[i].disabled {
                return;
            }
            crate::focus_ring::keyboard_used(window);
            self.choose(i, cx);
            return;
        }

        let enabled = enabled_bits(&self.items);
        let Some(alvo) = focus_target(key, self.orientation, self.focused_item(window), &enabled)
        else {
            return;
        };
        // Chegou aqui = o usuário está no teclado. O `focus_ring::init` já teria marcado isso; marcar
        // de novo faz a seta acender o anel mesmo num app que esqueceu de inicializar o crate.
        crate::focus_ring::keyboard_used(window);
        // Mover o foco é TUDO o que a seta faz:
        //
        // - a seleção não muda (**nenhum `choose` aqui**) — ver "Teclado" no doc do módulo;
        // - o [`Self::last_focused`] não é escrito aqui de propósito. Ele tem um escritor só, a
        //   sincronização no [`Render::render`], e escrever nos dois lugares seria uma escrita
        //   inalcançável por teste: o `window.focus` chama `refresh`, então o render acontece e lê o
        //   foco novo — e quando o `focus` é no-op (o alvo já era o focado), o valor que a escrita
        //   adiantada gravaria é o que já está lá. Medido por mutação: remover a escrita daqui não
        //   quebra teste nenhum, então ela não fica.
        if let Some(h) = self.focus_handles.get(alvo) {
            window.focus(h);
        }
        // Sem `cx.notify()`: o repintado vem do `refresh` do `window.focus` (e do
        // `focus_ring::keyboard_used`, quando a modalidade muda). Quando o alvo já era o item focado,
        // nada mudou e não há o que repintar.
    }

    /// O [`Toggle`] do item `i`, já com estado, costura, filete e clique.
    fn item_element(&self, i: usize, bevel_dir: f32, cx: &mut Context<Self>) -> Toggle {
        let item = &self.items[i];
        let total = self.items.len();
        let id = self.item_ids[i].clone();

        let mut toggle = match (item.label.clone(), item.icon.clone()) {
            (Some(label), Some(icon)) => Toggle::new(id, label).with_icon(icon),
            (Some(label), None) => Toggle::new(id, label),
            (None, Some(icon)) => Toggle::icon(id, icon),
            // Os construtores de [`ToggleGroupItem`] não deixam chegar aqui (um item é rótulo, ícone,
            // ou os dois). Um toggle vazio é melhor que um `panic` no meio de um render.
            (None, None) => Toggle::new(id, ""),
        };

        toggle = toggle
            .pressed(self.pressed[i])
            .disabled(item.disabled)
            // O contexto do original: variante e tamanho descem do grupo.
            .variant(self.variant)
            .size(self.size);

        if let Some(handle) = self.focus_handles.get(i) {
            toggle = toggle.focus(handle);
        }

        toggle.join = join_for(i, total, self.variant, self.orientation);
        toggle.bevel_hidden = !bevel_visible(i, total, self.orientation, bevel_dir);

        toggle.on_toggle(cx.listener(move |this, _novo: &bool, _window, cx| {
            // O valor que o toggle manda é ignorado de propósito. Num grupo `Single` o toque muda
            // mais de um item, então o novo estado do grupo não é derivável do que o item clicado
            // acha que virou — quem sabe é o [`Self::choose`], que olha o vetor inteiro. Aceitar o
            // valor aqui seria uma segunda fonte de verdade, e a de dentro é sempre a que
            // dessincroniza.
            this.choose(i, cx);
        }))
    }
}

/// Os ids dos itens de um grupo. Ver [`ToggleGroup::item_ids`].
fn item_ids(total: usize, entity: u64) -> Vec<ElementId> {
    (0..total)
        .map(|i| ElementId::Name(SharedString::from(format!("empire-toggle-group-{entity}-{i}"))))
        .collect()
}

impl EventEmitter<ToggleGroupEvent> for ToggleGroup {}

impl Render for ToggleGroup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let variant = self.variant;
        let orientation = self.orientation;
        let total = self.items.len();

        // --- O roving tabindex ------------------------------------------------------------------
        //
        // Primeiro sincronizar o "último focado" com o foco de VERDADE: o foco também chega por
        // clique e por `Tab`, e nenhum dos dois passa pelo `on_key`. É o equivalente do `onFocus` de
        // cada item do composite do base-ui, que é lá que o `highlightedIndex` é reescrito. Ler no
        // render funciona porque toda troca de foco pede um frame novo (`Window::focus` chama
        // `refresh`).
        if let Some(i) = self.focused_item(window) {
            self.last_focused = Some(i);
        }
        let enabled = enabled_bits(&self.items);
        // A pergunta é a mesma que o [`crate::radio_group`] faz, com o argumento significando outra
        // coisa: lá é "o selecionado, senão o primeiro habilitado", aqui é "o último focado, senão o
        // primeiro habilitado".
        let parada = crate::radio_group::tab_stop_index(self.last_focused, &enabled);
        // Antes de montar os toggles: é o `Toggle::focus` que clona o handle pro `track_focus`, e o
        // clone lê o `tab_stop` do `FocusRef` no instante em que acontece.
        self.mark_tab_stops(parada);
        // O SENTIDO do bisel no tema corrente, lido uma vez por frame: é ele que escolhe em qual
        // ponta do grupo vertical o filete sobrevive.
        let bevel_dir = crate::toggle::bevel_dir();

        // Os toggles são montados ANTES da raiz porque o separador de cada emenda precisa saber o
        // estado dos DOIS vizinhos, e é do toggle que essa informação sai (`id` e `is_pressed`) —
        // mesma ordem de leitura do `Group::render`, que consulta os filhos antes de consumi-los.
        let mut toggles = Vec::with_capacity(total);
        for i in 0..total {
            toggles.push(self.item_element(i, bevel_dir, cx));
        }
        // Só a variante com separador pergunta: na `Default` não há linha nenhuma pra clarear, e
        // consultar a tabela de hover N vezes por frame pra jogar o resultado fora é trabalho à toa.
        let vizinhos: Vec<(f32, bool)> = if separators(variant) {
            toggles
                .iter()
                .map(|t| {
                    (
                        // A tabela de hover é por `ElementId`, e o `Toggle` grava nela no `on_hover`.
                        crate::button::interaction_amount(t.id().clone()),
                        t.is_pressed(),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };

        let mut raiz = div()
            .id(("empire-toggle-group", self.id))
            .flex()
            // `w-fit`: o grupo mede o conteúdo, não estica.
            .flex_none();
        if orientation == Orientation::Vertical {
            raiz = raiz.flex_col();
        }
        let vao = gap(variant);
        if vao > 0.0 {
            raiz = raiz.gap(px(vao));
        }

        for (i, toggle) in toggles.into_iter().enumerate() {
            if separators(variant) && i > 0 {
                let (realce_a, ligado_a) = vizinhos[i - 1];
                let (realce_b, ligado_b) = vizinhos[i];
                raiz = raiz.child(separador(
                    orientation,
                    SeparatorTone::Input,
                    // O separador reage ao vizinho mais forte de cada lado: basta UM estar sob o
                    // ponteiro (ou ligado) pra a linha clarear. É o que as quatro regras `dark:` do
                    // original dizem, cada uma cobrindo um lado (`:has(+…)` e `…+…`).
                    realce_a.max(realce_b),
                    ligado_a || ligado_b,
                    // Foco: ver "Não reproduzível" no doc do módulo — o coss não tem essa regra pro
                    // grupo de toggles.
                    false,
                ));
            }
            raiz = raiz.child(toggle);
        }

        // O teclado é ouvido pela RAIZ, não pelo item: o evento sobe do elemento focado pelos
        // ancestrais, então um listener só cobre todos (mesmo arranjo do `crate::tabs`).
        raiz = raiz.on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            this.on_key(event, window, cx);
        }));

        // **O `w-fit` que o GPUI não tem**, exatamente pelo motivo que o `crate::group` documenta:
        // sem largura declarada, um `div` de flex ESTICA no eixo transversal do pai, e num grupo
        // vertical o separador (que não declara largura, pra acompanhar o item mais largo)
        // atravessaria a linha inteira. A linha em volta troca o eixo, e com `flex_none` o grupo
        // passa a medir o conteúdo. No horizontal é um no-op.
        div().flex().child(raiz)
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    // A regra de cor do separador é do [`crate::group`] — só o TESTE dela vive aqui, porque é aqui
    // que os números da referência do `toggle-group.tsx` estão escritos.
    use crate::group::cor_do_separador;
    use crate::theme;

    /// As duas variantes, pra nenhum teste esquecer uma.
    const VARIANTES: [ToggleVariant; 2] = [ToggleVariant::Default, ToggleVariant::Outline];

    /// Os dois eixos.
    const EIXOS: [Orientation; 2] = [Orientation::Horizontal, Orientation::Vertical];

    // --- Números da referência -------------------------------------------------------------------

    /// **O vão da variante `Default` é 2px, e a `Outline` não tem vão.**
    ///
    /// `gap-0.5` com `--spacing` de 4px dá 2. Se alguém ler "0.5" como meio pixel, ou copiar o
    /// `gap-2` (8px) do grupo aninhado do [`crate::Group`], é aqui que cai.
    #[test]
    fn o_vao_da_variante_default_e_de_dois_pixels() {
        const SPACING: f32 = 4.0;
        assert_eq!(GAP, 0.5 * SPACING, "gap-0.5 = 0,5 × 4px");
        assert_eq!(gap(ToggleVariant::Default), GAP);
        assert_eq!(
            gap(ToggleVariant::Outline),
            0.0,
            "costurado não tem vão: o que separa é o separador"
        );
    }

    /// **Vão e separador são exclusivos**: cada variante tem UM dos dois, nunca os dois nem nenhum.
    ///
    /// É a forma dos dois ramos do ternário do original, e a varredura é pela lista única — se um dia
    /// aparecer uma terceira variante, ela não passa batido sem responder a esta pergunta.
    #[test]
    fn cada_variante_tem_vao_ou_separador_nunca_os_dois() {
        for v in VARIANTES {
            assert_ne!(
                gap(v) > 0.0,
                separators(v),
                "{v:?}: ou tem vão, ou tem separador"
            );
        }
        assert_eq!(VARIANTES.len(), 2, "a lista única cobre as duas variantes");
    }

    /// **O separador tem 1px**, e é o MESMO valor do [`crate::Group`] — não uma cópia.
    ///
    /// O `w-px`/`h-px` do `Separator` do coss. Este teste é o que garante que reusar
    /// [`crate::group::separador`] entrega a espessura da referência: se o `Group` mudar de ideia
    /// sobre a espessura, o botão segmentado descobre aqui em vez de na tela.
    #[test]
    fn o_separador_tem_um_pixel() {
        assert_eq!(crate::group::SEPARATOR, 1.0, "w-px / h-px");
    }

    /// **O `/N` do Tailwind MULTIPLICA o alfa**, e as duas camadas do separador compõem.
    ///
    /// O original pinta o separador em duas camadas sobrepostas: `bg-input` (branco a 8% no escuro) e
    /// um `::before` em `dark:before:bg-input/32`. O `/32` não é "32% de alfa", é 32% do alfa que
    /// `--input` já tem — 2,56%. Compondo as duas: `1 − (1−0,08)(1−0,0256) ≈ 10,4%`.
    ///
    /// Os três degraus do original, em cima do mesmo `bg-input`:
    ///
    /// | estado do vizinho | `::before` | composto |
    /// |---|---|---|
    /// | repouso | `input/32` = 2,56% | ~10,4% |
    /// | sob o ponteiro | `input/64` = 5,12% | ~12,7% |
    /// | LIGADO | `input` = 8% | ~15,4% |
    ///
    /// Se alguém trocar a multiplicação por substituição, o repouso viraria ~34% e este teste cai
    /// três vezes.
    #[test]
    fn o_alfa_do_separador_compoe_as_duas_camadas() {
        theme::set_theme(theme::ThemeMode::Dark);
        /// `--input` no tema escuro: branco a 8%.
        const INPUT: f32 = 0.08;
        /// Duas camadas da MESMA cor, uma sobre a outra.
        fn composto(antes: f32) -> f32 {
            1.0 - (1.0 - INPUT) * (1.0 - antes)
        }

        let alfa = |realce: f32, ligado: bool| {
            cor_do_separador(SeparatorTone::Input, realce, ligado, false).alpha()
        };

        let casos = [
            ("repouso", alfa(0.0, false), composto(INPUT * 0.32)),
            ("ponteiro", alfa(1.0, false), composto(INPUT * 0.64)),
            ("ligado", alfa(0.0, true), composto(INPUT)),
        ];
        for (nome, nosso, esperado) in casos {
            assert!(
                (nosso - esperado).abs() < 0.005,
                "{nome}: esperado ~{esperado:.4} de alfa, veio {nosso:.4}"
            );
        }

        // E a ordem: cada degrau clareia. Se dois empatarem, um dos estados ficou invisível.
        assert!(alfa(0.0, false) < alfa(1.0, false));
        assert!(alfa(1.0, false) < alfa(0.0, true));

        // No tema CLARO nada disso acontece: as três regras do original são `dark:`.
        theme::set_theme(theme::ThemeMode::Light);
        assert_eq!(
            alfa(0.0, false),
            alfa(1.0, true),
            "no claro o separador não reage a ponteiro nem a ligado"
        );
    }

    // --- Costura ---------------------------------------------------------------------------------

    /// **Só a `Outline` costura e separa.** A `Default` é uma fileira de toggles soltos: cada um com
    /// os quatro cantos e as quatro bordas dele, e 2px de vão entre eles.
    #[test]
    fn a_variante_default_nao_costura_nem_separa() {
        assert!(!separators(ToggleVariant::Default));
        assert!(separators(ToggleVariant::Outline));

        for orientation in EIXOS {
            for i in 0..3 {
                let j = join_for(i, 3, ToggleVariant::Default, orientation);
                assert_eq!(
                    j,
                    Join::NONE,
                    "Default, {orientation:?}, item {i}: nada de costura"
                );
                assert!(!j.is_joined());
            }
        }
    }

    /// **A costura da `Outline` sai da POSIÇÃO, nos dois eixos.** Primeiro aberto no início, último
    /// aberto no fim, meio fechado dos dois lados — e a orientação viaja no `Join`, porque é ela que
    /// decide se `start` é esquerda ou topo (ver [`crate::group::Join`]).
    #[test]
    fn a_costura_da_outline_sai_da_posicao_nos_dois_eixos() {
        for orientation in EIXOS {
            let junta = |i, total| join_for(i, total, ToggleVariant::Outline, orientation);

            // Um item só: solto, mesmo costurando. (Não é `Join::NONE`: o eixo continua declarado, e
            // ele é irrelevante quando não há emenda — com `start`/`end` em `false`, nem
            // `Join::rounded` nem `Join::overlay` olham a orientação.)
            assert!(!junta(0, 1).is_joined(), "{orientation:?}: item único");

            let (primeiro, meio, ultimo) = (junta(0, 3), junta(1, 3), junta(2, 3));
            assert_eq!(
                (primeiro.start, primeiro.end),
                (false, true),
                "{orientation:?}: o primeiro só encosta depois"
            );
            assert_eq!(
                (meio.start, meio.end),
                (true, true),
                "{orientation:?}: o do meio encosta dos dois lados"
            );
            assert_eq!(
                (ultimo.start, ultimo.end),
                (true, false),
                "{orientation:?}: o último só encosta antes"
            );

            for j in [primeiro, meio, ultimo] {
                assert_eq!(
                    j.orientation, orientation,
                    "o eixo tem que viajar no Join: sem ele, `start` viraria esquerda num grupo \
                     vertical"
                );
            }
        }
    }

    // --- O filete do bisel -----------------------------------------------------------------------

    /// **No horizontal o filete aparece em todos os itens.** Ali ele continua a linha do vizinho, e é
    /// pra isso que existe o sangramento de meio pixel na emenda.
    #[test]
    fn no_horizontal_o_filete_aparece_em_todos() {
        for dir in [1.0, -1.0] {
            for i in 0..3 {
                assert!(
                    bevel_visible(i, 3, Orientation::Horizontal, dir),
                    "horizontal, dir {dir}, item {i}"
                );
            }
        }
    }

    /// **No vertical o filete sobrevive só na ponta pra onde o bisel APONTA** — e a ponta sai do
    /// SINAL, não do nome do tema.
    ///
    /// Este é o teste que impede a versão errada mais provável do porte: hardcodar "o último item",
    /// que é o que a primeira das três regras do original diz — e que está certa só no tema claro.
    #[test]
    fn no_vertical_o_filete_fica_na_ponta_do_bisel() {
        // Bisel que DESCE (`0 1px`): a ponta é a de baixo, o último item.
        let desce: Vec<bool> = (0..3)
            .map(|i| bevel_visible(i, 3, Orientation::Vertical, 1.0))
            .collect();
        assert_eq!(desce, vec![false, false, true], "filete embaixo → só o último");

        // Bisel que SOBE (`0 -1px`): a ponta é a de cima, o primeiro item.
        let sobe: Vec<bool> = (0..3)
            .map(|i| bevel_visible(i, 3, Orientation::Vertical, -1.0))
            .collect();
        assert_eq!(sobe, vec![true, false, false], "filete em cima → só o primeiro");

        // Num grupo de um item só ele é primeiro E último: o filete aparece nos dois sentidos.
        assert!(bevel_visible(0, 1, Orientation::Vertical, 1.0));
        assert!(bevel_visible(0, 1, Orientation::Vertical, -1.0));
    }

    /// **E os dois temas caem cada um num lado da regra acima.** É o elo entre a regra (que é sobre o
    /// sinal) e a paleta do [`crate::toggle`] (que é quem tem o sinal): sem este teste, a regra
    /// poderia estar perfeita e ligada ao sinal errado.
    #[test]
    fn a_ponta_do_filete_troca_com_o_tema() {
        theme::set_theme(theme::ThemeMode::Light);
        let claro = crate::toggle::bevel_dir();
        assert!(claro > 0.0, "no claro o bisel desce");
        assert!(
            bevel_visible(2, 3, Orientation::Vertical, claro)
                && !bevel_visible(0, 3, Orientation::Vertical, claro),
            "claro: o filete é o do ÚLTIMO item"
        );

        theme::set_theme(theme::ThemeMode::Dark);
        let escuro = crate::toggle::bevel_dir();
        assert!(escuro < 0.0, "no escuro o bisel sobe");
        assert!(
            bevel_visible(0, 3, Orientation::Vertical, escuro)
                && !bevel_visible(2, 3, Orientation::Vertical, escuro),
            "escuro: o filete é o do PRIMEIRO item"
        );
    }

    // --- Seleção ---------------------------------------------------------------------------------

    /// **A seleção única APAGA os irmãos.** É a diferença que dá nome ao modo, e a que não acontece
    /// de graça.
    #[test]
    fn a_selecao_unica_apaga_os_irmaos() {
        let modo = ToggleGroupMode::Single;

        // Acender o 2 com o 0 aceso deixa SÓ o 2.
        let estado = apply_toggle(&[true, false, false], modo, 2).expect("índice válido");
        assert_eq!(estado, vec![false, false, true]);
        assert_eq!(selected_indices(&estado), vec![2], "um só, e é o novo");

        // Do zero, acender um acende só ele.
        let estado = apply_toggle(&[false, false, false], modo, 1).expect("índice válido");
        assert_eq!(selected_indices(&estado), vec![1]);

        // Tocar no que já está aceso APAGA, e o grupo fica vazio (o comportamento do base-ui).
        let estado = apply_toggle(&[false, true, false], modo, 1).expect("índice válido");
        assert!(
            selected_indices(&estado).is_empty(),
            "clicar no aceso desliga: um grupo que não deixa desmarcar seria um radio"
        );
    }

    /// **A seleção múltipla acumula** — e não apaga ninguém, nunca.
    #[test]
    fn a_selecao_multipla_acumula() {
        let modo = ToggleGroupMode::Multiple;

        let estado = apply_toggle(&[true, false, false], modo, 2).expect("índice válido");
        assert_eq!(
            selected_indices(&estado),
            vec![0, 2],
            "o 0 continua aceso: é isto que separa múltipla de única"
        );

        // E desligar um não mexe nos outros.
        let estado = apply_toggle(&estado, modo, 0).expect("índice válido");
        assert_eq!(selected_indices(&estado), vec![2]);

        // Todos acesos é estado válido aqui, e inalcançável no modo `Single`.
        let mut estado = vec![false; 3];
        for i in 0..3 {
            estado = apply_toggle(&estado, modo, i).expect("índice válido");
        }
        assert_eq!(selected_indices(&estado), vec![0, 1, 2]);
    }

    /// Índice fora da lista não muda nada — e é `None`, não um estado igual, pra quem chama poder
    /// distinguir "não existe" de "não mudou".
    #[test]
    fn indice_fora_da_lista_nao_muda_nada() {
        for modo in [ToggleGroupMode::Single, ToggleGroupMode::Multiple] {
            assert!(apply_toggle(&[false, false], modo, 2).is_none(), "{modo:?}");
            assert!(apply_toggle(&[], modo, 0).is_none(), "{modo:?}: lista vazia");
        }
    }

    /// O estado inicial respeita o modo: pedir dois ligados num grupo de seleção única é dado
    /// inválido, e só o primeiro vale.
    #[test]
    fn o_estado_inicial_respeita_o_modo() {
        assert_eq!(
            pressed_bits(3, &[1, 2], ToggleGroupMode::Single),
            vec![false, true, false],
            "Single: só o primeiro índice pedido"
        );
        assert_eq!(
            pressed_bits(3, &[1, 2], ToggleGroupMode::Multiple),
            vec![false, true, true],
            "Multiple: os dois"
        );
        // Índice fora da lista é ignorado, e não é `panic` nem cresce o vetor.
        assert_eq!(
            pressed_bits(2, &[5], ToggleGroupMode::Multiple),
            vec![false, false]
        );
        assert!(pressed_bits(0, &[0], ToggleGroupMode::Multiple).is_empty());
    }

    /// O default do componente é o do base-ui: seleção **única**, variante e tamanho `Default`,
    /// horizontal.
    #[test]
    fn defaults_batem_com_o_primitivo() {
        assert_eq!(
            ToggleGroupMode::default(),
            ToggleGroupMode::Single,
            "o `toggleMultiple` do base-ui é `false` por default"
        );
        assert_eq!(ToggleVariant::default(), ToggleVariant::Default);
        assert_eq!(ToggleSize::default(), ToggleSize::Default);
        assert_eq!(Orientation::default(), Orientation::Horizontal);
    }

    /// Um item é rótulo, ícone, ou os dois — e nunca nenhum dos dois.
    #[test]
    fn um_item_e_rotulo_icone_ou_os_dois() {
        let rotulo = ToggleGroupItem::new("Negrito");
        assert_eq!(rotulo.label(), Some(&SharedString::from("Negrito")));
        assert!(rotulo.icon_path().is_none());

        let icone = ToggleGroupItem::icon("b.svg");
        assert!(icone.label().is_none());
        assert_eq!(icone.icon_path(), Some(&SharedString::from("b.svg")));

        let dois = ToggleGroupItem::new("Negrito").with_icon("b.svg");
        assert!(dois.label().is_some() && dois.icon_path().is_some());

        assert!(!rotulo.is_disabled());
        assert!(ToggleGroupItem::new("x").disabled(true).is_disabled());
    }

    // --- Teclado: o roving tabindex --------------------------------------------------------------

    /// Três itens com o do meio travado — a lista que os testes de navegação usam.
    ///
    /// `pub(super)` porque o módulo de testes de janela monta a MESMA lista: duas listas com o mesmo
    /// item travado no meio divergiriam, e é o índice 1 que os dois lados assumem.
    pub(super) fn itens_com_o_meio_travado() -> Vec<ToggleGroupItem> {
        vec![
            ToggleGroupItem::new("Um"),
            ToggleGroupItem::new("Dois").disabled(true),
            ToggleGroupItem::new("Tres"),
        ]
    }

    /// **Só o item travado sai da navegação.** Não há `disabled` de grupo aqui — é a diferença de
    /// contrato que impediu compartilhar esta função com o [`crate::radio_group`].
    #[test]
    fn o_item_travado_sai_da_navegacao() {
        assert_eq!(
            enabled_bits(&itens_com_o_meio_travado()),
            vec![true, false, true]
        );
        assert!(enabled_bits(&[]).is_empty(), "grupo vazio");
        assert_eq!(
            enabled_bits(&[ToggleGroupItem::new("x")]),
            vec![true],
            "sem `disabled`, todo item navega"
        );
    }

    /// **Só as setas do EIXO do grupo andam** — num grupo horizontal o `↓` não é tratado, num vertical
    /// o `→` não é. É o filtro do composite do base-ui (ver [`arrow_dir`]), e a diferença em relação ao
    /// [`crate::radio_group`], onde as quatro setas fazem a mesma coisa.
    #[test]
    fn so_as_setas_do_eixo_do_grupo_andam() {
        let h = Orientation::Horizontal;
        let v = Orientation::Vertical;

        assert_eq!(arrow_dir("right", h), Some(1), "horizontal: → avança");
        assert_eq!(arrow_dir("left", h), Some(-1), "horizontal: ← volta");
        assert_eq!(arrow_dir("down", h), None, "horizontal: ↓ não é do eixo");
        assert_eq!(arrow_dir("up", h), None, "horizontal: ↑ não é do eixo");

        assert_eq!(arrow_dir("down", v), Some(1), "vertical: ↓ avança");
        assert_eq!(arrow_dir("up", v), Some(-1), "vertical: ↑ volta");
        assert_eq!(arrow_dir("right", v), None, "vertical: → não é do eixo");
        assert_eq!(arrow_dir("left", v), None, "vertical: ← não é do eixo");

        // E cada eixo trata exatamente DUAS teclas: o que sobra não é seta.
        for eixo in EIXOS {
            let tratadas = ["right", "left", "down", "up"]
                .into_iter()
                .filter(|k| arrow_dir(k, eixo).is_some())
                .count();
            assert_eq!(tratadas, 2, "{eixo:?}: duas setas, uma pra cada sentido");
            assert_eq!(arrow_dir("home", eixo), None, "Home não é seta");
            assert_eq!(arrow_dir("enter", eixo), None, "Enter não é seta");
        }
    }

    /// **A seta pula o item travado, dá a volta na ponta, e `Home`/`End` vão pras pontas
    /// habilitadas.** As três coisas são as funções do [`crate::radio_group`] reusadas — este teste é o
    /// segundo leitor delas, e o que avisa se um refactor de lá mudar a navegação por baixo daqui.
    #[test]
    fn o_alvo_do_foco_pula_travado_e_da_a_volta() {
        let enabled = enabled_bits(&itens_com_o_meio_travado());
        let alvo = |key, focado| focus_target(key, Orientation::Horizontal, focado, &enabled);

        assert_eq!(alvo("right", Some(0)), Some(2), "→ pula o travado");
        assert_eq!(alvo("right", Some(2)), Some(0), "→ dá a volta");
        assert_eq!(alvo("left", Some(2)), Some(0), "← pula o travado");
        assert_eq!(alvo("left", Some(0)), Some(2), "← dá a volta");
        assert_eq!(alvo("home", Some(2)), Some(0), "Home");
        assert_eq!(alvo("end", Some(0)), Some(2), "End");

        // Tecla que não move foco nenhum.
        assert_eq!(alvo("enter", Some(0)), None, "Enter não move foco");
        assert_eq!(alvo("space", Some(0)), None, "Space não move foco");
        assert_eq!(alvo("down", Some(0)), None, "o eixo cruzado não move foco");
        assert_eq!(alvo("a", Some(0)), None);

        // O grupo VERTICAL faz o mesmo com as setas do eixo dele, e ignora as do horizontal.
        let alvo_v = |key, focado| focus_target(key, Orientation::Vertical, focado, &enabled);
        assert_eq!(alvo_v("down", Some(0)), Some(2), "↓ pula o travado");
        assert_eq!(alvo_v("home", Some(2)), Some(0), "Home vale nos dois eixos");
        assert_eq!(alvo_v("right", Some(0)), None, "o eixo cruzado do vertical");

        // ⚠️ **Com o item do meio travado a DIREÇÃO não é observável**: só sobram dois itens, e
        // avançar e voltar caem no mesmo. Então o sentido é afirmado numa lista sem travas, onde
        // inverter um dos dois sentidos de [`arrow_dir`] muda o resultado.
        let todos = [true, true, true];
        let meio = Some(1);
        assert_eq!(
            focus_target("right", Orientation::Horizontal, meio, &todos),
            Some(2),
            "→ avança"
        );
        assert_eq!(
            focus_target("left", Orientation::Horizontal, meio, &todos),
            Some(0),
            "← volta"
        );
        assert_eq!(
            focus_target("down", Orientation::Vertical, meio, &todos),
            Some(2),
            "↓ avança"
        );
        assert_eq!(
            focus_target("up", Orientation::Vertical, meio, &todos),
            Some(0),
            "↑ volta"
        );

        // Grupo sem ninguém habilitado: não há pra onde ir, por tecla nenhuma.
        let travado = vec![false, false];
        for key in ["right", "left", "home", "end"] {
            assert_eq!(
                focus_target(key, Orientation::Horizontal, Some(0), &travado),
                None,
                "{key} num grupo todo travado"
            );
        }
    }

    /// **A parada de `Tab` é o último item que teve foco, senão o primeiro habilitado** — e é `None`
    /// num grupo sem nenhum item habilitado, que é o que tira o grupo inteiro da ordem de tabulação.
    ///
    /// Diferente do [`crate::radio_group`], o argumento não é "o selecionado": em
    /// [`ToggleGroupMode::Multiple`] não existe um. Ver "Teclado" no doc do módulo.
    #[test]
    fn a_parada_de_tab_e_o_ultimo_focado_senao_o_primeiro_habilitado() {
        let parada =
            |ultimo, enabled: &[bool]| crate::radio_group::tab_stop_index(ultimo, enabled);
        let enabled = enabled_bits(&itens_com_o_meio_travado());

        assert_eq!(parada(None, &enabled), Some(0), "ninguém focou ainda");
        assert_eq!(parada(Some(2), &enabled), Some(2), "o último focado");
        assert_eq!(
            parada(Some(1), &enabled),
            Some(0),
            "o último focado ficou travado: cai no primeiro habilitado"
        );
        assert_eq!(
            parada(Some(0), &[false, false, false]),
            None,
            "grupo todo travado sai do Tab"
        );
        assert_eq!(parada(None, &[]), None, "grupo vazio");
    }

    /// **Os ids dos itens carregam o id da ENTIDADE.** A tabela de hover do [`crate::button`] é
    /// global por [`ElementId`]: sem o id da entidade na chave, passar o mouse no item 0 de um grupo
    /// clarearia o separador do item 0 de TODOS os grupos da tela.
    #[test]
    fn os_ids_dos_itens_nao_colidem_entre_grupos() {
        let a = item_ids(2, 7);
        let b = item_ids(2, 8);
        assert_eq!(a.len(), 2);
        assert_ne!(a[0], a[1], "dois itens do mesmo grupo têm ids diferentes");
        assert_ne!(a[0], b[0], "o item 0 de dois grupos tem ids diferentes");
        assert!(item_ids(0, 1).is_empty());
    }
}

/// Testes que precisam de uma **janela de verdade**: os de cima cobrem geometria e seleção
/// (aritmética pura), mas nada ali prova que o clique num item chega ao [`ToggleGroup::choose`]. Esse
/// caminho passa pelo `on_toggle` do [`crate::Toggle`], que é o ponto de solda entre os dois
/// componentes — e é solda que só se testa passando um clique de verdade. Mesmo arranjo do
/// [`crate::switch`] e do [`crate::tabs`].
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{point, AppContext as _, Modifiers, Pixels, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Harness {
        grupo: gpui::Entity<ToggleGroup>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().flex().flex_col().child(self.grupo.clone())
        }
    }

    /// Abre a janela com três itens de rótulo, o do meio começando LIGADO, e devolve a entidade, os
    /// eventos capturados e o contexto visual.
    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
        mode: ToggleGroupMode,
    ) -> (
        gpui::Entity<ToggleGroup>,
        Rc<RefCell<Vec<(usize, Vec<usize>)>>>,
        VisualTestContext,
    ) {
        abrir_itens(
            cx,
            vec![
                ToggleGroupItem::new("Um"),
                ToggleGroupItem::new("Dois"),
                ToggleGroupItem::new("Tres"),
            ],
            mode,
        )
    }

    /// O mesmo, com a lista de itens escolhida por quem chama — é o que os testes de teclado usam pra
    /// pôr um item travado no meio.
    #[allow(clippy::type_complexity)]
    fn abrir_itens(
        cx: &mut TestAppContext,
        itens: Vec<ToggleGroupItem>,
        mode: ToggleGroupMode,
    ) -> (
        gpui::Entity<ToggleGroup>,
        Rc<RefCell<Vec<(usize, Vec<usize>)>>>,
        VisualTestContext,
    ) {
        let eventos: Rc<RefCell<Vec<(usize, Vec<usize>)>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_window, cx| {
            let grupo = cx.new(|cx| {
                ToggleGroup::new(itens, cx)
                    .mode(mode)
                    .variant(ToggleVariant::Outline)
                    .with_selected(&[1])
            });
            cx.subscribe(&grupo, move |_this, _g, ev: &ToggleGroupEvent, _cx| match ev {
                ToggleGroupEvent::Change { item, selected } => {
                    capturados.borrow_mut().push((*item, selected.clone()));
                }
            })
            .detach();
            Harness { grupo }
        });

        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let grupo = vcx.read(|cx| harness.read(cx).grupo.clone());
        (grupo, eventos, vcx)
    }

    /// Um ponto dentro do PRIMEIRO item. O grupo encosta no canto da janela e todo item tem no
    /// mínimo 28px de lado (o `min-w` do menor tamanho), então (10, 16) cai no item 0 em qualquer
    /// tamanho ou rótulo.
    fn no_primeiro_item() -> gpui::Point<Pixels> {
        point(px(10.0), px(16.0))
    }

    /// **O clique no item chega ao grupo, e num grupo `Single` ele apaga o irmão aceso.** É o
    /// contrato inteiro do componente num teste: solda, regra de seleção e evento.
    #[gpui::test]
    fn o_clique_apaga_o_irmao_no_modo_unico(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, ToggleGroupMode::Single);
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), vec![1], "começa no 1"));

        vcx.simulate_click(no_primeiro_item(), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| {
            assert_eq!(
                grupo.read(cx).selected(),
                vec![0],
                "o 0 acendeu e o 1 apagou — sem isso o modo não é único"
            );
        });
        assert_eq!(
            *eventos.borrow(),
            vec![(0, vec![0])],
            "o evento leva o item tocado E o resultado"
        );
    }

    /// **No modo múltiplo o mesmo clique acumula.** O par deste teste com o de cima é o que prova que
    /// o modo é levado a sério: a mesma interação, dois resultados.
    #[gpui::test]
    fn o_clique_acumula_no_modo_multiplo(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, ToggleGroupMode::Multiple);

        vcx.simulate_click(no_primeiro_item(), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| {
            assert_eq!(
                grupo.read(cx).selected(),
                vec![0, 1],
                "o 1 continuou aceso"
            );
        });
        assert_eq!(*eventos.borrow(), vec![(0, vec![0, 1])]);
    }

    /// **Um item desabilitado não responde ao clique e não emite.** O `opacity-64` não é o que
    /// impede — é o guarda do [`crate::Toggle`], que só pendura o `on_click` quando está habilitado.
    #[gpui::test]
    fn item_desabilitado_ignora_o_clique(cx: &mut TestAppContext) {
        let eventos: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_window, cx| {
            let grupo = cx.new(|cx| {
                ToggleGroup::new(
                    vec![ToggleGroupItem::new("Um").disabled(true)],
                    cx,
                )
                .variant(ToggleVariant::Outline)
            });
            cx.subscribe(&grupo, move |_this, _g, _ev: &ToggleGroupEvent, _cx| {
                *capturados.borrow_mut() += 1;
            })
            .detach();
            Harness { grupo }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let grupo = vcx.read(|cx| harness.read(cx).grupo.clone());

        vcx.simulate_click(no_primeiro_item(), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert!(grupo.read(cx).selected().is_empty(), "não acendeu"));
        assert_eq!(*eventos.borrow(), 0, "não emitiu");
    }

    /// **Espaço e enter alternam o item FOCADO.** É o que faz o anel de foco significar algo: sem
    /// tecla que alterne, um item alcançável por `Tab` seria um beco sem saída (ver "Superset
    /// consciente" no doc do módulo).
    ///
    /// O foco chega pelo clique — o `track_focus` do [`crate::Toggle`] faz o mouse-down focar —, que é
    /// o mesmo estado a que um usuário de teclado chega por `Tab`, sem depender da ordem de tabulação
    /// do harness. Mesmo arranjo do teste irmão no [`crate::switch`].
    #[gpui::test]
    fn espaco_e_enter_alternam_o_item_focado(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, ToggleGroupMode::Single);

        vcx.simulate_click(no_primeiro_item(), Modifiers::default()); // acende o 0 e leva o foco
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), vec![0]));

        vcx.simulate_keystrokes("space");
        vcx.run_until_parked();
        vcx.read(|cx| {
            assert!(
                grupo.read(cx).selected().is_empty(),
                "espaço apagou o item focado"
            );
        });

        vcx.simulate_keystrokes("enter");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), vec![0], "enter acendeu de novo"));

        assert_eq!(
            eventos.borrow().len(),
            3,
            "as três interações emitiram: clique, espaço, enter"
        );
    }

    /// **O grupo inteiro é UMA parada de `Tab`.**
    ///
    /// Medido do jeito que importa: pelo [`gpui::Window::focus_next`], que é o que o `Tab` do
    /// `gpui_component::Root` chama. Com três itens habilitados, o segundo `focus_next` não tem pra
    /// onde ir dentro do grupo — e é isso que este teste proíbe.
    #[gpui::test]
    fn o_grupo_inteiro_e_uma_parada_de_tab(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, ToggleGroupMode::Single);

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(0),
                "o Tab entra no grupo pelo primeiro item habilitado"
            );
        });

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(0),
                "o Tab NÃO anda item a item dentro do grupo"
            );
        });

        // E entrar por `Tab` não mexe na seleção.
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), vec![1], "o Tab não seleciona"));
        assert!(eventos.borrow().is_empty(), "e não emite");
    }

    /// **As setas movem o FOCO e não a seleção** — a decisão que separa este composite de um grupo de
    /// rádios (ver "Teclado" no doc do módulo). Se alguém "unificar" os dois chamando `choose` na seta,
    /// é aqui que cai.
    #[gpui::test]
    fn as_setas_movem_o_foco_e_nao_a_selecao(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, ToggleGroupMode::Single);

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();

        vcx.simulate_keystrokes("right");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            let g = grupo.read(cx);
            assert_eq!(g.focused_item(window), Some(1), "→ moveu o foco");
            assert_eq!(g.selected(), vec![1], "e NÃO mexeu na seleção");
        });

        // Anda até dar a volta, e a seleção continua a mesma o caminho inteiro.
        vcx.simulate_keystrokes("right right");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            let g = grupo.read(cx);
            assert_eq!(g.focused_item(window), Some(0), "→ deu a volta");
            assert_eq!(g.selected(), vec![1]);
        });

        vcx.simulate_keystrokes("left");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(2), "← deu a volta");
        });

        vcx.simulate_keystrokes("home");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(0), "Home");
        });
        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(2), "End");
        });

        // **Nenhuma** das seis teclas emitiu nada: a seta não é interação de seleção.
        assert!(
            eventos.borrow().is_empty(),
            "seta e Home/End não emitem Change"
        );

        // E o `Enter` no item onde a seta parou alterna ESSE item — o par que prova que o foco andou
        // de verdade, e não só o campo interno.
        vcx.simulate_keystrokes("enter");
        vcx.run_until_parked();
        vcx.read(|cx| {
            assert_eq!(
                grupo.read(cx).selected(),
                vec![2],
                "o Enter alternou o item onde o End deixou o foco"
            );
        });
    }

    /// **O eixo cruzado não move o foco**: num grupo horizontal o `↓`/`↑` não é do eixo, e o primitivo
    /// não os trata. Ver [`arrow_dir`].
    #[gpui::test]
    fn o_eixo_cruzado_nao_move_o_foco(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir(cx, ToggleGroupMode::Single);

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.simulate_keystrokes("down down up");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(0),
                "num grupo horizontal o foco não anda com ↓/↑"
            );
        });

        // E o mesmo grupo responde ao eixo dele — senão este teste passaria por um grupo surdo.
        vcx.simulate_keystrokes("right");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(1), "→ move");
        });
    }

    /// **A seta pula o item travado**, e o travado nunca recebe o foco.
    #[gpui::test]
    fn a_seta_pula_o_item_travado(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir_itens(
            cx,
            super::tests::itens_com_o_meio_travado(),
            ToggleGroupMode::Multiple,
        );

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(0), "entrada");
        });

        vcx.simulate_keystrokes("right");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(2),
                "→ pulou o item 1, que está travado"
            );
        });

        vcx.simulate_keystrokes("left");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(0),
                "← pulou o travado no outro sentido"
            );
        });
    }

    /// **A parada de `Tab` acompanha o último item que teve foco.** É o `highlightedIndex` do
    /// composite: sair do grupo e voltar por `Tab` devolve o usuário ao item onde ele estava, e não ao
    /// primeiro.
    #[gpui::test]
    fn a_parada_de_tab_segue_o_ultimo_focado(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir(cx, ToggleGroupMode::Single);

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(2), "o foco está no 2");
        });

        // Sai do grupo (é o que um `Tab` no último item da janela faria) e volta.
        vcx.update(|window, _cx| window.blur());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), None, "o foco saiu");
        });

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(2),
                "a parada de Tab é o último focado, não o primeiro item"
            );
        });
    }

    /// **O clique também move a parada de `Tab`.** O foco por clique não passa pelo `on_key`, então
    /// quem o registra é a sincronização do `render` — e sem ela o `Tab` levaria o usuário de volta pro
    /// primeiro item depois de ele ter clicado em outro.
    #[gpui::test]
    fn o_clique_move_a_parada_de_tab(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir(cx, ToggleGroupMode::Multiple);

        // O item 0 é o que o `Tab` alcançaria agora.
        vcx.simulate_click(no_primeiro_item(), Modifiers::default());
        vcx.run_until_parked();
        // Move o foco pro 2 pelo teclado e clica FORA de qualquer item pra soltar o foco.
        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        vcx.update(|window, _cx| window.blur());
        vcx.run_until_parked();

        // Agora um clique no item 0 tem que trazer a parada de volta pra ele.
        vcx.simulate_click(no_primeiro_item(), Modifiers::default());
        vcx.run_until_parked();
        vcx.update(|window, _cx| window.blur());
        vcx.run_until_parked();
        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(0),
                "o clique registrou o item como o último focado"
            );
        });
    }

    /// **Um grupo com todos os itens travados sai da ordem de tabulação.** Uma parada de `Tab` num beco
    /// sem saída é pior que nenhuma.
    ///
    /// ⚠️ Este teste é um **guarda**, e não a prova do [`crate::radio_group::tab_stop_index`]: medido
    /// por mutação, ele sobrevive a um [`enabled_bits`] que devolva `true` pra todo item, porque aí a
    /// parada cairia num handle que o [`crate::Toggle`] nunca rastreia (ver "um detalhe medido" no doc
    /// do módulo) e o `focus_next` continuaria não achando nada. Quem prova que a máscara zerada
    /// devolve `None` é o teste puro
    /// [`super::tests::a_parada_de_tab_e_o_ultimo_focado_senao_o_primeiro_habilitado`]. As duas camadas
    /// existem de propósito — este teste é o que pega a hora em que uma delas cair.
    #[gpui::test]
    fn grupo_todo_travado_sai_do_tab(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir_itens(
            cx,
            vec![
                ToggleGroupItem::new("Um").disabled(true),
                ToggleGroupItem::new("Dois").disabled(true),
            ],
            ToggleGroupMode::Multiple,
        );

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                None,
                "nenhum handle do grupo recebeu o foco"
            );
        });
    }

    /// **`ToggleGroup::focus` leva o teclado pra entrada do grupo** sem quem chama saber qual dos N
    /// handles é ela.
    #[gpui::test]
    fn focus_leva_pra_parada_do_grupo(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir_itens(
            cx,
            super::tests::itens_com_o_meio_travado(),
            ToggleGroupMode::Multiple,
        );

        vcx.update(|window, cx| grupo.read(cx).focus(window));
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(0), "o primeiro habilitado");
        });

        // Depois de o foco andar, é pro último focado que ela leva.
        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        vcx.update(|window, _cx| window.blur());
        vcx.run_until_parked();
        vcx.update(|window, cx| grupo.read(cx).focus(window));
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(2), "o último focado");
        });
    }

    /// A janela do teste de medição: dois handles marcados como parada de `Tab`, e o `track_focus`
    /// pendurado em **um** só.
    struct DoisHandles {
        rastreado: FocusHandle,
        solto: FocusHandle,
    }

    impl Render for DoisHandles {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            // O `tab_stop` grava no `FocusRef` compartilhado, então marcar um clone marca o handle —
            // é exatamente o que o `ToggleGroup::mark_tab_stops` faz. O deste lado é descartado na
            // hora, e a marca fica.
            let _ = self.solto.clone().tab_stop(true);
            div()
                .id("rastreado")
                .track_focus(&self.rastreado.clone().tab_stop(true))
                .child("rastreado")
        }
    }

    /// **MEDIDO: marcar `tab_stop` num handle sem `track_focus` não faz nada.**
    ///
    /// O [`crate::Toggle`] só pendura `track_focus` quando o item está habilitado, então o handle de um
    /// item travado nunca entra na tabela de paradas do frame — e o `tab_stop` dele é ignorado, porque
    /// a tabela só é alimentada na pintura, pelo `track_focus`. Este teste é a medição que sustenta
    /// essa afirmação no doc do módulo; deduzir do código do GPUI não bastava.
    #[gpui::test]
    fn marcar_tab_stop_sem_track_focus_nao_entra_na_ordem(cx: &mut TestAppContext) {
        let window = cx.add_window(|_window, cx| DoisHandles {
            rastreado: cx.focus_handle(),
            solto: cx.focus_handle(),
        });
        let view = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        for volta in 1..=2 {
            vcx.update(|window, _cx| window.focus_next());
            vcx.run_until_parked();
            vcx.update(|window, cx| {
                let v = view.read(cx);
                assert!(
                    v.rastreado.is_focused(window),
                    "volta {volta}: o Tab só alcança o handle com track_focus"
                );
                assert!(
                    !v.solto.is_focused(window),
                    "volta {volta}: o handle marcado sem track_focus é invisível pro Tab"
                );
            });
        }
    }

    /// **`set_selected` sincroniza sem emitir.** Um `set_` que emitisse fecharia um laço com quem
    /// assina: o assinante grava, o grava emite, o assinante grava…
    #[gpui::test]
    fn set_selected_nao_emite(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, ToggleGroupMode::Single);

        vcx.update(|_window, cx| {
            grupo.update(cx, |g, cx| g.set_selected(&[2], cx));
        });
        vcx.run_until_parked();

        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), vec![2]));
        assert!(eventos.borrow().is_empty(), "sincronizar de fora não emite");
    }
}

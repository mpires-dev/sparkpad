//! `RadioGroup` — a escolha **1-de-N** em círculos, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/radio-group.tsx`
//!
//! É o irmão do [`crate::checkbox::Checkbox`]: mesma caixa de 16px, mesma borda `--input`, mesmo anel
//! de foco, mesma atenuação quando desabilitado, mesmo rótulo clicável. Muda **a forma** (círculo em
//! vez de quadrado de 4px de raio), **o indicador** (um ponto de 6px em vez de um `check`) e — a parte
//! que não é cosmética — **a exclusividade**: num grupo de rádios só um item pode estar marcado, e é o
//! grupo que garante isso.
//!
//! # Anatomia
//!
//! ```text
//!     ╭────╮                  ╭────╮
//!     │    │  Rótulo          │ ●  │  Rótulo
//!     ╰────╯                  ╰────╯
//!      16px                    16px
//!   desmarcado                marcado
//!   borda --input        --primary cobrindo a borda
//!   + bisel de 1px       + ponto de 6px em --primary-foreground
//!   + shadow-xs/5        sem sombra, sem bisel
//! ```
//!
//! A caixa é `size-4.5` com `sm:size-4` derrubando pra **16px** (o `sm:` do Tailwind sempre vale numa
//! janela de desktop), e o ponto é `before:size-2` com `sm:before:size-1.5` → **6px**. O grupo empilha
//! em coluna com `gap-3` = **12px**.
//!
//! # O que veio do [`crate::checkbox`], e por que
//!
//! As duas referências do coss (`checkbox.tsx` e `radio-group.tsx`) declaram a **mesma** lista de
//! utilitários de cor e de medida, caractere por caractere — só o raio e o indicador diferem. Então
//! este módulo **não tem paleta própria**: ele lê a [`crate::checkbox::palette`] e as constantes
//! [`crate::checkbox::BOX`], [`BORDER`], [`RING_WIDTH`], [`RING_OFFSET`], [`DISABLED_OPACITY`],
//! [`GAP`], [`LABEL_SIZE`] e [`LABEL_LINE_HEIGHT`], mais o par [`ease`]/[`progress`] e a duração
//! [`TRANSITION`]. Uma segunda tabela com os mesmos quinze valores divergiria no primeiro refactor —
//! a promoção de visibilidade lá é o que impede isso.
//!
//! O que é **daqui** é só o que realmente difere: [`DOT`], [`GROUP_GAP`], o [`overlay`] circular
//! (`rounded_full` em vez de raio fixo) e o [`indicator`].
//!
//! # Por que isto NÃO é um [`crate::Group`]
//!
//! **Decisão declarada:** o `RadioGroup` não implementa [`crate::group::GroupChild`] nem se apoia no
//! [`crate::Group`], e não é omissão. O `Group` deste crate resolve **costura de superfícies**: ele
//! tira raio e borda na emenda entre dois filhos e põe um separador de 1px no lugar, pra que
//! botão+campo+botão leiam como UMA peça. O `radio-group.tsx` do coss é `flex flex-col gap-3` — filhos
//! **soltos**, com 12px de respiro, cada um com o seu círculo e o seu contorno. Não existe emenda pra
//! costurar, e um separador entre duas opções de rádio seria ruído.
//!
//! É a mesma leitura que o [`crate::toggle_group`] fez ao ser construído *ao lado* do `Group` e não
//! em cima dele, e por três das mesmas razões: (1) o `Group` é [`gpui::RenderOnce`] e não pode ser
//! dono de estado, e exclusividade **exige** um dono só; (2) o `Group` não tem vão entre filhos (o
//! `gap-2` dele é pra grupo ANINHADO); (3) o `Join` fala de emendas, e aqui não há nenhuma. A
//! diferença em relação ao `toggle_group` é que lá pelo menos a variante `Outline` era costurada — aqui
//! nenhuma é.
//!
//! # Teclado: roving tabindex de verdade
//!
//! Um grupo de rádios não é uma fileira de checkboxes. No navegador ele tem **UMA** parada de `Tab`
//! (a opção marcada, ou a primeira habilitada se nenhuma está) e as **setas** movem a seleção junto com
//! o foco. É o que este módulo faz, e não uma aproximação: o GPUI 0.2.2 tem
//! [`gpui::FocusHandle::tab_stop`], e a ordem de `Tab` do [`gpui::Window::focus_next`] só enxerga
//! handles com essa marca. Então a cada frame **exatamente um** dos handles do grupo é marcado como
//! parada (ver [`tab_stop_index`]) e os outros são desmarcados — o roving tabindex do HTML, no
//! mecanismo do GPUI.
//!
//! Este módulo foi o **primeiro** a usar o `tab_stop` nesta base, e por um tempo o
//! [`crate::toggle_group`] declarava o roving tabindex como "não reproduzível" — uma declaração
//! escrita antes de o mecanismo ser usado aqui. Hoje o `toggle_group` também o reproduz, e **reusa**
//! o [`step_index`], o [`edge_index`] e o [`tab_stop_index`] daqui (é por isso que os três são
//! `pub(crate)`): a geometria do movimento é a mesma. O que NÃO é compartilhado é o EFEITO da seta —
//! lá ela move só o foco, aqui ela move a seleção junto, que é a ativação automática do rádio nativo.
//!
//! As teclas, todas com o foco dentro do grupo:
//!
//! | tecla | efeito |
//! |---|---|
//! | `↓` / `→` | próxima opção habilitada, dando a volta |
//! | `↑` / `←` | anterior habilitada, dando a volta |
//! | `Home` / `End` | primeira / última habilitada |
//! | `Space` | marca a opção **focada** |
//!
//! # Contrato
//!
//! É um `Entity` (view) que mantém o próprio estado e **emite** [`RadioGroupEvent::Change`] com o
//! índice recém-marcado. Quem usa só assina o evento — mesmo padrão desacoplado do
//! [`crate::Checkbox`], do [`crate::Switch`] e do [`crate::ToggleGroup`].
//!
//! ```ignore
//! let grupo = cx.new(|cx| {
//!     RadioGroup::new(
//!         vec![
//!             RadioGroupItem::new("Comedy"),
//!             RadioGroupItem::new("Drama"),
//!             RadioGroupItem::new("Horror").disabled(true),
//!         ],
//!         cx,
//!     )
//!     .with_selected(Some(0))
//! });
//! cx.subscribe(&grupo, |_this, _g, e: &RadioGroupEvent, _cx| println!("{e:?}")).detach();
//! ```
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`not-dark:bg-clip-padding`**: o GPUI pinta o fundo na border box, então no tema claro o
//!   `--input` da borda (preto a 10%) cai sobre o branco do fundo em vez de sobre a superfície de
//!   trás. Sobre uma página branca — o `--background` do coss no claro — o pixel é o MESMO; a
//!   diferença só apareceria com o rádio sobre uma superfície colorida. É a mesma omissão declarada no
//!   [`crate::checkbox`] e no [`crate::slider`].
//! - **`before:shadow-[0_±1px_…]` (o bisel) não pode ser [`gpui::BoxShadow`]**: o `paint_shadows` do
//!   GPUI insere a sombra como uma forma **completa**, sem recortar a área do próprio elemento (o CSS
//!   recorta), e num overlay transparente isso vira uma lavagem de cor sobre o círculo inteiro. Então o
//!   filete é o que ele é: **borda de 1px num único lado** de um overlay que cobre a border box, com o
//!   lado saindo do SINAL do deslocamento da referência (ver [`bevel_on_bottom`]). Mesma decisão do
//!   [`crate::checkbox`], do [`crate::card`] e do [`crate::input`].
//! - **`focus-visible:ring-2 ring-offset-1`**: `ring` não existe no GPUI e não dá pra fingir com
//!   sombra (o `spread_radius` dilata os limites mas mantém o raio). São dois overlays absolutos
//!   concêntricos e adjacentes — ver [`ring_overlays`].
//! - **`focus-visible`**: o GPUI não distingue foco de teclado de foco de ponteiro. A modalidade vem do
//!   [`crate::focus_ring`], que é estado da SESSÃO.
//! - **`transition-shadow`**: não há `transition` no GPUI. O desvanecimento da sombra e do anel vem de
//!   tempo decorrido + `request_animation_frame`, com o [`crate::checkbox::ease`] e a curva
//!   [`EASE_TAILWIND`].
//!
//! **Resolvido em número**
//!
//! - `size-4.5 sm:size-4` → **16px** ([`crate::checkbox::BOX`]); `before:size-2 sm:before:size-1.5` →
//!   **6px** ([`DOT`]); `gap-3` → **12px** ([`GROUP_GAP`]), com o `--spacing` do Tailwind valendo 4px.
//! - `rounded-full` → `rounded_full()`, que no GPUI é `px(9999)` aparado em `min(w,h)/2` na hora de
//!   pintar — ou seja, círculo exato em qualquer um dos overlays dilatados. Mesma leitura que o
//!   [`crate::avatar`] documenta.
//! - `data-disabled:opacity-64` → **0,64** aplicado na FILEIRA (círculo + ponto + rótulo), como o
//!   original faz com o conjunto em vez de cor por cor.
//! - O `/N` do Tailwind v4 **multiplica** o alfa existente, então `dark:not-data-checked:bg-input/32` é
//!   branco a 8% × 32% ≈ 2,5% — a conta já está feita na paleta do [`crate::checkbox`].
//!
//! **Desvio consciente**
//!
//! - **`Enter` NÃO marca.** O [`crate::Checkbox`], o [`crate::Switch`] e o [`crate::ToggleGroup`]
//!   aceitam `Enter` além de `Space`, e aqui isso foi deliberadamente **não** copiado: num grupo de
//!   rádios do navegador o `Enter` submete o formulário, não muda a opção — quem muda são as setas e o
//!   `Space`. Não há risco de beco sem saída (que é a razão pela qual os outros três tratam a tecla):
//!   um item alcançado por `Tab` já é operável por seta e por `Space`.
//! - **`cursor-pointer` na fileira habilitada.** O primitivo da referência não declara cursor pro
//!   estado normal (só o `cursor-not-allowed` do desabilitado); a mãozinha é a convenção da casa, a
//!   mesma do [`crate::button`] e do [`crate::checkbox`].
//! - **O bisel escuro é o DOBRO da referência** (branco a ~11,8%, alfa `0x1e`, contra os 6% do
//!   original). Não é decisão deste módulo: ele vem inteiro da paleta do [`crate::checkbox`], onde o
//!   desvio está declarado e travado por teste. Vale aqui pela mesma razão — filete BRANCO sobre fundo
//!   escuro.
//! - **O rótulo não vem da referência.** O `radio-group.tsx` é só o círculo; o texto ao lado é um
//!   `<Label>` que o call site compõe. Os valores (`text-sm` = 14px com 20px de entrelinha,
//!   `--foreground`, `gap-2` = 8px) são os do [`crate::checkbox`], que já os declarou como convenção de
//!   label de formulário do coss — e reusá-los é o que mantém um checkbox e um rádio alinhados na mesma
//!   coluna de um formulário.
//! - **A fileira INTEIRA marca**, não só o círculo. É consequência do rótulo, que a referência não tem.
//!
//! **Superset consciente**
//!
//! - **Roving tabindex real** (ver a seção de teclado). O `RadioGroupPrimitive` do base-ui é um
//!   *composite* e faz isso nativamente; aqui é explícito, via [`gpui::FocusHandle::tab_stop`].
//! - **`Home`/`End`** vão pras pontas habilitadas. O rádio nativo do navegador não faz isso; o
//!   composite do base-ui faz, e o [`crate::tabs`] desta base também.
//! - **[`RadioGroupItem`] é dado puro** (rótulo + `disabled`), e não uma `Entity` nem um elemento. É o
//!   mesmo arranjo do [`crate::ToggleGroupItem`] e do `TabsTab`, e pela razão mais forte de lá: a
//!   exclusividade é uma regra do CONJUNTO, e com o estado dentro de cada item ela viraria uma
//!   varredura escrevendo em N donos. Aqui a exclusividade é o TIPO — [`Option<usize>`], que não tem
//!   como guardar dois.
//! - **`invalid` é do grupo, não do item.** No original o `aria-invalid` chega em cada `Radio` pelo
//!   campo de formulário que os envolve, ou seja é sempre o grupo inteiro que fica inválido. Um
//!   `invalid` por item permitiria um estado que o original não produz.
//! - **Seleção vazia é representável** (`Option<usize>`), e é o estado inicial. O base-ui também
//!   permite (`value` não controlado começa vazio); o que NÃO existe é voltar pro vazio por interação —
//!   clicar no item já marcado não o desmarca (é a diferença entre um rádio e o
//!   [`crate::ToggleGroupMode::Single`], que aceita ficar vazio).
//!
//! **Ausente**
//!
//! - **`orientation` / grupo horizontal.** A referência crava `flex flex-col`: existe UMA direção.
//!   Nenhum builder de orientação foi inventado — e por isso as setas dos dois eixos fazem a mesma
//!   coisa (`↓`/`→` avançam, `↑`/`←` voltam), que é o comportamento do rádio nativo.
//! - **`readOnly`, `required`, `name` e `value`** do primitivo: são props de formulário HTML. Este
//!   crate não tem formulário, e o índice é o identificador (o mesmo contrato do
//!   [`crate::ToggleGroup`] e do [`crate::Tabs`]).
//! - **Variante de tamanho.** O `size-4.5 sm:size-4` do original é um breakpoint, não uma prop.
//!
//! # Sem cobertura de teste — declarado
//!
//! - **A pintura.** Que a sombra sai com o alfa escalado, que a borda de UM lado desenha o filete no
//!   arco de baixo de um círculo e que o `rounded_full` fecha nos overlays dilatados são propriedades
//!   do shader de quad do GPUI, invisíveis a teste de unidade. O que dá pra travar — os números e o
//!   LADO do bisel — está travado.
//! - **A conversão border box → `inset` do GPUI** (o `-1` de [`overlay_inset`]) é medida com uma janela
//!   de verdade no [`crate::checkbox`] (`overlay_zero_coincide_com_a_border_box_da_caixa`), e a
//!   aritmética aqui é a mesma função da mesma constante. Não há segunda medição.
//! - **A composição `anel_aceso = focado && focus_ring::visible()`.** Verificada por mutação e
//!   **sobreviveu**: trocá-la por só `focado` não quebra teste nenhum, porque o que ela alimenta é a
//!   PINTURA (os dois overlays do anel, e a troca da borda pro `invalid_border_focus`). O que dá pra
//!   provar está provado — que a modalidade é do último input está no [`crate::focus_ring`], e que o
//!   clique não a liga e a tecla liga está em
//!   `tests_de_janela::o_clique_nao_acende_o_anel_mas_a_tecla_acende`. O elo entre os dois e o pixel
//!   fica declarado aqui em vez de coberto por um teste que só reafirmaria o `&&`.
//! - **A largura da fileira** tem teste (`tests_de_janela::a_fileira_nao_estica_alem_do_rotulo`) mas
//!   **não tem mutação**, e é honesto dizer por quê: não há decisão pra quebrar. O CSS esticaria a
//!   fileira no eixo transversal (`align-items: stretch`) e o alvo de clique iria até a borda do pai; o
//!   GPUI 0.2.2 não estica, medido — então não existe `items_start` a remover. O teste fica como guarda
//!   contra alguém pôr `w_full`/`flex_1` na fileira, não como prova de uma linha de código.
//! - **O gate `if habilitado` do [`RadioGroup::item_row`]** (que decide pendurar `track_focus` e
//!   `on_click` ou aplicar o `opacity-64`) é **defesa em profundidade**, e por isso não é falsificável:
//!   com ele removido, o clique num item travado ainda é recusado pelo [`RadioGroup::choose`], e um
//!   grupo inteiramente desabilitado ainda sai do `Tab` porque o [`tab_stop_index`] devolve `None`. Fica
//!   assim de propósito — é a mesma decisão que o [`crate::toggle_group`] declara pro guarda de
//!   `disabled` do `on_key` dele: o dia em que uma das outras duas camadas mudar, esta é a que impede o
//!   item inerte voltar a responder. O que sobra sem cobertura é só o `opacity`, que é pintura.

use crate::checkbox::{
    ease, palette, progress, BORDER, BOX, DISABLED_OPACITY, EASE_TAILWIND, GAP, LABEL_LINE_HEIGHT,
    LABEL_SIZE, RING_OFFSET, RING_WIDTH, TRANSITION,
};
use gpui::{
    div, point, px, BoxShadow, Context, CursorStyle, Div, EventEmitter, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Render, SharedString,
    Stateful, StatefulInteractiveElement, Styled, Window,
};
use std::time::Instant;

// =================================================================================================
// Geometria — só o que difere do checkbox
// =================================================================================================
//
// Cor NÃO aparece neste módulo, e é de propósito: ela é toda do `crate::checkbox`. Uma segunda paleta
// aqui seria uma segunda fonte de verdade pros mesmos `--input`, `--primary`, `--ring` e
// `--destructive` — ver "O que veio do checkbox" no doc do módulo.

/// Diâmetro do ponto do indicador — `before:size-2` com `sm:before:size-1.5`, e o `sm:` sempre vale em
/// desktop → **6px** (1,5 × 4px de `--spacing`).
const DOT: f32 = 6.0;

/// Respiro entre as opções — o `gap-3` do `RadioGroup` → **12px** (3 × 4px de `--spacing`).
const GROUP_GAP: f32 = 12.0;

// =================================================================================================
// Overlays absolutos — a versão circular
// =================================================================================================

/// O `inset` de um overlay que coincide com a **border box** do círculo dilatada em `out` pixels.
///
/// ⚠️ O `inset` do GPUI é relativo à **padding box** do pai (é a regra do CSS: o bloco contêiner de um
/// filho absoluto é a padding box do contêiner). Com o círculo tendo 1px de borda, coincidir com a
/// **border** box é `inset: -1px` — e é exatamente o que a referência escreve no indicador
/// (`absolute -inset-px`).
///
/// Só o `inset`, e não o raio: no [`crate::checkbox`] o gêmeo desta função devolve os dois porque lá o
/// raio é um número que cresce junto com a dilatação. Aqui é `rounded-full`, então o raio não é
/// parâmetro de nada — o GPUI apara `px(9999)` em `min(w,h)/2` na hora de pintar, e qualquer dilatação
/// continua um círculo.
fn overlay_inset(out: f32) -> f32 {
    -(BORDER + out)
}

/// Um overlay absoluto **circular** que coincide com a border box do círculo dilatada em `out` pixels.
fn overlay(out: f32) -> Div {
    let inset = overlay_inset(out);
    div()
        .absolute()
        .top(px(inset))
        .left(px(inset))
        .right(px(inset))
        .bottom(px(inset))
        .rounded_full()
}

/// **De que lado cai o fio de bisel**, a partir do sinal do deslocamento da referência.
///
/// `before:shadow-[0_1px_…]` (tema claro) empurra a sombra pra BAIXO, então o filete visível é o de
/// baixo; `0 -1px` (escuro) → o de cima. É a mesma regra do [`crate::checkbox`] e do [`crate::card`], e
/// está aqui como função pura porque um `if` dentro do construtor do overlay é inalcançável por teste —
/// e trocar o lado não quebra nada que compile.
fn bevel_on_bottom(bevel_dir: f32) -> bool {
    bevel_dir > 0.0
}

/// O **fio de bisel** de 1px sobre a borda do círculo.
///
/// No coss é `before:absolute before:inset-0 before:rounded-full` com `box-shadow: 0 ±1px <cor>`: um
/// pseudo-elemento transparente na padding box, cuja sombra só aparece no 1px que ESCAPA — ou seja, um
/// crescente sobre a borda. Aqui é a borda de 1px de um único lado de um overlay que cobre a border
/// box; ver "Não reproduzível" no doc do módulo pra por que não pode ser sombra.
fn bevel() -> Div {
    let p = palette();
    let o = overlay(0.0).border_color(p.bevel.hsla());
    if bevel_on_bottom(p.bevel_dir) {
        o.border_b(px(BORDER))
    } else {
        o.border_t(px(BORDER))
    }
}

/// O **anel de foco** — `focus-visible:ring-2 ring-ring ring-offset-1 ring-offset-background`.
///
/// No CSS um `ring` com offset são duas sombras concêntricas: 1px da cor do FUNDO colada no elemento, e
/// 2px da cor do anel por fora dela. Então são dois overlays com bandas **adjacentes e sem
/// sobreposição**: a coroa vai de 0 a 1 fora da border box, o anel de 1 a 3.
///
/// `k` é a opacidade do conjunto — o anel entra e sai desvanecendo, porque no CSS ele é `box-shadow` e
/// o círculo declara `transition-shadow`.
///
/// ⚠️ A coroa é pintada com `--background`, literalmente como o `ring-offset-background` do Tailwind.
/// Sobre uma superfície que NÃO é `--background` (um card, um popover) aparece um fio de 1px da cor
/// errada em volta — e é assim na referência também.
fn ring_overlays(k: f32, invalid: bool) -> [Div; 2] {
    let p = palette();
    let cor = if invalid { p.invalid_ring } else { p.ring };
    [
        overlay(RING_OFFSET)
            .border(px(BORDER))
            .border_color(p.ring_offset.scaled(k)),
        overlay(RING_OFFSET + RING_WIDTH)
            .border(px(RING_WIDTH))
            .border_color(cor.scaled(k)),
    ]
}

/// O indicador do estado marcado — `absolute -inset-px size-4 rounded-full data-checked:bg-primary`
/// com um `before:size-1.5 before:rounded-full before:bg-primary-foreground` no meio.
///
/// Ou seja: um disco `--primary` que cobre a border box INTEIRA (borda incluída, que é o que o
/// `-inset-px` faz) e, centrado nele, o ponto de [`DOT`] pixels em `--primary-foreground`. Diferente do
/// [`crate::checkbox`], não há SVG nenhum — nenhum ícone novo entrou em `assets/`.
///
/// Não existe o equivalente do `data-indeterminate` do checkbox: um rádio tem dois estados.
fn indicator() -> Div {
    let p = palette();
    overlay(0.0)
        .flex()
        .items_center()
        .justify_center()
        .bg(p.primary.hsla())
        .child(
            div()
                .flex_none()
                .w(px(DOT))
                .h(px(DOT))
                .rounded_full()
                .bg(p.primary_fg.hsla()),
        )
}

// =================================================================================================
// Estado — as regras puras
// =================================================================================================

/// Se o círculo está no estado de **repouso**, o único em que a sombra `shadow-xs/5` e o fio de bisel
/// aparecem. É literalmente o par de condições da referência, que por acaso é o mesmo pros dois:
///
/// - `[[data-disabled],[data-checked],[aria-invalid]]:shadow-none`
/// - `not-data-disabled:not-data-checked:not-aria-invalid:before:shadow-[…]`
///
/// O gêmeo do [`crate::checkbox`] tem um terceiro estado (indeterminado, que conserva os dois); aqui
/// `checked` é um `bool` porque o rádio não tem terceiro estado.
fn resting(disabled: bool, checked: bool, invalid: bool) -> bool {
    !disabled && !checked && !invalid
}

/// **Quem está habilitado**, item por item: o `disabled` do grupo desabilita TODOS, e cada item pode se
/// desabilitar sozinho. É a composição do `disabled` do `RadioGroupPrimitive` com o do `Radio`, e a
/// máscara que a navegação por setas e o roving tabindex consultam.
fn enabled_bits(items: &[RadioGroupItem], group_disabled: bool) -> Vec<bool> {
    items
        .iter()
        .map(|it| !group_disabled && !it.disabled)
        .collect()
}

/// **Os bits marcado/desmarcado do grupo** — a forma em que o render precisa da seleção.
///
/// É aqui que a exclusividade fica observável: o `Option<usize>` já impede guardar dois índices, e esta
/// função é o que traduz isso pros N círculos. Um índice fora da lista não marca ninguém.
fn checked_bits(len: usize, selected: Option<usize>) -> Vec<bool> {
    (0..len).map(|i| selected == Some(i)).collect()
}

/// **A escolha do item `idx`**: devolve o novo índice selecionado, ou `None` quando nada muda.
///
/// Nada muda em três casos, e cada um por uma razão diferente:
///
/// - `idx` fora da lista — não existe o que escolher;
/// - `idx` desabilitado — `data-disabled` não responde a interação;
/// - `idx` **já é o selecionado** — e este é o que separa um rádio de um toggle: clicar no item marcado
///   NÃO o desmarca (o [`crate::ToggleGroupMode::Single`] desmarca, e é por isso que ele não é um
///   rádio). Devolver `None` aqui é o que impede reemitir um evento que não descreve mudança nenhuma.
fn apply_choice(selected: Option<usize>, enabled: &[bool], idx: usize) -> Option<usize> {
    if !enabled.get(idx).copied().unwrap_or(false) {
        return None;
    }
    if selected == Some(idx) {
        return None;
    }
    Some(idx)
}

/// **O passo das setas**: o próximo (`dir = 1`) ou o anterior (`dir = -1`) índice habilitado, dando a
/// volta na ponta.
///
/// `from = None` (nada escolhido ainda) faz a seta **entrar pela ponta de onde ela vem**: `↓` cai no
/// primeiro habilitado, `↑` no último. É o que o navegador faz com um grupo de rádios virgem.
///
/// Num grupo com um único item habilitado ele devolve o próprio `from` depois de dar a volta inteira —
/// e aí o [`apply_choice`] recusa, que é o resultado certo.
///
/// ⚠️ Gêmeo do `step_index` do [`crate::tabs`]. A duplicação é declarada e deliberada: o de lá é
/// privado do módulo e `tabs.rs` não é um dos arquivos que esta mudança tem permissão pra tocar. O
/// lugar certo dos dois é um módulo de navegação compartilhado (`tabs`, `menu` e este usariam o
/// mesmo) — extrair é um passo separado. A diferença de contrato é o `from` opcional, que as abas não
/// precisam ter (há sempre uma aba ativa).
pub(crate) fn step_index(from: Option<usize>, enabled: &[bool], dir: isize) -> Option<usize> {
    let len = enabled.len();
    if len == 0 {
        return None;
    }
    let Some(from) = from else {
        return edge_index(enabled, dir < 0);
    };
    let mut i = from.min(len - 1);
    for _ in 0..len {
        i = (i as isize + dir).rem_euclid(len as isize) as usize;
        if enabled[i] {
            return Some(i);
        }
    }
    None
}

/// O primeiro (`last = false`) ou o último (`last = true`) item habilitado — `Home`/`End`.
pub(crate) fn edge_index(enabled: &[bool], last: bool) -> Option<usize> {
    if last {
        enabled.iter().rposition(|e| *e)
    } else {
        enabled.iter().position(|e| *e)
    }
}

/// **Qual item é a ÚNICA parada de `Tab` do grupo** — o roving tabindex.
///
/// A opção marcada, se ela estiver habilitada; senão a primeira habilitada. `None` num grupo sem
/// nenhum item habilitado, e aí o grupo inteiro sai da ordem de tabulação — que é o certo: não há o
/// que fazer lá dentro.
///
/// A marcada vir primeiro não é detalhe: é o que faz `Tab` levar o usuário direto pra sua escolha atual
/// em vez de pro topo da lista, e é o comportamento do `<input type="radio">` do navegador.
pub(crate) fn tab_stop_index(selected: Option<usize>, enabled: &[bool]) -> Option<usize> {
    if let Some(i) = selected {
        if enabled.get(i).copied().unwrap_or(false) {
            return Some(i);
        }
    }
    edge_index(enabled, false)
}

/// Normaliza um índice vindo **de fora** (builder ou setter): `None` quando ele não existe na lista.
///
/// Um item **desabilitado** é aceito de propósito: o HTML permite `<input type=radio disabled checked>`
/// (um formulário pode chegar com a opção travada já marcada), e recusar aqui apagaria em silêncio um
/// estado que quem chama tinha. O que o desabilitado não faz é ser ESCOLHIDO por interação — isso é o
/// [`apply_choice`].
fn normalize(len: usize, i: Option<usize>) -> Option<usize> {
    i.filter(|i| *i < len)
}

// =================================================================================================
// Tempo — os relógios de UM item
// =================================================================================================

/// Os dois relógios de transição de um item.
///
/// O [`crate::Checkbox`] guarda estes quatro campos soltos no struct dele porque ele é UM controle;
/// aqui são N, então eles viram um vetor paralelo aos itens. A curva e a duração são as mesmas —
/// [`crate::checkbox::ease`], [`EASE_TAILWIND`] e [`TRANSITION`].
#[derive(Clone, Copy, Debug, Default)]
struct Clocks {
    /// Se o item estava em [`resting`] no último render. `None` = ainda não renderizou.
    seen_resting: Option<bool>,
    /// Instante da última troca do repouso — o relógio da sombra.
    shadow_at: Option<Instant>,
    /// Se o anel estava aceso no último render. `None` = ainda não renderizou.
    seen_ring: Option<bool>,
    /// Instante da última troca do anel.
    focus_at: Option<Instant>,
}

impl Clocks {
    /// Detecta a troca dos dois estados e (re)inicia o relógio de quem mudou.
    ///
    /// **O primeiro render nunca anima**: sem isso, todo rádio da tela desvaneceria a sombra ao abrir a
    /// janela. É o que o `None` inicial dos dois `seen_*` significa.
    fn tick(&mut self, em_repouso: bool, anel: bool) {
        if self.seen_resting != Some(em_repouso) {
            let primeiro = self.seen_resting.is_none();
            self.seen_resting = Some(em_repouso);
            if !primeiro {
                self.shadow_at = Some(Instant::now());
            }
        }
        if self.seen_ring != Some(anel) {
            let primeiro = self.seen_ring.is_none();
            self.seen_ring = Some(anel);
            if !primeiro {
                self.focus_at = Some(Instant::now());
            }
        }
    }

    /// `(opacidade da sombra, opacidade do anel, ainda animando)`.
    ///
    /// Cada progresso é "quanto andou desde a última troca"; virar "quanto do estado NOVO aplicar" é
    /// uma inversão quando o destino é apagado — daí o `1 - e`.
    fn amounts(self, em_repouso: bool, anel: bool) -> (f32, f32, bool) {
        let sombra_raw = progress(self.shadow_at, TRANSITION);
        let anel_raw = progress(self.focus_at, TRANSITION);
        let k = |raw: f32, destino: bool| {
            let e = ease(EASE_TAILWIND, raw);
            if destino {
                e
            } else {
                1.0 - e
            }
        };
        (
            k(sombra_raw, em_repouso),
            k(anel_raw, anel),
            sombra_raw < 1.0 || anel_raw < 1.0,
        )
    }
}

// =================================================================================================
// Um item
// =================================================================================================

/// A **definição** de uma opção: o que ela mostra e se dá pra escolher.
///
/// Não é um elemento nem uma `Entity` — ver "Superset consciente" no doc do módulo. O estado
/// marcado/desmarcado não está aqui de propósito: ele é a seleção do GRUPO.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadioGroupItem {
    label: SharedString,
    disabled: bool,
}

impl RadioGroupItem {
    /// Uma opção com `label`.
    ///
    /// Rótulo **vazio** (`""`) rende só o círculo, que é exatamente o que o `radio-group.tsx` é — use
    /// isso quando o texto ao lado for montado por quem chama.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
        }
    }

    /// Desabilita esta opção: ela para de responder, sai da navegação por setas e da ordem de
    /// tabulação, e esmaece pra 64% com cursor de "não permitido".
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// O rótulo (vazio = só o círculo).
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// Se esta opção está desabilitada.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// Evento emitido quando o usuário troca a opção escolhida.
///
/// Carrega **só o novo índice**: num grupo de rádios a seleção é um índice, então "o que o usuário fez"
/// e "o resultado" são a mesma coisa — diferente do [`crate::ToggleGroupEvent`], onde um toque muda o
/// estado de dois itens.
///
/// Nunca é emitido sem mudança: clicar (ou apertar `Space`) no item já marcado não emite nada, ver
/// [`apply_choice`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioGroupEvent {
    /// Nova opção marcada.
    Change(usize),
}

/// Escolha 1-de-N com o visual do coss. Ver o doc do módulo.
pub struct RadioGroup {
    /// As opções, na ordem dos índices.
    items: Vec<RadioGroupItem>,
    /// **A seleção — e a exclusividade.** Um `Option<usize>` não tem como guardar dois índices, então
    /// não existe caminho no código que deixe dois círculos marcados.
    selected: Option<usize>,
    /// Um handle de foco por item. É por eles que o roving tabindex acontece: a cada frame exatamente
    /// um recebe `tab_stop(true)`. Nascem e morrem junto com a lista — ver [`Self::set_items`].
    focus_handles: Vec<gpui::FocusHandle>,
    /// Os relógios de transição, um por item. Vetor paralelo, mesma disciplina.
    clocks: Vec<Clocks>,
    /// `disabled` do grupo: desabilita todos os itens.
    disabled: bool,
    /// `aria-invalid` do grupo — ver "Superset consciente" no doc do módulo.
    invalid: bool,
    /// Id estável da entidade, pro `div().id(..)` da raiz.
    id: u64,
}

impl RadioGroup {
    /// Um grupo com `items` e **nenhuma** opção marcada.
    pub fn new(items: Vec<RadioGroupItem>, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handles: items.iter().map(|_| cx.focus_handle()).collect(),
            clocks: vec![Clocks::default(); items.len()],
            items,
            selected: None,
            disabled: false,
            invalid: false,
            id: cx.entity_id().as_u64(),
        }
    }

    /// Marca uma opção (builder). Índice fora da lista é descartado — ver [`normalize`].
    pub fn with_selected(mut self, selected: Option<usize>) -> Self {
        self.selected = normalize(self.items.len(), selected);
        self
    }

    /// Desabilita o grupo INTEIRO (builder): nenhuma opção responde, o grupo sai da ordem de
    /// tabulação e todas as fileiras esmaecem pra 64%.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marca o grupo como **inválido** (builder) — o `aria-invalid` da referência: a borda de todos os
    /// círculos vira `--destructive` a 36% (64% com o foco visível), o anel de foco troca de cor, e a
    /// sombra e o bisel somem.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// As opções, na ordem dos índices.
    pub fn items(&self) -> &[RadioGroupItem] {
        &self.items
    }

    /// A opção marcada, se alguma.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Se a opção `i` está marcada (`false` pra índice fora da lista).
    pub fn is_selected(&self, i: usize) -> bool {
        self.selected == Some(i)
    }

    /// Se o grupo está desabilitado.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Se o grupo está marcado como inválido.
    pub fn is_invalid(&self) -> bool {
        self.invalid
    }

    /// Marca uma opção de fora (sincronização externa). **Não** emite `Change` — evita loops de
    /// feedback com quem assina o evento, mesma regra do `Checkbox::set_checked` e do
    /// `ToggleGroup::set_selected`.
    pub fn set_selected(&mut self, selected: Option<usize>, cx: &mut Context<Self>) {
        let novo = normalize(self.items.len(), selected);
        if self.selected == novo {
            return;
        }
        self.selected = novo;
        cx.notify();
    }

    /// Troca a lista de opções. Sincronização de fora, **não** emite.
    ///
    /// Os vetores paralelos (handles e relógios) são refeitos junto: eles são POR ITEM, e um deles fora
    /// de sincronia com a lista é um `panic` de índice esperando um render. A seleção é perdida porque
    /// os índices antigos não descrevem mais nada — o mesmo preço de honestidade do
    /// `ToggleGroup::set_items`.
    pub fn set_items(&mut self, items: Vec<RadioGroupItem>, cx: &mut Context<Self>) {
        self.focus_handles = items.iter().map(|_| cx.focus_handle()).collect();
        self.clocks = vec![Clocks::default(); items.len()];
        self.items = items;
        self.selected = None;
        cx.notify();
    }

    /// Habilita/desabilita o grupo inteiro em runtime.
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        cx.notify();
    }

    /// Marca/desmarca o grupo como inválido em runtime.
    pub fn set_invalid(&mut self, invalid: bool, cx: &mut Context<Self>) {
        if self.invalid == invalid {
            return;
        }
        self.invalid = invalid;
        cx.notify();
    }

    /// Põe o foco de teclado na parada de `Tab` do grupo (ver [`tab_stop_index`]). No-op num grupo sem
    /// nenhum item habilitado.
    ///
    /// É o equivalente de `focus()` num `<fieldset>` de rádios: quem chama não precisa saber qual dos N
    /// handles é o de entrada.
    pub fn focus(&self, window: &mut Window) {
        let enabled = enabled_bits(&self.items, self.disabled);
        if let Some(i) = tab_stop_index(self.selected, &enabled) {
            if let Some(h) = self.focus_handles.get(i) {
                h.focus(window);
            }
        }
    }

    /// O índice do item que está com o foco de teclado, se algum.
    fn focused_item(&self, window: &Window) -> Option<usize> {
        self.focus_handles.iter().position(|h| h.is_focused(window))
    }

    /// Escolhe a opção `idx` por interação e **emite** [`RadioGroupEvent::Change`]. Nada acontece
    /// quando a escolha não muda nada — ver [`apply_choice`].
    fn choose(&mut self, idx: usize, cx: &mut Context<Self>) {
        let enabled = enabled_bits(&self.items, self.disabled);
        let Some(novo) = apply_choice(self.selected, &enabled, idx) else {
            return;
        };
        self.selected = Some(novo);
        cx.emit(RadioGroupEvent::Change(novo));
        cx.notify();
    }

    /// Teclado no grupo. Ver a tabela em "Teclado" no doc do módulo.
    ///
    /// As setas movem a seleção **e** o foco (ativação automática, como o rádio nativo e como o
    /// [`crate::tabs`] desta base). `Space` marca o item FOCADO — o caso de quem chegou ali por `Tab`.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let enabled = enabled_bits(&self.items, self.disabled);
        let alvo = match event.keystroke.key.as_str() {
            // Os dois eixos fazem a mesma coisa: a referência crava `flex-col`, e o rádio nativo
            // responde às quatro setas. Ver "Ausente" no doc do módulo.
            "down" | "right" => step_index(self.selected, &enabled, 1),
            "up" | "left" => step_index(self.selected, &enabled, -1),
            "home" => edge_index(&enabled, false),
            "end" => edge_index(&enabled, true),
            "space" => self.focused_item(window),
            // `enter` NÃO está aqui de propósito — ver "Desvio consciente" no doc do módulo.
            _ => return,
        };
        let Some(alvo) = alvo else { return };
        // Chegou aqui = o usuário está no teclado. O `focus_ring::init` já teria marcado isso; marcar
        // de novo faz a tecla acender o anel mesmo num app que esqueceu de inicializar o crate.
        crate::focus_ring::keyboard_used(window);
        self.choose(alvo, cx);
        // O foco acompanha a seleção mesmo quando o `choose` recusou (item já marcado, ou único
        // habilitado): focar quem já está focado é no-op, e não fazer nada deixaria o anel parado num
        // item que a seta acabou de passar.
        if let Some(h) = self.focus_handles.get(alvo) {
            window.focus(h);
        }
    }

    /// A fileira da opção `i`: círculo + rótulo, já com estado, foco e clique.
    ///
    /// Devolve também **se há transição em curso** — é o que faz o `render` pedir o próximo frame.
    fn item_row(
        &self,
        i: usize,
        marcado: bool,
        habilitado: bool,
        parada: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> (Stateful<Div>, bool) {
        let p = palette();
        let em_repouso = resting(!habilitado, marcado, self.invalid);
        let focado = habilitado
            && self
                .focus_handles
                .get(i)
                .is_some_and(|h| h.is_focused(window));
        // A referência pede `focus-visible:ring-2`, não `focus:ring-2` — o anel só acende quando o foco
        // veio do TECLADO. Ver [`crate::focus_ring`].
        let anel_aceso = focado && crate::focus_ring::visible();
        let (sombra_k, anel_k, animando) = self.clocks[i].amounts(em_repouso, anel_aceso);

        // --- O círculo ------------------------------------------------------------------------
        //
        // `relative` porque indicador, bisel e anel são filhos ABSOLUTOS. Sem `overflow_hidden`: o
        // anel PRECISA sair do círculo, e o ContentMask do GPUI é retangular — ele cortaria o anel sem
        // sequer respeitar o raio.
        let borda = if self.invalid {
            if anel_aceso {
                p.invalid_border_focus
            } else {
                p.invalid_border
            }
        } else {
            p.border
        };
        let fundo = if marcado { p.bg } else { p.bg_unchecked };
        let mut caixa = div()
            .relative()
            .flex_none()
            .w(px(BOX))
            .h(px(BOX))
            .rounded_full()
            .border(px(BORDER))
            .border_color(borda.hsla())
            .bg(fundo.hsla());

        // `shadow-xs/5` = `0 1px 2px 0 rgb(0 0 0/.05)`. Aqui pode ser sombra de verdade e não overlay:
        // ela é EXTERNA e o círculo tem fundo. Mesmo compromisso do `crate::checkbox` no tema escuro,
        // onde o fundo do círculo vazio é translúcido (~2,5% de branco) e a sombra atravessa de forma
        // imperceptível.
        if sombra_k > 0.0 {
            caixa = caixa.shadow(vec![BoxShadow {
                color: p.shadow.scaled(sombra_k),
                offset: point(px(0.0), px(1.0)),
                blur_radius: px(2.0),
                spread_radius: px(0.0),
            }]);
        }
        // O bisel é instantâneo: no CSS ele é `box-shadow` de um pseudo-elemento, e `transition` não é
        // herdada por pseudo-elemento.
        if em_repouso {
            caixa = caixa.child(bevel());
        }
        // `data-unchecked:hidden`.
        if marcado {
            caixa = caixa.child(indicator());
        }
        // O anel vem por último: é absoluto, então a ordem só decide quem pinta em cima.
        if anel_k > 0.0 {
            caixa = caixa.children(ring_overlays(anel_k, self.invalid));
        }

        // --- A fileira ------------------------------------------------------------------------
        //
        // O id só precisa ser único ENTRE AS IRMÃS: o `GlobalElementId` do GPUI é o caminho de ids dos
        // ancestrais, e a raiz já carrega o id da entidade.
        let mut row = div()
            .id(("empire-radio", i))
            .flex()
            .items_center()
            .gap(px(GAP))
            .cursor(if habilitado {
                CursorStyle::PointingHand
            } else {
                // `data-disabled:cursor-not-allowed`.
                CursorStyle::OperationNotAllowed
            });

        if habilitado {
            if let Some(h) = self.focus_handles.get(i) {
                // **O roving tabindex.** `tab_stop` grava no `FocusRef` compartilhado, e o
                // `Window::focus_next` só enxerga handles marcados — então marcar um e DESmarcar os
                // outros a cada frame é o que dá UMA parada de `Tab` pro grupo inteiro. O valor é
                // escrito sempre (nunca só quando `true`), senão um item que já foi parada continuaria
                // sendo depois de a seleção sair dele.
                row = row.track_focus(&h.clone().tab_stop(parada));
            }
            row = row
                // Área de clique generosa: a fileira INTEIRA (círculo + rótulo) escolhe.
                .on_click(cx.listener(move |this, _e, _window, cx| this.choose(i, cx)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_this, _e: &MouseDownEvent, window, _cx| {
                        // O mouse-down é ponteiro: apaga o anel (ver `crate::focus_ring`). Fica aqui e
                        // não no `on_click` porque é o evento mais cedo.
                        crate::focus_ring::pointer_used(window);
                    }),
                );
        } else {
            // `data-disabled:opacity-64`: esmaece o conjunto — círculo, ponto e rótulo.
            row = row.opacity(DISABLED_OPACITY);
        }

        row = row.child(caixa);
        // Rótulo vazio = só o círculo, que é o que o `radio-group.tsx` é.
        if !self.items[i].label.is_empty() {
            row = row.child(
                div()
                    .text_size(px(LABEL_SIZE))
                    .line_height(px(LABEL_LINE_HEIGHT))
                    .text_color(p.foreground.hsla())
                    .child(self.items[i].label.clone()),
            );
        }

        (row, animando)
    }
}

impl EventEmitter<RadioGroupEvent> for RadioGroup {}

impl Render for RadioGroup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let total = self.items.len();
        let enabled = enabled_bits(&self.items, self.disabled);
        let marcados = checked_bits(total, self.selected);
        let parada = tab_stop_index(self.selected, &enabled);

        // Passo 1: os relógios (precisa de `&mut self`). Nem o foco nem a modalidade chegam por
        // callback — as duas são lidas a cada render, e a troca é detectada comparando com o último
        // valor visto.
        for i in 0..total {
            let em_repouso = resting(!enabled[i], marcados[i], self.invalid);
            let anel = enabled[i]
                && self.focus_handles[i].is_focused(window)
                && crate::focus_ring::visible();
            self.clocks[i].tick(em_repouso, anel);
        }

        // Passo 2: os elementos.
        let mut raiz = div()
            .id(("empire-radio-group", self.id))
            .flex()
            // `flex flex-col gap-3` — a referência crava a coluna. Ver "Ausente" no doc do módulo.
            .flex_col()
            .gap(px(GROUP_GAP));
        // ⚠️ **Sem `items_start` aqui, e isso foi MEDIDO.** No CSS o default de `align-items` num
        // contêiner de coluna é `stretch`, então cada fileira ganharia a largura do grupo — e como a
        // fileira INTEIRA é clicável (ver [`Self::item_row`]), o alvo de clique se estenderia até a
        // borda do pai, com uma faixa de vazio respondendo por uma opção. O GPUI 0.2.2 **não** faz
        // isso: a fileira já mede o conteúdo (círculo + respiro + texto). Um `.items_start()` aqui
        // seria estilo sem efeito, então ele não está aqui — o que trava a propriedade é o teste
        // [`tests_de_janela::a_fileira_nao_estica_alem_do_rotulo`], que é o que pegaria alguém pondo
        // `w_full`/`flex_1` na fileira e ressuscitando o problema.
        let mut animando = false;
        for i in 0..total {
            let (row, anim) = self.item_row(i, marcados[i], enabled[i], parada == Some(i), window, cx);
            animando |= anim;
            raiz = raiz.child(row);
        }

        // O teclado é ouvido pela RAIZ, não pelo item: o evento sobe do elemento focado pelos
        // ancestrais, então um listener só cobre todos (mesmo arranjo do `crate::tabs` e do
        // `crate::toggle_group`).
        raiz = raiz.on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            this.on_key(event, window, cx);
        }));

        // Enquanto houver transição em curso, pede o próximo frame — é o motor das duas animações
        // deste componente.
        if animando {
            window.request_animation_frame();
        }

        raiz
    }
}

#[cfg(test)]
// Travar o valor da referência é o PROPÓSITO destes testes: `assert_eq!(DOT, 6.0)` existe pra que mexer
// na constante quebre o teste, não pra calcular nada.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Três itens, o do meio desabilitado — a lista que a maioria dos testes de navegação usa.
    ///
    /// `pub(super)` porque o módulo de testes de janela monta a MESMA lista: duas listas com o mesmo
    /// item travado no meio divergiriam, e é o índice 1 que os dois testes assumem.
    pub(super) fn itens() -> Vec<RadioGroupItem> {
        vec![
            RadioGroupItem::new("Comedy"),
            RadioGroupItem::new("Drama").disabled(true),
            RadioGroupItem::new("Horror"),
        ]
    }

    // --- Geometria ----------------------------------------------------------------------------

    /// **Os números do `radio-group.tsx`, resolvidos no breakpoint `sm:`** (que numa janela de desktop
    /// sempre vale). Comparados contra LITERAIS, não contra as constantes que eles protegem.
    ///
    /// Os quatro primeiros vêm do [`crate::checkbox`] por promoção de visibilidade, e é justamente por
    /// isso que eles são afirmados aqui: este teste é o segundo leitor daqueles valores, e o que avisa
    /// se um refactor do checkbox mudar a caixa por baixo do rádio.
    #[test]
    fn geometria_da_referencia() {
        assert_eq!(BOX, 16.0, "size-4.5 com sm:size-4");
        assert_eq!(DOT, 6.0, "before:size-2 com sm:before:size-1.5 → 1,5 × 4px");
        assert_eq!(GROUP_GAP, 12.0, "gap-3 → 3 × 4px");
        assert_eq!(BORDER, 1.0, "border");
        assert_eq!(RING_WIDTH, 2.0, "focus-visible:ring-2");
        assert_eq!(RING_OFFSET, 1.0, "focus-visible:ring-offset-1");
        assert_eq!(DISABLED_OPACITY, 0.64, "data-disabled:opacity-64");
        assert_eq!(GAP, 8.0, "gap-2 da linha de formulário (rótulo)");
        assert_eq!(LABEL_SIZE, 14.0, "text-sm");
        assert_eq!(LABEL_LINE_HEIGHT, 20.0, "--text-sm--line-height");
    }

    /// **O `--spacing` do Tailwind é 4px**, e é dele que saem os dois números deste módulo. Se alguém
    /// ler `size-1.5` como 1,5px ou `gap-3` como 3px, é aqui que cai.
    #[test]
    fn os_numeros_saem_do_spacing_de_quatro_pixels() {
        const SPACING: f32 = 4.0;
        assert_eq!(DOT, 1.5 * SPACING, "sm:before:size-1.5");
        assert_eq!(GROUP_GAP, 3.0 * SPACING, "gap-3");
    }

    /// O ponto cabe no círculo com folga simétrica, e não invade a borda. 6 dentro de 16 deixa 5px de
    /// cada lado — que é o que o `items-center justify-center` da referência produz.
    #[test]
    fn o_ponto_cabe_centrado_no_circulo() {
        let folga = (BOX - DOT) / 2.0;
        assert_eq!(folga, 5.0);
        assert!(folga > BORDER, "o ponto não encosta na borda");
    }

    /// **A conversão border box → `inset` do GPUI.** O `inset` é relativo à padding box do pai, então
    /// um overlay que coincide com a BORDER box está em `-BORDER`. Errar isto é o jeito clássico de o
    /// bisel virar uma segunda linha ao lado da borda em vez de cair sobre ela.
    #[test]
    fn overlay_converte_de_border_box_pro_inset_do_gpui() {
        assert_eq!(overlay_inset(0.0), -1.0, "border box");
        assert_eq!(overlay_inset(RING_OFFSET), -2.0, "coroa");
        assert_eq!(overlay_inset(RING_OFFSET + RING_WIDTH), -4.0, "anel");
        // O inset acompanha a dilatação, um a um.
        for out in [0.0, 1.0, 3.0, 7.5] {
            assert_eq!(-overlay_inset(out) - BORDER, out, "dilatação {out}");
        }
    }

    /// As duas bandas do anel são **adjacentes**: a coroa da cor do fundo ocupa de 0 a 1 fora da border
    /// box, o anel de 1 a 3. Sobreposição pintaria a coroa por cima do anel; folga deixaria uma fresta.
    #[test]
    fn as_bandas_do_anel_sao_adjacentes() {
        let coroa = (0.0, RING_OFFSET);
        let anel = (RING_OFFSET, RING_OFFSET + RING_WIDTH);
        assert_eq!(coroa.1, anel.0, "sem fresta e sem sobreposição");
        assert_eq!(anel.1, 3.0, "o anel para 3px fora do círculo");
    }

    /// **O lado do bisel sai do SINAL do deslocamento da referência**: `0 1px` (claro) → filete
    /// embaixo, `0 -1px` (escuro) → em cima. Trocar isto não quebra nada que compile.
    #[test]
    fn o_lado_do_bisel_segue_o_sinal_da_referencia() {
        assert!(bevel_on_bottom(1.0), "shadow-[0_1px_…] → borda de BAIXO");
        assert!(!bevel_on_bottom(-1.0), "shadow-[0_-1px_…] → borda de CIMA");
        // E o tema corrente sempre responde a um dos dois — nunca a nenhum. O `set_theme` mexe em
        // estado `thread_local`, então o tema escuro (o default do crate) é restaurado no fim.
        use crate::theme::{set_theme, ThemeMode};
        for modo in [ThemeMode::Dark, ThemeMode::Light] {
            set_theme(modo);
            assert_ne!(palette().bevel_dir, 0.0, "o bisel tem sentido em {modo:?}");
        }
        set_theme(ThemeMode::Dark);
    }

    // --- Repouso ------------------------------------------------------------------------------

    /// **Sombra e bisel só existem em repouso**, e o repouso é o trio de condições da referência.
    /// Diferente do checkbox, não há terceiro estado que os conserve: marcado perde os dois, sempre.
    #[test]
    fn repouso_e_a_tabela_da_referencia() {
        let casos = [
            (false, false, false, true, "desmarcado e válido: repouso"),
            (false, true, false, false, "data-checked:shadow-none"),
            (true, false, false, false, "data-disabled:shadow-none"),
            (false, false, true, false, "aria-invalid:shadow-none"),
            (true, true, true, false, "tudo junto"),
        ];
        for (disabled, checked, invalid, esperado, quem) in casos {
            assert_eq!(resting(disabled, checked, invalid), esperado, "{quem}");
        }
    }

    // --- Exclusividade ------------------------------------------------------------------------

    /// **No máximo UM item fica marcado, em qualquer estado alcançável.** É a propriedade que define um
    /// grupo de rádios, e a varredura cobre toda seleção possível de um grupo de até 4 itens, mais os
    /// índices fora da lista.
    #[test]
    fn no_maximo_um_item_fica_marcado() {
        for len in 0..=4usize {
            for sel in (0..len + 2).map(Some).chain([None]) {
                let bits = checked_bits(len, sel);
                assert_eq!(bits.len(), len, "um bit por item (len {len})");
                let marcados = bits.iter().filter(|b| **b).count();
                assert!(marcados <= 1, "len {len}, sel {sel:?}: {marcados} marcados");
                // E quando o índice existe, é exatamente ELE que está marcado.
                match sel {
                    Some(i) if i < len => {
                        assert_eq!(marcados, 1);
                        assert!(bits[i], "len {len}: o marcado é o {i}");
                    }
                    _ => assert_eq!(marcados, 0, "len {len}, sel {sel:?}: nenhum marcado"),
                }
            }
        }
    }

    /// **Escolher um item apaga o irmão** — a exclusividade vista de fora, antes e depois.
    #[test]
    fn escolher_um_item_apaga_o_irmao() {
        let habilitados = [true, true, true];
        let antes = checked_bits(3, Some(0));
        assert_eq!(antes, vec![true, false, false]);

        let novo = apply_choice(Some(0), &habilitados, 2).expect("o item 2 aceita a escolha");
        let depois = checked_bits(3, Some(novo));
        assert_eq!(depois, vec![false, false, true], "o 0 apagou quando o 2 acendeu");
    }

    /// **As três recusas do [`apply_choice`]**, cada uma por um motivo diferente. A do meio é a que
    /// separa um rádio de um toggle: clicar no já marcado NÃO desmarca.
    #[test]
    fn apply_choice_recusa_fora_da_lista_desabilitado_e_repetido() {
        let habilitados = [true, false, true];
        assert_eq!(apply_choice(None, &habilitados, 0), Some(0), "escolha normal");
        assert_eq!(apply_choice(Some(0), &habilitados, 2), Some(2), "troca");
        assert_eq!(apply_choice(Some(2), &habilitados, 2), None, "o já marcado não desmarca");
        assert_eq!(apply_choice(None, &habilitados, 1), None, "desabilitado não responde");
        assert_eq!(apply_choice(None, &habilitados, 9), None, "fora da lista");
        assert_eq!(apply_choice(None, &[], 0), None, "grupo vazio");
    }

    /// **O `disabled` do grupo desabilita TODO item**, e o do item vale sozinho. É a composição das
    /// duas props da referência.
    #[test]
    fn o_grupo_desabilitado_desabilita_todo_item() {
        assert_eq!(enabled_bits(&itens(), false), vec![true, false, true], "só o item travado cai");
        assert_eq!(
            enabled_bits(&itens(), true),
            vec![false, false, false],
            "o grupo desabilitado derruba até quem estava habilitado"
        );
        assert!(enabled_bits(&[], false).is_empty(), "grupo vazio");
    }

    /// Índice vindo de fora: existe, entra; não existe, é descartado. Um item **desabilitado** entra —
    /// ver o doc de [`normalize`].
    #[test]
    fn selecao_externa_fora_da_lista_e_descartada() {
        assert_eq!(normalize(3, Some(2)), Some(2));
        assert_eq!(normalize(3, Some(1)), Some(1), "desabilitado ou não, o índice existe");
        assert_eq!(normalize(3, Some(3)), None, "fora por um");
        assert_eq!(normalize(0, Some(0)), None, "grupo vazio");
        assert_eq!(normalize(3, None), None);
    }

    // --- Teclado ------------------------------------------------------------------------------

    /// **As setas andam só pelos habilitados e dão a volta.** O item do meio está desabilitado, então
    /// descer do 0 pula pro 2, e descer do 2 volta pro 0.
    #[test]
    fn as_setas_andam_pelos_habilitados_e_dao_a_volta() {
        let habilitados = [true, false, true];
        assert_eq!(step_index(Some(0), &habilitados, 1), Some(2), "↓ pula o desabilitado");
        assert_eq!(step_index(Some(2), &habilitados, 1), Some(0), "↓ dá a volta");
        assert_eq!(step_index(Some(2), &habilitados, -1), Some(0), "↑ pula o desabilitado");
        assert_eq!(step_index(Some(0), &habilitados, -1), Some(2), "↑ dá a volta");
    }

    /// **Com nada escolhido a seta entra pela ponta de onde ela vem**: `↓` no primeiro habilitado, `↑`
    /// no último. É o que o navegador faz com um grupo de rádios virgem.
    #[test]
    fn as_setas_entram_pela_ponta_quando_nada_esta_escolhido() {
        let habilitados = [false, true, true];
        assert_eq!(step_index(None, &habilitados, 1), Some(1), "↓ → primeiro habilitado");
        assert_eq!(step_index(None, &habilitados, -1), Some(2), "↑ → último habilitado");
    }

    /// Casos de contorno da navegação: grupo vazio, grupo sem nenhum habilitado, e grupo com UM
    /// habilitado (onde a seta dá a volta inteira e devolve o próprio item — e aí o [`apply_choice`]
    /// recusa, que é o resultado certo).
    #[test]
    fn a_navegacao_nos_cantos() {
        assert_eq!(step_index(Some(0), &[], 1), None, "grupo vazio");
        assert_eq!(step_index(None, &[], 1), None, "grupo vazio, sem seleção");
        assert_eq!(step_index(Some(0), &[false, false], 1), None, "ninguém habilitado");
        let so_um = [false, true, false];
        assert_eq!(step_index(Some(1), &so_um, 1), Some(1), "um habilitado: volta pra si");
        assert_eq!(apply_choice(Some(1), &so_um, 1), None, "e a escolha é recusada");
    }

    /// `Home`/`End` vão pras pontas **habilitadas**, não pras geométricas.
    #[test]
    fn home_e_end_vao_pras_pontas_habilitadas() {
        let habilitados = [false, true, true, false];
        assert_eq!(edge_index(&habilitados, false), Some(1), "Home pula o 0 travado");
        assert_eq!(edge_index(&habilitados, true), Some(2), "End pula o 3 travado");
        assert_eq!(edge_index(&[false, false], false), None, "ninguém habilitado");
        assert_eq!(edge_index(&[], true), None, "grupo vazio");
    }

    /// **Há exatamente UMA parada de `Tab`, e ela segue a seleção.** Sem seleção (ou com ela num item
    /// travado) a entrada é o primeiro habilitado; sem nenhum habilitado o grupo sai da ordem de
    /// tabulação inteiro.
    #[test]
    fn so_ha_uma_parada_de_tab_e_ela_segue_a_selecao() {
        let habilitados = [true, false, true];
        assert_eq!(tab_stop_index(Some(2), &habilitados), Some(2), "a marcada é a entrada");
        assert_eq!(
            tab_stop_index(Some(1), &habilitados),
            Some(0),
            "marcada mas travada: cai no primeiro habilitado"
        );
        assert_eq!(tab_stop_index(None, &habilitados), Some(0), "sem seleção: o primeiro");
        assert_eq!(
            tab_stop_index(Some(0), &[false, false, false]),
            None,
            "sem ninguém habilitado o grupo sai do Tab"
        );
        assert_eq!(tab_stop_index(None, &[]), None, "grupo vazio");
    }

    // --- Tempo --------------------------------------------------------------------------------

    /// **O primeiro render nunca anima.** Sem isto, todo rádio da tela desvaneceria a sombra ao abrir
    /// a janela — e o sintoma é o oposto do que se procura num bug de animação.
    #[test]
    fn o_primeiro_render_nunca_anima() {
        let mut c = Clocks::default();
        c.tick(true, false);
        assert_eq!(c.seen_resting, Some(true), "o primeiro render REGISTRA o estado");
        assert_eq!(c.seen_ring, Some(false));
        assert!(c.shadow_at.is_none(), "mas não liga o relógio da sombra");
        assert!(c.focus_at.is_none(), "nem o do anel");

        // Repetir o mesmo estado também não liga nada.
        c.tick(true, false);
        assert!(c.shadow_at.is_none() && c.focus_at.is_none(), "estado repetido não anima");

        // A TROCA liga o relógio de quem mudou, e só dele.
        c.tick(false, false);
        assert!(c.shadow_at.is_some(), "a sombra trocou: relógio ligado");
        assert!(c.focus_at.is_none(), "o anel não trocou: relógio parado");
        c.tick(false, true);
        assert!(c.focus_at.is_some(), "agora o anel trocou");
    }

    /// Com os relógios parados (nada nunca mudou) o item já está **assentado**: a opacidade é 1 no
    /// estado corrente, 0 no oposto, e não há frame pendente.
    #[test]
    fn relogio_parado_ja_esta_assentado() {
        let c = Clocks::default();
        let (sombra, anel, animando) = c.amounts(true, false);
        assert_eq!(sombra, 1.0, "em repouso: sombra cheia");
        assert_eq!(anel, 0.0, "sem anel: anel invisível");
        assert!(!animando, "nada pra animar");

        let (sombra, anel, _) = c.amounts(false, true);
        assert_eq!(sombra, 0.0, "fora de repouso: sem sombra");
        assert_eq!(anel, 1.0, "com anel: anel cheio");
    }
}

/// Testes que precisam de uma **janela de verdade**: os de cima cobrem geometria e as regras puras,
/// mas nada ali prova que o clique chega ao `choose`, que a seta move o foco, ou que o grupo tem UMA
/// parada de `Tab`. Isso só se vê passando eventos reais pelo despacho do GPUI — mesmo arranjo do
/// [`crate::checkbox`].
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{AppContext as _, Modifiers, Pixels, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Um harness mínimo que ancora o grupo no canto da janela. A geometria é FIXA, então o centro de
    /// cada fileira é conhecido sem medir nada.
    struct Harness {
        grupo: gpui::Entity<RadioGroup>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().flex().flex_col().child(self.grupo.clone())
        }
    }

    /// A altura de uma fileira: o rótulo tem 20px de entrelinha e o círculo 16, e a fileira é centrada
    /// pelo eixo transversal — então ela mede a entrelinha.
    const ROW: f32 = LABEL_LINE_HEIGHT;

    /// O centro do CÍRCULO da fileira `i`, em coordenadas da janela.
    fn centro_do_circulo(i: usize) -> gpui::Point<Pixels> {
        let y = (ROW + GROUP_GAP) * i as f32 + ROW / 2.0;
        point(px(BOX / 2.0), px(y))
    }

    /// Um ponto sobre o RÓTULO da fileira `i`, à direita do círculo e do respiro.
    fn sobre_o_rotulo(i: usize) -> gpui::Point<Pixels> {
        let y = (ROW + GROUP_GAP) * i as f32 + ROW / 2.0;
        point(px(BOX + GAP + 4.0), px(y))
    }

    /// Abre a janela com um grupo de três opções montado por `montar`, e devolve a entidade, os eventos
    /// capturados e o contexto visual.
    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
        montar: impl FnOnce(RadioGroup) -> RadioGroup + 'static,
    ) -> (
        gpui::Entity<RadioGroup>,
        Rc<RefCell<Vec<usize>>>,
        VisualTestContext,
    ) {
        let eventos: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_window, cx| {
            let grupo = cx.new(|cx| {
                montar(RadioGroup::new(
                    vec![
                        RadioGroupItem::new("Comedy"),
                        RadioGroupItem::new("Drama"),
                        RadioGroupItem::new("Horror"),
                    ],
                    cx,
                ))
            });
            cx.subscribe(&grupo, move |_this, _g, ev: &RadioGroupEvent, _cx| match ev {
                RadioGroupEvent::Change(i) => capturados.borrow_mut().push(*i),
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

    /// **O clique escolhe e emite**, e o irmão apaga. É o contrato público do componente.
    #[gpui::test]
    fn clique_escolhe_e_emite(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, |g| g);
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), None, "nasce vazio"));

        vcx.simulate_click(centro_do_circulo(0), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(0)));
        assert_eq!(*eventos.borrow(), vec![0]);

        vcx.simulate_click(centro_do_circulo(2), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| {
            let g = grupo.read(cx);
            assert_eq!(g.selected(), Some(2), "trocou");
            assert!(!g.is_selected(0), "e o primeiro apagou");
        });
        assert_eq!(*eventos.borrow(), vec![0, 2]);
    }

    /// **Clicar no já marcado não muda nada e não reemite.** É a diferença entre um rádio e o
    /// `ToggleGroupMode::Single`, e o teste que impede alguém "unificar" os dois.
    #[gpui::test]
    fn clicar_no_ja_marcado_nao_desmarca_nem_reemite(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, |g| g.with_selected(Some(1)));

        vcx.simulate_click(centro_do_circulo(1), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(1), "continua marcado"));
        assert!(eventos.borrow().is_empty(), "não emitiu evento sem mudança");
    }

    /// **O RÓTULO também escolhe.** A área de clique generosa é uma promessa do doc deste módulo; se
    /// alguém mover o `on_click` da fileira pro círculo, o texto para de responder e nada mais avisa.
    #[gpui::test]
    fn o_clique_no_rotulo_escolhe(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, |g| g);

        vcx.simulate_click(sobre_o_rotulo(2), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(2), "clicar no texto marcou"));
        assert_eq!(*eventos.borrow(), vec![2]);
    }

    /// **Grupo desabilitado ignora o clique** — e o `opacity-64` não é o que impede, é o guarda no
    /// `choose` mais o fato de os handlers nem serem pendurados.
    #[gpui::test]
    fn grupo_desabilitado_ignora_o_clique(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, |g| g.disabled(true));

        vcx.simulate_click(centro_do_circulo(0), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), None, "não escolheu"));
        assert!(eventos.borrow().is_empty(), "não emitiu");
    }

    /// **As setas movem a seleção E o foco**, pulando o item desabilitado e dando a volta. A ativação
    /// automática é o comportamento do rádio nativo.
    #[gpui::test]
    fn as_setas_movem_a_selecao_e_o_foco(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, |g| g.with_selected(Some(0)));

        // Entra no grupo pelo Tab do GPUI: a única parada é o item marcado.
        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(0), "Tab caiu no marcado");
        });

        vcx.simulate_keystrokes("down");
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            let g = grupo.read(cx);
            assert_eq!(g.selected(), Some(1), "↓ marcou o próximo");
            assert_eq!(g.focused_item(window), Some(1), "e o foco foi junto");
        });

        // Dá a volta: do último pro primeiro.
        vcx.simulate_keystrokes("down down");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(0), "↓ deu a volta"));

        // E de volta pra cima.
        vcx.simulate_keystrokes("up");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(2), "↑ deu a volta"));

        // `end` e `home` vão pras pontas.
        vcx.simulate_keystrokes("home");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(0), "Home"));
        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(2), "End"));

        assert_eq!(*eventos.borrow(), vec![1, 2, 0, 2, 0, 2], "cada movimento emitiu");
    }

    /// **`Space` marca o item focado, e `Enter` NÃO.** O `Enter` fora é decisão declarada (no navegador
    /// ele submete o formulário) — este teste é o que impede alguém devolvê-lo por engano.
    #[gpui::test]
    fn espaco_marca_o_focado_e_enter_nao(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, |g| g);

        // O foco chega pelo clique (o `track_focus` faz o mouse-down focar), sem depender do Tab.
        vcx.simulate_click(centro_do_circulo(1), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(1)));

        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(1), "o foco está no 1");
        });

        // **A montagem que faz as duas teclas serem distinguíveis:** marca OUTRO item de fora, sem
        // mexer no foco. Agora o item focado (1) NÃO é o marcado (0), então qualquer tecla que "marque
        // o focado" muda o estado — e uma que não marque, não muda. Sem este passo o `enter` cairia no
        // item já marcado, o `apply_choice` recusaria por repetição, e o teste passaria mesmo com o
        // `enter` tratado (foi exatamente o mutante que sobreviveu à primeira rodada).
        vcx.update(|_window, cx| grupo.update(cx, |g, cx| g.set_selected(Some(0), cx)));
        vcx.run_until_parked();
        assert_eq!(*eventos.borrow(), vec![1], "o set_selected não emite");

        // `enter` no item focado: NADA. No navegador ele submeteria o formulário.
        vcx.simulate_keystrokes("enter");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(0), "enter não marcou o focado"));
        assert_eq!(*eventos.borrow(), vec![1], "e não emitiu");

        // `space` no MESMO item focado, com o MESMO estado: marca.
        vcx.simulate_keystrokes("space");
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(1), "space marcou o focado"));
        assert_eq!(*eventos.borrow(), vec![1, 1], "e emitiu");
    }

    /// **Há UMA parada de `Tab` pro grupo inteiro, e ela é a opção marcada.**
    ///
    /// Medido do jeito que importa: pelo [`gpui::Window::focus_next`], que é o que o `Tab` do
    /// `gpui_component::Root` chama. Se cada item fosse uma parada, o segundo `focus_next` iria pro
    /// item seguinte — e é isso que este teste proíbe.
    #[gpui::test]
    fn so_um_item_e_parada_de_tab(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir(cx, |g| g.with_selected(Some(2)));

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(2),
                "a entrada é a opção MARCADA, não a primeira"
            );
        });

        // Um segundo `Tab` não tem pra onde ir dentro do grupo: só existe uma parada.
        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(2),
                "o Tab não anda item a item dentro do grupo"
            );
        });

        // E a parada ACOMPANHA a seleção: escolher outro item move a entrada do grupo.
        vcx.simulate_keystrokes("home");
        vcx.run_until_parked();
        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(0), "a parada seguiu a seleção");
        });
    }

    /// **Um grupo sem nenhum item habilitado sai da ordem de tabulação.** Não há o que fazer lá dentro,
    /// e uma parada de `Tab` num beco sem saída é pior que nenhuma.
    #[gpui::test]
    fn grupo_desabilitado_sai_do_tab(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir(cx, |g| g.disabled(true).with_selected(Some(1)));

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

    /// **O guarda de habilitado do [`RadioGroup::choose`] recusa, mesmo chamado direto.**
    ///
    /// Pelo clique ele é redundante — a fileira desabilitada nem recebe `on_click`, e o item
    /// desabilitado nem recebe `track_focus` (então o `Space` também não o alcança). Este teste chama o
    /// `choose` **sem passar pela interface**, que é o único jeito de o guarda ser falsificável: sem ele
    /// o teste falha, e é por isso que o guarda não é código de fé.
    #[gpui::test]
    fn choose_recusa_indice_desabilitado_chamado_direto(cx: &mut TestAppContext) {
        let eventos: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();
        let window = cx.add_window(move |_window, cx| {
            let grupo = cx.new(|cx| RadioGroup::new(super::tests::itens(), cx));
            cx.subscribe(&grupo, move |_t, _g, ev: &RadioGroupEvent, _cx| match ev {
                RadioGroupEvent::Change(i) => capturados.borrow_mut().push(*i),
            })
            .detach();
            Harness { grupo }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let grupo = vcx.read(|cx| harness.read(cx).grupo.clone());

        // O índice 1 é o desabilitado de `tests::itens`.
        vcx.update(|_window, cx| grupo.update(cx, |g, cx| g.choose(1, cx)));
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), None, "o guarda recusou"));
        assert!(eventos.borrow().is_empty(), "e não emitiu");

        // O mesmo caminho com um índice HABILITADO passa — senão este teste passaria por um `choose`
        // que recusa tudo.
        vcx.update(|_window, cx| grupo.update(cx, |g, cx| g.choose(2, cx)));
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(2)));
        assert_eq!(*eventos.borrow(), vec![2]);
    }

    /// **O item desabilitado não responde ao clique, mas os irmãos continuam funcionando.**
    #[gpui::test]
    fn item_desabilitado_ignora_o_clique(cx: &mut TestAppContext) {
        let eventos: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();
        let window = cx.add_window(move |_window, cx| {
            let grupo = cx.new(|cx| RadioGroup::new(super::tests::itens(), cx));
            cx.subscribe(&grupo, move |_t, _g, ev: &RadioGroupEvent, _cx| match ev {
                RadioGroupEvent::Change(i) => capturados.borrow_mut().push(*i),
            })
            .detach();
            Harness { grupo }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let grupo = vcx.read(|cx| harness.read(cx).grupo.clone());

        // O item 1 é o desabilitado da lista de `tests::itens`.
        vcx.simulate_click(centro_do_circulo(1), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), None, "o travado não escolheu"));
        assert!(eventos.borrow().is_empty());

        // O vizinho continua vivo.
        vcx.simulate_click(centro_do_circulo(2), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(2)));
        assert_eq!(*eventos.borrow(), vec![2]);
    }

    /// **O clique NÃO acende o anel de foco; a tecla acende.** Este é o `focus-visible` da referência
    /// (ver [`crate::focus_ring`]): gatear só em `is_focused` acenderia um anel de 2px em todo clique.
    #[gpui::test]
    fn o_clique_nao_acende_o_anel_mas_a_tecla_acende(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir(cx, |g| g);

        vcx.simulate_click(centro_do_circulo(0), Modifiers::default());
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert!(
                !crate::focus_ring::visible(),
                "clicar não pode acender o anel (a referência é focus-visible)"
            );
            assert_eq!(
                grupo.read(cx).focused_item(window),
                Some(0),
                "mas o foco VAI pro item — é o que faz o teclado continuar dali"
            );
        });

        vcx.simulate_keystrokes("down");
        vcx.run_until_parked();
        assert!(crate::focus_ring::visible(), "a seta acende o anel");

        vcx.simulate_click(centro_do_circulo(0), Modifiers::default());
        vcx.run_until_parked();
        assert!(!crate::focus_ring::visible(), "voltar pro mouse apaga o anel");
    }

    /// **O indicador não intercepta o clique.** Ele cobre o círculo INTEIRO quando marcado e é pintado
    /// por cima; se criasse hitbox própria, trocar de opção depois de marcar uma seria impossível — e o
    /// meio dele é exatamente onde o usuário clica.
    #[gpui::test]
    fn o_indicador_nao_intercepta_o_clique(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir(cx, |g| g.with_selected(Some(0)));

        // Clica no MEIO do item marcado (onde está o ponto), e depois no vizinho.
        vcx.simulate_click(centro_do_circulo(0), Modifiers::default());
        vcx.run_until_parked();
        vcx.simulate_click(centro_do_circulo(1), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| {
            assert_eq!(
                grupo.read(cx).selected(),
                Some(1),
                "o clique atravessou o disco do indicador"
            );
        });
    }

    /// **A fileira não estica além do rótulo.** Sem `items_start` no grupo, o `stretch` do flex daria
    /// a cada fileira a largura do pai e uma faixa de vazio à direita do texto passaria a escolher a
    /// opção — um alvo de clique invisível de dezenas de pixels.
    #[gpui::test]
    fn a_fileira_nao_estica_alem_do_rotulo(cx: &mut TestAppContext) {
        let (grupo, eventos, mut vcx) = abrir(cx, |g| g);

        // Bem à direita de "Comedy" a 14px, mas na MESMA altura da primeira fileira.
        vcx.simulate_click(point(px(400.0), px(ROW / 2.0)), Modifiers::default());
        vcx.run_until_parked();

        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), None, "o vazio não escolhe"));
        assert!(eventos.borrow().is_empty());

        // E o rótulo, que fica ANTES do vazio, continua escolhendo — senão este teste passaria por
        // um grupo que não responde a nada.
        vcx.simulate_click(sobre_o_rotulo(0), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(0)));
    }

    /// **`RadioGroup::focus` leva o teclado pra entrada do grupo** sem quem chama saber qual dos N
    /// handles é ela.
    #[gpui::test]
    fn focus_leva_pra_parada_do_grupo(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir(cx, |g| g.with_selected(Some(1)));

        vcx.update(|window, cx| grupo.read(cx).focus(window));
        vcx.run_until_parked();
        vcx.update(|window, cx| {
            assert_eq!(grupo.read(cx).focused_item(window), Some(1), "foi pro marcado");
        });
    }

    /// **Trocar a lista refaz os vetores paralelos.** Um handle ou um relógio fora de sincronia com os
    /// itens é um `panic` de índice esperando um render — então o render depois da troca é o teste.
    #[gpui::test]
    fn trocar_a_lista_nao_deixa_vetor_fora_de_sincronia(cx: &mut TestAppContext) {
        let (grupo, _eventos, mut vcx) = abrir(cx, |g| g.with_selected(Some(2)));

        vcx.update(|_window, cx| {
            grupo.update(cx, |g, cx| {
                g.set_items(vec![RadioGroupItem::new("Só um")], cx);
            })
        });
        vcx.run_until_parked();

        vcx.read(|cx| {
            let g = grupo.read(cx);
            assert_eq!(g.items().len(), 1);
            assert_eq!(g.selected(), None, "a seleção antiga não descreve mais nada");
        });

        // E o grupo novo continua clicável.
        vcx.simulate_click(centro_do_circulo(0), Modifiers::default());
        vcx.run_until_parked();
        vcx.read(|cx| assert_eq!(grupo.read(cx).selected(), Some(0)));
    }
}

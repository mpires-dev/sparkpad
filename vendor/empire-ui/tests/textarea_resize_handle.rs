//! **A alça de redimensionar da textarea, arrastada de verdade numa janela.**
//!
//! # Por que este arquivo existe, se a matemática já tem teste unitário
//!
//! `input.rs` trava as decisões sem GPU: a conta do arraste (`altura_apos_arraste`), os limites
//! (`teto_do_arraste`), quem vê a alça (`alca_visivel`) e a precedência sobre o `height()`. O que
//! nenhum daqueles pode provar é que o **gesto** liga uma coisa na outra:
//!
//! - que a alça está de fato no canto inferior direito da moldura, e que apertar ALI abre um arraste
//!   (um `absolute` mal colocado, um `occlude` esquecido, ou um glifo que colapsou pra 0px de lado
//!   passariam batidos em qualquer `assert` de número);
//! - que o `DragMoveEvent` chega, que o `bounds` que ele carrega é a altura EXTERNA da moldura, e que
//!   o `window.refresh()` faz o quadro seguinte sair com o tamanho novo;
//! - que o arraste **vence o crescimento automático** do [`empire_ui::input::growing`] — que é a
//!   pergunta de projeto mais delicada da alça, e a única cuja resposta está no encontro entre a
//!   moldura (`h` vs `min_h`) e o miolo do núcleo (que pede altura MÍNIMA em linhas);
//! - que a alça de um campo não mexe no vizinho, nem o arraste do `ScrubInput` mexe em nenhum dos dois.
//!
//! # Como o gesto é simulado
//!
//! O GPUI só declara um arraste depois de **2px** de deslocamento com o botão apertado
//! (`DRAG_THRESHOLD` em `gpui/src/elements/div.rs`), e o listener que o declara roda na fase de
//! **bolha**, enquanto o `on_drag_move` roda na de **captura** — então o move que ABRE o arraste não
//! dispara movimento, e é do segundo move em diante que a altura muda. [`arrastar`] reproduz isso:
//! `mouse_down`, um move de 3px pra abrir, o move até o destino, e `mouse_up`.
//!
//! O passo de abertura é medido dentro do destino de propósito: é o comportamento real (o usuário
//! atravessa os 2px indo pra onde vai), e a âncora é o `mouse_down`, não o primeiro move — então os 3px
//! não somam erro nenhum ao resultado.
//!
//! # A geometria dos pontos
//!
//! O harness é o mesmo de `textarea_geometry.rs`: `p(20)` + `w(320)`, então a moldura ocupa
//! `x ∈ [20, 300]`. Com borda de 1px, a caixa de conteúdo dela começa em 21 e termina em 299; a alça é
//! um quadrado de `InputSize::icon() + 6` = **21px** encostado no canto inferior direito. Numa textarea
//! `Md` no piso (72px de altura externa, `y ∈ [20, 92]`) isso põe a alça em `x ∈ [278, 299]`,
//! `y ∈ [70, 91]` — e [`ponto_da_alca`] devolve o centro dela.

use gpui::{
    div, point, px, AppContext, Context, Entity, InteractiveElement, IntoElement, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render,
    Styled, TestAppContext, VisualTestContext, Window,
};
use gpui_component::input::InputState;
use gpui_component::Root;

use empire_ui::input::{self, InputSize, TEXTAREA_ROWS};
use empire_ui::Input;

/// Respiro em volta do campo, igual ao de `textarea_geometry.rs`.
const PAD: f32 = 20.0;
/// Largura do container do harness.
const LARGURA: f32 = 320.0;
/// Piso externo de uma textarea `Md` — o `min-h-17.5` do coss mais as duas bordas.
const PISO_MD: f32 = 72.0;
/// Teto externo do arraste no `Md`: 20 linhas de 20px + 2×5 de respiro + as duas bordas.
const TETO_MD: f32 = 412.0;

/// Como o campo do harness é montado.
#[derive(Clone, Copy, PartialEq)]
enum Forma {
    /// Textarea com o piso do tamanho — a alça aparece.
    Textarea,
    /// Textarea sem moldura (é o que o `InputGroup` hospeda) — o `resize-none` da referência.
    SemMoldura,
    /// Textarea com a alça desligada no call site.
    SemAlca,
    /// Textarea desabilitada.
    Desabilitada,
}

struct Harness {
    state: Entity<InputState>,
    forma: Forma,
    size: InputSize,
}

impl Render for Harness {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        empire_ui::theme::set_theme(empire_ui::theme::ThemeMode::Dark);
        let campo = Input::new(&self.state).size(self.size).textarea();
        let campo = match self.forma {
            Forma::Textarea => campo,
            Forma::SemMoldura => campo.unstyled(),
            Forma::SemAlca => campo.resize_none(),
            Forma::Desabilitada => campo.disabled(true),
        };
        div()
            .p(px(PAD))
            .w(px(LARGURA))
            .child(div().debug_selector(|| "campo".into()).w_full().child(campo))
    }
}

/// Abre uma janela com uma textarea que **cresce** de `TEXTAREA_ROWS` até `max_rows` linhas.
fn abrir(
    cx: &mut TestAppContext,
    forma: Forma,
    max_rows: usize,
) -> (Entity<InputState>, VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        input::init(cx);
    });
    let mut saida = None;
    let window = cx.add_window(|window, cx| {
        let state = cx.new(|cx| input::growing(TEXTAREA_ROWS, max_rows, window, cx));
        saida = Some(state.clone());
        let view = cx.new(|_cx| Harness { state, forma, size: InputSize::Md });
        Root::new(view, window, cx)
    });
    let state = saida.expect("o estado é criado no build da janela");
    let vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.run_until_parked();
    (state, vcx)
}

/// Altura EXTERNA do campo, como ela saiu no frame pintado.
fn altura(vcx: &mut VisualTestContext) -> Pixels {
    vcx.debug_bounds("campo").expect("a sonda está na árvore").size.height
}

/// Centro da alça de uma moldura que começa em `topo` e mede `altura_externa`. Ver o doc do módulo
/// pela conta.
fn ponto_da_alca_em(topo: f32, altura_externa: f32) -> Point<Pixels> {
    let alvo = InputSize::Md.icon() + 6.0;
    let direita = PAD + LARGURA - 2.0 * PAD - 1.0; // borda interna direita da moldura
    let baixo = topo + altura_externa - 1.0; // borda interna de baixo
    point(px(direita - alvo / 2.0), px(baixo - alvo / 2.0))
}

/// Centro da alça do campo único do harness, que começa em [`PAD`].
fn ponto_da_alca(altura_externa: f32) -> Point<Pixels> {
    ponto_da_alca_em(PAD, altura_externa)
}

/// Um ponto no MEIO da área de texto — longe da alça. Serve de controle negativo.
fn ponto_no_texto() -> Point<Pixels> {
    point(px(PAD + 60.0), px(PAD + 30.0))
}

/// Arrasta de `de` até `de.y + dy`, com o gesto completo que o GPUI exige (ver o doc do módulo).
fn arrastar(vcx: &mut VisualTestContext, de: Point<Pixels>, dy: f32) {
    vcx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: de,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    vcx.run_until_parked();

    // Passo de abertura: > DRAG_THRESHOLD (2px), no SENTIDO do destino.
    let abertura = point(de.x, de.y + px(3.0 * dy.signum()));
    mover(vcx, abertura);
    // E o movimento que de fato conta.
    mover(vcx, point(de.x, de.y + px(dy)));

    vcx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: point(de.x, de.y + px(dy)),
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    vcx.run_until_parked();
}

/// Um `mouse_move` com o botão esquerdo apertado — o que o GPUI lê como arraste em curso.
fn mover(vcx: &mut VisualTestContext, para: Point<Pixels>) {
    vcx.simulate_event(MouseMoveEvent {
        position: para,
        pressed_button: Some(MouseButton::Left),
        modifiers: Modifiers::default(),
    });
    vcx.run_until_parked();
}

/// Escreve no campo pelo estado (o caminho que atualiza o modo de crescimento do núcleo).
fn escrever(state: &Entity<InputState>, vcx: &mut VisualTestContext, texto: &str) {
    let texto = texto.to_string();
    vcx.update(|window, cx| {
        state.update(cx, |st, cx| st.set_value(texto.clone(), window, cx));
    });
    vcx.run_until_parked();
}

/// `n` linhas curtas — curtas o bastante pra não quebrarem na largura do harness.
fn linhas(n: usize) -> String {
    (0..n).map(|i| format!("l{i}\n")).collect::<String>().trim_end().to_string()
}

// =================================================================================================
// O gesto
// =================================================================================================

/// **Arrastar a alça pra baixo aumenta a caixa, 1px de mouse por 1px de altura.**
///
/// É o teste de fumaça da peça inteira: alça no lugar certo, arraste abrindo, `DragMoveEvent`
/// chegando, altura escrita, quadro novo pintado. Se qualquer elo estiver frouxo, ele cai.
#[gpui::test]
fn arrastar_a_alca_pra_baixo_aumenta_a_caixa(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Forma::Textarea, 8);
    assert_eq!(altura(&mut vcx), px(PISO_MD), "começa no piso do coss");

    arrastar(&mut vcx, ponto_da_alca(PISO_MD), 40.0);
    assert_eq!(altura(&mut vcx), px(112.0), "72 + 40: a caixa segue o dedo");
}

/// **Arrastar pra cima para no piso do coss.** A alça é um superset, não uma licença pra furar a
/// geometria da referência: nem 71px, nem zero.
#[gpui::test]
fn arrastar_pra_cima_para_no_piso_do_coss(cx: &mut TestAppContext) {
    // Sobe primeiro, pra haver curso de sobra pra descer.
    let (_s, mut vcx) = abrir(cx, Forma::Textarea, 8);
    arrastar(&mut vcx, ponto_da_alca(PISO_MD), 120.0);
    assert_eq!(altura(&mut vcx), px(192.0));

    // Agora 500px pra cima — muito mais do que o curso disponível.
    arrastar(&mut vcx, ponto_da_alca(192.0), -500.0);
    assert_eq!(altura(&mut vcx), px(PISO_MD), "o piso do tamanho segura");
}

/// **O teto segura.** Um arraste absurdo para em [`TETO_MD`] — sem ele a alça sairia da tela junto com
/// o fim da caixa, e o usuário ficaria sem como voltar.
#[gpui::test]
fn o_teto_segura_o_arraste(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Forma::Textarea, 8);
    arrastar(&mut vcx, ponto_da_alca(PISO_MD), 5_000.0);
    assert_eq!(altura(&mut vcx), px(TETO_MD), "20 linhas, e para");
}

// =================================================================================================
// O conflito com o crescimento automático
// =================================================================================================

/// **O arraste vence o crescimento automático — e continua vencendo depois.**
///
/// É a decisão de projeto mais delicada da alça: com [`empire_ui::input::growing`] a caixa acompanha o
/// texto, e no instante em que o usuário arrasta ela passa a obedecer a ele. Intenção explícita ganha
/// de automática.
///
/// O teste é em três atos, e é o terceiro que importa: não basta o arraste mudar a altura uma vez — o
/// texto que chega DEPOIS não pode desfazê-lo. É ali que a diferença entre `h` e `min_h` na moldura
/// aparece, e é o único lugar em que ela aparece.
#[gpui::test]
fn o_arraste_vence_o_crescimento_automatico(cx: &mut TestAppContext) {
    let (state, mut vcx) = abrir(cx, Forma::Textarea, 8);

    // Ato 1: o crescimento automático funcionando — 6 linhas levantam a caixa a 132.
    escrever(&state, &mut vcx, &linhas(6));
    assert_eq!(altura(&mut vcx), px(132.0), "6×20 + 2×5 + 2, pelo `growing`");

    // Ato 2: o usuário arrasta pra CIMA, contra o que o conteúdo pediria. O arraste ganha.
    arrastar(&mut vcx, ponto_da_alca(132.0), -40.0);
    assert_eq!(
        altura(&mut vcx),
        px(92.0),
        "132 - 40: o arraste vence o crescimento, e o texto passa a rolar por dentro"
    );

    // Ato 3: mais texto NÃO desfaz o arraste. É o que separa `h` de `min_h`.
    escrever(&state, &mut vcx, &linhas(8));
    assert_eq!(
        altura(&mut vcx),
        px(92.0),
        "o teto do `growing` pediria 172; a caixa arrastada não se mexe mais"
    );

    // E menos texto também não: a caixa é do usuário agora, nos dois sentidos.
    escrever(&state, &mut vcx, "uma linha só");
    assert_eq!(altura(&mut vcx), px(92.0), "o piso do `growing` pediria 72");
}

// =================================================================================================
// Onde a alça NÃO está
// =================================================================================================

/// **Apertar no meio do texto e arrastar não redimensiona nada.**
///
/// Controle negativo do `absolute`/`occlude`: se a alça estivesse esticada pela moldura (um
/// `bottom_0 right_0` sem `size`, por exemplo), este arraste também a pegaria — e um clique pra
/// posicionar o cursor viraria um redimensionamento.
#[gpui::test]
fn arrastar_no_meio_do_texto_nao_redimensiona(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Forma::Textarea, 8);
    arrastar(&mut vcx, ponto_no_texto(), 60.0);
    assert_eq!(altura(&mut vcx), px(PISO_MD), "só a alça redimensiona");
}

/// **Sem moldura não há alça — é o `resize-none` do `input-group`, e ele sai de graça.**
///
/// O coss escreve `**:[textarea]:resize-none` dentro do `input-group`, que é exatamente onde o campo
/// entra `unstyled`. Aqui a alça é peça da moldura, então tirar a moldura tira a alça: o mesmo
/// resultado, sem uma segunda regra pra manter.
///
/// A altura sem moldura é o MIOLO (70, não 72), então o ponto de pega é calculado com ela.
#[gpui::test]
fn sem_moldura_nao_ha_alca(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Forma::SemMoldura, 8);
    assert_eq!(altura(&mut vcx), px(70.0), "sem moldura o campo mede o miolo");
    arrastar(&mut vcx, ponto_da_alca(70.0), 60.0);
    assert_eq!(altura(&mut vcx), px(70.0), "no `input-group` não há o que arrastar");
}

/// **`resize_none()` desliga a alça** — o desligamento explícito, pra a tela que quer uma caixa
/// multi-linha de altura imutável mas COM a moldura da casa.
#[gpui::test]
fn resize_none_desliga_a_alca(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Forma::SemAlca, 8);
    arrastar(&mut vcx, ponto_da_alca(PISO_MD), 60.0);
    assert_eq!(altura(&mut vcx), px(PISO_MD));
}

/// **Um campo desabilitado não redimensiona.** Quem não aceita edição não aceita gesto de geometria —
/// e uma alça viva num campo apagado é a pior das duas leituras possíveis.
#[gpui::test]
fn campo_desabilitado_nao_redimensiona(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Forma::Desabilitada, 8);
    arrastar(&mut vcx, ponto_da_alca(PISO_MD), 60.0);
    assert_eq!(altura(&mut vcx), px(PISO_MD));
}

// =================================================================================================
// Duas textareas na mesma tela
// =================================================================================================

/// Harness com DOIS campos empilhados, cada um com a própria alça.
struct DoisCampos {
    a: Entity<InputState>,
    b: Entity<InputState>,
}

impl Render for DoisCampos {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        empire_ui::theme::set_theme(empire_ui::theme::ThemeMode::Dark);
        div()
            .p(px(PAD))
            .w(px(LARGURA))
            .flex()
            .flex_col()
            .gap(px(0.0))
            .child(
                div()
                    .debug_selector(|| "a".into())
                    .w_full()
                    .child(Input::new(&self.a).textarea()),
            )
            .child(
                div()
                    .debug_selector(|| "b".into())
                    .w_full()
                    .child(Input::new(&self.b).textarea()),
            )
    }
}

/// **Arrastar a alça de um campo não mexe no vizinho — nem depois de o vizinho já ter sido
/// arrastado.**
///
/// O `on_drag_move` do GPUI casa por TIPO, então a moldura do campo B recebe o MESMO evento que a do
/// campo A e tem que se calar. Quem a cala é o `dono` no payload do arraste.
///
/// # A ordem dos atos não é enfeite
///
/// Uma primeira versão deste teste arrastava só o campo A e conferia o B — e **passava com a checagem
/// de dono removida**, porque o B nunca tinha sido arrastado e portanto não tinha âncora, e a falta de
/// âncora já o fazia sair. O teste era verde sem provar nada.
///
/// Aqui o B é arrastado PRIMEIRO. A partir daí ele tem âncora e altura de gesto guardadas, e a única
/// coisa que o impede de acompanhar o arraste do A é o `dono`. É a diferença entre um teste que descreve
/// o cenário fácil e um que mata a mutação.
#[gpui::test]
fn a_alca_de_um_campo_nao_mexe_no_vizinho(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        input::init(cx);
    });
    let window = cx.add_window(|window, cx| {
        let a = cx.new(|cx| input::growing(TEXTAREA_ROWS, 8, window, cx));
        let b = cx.new(|cx| input::growing(TEXTAREA_ROWS, 8, window, cx));
        let view = cx.new(|_cx| DoisCampos { a, b });
        Root::new(view, window, cx)
    });
    let mut vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.run_until_parked();

    let de = |vcx: &mut VisualTestContext, sel: &'static str| {
        vcx.debug_bounds(sel).expect("a sonda está na árvore").size.height
    };
    assert_eq!(de(&mut vcx, "a"), px(PISO_MD));
    assert_eq!(de(&mut vcx, "b"), px(PISO_MD));

    // Ato 1: o campo B é arrastado. Ele passa a ter âncora e altura de gesto na tabela.
    arrastar(&mut vcx, ponto_da_alca_em(PAD + PISO_MD, PISO_MD), 30.0);
    assert_eq!(de(&mut vcx, "b"), px(102.0), "o campo B cresceu");
    assert_eq!(de(&mut vcx, "a"), px(PISO_MD), "e o A não se mexeu");

    // Ato 2: agora o campo A. O B tem âncora velha guardada, e só o `dono` o segura.
    arrastar(&mut vcx, ponto_da_alca(PISO_MD), 40.0);
    assert_eq!(de(&mut vcx, "a"), px(112.0), "o campo A cresceu");
    assert_eq!(
        de(&mut vcx, "b"),
        px(102.0),
        "o campo B recebeu o mesmo `DragMoveEvent` e ignorou: o dono não é ele"
    );
}

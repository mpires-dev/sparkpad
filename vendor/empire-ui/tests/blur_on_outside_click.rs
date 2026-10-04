//! **Clicar fora tira o foco** — no [`empire_ui::Input`] e no [`empire_ui::Select`].
//!
//! O GPUI não desfoca nada sozinho: um campo focado continua focado, e com o anel aceso, mesmo depois
//! de um clique no vazio. No navegador não é assim.
//!
//! Estes testes existem porque a implementação ingênua (desfocar dentro do próprio
//! `on_mouse_down_out`) passa no caso óbvio e **quebra o caso comum**: clicar do campo A direto no
//! campo B, onde o mesmo evento que tira o foco do A é o que dá foco ao B. Por isso o caso
//! `do_campo_a_pro_campo_b_o_b_fica_focado` importa mais que o primeiro teste do arquivo.
//!
//! No `Select` há três cliques que parecem o mesmo e não são: fora de tudo (fecha e desfoca), no
//! próprio gatilho (fecha e MANTÉM o foco) e numa opção (escolhe, fecha e MANTÉM o foco). Cada um tem
//! um teste, porque a diferença entre eles é justamente onde um handler mal colocado erra.

use gpui::{
    div, point, px, AppContext, Context, Entity, Focusable, IntoElement, Modifiers, MouseButton,
    MouseDownEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, Styled, TestAppContext,
    VisualTestContext, Window,
};
use gpui_component::{input::InputState, Root};

use empire_ui::{input, Input, Select};

/// Respiro em volta dos controles no harness. Ele é o "vazio": um clique aqui é clique fora de tudo,
/// e é grande o suficiente pra caber com folga em qualquer coordenada que os testes usem.
const PAD: f32 = 40.0;
/// Largura de cada controle.
const LARGURA: f32 = 240.0;

// =================================================================================================
// Harness de dois campos de texto
// =================================================================================================

struct DoisCampos {
    a: Entity<InputState>,
    b: Entity<InputState>,
}

impl Render for DoisCampos {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        empire_ui::theme::set_theme(empire_ui::theme::ThemeMode::Dark);
        // Empilhados com um vão generoso: dá pra clicar no vão sem acertar nenhum dos dois.
        div()
            .p(px(PAD))
            .flex()
            .flex_col()
            .gap(px(PAD))
            .child(Input::new(&self.a))
            .child(Input::new(&self.b))
    }
}

fn dois_campos(cx: &mut TestAppContext) -> (Entity<InputState>, Entity<InputState>, VisualTestContext)
{
    cx.update(|cx| {
        gpui_component::init(cx);
        input::init(cx);
    });
    let mut saida = None;
    let window = cx.add_window(|window, cx| {
        let a = cx.new(|cx| input::single_line(window, cx));
        let b = cx.new(|cx| input::single_line(window, cx));
        saida = Some((a.clone(), b.clone()));
        let view = cx.new(|_cx| DoisCampos { a, b });
        Root::new(view, window, cx)
    });
    let (a, b) = saida.expect("estados criados");
    let vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.run_until_parked();
    (a, b, vcx)
}

// =================================================================================================
// Utilitários
// =================================================================================================

fn clicar(vcx: &mut VisualTestContext, p: Point<Pixels>) {
    vcx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: p,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    vcx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: p,
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    // DUAS vezes: a checagem de desfoque é adiada um frame de propósito (ver
    // `empire_ui::input::blur_on_outside_click`), então um `run_until_parked` só pode observar o
    // estado ANTES dela rodar. Se este teste passasse com uma chamada só, ele não estaria testando o
    // caminho adiado.
    vcx.run_until_parked();
    vcx.run_until_parked();
}

fn focado(state: &Entity<InputState>, vcx: &mut VisualTestContext) -> bool {
    vcx.update(|window, cx| state.read(cx).focus_handle(cx).is_focused(window))
}

/// Um ponto no respiro, longe de qualquer controle.
fn no_vazio() -> Point<Pixels> {
    point(px(PAD / 2.0), px(PAD / 2.0))
}

// =================================================================================================
// Input
// =================================================================================================

/// **O caso pedido:** campo focado, clique no vazio, foco vai embora.
#[gpui::test]
fn clicar_no_vazio_desfoca_o_campo(cx: &mut TestAppContext) {
    let (a, _b, mut vcx) = dois_campos(cx);

    vcx.update(|window, cx| a.read(cx).focus_handle(cx).focus(window));
    vcx.run_until_parked();
    assert!(focado(&a, &mut vcx), "o campo começa focado");

    clicar(&mut vcx, no_vazio());
    assert!(!focado(&a, &mut vcx), "clicar no vazio tinha que desfocar");
}

/// **O caso que a implementação ingênua quebra.**
///
/// Clicar do campo A direto no B: o mesmo evento que dispara o `on_mouse_down_out` do A é o que dá
/// foco ao B. Desfocando de dentro do handler, o A apagaria o foco que o B acabou de ganhar e o
/// clique não focaria nada.
#[gpui::test]
fn do_campo_a_pro_campo_b_o_b_fica_focado(cx: &mut TestAppContext) {
    let (a, b, mut vcx) = dois_campos(cx);

    vcx.update(|window, cx| a.read(cx).focus_handle(cx).focus(window));
    vcx.run_until_parked();
    assert!(focado(&a, &mut vcx));

    // O centro do segundo campo: dois controles empilhados com `gap` e `p` iguais a PAD.
    let alvo = point(px(PAD + LARGURA / 2.0), px(PAD + 30.0 + PAD + 15.0));
    clicar(&mut vcx, alvo);

    assert!(focado(&b, &mut vcx), "o B tem que ficar focado");
    assert!(!focado(&a, &mut vcx), "e o A tem que ter perdido o foco");
}

/// Clicar **no próprio campo** não pode desfocá-lo — o `on_mouse_down_out` não dispara ali, e este
/// teste é o que garante que o handler não foi pendurado num elemento errado (um pai que não cobre o
/// campo faria o clique contar como "fora").
#[gpui::test]
fn clicar_no_proprio_campo_mantem_o_foco(cx: &mut TestAppContext) {
    let (a, _b, mut vcx) = dois_campos(cx);

    let dentro = point(px(PAD + LARGURA / 2.0), px(PAD + 15.0));
    clicar(&mut vcx, dentro);
    assert!(focado(&a, &mut vcx), "clicar dentro foca e mantém");

    clicar(&mut vcx, dentro);
    assert!(focado(&a, &mut vcx), "e clicar de novo não desfoca");
}

// =================================================================================================
// Select
// =================================================================================================

struct ComSelect {
    select: Entity<Select>,
}

impl Render for ComSelect {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        empire_ui::theme::set_theme(empire_ui::theme::ThemeMode::Dark);
        div()
            .p(px(PAD))
            .child(div().w(px(LARGURA)).child(self.select.clone()))
    }
}

fn com_select(cx: &mut TestAppContext) -> (Entity<Select>, VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        input::init(cx);
    });
    let mut saida = None;
    let window = cx.add_window(|window, cx| {
        let opcoes: Vec<gpui::SharedString> = ["Um", "Dois", "Três"]
            .iter()
            .map(|s| gpui::SharedString::from(*s))
            .collect();
        let select = cx.new(|cx| Select::new(opcoes, 0, cx));
        saida = Some(select.clone());
        let view = cx.new(|_cx| ComSelect { select });
        Root::new(view, window, cx)
    });
    let select = saida.expect("select criado");
    let vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.run_until_parked();
    (select, vcx)
}

fn select_focado(select: &Entity<Select>, vcx: &mut VisualTestContext) -> bool {
    vcx.update(|window, cx| {
        select.read(cx).focus_handle(cx).is_focused(window)
    })
}

/// O centro do gatilho no harness.
fn no_gatilho() -> Point<Pixels> {
    point(px(PAD + LARGURA / 2.0), px(PAD + 16.0))
}

/// **O caso pedido:** menu aberto, clique fora — fecha **e** desfoca. Era o defeito relatado: fechava
/// e o gatilho ficava com o anel aceso.
#[gpui::test]
fn clicar_fora_com_o_menu_aberto_fecha_e_desfoca(cx: &mut TestAppContext) {
    let (select, mut vcx) = com_select(cx);

    clicar(&mut vcx, no_gatilho());
    assert!(vcx.read(|cx| select.read(cx).is_open()), "abriu");
    assert!(select_focado(&select, &mut vcx), "e está focado");

    clicar(&mut vcx, no_vazio());
    assert!(!vcx.read(|cx| select.read(cx).is_open()), "fechou");
    assert!(!select_focado(&select, &mut vcx), "e desfocou");
}

/// Com o menu **fechado**, clicar fora também desfoca — é o outro caminho, e passa por um handler
/// diferente (o do gatilho, não o do popup).
#[gpui::test]
fn clicar_fora_com_o_menu_fechado_desfoca(cx: &mut TestAppContext) {
    let (select, mut vcx) = com_select(cx);

    // Abre e fecha pelo próprio gatilho: sobra focado, sem menu.
    clicar(&mut vcx, no_gatilho());
    clicar(&mut vcx, no_gatilho());
    assert!(!vcx.read(|cx| select.read(cx).is_open()), "fechado");
    assert!(select_focado(&select, &mut vcx), "mas ainda focado");

    clicar(&mut vcx, no_vazio());
    assert!(!select_focado(&select, &mut vcx), "clicar fora desfoca");
}

/// **Clicar no próprio gatilho pra fechar MANTÉM o foco.**
///
/// Este é o caso que separa uma implementação certa de uma quase-certa: o handler do popup dispara
/// pra qualquer clique fora do POPUP, e o gatilho está fora do popup. Sem testar os bounds do
/// gatilho, fechar o menu no próprio gatilho desfocaria o controle.
#[gpui::test]
fn fechar_pelo_gatilho_mantem_o_foco(cx: &mut TestAppContext) {
    let (select, mut vcx) = com_select(cx);

    clicar(&mut vcx, no_gatilho());
    assert!(vcx.read(|cx| select.read(cx).is_open()));

    clicar(&mut vcx, no_gatilho());
    assert!(!vcx.read(|cx| select.read(cx).is_open()), "fechou");
    assert!(
        select_focado(&select, &mut vcx),
        "e o foco FICA: o clique não foi fora de tudo"
    );
}

/// **Escolher uma opção mantém o foco.** No navegador um `<select>` continua focado depois da
/// escolha; e a opção também está "fora do gatilho", então é o segundo caso em que um handler
/// desatento desfocaria.
#[gpui::test]
fn escolher_uma_opcao_mantem_o_foco(cx: &mut TestAppContext) {
    let (select, mut vcx) = com_select(cx);

    clicar(&mut vcx, no_gatilho());
    assert!(vcx.read(|cx| select.read(cx).is_open()));

    // O menu abre logo abaixo do gatilho; o primeiro item fica dentro dessa faixa.
    let primeira_opcao = point(px(PAD + LARGURA / 2.0), px(PAD + 32.0 + 4.0 + 14.0));
    clicar(&mut vcx, primeira_opcao);

    assert!(!vcx.read(|cx| select.read(cx).is_open()), "escolher fecha");
    assert!(
        select_focado(&select, &mut vcx),
        "e o foco fica no gatilho, como num <select> do navegador"
    );
}

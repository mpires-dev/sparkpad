//! O botão de **mostrar/esconder senha** do [`empire_ui::Input`] tem que ser um TOGGLE.
//!
//! O botão do núcleo (`gpui_component::input::Input::mask_toggle`) é press-and-hold: revela no
//! mouse-down e volta a esconder no mouse-up. O nosso alterna a cada clique — e é isso que estes
//! testes travam, porque a diferença entre os dois só aparece DEPOIS de soltar o botão.

use gpui::{
    div, point, px, AppContext, Context, Entity, IntoElement, Modifiers, MouseButton,
    MouseDownEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, Styled, TestAppContext,
    VisualTestContext, Window,
};
use gpui_component::{input::InputState, Root};

use empire_ui::{input, Input};

/// Largura do container do campo no harness.
const LARGURA: f32 = 400.0;
/// Padding do container.
const PAD: f32 = 20.0;

struct Harness {
    state: Entity<InputState>,
    disabled: bool,
}

impl Render for Harness {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        empire_ui::theme::set_theme(empire_ui::theme::ThemeMode::Dark);
        div().w(px(LARGURA)).p(px(PAD)).child(
            Input::new(&self.state)
                .disabled(self.disabled)
                .mask_toggle(),
        )
    }
}

fn open(cx: &mut TestAppContext, disabled: bool) -> (Entity<InputState>, VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        input::init(cx);
    });

    let mut saida: Option<Entity<InputState>> = None;
    let window = cx.add_window(|window, cx| {
        let state = cx.new(|cx| {
            input::single_line(window, cx)
                .default_value("s3nh4-secreta")
                .masked(true)
        });
        saida = Some(state.clone());
        let view = cx.new(|_cx| Harness { state, disabled });
        Root::new(view, window, cx)
    });

    let state = saida.expect("estado criado no build da janela");
    let vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.run_until_parked();
    (state, vcx)
}

/// Centro do botão do olho.
///
/// Ele é o último item do conteúdo, então encosta na borda direita menos o respiro horizontal:
/// `PAD` do container + 1px de borda + 10px de `pad_x` do tamanho Md, e o botão tem ~21px de lado.
fn olho() -> Point<Pixels> {
    let borda_direita = PAD + LARGURA - 2.0 * PAD; // o campo ocupa a largura útil
    point(px(borda_direita - 1.0 - 10.0 - 10.0), px(36.0))
}

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
    vcx.run_until_parked();
}

fn mascarado(state: &Entity<InputState>, vcx: &mut VisualTestContext) -> bool {
    vcx.read(|cx| state.read(cx).is_masked())
}

/// **O clique COMPLETO revela — e continua revelado.** É a diferença central em relação ao botão
/// do núcleo: lá o mouse-up voltava a mascarar, então era preciso manter o clique pressionado.
#[gpui::test]
fn um_clique_revela_e_a_senha_continua_visivel(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, false);
    assert!(mascarado(&state, &mut vcx), "começa mascarado");

    clicar(&mut vcx, olho());

    assert!(
        !mascarado(&state, &mut vcx),
        "depois de UM clique completo (com o botão já solto) a senha tem que seguir visível"
    );
}

/// **O segundo clique esconde de novo** — alterna, não é uma via de mão única.
#[gpui::test]
fn segundo_clique_esconde_de_novo(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, false);

    clicar(&mut vcx, olho());
    assert!(!mascarado(&state, &mut vcx), "1º clique revela");

    clicar(&mut vcx, olho());
    assert!(mascarado(&state, &mut vcx), "2º clique esconde");

    clicar(&mut vcx, olho());
    assert!(!mascarado(&state, &mut vcx), "3º clique revela de novo");
}

/// **Campo desabilitado não alterna.** Um campo apagado que ainda revela senha ao clique seria
/// tanto um bug visual quanto de expectativa.
#[gpui::test]
fn campo_desabilitado_nao_alterna(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, true);

    clicar(&mut vcx, olho());

    assert!(
        mascarado(&state, &mut vcx),
        "campo desabilitado tem que continuar mascarado"
    );
}

/// **Clicar no texto não alterna a máscara.** Protege contra o botão do olho ter área de clique
/// maior do que aparenta (ou o handler estar no container inteiro em vez do botão).
#[gpui::test]
fn clicar_no_texto_nao_alterna(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, false);

    clicar(&mut vcx, point(px(PAD + 30.0), px(36.0))); // começo do texto

    assert!(
        mascarado(&state, &mut vcx),
        "clique no texto não pode revelar a senha"
    );
}

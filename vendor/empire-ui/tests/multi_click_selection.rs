//! Verificação **de ponta a ponta** da seleção por múltiplos cliques do [`empire_ui::Input`].
//!
//! Os testes unitários do módulo cobrem a *decisão* (quantos cliques → o que selecionar). O que
//! eles não cobrem é a **fiação**: o evento de mouse chega ao nosso handler? a ação despachada
//! acerta o campo focado? o `defer` roda depois do handler do núcleo (que colapsa a seleção no 3º
//! clique)? Nada disso é observável sem uma janela e eventos de verdade.
//!
//! Aqui abrimos uma janela headless (`VisualTestContext`) e forjamos os eventos com o
//! `click_count` exato, o que a API de conveniência (`simulate_mouse_down`, que fixa
//! `click_count: 1`) não permite.
//!
//! **Como a seleção é observada:** o `InputState` não expõe o range selecionado publicamente, mas
//! expõe `value()`. Então usamos a própria semântica de edição como sonda: **digitar substitui a
//! seleção**. Se depois de 3 cliques o texto todo estava selecionado, digitar `X` deixa o campo
//! com exatamente `"X"`. É uma asserção mais forte que ler o range — mede o efeito que o usuário
//! sente, não o estado interno.

use gpui::{
    div, point, px, AppContext, Context, Entity, IntoElement, Modifiers, MouseButton,
    MouseDownEvent, MouseUpEvent, ParentElement, Render, Styled, TestAppContext, VisualTestContext,
    Window,
};
use gpui_component::{input::InputState, Root};

use empire_ui::{input, Input};

/// View mínima que hospeda um campo — é o que a janela de teste renderiza.
struct Harness {
    state: Entity<InputState>,
    rows: Option<usize>,
    disabled: bool,
}

impl Render for Harness {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        empire_ui::theme::set_theme(empire_ui::theme::ThemeMode::Dark);
        let mut field = Input::new(&self.state).disabled(self.disabled);
        if let Some(r) = self.rows {
            field = field.rows(r);
        }
        // Largura generosa e sem label/hint, pra a geometria do clique ser previsível: o campo
        // começa no topo do container.
        div().w(px(400.0)).p(px(20.0)).child(field)
    }
}

/// Abre a janela de teste com um campo já preenchido. `rows = Some(n)` faz uma textarea.
///
/// A janela é montada com o `Root` do `gpui-component` por dentro: o `Input` do núcleo depende
/// dele pras camadas de popover/modal, e sem ele o paint entra em pânico
/// (`root.rs: called Option::unwrap() on a None value`) — foi o que derrubou a 1ª versão
/// destes testes.
fn open(
    cx: &mut TestAppContext,
    valor: &'static str,
    rows: Option<usize>,
) -> (Entity<InputState>, VisualTestContext) {
    open_with(cx, valor, rows, false)
}

/// Idem, com controle de `disabled` (usado pelo teste do campo desabilitado).
fn open_with(
    cx: &mut TestAppContext,
    valor: &'static str,
    rows: Option<usize>,
    disabled: bool,
) -> (Entity<InputState>, VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        input::init(cx);
    });

    // O `Entity<InputState>` é criado dentro da janela, mas precisamos dele aqui fora pra ler o
    // valor — então sai pela captura.
    let mut saida: Option<Entity<InputState>> = None;
    let window = cx.add_window(|window, cx| {
        let state = cx.new(|cx| match rows {
            Some(n) => input::multi_line(n, window, cx).default_value(valor),
            None => input::single_line(window, cx).default_value(valor),
        });
        saida = Some(state.clone());
        let view = cx.new(|_cx| Harness {
            state,
            rows,
            disabled,
        });
        Root::new(view, window, cx)
    });

    let state = saida.expect("o InputState é criado no build da janela");
    let vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.run_until_parked();
    (state, vcx)
}

/// Um ponto seguramente **dentro** do texto do campo: o container tem 20px de padding, a moldura
/// mais ~10px, e a altura do campo é 32px (tamanho Md) — então (60, 36) cai na primeira linha.
fn dentro_do_campo() -> gpui::Point<gpui::Pixels> {
    point(px(60.0), px(36.0))
}

/// Forja uma sequência de cliques no mesmo ponto, com `click_count` crescente — é o que o sistema
/// operacional entrega em cliques rápidos, e o que a API de conveniência do gpui não expõe.
fn clicar(vcx: &mut VisualTestContext, pos: gpui::Point<gpui::Pixels>, vezes: usize) {
    for n in 1..=vezes {
        vcx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: pos,
            modifiers: Modifiers::default(),
            click_count: n,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: pos,
            modifiers: Modifiers::default(),
            click_count: n,
        });
    }
    vcx.run_until_parked();
}

/// Valor corrente do campo.
fn valor(state: &Entity<InputState>, vcx: &mut VisualTestContext) -> String {
    vcx.read(|cx| state.read(cx).value().to_string())
}

// =================================================================================================

/// **1 clique = caret.** Digitar depois de um clique simples INSERE, não substitui — se algo
/// tivesse sido selecionado por engano, o texto original desapareceria.
#[gpui::test]
fn um_clique_apenas_posiciona_o_cursor(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "palavra outra", None);

    clicar(&mut vcx, dentro_do_campo(), 1);
    vcx.simulate_input("X");

    let v = valor(&state, &mut vcx);
    assert!(
        v.len() > 1,
        "1 clique não pode selecionar nada — digitar deveria INSERIR, mas o campo virou {v:?}"
    );
    assert!(
        v.contains("outra"),
        "o resto do texto tem que sobreviver a 1 clique + digitação; ficou {v:?}"
    );
}

/// **2 cliques = palavra.** Digitar troca SÓ a palavra clicada; o resto do texto continua lá.
/// Este comportamento já vinha do núcleo — o teste existe pra ele não regredir quando mexermos
/// no nosso handler (que roda no mesmo evento).
#[gpui::test]
fn dois_cliques_selecionam_a_palavra(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "primeira segunda", None);

    clicar(&mut vcx, dentro_do_campo(), 2);
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "X segunda",
        "2 cliques têm que trocar SÓ a palavra clicada"
    );
}

/// **3 cliques = tudo (campo de uma linha).** É o comportamento que faltava: o núcleo sozinho
/// colapsa a seleção no 3º clique, e o campo ficaria só com o caret.
#[gpui::test]
fn tres_cliques_selecionam_tudo_em_campo_de_uma_linha(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "um dois tres quatro", None);

    clicar(&mut vcx, dentro_do_campo(), 3);
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "X",
        "3 cliques deviam selecionar TODO o conteúdo, então digitar substitui tudo"
    );
}

/// **4+ cliques mantêm a seleção do 3º.** Um clique nervoso a mais não pode desfazer o que o
/// usuário acabou de selecionar.
#[gpui::test]
fn quatro_cliques_continuam_selecionando_tudo(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "um dois tres quatro", None);

    clicar(&mut vcx, dentro_do_campo(), 5);
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "X",
        "do 4º clique em diante o comportamento do 3º tem que se manter"
    );
}

/// **3 cliques numa textarea = a LINHA, não tudo.** É a diferença que o navegador faz entre
/// `<input>` e `<textarea>`, e o erro fácil de cometer aqui.
#[gpui::test]
fn tres_cliques_em_textarea_pegam_so_a_linha(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "linha um\nlinha dois\nlinha tres", Some(4));

    // O ponto cai na PRIMEIRA linha da textarea.
    clicar(&mut vcx, dentro_do_campo(), 3);
    vcx.simulate_input("X");

    // Asserção EXATA, e não por substring: a versão anterior deste teste só checava que
    // "linha um" tinha saído e as outras sobrado, e por isso PASSAVA mesmo com a seleção
    // começando no meio da linha (o bug do `MoveToStartOfLine` no-op). O valor completo é a
    // única forma de provar que a seleção foi do INÍCIO ao FIM da linha.
    assert_eq!(
        valor(&state, &mut vcx),
        "X\nlinha dois\nlinha tres",
        "o 3º clique tem que trocar a linha INTEIRA e só ela"
    );
}

/// **Campo desabilitado não seleciona nada.** Sem a guarda de `disabled`, o triplo clique
/// despacharia `SelectAll` pro elemento focado — que poderia ser outro campo da tela.
#[gpui::test]
fn campo_desabilitado_nao_reage_a_multiplos_cliques(cx: &mut TestAppContext) {
    let (state, mut vcx) = open_with(cx, "nao mexe", None, true);

    clicar(&mut vcx, dentro_do_campo(), 3);
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "nao mexe",
        "campo desabilitado não pode aceitar seleção nem digitação"
    );
}

/// **`cmd-a` (ou `ctrl-a`) continua selecionando tudo.** O usuário confirmou que isto já
/// funcionava; o teste garante que o nosso handler de mouse não atropelou o caminho de teclado.
#[gpui::test]
fn atalho_de_selecionar_tudo_continua_funcionando(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "um dois tres", None);

    // Um clique pra focar, e então o atalho.
    clicar(&mut vcx, dentro_do_campo(), 1);
    #[cfg(target_os = "macos")]
    vcx.simulate_keystrokes("cmd-a");
    #[cfg(not(target_os = "macos"))]
    vcx.simulate_keystrokes("ctrl-a");
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "X",
        "selecionar-tudo pelo teclado tem que substituir todo o conteúdo"
    );
}

/// **Recortar e colar** fazem a volta completa pelo clipboard, com a seleção vinda de 3 cliques.
/// Cobre de uma vez: triplo clique seleciona tudo → `cut` limpa → `paste` devolve.
#[gpui::test]
fn recortar_e_colar_apos_tres_cliques(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "conteudo inteiro", None);

    clicar(&mut vcx, dentro_do_campo(), 3);
    #[cfg(target_os = "macos")]
    let (cut, paste) = ("cmd-x", "cmd-v");
    #[cfg(not(target_os = "macos"))]
    let (cut, paste) = ("ctrl-x", "ctrl-v");

    vcx.simulate_keystrokes(cut);
    assert_eq!(
        valor(&state, &mut vcx),
        "",
        "recortar com tudo selecionado tem que esvaziar o campo"
    );

    vcx.simulate_keystrokes(paste);
    assert_eq!(
        valor(&state, &mut vcx),
        "conteudo inteiro",
        "colar tem que devolver exatamente o que foi recortado"
    );
}

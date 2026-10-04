//! **Sondas de paridade com o Chrome** para a seleção por mouse do [`empire_ui::Input`].
//!
//! Cada teste aqui codifica uma regra observável do `SelectionController` do Blink (o controlador
//! de seleção do Chromium) e verifica se o nosso campo se comporta igual. As regras testadas:
//!
//! | Regra do Blink | Teste |
//! |---|---|
//! | triplo clique = granularidade de **parágrafo** (entre `\n`), não de linha visual | [`triplo_clique_pega_o_paragrafo_da_linha_clicada`] |
//! | duplo clique **não** engole o espaço à direita (comportamento macOS) | [`duplo_clique_nao_engole_o_espaco_seguinte`] |
//! | arrastar depois do duplo clique estende **por palavra** | [`arrastar_depois_do_duplo_clique_estende_por_palavra`] |
//! | shift+clique depois do duplo clique estende **por palavra** | [`shift_clique_depois_do_duplo_clique_estende_por_palavra`] |
//! | duplo clique sobre texto selecionado seleciona a palavra | [`duplo_clique_sobre_texto_selecionado_seleciona_a_palavra`] |
//! | a seleção sobrevive ao mouse-**down** de um clique dentro dela | [`selecao_sobrevive_ao_mouse_down_dentro_dela`] |
//! | o caret é posto no mouse-**up**, se não houve arraste | [`clique_completo_dentro_da_selecao_poe_o_caret_no_mouse_up`] |
//!
//! Como sempre neste crate, a seleção é observada pelo EFEITO: digitar substitui a seleção, então
//! o `value()` final prova o que estava selecionado.
//!
//! As três últimas regras eram lacunas até termos o fork local do `gpui-component`
//! (`vendor/gpui-component`, ver o `PATCHES.md` de lá): dependem do estado de seleção do
//! `InputState`, que é privado. Hoje todas passam — e este arquivo é a rede que impede o patch de
//! regredir num upgrade da dependência.

use gpui::{
    div, point, px, AppContext, Context, Entity, IntoElement, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, Styled,
    TestAppContext, VisualTestContext, Window,
};
use gpui_component::{input::InputState, Root};

use empire_ui::{input, Input};

struct Harness {
    state: Entity<InputState>,
    rows: Option<usize>,
}

impl Render for Harness {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        empire_ui::theme::set_theme(empire_ui::theme::ThemeMode::Dark);
        let mut field = Input::new(&self.state);
        if let Some(r) = self.rows {
            field = field.rows(r);
        }
        div().w(px(600.0)).p(px(20.0)).child(field)
    }
}

fn open(
    cx: &mut TestAppContext,
    valor: &'static str,
    rows: Option<usize>,
) -> (Entity<InputState>, VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        input::init(cx);
    });

    let mut saida: Option<Entity<InputState>> = None;
    let window = cx.add_window(|window, cx| {
        let state = cx.new(|cx| match rows {
            Some(n) => input::multi_line(n, window, cx).default_value(valor),
            None => input::single_line(window, cx).default_value(valor),
        });
        saida = Some(state.clone());
        let view = cx.new(|_cx| Harness { state, rows });
        Root::new(view, window, cx)
    });

    let state = saida.expect("estado criado no build da janela");
    let vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.run_until_parked();
    (state, vcx)
}

/// Origem do texto: 20px de padding do container + 1px de borda + 11px de `pad_x` do tamanho Md.
const TEXTO_X0: f32 = 32.0;
/// Altura de uma linha de textarea = o `leading` que mandamos pro núcleo, que num multi-linha é o par
/// do Tailwind pro `text-sm` (14/**20**), e não a altura do miolo. Ver `input::TEXTAREA_LINE_HEIGHT`.
///
/// (Num campo de UMA linha o `leading` é a altura do miolo — 30 no Md —, mas ali só existe uma linha e
/// esta constante não é usada pra sair dela.)
const LINHA_H: f32 = 20.0;
/// Centro vertical da primeira linha: 20px de padding do container + 1px de borda + 5px de respiro
/// vertical da textarea no Md (`py-[calc(--spacing(1.5)-1px)]`) + meia linha.
const LINHA_1_Y: f32 = 36.0;

/// Um ponto na linha `linha` (0-based), aproximadamente no caractere `col`.
///
/// A largura por caractere é uma aproximação da fonte do sistema no corpo Md — os textos usados
/// nos testes têm palavras longas de propósito, pra o ponto cair com folga DENTRO da palavra
/// pretendida mesmo com alguns pixels de erro.
fn pos(col: f32, linha: usize) -> Point<Pixels> {
    // Aproximação da fonte do sistema a 14px. Os textos dos testes usam palavras LONGAS de
    // propósito, então alguns pixels de erro aqui não mudam em qual palavra o ponto cai.
    const LARGURA_CHAR: f32 = 7.6;
    point(
        px(TEXTO_X0 + col * LARGURA_CHAR),
        px(LINHA_1_Y + linha as f32 * LINHA_H),
    )
}

fn clicar(vcx: &mut VisualTestContext, p: Point<Pixels>, vezes: usize) {
    for n in 1..=vezes {
        vcx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: p,
            modifiers: Modifiers::default(),
            click_count: n,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: p,
            modifiers: Modifiers::default(),
            click_count: n,
        });
    }
    vcx.run_until_parked();
}

/// Duplo clique em `de` e, com o botão ainda pressionado, arraste até `ate`.
fn duplo_clique_e_arrastar(vcx: &mut VisualTestContext, de: Point<Pixels>, ate: Point<Pixels>) {
    for n in 1..=2 {
        vcx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: de,
            modifiers: Modifiers::default(),
            click_count: n,
            first_mouse: false,
        });
        if n == 1 {
            vcx.simulate_event(MouseUpEvent {
                button: MouseButton::Left,
                position: de,
                modifiers: Modifiers::default(),
                click_count: n,
            });
        }
    }
    // Arrasta com o botão pressionado (o 2º mouse-down não teve up).
    vcx.simulate_event(MouseMoveEvent {
        position: ate,
        pressed_button: Some(MouseButton::Left),
        modifiers: Modifiers::default(),
    });
    vcx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: ate,
        modifiers: Modifiers::default(),
        click_count: 2,
    });
    vcx.run_until_parked();
}

/// Clique com Shift pressionado (estende a seleção corrente).
fn shift_clique(vcx: &mut VisualTestContext, p: Point<Pixels>) {
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    vcx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: p,
        modifiers: shift,
        click_count: 1,
        first_mouse: false,
    });
    vcx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: p,
        modifiers: shift,
        click_count: 1,
    });
    vcx.run_until_parked();
}

/// Seleciona tudo pelo atalho da plataforma.
fn selecionar_tudo(vcx: &mut VisualTestContext) {
    #[cfg(target_os = "macos")]
    vcx.simulate_keystrokes("cmd-a");
    #[cfg(not(target_os = "macos"))]
    vcx.simulate_keystrokes("ctrl-a");
}

fn valor(state: &Entity<InputState>, vcx: &mut VisualTestContext) -> String {
    vcx.read(|cx| state.read(cx).value().to_string())
}

// =================================================================================================

/// Blink usa granularidade de **parágrafo** no triplo clique (`HandleTripleClick`), e num
/// `<textarea>` parágrafo = trecho entre `\n`. Confere que o triplo clique pega a linha CLICADA
/// (aqui a segunda), não a primeira nem tudo.
#[gpui::test]
fn triplo_clique_pega_o_paragrafo_da_linha_clicada(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "primeira linha\nsegunda linha\nterceira linha", Some(4));

    clicar(&mut vcx, pos(4.0, 1), 3);
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "primeira linha\nX\nterceira linha",
        "o triplo clique tem que trocar SÓ o parágrafo da linha clicada"
    );
}

/// No macOS o Chrome **não** engole o espaço depois da palavra no duplo clique (o
/// `AppendTrailingWhitespace` do Blink é ligado por `IsSelectTrailingWhitespaceEnabled()`, que é
/// comportamento de Windows). Então o espaço tem que sobreviver.
#[gpui::test]
fn duplo_clique_nao_engole_o_espaco_seguinte(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "primeiro segundo", None);

    clicar(&mut vcx, pos(3.0, 0), 2);
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "X segundo",
        "o espaço depois da palavra tem que sobreviver (comportamento macOS)"
    );
}

/// Blink mantém a granularidade durante o arraste (`UpdateSelectionForMouseDrag` usa
/// `Selection().Granularity()`), então arrastar depois de um duplo clique cresce **de palavra em
/// palavra**, não de caractere em caractere.
///
/// **Lacuna conhecida.** O núcleo tem um `selected_word_range`, mas ele só IMPEDE que a seleção
/// encolha abaixo da palavra do duplo clique — não arredonda a ponta que se move pra fronteira da
/// palavra. Medido: arrastar até o meio de `bbbb` seleciona `aaaa b`, e não `aaaa bbbb`.
/// A mesma limitação está no HEAD do upstream (mesmo mecanismo de clamp), então não é questão de
/// atualizar a dependência — fechar isso exige ser dono do código de seleção do núcleo.
#[gpui::test]
fn arrastar_depois_do_duplo_clique_estende_por_palavra(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "aaaaaaaa bbbbbbbb cccccccc", None);

    // Duplo clique em "aaaa", arrasta até o MEIO de "bbbb".
    duplo_clique_e_arrastar(&mut vcx, pos(4.0, 0), pos(13.0, 0));
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "X cccccccc",
        "arrastar após duplo clique tem que engolir a palavra INTEIRA sob o cursor"
    );
}

/// Mesma regra da granularidade, agora pelo shift+clique (`HandleSingleClick` do Blink usa
/// `Selection().Granularity()` ao estender).
///
/// **Lacuna conhecida:** o núcleo limpa o `selected_word_range` no `mouse_up`, então a
/// granularidade de palavra morre ao soltar o botão — o shift+clique seguinte estende por
/// caractere. Corrigir exige o estado de seleção do núcleo, que é `pub(super)`.
#[gpui::test]
fn shift_clique_depois_do_duplo_clique_estende_por_palavra(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "aaaaaaaa bbbbbbbb cccccccc", None);

    clicar(&mut vcx, pos(4.0, 0), 2); // seleciona "aaaa"
    shift_clique(&mut vcx, pos(13.0, 0)); // estende até o meio de "bbbb"
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "X cccccccc",
        "shift+clique após duplo clique tem que estender por PALAVRA"
    );
}

/// **Duplo clique sobre texto já selecionado seleciona a PALAVRA.**
///
/// Cuidado com a leitura da fonte do Blink aqui: o `HandleDoubleClick` tem um retorno-cedo quando
/// a seleção já é um range, mas ele não vale pra este gesto — o mouse-up do 1º clique já colapsou
/// a seleção (regra do caret pendente, testada abaixo), então o 2º clique encontra seleção vazia.
/// O comportamento observável do Chrome é selecionar a palavra clicada.
#[gpui::test]
fn duplo_clique_sobre_texto_selecionado_seleciona_a_palavra(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "aaaaaaaa bbbbbbbb cccccccc", None);

    clicar(&mut vcx, pos(4.0, 0), 1);
    selecionar_tudo(&mut vcx);

    clicar(&mut vcx, pos(13.0, 0), 2); // duplo clique sobre a seleção
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "aaaaaaaa X cccccccc",
        "duplo clique sobre texto selecionado seleciona a palavra clicada"
    );
}

/// **A seleção sobrevive ao mouse-DOWN de um clique dentro dela.**
///
/// É o `mouse_down_was_single_click_in_selection_` do Blink: apertar o botão sobre um texto
/// selecionado não pode desfazer a seleção, porque o gesto ainda pode virar um arraste — ou o 2º
/// clique de um duplo clique. Sem esta regra, o 1º clique de um duplo clique colapsava a seleção.
///
/// Observado sem soltar o botão: se a seleção ainda está inteira, digitar substitui tudo.
#[gpui::test]
fn selecao_sobrevive_ao_mouse_down_dentro_dela(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "aaaaaaaa bbbbbbbb cccccccc", None);

    clicar(&mut vcx, pos(4.0, 0), 1);
    selecionar_tudo(&mut vcx);

    // Só o mouse-DOWN, sem soltar.
    vcx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: pos(13.0, 0),
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    vcx.run_until_parked();
    vcx.simulate_input("X");

    assert_eq!(
        valor(&state, &mut vcx),
        "X",
        "a seleção tem que sobreviver ao mouse-down pra o gesto poder virar arraste ou duplo clique"
    );
}

/// **O caret é posicionado no mouse-UP**, se o clique dentro da seleção não virou arraste. É o que
/// faz "clicar num texto selecionado" desfazer a seleção e deixar o cursor ali.
#[gpui::test]
fn clique_completo_dentro_da_selecao_poe_o_caret_no_mouse_up(cx: &mut TestAppContext) {
    let (state, mut vcx) = open(cx, "aaaaaaaa bbbbbbbb cccccccc", None);

    clicar(&mut vcx, pos(4.0, 0), 1);
    selecionar_tudo(&mut vcx);

    clicar(&mut vcx, pos(13.0, 0), 1); // clique completo (down + up)
    vcx.simulate_input("X");

    let v = valor(&state, &mut vcx);
    assert_ne!(
        v, "X",
        "ao soltar, a seleção tem que ser desfeita e o caret posicionado — digitar INSERE"
    );
    assert!(
        v.contains("aaaaaaaa") && v.contains("cccccccc") && v.contains('X'),
        "o texto tem que sobreviver com o X inserido no ponto clicado; ficou {v:?}"
    );
}

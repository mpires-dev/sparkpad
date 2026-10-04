//! **A geometria da textarea, medida na janela** — piso por tamanho, piso por linhas, crescimento
//! com o conteúdo e o campo sem moldura.
//!
//! # Por que medir em vez de afirmar a fórmula
//!
//! Os números da referência (`min-h-17.5` = 70, `py` = 5, entrelinha 20) já estão travados nos testes
//! unitários de `input.rs`. O que aqueles testes **não** podem provar é que a moldura, o miolo e o
//! elemento de texto do núcleo concordam: o piso pode estar certo na constante e a caixa sair errada
//! porque a altura foi declarada no lugar errado da cadeia (moldura → conteúdo → wrapper → núcleo),
//! porque um `h_full()` venceu um `min_h`, ou porque a entrelinha que chegou ao núcleo não era a que
//! a moldura usou pra calcular o piso. Todos esses erros já aconteceram neste componente.
//!
//! Então aqui a altura é lida do frame RENDERIZADO, via `debug_bounds`. O harness embrulha o campo num
//! `div` de altura automática, cuja caixa é exatamente a do campo (um campo sem label nem linha de
//! apoio tem um filho só, então não há `gap` a descontar).
//!
//! # O que cada número prova
//!
//! - **72 / 68 / 76** — o piso externo dos três tamanhos (`min-h-*` + as duas bordas).
//! - **132 com seis linhas de texto** — o crescimento é de 20 em 20. Este é o teste que trava a
//!   ENTRELINHA de verdade: com o default do GPUI (22,65) seriam ~148, e com a entrelinha de um campo
//!   de uma linha (30) seriam 192. Nenhum piso esconde esse erro, porque a caixa aqui está ACIMA do
//!   piso.
//! - **172 com doze linhas e teto de oito** — o teto do [`empire_ui::input::growing`] segura.
//! - **72 com seis linhas num estado [`empire_ui::input::multi_line`]** — a caixa parada rola por
//!   dentro em vez de crescer, que é a outra metade da decisão.
//!
//! # E um teste que não mede pixel nenhum
//!
//! `toda_linha_da_caixa_aceita_clique` clica em cada linha e confere onde o caret caiu. Ele está aqui
//! porque foi ele que pegou o pior modo de falha desta mudança: com a moldura em `min_h`, o miolo do
//! modo "texto puro" do núcleo pedia UMA linha de altura e contava com o pai — que, dimensionado por
//! `min_h`, não resolve porcentagem de filho. A caixa media 92px certinhos, o texto aparecia inteiro, e
//! só a primeira linha aceitava clique. Nenhuma medida de altura pega isso.

use gpui::{
    div, point, px, AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement,
    Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement, Pixels, Point, Render,
    Styled, TestAppContext, VisualTestContext, Window,
};
use gpui_component::input::InputState;
use gpui_component::Root;

use empire_ui::input::{self, InputSize, TEXTAREA_ROWS};
use empire_ui::Input;

/// Respiro em volta do campo no harness. Só existe pra o campo não colar na borda da janela.
const PAD: f32 = 20.0;

/// Como o campo do harness é montado. Cada variante é um pedido diferente ao componente.
#[derive(Clone, Copy)]
enum Forma {
    /// Campo de uma linha (a referência de controle: nada aqui deve tê-lo mudado).
    UmaLinha,
    /// Textarea com o piso do tamanho — `Input::textarea()`.
    PisoDoTamanho,
    /// Textarea com piso de `n` linhas — `Input::rows(n)`.
    Linhas(usize),
    /// Textarea com o piso do tamanho, **sem moldura** — `Input::textarea().unstyled()`.
    SemMoldura,
    /// Campo de uma linha sem moldura.
    UmaLinhaSemMoldura,
}

struct Harness {
    state: Entity<InputState>,
    forma: Forma,
    size: InputSize,
}

impl Render for Harness {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        empire_ui::theme::set_theme(empire_ui::theme::ThemeMode::Dark);
        let campo = Input::new(&self.state).size(self.size);
        let campo = match self.forma {
            Forma::UmaLinha => campo,
            Forma::PisoDoTamanho => campo.textarea(),
            Forma::Linhas(n) => campo.rows(n),
            Forma::SemMoldura => campo.textarea().unstyled(),
            Forma::UmaLinhaSemMoldura => campo.unstyled(),
        };
        div().p(px(PAD)).w(px(320.0)).child(
            // O `div` sonda: altura automática, então mede exatamente a caixa do campo.
            div()
                .debug_selector(|| "campo".into())
                .w_full()
                .child(campo),
        )
    }
}

/// Como o ESTADO é criado — a outra metade da decisão de altura (ver o doc do módulo).
#[derive(Clone, Copy)]
enum Estado {
    UmaLinha,
    /// Caixa parada, com `rows` linhas.
    Parada(usize),
    /// Caixa que cresce, de `min_rows` até `max_rows`.
    Cresce(usize, usize),
}

fn abrir(
    cx: &mut TestAppContext,
    estado: Estado,
    forma: Forma,
    size: InputSize,
) -> (Entity<InputState>, VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        input::init(cx);
    });
    let mut saida = None;
    let window = cx.add_window(|window, cx| {
        let state = cx.new(|cx| match estado {
            Estado::UmaLinha => input::single_line(window, cx),
            Estado::Parada(rows) => input::multi_line(rows, window, cx),
            Estado::Cresce(min, max) => input::growing(min, max, window, cx),
        });
        saida = Some(state.clone());
        let view = cx.new(|_cx| Harness { state, forma, size });
        Root::new(view, window, cx)
    });
    let state = saida.expect("o estado é criado no build da janela");
    let vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.run_until_parked();
    (state, vcx)
}

/// Altura EXTERNA do campo, como ela saiu no frame pintado.
fn altura(vcx: &mut VisualTestContext) -> Pixels {
    vcx.debug_bounds("campo")
        .expect("a sonda está na árvore")
        .size
        .height
}

/// Escreve no campo pelo estado (é o caminho que atualiza o modo de crescimento do núcleo).
fn escrever(state: &Entity<InputState>, vcx: &mut VisualTestContext, texto: &str) {
    let texto = texto.to_string();
    vcx.update(|window, cx| {
        state.update(cx, |st, cx| st.set_value(texto.clone(), window, cx));
    });
    vcx.run_until_parked();
}

/// `n` linhas curtas — curtas o bastante pra caberem sem quebra na largura do harness, então o
/// número de linhas quebradas é exatamente `n`.
fn linhas(n: usize) -> String {
    (0..n).map(|i| format!("l{i}\n")).collect::<String>().trim_end().to_string()
}

/// Um ponto no começo da linha `linha` (0-based) de uma textarea `Md`: 20 de respiro do harness + 1 de
/// borda + 5 de respiro vertical, mais meia entrelinha pra cair no MEIO da linha. O `x` fica logo
/// depois da primeira coluna.
fn ponto_na_linha(linha: usize) -> Point<Pixels> {
    point(px(34.0), px(20.0 + 1.0 + 5.0 + 10.0 + linha as f32 * 20.0))
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

/// Em que linha o caret está, escrevendo um `X` e vendo onde ele caiu.
fn linha_do_caret(state: &Entity<InputState>, vcx: &mut VisualTestContext) -> usize {
    vcx.simulate_input("X");
    vcx.run_until_parked();
    let valor = vcx.read(|cx| state.read(cx).value().to_string());
    let linha = valor
        .lines()
        .position(|l| l.contains('X'))
        .expect("o X foi escrito em alguma linha");
    // Desfaz, pra a próxima medição partir do mesmo texto.
    let limpo = valor.replace('X', "");
    vcx.update(|window, cx| {
        state.update(cx, |st, cx| st.set_value(limpo.clone(), window, cx));
    });
    vcx.run_until_parked();
    linha
}

// =================================================================================================
// O piso, por tamanho
// =================================================================================================

/// **O piso do `Md`: 72px** — `min-h-17.5` (70) + as duas bordas de 1px.
///
/// É o número da referência chegando à tela. Se a altura fosse declarada como `h` em vez de `min_h`,
/// este teste passaria igual — é o de crescimento que separa os dois.
#[gpui::test]
fn piso_do_md_e_setenta_e_dois(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Estado::Parada(TEXTAREA_ROWS), Forma::PisoDoTamanho, InputSize::Md);
    assert_eq!(altura(&mut vcx), px(72.0));
}

/// **O piso do `Sm`: 68px** — `min-h-16.5` (66) + 2. O respiro menor (3) é o que o encurta.
#[gpui::test]
fn piso_do_sm_e_sessenta_e_oito(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Estado::Parada(TEXTAREA_ROWS), Forma::PisoDoTamanho, InputSize::Sm);
    assert_eq!(altura(&mut vcx), px(68.0));
}

/// **O piso do `Lg`: 76px** — `min-h-18.5` (74) + 2. Mesmas três linhas, respiro de 7.
#[gpui::test]
fn piso_do_lg_e_setenta_e_seis(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Estado::Parada(TEXTAREA_ROWS), Forma::PisoDoTamanho, InputSize::Lg);
    assert_eq!(altura(&mut vcx), px(76.0));
}

/// **`rows(3)` é o piso da referência.** É o reencontro dos dois modelos de altura: pedir "três
/// linhas" e pedir "o `min-h` do tamanho" dá a MESMA caixa.
#[gpui::test]
fn rows_tres_da_a_mesma_caixa_que_o_piso_do_tamanho(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(
        cx,
        Estado::Parada(TEXTAREA_ROWS),
        Forma::Linhas(TEXTAREA_ROWS),
        InputSize::Md,
    );
    assert_eq!(altura(&mut vcx), px(72.0));
}

/// **`rows(n)` dá uma caixa de `n` linhas de 20px.** Seis linhas = 6×20 + 2×5 + 2 = **132**.
///
/// Aqui o número já não pode sair do `min-h` da referência: ele só fecha com a entrelinha 20 e o
/// respiro 5. É um total — moldura e estado pedem as mesmas 6 linhas, e quem isola o piso da moldura é
/// `o_piso_da_moldura_vence_um_estado_que_pede_menos`.
#[gpui::test]
fn rows_seis_mede_cento_e_trinta_e_dois(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Estado::Parada(6), Forma::Linhas(6), InputSize::Md);
    assert_eq!(altura(&mut vcx), px(132.0));
}

/// **O piso da MOLDURA vence um estado que pede menos** — nos dois jeitos de declarar piso — e
/// continua sendo piso, não altura.
///
/// Este é o teste que faz o `min_h` da moldura valer alguma coisa, e ele existe porque a
/// mutação-verificação mostrou que os outros não fazem: quando o estado e o elemento pedem o MESMO
/// número de linhas, o miolo sozinho já produz a altura certa, e um piso errado (ou ausente) segue
/// verde. `textarea_min_height + 2 → + 1` passou em todos os testes de piso até este existir.
///
/// Aqui os dois DISCORDAM de propósito: o estado cresce a partir de uma linha (ou de três), o elemento
/// pede o piso do tamanho (ou seis linhas), e só a moldura pode segurar o número.
///
/// A última parte é o que distingue piso de altura: acima dele, quem manda é o conteúdo.
#[gpui::test]
fn o_piso_da_moldura_vence_um_estado_que_pede_menos(cx: &mut TestAppContext) {
    // Piso pelo TAMANHO (`Input::textarea()`), com um estado que só pediria 32px.
    let (_s, mut vcx) = abrir(cx, Estado::Cresce(1, 10), Forma::PisoDoTamanho, InputSize::Md);
    assert_eq!(
        altura(&mut vcx),
        px(72.0),
        "o estado pediria 32 (1 linha); o min-h do tamanho vence"
    );

    // Piso por LINHAS (`Input::rows(6)`), com um estado que só pediria 72px.
    let (state, mut vcx) = abrir(
        cx,
        Estado::Cresce(TEXTAREA_ROWS, 10),
        Forma::Linhas(6),
        InputSize::Md,
    );
    assert_eq!(
        altura(&mut vcx),
        px(132.0),
        "o estado pediria 72 (3 linhas); o piso de 6 linhas vence"
    );

    // E acima do piso o conteúdo manda — é `min_h`, não `h`.
    escrever(&state, &mut vcx, &linhas(8));
    assert_eq!(altura(&mut vcx), px(172.0));
}

/// Um campo de UMA linha não mudou: 32px no `Md` (miolo 30 + as duas bordas). Está aqui porque a
/// entrelinha e a altura passaram a ter dois ramos, e um erro no ramo do textarea é fácil de espalhar
/// pro outro.
#[gpui::test]
fn campo_de_uma_linha_continua_com_trinta_e_dois(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Estado::UmaLinha, Forma::UmaLinha, InputSize::Md);
    assert_eq!(altura(&mut vcx), px(32.0));
}

// =================================================================================================
// `field-sizing-content`
// =================================================================================================

/// **A caixa cresce de 20 em 20, do piso até o teto** — o `field-sizing-content` da referência.
///
/// É também o teste que trava a ENTRELINHA na tela: 6 linhas dão 132 só se cada linha medir 20. Com o
/// default do GPUI (22,65) daria ~148; com a entrelinha de um campo de uma linha (30), 192.
#[gpui::test]
fn textarea_que_cresce_sobe_de_20_em_20_ate_o_teto(cx: &mut TestAppContext) {
    let (state, mut vcx) = abrir(
        cx,
        Estado::Cresce(TEXTAREA_ROWS, 8),
        Forma::Linhas(TEXTAREA_ROWS),
        InputSize::Md,
    );

    // Vazia: o piso, igual à caixa parada.
    assert_eq!(altura(&mut vcx), px(72.0), "vazia, o piso");

    // Dentro do piso não cresce: três linhas ainda são o piso.
    escrever(&state, &mut vcx, &linhas(3));
    assert_eq!(altura(&mut vcx), px(72.0), "3 linhas cabem no piso");

    // A partir da quarta, cada linha vale uma entrelinha.
    escrever(&state, &mut vcx, &linhas(4));
    assert_eq!(altura(&mut vcx), px(92.0), "4 linhas = 4×20 + 2×5 + 2");
    escrever(&state, &mut vcx, &linhas(6));
    assert_eq!(altura(&mut vcx), px(132.0), "6 linhas = 6×20 + 2×5 + 2");

    // No teto, para. O texto excedente rola por dentro.
    escrever(&state, &mut vcx, &linhas(8));
    assert_eq!(altura(&mut vcx), px(172.0), "8 linhas, o teto");
    escrever(&state, &mut vcx, &linhas(20));
    assert_eq!(altura(&mut vcx), px(172.0), "20 linhas: o teto segura");
}

/// **A caixa PARADA não cresce** — o texto rola por dentro. É a outra metade da decisão: quem quer o
/// `field-sizing-content` pede [`empire_ui::input::growing`]; quem quer o `<textarea>` clássico pede
/// [`empire_ui::input::multi_line`].
#[gpui::test]
fn textarea_parada_nao_cresce_com_o_conteudo(cx: &mut TestAppContext) {
    let (state, mut vcx) = abrir(
        cx,
        Estado::Parada(TEXTAREA_ROWS),
        Forma::Linhas(TEXTAREA_ROWS),
        InputSize::Md,
    );
    assert_eq!(altura(&mut vcx), px(72.0));
    escrever(&state, &mut vcx, &linhas(12));
    assert_eq!(altura(&mut vcx), px(72.0), "a caixa fica; o texto rola");
}

// =================================================================================================
// A caixa toda é clicável
// =================================================================================================

/// **Cada linha da caixa aceita clique, e o caret cai NA linha clicada** — nos dois modos de estado.
///
/// É o teste que pega a falha que uma medida de altura não pega: a caixa com a altura certa, o texto
/// todo desenhado e só a primeira linha viva. Ver o doc do módulo, e o porquê no doc de
/// [`empire_ui::input::multi_line`].
#[gpui::test]
fn toda_linha_da_caixa_aceita_clique(cx: &mut TestAppContext) {
    for (nome, estado) in [
        ("parada", Estado::Parada(4)),
        ("cresce", Estado::Cresce(4, 8)),
    ] {
        let (state, mut vcx) = abrir(cx, estado, Forma::Linhas(4), InputSize::Md);
        vcx.update(|window, cx| {
            state.update(cx, |st, cx| st.set_value(linhas(4), window, cx));
        });
        vcx.run_until_parked();

        for linha in 0..4 {
            clicar(&mut vcx, ponto_na_linha(linha));
            assert_eq!(
                linha_do_caret(&state, &mut vcx),
                linha,
                "{nome}: o clique na linha {linha} tem que pôr o caret nela"
            );
        }
    }
}

// =================================================================================================
// `unstyled`
// =================================================================================================

/// **Sem moldura, o campo mede o MIOLO** — vão-se as duas bordas de 1px e os 2px que elas
/// acrescentavam: 70 numa textarea `Md`, 30 num campo de uma linha.
///
/// É o que a referência faz no `unstyled`: o `min-h`/`h` sempre foi do miolo, e o par de bordas era do
/// wrapper — que no `input-group` passa a ser de quem hospeda.
#[gpui::test]
fn sem_moldura_o_campo_mede_o_miolo(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Estado::Parada(TEXTAREA_ROWS), Forma::SemMoldura, InputSize::Md);
    assert_eq!(altura(&mut vcx), px(70.0), "textarea sem moldura = o piso do miolo");
}

/// O mesmo no campo de uma linha: 30 em vez de 32.
#[gpui::test]
fn sem_moldura_o_campo_de_uma_linha_mede_trinta(cx: &mut TestAppContext) {
    let (_s, mut vcx) = abrir(cx, Estado::UmaLinha, Forma::UmaLinhaSemMoldura, InputSize::Md);
    assert_eq!(altura(&mut vcx), px(30.0));
}

/// **Sem moldura o campo ainda cresce.** O `unstyled` mexe na superfície, não na geometria vertical —
/// se ele desligasse o crescimento, um textarea dentro de um grupo ficaria preso no piso.
#[gpui::test]
fn sem_moldura_a_textarea_continua_crescendo(cx: &mut TestAppContext) {
    let (state, mut vcx) = abrir(
        cx,
        Estado::Cresce(TEXTAREA_ROWS, 8),
        Forma::SemMoldura,
        InputSize::Md,
    );
    assert_eq!(altura(&mut vcx), px(70.0));
    escrever(&state, &mut vcx, &linhas(6));
    assert_eq!(altura(&mut vcx), px(130.0), "6×20 + 2×5, sem as bordas");
}

/// **Sem moldura, o clique fora NÃO desfoca** — e com moldura, desfoca.
///
/// Não é detalhe: sem moldura, "fora do campo" deixa de querer dizer "fora do controle". Num grupo o
/// clique cai no respiro do wrapper ou num addon, que são o MESMO controle, e desfocar ali seria um
/// bug visível (o anel apagando quando o usuário clica na própria caixa). Então o handler sai do campo
/// e passa a ser de quem hospeda, com `input::blur_on_outside_click` na superfície dele.
#[gpui::test]
fn sem_moldura_o_desfoque_por_clique_fora_e_de_quem_hospeda(cx: &mut TestAppContext) {
    // Com moldura: o clique no vazio desfoca (o comportamento de sempre).
    let (state, mut vcx) = abrir(cx, Estado::UmaLinha, Forma::UmaLinha, InputSize::Md);
    vcx.update(|window, cx| state.update(cx, |st, cx| st.focus(window, cx)));
    vcx.run_until_parked();
    assert!(focado(&state, &mut vcx), "com moldura: focado depois de focar");
    clicar(&mut vcx, point(px(160.0), px(300.0)));
    assert!(!focado(&state, &mut vcx), "com moldura: o clique no vazio desfoca");

    // Sem moldura: o mesmo clique NÃO desfoca — quem decide isso é o hospedeiro.
    let (state, mut vcx) = abrir(cx, Estado::UmaLinha, Forma::UmaLinhaSemMoldura, InputSize::Md);
    vcx.update(|window, cx| state.update(cx, |st, cx| st.focus(window, cx)));
    vcx.run_until_parked();
    assert!(focado(&state, &mut vcx), "sem moldura: focado depois de focar");
    clicar(&mut vcx, point(px(160.0), px(300.0)));
    assert!(
        focado(&state, &mut vcx),
        "sem moldura: o campo NÃO se desfoca sozinho — o handler é de quem hospeda"
    );
}

/// O campo está focado?
fn focado(state: &Entity<InputState>, vcx: &mut VisualTestContext) -> bool {
    vcx.update(|window, cx| state.read(cx).focus_handle(cx).is_focused(window))
}

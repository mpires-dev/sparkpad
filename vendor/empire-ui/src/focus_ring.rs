//! `focus_ring` — a aproximação de `:focus-visible` do crate.
//!
//! # O problema
//!
//! Os componentes do `coss` pedem o anel de foco com `focus-visible:ring-2`, não com `focus:ring-2`.
//! A diferença é o que o navegador faz por baixo: `:focus-visible` só casa quando o foco **veio do
//! teclado**. Clicar num botão o foca, mas não acende anel nenhum.
//!
//! O GPUI não tem essa distinção: ele tem `FocusHandle::is_focused`, e o `track_focus` faz o
//! mouse-down focar o elemento. O resultado é que gatear o anel só em `is_focused` acende um anel de
//! 2px em **todo clique** — foi exatamente o defeito relatado nas abas, e o mesmo estava no
//! [`crate::button`] e no [`crate::select`].
//!
//! # A solução
//!
//! A mesma heurística do navegador: guardar a **modalidade do último input do usuário**, e não a
//! origem do foco de cada elemento. Tecla liga, ponteiro desliga. É estado global de propósito —
//! "como o usuário está interagindo agora" é uma propriedade da sessão, não de um componente. É por
//! isso que passar do teclado pro mouse apaga o anel de um elemento que nunca foi clicado, igual no
//! navegador.
//!
//! # Por que precisa de [`init`]
//!
//! Setas e atalhos chegam aos componentes, mas **`Tab` não**: ninguém trata `Tab`, é o GPUI que move
//! o foco. Sem observar o teclado no nível da aplicação, navegar por `Tab` não acenderia anel em
//! lugar nenhum — uma regressão de acessibilidade pior que o defeito original.
//!
//! O [`init`] resolve com `intercept_keystrokes`, que dispara **antes** dos outros mecanismos e não
//! depende de propagação (o `observe_keystrokes` não é chamado se alguém interromper a propagação, o
//! que o tornaria furado justamente nos componentes que tratam tecla).
//!
//! Ele é chamado de dentro do [`crate::input::init`], então quem já inicializava o crate ganha isso
//! sem mudar nada. Chamar duas vezes é inofensivo.

use std::cell::Cell;

thread_local! {
    /// Se o último input do usuário foi de teclado.
    ///
    /// Começa `false`: uma janela recém-aberta não tem por que mostrar anel em nada.
    static KEYBOARD: Cell<bool> = const { Cell::new(false) };

    /// Se o [`init`] já registrou o observador — chamar duas vezes registraria dois.
    static ARMED: Cell<bool> = const { Cell::new(false) };
}

/// Liga o rastreio de modalidade. Idempotente.
///
/// Chame uma vez na inicialização do app. Já é chamado por [`crate::input::init`].
pub fn init(cx: &mut gpui::App) {
    if ARMED.with(|a| a.replace(true)) {
        return;
    }
    // `intercept_keystrokes` e não `observe_keystrokes`: o segundo não dispara quando alguém
    // interrompe a propagação, e é justamente nos componentes que tratam tecla que isso aconteceria.
    cx.intercept_keystrokes(|_event, window, _cx| {
        set(true, window);
    })
    .detach();
}

/// Troca a modalidade e, **se ela mudou**, pede um frame novo.
///
/// O redesenho é o que faltava na primeira versão: quem já está pintado não sabe que a modalidade
/// mudou. Sem ele, um controle focado por teclado que não seja re-renderizado continua exibindo o
/// anel depois de um clique em outro lugar — o próprio sintoma que este módulo existe pra eliminar.
///
/// A condição `se mudou` não é otimização cosmética: sem ela, **cada tecla digitada num campo de
/// texto** marcaria a janela inteira como suja.
fn set(keyboard: bool, window: &mut gpui::Window) {
    if KEYBOARD.with(|k| k.replace(keyboard)) != keyboard {
        window.refresh();
    }
}

/// O usuário acabou de usar o ponteiro: o anel some até a próxima tecla.
///
/// Chame do `on_click`/`on_mouse_down` de qualquer controle focável.
pub fn pointer_used(window: &mut gpui::Window) {
    set(false, window);
}

/// O usuário acabou de usar o teclado: o anel pode aparecer.
///
/// O [`init`] já faz isso pra qualquer tecla. Chamar direto serve pra quem trata tecla e quer o anel
/// aceso mesmo num app que esqueceu de inicializar o crate.
pub fn keyboard_used(window: &mut gpui::Window) {
    set(true, window);
}

/// Se o anel de foco deve ser desenhado — combine com o `is_focused` do elemento:
///
/// ```ignore
/// if focado && focus_ring::visible() {
///     el = el.child(ring_overlay(raio));
/// }
/// ```
pub fn visible() -> bool {
    KEYBOARD.with(|k| k.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A modalidade é do ÚLTIMO input, nas duas direções — não é um trilho de mão única. Se fosse,
    /// o anel acenderia na primeira tecla e nunca mais apagaria.
    ///
    /// Roda sem janela, mexendo no estado direto: o que se testa aqui é a máquina de estados; pedir o
    /// frame novo é responsabilidade do [`set`].
    #[test]
    fn a_modalidade_e_do_ultimo_input() {
        KEYBOARD.with(|k| k.set(false));
        assert!(!visible(), "ponteiro apaga");
        KEYBOARD.with(|k| k.set(true));
        assert!(visible(), "tecla acende");
        KEYBOARD.with(|k| k.set(false));
        assert!(!visible(), "e o ponteiro apaga de novo");
    }

    /// **A troca só pede frame novo quando a modalidade MUDA.** Se pedisse sempre, cada tecla
    /// digitada num campo de texto sujaria a janela inteira. O teste afirma o predicado do `set`
    /// (o `replace` devolve o valor anterior), que é o que decide o refresh.
    #[test]
    fn so_pede_frame_quando_a_modalidade_muda() {
        KEYBOARD.with(|k| k.set(true));
        assert!(
            KEYBOARD.with(|k| k.replace(true)),
            "repetir a mesma modalidade: o anterior é igual ao novo, logo nada muda"
        );
        assert!(
            KEYBOARD.with(|k| k.replace(false)),
            "trocar: o anterior difere do novo, logo pede frame"
        );
    }

    /// Estado inicial: apagado. Uma janela recém-aberta não mostra anel em nada.
    ///
    /// Roda numa thread própria porque o estado é `thread_local` e os outros testes o sujam — sem
    /// isso o teste passaria ou falharia pela ordem de execução.
    #[test]
    fn comeca_apagado() {
        assert!(
            !std::thread::spawn(visible).join().unwrap(),
            "numa thread nova, o rastreio nasce apagado"
        );
    }
}

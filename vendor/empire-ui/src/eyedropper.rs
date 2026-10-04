//! `eyedropper` — pega a cor de um pixel qualquer da tela.
//!
//! # Quem faz o trabalho é o sistema
//!
//! Amostrar um pixel da tela não é uma coisa que uma biblioteca de UI possa fazer sozinha: exige ler o
//! framebuffer de OUTRAS janelas, o que todo sistema operacional moderno trata como captura de tela e
//! protege atrás de permissão. Então não se implementa a lupa — se pede a do sistema.
//!
//! - **macOS**: [`NSColorSampler`], do AppKit (10.15+). É a mesma lupa do painel de cores do sistema:
//!   mostra o ampliador, deixa o usuário clicar em qualquer lugar da tela, e chama de volta com uma
//!   `NSColor` (ou `nil` se cancelou). Quem lida com a permissão e com a UI é o sistema.
//! - **resto**: sem suporte. [`is_supported`] devolve `false` e quem chama **não deve desenhar o
//!   botão** — é o que a referência que inspirou isto faz no navegador (esconder o botão quando não há
//!   `window.EyeDropper`), e é melhor que um botão que não faz nada.
//!
//! # A volta pro GPUI
//!
//! O callback do AppKit chega na thread principal, mas FORA do ciclo de update do GPUI: ele vem do
//! run loop do AppKit quando o usuário termina de escolher. A ponte é um [`gpui::AsyncApp`], capturado
//! antes de abrir a lupa — é ele que sabe reentrar no `App` de fora de um update.
//!
//! Por isso [`pick`] é assíncrona por natureza (callback, não retorno): entre o clique no botão e a cor
//! escolhida passa o tempo que o usuário quiser.
//!
//! [`NSColorSampler`]: https://developer.apple.com/documentation/appkit/nscolorsampler

use gpui::App;

/// Se esta plataforma sabe amostrar a cor de um pixel da tela.
///
/// `false` → **não desenhe o botão de conta-gotas**. Ver o doc do módulo.
pub fn is_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Abre a lupa do sistema e chama `on_pick` quando o usuário termina.
///
/// `Some([r, g, b])` em `[0,1]` (sRGB) com a cor escolhida; `None` se ele cancelou — ou se a
/// plataforma não tem suporte, caso em que o callback vem imediatamente.
///
/// O callback recebe um `&mut App`, e não um `&mut Window`: quem precisa de janela pra aplicar a cor
/// captura o [`gpui::AnyWindowHandle`] antes de chamar e reentra por ele (ver o uso em
/// `crate::color_picker`).
pub fn pick<F>(cx: &mut App, on_pick: F)
where
    F: FnOnce(Option<[f32; 3]>, &mut App) + 'static,
{
    #[cfg(target_os = "macos")]
    macos::pick(cx, on_pick);

    #[cfg(not(target_os = "macos"))]
    {
        // Sem lupa nesta plataforma: responde na hora em vez de deixar quem chamou esperando por um
        // callback que nunca vem.
        on_pick(None, cx);
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use block2::RcBlock;
    use objc2_app_kit::{NSColor, NSColorSampler, NSColorSpace};
    use std::cell::RefCell;
    use std::rc::Rc;

    pub(super) fn pick<F>(cx: &mut App, on_pick: F)
    where
        F: FnOnce(Option<[f32; 3]>, &mut App) + 'static,
    {
        // A ponte de volta pro GPUI: o callback do AppKit não roda dentro de um update, então precisa
        // de um handle que saiba entrar num.
        let app = cx.to_async();

        // `FnOnce` dentro de um bloco `Fn`: o sistema promete chamar o handler UMA vez (por escolha ou
        // por cancelamento), mas o tipo do bloco não sabe disso. O `RefCell<Option<_>>` é o que deixa
        // consumir o callback na primeira chamada e ignorar uma segunda, se ela existisse.
        let uma_vez = Rc::new(RefCell::new(Some(on_pick)));

        let handler = RcBlock::new(move |cor: *mut NSColor| {
            let Some(cb) = uma_vez.borrow_mut().take() else {
                return;
            };
            let rgb = unsafe { rgb_de(cor) };
            // Se a app já morreu, não há o que atualizar — o `Result` é descartado de propósito.
            let _ = app.update(|app| cb(rgb, app));
        });

        // O `NSColorSampler` se retém sozinho até a sessão terminar (está documentado no header), então
        // deixá-lo cair de escopo aqui é seguro.
        let sampler = NSColorSampler::new();
        unsafe { sampler.showSamplerWithSelectionHandler(&handler) };
    }

    /// `NSColor` → `[r, g, b]` em `[0,1]`. `nil` (cancelou) → `None`.
    ///
    /// # Safety
    ///
    /// `cor` tem que ser um ponteiro válido pra `NSColor` ou nulo — é o que o handler do
    /// `NSColorSampler` entrega.
    unsafe fn rgb_de(cor: *mut NSColor) -> Option<[f32; 3]> {
        let cor = unsafe { cor.as_ref() }?;
        // **A conversão de espaço não é opcional.** A cor que a lupa devolve vem no espaço do MONITOR
        // (Display P3 na maioria dos Macs de hoje), e ler `redComponent` dela direto daria um valor
        // que só significa algo naquele monitor — o mesmo pixel amarelo daria hex diferente em telas
        // diferentes. Pedindo sRGB, o número passa a ser o que o resto do picker entende.
        let srgb = cor.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
        Some([
            srgb.redComponent() as f32,
            srgb.greenComponent() as f32,
            srgb.blueComponent() as f32,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O suporte segue a plataforma, e é ele que decide se o botão existe. Um `is_supported` que
    /// mentisse pra cima deixaria um botão morto na tela; pra baixo, esconderia um recurso que
    /// funciona.
    #[test]
    fn o_suporte_seque_a_plataforma() {
        assert_eq!(is_supported(), cfg!(target_os = "macos"));
    }
}

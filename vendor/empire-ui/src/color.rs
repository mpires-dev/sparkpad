//! `color` — o tipo de cor usado pelas paletas de componente do crate.
//!
//! # Por que um newtype
//!
//! O crate tem **duas** convenções de cor:
//!
//! - a [`crate::theme::Palette`] guarda `0xRRGGBB` (6 dígitos, pro [`gpui::rgb`]);
//! - as paletas de componente guardam `0xRRGGBBAA` (8 dígitos, pro [`gpui::rgba`]).
//!
//! Com as duas sendo `u32`, o compilador aceitava qualquer troca em silêncio — e trocar **desloca
//! os canais**: `rgba(0xffffff)` é lido como `0x00FFFFFF`, ou seja ciano. Isso aconteceu três vezes
//! no campo de texto (campos ciano no tema claro, anel de foco teal, texto azul), sempre sem erro
//! de compilação, e a terceira só porque um `grep` de verificação não cobriu um call site.
//!
//! Com [`Rgba8`], `rgb(cor_de_componente)` e `rgba(token_do_tema)` **não compilam**. A única ponte
//! entre as convenções é [`opaque`], explícita e num lugar só.

/// Uma cor `0xRRGGBBAA` de paleta de componente.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rgba8(pub(crate) u32);

impl Rgba8 {
    /// A cor pronta pro GPUI.
    pub(crate) fn hsla(self) -> gpui::Hsla {
        gpui::rgba(self.0).into()
    }

    /// A mesma cor com o alfa multiplicado por `k` — o mecanismo de cross-fade, já que o GPUI não
    /// interpola estilo sozinho. `k` é aparado em `[0,1]` (o `delta` de um animador pode passar de
    /// 1 por arredondamento).
    pub(crate) fn scaled(self, k: f32) -> gpui::Hsla {
        let mut hsla = self.hsla();
        hsla.a *= k.clamp(0.0, 1.0);
        hsla
    }

    /// O alfa, em `[0,1]` — usado pelos testes pra afirmar o que é opaco e o que é translúcido.
    #[cfg(test)]
    pub(crate) fn alpha(self) -> f32 {
        self.hsla().a
    }
}

/// Interpola entre duas cores, com `t` em `[0,1]` (`0` = `a`, `1` = `b`).
///
/// A interpolação é feita em **RGBA**, não em HSL. Em HSL o matiz é circular: interpolar entre um
/// cinza e um vermelho passearia pelo círculo de cor e produziria tons intermediários que não estão
/// em nenhuma das duas pontas. Em RGBA o caminho é a reta esperada.
///
/// # Por que ALFA PREMULTIPLICADO
///
/// Interpolar RGB e alfa em canais separados só funciona quando as duas pontas têm o MESMO alfa.
/// Quando os alfas diferem, o RGB de uma camada quase transparente ainda pesa integralmente na
/// conta — e o RGB de uma camada quase transparente é praticamente arbitrário, porque quase não
/// aparece.
///
/// O caso que quebrou: o hover da `Outline` no tema claro vai de branco OPACO (`1,1,1 @ 1.0`) pro
/// acento, que é preto a 2% (`0,0,0 @ 0.02`). No meio do caminho a conta ingênua dá `0.5,0.5,0.5
/// @ 0.51` — um cinza MÉDIO a meia opacidade, bem mais escuro que as duas pontas. O botão escurecia
/// no meio do fade e clareava de novo: um flash cinza a cada passada do ponteiro.
///
/// Premultiplicando (`rgb × a`), o mesmo meio de caminho dá `0.98,0.98,0.98 @ 0.51` — branco a meia
/// opacidade, que sobre o painel branco é praticamente branco. A rampa fica monótona, que é o que
/// o olho espera.
///
/// Onde as duas pontas já têm o mesmo alfa (ou as duas são opacas), premultiplicar dá exatamente o
/// mesmo resultado de antes — o fator comum sai na divisão. Ou seja: isto conserta os casos
/// quebrados sem mexer em nenhum dos que já estavam certos.
///
/// `t` é aparado, porque quem chama costuma derivar de tempo decorrido e pode passar de 1.
pub(crate) fn lerp(a: gpui::Hsla, b: gpui::Hsla, t: f32) -> gpui::Hsla {
    let t = t.clamp(0.0, 1.0);
    let (a, b): (gpui::Rgba, gpui::Rgba) = (a.into(), b.into());

    let alfa = a.a + (b.a - a.a) * t;
    // Sem cobertura nenhuma não há cor pra recuperar: dividir por zero devolveria NaN, que o GPUI
    // pinta como preto. Transparente puro é a resposta certa.
    if alfa <= f32::EPSILON {
        return gpui::Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
        .into();
    }

    // Interpola o produto `rgb × a` (a CONTRIBUIÇÃO de cada camada) e devolve à convenção
    // não-premultiplicada dividindo pelo alfa resultante.
    let canal = |ca: f32, cb: f32| (ca * a.a + (cb * b.a - ca * a.a) * t) / alfa;
    gpui::Rgba {
        r: canal(a.r, b.r),
        g: canal(a.g, b.g),
        b: canal(a.b, b.b),
        a: alfa,
    }
    .into()
}

/// Eleva um token de 6 dígitos da [`crate::theme::Palette`] (`0xRRGGBB`) à convenção de paleta de
/// componente (`0xRRGGBBAA`, opaco).
///
/// É a **única** ponte entre as duas convenções. Se você está escrevendo `Rgba8(algum_token())`,
/// provavelmente queria isto.
pub(crate) fn opaque(c: u32) -> Rgba8 {
    Rgba8((c << 8) | 0xff)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `opaque` preserva o RGB e fixa o alfa em 1 — se ele deslocasse os canais, seria o mesmo bug
    /// que o newtype existe pra impedir.
    #[test]
    fn opaque_preserva_o_rgb_e_fixa_o_alfa() {
        let elevado: gpui::Rgba = opaque(0x2dd4bf).hsla().into();
        let original: gpui::Rgba = gpui::rgb(0x2dd4bf);
        assert_eq!(elevado.a, 1.0);
        assert!((elevado.r - original.r).abs() < 1e-6);
        assert!((elevado.g - original.g).abs() < 1e-6);
        assert!((elevado.b - original.b).abs() < 1e-6);
    }

    /// O alfa escalado multiplica e apara.
    #[test]
    fn alfa_escalado_multiplica_e_apara() {
        let meio = Rgba8(0xff000080);
        let cheio = meio.hsla().a;
        assert!((meio.scaled(1.0).a - cheio).abs() < 1e-6);
        assert!((meio.scaled(0.5).a - cheio * 0.5).abs() < 1e-6);
        assert_eq!(meio.scaled(0.0).a, 0.0);
        assert!((meio.scaled(3.0).a - cheio).abs() < 1e-6, "aparado em 1");
        assert_eq!(meio.scaled(-1.0).a, 0.0, "aparado em 0");
    }

    /// **Regressão do flash cinza no hover.** Indo de branco OPACO pra preto a 2%, nenhum ponto do
    /// caminho pode aparecer mais escuro que as duas pontas quando composto sobre um fundo branco.
    ///
    /// Com a interpolação ingênua (RGB e alfa em canais separados) o meio do caminho era cinza
    /// médio a 51% — sobre branco, ~0.75 de luminância contra ~0.98 das pontas. Era o flash que se
    /// via ao passar o ponteiro sobre um `Button` `Outline` no tema claro.
    #[test]
    fn o_caminho_entre_opaco_e_quase_transparente_nao_escurece() {
        let branco = Rgba8(0xffffffff).hsla();
        let acento = Rgba8(0x0000000a).scaled(0.5); // preto ~2%, o hover da Outline no claro

        // Luminância do resultado composto sobre um painel branco.
        let sobre_branco = |c: gpui::Hsla| {
            let c: gpui::Rgba = c.into();
            c.r * c.a + 1.0 * (1.0 - c.a)
        };

        let pontas = sobre_branco(branco).min(sobre_branco(acento));
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let meio = sobre_branco(lerp(branco, acento, t));
            assert!(
                meio >= pontas - 1e-3,
                "t={t}: {meio} é mais escuro que a ponta mais escura ({pontas})"
            );
        }
    }

    /// A rampa é **monótona**: cada passo caminha na mesma direção. Um fade que vai e volta é
    /// exatamente o que o olho lê como "piscada".
    #[test]
    fn a_rampa_do_fade_e_monotona() {
        let branco = Rgba8(0xffffffff).hsla();
        let acento = Rgba8(0x0000000a).scaled(0.5);
        let sobre_branco = |c: gpui::Hsla| {
            let c: gpui::Rgba = c.into();
            c.r * c.a + 1.0 * (1.0 - c.a)
        };

        let mut anterior = sobre_branco(lerp(branco, acento, 0.0));
        for i in 1..=20 {
            let atual = sobre_branco(lerp(branco, acento, i as f32 / 20.0));
            assert!(
                atual <= anterior + 1e-3,
                "passo {i}: subiu de {anterior} pra {atual} numa rampa que só desce"
            );
            anterior = atual;
        }
    }

    /// Premultiplicar não pode mudar o que já estava certo: com as duas pontas no MESMO alfa (aqui,
    /// as duas opacas), o resultado é o da interpolação simples, canal a canal.
    #[test]
    fn com_alfas_iguais_o_resultado_e_o_da_interpolacao_simples() {
        for (x, y) in [(0xffffffffu32, 0x000000ffu32), (0x2dd4bf80, 0xff000080)] {
            let (a, b) = (Rgba8(x).hsla(), Rgba8(y).hsla());
            let (ra, rb): (gpui::Rgba, gpui::Rgba) = (a.into(), b.into());
            for i in 0..=10 {
                let t = i as f32 / 10.0;
                let got: gpui::Rgba = lerp(a, b, t).into();
                assert!((got.r - (ra.r + (rb.r - ra.r) * t)).abs() < 1e-4, "r em t={t}");
                assert!((got.g - (ra.g + (rb.g - ra.g) * t)).abs() < 1e-4, "g em t={t}");
                assert!((got.b - (ra.b + (rb.b - ra.b) * t)).abs() < 1e-4, "b em t={t}");
                assert!((got.a - (ra.a + (rb.a - ra.a) * t)).abs() < 1e-4, "a em t={t}");
            }
        }
    }

    /// Um fade que NASCE do transparente (`Ghost`, `Link`) preserva o matiz do alvo o caminho
    /// inteiro — só a cobertura cresce. Na conta ingênua ele nascia do PRETO, porque
    /// `transparent_black` tem RGB zerado: um alvo claro escurecia antes de aparecer.
    #[test]
    fn fade_a_partir_do_transparente_preserva_o_matiz_do_alvo() {
        let alvo = Rgba8(0xffffff14).hsla(); // branco a 8%, o hover do Ghost no escuro
        let rgb_alvo: gpui::Rgba = alvo.into();
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            let got: gpui::Rgba = lerp(gpui::transparent_black(), alvo, t).into();
            assert!((got.r - rgb_alvo.r).abs() < 1e-4, "t={t}: o matiz mudou");
            assert!((got.a - rgb_alvo.a * t).abs() < 1e-4, "t={t}: a cobertura é que cresce");
        }
    }

    /// Alfa resultante zero devolve transparente puro, não `NaN` — que o GPUI pintaria de preto.
    #[test]
    fn alfa_zero_nao_vira_nan() {
        let c: gpui::Rgba = lerp(gpui::transparent_black(), gpui::transparent_black(), 0.5).into();
        assert!(c.r.is_finite() && c.g.is_finite() && c.b.is_finite() && c.a.is_finite());
        assert_eq!(c.a, 0.0);
    }

    /// Uma cor de 8 dígitos decodifica com os canais no lugar. Se `Rgba8` fosse construído a partir
    /// de um valor de 6 dígitos, `r` viria zerado — a assinatura do bug histórico.
    #[test]
    fn decodifica_com_os_canais_no_lugar() {
        let c: gpui::Rgba = Rgba8(0x262626ff).hsla().into();
        assert_eq!(c.a, 1.0);
        assert!(c.r > 0.1, "canal vermelho presente");
        assert!((c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01, "neutro");
    }
}

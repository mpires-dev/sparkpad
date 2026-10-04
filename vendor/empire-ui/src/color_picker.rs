//! `ColorPicker` — popover de seleção de cor estilo Figma (G8).
//!
//! Um `Entity` GPUI próprio com estado **HSVA** (h 0–360, s 0–1, v 0–1, a 0–1),
//! derivado de/para RGBA `[f32;4]` em `[0,1]`. Emite [`ColorPickerEvent`]:
//!
//! - [`ColorPickerEvent::Changed`] a cada alteração (live → o inspector grava no modelo).
//! - [`ColorPickerEvent::Closed`] ao fechar (clique-fora / `Escape`) — o shell persiste a cor
//!   escolhida nas **recentes**.
//!
//! Layout: quadrado **SV** (saturação×valor) + barra de **ALPHA** + barra de **HUE** (arco-íris) à
//! esquerda; coluna direita com previews Previous/New, o [`crate::Select`] de modo (RGB/HSB) com o
//! conta-gotas ao lado, os três campos numéricos e a fileira hex + opacidade + copiar; rodapé com 27
//! **swatches** (recentes persistidas na frente, paleta do design atrás).
//!
//! As duas fileiras de valores são [`crate::Group`]: os três canais são recortes de UM valor, e o hex
//! com a opacidade e o copiar são a mesma fileira. Costuradas, elas dizem isso na tela — uma peça só,
//! com filete de 1px nas emendas — em vez de parecerem controles independentes lado a lado.
//!
//! **É o primeiro componente autoral da biblioteca** — não tem referência no coss pra copiar. Então
//! ele é montado com as peças da casa: superfície de [`crate::card::Card`], [`crate::Input`],
//! [`crate::Select`] e [`crate::Button`]. Nada de superfície, campo ou botão desenhado à mão aqui —
//! era justamente o que fazia ele parecer de outra família.
//!
//! Os gradientes do SV/alpha/hue são GPU (`div` com [`gpui::linear_gradient`], 2 stops cada),
//! compostos por sobreposição de `div`s full-size (GPUI só faz 2 stops por gradiente).

use crate::color::Rgba8;
use crate::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    canvas, div, linear_color_stop, linear_gradient, px, rgb, rgba, App, AppContext, Bounds, Div,
    ClipboardItem, Context, CursorStyle, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement,
    Pixels, Render, SharedString, Styled, Subscription, Window,
};
// Do núcleo sobra só o ESTADO de edição: a apresentação é o nosso `crate::Input`.
use gpui_component::input::{InputEvent, InputState};

// =================================================================================================
// HSV ↔ RGB (funções puras, testadas)
// =================================================================================================

/// **HSV → RGB.** `h` em 0–360, `s`/`v` em 0–1. Retorna `(r,g,b)` em 0–1.
/// Algoritmo padrão (setor de 60°), igual ao da Wikipedia (HSL and HSV).
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h.rem_euclid(360.0);
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r1 + m, g1 + m, b1 + m)
}

/// **RGB → HSV.** `r`/`g`/`b` em 0–1. Retorna `(h 0–360, s 0–1, v 0–1)`.
/// `h` indefinido (cinza) → 0; preserva o hue corrente é responsabilidade do chamador
/// (ver [`ColorPicker::set_rgba`]).
pub fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let r = r.clamp(0.0, 1.0);
    let g = g.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max <= 0.0 { 0.0 } else { delta / max };
    let h = if delta <= 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    (h.rem_euclid(360.0), s, v)
}

/// Parseia `#RGB`/`#RRGGBB` (com ou sem `#`) → `(r,g,b)` em 0–1. `None` se inválido.
/// Ignora alpha (o alpha do picker vem do controle de opacidade, não do hex).
pub fn parse_hex(s: &str) -> Option<(f32, f32, f32)> {
    let s = s.trim().trim_start_matches('#');
    let (r, g, b) = match s.len() {
        3 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            (r, g, b)
        }
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            (r, g, b)
        }
        _ => return None,
    };
    Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
}

/// `(r,g,b)` em 0–1 → string hex `#RRGGBB` (maiúsculo, sem alpha).
pub fn hex_from_rgb(r: f32, g: f32, b: f32) -> String {
    let to8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", to8(r), to8(g), to8(b))
}

/// Escreve num campo, **menos se ele estiver com o foco**.
///
/// # Por que a exceção
///
/// `InputState::set_value` sempre colapsa a seleção no fim do texto (`selected_range = len..len`), e
/// não tem atalho pra texto idêntico: escrever o mesmo valor de volta já apaga a seleção. Isso quebrava
/// o "focar seleciona tudo" num caso específico e relatado — clicar de um campo DIRETO no outro:
///
/// 1. o clique foca o campo novo, e o render seguinte seleciona o conteúdo dele;
/// 2. **depois do render**, as escutas de foco do núcleo disparam o `Blur` do campo antigo;
/// 3. o `Blur` faz commit, o commit chama `sync_inputs`, e o `set_value` cai sobre o campo novo.
///
/// Passando pelo vazio antes funcionava porque o `Blur` já tinha acontecido no clique anterior — era
/// exatamente a diferença entre os dois caminhos.
///
/// Além de consertar isso, a exceção é o comportamento certo por si: reescrever o texto do campo em que
/// a pessoa está digitando é hostil, mesmo sem seleção nenhuma envolvida. É o que o navegador faz.
fn escreve(
    campo: &Entity<InputState>,
    valor: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
) {
    if gpui::Focusable::focus_handle(campo.read(cx), cx).is_focused(window) {
        return;
    }
    campo.update(cx, |st, cx| st.set_value(valor, window, cx));
}

/// A **alça de uma barra** (hue/alpha), ancorada por FRAÇÃO da altura.
///
/// Mesma razão do quadrado SV: as barras são elásticas, e uma alça em `f * ALTURA` precisaria da
/// altura medida, que não existe no primeiro frame (armadilha nº 14). O ancorador tem altura zero e
/// fica na fração exata; a alça pendura nele recuada de meia altura, que é o jeito de somar fração
/// com pixel sem um `calc` que o GPUI não tem.
///
/// ⚠️ A geometria pede 2px de sobra em cada lado (o `-2` do recuo, com `BAR_W + 4` de largura), mas
/// **isso não aparece**: a alça é filha de um container com `overflow_hidden`, e a máscara retangular
/// do GPUI a corta na largura da barra — medido, ela sai com os 14px da barra, não com 18. Pra a
/// sobra existir, a alça teria que ser IRMÃ da barra, como a do quadrado SV passou a ser (ver
/// `render_sv`). Fica como está por decisão pendente: é mudança visível nas duas barras.
fn bar_handle(fracao: f32, miolo: impl IntoElement) -> Div {
    div()
        .absolute()
        .top(gpui::relative(fracao.clamp(0.0, 1.0)))
        .w_full()
        .h(px(0.0))
        .child(
            div()
                .absolute()
                .left(px(-BAR_HANDLE_OVERHANG))
                .top(px(-BAR_HANDLE_H / 2.0))
                .w(px(BAR_W + 2.0 * BAR_HANDLE_OVERHANG))
                .h(px(BAR_HANDLE_H))
                // `rounded_full` numa caixa mais larga que alta dá a pílula: o raio vira metade do
                // lado menor.
                .rounded_full()
                .border_2()
                .border_color(palette().handle_ring.hsla())
                .shadow_sm()
                .child(miolo),
        )
}

/// O **miolo** de uma alça de barra: preenche o interior da pílula com a cor dada.
///
/// `size_full` aqui é o interior (a caixa de conteúdo, dentro do contorno de 2px), e o raio é o de
/// dentro — ver [`BAR_HANDLE_INNER_RADIUS`].
fn bar_handle_fill(cor: impl Into<gpui::Hsla>) -> Div {
    div()
        .size_full()
        .rounded(px(BAR_HANDLE_INNER_RADIUS))
        .bg(cor.into())
}

// =================================================================================================
// HSV ↔ HSL — identidades algébricas, sem passar por RGB
// =================================================================================================
//
// Os dois espaços compartilham o MATIZ, e saturação/luminosidade se convertem por álgebra direta.
// Fazer a volta por RGB funcionaria e arredondaria duas vezes: o valor escorregaria a cada tecla
// digitada, que é o defeito que o modo HSB desta base já evitava do mesmo jeito.

/// **HSV → HSL.** Recebe `s`/`v` em `[0,1]`, devolve `(s_hsl, l)` em `[0,1]`. O matiz não muda.
pub fn hsv_to_hsl(s: f32, v: f32) -> (f32, f32) {
    let (s, v) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let l = v * (1.0 - s / 2.0);
    // `l` em 0 ou 1 é preto ou branco: não há saturação definida, e a fórmula dividiria por zero.
    let s_hsl = if l <= 0.0 || l >= 1.0 {
        0.0
    } else {
        (v - l) / l.min(1.0 - l)
    };
    (s_hsl.clamp(0.0, 1.0), l)
}

/// **HSL → HSV.** Inversa exata de [`hsv_to_hsl`].
pub fn hsl_to_hsv(s_hsl: f32, l: f32) -> (f32, f32) {
    let (s_hsl, l) = (s_hsl.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    let v = l + s_hsl * l.min(1.0 - l);
    let s = if v <= 0.0 { 0.0 } else { 2.0 * (1.0 - l / v) };
    (s.clamp(0.0, 1.0), v)
}

// =================================================================================================
// sRGB ↔ OKLCH
// =================================================================================================
//
// OKLCH é a forma polar do **Oklab** (Björn Ottosson, 2020): um espaço perceptualmente uniforme, onde
// a mesma diferença numérica de luminosidade ou de croma vale a mesma diferença percebida — coisa que
// HSL não dá (amarelo e azul com o mesmo `l` de HSL têm brilhos percebidos bem diferentes). É o espaço
// que o próprio coss usa nas misturas do `globals.css` (`color-mix(in oklab, …)`).
//
// O caminho é sRGB → sRGB linear → LMS → Oklab → polar. As matrizes são as do artigo original.
//
// ⚠️ **O sRGB não alcança todo o OKLCH.** Croma alto em certos matizes cai fora do gamute, e a volta
// pra sRGB tem que aparar os canais em `[0,1]`. Então digitar um croma impossível faz o valor voltar
// pra borda do gamute — não é bug, é o espaço sendo maior que a tela.

/// Companding do sRGB: valor de canal (0–1, com gama) → linear.
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Inversa de [`srgb_to_linear`].
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// **sRGB → OKLCH.** `r`/`g`/`b` em `[0,1]`. Devolve `(l 0–1, c ≥ 0, h 0–360)`.
pub fn rgb_to_oklch(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (r, g, b) = (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b));

    let l_ = (0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m_ = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s_ = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();

    let lab_l = 0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_;
    let lab_a = 1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_;
    let lab_b = 0.025_904_037 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_;

    let c = lab_a.hypot(lab_b);
    // Croma nulo = cinza: o matiz é indefinido, e devolver 0 evita um ângulo aleatório do `atan2`.
    let h = if c <= 1e-6 {
        0.0
    } else {
        lab_b.atan2(lab_a).to_degrees().rem_euclid(360.0)
    };
    (lab_l.clamp(0.0, 1.0), c, h)
}

/// **OKLCH → sRGB**, com os canais **aparados** em `[0,1]` (ver o aviso de gamute acima).
pub fn oklch_to_rgb(l: f32, c: f32, h: f32) -> (f32, f32, f32) {
    let (sin, cos) = h.to_radians().sin_cos();
    let (lab_a, lab_b) = (c * cos, c * sin);

    let l_ = l + 0.396_337_78 * lab_a + 0.215_803_76 * lab_b;
    let m_ = l - 0.105_561_346 * lab_a - 0.063_854_17 * lab_b;
    let s_ = l - 0.089_484_18 * lab_a - 1.291_485_5 * lab_b;
    let (lms_l, lms_m, lms_s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    let r = 4.076_741_7 * lms_l - 3.307_711_6 * lms_m + 0.230_969_94 * lms_s;
    let g = -1.268_438 * lms_l + 2.609_757_4 * lms_m - 0.341_319_38 * lms_s;
    let b = -0.004_196_086_3 * lms_l - 0.703_418_6 * lms_m + 1.707_614_7 * lms_s;

    let ap = |c: f32| linear_to_srgb(c).clamp(0.0, 1.0);
    (ap(r), ap(g), ap(b))
}

// =================================================================================================
// Formatação dos três campos
// =================================================================================================
//
// Cada espaço tem a sua convenção de casas decimais, e ela não é enfeite: é o que decide o passo mais
// fino que dá pra digitar. Grau inteiro é 1/360 do círculo; 1% de luminosidade é o degrau que se
// percebe; e o croma do OKLCH vive em `0–0,4`, então sem três casas o campo teria 4 valores úteis.

/// Croma máximo aceito num campo OKLCH. O sRGB não passa de ~0,37 (no verde puro), e `0,5` deixa
/// margem pra digitar um valor alto e ver a cor parar na borda do gamute em vez de ser recusada.
const MAX_CHROMA: f32 = 0.5;

/// Graus inteiros — para o matiz do HSL e do OKLCH.
fn fmt_deg(h: f32) -> String {
    (h.rem_euclid(360.0).round() as i32).to_string()
}

/// Fração `[0,1]` → percentual inteiro.
fn fmt_pct(x: f32) -> String {
    ((x.clamp(0.0, 1.0) * 100.0).round() as i32).to_string()
}

/// Fração `[0,1]` → percentual com **uma** casa. É a luminosidade do OKLCH, onde o degrau de 1% já é
/// grosseiro perto do branco.
fn fmt_pct_1(x: f32) -> String {
    format!("{:.1}", x.clamp(0.0, 1.0) * 100.0)
}

/// Croma do OKLCH com **três** casas, como o CSS escreve (`oklch(62.8% 0.258 29)`).
fn fmt_chroma(c: f32) -> String {
    format!("{:.3}", c.clamp(0.0, MAX_CHROMA))
}

/// `Hsla` opaco da cor PURA do hue (s=1,v=1) — usado de fundo do quadrado SV e topo das barras.
fn hue_pure_hsla(h: f32) -> Hsla {
    let (r, g, b) = hsv_to_rgb(h, 1.0, 1.0);
    gpui::Rgba { r, g, b, a: 1.0 }.into()
}

// =================================================================================================
// Recentes persistidas (~/.config/fennel/recent_colors.json)
// =================================================================================================

/// **Paleta FIXA do design** (27 cores, na ordem exata do JSX): linha1 = 18 swatches, linha2 = 9.
/// Sempre exibida no rodapé (em sequência); as cores recentes do usuário entram NA FRENTE,
/// preenchendo 27 slots sem duplicar (ver [`ColorPicker::swatch_list`]).
pub const PALETTE: [&str; 27] = [
    // linha 1 (18)
    "#EE417B", "#863995", "#1998A7", "#0B7A6B", "#329043", "#FCC02B", "#5E4237", "#626262",
    "#40B3E7", "#EA1A65", "#006C65", "#98342A", "#569668", "#D07569", "#D889D1", "#008A53",
    "#DF9387", "#996861", // linha 2 (9)
    "#873A31", "#41908D", "#4DB996", "#97CF49", "#8BB6FF", "#D58C39", "#C23429", "#8E2166",
    "#DDDDDD",
];

/// Constrói os **27 swatches** do rodapé: as `recents` (na ordem dada — mais nova primeiro)
/// seguidas da [`PALETTE`] em sequência, com **dedup** (case-insensitive) e cap em 27. Sem
/// recentes → exatamente a paleta do design.
fn build_swatch_list(recents: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(27);
    let mut seen: Vec<String> = Vec::with_capacity(27);
    let push = |hex: String, out: &mut Vec<String>, seen: &mut Vec<String>| {
        let up = hex.to_ascii_uppercase();
        if out.len() < 27 && !seen.contains(&up) {
            seen.push(up);
            out.push(hex);
        }
    };
    for hex in recents {
        push(hex.clone(), &mut out, &mut seen);
    }
    for hex in PALETTE {
        push(hex.to_string(), &mut out, &mut seen);
    }
    out
}

/// Teto de cores recentes guardadas.
const MAX_RECENTS: usize = 16;

/// Nome da app usado na pasta de config das cores recentes. Default `"empire-ui"`; a app
/// troca via [`set_recents_app_name`].
static RECENTS_APP_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Define o nome da app na pasta de config dos **recentes do color picker** — as cores vão
/// pra `~/.config/<nome>/recent_colors.json`.
///
/// Chame **uma vez no bootstrap**, antes do primeiro [`ColorPicker`] existir. Sem isto o
/// default é `"empire-ui"`, e cada app que embute a lib compartilharia o mesmo arquivo.
///
/// Só a PRIMEIRA chamada vale (o valor é fixado com [`std::sync::OnceLock`]); devolve `true`
/// se fixou, `false` se já havia sido definido antes.
pub fn set_recents_app_name(name: impl Into<String>) -> bool {
    RECENTS_APP_NAME.set(name.into()).is_ok()
}

/// O nome de app corrente pra pasta de recentes.
pub fn recents_app_name() -> &'static str {
    RECENTS_APP_NAME.get().map_or("empire-ui", |s| s.as_str())
}

/// Caminho do arquivo de recentes: `~/.config/<app>/recent_colors.json`. `None` sem `$HOME`.
fn recents_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        std::path::Path::new(&home)
            .join(".config")
            .join(recents_app_name())
            .join("recent_colors.json"),
    )
}

/// Carrega as cores recentes do disco (hex strings). Lista vazia se o arquivo não existe / é
/// inválido (degrada sem quebrar).
pub fn load_recents() -> Vec<String> {
    let Some(path) = recents_path() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(&text).unwrap_or_default()
}

/// Persiste as cores recentes no disco (cria a pasta `~/.config/fennel`). Erros são ignorados
/// (best-effort — recentes não são críticas).
fn save_recents(recents: &[String]) {
    let Some(path) = recents_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(recents) {
        let _ = std::fs::write(&path, text);
    }
}

// =================================================================================================
// Eventos
// =================================================================================================

/// Em que espaço os três campos numéricos trabalham.
///
/// Antes disto o "RGB" da coluna direita era um desenho **inerte** — um `div` com o texto e um chevron,
/// sem menu e sem função. Agora é o nosso [`crate::Select`], e ele realmente troca o espaço: os três
/// campos passam a ler e escrever HSB usando a conversão que este módulo já tinha testada.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ColorMode {
    /// **OKLCH**: luminosidade perceptual (0–100%) / croma (0–0,4) / matiz (0–360°).
    ///
    /// A forma polar do Oklab. Perceptualmente uniforme, ao contrário de HSL — e o espaço que o
    /// `globals.css` do coss usa nas misturas. ⚠️ Maior que o sRGB: croma alto em certos matizes cai
    /// fora do gamute e volta aparado (ver [`oklch_to_rgb`]).
    Oklch,
    /// **HSL**: matiz (0–360°) / saturação (0–100%) / luminosidade (0–100%).
    Hsl,
    /// **RGB**: vermelho / verde / azul, 0–255.
    #[default]
    Rgb,
}

impl ColorMode {
    /// Os modos, **na ordem em que aparecem no Select**.
    ///
    /// Esta é a única lista: os rótulos do [`crate::Select`] são gerados daqui (ver
    /// [`ColorPicker::new`]), então não há como a ordem do menu divergir da que [`Self::from_index`]
    /// entende. Duas listas paralelas seria o jeito clássico de o 3º item passar a significar o 2º.
    pub const ALL: [ColorMode; 3] = [ColorMode::Oklch, ColorMode::Hsl, ColorMode::Rgb];

    /// O rótulo mostrado no Select.
    pub fn label(self) -> &'static str {
        match self {
            ColorMode::Oklch => "OKLCH",
            ColorMode::Hsl => "HSL",
            ColorMode::Rgb => "RGB",
        }
    }

    /// O índice deste modo em [`Self::ALL`].
    fn index(self) -> usize {
        Self::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }

    /// O modo de um índice do Select. Índice fora da lista cai no default em vez de entrar em pânico:
    /// um Select desalinhado é defeito de layout, não motivo pra derrubar a janela.
    fn from_index(i: usize) -> Self {
        Self::ALL.get(i).copied().unwrap_or_default()
    }
}

/// Evento emitido pelo [`ColorPicker`].
#[derive(Debug, Clone, Copy)]
pub enum ColorPickerEvent {
    /// Cor mudou (RGBA em `[0,1]`). O inspector grava no modelo (live update do preview).
    Changed([f32; 4]),
    /// Picker fechado (X / clique-fora / Esc). O shell fecha o popover e persiste a recente.
    Closed,
}

// =================================================================================================
// Widget
// =================================================================================================

// -------------------------------------------------------------------------------------------------
// Paleta — os tokens do coss, como todo componente da lib
// -------------------------------------------------------------------------------------------------
//
// Antes daqui o picker lia `theme::BG_FIELD`, `theme::ICON_SCRUB` e `theme::TEXT_VALUE`: a paleta do
// EDITOR de vídeo, não a do design system. Era a razão principal de ele parecer de outra família — as
// cores não eram as dos vizinhos. Convenção da casa: todo valor é `0xRRGGBBAA` (ver [`crate::color`]).

struct PickerPalette {
    /// `--muted-foreground`: as legendas Previous/New.
    ///
    /// Não há mais nenhum texto de CONTEÚDO pintado aqui — os valores todos moram dentro de
    /// [`crate::Input`] e [`crate::Select`], que pintam o próprio texto com `--foreground`. Era um dos
    /// objetivos da refatoração: o picker não escolhe mais a cor do texto que ele mostra.
    text_muted: Rgba8,
    /// `--border`: as bordas das superfícies internas (SV, barras, previews, swatches).
    border: Rgba8,
    /// Contorno das três alças (a bolinha do quadrado e as duas pílulas das barras).
    ///
    /// **Inverte com o tema**: branco no escuro, cinza no claro. Elas são desenhadas SOBRE conteúdo de
    /// cor arbitrária, então o contorno não pode sair do mesmo par de tokens das superfícies — o que
    /// garante o contraste é ele ir contra o fundo do painel.
    ///
    /// No claro não é preto: preto puro ficou agressivo demais — o contorno gritava mais que a cor que
    /// ele delimita. `#656565` recorta contra as duas pontas (contra o branco e contra as cores
    /// escuras do fim das barras) sem virar o elemento mais forte do painel.
    handle_ring: Rgba8,
}

const PICKER_LIGHT: PickerPalette = PickerPalette {
    text_muted: Rgba8(0x686868ff), // mix(neutral-500 90%, black)
    border: Rgba8(0x00000014),     // preto 8%
    handle_ring: Rgba8(0x656565ff),
};

const PICKER_DARK: PickerPalette = PickerPalette {
    handle_ring: Rgba8(0xffffffff),
    text_muted: Rgba8(0x818181ff), // mix(neutral-500 90%, white)
    border: Rgba8(0xffffff0f),     // branco 6%
};

fn palette() -> &'static PickerPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &PICKER_DARK,
        theme::ThemeMode::Light => &PICKER_LIGHT,
    }
}

// -------------------------------------------------------------------------------------------------
// Geometria
// -------------------------------------------------------------------------------------------------

/// Geometria do quadrado SV / barras (px). Espelha a referência Paper (167×167, barras 14×167).
/// Lado da alça do quadrado SV — o círculo branco.
const SV_HANDLE: f32 = 12.0;

/// Altura da alça das barras (hue/alpha) — a pílula. Alta o bastante pra sobrar miolo depois do
/// contorno de 2px: 10 − 4 = 6 de interior, que é o que faz a cor lá dentro ser legível.
const BAR_HANDLE_H: f32 = 10.0;

/// Quanto a alça das barras sobra pra fora de cada lado. Ela é mais larga que a barra pra o contorno
/// branco ficar legível contra qualquer cor embaixo — e agora isso REALMENTE acontece, porque ela
/// deixou de ser filha do container recortado (ver `render_hue`).
const BAR_HANDLE_OVERHANG: f32 = 2.0;

/// Raio do MIOLO da alça: o raio externo menos o contorno, senão as quinas do miolo aparecem
/// quadradas dentro da pílula (o `overflow_hidden` do GPUI é máscara retangular).
const BAR_HANDLE_INNER_RADIUS: f32 = BAR_HANDLE_H / 2.0 - 2.0;

/// Quantos segmentos de 2 stops formam o arco-íris da barra de matiz. Seis, porque o GPUI só faz 2
/// paradas por gradiente e o círculo de matiz tem 6 setores de 60°.
const HUE_SEGMENTS: usize = 6;

/// Altura do filete que fecha a costura entre dois segmentos do arco-íris — ver `render_hue`. Dois
/// pixels lógicos: a falha é de meio pixel, e o filete é centrado na fronteira, então 1px de cada
/// lado cobre com folga em qualquer escala de tela.
const SEAM_PATCH: f32 = 2.0;

/// Altura do bloco de preview Previous/New.
const PREVIEW_H: f32 = 39.0;

/// Lado do glifo do botão de copiar. Menor que os 16 padrão de um `IconSm`: ele é uma ação
/// secundária numa fileira de valores, e a 16 competia com os números ao lado.
const COPY_ICON: f32 = 14.0;

/// Quanto uma camada SOBREPOSTA cresce além da superfície — meio pixel lógico.
///
/// ## Por que uma camada precisa sangrar
///
/// O quadrado SV é uma pilha: fundo de matiz, gradiente branco→transparente, gradiente
/// transparente→preto. As três com o MESMO retângulo arredondado. Na curva, o antialiasing dá a cada
/// uma cobertura parcial, e cobertura parcial COMPÕE: com α=0,5 em duas camadas opacas, o resultado é
/// `0,5·preto + 0,25·branco + 0,25·(o que estiver embaixo)`. Ou seja, o fundo de matiz aparecia a 25%
/// numa faixa de 1px ao longo do arco, mesmo estando coberto por duas camadas opacas — um fio
/// avermelhado nas quinas, bem visível na inferior esquerda contra o preto.
///
/// A saída é a camada de cima crescer meio pixel: assim a borda dela cai FORA da faixa de
/// antialiasing da camada de baixo, e onde ela é opaca cobre aquela faixa inteira. O `overflow_hidden`
/// do pai (máscara retangular) recorta a sobra nos lados retos; nas quinas o arco fica meio pixel
/// mais cheio, o que não se vê — e, principalmente, é da MESMA cor do interior, não de outra.
const LAYER_BLEED: f32 = 0.5;

/// Uma camada full-size de uma superfície: arredondada e meio pixel maior. Ver [`LAYER_BLEED`].
///
/// Use pra toda camada que fique POR CIMA de outra. A de baixo (o fundo, ou a primeira da pilha) não
/// precisa: não há nada sob ela pra vazar.
fn layer(radius: f32) -> Div {
    div()
        .absolute()
        .top(px(-LAYER_BLEED))
        .left(px(-LAYER_BLEED))
        .right(px(-LAYER_BLEED))
        .bottom(px(-LAYER_BLEED))
        .rounded(px(radius + LAYER_BLEED))
}

// =================================================================================================
// A geometria do painel, derivada
// =================================================================================================
//
// Nada aqui é número escolhido a olho: a **altura da coluna direita** é a medida-mãe, o quadrado SV é
// um quadrado desse lado, e a largura do painel é a soma das peças. Assim a proporção 1:1 do quadrado
// é aritmética, não coincidência — e se alguém acrescentar uma fileira na coluna, o quadrado cresce
// com ela em vez de virar retângulo.
//
// Houve uma tentativa de fazer isso com `aspect-ratio`: o `Style` do GPUI TEM o campo e o repassa pro
// taffy, mas não há método de builder, e resolver a altura em runtime deixaria a largura do painel sem
// como acompanhar (sobraria vão no fim da linha). Somar as partes é exato e não precisa medir nada.

/// Vão entre as fileiras da coluna direita. O mesmo valor do [`COL_GAP`], por decisão de desenho: o
/// respiro entre peças do painel é um só, nas duas direções.
const ROW_GAP: f32 = 12.0;

/// Altura de uma fileira de controles da coluna (campos `Sm` e botão `IconSm`: 28).
const ROW_H: f32 = 28.0;

/// Vão entre o bloco de preview e os rótulos Previous/New.
const PREVIEW_GAP: f32 = 2.0;

/// Entrelinha dos rótulos Previous/New — ver `render_preview`.
const PREVIEW_LABEL_LH: f32 = 15.0;

/// **A medida-mãe**: a altura da coluna direita, somada das partes que ela empilha — o bloco de
/// preview com seus rótulos, e as três fileiras (modo, canais, hex) com os vãos entre elas.
const RIGHT_COL_HEIGHT: f32 =
    PREVIEW_H + PREVIEW_GAP + PREVIEW_LABEL_LH + 3.0 * ROW_GAP + 3.0 * ROW_H;

/// Lado do quadrado SV. **Quadrado**: o lado é a altura da coluna direita.
const SV_SIDE: f32 = RIGHT_COL_HEIGHT;

/// Vão entre as colunas da linha principal (quadrado · alpha · matiz · valores). Igual ao
/// [`ROW_GAP`] — ver a nota lá.
const COL_GAP: f32 = 12.0;

/// Largura fixa do painel, somada das peças da linha principal.
///
/// Era 540 com o quadrado de lado livre; com o quadrado amarrado à altura da coluna, a soma dá menos —
/// e o que sobrava antes era vão morto no fim da linha.
const PANEL_WIDTH: f32 = 2.0 * CARD_BORDER
    + 2.0 * CARD_PAD
    + SV_SIDE
    + 3.0 * COL_GAP
    + 2.0 * BAR_W
    + RIGHT_COL_WIDTH;

/// Borda do [`crate::card::Card`], que entra na largura porque o `width` dele é border-box.
const CARD_BORDER: f32 = 1.0;

/// Respiro interno do card compacto (`crate::card::PAD_TIGHT`).
const CARD_PAD: f32 = 16.0;

/// Quanto tempo o botão de copiar fica mostrando o check depois de copiar.
const COPIED_FEEDBACK: std::time::Duration = std::time::Duration::from_secs(1);

/// Largura do campo de opacidade. Fixa porque o valor é sempre 0–100: crescer com o painel só daria
/// um campo vazio e largo do lado do hex.
///
/// 64 e não 88: `100` mede 21px de glifo, e com o respiro de 10 de cada lado o campo pede 42. Os 88
/// que estavam aqui vinham da referência, onde a fileira era mais larga.
const OPACITY_WIDTH: f32 = 64.0;

/// **Largura da coluna direita** (previews + modo + valores).
///
/// Fixa, e não elástica: o conteúdo dela é todo de controles de propósito fixo, então dividir a linha
/// meio a meio com o quadrado SV só dava campos com muito vazio dentro. O quadrado fica com a sobra.
///
/// O número sai do MÍNIMO da fileira mais exigente, medido na janela (glifos: `#FFFFFF` = 52,5,
/// `100` = 21, `255` = 21, `RGB` = 26):
///
/// | fileira | mínimo |
/// |---|---|
/// | modo: Select (`min-w-36` = 144) + vão 8 + conta-gotas 28 | **180** |
/// | hex: campo 73,5 + emenda + opacidade 64 + emenda + copiar 28 | 167,5 |
/// | três canais: 3 × 42 + 2 emendas | 128 |
///
/// Então quem manda é a fileira do modo, e 192 deixa 12px de folga sobre ela. Baixar disto começa a
/// apertar o piso do gatilho do Select; se um dia for preciso, o caminho é `Select::fit`.
const RIGHT_COL_WIDTH: f32 = 192.0;
const BAR_W: f32 = 14.0;

/// Raio das superfícies grandes — SV, barras de hue/alpha e os previews.
///
/// **Na escala do design system.** Antes eram 4px, que não existe na escala do coss
/// (`--radius-sm` 6, `--radius-md` 8, `--radius` 10, `--radius-xl` 14, `--radius-2xl` 16). Os
/// controles do picker são caixas de conteúdo, então usam o `--radius-md` que o resto da lib usa em
/// caixa média (o item de menu, o polegar do switch, a cartelinha do `empty`).
const SURFACE_RADIUS: f32 = 8.0;

/// Raio das amostras do rodapé — `--radius-sm`. Antes 3px, também fora da escala.
const SWATCH_RADIUS: f32 = 6.0;

// ⚠️ PENDENTE, declarado: os polegares do SV e das barras ainda são três desenhos diferentes (um
// anel de 12px no SV, retângulos de 14×6 nas barras). O certo é o polegar do [`crate::slider`] — 16px,
// branco, borda de 1px, bisel embaixo, sombra — porque arrastar um valor contínuo é o mesmo gesto nos
// dois lugares. Isso exige expor o polegar do `slider` como `pub(crate)`, que é a consolidação que
// vários componentes já pedem, e ficou pra um passo separado.


/// Qual handle está sendo arrastado (mouse capturado pelo `occlude()` do popover).
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Sv,
    Hue,
    Alpha,
}

/// O **color picker** (popover). Estado HSVA + os inputs editáveis (R/G/B/Hex/opacidade).
pub struct ColorPicker {
    h: f32, // 0–360
    s: f32, // 0–1
    v: f32, // 0–1
    a: f32, // 0–1
    /// Cor de quando o picker abriu (preview "Previous").
    previous: [f32; 4],
    /// Cores recentes (hex), persistidas. Carregadas no construtor.
    recents: Vec<String>,
    /// Arraste ativo de handle (SV/hue/alpha), `None` se nenhum.
    drag: Option<Drag>,
    /// Bounds (px janela) capturados a cada paint dos 3 alvos de arraste (pro mapeamento cursor→valor).
    sv_bounds: Option<Bounds<Pixels>>,
    hue_bounds: Option<Bounds<Pixels>>,
    alpha_bounds: Option<Bounds<Pixels>>,
    /// Inputs editáveis (R/G/B 0–255, Hex, opacidade 0–100).
    r_input: Entity<InputState>,
    g_input: Entity<InputState>,
    b_input: Entity<InputState>,
    hex_input: Entity<InputState>,
    opacity_input: Entity<InputState>,
    /// Em que espaço os três campos trabalham.
    mode: ColorMode,
    /// Qual dos cinco campos tinha o foco no frame anterior — ver [`ColorPicker::selecionar_ao_focar`].
    campo_focado: Option<usize>,
    /// Quando o hex foi copiado, pra o botão mostrar o check por [`COPIED_FEEDBACK`].
    ///
    /// `None` = estado normal. É o padrão de animação da casa: o progresso vem do tempo decorrido, não
    /// de um temporizador guardado (ver [`crate::button`]).
    copied_at: Option<std::time::Instant>,
    /// O [`crate::Select`] que escolhe o espaço. Antes isto era um `div` inerte com o texto "RGB".
    mode_select: Entity<crate::Select>,
    focus_handle: FocusHandle,
    _subs: Vec<Subscription>,
}

impl EventEmitter<ColorPickerEvent> for ColorPicker {}

impl Focusable for ColorPicker {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ColorPicker {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Os campos nascem VAZIOS: quem os preenche é o `sync_inputs` no fim deste construtor.
        //
        // Eles tinham literais aqui (`"255"`, `"#FFFFFF"`, `"100"`), que eram uma segunda fonte de
        // verdade pra cor inicial — e ficavam certos por coincidência, porque a cor inicial é branca.
        // Bastava trocar o modo default pra OKLCH e o painel abria mostrando `255 255 255` num campo
        // que devia dizer `100.0 0.000 0`.
        let r_input = cx.new(|cx| InputState::new(window, cx));
        let g_input = cx.new(|cx| InputState::new(window, cx));
        let b_input = cx.new(|cx| InputState::new(window, cx));
        let hex_input = cx.new(|cx| InputState::new(window, cx));
        let opacity_input = cx.new(|cx| InputState::new(window, cx));

        // O Select do modo — o nosso, com as duas opções que a matemática deste módulo já suporta.
        let mode_select = cx.new(|cx| {
            crate::Select::new(
                ColorMode::ALL
                    .iter()
                    .map(|m| SharedString::from(m.label()))
                    .collect(),
                ColorMode::default().index(),
                cx,
            )
            .size(crate::SelectSize::Sm)
        });

        let mut subs = Vec::new();
        // ⚠️ **Todas as assinaturas daqui usam `subscribe_in`, e não `subscribe`.**
        //
        // A diferença é que ela entrega um `&mut Window` no callback — e sem Window não dá pra chamar
        // `sync_inputs`, porque escrever num `InputState` exige um. Com o `subscribe` comum, trocar o
        // espaço no Select trocava o `self.mode` e os três campos continuavam mostrando os números do
        // espaço ANTERIOR até o próximo arraste. (O comentário aqui já prometia o `sync_inputs`; ele
        // não existia.)
        //
        // Vale pros commits também: digitar um valor num campo agora reescreve os outros, então mexer
        // no R atualiza o hex, e mexer na luminosidade do OKLCH atualiza tudo. Sem isso os três
        // espaços seriam inúteis — o valor digitado não teria como se mostrar convertido.
        //
        // Não há laço: `set_value` emite `Change`, e estas assinaturas só reagem a `PressEnter`/`Blur`.

        // Trocar o espaço não muda a COR: só o que os três campos mostram. Daí o `sync_inputs`.
        subs.push(cx.subscribe_in(
            &mode_select,
            window,
            |this, _sel, ev: &crate::SelectEvent, window, cx| {
                let crate::SelectEvent::Change(i) = ev;
                this.mode = ColorMode::from_index(*i);
                this.sync_inputs(window, cx);
                cx.notify();
            },
        ));
        // R/G/B: commit no Enter/Blur → re-parseia os 3 → atualiza a cor (preserva alpha).
        for (which, st) in [(0usize, &r_input), (1, &g_input), (2, &b_input)] {
            subs.push(cx.subscribe_in(st, window, move |this, _st, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    this.commit_rgb_inputs(which, cx);
                    this.sync_inputs(window, cx);
                }
            }));
        }
        // Hex: commit → parseia → atualiza (preserva alpha).
        subs.push(cx.subscribe_in(
            &hex_input,
            window,
            |this, _st, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    // O `adopt_rgb` lá dentro já sincroniza os campos.
                    this.commit_hex_input(window, cx);
                }
            },
        ));
        // Opacidade (0–100): commit → atualiza alpha.
        subs.push(cx.subscribe_in(
            &opacity_input,
            window,
            |this, _st, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    this.commit_opacity_input(cx);
                    this.sync_inputs(window, cx);
                }
            },
        ));

        let mut this = Self {
            h: 0.0,
            s: 0.0,
            v: 1.0,
            a: 1.0,
            previous: [1.0, 1.0, 1.0, 1.0],
            recents: load_recents(),
            drag: None,
            sv_bounds: None,
            hue_bounds: None,
            alpha_bounds: None,
            mode: ColorMode::default(),
            campo_focado: None,
            copied_at: None,
            mode_select,
            r_input,
            g_input,
            b_input,
            hex_input,
            opacity_input,
            focus_handle: cx.focus_handle(),
            _subs: subs,
        };
        // A primeira escrita nos campos vem daqui, e não de literais na criação deles: assim o que
        // eles mostram sai SEMPRE do estado e do modo, inclusive no primeiro frame.
        //
        // ⚠️ **Sem teste, e de propósito.** Hoje isto não é observável: com a cor inicial branca e o
        // modo default em RGB, os literais que estavam ali (`"255"`, `"#FFFFFF"`, `"100"`) produzem
        // exatamente o mesmo texto que o `sync_inputs`. Escrevi um teste, ele passou também com os
        // literais de volta, e um teste que passa na própria mutação é pior que nenhum.
        //
        // O que o conserto compra é o futuro: no minuto em que o modo default virar OKLCH — ou a cor
        // inicial deixar de ser branca — os literais passariam a mentir no primeiro frame. Eu VI isso
        // acontecer ao trocar o default pra OKLCH pra conferir a largura dos campos: o Select dizia
        // OKLCH e os campos diziam `255 255 255`.
        this.sync_inputs(window, cx);
        this
    }

    // --- conversões de estado ---

    /// RGBA corrente em `[0,1]` (HSVA → RGBA + alpha).
    pub fn rgba(&self) -> [f32; 4] {
        let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
        [r, g, b, self.a]
    }

    /// **Retarget**: seta o estado a partir de um RGBA `[0,1]` (ao abrir o picker no slot).
    /// Guarda como `previous`, deriva HSVA e re-sincroniza os inputs. **Não** emite evento.
    pub fn set_rgba(&mut self, rgba: [f32; 4], window: &mut Window, cx: &mut Context<Self>) {
        self.previous = rgba;
        let (h, s, v) = rgb_to_hsv(rgba[0], rgba[1], rgba[2]);
        // Preserva o hue corrente quando a cor é acromática (s≈0): senão o hue saltaria pra 0
        // e a barra de hue "pularia" ao abrir num cinza/branco/preto.
        self.h = if s <= 0.0001 { self.h } else { h };
        self.s = s;
        self.v = v;
        self.a = rgba[3];
        self.sync_inputs(window, cx);
        cx.notify();
    }

    /// Os cinco campos, **em ordem estável** — é o índice desta lista que [`Self::campo_focado`]
    /// guarda.
    fn campos(&self) -> [&Entity<InputState>; 5] {
        [
            &self.r_input,
            &self.g_input,
            &self.b_input,
            &self.hex_input,
            &self.opacity_input,
        ]
    }

    /// **Focar um campo seleciona o conteúdo dele**, nos cinco campos deste painel.
    ///
    /// É comportamento DESTE componente, não do [`crate::Input`]: num formulário, focar um campo e
    /// apagar o que estava lá é hostil. Aqui é o contrário — os campos mostram um valor derivado da
    /// cor, e quem clica neles quase sempre quer substituir o número inteiro, não editar um dígito no
    /// meio de `#FF8000`. É também o que faz a máscara do `%` funcionar sem atrapalhar: digitar por
    /// cima da seleção troca `100%` por `50` de uma vez.
    ///
    /// # Por que no render, e não numa assinatura de `InputEvent::Focus`
    ///
    /// Foi a primeira tentativa, e ela **não é verificável**: no `TestAppContext` o `Focus` nunca
    /// chega. Medi — com um clique que comprovadamente focou o campo (o caractere digitado entrou
    /// nele), a assinatura recebeu só `Change`. O `on_focus` do núcleo depende do CAMINHO de foco, que
    /// é reconstruído no desenho, e nesse ambiente ele não fecha no campo.
    ///
    /// Em produção o evento funciona (o log de eventos do storybook mostra Focus). Mas entre uma
    /// implementação que funciona e não dá pra testar, e uma que funciona e dá, a segunda é melhor:
    /// comparar o foco com o do frame anterior custa cinco `is_focused` por render e é observável.
    ///
    /// A ordem também sai de graça. O campo posiciona o cursor no próprio mouse-down; como esta
    /// checagem roda no render seguinte, a seleção vem depois e sobrevive ao clique.
    fn selecionar_ao_focar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let agora = self
            .campos()
            .iter()
            .position(|st| gpui::Focusable::focus_handle(st.read(cx), cx).is_focused(window));
        if agora == self.campo_focado {
            return;
        }
        self.campo_focado = agora;
        if let Some(i) = agora {
            let campo = self.campos()[i].clone();
            campo.update(cx, |st, cx| st.select_all_now(cx));
        }
    }

    /// Re-sincroniza os 5 inputs (R/G/B/Hex/opacidade) com o estado HSVA corrente. Sem emitir.
    ///
    /// **O campo que está com o foco não é reescrito** — ver [`escreve`].
    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
        // Os três campos mostram o espaço escolhido no Select. O hex e a opacidade não mudam de
        // significado, então ficam fora dessa decisão.
        let (a, b_, c) = match self.mode {
            ColorMode::Rgb => {
                let to255 = |c: f32| ((c.clamp(0.0, 1.0) * 255.0).round() as i32).to_string();
                (to255(r), to255(g), to255(b))
            }
            ColorMode::Hsl => {
                // O matiz vem do ESTADO, não de uma volta por RGB: HSV e HSL compartilham o matiz, e
                // reconvertê-lo faria o valor escorregar a cada tecla.
                let (s_hsl, l) = hsv_to_hsl(self.s, self.v);
                (fmt_deg(self.h), fmt_pct(s_hsl), fmt_pct(l))
            }
            ColorMode::Oklch => {
                let (l, croma, matiz) = rgb_to_oklch(r, g, b);
                (fmt_pct_1(l), fmt_chroma(croma), fmt_deg(matiz))
            }
        };
        escreve(&self.r_input, a, window, cx);
        escreve(&self.g_input, b_, window, cx);
        escreve(&self.b_input, c, window, cx);
        escreve(&self.hex_input, hex_from_rgb(r, g, b), window, cx);
        // O **`%` viaja com o valor**, exatamente como o `#` do hex neste mesmo painel: é o que dá o
        // efeito de máscara sem precisar de máscara de verdade. O commit já o remove
        // (`trim_end_matches('%')`), e o foco selecionando tudo faz digitar por cima trocar os dois de
        // uma vez.
        //
        // O `MaskPattern` do núcleo NÃO serve aqui: ele é posicional e de largura fixa (`999%`), então
        // 50 apareceria como `50_%` e pediria três dígitos. Máscara de largura fixa é pra telefone e
        // data, não pra um número de 0 a 100.
        let pct = (self.a.clamp(0.0, 1.0) * 100.0).round() as i32;
        escreve(&self.opacity_input, format!("{pct}%"), window, cx);
    }

    /// Emite `Changed` com o RGBA corrente (live update do inspector).
    fn emit_changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(ColorPickerEvent::Changed(self.rgba()));
    }

    // --- commits dos inputs editáveis ---

    fn commit_rgb_inputs(&mut self, _which: usize, cx: &mut Context<Self>) {
        let num = |st: &Entity<InputState>| -> Option<f32> {
            st.read(cx).value().trim().parse::<f32>().ok()
        };
        let (Some(x), Some(y), Some(z)) =
            (num(&self.r_input), num(&self.g_input), num(&self.b_input))
        else {
            return;
        };
        match self.mode {
            ColorMode::Rgb => {
                let f = |v: f32| (v / 255.0).clamp(0.0, 1.0);
                let (h, s, v) = rgb_to_hsv(f(x), f(y), f(z));
                // Cinza não tem matiz definido: preserva o que estava, senão o SV pularia pro
                // vermelho ao digitar `0,0,0`.
                self.h = if s <= 0.0001 { self.h } else { h };
                self.s = s;
                self.v = v;
            }
            ColorMode::Hsl => {
                // Matiz direto (é o mesmo dos dois espaços) e S/L por álgebra: sem volta por RGB,
                // sem arredondar duas vezes.
                self.h = x.rem_euclid(360.0);
                let (s, v) = hsl_to_hsv(y / 100.0, z / 100.0);
                self.s = s;
                self.v = v;
            }
            ColorMode::Oklch => {
                // Aqui a volta por RGB é obrigatória: o estado interno é HSV, que é sRGB, e OKLCH não
                // tem atalho pra ele. Croma fora do gamute volta aparado (ver `oklch_to_rgb`).
                let (r, g, b) = oklch_to_rgb((x / 100.0).clamp(0.0, 1.0), y.clamp(0.0, MAX_CHROMA), z.rem_euclid(360.0));
                let (h, s, v) = rgb_to_hsv(r, g, b);
                self.h = if s <= 0.0001 { self.h } else { h };
                self.s = s;
                self.v = v;
            }
        }
        self.emit_changed(cx);
        cx.notify();
    }

    /// **Adota um RGB vindo de fora**: hex digitado, swatch clicado, cor amostrada da tela.
    ///
    /// Duas coisas que ela NÃO faz, e são o motivo de existir separada do [`Self::set_rgba`]:
    ///
    /// - **não mexe no `previous`** — aquele é a cor de quando o painel abriu, e é o que dá sentido à
    ///   comparação Previous/New. Um conta-gotas que reescrevesse o `previous` apagaria justamente a
    ///   referência que o usuário quer comparar.
    /// - **não mexe no alpha** — a opacidade é decisão de outro controle. A tela devolve um pixel já
    ///   composto, então o alpha dela não diria nada sobre a intenção de quem escolheu.
    fn adopt_rgb(&mut self, r: f32, g: f32, b: f32, window: &mut Window, cx: &mut Context<Self>) {
        let (h, s, v) = rgb_to_hsv(r, g, b);
        // Preserva o matiz nos acromáticos: senão pegar um branco jogaria a barra de matiz pro
        // vermelho, e o próximo arraste no quadrado sairia de uma cor que o usuário não escolheu.
        self.h = if s <= 0.0001 { self.h } else { h };
        self.s = s;
        self.v = v;
        self.sync_inputs(window, cx);
        self.emit_changed(cx);
        cx.notify();
    }

    fn commit_hex_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let txt = self.hex_input.read(cx).value();
        let Some((r, g, b)) = parse_hex(&txt) else {
            return;
        };
        self.adopt_rgb(r, g, b, window, cx);
    }

    fn commit_opacity_input(&mut self, cx: &mut Context<Self>) {
        let txt = self.opacity_input.read(cx).value();
        let Some(p) = txt.trim().trim_end_matches('%').parse::<f32>().ok() else {
            return;
        };
        self.a = (p / 100.0).clamp(0.0, 1.0);
        self.emit_changed(cx);
        cx.notify();
    }

    // --- arraste dos handles (SV / hue / alpha) ---

    fn drag_move(&mut self, pos: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = self.drag else { return };
        match drag {
            Drag::Sv => {
                if let Some(b) = self.sv_bounds {
                    let fx = ((pos.x - b.origin.x) / b.size.width).clamp(0.0, 1.0);
                    let fy = ((pos.y - b.origin.y) / b.size.height).clamp(0.0, 1.0);
                    self.s = fx;
                    self.v = 1.0 - fy;
                }
            }
            Drag::Hue => {
                if let Some(b) = self.hue_bounds {
                    let fy = ((pos.y - b.origin.y) / b.size.height).clamp(0.0, 1.0);
                    self.h = fy * 360.0;
                }
            }
            Drag::Alpha => {
                if let Some(b) = self.alpha_bounds {
                    let fy = ((pos.y - b.origin.y) / b.size.height).clamp(0.0, 1.0);
                    self.a = 1.0 - fy;
                }
            }
        }
        // Emite o `Changed` (live) e re-renderiza (chips/preview/handle). Os campos numéricos
        // editáveis (R/G/B/Hex/opacidade) precisam de `&mut Window` pra `set_value` → quem chama
        // `drag_move` com um Window à mão (mouse-down / mouse-move dos handles) faz `sync_inputs`
        // logo após. Aqui só o estado + emit + notify.
        self.emit_changed(cx);
        cx.notify();
    }

    // --- ações da coluna direita ---

    /// Copia o hex `#RRGGBB` corrente pro clipboard.
    fn copy_hex(&mut self, cx: &mut Context<Self>) {
        let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
        let hex = hex_from_rgb(r, g, b);
        cx.write_to_clipboard(ClipboardItem::new_string(hex));
        // Marca o instante: o render decide o ícone pelo tempo decorrido e pede os frames até voltar.
        self.copied_at = Some(std::time::Instant::now());
        cx.notify();
    }

    /// Se o botão de copiar deve estar mostrando o check agora.
    fn copiado_agora(&self) -> bool {
        self.copied_at.is_some_and(|t| t.elapsed() < COPIED_FEEDBACK)
    }

    /// Fecha o picker: persiste a cor corrente nas recentes e emite `Closed`.
    fn close(&mut self, cx: &mut Context<Self>) {
        self.push_recent();
        cx.emit(ColorPickerEvent::Closed);
    }

    /// Insere a cor corrente no topo das recentes (unshift + dedup + cap), e persiste.
    fn push_recent(&mut self) {
        let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
        let hex = hex_from_rgb(r, g, b);
        self.recents.retain(|c| !c.eq_ignore_ascii_case(&hex));
        self.recents.insert(0, hex);
        self.recents.truncate(MAX_RECENTS);
        save_recents(&self.recents);
    }

    /// Os 27 swatches a exibir no rodapé: **recentes** (mais nova primeiro, dedup) seguidas da
    /// **paleta preset** (em sequência), preenchendo 27 slots sem duplicar.
    fn swatch_list(&self) -> Vec<String> {
        build_swatch_list(&self.recents)
    }

    /// Aplica um swatch (hex) → estado + emite `Changed`. Alpha preservado.
    fn apply_swatch(&mut self, hex: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((r, g, b)) = parse_hex(hex) {
            self.adopt_rgb(r, g, b, window, cx);
        }
    }

    // --- render helpers ---

    /// O quadrado SV: 3 divs sobrepostos (hue puro / branco→transparente / transparente→preto)
    /// + handle em (s, 1−v). Captura bounds via `canvas` (1º filho) pro mapeamento do drag.
    fn render_sv(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let hue = hue_pure_hsla(self.h);
        let transparent = gpui::Rgba { r: 1.0, g: 1.0, b: 1.0, a: 0.0 };
        let white = gpui::Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let black = gpui::Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

        let entity = cx.entity();
        let capture = canvas(
            move |bounds, _, app| {
                entity.update(app, |this, _| this.sv_bounds = Some(bounds));
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        // A alça é ancorada por FRAÇÃO, não por pixel: o quadrado é elástico agora, e uma alça em
        // `s * LADO` precisaria do lado medido — que não existe no primeiro frame (armadilha nº 14).
        // O ancorador tem tamanho zero e fica na fração exata; a alça pendura nele recuada de meio
        // lado, que é o jeito de somar fração com pixel sem um `calc` que o GPUI não tem.
        // O miolo da alça é PREENCHIDO com a cor escolhida, não vazado.
        //
        // Vazado ele funciona no meio do quadrado (o miolo mostra a cor que está atrás, que é a
        // escolhida), mas quebra exatamente onde a alça passou a aparecer inteira: numa quina, metade
        // do miolo cai fora do quadrado e mostra o fundo do painel — um anel partido em vez de uma
        // bolinha. Preenchido, ele mostra a mesma cor nas duas situações.
        let miolo = {
            let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
            gpui::Rgba { r, g, b, a: 1.0 }
        };
        let handle = div()
            .absolute()
            .left(gpui::relative(self.s.clamp(0.0, 1.0)))
            .top(gpui::relative(1.0 - self.v.clamp(0.0, 1.0)))
            .size(px(0.0))
            .child(
                div()
                    .absolute()
                    .left(px(-SV_HANDLE / 2.0))
                    .top(px(-SV_HANDLE / 2.0))
                    .size(px(SV_HANDLE))
                    .rounded_full()
                    .bg(miolo)
                    .border_2()
                    .border_color(palette().handle_ring.hsla())
                    .shadow_sm(),
            );

        let superficie = div()
            .id("cp-sv")
            .relative()
            .size_full()
            .rounded(px(SURFACE_RADIUS))
            .overflow_hidden()
            .bg(hue)
            // ⚠️ O RAIO VAI EM CADA CAMADA, não só no pai. O `overflow_hidden` do GPUI é máscara
            // RETANGULAR (armadilha nº 15), então um gradiente em `inset_0` pinta por cima das
            // quinas arredondadas do pai. Era o defeito visível: das quatro quinas do quadrado só a
            // superior direita parecia redonda — justamente a única onde os DOIS gradientes são
            // transparentes e o fundo do pai aparecia.
            // branco → transparente (esquerda → direita) = ângulo 90°.
            .child(layer(SURFACE_RADIUS).bg(linear_gradient(
                90.0,
                linear_color_stop(white, 0.0),
                linear_color_stop(transparent, 1.0),
            )))
            // transparente → preto (cima → baixo) = ângulo 180°.
            .child(layer(SURFACE_RADIUS).bg(linear_gradient(
                180.0,
                linear_color_stop(transparent, 0.0),
                linear_color_stop(black, 1.0),
            )))
            .child(capture)
            .cursor(CursorStyle::Crosshair)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.drag = Some(Drag::Sv);
                    this.drag_move(e.position, cx);
                    this.sync_inputs(window, cx);
                }),
            );

        // **A alça é IRMÃ da superfície, não filha.**
        //
        // A superfície tem `overflow_hidden` pra os gradientes não escaparem, e a máscara do GPUI é
        // retangular: como filha, a alça era cortada ao meio sempre que a cor ia pra uma borda. Do
        // lado de fora do recorte ela aparece inteira, sobrando pra fora do quadrado — que é o que se
        // espera de um alvo de arraste (armadilha nº 6 da casa: `overflow_hidden` recorta filho
        // absoluto, então quem precisa passar por cima da borda tem que ser irmão).
        //
        // Ela vem DEPOIS da superfície, então pinta por cima dela. E não intercepta o clique: um `div`
        // sem `id` nem handler não cria hitbox no GPUI, então o mouse-down atravessa e chega na
        // superfície atrás — clicar na própria alça continua começando um arraste.
        //
        // Quem é elástico na linha agora é este container; a superfície preenche ele.
        div()
            .relative()
            // Quadrado, e o lado é a altura da coluna direita (ver `SV_SIDE`). Não é mais `flex_1`:
            // elástico ele acompanhava a sobra da linha e saía retângulo.
            .size(px(SV_SIDE))
            .flex_none()
            .child(superficie)
            .child(handle)
    }

    /// Barra de HUE (vertical, arco-íris): 6 segmentos de 2 stops empilhados + handle em h/360.
    fn render_hue(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // 7 paradas do arco-íris (red→…→red), cada par vira 1 segmento de 1/6 da altura.
        let stops = [0.0f32, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0];
        let mut col = div().absolute().inset_0().flex().flex_col();
        for i in 0..HUE_SEGMENTS {
            let from = hue_pure_hsla(stops[i]);
            let to = hue_pure_hsla(stops[i + 1]);
            // O raio vai nos segmentos das PONTAS, pelo mesmo motivo do quadrado SV: a máscara do
            // `overflow_hidden` é retangular, então quem arredonda a barra são as camadas, não o pai.
            col = col.child(
                div()
                    .flex_1()
                    .when(i == 0, |d| d.rounded_t(px(SURFACE_RADIUS)))
                    .when(i == HUE_SEGMENTS - 1, |d| d.rounded_b(px(SURFACE_RADIUS)))
                    .bg(linear_gradient(
                        180.0,
                        linear_color_stop(from, 0.0),
                        linear_color_stop(to, 1.0),
                    )),
            );
        }

        // **Os filetes de costura.** Seis segmentos de `flex_1` dividem a altura em sextos que quase
        // nunca são inteiros (aqui a barra tem ~182px → 30,25 cada), e numa das cinco fronteiras a
        // conta cai em meio pixel de dispositivo: nenhum dos dois quads cobre aquela linha e aparece
        // uma risca de 1px com a cor do que está atrás. É um defeito ANTIGO — com a barra de 167 fixos
        // ele estava noutra altura, só isso.
        //
        // O conserto é fechar cada fronteira com um filete da cor EXATA dela. Onde não há falha o
        // filete é invisível (a cor é a mesma que o gradiente já entrega ali); onde há, ele tapa com a
        // cor certa. Nada de esticar o gradiente pra sobrepor os vizinhos, que resolveria a risca à
        // custa de deslocar o matiz na emenda.
        for (i, fronteira) in stops.iter().enumerate().take(HUE_SEGMENTS).skip(1) {
            col = col.child(
                div()
                    .absolute()
                    .top(gpui::relative(i as f32 / HUE_SEGMENTS as f32))
                    .left_0()
                    .right_0()
                    .h(px(0.0))
                    .child(
                        div()
                            .absolute()
                            .top(px(-SEAM_PATCH / 2.0))
                            .left_0()
                            .right_0()
                            .h(px(SEAM_PATCH))
                            .bg(hue_pure_hsla(*fronteira)),
                    ),
            );
        }

        let entity = cx.entity();
        let capture = canvas(
            move |bounds, _, app| {
                entity.update(app, |this, _| this.hue_bounds = Some(bounds));
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        // O miolo mostra o VALOR desta coluna na posição da alça: o matiz puro do `h` corrente.
        let handle = bar_handle(
            (self.h / 360.0).clamp(0.0, 1.0),
            bar_handle_fill(hue_pure_hsla(self.h)),
        );

        let barra = div()
            .id("cp-hue")
            .relative()
            .size_full()
            .rounded(px(SURFACE_RADIUS))
            .overflow_hidden()
            .child(col)
            .child(capture)
            .cursor(CursorStyle::PointingHand)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.drag = Some(Drag::Hue);
                    this.drag_move(e.position, cx);
                    this.sync_inputs(window, cx);
                }),
            );

        // A alça é IRMÃ da barra, pelo mesmo motivo da bolinha do quadrado SV: a barra tem
        // `overflow_hidden` pros segmentos não escaparem, e a máscara do GPUI é retangular — como
        // filha, a alça era cortada nos 2px que sobram de cada lado E pela metade nos extremos do
        // curso. Fora do recorte ela aparece inteira (armadilha nº 6 da casa).
        div()
            .relative()
            .w(px(BAR_W))
            .flex_none()
            .child(barra)
            .child(handle)
    }

    /// Barra de ALPHA (vertical): xadrez sutil + (cor opaca no topo → transparente embaixo) + handle.
    fn render_alpha(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
        let opaque = gpui::Rgba { r, g, b, a: 1.0 };
        let transparent = gpui::Rgba { r, g, b, a: 0.0 };

        // Xadrez sutil: fundo cinza claro + linhas (pattern slash) — aproximação leve do checkerboard.
        // As duas camadas do xadrez levam o raio (o `overflow_hidden` não arredonda — ver o SV).
        let checker = div()
            .absolute()
            .inset_0()
            .rounded(px(SURFACE_RADIUS))
            .bg(rgb(0xCCCCCC))
            .child(
                layer(SURFACE_RADIUS).bg(gpui::pattern_slash(rgba(0x80808055).into(), 3.0, 3.0)),
            );

        let entity = cx.entity();
        let capture = canvas(
            move |bounds, _, app| {
                entity.update(app, |this, _| this.alpha_bounds = Some(bounds));
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        // O miolo mostra o VALOR desta coluna: a cor corrente NO alpha corrente. Como ela é
        // translúcida, vai sobre o mesmo xadrez da barra — sem ele, um alpha baixo apareceria contra o
        // fundo do painel e leria como "quase preto" em vez de "quase transparente".
        let cor_no_alpha = {
            let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
            gpui::Rgba { r, g, b, a: self.a.clamp(0.0, 1.0) }
        };
        let handle = bar_handle(
            (1.0 - self.a).clamp(0.0, 1.0),
            bar_handle_fill(rgb(0xCCCCCC))
                .child(
                    layer(BAR_HANDLE_INNER_RADIUS)
                        .bg(gpui::pattern_slash(rgba(0x80808055).into(), 3.0, 3.0)),
                )
                .child(layer(BAR_HANDLE_INNER_RADIUS).bg(cor_no_alpha)),
        );

        let barra = div()
            .id("cp-alpha")
            .relative()
            .size_full()
            .rounded(px(SURFACE_RADIUS))
            .overflow_hidden()
            .child(checker)
            .child(layer(SURFACE_RADIUS).bg(linear_gradient(
                180.0,
                linear_color_stop(opaque, 0.0),
                linear_color_stop(transparent, 1.0),
            )))
            .child(capture)
            .cursor(CursorStyle::PointingHand)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.drag = Some(Drag::Alpha);
                    this.drag_move(e.position, cx);
                    this.sync_inputs(window, cx);
                }),
            );

        // Alça irmã da barra — ver o mesmo arranjo em `render_hue`.
        div()
            .relative()
            .w(px(BAR_W))
            .flex_none()
            .child(barra)
            .child(handle)
    }


    /// Um swatch 16px clicável (hex). Clicar seta a cor.
    fn swatch(&self, hex: String, cx: &mut Context<Self>) -> impl IntoElement {
        let packed = parse_hex(&hex)
            .map(|(r, g, b)| {
                let to8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
                (to8(r) << 24) | (to8(g) << 16) | (to8(b) << 8) | 0xFF
            })
            .unwrap_or(0xFFFFFFFF);
        let hex_for_click = hex.clone();
        div()
            .id(SharedString::from(format!("cp-swatch-{hex}")))
            .flex_none()
            .size(px(16.0))
            .rounded(px(SWATCH_RADIUS))
            .border_1()
            .border_color(palette().border.hsla())
            .bg(rgba(packed))
            .cursor(CursorStyle::PointingHand)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _e: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.apply_swatch(&hex_for_click, window, cx);
                }),
            )
    }

    /// Bloco de preview Previous / New (duas metades) + a linha de labels.
    ///
    /// Esquerda = a cor de QUANDO O PICKER ABRIU (`previous`); direita = a cor ATUAL
    /// (`new`). Bloco `w 201` × `h 39`, `rounded 4`, `overflow-clip`. Labels "Previous"/"New"
    /// `#999999` 10px, cada um `flex-1 text-center`.
    fn render_preview(&self) -> impl IntoElement {
        let pack = |c: [f32; 4]| {
            let to8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
            (to8(c[0]) << 24) | (to8(c[1]) << 16) | (to8(c[2]) << 8) | to8(c[3])
        };
        let prev = pack(self.previous);
        let new = pack(self.rgba());
        // `--muted-foreground` no lugar do `#999999` cru da referência: é a mesma leitura de "legenda",
        // mas acompanha o tema como o resto da biblioteca. Entrelinha explícita porque a altura deste
        // bloco vem do conteúdo — sem ela o GPUI usaria a razão de ouro (16,2px) em vez dos 15 do
        // `line-height: 1.5` que um `text-[10px]` herda no Tailwind.
        let label = |txt: &'static str| {
            div()
                .flex_1()
                .text_center()
                .text_size(px(10.0))
                .line_height(px(PREVIEW_LABEL_LH))
                .text_color(palette().text_muted.hsla())
                .child(txt)
        };
        div()
            .flex()
            .flex_col()
            .gap(px(PREVIEW_GAP))
            .w_full()
            .child(
                // As 2 metades. **O raio vai em cada metade**, não no pai: o `overflow_hidden` do
                // GPUI é máscara retangular (armadilha nº 15), então metades quadradas pintavam por
                // cima das quinas arredondadas do bloco — era o mesmo defeito do quadrado SV, e aqui
                // ele deixava as QUATRO quinas quadradas, porque as duas metades são opacas.
                //
                // Cada metade arredonda só a ponta que é ponta do bloco: `previous` a esquerda, `new`
                // a direita. As duas bordas que se encontram no meio ficam retas, e é isso que faz o
                // par ler como um bloco só partido ao meio, e não como dois chips soltos.
                div()
                    .flex()
                    .w_full()
                    .h(px(PREVIEW_H))
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .rounded_l(px(SURFACE_RADIUS))
                            .bg(rgba(prev)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .rounded_r(px(SURFACE_RADIUS))
                            .bg(rgba(new)),
                    ),
            )
            .child(
                // Labels Previous/New: `justify-between self-stretch`, cada `flex-1 text-center`.
                div()
                    .flex()
                    .w_full()
                    .justify_between()
                    .child(label("Previous"))
                    .child(label("New")),
            )
    }
}

impl Render for ColorPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Antes de montar: quem acabou de receber o foco tem o conteúdo selecionado.
        self.selecionar_ao_focar(window, cx);

        // --- Coluna direita: preview + modo/conta-gotas + valores ------------------------------
        //
        // A fileira de ações que existia aqui em cima saiu inteira. Cada botão dela foi pra onde
        // pertence, e um deixou de existir:
        //
        // - **fechar**: removido. Um popover fecha clicando fora ou com `Escape`, e os dois caminhos
        //   continuam ligados — um X só ocupava o canto pra repetir o que o gesto já faz.
        // - **conta-gotas**: foi pro lado do Select de modo, junto dos controles que decidem a cor.
        // - **copiar**: é o último item do grupo do hex — a ação fica na mesma fileira dos valores que
        //   ela copia, costurada a eles. (Passou um tempo DENTRO do campo, no lugar do olho da senha;
        //   como item do grupo ela ganha superfície própria e uma parada de Tab, o que é mais honesto
        //   pra uma ação que não é do campo, é da cor.)

        // **O nosso [`crate::Select`]** numa linha própria, no lugar do `div` inerte que só desenhava
        // "RGB" e um chevron. Ele traz o menu ancorado, o teclado, o anel de foco e — desde esta
        // sessão — fechar e desfocar ao clicar fora. E linha própria porque o gatilho tem `min-w-36`:
        // ao lado dos três campos ele não caberia, e é exatamente o que estourava o layout antes.
        let mode_row = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .w_full()
            .child(div().flex_1().min_w(px(0.0)).child(self.mode_select.clone()))
            // Conta-gotas, ao lado do Select: os dois decidem a cor, então ficam juntos.
            //
            // **Só aparece onde funciona** (ver [`crate::eyedropper::is_supported`]). É o que a
            // referência faz no navegador: sem `window.EyeDropper`, o botão não é desenhado — melhor
            // que um botão que não faz nada.
            .when(crate::eyedropper::is_supported(), |d| {
                d.child(
                    crate::Button::icon("cp-eyedropper", "icons/pipette.svg")
                        .variant(crate::ButtonVariant::Ghost)
                        .size(crate::ButtonSize::IconSm)
                        .on_click(cx.listener(|_this, _e, window, cx| {
                            // O handle da janela vai capturado porque a cor chega DEPOIS, num callback
                            // do sistema que só tem `&mut App` — e aplicar a cor precisa de `Window`
                            // (o `sync_inputs` escreve nos campos). Reentrar pelo handle é a ponte.
                            let entity = cx.entity();
                            let janela = window.window_handle();
                            crate::eyedropper::pick(cx, move |rgb, app| {
                                let Some([r, g, b]) = rgb else {
                                    return; // cancelou: nada muda
                                };
                                let _ = janela.update(app, |_, window, cx| {
                                    entity.update(cx, |this, cx| this.adopt_rgb(r, g, b, window, cx));
                                });
                            });
                        })),
                )
            });

        // **Os três campos são um [`crate::Group`].**
        //
        // Eles são três recortes de UM valor — a mesma cor em três canais —, não três controles
        // independentes, e é isso que o grupo diz na tela: uma peça só, com o raio nas pontas e
        // filetes de 1px nas emendas. O `full_width` faz os três dividirem a linha em partes iguais.
        //
        // O do meio é o único costurado dos dois lados, então ele perde as duas bordas e sobra 2px
        // mais estreito por dentro. Com o número alinhado à esquerda não se vê; se um dia estes
        // valores forem centralizados, é aqui que a assimetria aparece.
        let rgb_row = crate::group::Group::new()
            .full_width()
            .child(crate::Input::new(&self.r_input).size(crate::InputSize::Sm))
            .child(crate::Input::new(&self.g_input).size(crate::InputSize::Sm))
            .child(crate::Input::new(&self.b_input).size(crate::InputSize::Sm));

        // **Hex + opacidade + copiar, num [`crate::Group`] só.**
        //
        // O copiar saiu do SUFIXO do campo hex e virou o último item do grupo. A leitura muda de
        // "uma ação dentro do campo" pra "três peças da mesma fileira": os dois valores que descrevem
        // a cor e a ação que os leva embora, costurados numa peça com filetes de 1px nas emendas.
        //
        // Como cada filho se comporta na largura, com `full_width`:
        //
        // - **hex**: elástico (`Width::Full`), fica com a sobra.
        // - **opacidade**: [`OPACITY_WIDTH`] fixo — é sempre 0–100.
        // - **copiar**: quadrado de 28, porque um `Button` é `flex_none` e não estica. Aqui isso é o
        //   que se quer; num grupo de botões `full_width` seria um vão sobrando no fim.
        //
        // A altura NÃO é fixada nesta linha: quem manda nela é o `size` dos filhos, e o `IconSm` do
        // botão (28) casa com o `Sm` dos campos (28). O `h 24` que esta linha tinha vinha da
        // referência, onde os campos eram `div`s de 24px desenhados à mão.
        let hex_row = crate::group::Group::new()
            .full_width()
            .child(crate::Input::new(&self.hex_input).size(crate::InputSize::Sm))
            .child(
                crate::Input::new(&self.opacity_input)
                    .size(crate::InputSize::Sm)
                    .width(px(OPACITY_WIDTH)),
            )
            // `Outline` porque é o que a referência usa em botão costurado a campo (`p-group-2`): ele
            // tem superfície própria, como os vizinhos. O ícone vira check por 1 segundo depois de
            // copiar — o retorno vem do tempo decorrido, ver `copiado_agora`.
            .child(
                crate::Button::icon(
                    "cp-copy",
                    if self.copiado_agora() {
                        "icons/check.svg"
                    } else {
                        "icons/copy.svg"
                    },
                )
                .variant(crate::ButtonVariant::Outline)
                .size(crate::ButtonSize::IconSm)
                // O botão continua com 28 pra casar com a altura dos campos vizinhos; só o GLIFO
                // encolhe. Ver `crate::ButtonSize::icon_size` pro tamanho padrão de cada tamanho.
                .icon_size(px(COPY_ICON))
                .on_click(cx.listener(|this, _e, _w, cx| this.copy_hex(cx))),
            );

        // Coluna direita: `flex-1` (preenche o resto do painel), `flex-col items-end gap 14`.
        // TODOS os blocos internos são `self-stretch` (preenchem a largura da coluna).
        let right_col = div()
            .flex()
            .flex_col()
            .items_end()
            .gap(px(ROW_GAP))
            .w(px(RIGHT_COL_WIDTH))
            .flex_none()
            .child(self.render_preview())
            .child(mode_row)
            .child(rgb_row)
            .child(hex_row);

        // --- Rodapé: swatches = recentes (na frente) + paleta do design, 27 slots, 2 linhas ---
        // `flex-col gap-2 w-105` (420px). Linha1 = 18 swatches `justify-between self-stretch`.
        // Linha2 = 9 swatches `gap 7.9px`. Cada swatch `size-4` (16px) `rounded-[3px] shrink-0`.
        let swatches = self.swatch_list();
        let mut line1 = div().flex().items_start().justify_between().w_full();
        for hex in swatches.iter().take(18) {
            line1 = line1.child(self.swatch(hex.clone(), cx));
        }
        let mut line2 = div().flex().items_start().gap(px(7.9));
        for hex in swatches.iter().skip(18).take(9) {
            line2 = line2.child(self.swatch(hex.clone(), cx));
        }
        let footer = div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .w(px(420.0))
            .child(line1)
            .child(line2);

        // --- A superfície é um `Card` levantado -------------------------------------------------
        //
        // Antes era um `div` cru: `BG_PANEL` + `border_subtle` + raio 8 + uma sombra escrita à mão de
        // **preto a 55%** com blur 32 e spread 2. Onze vezes o alfa de qualquer outra sombra da
        // biblioteca — e era isso que ficava estranho no tema escuro.
        //
        // Agora quem desenha a superfície é o [`crate::card::Card`]: fundo `--card`, borda `--border`,
        // raio 16, bisel, e a sombra `shadow-lg/5` via [`Card::elevated`] — o mesmo par de camadas que
        // o menu, o toast e o dialog usam. O `compact` aperta o respiro pra 16px, porque 24 em volta
        // de controles de 24px deixaria a peça vazia.
        //
        // O `div` externo continua existindo, mas só pra COMPORTAMENTO: foco, `occlude`, arraste e
        // teclas. Ele não pinta nada.
        let conteudo = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .w_full()
            // Linha principal: SV + alpha + hue + coluna direita.
            .child(
                // **Sem `items_start`**: o `align-items` default do flex é `stretch`, e é ele que dá
                // ao quadrado SV e às duas barras a altura da coluna direita. Com `items_start` os
                // três ficavam no tamanho que declarassem — e eles declaravam um quadrado fixo.
                //
                // Quem define a altura da linha, então, é a coluna da direita (preview + as três
                // fileiras + os vãos). Ninguém aqui fixa altura nenhuma.
                div()
                    .flex()
                    .gap(px(COL_GAP))
                    .w_full()
                    .child(self.render_sv(cx))
                    .child(self.render_alpha(cx))
                    .child(self.render_hue(cx))
                    .child(right_col),
            )
            .child(footer);

        // O check do botão de copiar volta a ser a prancheta **pelo tempo decorrido** — ninguém agenda
        // o retorno. Então enquanto ele está aceso alguém tem que pedir o frame seguinte, senão o
        // ícone congela no check até o próximo redesenho por outro motivo (armadilha nº 12 da casa).
        //
        // Aqui é `request_animation_frame` e não `on_next_frame` + `refresh`: o `ColorPicker` é uma
        // view própria, então a "view corrente" durante este render é ele mesmo — é exatamente a view
        // que precisa ser notificada. (O `dialog`/`sheet` precisam do `refresh` porque lá o elemento é
        // montado por outra view.)
        if self.copiado_agora() {
            window.request_animation_frame();
        }

        div()
            .key_context("ColorPicker")
            .track_focus(&self.focus_handle)
            .occlude()
            .w(px(PANEL_WIDTH))
            // Clicar FORA do popover fecha (emite `Closed` via `close`).
            .on_mouse_down_out(cx.listener(|this, _e, _w, cx| {
                this.close(cx);
            }))
            .child(
                crate::card::Card::new()
                    .elevated()
                    .compact()
                    .width(px(PANEL_WIDTH))
                    .panel(conteudo),
            )
            // Arraste de handle: o `occlude()` faz o mouse-move/up sobre o popover ficarem aqui
            // (não vazam). Move → atualiza o handle ativo + sincroniza inputs; up → encerra o drag.
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, window, cx| {
                if this.drag.is_some() {
                    this.drag_move(e.position, cx);
                    // Sincroniza os campos numéricos durante o drag (precisa do Window).
                    this.sync_inputs(window, cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _e: &gpui::MouseUpEvent, _w, cx| {
                    this.drag = None;
                    cx.notify();
                }),
            )
            // Esc → fecha.
            .on_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, _w, cx| {
                if e.keystroke.key == "escape" {
                    this.close(cx);
                    cx.stop_propagation();
                }
            }))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::assertions_on_constants)]
    use super::*;

    pub(super) fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn hsv_rgb_roundtrip_primaries() {
        // Vermelho, verde, azul puros.
        for (h, s, v) in [(0.0, 1.0, 1.0), (120.0, 1.0, 1.0), (240.0, 1.0, 1.0)] {
            let (r, g, b) = hsv_to_rgb(h, s, v);
            let (h2, s2, v2) = rgb_to_hsv(r, g, b);
            assert!(approx(h, h2), "h {h} != {h2}");
            assert!(approx(s, s2));
            assert!(approx(v, v2));
        }
    }

    #[test]
    fn hsv_rgb_roundtrip_random() {
        for h in [10.0f32, 47.0, 200.0, 305.0] {
            for s in [0.2f32, 0.6, 0.95] {
                for v in [0.3f32, 0.7, 1.0] {
                    let (r, g, b) = hsv_to_rgb(h, s, v);
                    let (h2, s2, v2) = rgb_to_hsv(r, g, b);
                    assert!(approx(s, s2), "s {s} != {s2} (h={h})");
                    assert!(approx(v, v2), "v {v} != {v2}");
                    // hue só faz sentido com s,v > 0.
                    if s > 0.01 && v > 0.01 {
                        let dh = (h - h2).abs().min((h - h2).abs() - 360.0).abs();
                        assert!(dh < 0.5 || approx(h, h2), "h {h} != {h2}");
                    }
                }
            }
        }
    }

    #[test]
    fn hsv_known_values() {
        // Branco = s0 v1; preto = v0; cinza médio.
        assert_eq!(hsv_to_rgb(0.0, 0.0, 1.0), (1.0, 1.0, 1.0));
        assert_eq!(hsv_to_rgb(0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
        let (r, g, b) = hsv_to_rgb(0.0, 0.0, 0.5);
        assert!(approx(r, 0.5) && approx(g, 0.5) && approx(b, 0.5));
    }

    #[test]
    fn parse_hex_forms() {
        assert_eq!(parse_hex("#FFFFFF"), Some((1.0, 1.0, 1.0)));
        assert_eq!(parse_hex("000000"), Some((0.0, 0.0, 0.0)));
        assert_eq!(parse_hex("#FFF"), Some((1.0, 1.0, 1.0)));
        let red = parse_hex("#FF0000").unwrap();
        assert!(approx(red.0, 1.0) && approx(red.1, 0.0) && approx(red.2, 0.0));
        assert_eq!(parse_hex("#GGGGGG"), None);
        assert_eq!(parse_hex("xyz"), None);
        assert_eq!(parse_hex("#1234"), None);
    }

    #[test]
    fn hex_from_rgb_roundtrip() {
        assert_eq!(hex_from_rgb(1.0, 0.0, 0.0), "#FF0000");
        assert_eq!(hex_from_rgb(0.0, 1.0, 0.0), "#00FF00");
        let (r, g, b) = parse_hex("#3B82F6").unwrap();
        assert_eq!(hex_from_rgb(r, g, b), "#3B82F6");
    }

    /// **A sangria das camadas não é enfeite.** Zero aqui devolve o fio de matiz nas quinas do
    /// quadrado SV (ver [`LAYER_BLEED`]), que foi um defeito relatado; e ela tem que ser MENOR que um
    /// pixel lógico, senão a sobra passaria a aparecer nos lados retos, onde a máscara do
    /// `overflow_hidden` recorta no limite exato.
    #[test]
    fn a_sangria_das_camadas_e_meio_pixel() {
        assert!(LAYER_BLEED > 0.0, "sem sangria, o matiz vaza no antialiasing da quina");
        assert!(LAYER_BLEED < 1.0, "mais que um pixel e a sobra vira quina quadrada");
        assert_eq!(LAYER_BLEED, 0.5);
    }

    /// **HSV ↔ HSL fecha exatamente**, porque são identidades algébricas e não uma volta por RGB. Um
    /// erro aqui apareceria como valor escorregando a cada tecla digitada no modo HSL.
    #[test]
    fn hsv_hsl_roundtrip() {
        let mut seed = 0x51ed_2701u32;
        let mut rnd = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / ((1u32 << 24) as f32)
        };
        for _ in 0..500 {
            let (s, v) = (rnd(), rnd());
            let (s_hsl, l) = hsv_to_hsl(s, v);
            let (s2, v2) = hsl_to_hsv(s_hsl, l);
            assert!(approx(v, v2), "v {v} → {v2}");
            // Com v=0 (preto) a saturação de HSV é indefinida: qualquer `s` dá o mesmo preto, então a
            // volta não tem como recuperá-la. Fora daí a ida e volta é exata.
            if v > 1e-4 {
                assert!(approx(s, s2), "s {s} → {s2} (v={v})");
            }
        }
    }

    /// Os casos de borda do HSL: preto, branco e cinza não têm saturação definida, e é onde a fórmula
    /// dividiria por zero.
    #[test]
    fn hsl_nos_extremos() {
        assert_eq!(hsv_to_hsl(1.0, 0.0), (0.0, 0.0), "preto: l=0, sem saturação");
        assert_eq!(hsv_to_hsl(0.0, 1.0), (0.0, 1.0), "branco: l=1, sem saturação");
        let (s, l) = hsv_to_hsl(0.0, 0.5);
        assert!(approx(s, 0.0) && approx(l, 0.5), "cinza médio: l=0,5 sem saturação");
        // Cor cheia (s=1, v=1) é l=0,5 com saturação máxima — a identidade mais conhecida do HSL.
        let (s, l) = hsv_to_hsl(1.0, 1.0);
        assert!(approx(s, 1.0) && approx(l, 0.5));
    }

    /// **sRGB ↔ OKLCH fecha** dentro do gamute. Fora dele a volta é aparada de propósito, então o teste
    /// só afirma o que é afirmável: as cores que o sRGB alcança voltam iguais.
    #[test]
    fn rgb_oklch_roundtrip() {
        let mut seed = 0x2b3c_4d5eu32;
        let mut rnd = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / ((1u32 << 24) as f32)
        };
        for _ in 0..500 {
            let (r, g, b) = (rnd(), rnd(), rnd());
            let (l, c, h) = rgb_to_oklch(r, g, b);
            let (r2, g2, b2) = oklch_to_rgb(l, c, h);
            assert!(
                approx(r, r2) && approx(g, g2) && approx(b, b2),
                "({r:.4},{g:.4},{b:.4}) → oklch({l:.4},{c:.4},{h:.2}) → ({r2:.4},{g2:.4},{b2:.4})"
            );
        }
    }

    /// **Valores conhecidos do OKLCH**, conferidos contra o que o CSS/oklch.com dão para as mesmas
    /// cores. É o que pega uma matriz digitada errado — um round-trip passa até com matriz trocada,
    /// desde que a inversa combine com ela.
    #[test]
    fn oklch_valores_conhecidos() {
        // Branco: luminosidade cheia, sem croma.
        let (l, c, _) = rgb_to_oklch(1.0, 1.0, 1.0);
        assert!(approx(l, 1.0), "branco l={l}");
        assert!(c < 1e-4, "branco não tem croma: {c}");

        // Preto.
        let (l, c, _) = rgb_to_oklch(0.0, 0.0, 0.0);
        assert!(l < 1e-4 && c < 1e-4, "preto: l={l} c={c}");

        // Vermelho puro #FF0000 = oklch(62.8% 0.2577 29.23).
        let (l, c, h) = rgb_to_oklch(1.0, 0.0, 0.0);
        assert!((l - 0.628).abs() < 0.002, "vermelho l={l}");
        assert!((c - 0.2577).abs() < 0.002, "vermelho c={c}");
        assert!((h - 29.23).abs() < 0.2, "vermelho h={h}");

        // Verde puro #00FF00 = oklch(86.64% 0.2948 142.5) — é o de maior croma no sRGB.
        let (l, c, h) = rgb_to_oklch(0.0, 1.0, 0.0);
        assert!((l - 0.8664).abs() < 0.002, "verde l={l}");
        assert!((c - 0.2948).abs() < 0.002, "verde c={c}");
        assert!((h - 142.5).abs() < 0.3, "verde h={h}");

        // Azul puro #0000FF = oklch(45.2% 0.3132 264.05).
        let (l, c, h) = rgb_to_oklch(0.0, 0.0, 1.0);
        assert!((l - 0.452).abs() < 0.002, "azul l={l}");
        assert!((c - 0.3132).abs() < 0.002, "azul c={c}");
        assert!((h - 264.05).abs() < 0.3, "azul h={h}");

        // Um cinza: croma zero, e o gama do sRGB faz 50% de canal NÃO dar 50% de luminosidade.
        let (l, c, _) = rgb_to_oklch(0.5, 0.5, 0.5);
        assert!(c < 1e-4, "cinza não tem croma: {c}");
        assert!((l - 0.5981).abs() < 0.002, "cinza 50% → l={l}");
    }

    /// **A lista de modos é a única fonte**: os rótulos do Select saem de `ALL`, e `index`/`from_index`
    /// têm que ser inversas em cima dela. Se alguém acrescentar um modo no enum e esquecer o `ALL`, ou
    /// trocar a ordem de um só lado, o 3º item do menu passa a significar o 2º espaço — em silêncio.
    #[test]
    fn os_modos_e_os_indices_do_select_batem() {
        assert_eq!(ColorMode::ALL.len(), 3);
        for (i, modo) in ColorMode::ALL.iter().enumerate() {
            assert_eq!(modo.index(), i, "{} está fora de ordem", modo.label());
            assert_eq!(ColorMode::from_index(i), *modo);
        }
        // Índice fora da lista cai no default em vez de entrar em pânico.
        assert_eq!(ColorMode::from_index(99), ColorMode::default());
        // Rótulos distintos: dois iguais deixariam o menu ambíguo.
        let mut labels: Vec<&str> = ColorMode::ALL.iter().map(|m| m.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 3);
    }

    /// As casas decimais de cada campo. São o passo mais fino que dá pra digitar, então travar isto é
    /// travar a usabilidade do modo: croma com 2 casas deixaria o campo com ~40 valores úteis.
    #[test]
    fn a_formatacao_de_cada_campo() {
        assert_eq!(fmt_deg(29.23), "29");
        assert_eq!(fmt_deg(-1.0), "359", "grau negativo dá a volta");
        assert_eq!(fmt_pct(0.5), "50");
        assert_eq!(fmt_pct_1(0.628), "62.8");
        assert_eq!(fmt_chroma(0.2577), "0.258");
        assert_eq!(fmt_chroma(9.0), "0.500", "croma acima do teto é aparado");
    }

    /// **O contorno das alças inverte com o tema.** Elas ficam sobre conteúdo de cor arbitrária, então
    /// o que garante contraste é ser o oposto do fundo do painel — não um token de superfície.
    #[test]
    fn o_contorno_das_alcas_inverte_com_o_tema() {
        assert_eq!(PICKER_DARK.handle_ring.0, 0xffffffff, "escuro: contorno branco");
        assert_eq!(
            PICKER_LIGHT.handle_ring.0, 0x656565ff,
            "claro: cinza, não preto — preto puro grita mais que a cor que o contorno delimita"
        );
        // Opacos os dois: um contorno translúcido deixaria a cor de baixo atravessar e some justamente
        // sobre as cores que ele precisa recortar.
        assert_eq!(PICKER_DARK.handle_ring.0 & 0xff, 0xff);
        assert_eq!(PICKER_LIGHT.handle_ring.0 & 0xff, 0xff);
    }

    #[test]
    fn swatch_list_no_recents_is_palette() {
        // Sem recentes → exatamente a paleta do design (27 cores, em ordem).
        let list = build_swatch_list(&[]);
        assert_eq!(list.len(), 27);
        assert_eq!(list, PALETTE.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    }

    #[test]
    fn swatch_list_recents_first_dedup_capped() {
        // Uma recente NOVA (não na paleta) vai pra frente; total continua 27.
        let recents = vec!["#123456".to_string()];
        let list = build_swatch_list(&recents);
        assert_eq!(list.len(), 27);
        assert_eq!(list[0], "#123456");
        // A recente NÃO duplica a paleta; a última cor da paleta (#DDDDDD) cai fora (cap 27).
        assert_eq!(list.iter().filter(|c| c.eq_ignore_ascii_case("#DDDDDD")).count(), 0);

        // Uma recente que JÁ está na paleta não duplica: aparece só na frente, não 2x.
        let recents2 = vec!["#ee417b".to_string()]; // = PALETTE[0], case diferente
        let list2 = build_swatch_list(&recents2);
        assert_eq!(list2.len(), 27);
        assert_eq!(list2[0], "#ee417b");
        assert_eq!(
            list2.iter().filter(|c| c.eq_ignore_ascii_case("#EE417B")).count(),
            1
        );
    }
}

/// Testes do **botão de copiar** — a única parte do picker cujo comportamento é temporal.
///
/// Precisam de janela porque `copy_hex` mexe na área de transferência do `App` e notifica a view.
///
/// O que **não** está aqui de propósito: se o clique chega no botão (em vez de cair no campo de texto
/// atrás dele). Isso é do [`crate::Input::icon_button`], e `tests/password_toggle.rs` já testa esse
/// roteamento na mesma peça — inclusive o caso de clicar no texto e o botão NÃO reagir.
///
/// O que eles NÃO cobrem: o `request_animation_frame` do fim do `render`. Sem ele o check acende e
/// **fica** aceso até algum outro motivo redesenhar a view, e não achei como observar isso do
/// `TestAppContext` (o executor de teste não roda o callback de frame da plataforma — foi o mesmo
/// muro que apareceu no desfoque adiado, ver `crate::input::blur_on_outside_click`). Fica declarado
/// em vez de testado.
#[cfg(test)]
mod tests_de_janela {
    #![allow(clippy::assertions_on_constants)]
    use super::tests::approx;
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    /// Abre uma janela com o picker dentro de um [`gpui_component::Root`].
    ///
    /// O `Root` não é enfeite: o campo de texto do núcleo chama `Root::read` no caminho de teclado, e
    /// sem ele um `simulate_input` derruba o teste com "the window root view should be of type
    /// `ui::Root`". Como o `Root` passa a ser a raiz, o picker é guardado à parte.
    /// Onde a coluna direita começa, em coordenadas da janela. **Derivado**, não literal: ele muda com
    /// o lado do quadrado e com o vão das colunas, e um número cravado aqui deixaria os cliques dos
    /// testes caindo no vizinho depois de qualquer ajuste de geometria.
    fn x_da_coluna_direita() -> f32 {
        CARD_BORDER + CARD_PAD + SV_SIDE + 3.0 * COL_GAP + 2.0 * BAR_W
    }

    /// O topo do conteúdo do painel.
    fn y_do_conteudo() -> f32 {
        CARD_BORDER + CARD_PAD
    }

    fn picker(cx: &mut TestAppContext) -> (Entity<ColorPicker>, VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let mut saida = None;
        let window = cx.add_window(|window, cx| {
            let picker = cx.new(|cx| ColorPicker::new(window, cx));
            saida = Some(picker.clone());
            gpui_component::Root::new(picker, window, cx)
        });
        let picker = saida.expect("picker montado");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        (picker, vcx)
    }

    /// Clicar em copiar **põe o hex na área de transferência** (era o pedido: "deve de fato copiar")
    /// e acende o check.
    #[gpui::test]
    fn copiar_poe_o_hex_na_area_de_transferencia_e_acende_o_check(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);

        // O picker nasce branco; escolho uma cor com os três canais diferentes, pra um hex com os
        // canais na ordem errada não passar batido.
        vcx.update(|window, cx| {
            picker.update(cx, |p, cx| {
                p.set_rgba([1.0, 0.5, 0.0, 1.0], window, cx);
                assert!(!p.copiado_agora(), "em repouso o ícone é a prancheta");
                p.copy_hex(cx);
                assert!(p.copiado_agora(), "copiar acende o check");
            });
        });

        let na_area = vcx.update(|_w, cx| cx.read_from_clipboard().and_then(|c| c.text()));
        assert_eq!(na_area.as_deref(), Some("#FF8000"));
    }

    /// **Focar um campo seleciona tudo.**
    ///
    /// Não dá pra ler a seleção de fora (o `selected_range` do núcleo é `pub(super)`), então o teste
    /// afirma o COMPORTAMENTO, que é o que interessa: com tudo selecionado, digitar um caractere
    /// substitui o valor inteiro. Se a seleção não acontecesse, o dígito seria inserido no cursor e o
    /// campo diria `1005%` ou `5100%` em vez de `5`.
    #[gpui::test]
    fn focar_um_campo_seleciona_tudo(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);
        vcx.run_until_parked();

        // A opacidade abre em `100%` — é o caso com máscara, o mais exigente dos cinco.
        let estado = vcx.update(|_w, cx| picker.read(cx).opacity_input.clone());
        assert_eq!(vcx.update(|_w, cx| estado.read(cx).value().to_string()), "100%");

        // **Clica no campo**, em vez de chamar `focus()`. Não é preciosismo: o `on_focus` do núcleo (que
        // emite o `InputEvent::Focus`) só dispara quando o CAMINHO de foco é recalculado num desenho,
        // e chamar `focus()` deixa `is_focused` verdadeiro sem produzir esse caminho — medido, o evento
        // não chegava. O clique é o caminho de verdade, e de quebra prova a ORDEM: o próprio campo
        // posiciona o cursor no mouse-down, e a seleção tem que sobreviver a isso.
        //
        // A posição sai das constantes de geometria (ver `PANEL_WIDTH`): respiro 17, quadrado 182, dois
        // vãos e as duas barras até a coluna direita em 251, onde a fileira do hex é a terceira.
        let x = x_da_coluna_direita() + 98.0 + 1.0 + OPACITY_WIDTH / 2.0;
        let y = y_do_conteudo()
            + PREVIEW_H
            + PREVIEW_GAP
            + PREVIEW_LABEL_LH
            + 3.0 * ROW_GAP
            + 2.0 * ROW_H
            + ROW_H / 2.0;
        vcx.simulate_click(gpui::point(px(x), px(y)), gpui::Modifiers::default());
        vcx.run_until_parked();

        vcx.simulate_input("5");
        vcx.run_until_parked();

        assert_eq!(
            vcx.update(|_w, cx| estado.read(cx).value().to_string()),
            "5",
            "digitar tinha que substituir o valor inteiro, não inserir no cursor"
        );
    }

    /// **O `sync_inputs` não reescreve o campo focado** — e é isto que conserta o caso relatado.
    ///
    /// O `set_value` do núcleo sempre colapsa a seleção no fim do texto, sem atalho pra texto igual.
    /// Então bastava um `sync` depois do render pra desfazer a seleção — e é o que acontecia ao clicar
    /// de um campo direto no outro, porque o `Blur` do antigo (que dispara commit + sync) é entregue
    /// pelas escutas de foco DEPOIS do render. Ver [`escreve`].
    ///
    /// Este teste roda o `sync` à mão justamente pra não depender dessa ordem: sem a exceção do foco,
    /// ele falha; com ela, passa.
    #[gpui::test]
    fn o_sync_nao_reescreve_o_campo_focado(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);
        vcx.run_until_parked();

        let x = x_da_coluna_direita() + 98.0 + 1.0 + OPACITY_WIDTH / 2.0;
        let y = y_do_conteudo()
            + PREVIEW_H
            + PREVIEW_GAP
            + PREVIEW_LABEL_LH
            + 3.0 * ROW_GAP
            + 2.0 * ROW_H
            + ROW_H / 2.0;
        vcx.simulate_click(gpui::point(px(x), px(y)), gpui::Modifiers::default());
        vcx.run_until_parked();

        // O que o `Blur` de um vizinho faria, na ordem em que ele faz: depois do render.
        vcx.update(|window, cx| {
            picker.update(cx, |p, cx| p.sync_inputs(window, cx));
        });
        vcx.run_until_parked();

        let estado = vcx.update(|_w, cx| picker.read(cx).opacity_input.clone());
        vcx.simulate_input("7");
        vcx.run_until_parked();

        assert_eq!(
            vcx.update(|_w, cx| estado.read(cx).value().to_string()),
            "7",
            "um sync depois do foco não pode desfazer a seleção do campo focado"
        );
    }

    /// **O caminho do usuário: clicar de um campo direto no outro.**
    ///
    /// ⚠️ Este teste passa COM e SEM o conserto — a ordem em que o `TestAppContext` entrega o `Blur`
    /// é a favorável, e por isso ele não pegou o defeito relatado (quem pega é o de cima). Fica porque
    /// cobre o fluxo de dois cliques em sequência, que nenhum outro cobre: se um dia a seleção parar de
    /// valer no segundo clique, é aqui que aparece.
    #[gpui::test]
    fn clicar_de_um_campo_direto_no_outro_seleciona(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);
        vcx.run_until_parked();

        // Primeiro campo: o R (o primeiro dos três canais).
        let x_r = x_da_coluna_direita() + 63.0 / 2.0;
        let y_canais = y_do_conteudo()
            + PREVIEW_H
            + PREVIEW_GAP
            + PREVIEW_LABEL_LH
            + 2.0 * ROW_GAP
            + ROW_H
            + ROW_H / 2.0;
        vcx.simulate_click(gpui::point(px(x_r), px(y_canais)), gpui::Modifiers::default());
        vcx.run_until_parked();

        // Sem passar pelo vazio: direto na opacidade.
        let x_op = x_da_coluna_direita() + 98.0 + 1.0 + OPACITY_WIDTH / 2.0;
        let y_hex = y_do_conteudo()
            + PREVIEW_H
            + PREVIEW_GAP
            + PREVIEW_LABEL_LH
            + 3.0 * ROW_GAP
            + 2.0 * ROW_H
            + ROW_H / 2.0;
        vcx.simulate_click(gpui::point(px(x_op), px(y_hex)), gpui::Modifiers::default());
        vcx.run_until_parked();

        let estado = vcx.update(|_w, cx| picker.read(cx).opacity_input.clone());
        vcx.simulate_input("7");
        vcx.run_until_parked();

        assert_eq!(
            vcx.update(|_w, cx| estado.read(cx).value().to_string()),
            "7",
            "vindo de outro campo focado, a seleção também tem que valer"
        );
    }

    /// **O `%` viaja com o valor, e o commit o remove.** É o par que faz a máscara funcionar: sem o
    /// primeiro não há máscara; sem o segundo, confirmar o campo não parsearia mais nada e a opacidade
    /// travaria no último valor válido.
    #[gpui::test]
    fn a_opacidade_mostra_o_pct_e_ainda_parseia(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);
        vcx.run_until_parked();

        let estado = vcx.update(|_w, cx| picker.read(cx).opacity_input.clone());
        assert_eq!(
            vcx.update(|_w, cx| estado.read(cx).value().to_string()),
            "100%",
            "o campo mostra o símbolo"
        );

        // Digita COM o símbolo (é o que sobra se o usuário editar em vez de substituir) e confirma.
        vcx.update(|window, cx| {
            estado.update(cx, |st, cx| st.set_value("40%", window, cx));
            picker.update(cx, |p, cx| {
                p.commit_opacity_input(cx);
                p.sync_inputs(window, cx);
            });
        });
        vcx.run_until_parked();

        vcx.update(|_w, cx| {
            assert!(
                approx(picker.read(cx).rgba()[3], 0.4),
                "o `%` não pode impedir o parse: alpha ficou {}",
                picker.read(cx).rgba()[3]
            );
        });
        assert_eq!(
            vcx.update(|_w, cx| estado.read(cx).value().to_string()),
            "40%",
            "e volta formatado com o símbolo"
        );
    }

    /// **A cor amostrada da tela entra pelo mesmo caminho do hex e do swatch** — e sem tocar no que
    /// não é dela.
    ///
    /// O que este teste guarda são as duas omissões deliberadas do [`ColorPicker::adopt_rgb`], que é o
    /// caminho que o conta-gotas usa: o `previous` (a cor de quando o painel abriu, que dá sentido à
    /// comparação Previous/New) e o alpha (decisão de outro controle). Um conta-gotas que reescrevesse
    /// qualquer um dos dois apagaria informação que o usuário não pediu pra perder.
    ///
    /// A lupa em si não é testável daqui: ela é UI do sistema e exige um clique humano na tela.
    #[gpui::test]
    fn a_cor_amostrada_nao_mexe_no_previous_nem_no_alpha(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);
        // Abre num azul meio transparente: é o estado que o conta-gotas NÃO pode estragar.
        vcx.update(|window, cx| {
            picker.update(cx, |p, cx| p.set_rgba([0.0, 0.0, 1.0, 0.4], window, cx));
        });
        vcx.run_until_parked();

        vcx.update(|window, cx| {
            picker.update(cx, |p, cx| p.adopt_rgb(1.0, 0.0, 0.0, window, cx));
        });
        vcx.run_until_parked();

        vcx.update(|_w, cx| {
            let p = picker.read(cx);
            let [r, g, b, a] = p.rgba();
            assert!(
                approx(r, 1.0) && approx(g, 0.0) && approx(b, 0.0),
                "a cor nova é a amostrada: ({r},{g},{b})"
            );
            assert!(approx(a, 0.4), "o alpha é de outro controle: {a}");
            assert_eq!(
                p.previous,
                [0.0, 0.0, 1.0, 0.4],
                "o `previous` é a cor de quando o painel abriu — o conta-gotas não mexe nele"
            );
        });
    }

    /// **Amostrar um branco não joga a barra de matiz pro vermelho.** Acromático não tem matiz, e o
    /// `rgb_to_hsv` devolve 0 pra ele — adotar esse 0 faria o próximo arraste no quadrado sair de uma
    /// cor que o usuário nunca escolheu.
    #[gpui::test]
    fn amostrar_um_acromatico_preserva_o_matiz(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);
        // Verde: matiz 120.
        vcx.update(|window, cx| {
            picker.update(cx, |p, cx| p.set_rgba([0.0, 1.0, 0.0, 1.0], window, cx));
        });
        vcx.run_until_parked();

        vcx.update(|window, cx| {
            picker.update(cx, |p, cx| p.adopt_rgb(1.0, 1.0, 1.0, window, cx));
        });
        vcx.run_until_parked();

        vcx.update(|_w, cx| {
            let p = picker.read(cx);
            assert!(approx(p.s, 0.0) && approx(p.v, 1.0), "a cor é branca");
            assert!(approx(p.h, 120.0), "mas o matiz continua o verde: {}", p.h);
        });
    }

    /// **Trocar o espaço no Select troca os três campos.** É a funcionalidade pedida.
    ///
    /// O teste dirige o Select pelo EVENTO que um clique numa opção emitiria (`set_selected` não
    /// emite, de propósito, pra não fazer laço com quem assina). Assim ele cobre a corrente inteira:
    /// evento → `this.mode` → `sync_inputs` → o texto dos campos.
    ///
    /// Os valores esperados são os do vermelho puro em cada espaço, conferidos contra o CSS:
    /// `rgb(255 0 0)` = `hsl(0 100% 50%)` = `oklch(62.8% 0.258 29)`.
    #[gpui::test]
    fn trocar_o_espaco_troca_os_tres_campos(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);
        vcx.update(|window, cx| {
            picker.update(cx, |p, cx| p.set_rgba([1.0, 0.0, 0.0, 1.0], window, cx));
        });
        vcx.run_until_parked();

        let campos = |vcx: &mut VisualTestContext| -> [String; 3] {
            vcx.update(|_w, cx| {
                let p = picker.read(cx);
                [
                    p.r_input.read(cx).value().to_string(),
                    p.g_input.read(cx).value().to_string(),
                    p.b_input.read(cx).value().to_string(),
                ]
            })
        };
        let escolher = |modo: ColorMode, vcx: &mut VisualTestContext| {
            let idx = modo.index();
            vcx.update(|_w, cx| {
                let sel = picker.read(cx).mode_select.clone();
                sel.update(cx, |_sel, cx| {
                    cx.emit(crate::SelectEvent::Change(idx));
                });
            });
            vcx.run_until_parked();
        };

        escolher(ColorMode::Rgb, &mut vcx);
        assert_eq!(campos(&mut vcx), ["255", "0", "0"], "RGB");

        escolher(ColorMode::Hsl, &mut vcx);
        assert_eq!(campos(&mut vcx), ["0", "100", "50"], "HSL");

        escolher(ColorMode::Oklch, &mut vcx);
        let oklch = campos(&mut vcx);
        assert_eq!(oklch, ["62.8", "0.258", "29"], "OKLCH");

        // E voltar pro RGB devolve os 255/0/0: a troca de espaço não mexe na COR, só na leitura dela.
        escolher(ColorMode::Rgb, &mut vcx);
        assert_eq!(campos(&mut vcx), ["255", "0", "0"], "de volta ao RGB");
    }

    /// **Digitar num campo atualiza os outros.** Sem isso os espaços novos seriam inúteis: o valor
    /// digitado em OKLCH não teria como aparecer convertido no hex, e o painel mostraria dois estados
    /// diferentes ao mesmo tempo.
    #[gpui::test]
    fn digitar_num_campo_reescreve_os_outros(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);
        // Começa no branco; escreve o vermelho puro nos campos RGB e confirma com Enter.
        vcx.update(|window, cx| {
            picker.update(cx, |p, cx| {
                p.r_input
                    .update(cx, |st, cx| st.set_value("255", window, cx));
                p.g_input.update(cx, |st, cx| st.set_value("0", window, cx));
                p.b_input.update(cx, |st, cx| st.set_value("0", window, cx));
                p.commit_rgb_inputs(0, cx);
                p.sync_inputs(window, cx);
            });
        });
        vcx.run_until_parked();

        let hex = vcx.update(|_w, cx| picker.read(cx).hex_input.read(cx).value().to_string());
        assert_eq!(hex, "#FF0000", "o hex tem que acompanhar o que foi digitado em RGB");
    }

    /// **O quadrado SV é quadrado E do tamanho da linha.**
    ///
    /// O 1:1 em si é barato: o lado sai de um `.size()`, então ele é quadrado por construção. O que
    /// este teste realmente guarda é a segunda asserção — que [`SV_SIDE`] continua sendo a altura de
    /// FATO da coluna direita.
    ///
    /// Como se mede isso sem sondar a coluna: as duas barras não declaram altura, elas esticam pela
    /// altura da LINHA, que é o maior entre o quadrado e a coluna. Se a soma de [`RIGHT_COL_HEIGHT`]
    /// deixar de bater — alguém acrescenta uma fileira e não mexe nas constantes —, a linha passa a
    /// ser mais alta que o quadrado, as barras crescem e o quadrado fica curto. É exatamente o que
    /// se vê na tela, e é o que a comparação abaixo pega.
    #[gpui::test]
    fn o_quadrado_sv_e_quadrado_e_do_tamanho_da_linha(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);
        vcx.run_until_parked();

        let (sv, barra) = vcx.update(|_w, cx| {
            let p = picker.read(cx);
            (
                p.sv_bounds.expect("o `canvas` do SV mediu"),
                p.alpha_bounds.expect("o `canvas` da barra de alpha mediu"),
            )
        });
        let (largura, altura) = (f32::from(sv.size.width), f32::from(sv.size.height));
        assert_eq!(
            largura, altura,
            "o quadrado SV tem que ser 1:1 — saiu {largura}×{altura}"
        );
        assert_eq!(
            altura,
            f32::from(barra.size.height),
            "o quadrado tem que ter a altura da LINHA (a que as barras esticam): se divergir, \
             `RIGHT_COL_HEIGHT` não é mais a altura da coluna direita"
        );
    }

    /// **O check apaga sozinho.** O estado é um instante, não um sinalizador: passado
    /// [`COPIED_FEEDBACK`], `copiado_agora` já responde `false` sem ninguém ter que desligá-lo.
    ///
    /// O relógio aqui é o de parede (`Instant`), que o `advance_clock` do executor de teste não move
    /// — então o teste envelhece o instante em vez de esperar 1 segundo.
    #[gpui::test]
    fn o_check_apaga_sozinho_passado_o_tempo(cx: &mut TestAppContext) {
        let (picker, mut vcx) = picker(cx);

        vcx.update(|_w, cx| picker.update(cx, |p, cx| {
            p.copy_hex(cx);
            assert!(p.copiado_agora());

            // Um piscar de olhos antes do fim: ainda aceso.
            let piscar = std::time::Duration::from_millis(50);
            p.copied_at = std::time::Instant::now().checked_sub(COPIED_FEEDBACK - piscar);
            assert!(p.copiado_agora(), "antes de 1s continua check");

            // Um piscar depois: apagado.
            p.copied_at = std::time::Instant::now().checked_sub(COPIED_FEEDBACK + piscar);
            assert!(!p.copiado_agora(), "passado 1s volta pra prancheta");
        }));
    }
}

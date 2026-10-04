//! `meter` — o medidor do [coss][1], sobre o primitivo [Meter do Base UI][2].
//!
//! [1]: https://github.com/cosscom/coss/blob/main/apps/ui/registry/default/ui/meter.tsx
//! [2]: https://github.com/mui/base-ui/tree/master/packages/react/src/meter
//!
//! # O que ele é (e o que NÃO é)
//!
//! Uma **leitura escalar limitada**: quanto de um todo conhecido está ocupado — disco, banda, nota.
//! Não é barra de progresso de tarefa e não é controle: nada aqui responde a mouse ou teclado. É a
//! distinção que a própria referência faz ("if displaying task completion or async progress -> use
//! Progress instead").
//!
//! # A anatomia
//!
//! Uma coluna de largura cheia com 8px de respiro entre as fatias. Em cima, opcional, a linha de
//! cabeçalho: o rótulo à esquerda e a leitura à direita, os dois em `text-sm`/`--foreground`. Embaixo,
//! um trilho de 8px em `--input` com o indicador em `--primary` preenchendo da esquerda até a
//! porcentagem do valor.
//!
//! **Não há raio em peça nenhuma.** O `.tsx` não traz `rounded-*` nem no trilho nem no indicador — o
//! medidor é retangular de propósito, e isso tem consequência direta no recorte (ver as diferenças).
//!
//! # A conta de valor → largura
//!
//! É o coração do componente, e ela **não** é a do [`crate::slider`]. Vem do `MeterRoot` do Base UI,
//! nesta ordem exata:
//!
//! ```text
//! bruto   = (valor − min) × 100 / (max − min)
//! percent = clamp_js(se bruto é NaN { 0 } senão { bruto }, 0, 100)
//! aparado = clamp_js(se valor é NaN { min } senão { valor }, min, max)
//! ```
//!
//! `clamp_js` é `Math.max(min, Math.min(v, max))` — **não** o [`f32::clamp`], que entra em pânico com
//! `min > max` e propaga `NaN`. A diferença aparece justo nos limites que este componente precisa
//! acertar. Ver [`clamp_js`], [`percent_of`] e [`clamp_value`].
//!
//! O que cada limite produz, e por quê:
//!
//! | caso | `percent` | `aparado` | de onde vem |
//! |---|---|---|---|
//! | valor no mínimo | `0` | `min` | a conta é linear |
//! | valor no máximo | `100` | `max` | idem |
//! | valor abaixo do mínimo | `0` | `min` | o `clamp` externo |
//! | valor acima do máximo | `100` | `max` | idem |
//! | `min == max`, valor **acima** | `100` | `min` | divisão por zero dá `+∞`, aparado em 100 |
//! | `min == max`, valor **igual** | `0` | `min` | `0/0` dá `NaN`, e o primitivo troca `NaN` por 0 |
//! | `min == max`, valor **abaixo** | `0` | `min` | `−∞`, aparado em 0 |
//! | `min > max` (invertida) | espelhado | `min` | a divisão troca de sinal duas vezes |
//! | valor `NaN` | `0` | `min` | os dois fallbacks explícitos do primitivo |
//!
//! A linha de `min == max` é o motivo de eu **não** ter reusado o [`crate::slider`]: a `fraction` dele
//! devolve `0` pra faixa degenerada, sempre. Aqui a referência devolve `100` quando o valor está acima
//! — o medidor lê "cheio", que é a leitura sensata pra "o teto é o piso e você passou dele". São duas
//! contas diferentes com a mesma cara, e trocar uma pela outra não quebraria nada que compile.
//!
//! # O que NÃO é compartilhado com o `slider` — decidido, não implícito
//!
//! O slider é o parente mais próximo (trilho + preenchimento + conversão valor→largura), e **nada de
//! geometria é compartilhado**. Não é descuido:
//!
//! - **Altura do trilho**: 8px aqui (`h-2`), 4px no slider (`h-1`). Números diferentes, e diferentes
//!   *por design* — o slider tem um polegar de 16px em cima do trilho, então o trilho pode ser fino; o
//!   medidor é só o trilho, e um de 4px não se lê.
//! - **Raio**: `rounded_full` no slider, **nenhum** aqui. Ver acima.
//! - **A conta**: a [`percent_of`] daqui e a `slider::fraction` divergem na faixa degenerada e na
//!   invertida. Ver a seção anterior.
//! - **Recuo nas pontas**: o slider recua 2px de cada lado (`inset-x-0.5`) pra acomodar o polegar
//!   alinhado por borda. Aqui não há polegar e não há recuo: o indicador começa em 0.
//!
//! O que **coincide** são os valores dos tokens `--input`, `--primary` e `--foreground`, porque são os
//! mesmos tokens. Eles continuam declarados em [`METER_LIGHT`]/[`METER_DARK`], que é a convenção
//! unânime da casa (17 componentes, cada um com a sua paleta): a paleta de componente é o registro do
//! que a referência pediu, não um cache do tema.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`transition-all duration-500`** no indicador. O GPUI não tem transição de estilo. Reproduzir
//!   exigiria o medidor guardar o valor ANTERIOR entre frames, o que o tira do [`RenderOnce`] e o
//!   transforma numa `Entity` com estado — mudança de forma de API pra uma suavização de 500ms. A
//!   largura muda de um frame pro outro.
//! - **A semântica de acessibilidade inteira**: `role="meter"`, `aria-valuenow`, `aria-valuemin`,
//!   `aria-valuemax`, `aria-valuetext`, `aria-labelledby`, e o `<span>` invisível que o Base UI injeta
//!   pra forçar o NVDA a ler o rótulo. **O gpui 0.2.2 não tem árvore de acessibilidade**: não há
//!   `accesskit` (nem qualquer dependência de a11y) no `Cargo.toml` dele, e não há `role`/`aria` em
//!   `styled.rs` nem em `window.rs` — o que existe é [`gpui::FocusHandle`] e `tab_stop`, que são
//!   navegação por foco, não semântica de papel. Verificado por grep na fonte da 0.2.2, não presumido.
//!   O único resíduo aproveitável: [`Meter::clamped_value`] devolve exatamente o número que iria em
//!   `aria-valuenow`, e [`Meter::formatted_value`] o que iria em `aria-valuetext` — quem um dia tiver
//!   onde pendurar isso já tem os dois números prontos.
//! - **`data-slot`**: atributo de DOM, usado no original pra estilizar de fora. Sem DOM, sem atributo.
//! - **`block`** no trilho: o GPUI não tem fluxo de bloco, todo elemento é flex. Sem efeito observável
//!   — o trilho tem um filho só, e esse filho declara as duas dimensões.
//! - **`insetInlineStart: 0`** do indicador: é o `left` *lógico*, que num contexto RTL viraria
//!   `right`. O GPUI não tem direção de escrita lógica. O indicador é o primeiro filho de uma flex
//!   row, o que dá o mesmo pixel em LTR.
//!
//! **Resolvido em número**
//!
//! - **`h-2`** → 8px, **`gap-2`** → 8px (`--spacing` = 4px). Ver [`TRACK_HEIGHT`] e [`GAP`].
//! - **`text-sm`** → o PAR do Tailwind, `(14, 20)`. Fixar a entrelinha é obrigatório: o default do
//!   GPUI é `relative(1.618_034)`, que sobre 14px dá **22,7** — o cabeçalho ficaria 3px mais alto que
//!   a referência. Ver [`TEXT_SM`]/[`TEXT_SM_LINE`].
//! - **`font-medium`** → [`gpui::FontWeight::MEDIUM`] (500), só no rótulo. A leitura não é `medium`.
//! - **`bg-input`**, **`bg-primary`**, **`text-foreground`** → ver [`METER_LIGHT`]/[`METER_DARK`].
//! - **`tabular-nums`** → a feature OpenType `tnum`, aplicada por [`Styled::text_style`] e **não** por
//!   `.font(...)`: o `.font()` do GPUI escreve família E features de uma vez, o que trocaria a família
//!   herdada do shell por uma escrita à mão aqui dentro. Ver [`tabular_nums`].
//! - **`width: {percentageValue}%`** → [`gpui::relative`]`(percent / 100)`. Percentual de verdade,
//!   resolvido pelo taffy no layout — não pixel medido por `canvas` como no slider (lá a medida é
//!   necessária porque o ponteiro precisa virar valor; aqui não há ponteiro).
//! - **`height: inherit`** → `h_full()`. Dá os mesmos 8px do trilho.
//! - **`Intl.NumberFormat(style: "percent")`** → `format!("{}%", percent.round())`. O `style: percent`
//!   do `Intl` multiplica por 100 e arredonda a **0 casas**; o modo default dele é `halfExpand`, que é
//!   exatamente o [`f32::round`] do Rust (meio pra longe do zero). Ver [`format_percent`], e "Ausente"
//!   pro que ficou de fora.
//! - **Nenhum modificador `/N` neste componente**: os três tokens entram inteiros, sem
//!   `bg-primary/90` nem nada do gênero. Por isso o [`crate::color::Rgba8::scaled`] não aparece aqui —
//!   usá-lo seria inventar um alfa que a referência não pede.
//!
//! **Desvio consciente**
//!
//! - **A linha de cabeçalho é do componente, não de quem chama.** No React ela é um
//!   `<div className="flex items-center justify-between gap-2">` que cada particle escreve à mão — e os
//!   três que têm rótulo (`p-meter-1`, `p-meter-3`, `p-meter-4`) escrevem **as mesmas** classes. Aqui
//!   ela está embutida, com essas classes e nada mais. Quem quiser outro arranjo tem
//!   [`meter_label`]/[`meter_value`] soltos, que são as peças que a referência expõe.
//! - **Faixa invertida (`min > max`) NÃO é corrigida.** O [`crate::slider::Slider::bounds`] troca os
//!   dois; aqui vale o primitivo ao pé da letra, porque a referência **não** é degenerada nesse caso —
//!   ela produz um preenchimento espelhado coerente (`min = 100, max = 0, valor = 30` → 70% cheio) e um
//!   `aria-valuenow` colapsado em `min`. Trocar min/max mudaria o pixel na tela (daria 30%), e o
//!   pedido é ser fiel. Fica documentado e testado em vez de "consertado".
//! - **`overflow-hidden` é fiel aqui, sem asterisco.** O `overflow_hidden` do GPUI é ContentMask
//!   **retangular** e por isso não recorta pelo raio — o defeito que assombra `slider`/`table`. Neste
//!   componente ele não existe: a referência não tem raio nenhum, então retangular É o CSS. E como a
//!   [`percent_of`] apara em 100, o indicador nunca passa do trilho: o recorte é cinto de segurança do
//!   original, não geometria carregada.
//!
//! **Superset consciente**
//!
//! - **[`Meter::percent`], [`Meter::clamped_value`] e [`Meter::formatted_value`]** como leitura
//!   pública. No React esses três números vivem no `MeterRootContext`, fechado pra fora do primitivo.
//!   Aqui são públicos porque um consumidor que queira escrever a própria leitura ("42% de 1,2 TB")
//!   precisa do MESMO número que o indicador usa — recalculá-lo na mão é como as duas metades da UI
//!   passam a discordar.
//!
//! **Ausente**
//!
//! - **`format` (`Intl.NumberFormatOptions`) e `locale`.** A leitura automática é sempre a porcentagem
//!   `N%` no formato en-US. Quem quer casas decimais, unidade, separador de milhar ou o espaço
//!   inquebrável de `fr-FR` passa o texto pronto em [`Meter::value_text`]. É a mesma decisão, e a mesma
//!   razão, do [`crate::slider::slider_value`]: formatação é produto, e um componente de biblioteca que
//!   escolhe por você atrapalha mais do que ajuda.
//! - **`getAriaValueText`**: sem árvore de acessibilidade não há o que alimentar.
//! - **`children` arbitrários na raiz.** Aqui a composição é fixa: cabeçalho opcional + trilho.
//! - **`MeterTrack` / `MeterIndicator` como peças soltas** — não há caso de uso pra um trilho sem
//!   medidor, e expor os dois obrigaria a passar a porcentagem de fora, que é justo o que dá divergência.
//! - **O re-export do `MeterPrimitive`**: escape hatch de React.
//!
//! **Sem cobertura de teste — declarado**
//!
//! - **Cor, peso, tamanho e entrelinha do texto.** São propriedade de estilo/pintura: o GPUI não deixa
//!   um teste ler o `TextStyleRefinement` montado nem a cena pintada. O que guarda isso é o teste das
//!   constantes e o da paleta (contra literais, não contra as próprias constantes), mais a captura de
//!   janela na integração. Tirar o `.line_height(...)` do [`meter_value`] não quebra a suíte.
//! - **Se a fonte honra o `tnum`.** O teste garante que a feature PEDIDA é `("tnum", 1)` — o erro
//!   plausível, um nome de tag errado, falha em silêncio. Que a fonte carregada tenha a tabela é do
//!   sistema de texto, fora do alcance de um teste de unidade.
//! - **`overflow_hidden` no trilho.** Invisível a teste, e (por causa do aparo em 100) sem nada pra
//!   recortar. Removê-lo não muda pixel nenhum hoje; ele está lá porque está na referência.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, relative, App, Div, FontFeatures, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window,
};
use std::sync::Arc;

use crate::color::Rgba8;
use crate::theme;

// =================================================================================================
// Paleta
// =================================================================================================

/// Os três tokens que o medidor usa. Nenhum deles leva modificador de alfa na referência.
struct MeterPalette {
    /// `--input`: o fundo do trilho (`bg-input`).
    input: Rgba8,
    /// `--primary`: o indicador (`bg-primary`).
    primary: Rgba8,
    /// `--foreground`: o rótulo e a leitura.
    foreground: Rgba8,
}

/// Tema **claro**.
const METER_LIGHT: MeterPalette = MeterPalette {
    input: Rgba8(0x0000001a),      // preto 10%
    primary: Rgba8(0x262626ff),    // neutral-800
    foreground: Rgba8(0x262626ff), // neutral-800
};

/// Tema **escuro**.
const METER_DARK: MeterPalette = MeterPalette {
    input: Rgba8(0xffffff14),      // branco 8%
    primary: Rgba8(0xf5f5f5ff),    // neutral-100
    foreground: Rgba8(0xf5f5f5ff), // neutral-100
};

fn palette() -> &'static MeterPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &METER_DARK,
        theme::ThemeMode::Light => &METER_LIGHT,
    }
}

// =================================================================================================
// Medidas, todas da referência
// =================================================================================================

/// Espessura do trilho — `h-2`.
const TRACK_HEIGHT: f32 = 8.0;

/// Respiro entre as fatias da coluna, e entre rótulo e leitura — `gap-2`, com `--spacing` = 4px.
const GAP: f32 = 8.0;

/// Corpo do `text-sm`.
const TEXT_SM: f32 = 14.0;

/// Entrelinha do `text-sm` — o par do Tailwind. Sem isto o GPUI usa `relative(1.618_034)`.
const TEXT_SM_LINE: f32 = 20.0;

/// O `min = 0` default do `MeterRoot`.
const DEFAULT_MIN: f32 = 0.0;

/// O `max = 100` default do `MeterRoot`.
const DEFAULT_MAX: f32 = 100.0;

// =================================================================================================
// A aritmética — pura, separada do render, e transcrita do primitivo
// =================================================================================================

/// O `clamp` do Base UI: `Math.max(min, Math.min(v, max))`.
///
/// **Não** é o [`f32::clamp`]. As duas diferenças importam aqui: o `f32::clamp` entra em pânico se
/// `min > max` (e faixa invertida é um caso que este componente tem que atravessar sem cair), e
/// propaga `NaN` em vez de deixar o piso vencer. Com `min > max` esta versão devolve sempre `min`,
/// que é exatamente o que o JS faz.
///
/// `NaN` nunca chega aqui: os chamadores o trocam antes, como o próprio primitivo faz
/// ("`clamp` handles infinity, but NaN needs an explicit fallback"). Infinito, sim — e é aparado.
///
/// ⚠️ **Em Rust o `NaN` não propaga como em JS**: `f32::min` devolve *o outro operando* quando um é
/// `NaN`, então `clamp_js(NaN, 0, 100)` devolve **100** em vez de `NaN`. É por isso que o guarda dos
/// chamadores é obrigatório e não cosmético.
///
/// `pub(crate)` porque o [`crate::number_field`] precisa do MESMO clamp: lá o
/// `number-field/utils/validate.ts` importa o mesmo `internals/clamp.ts` que o `MeterRoot` importa,
/// então o compartilhamento aqui espelha o de lá. Uma segunda cópia divergiria justo nos limites que
/// os dois componentes existem pra acertar.
pub(crate) fn clamp_js(v: f32, min: f32, max: f32) -> f32 {
    v.min(max).max(min)
}

/// A porcentagem `[0,100]` que o indicador preenche.
///
/// Ver a tabela de limites no doc do módulo — em especial a faixa degenerada, onde isto **difere**
/// da `fraction` do [`crate::slider`].
fn percent_of(value: f32, min: f32, max: f32) -> f32 {
    let bruto = (value - min) * 100.0 / (max - min);
    clamp_js(if bruto.is_nan() { 0.0 } else { bruto }, 0.0, 100.0)
}

/// O valor aparado na faixa — o que iria em `aria-valuenow`.
///
/// Repare que ele **não** é `percent_of` de volta: numa faixa invertida os dois discordam de
/// propósito (70% de preenchimento com valor colapsado em `min`), porque é o que o primitivo produz.
fn clamp_value(value: f32, min: f32, max: f32) -> f32 {
    clamp_js(if value.is_nan() { min } else { value }, min, max)
}

/// A leitura default: a porcentagem, arredondada a inteiro, com `%` colado.
///
/// É o `Intl.NumberFormat(locale, { style: "percent" })` do primitivo reduzido ao en-US: o estilo
/// `percent` multiplica por 100 e arredonda a 0 casas, com modo `halfExpand` — o mesmo do
/// [`f32::round`]. O argumento já vem em `[0,100]`, então o `as i32` não trunca nada.
fn format_percent(percent: f32) -> SharedString {
    format!("{}%", percent.round() as i32).into()
}

/// A feature OpenType do `tabular-nums`: `tnum` ligada.
///
/// Existe como função (e não literal no lugar de uso) pra o nome da tag ser testável — uma tag
/// errada não é erro de compilação nem de render, ela só não faz nada.
///
/// `pub(crate)` porque o [`crate::number_field`] pede a mesma classe `tabular-nums` na referência —
/// e uma segunda lista de features seria uma segunda chance de escrever a tag errada.
pub(crate) fn tabular_nums() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".to_string(), 1)]))
}

// =================================================================================================
// A leitura à direita
// =================================================================================================

/// O que aparece na leitura do cabeçalho.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum ValueText {
    /// Sem leitura — o default, e o caso do `p-meter-2`.
    #[default]
    None,
    /// A porcentagem formatada, como o `<MeterValue />` sem `children` (`p-meter-1`).
    Auto,
    /// Texto de quem chama — o `children` de função do `MeterValue` (`p-meter-3`, `p-meter-4`).
    Custom(SharedString),
}

/// O texto que a leitura mostra, ou `None` quando não há leitura.
///
/// Está fora do `render` de propósito: dentro dele seria um `match` inalcançável por teste, e o
/// erro plausível — o [`ValueText::Auto`] formatar o VALOR em vez da porcentagem — não quebraria
/// nada que compile.
fn value_reading(value_text: &ValueText, percent: f32) -> Option<SharedString> {
    match value_text {
        ValueText::None => None,
        ValueText::Auto => Some(format_percent(percent)),
        ValueText::Custom(t) => Some(t.clone()),
    }
}

/// Se a linha de cabeçalho existe: só quando há rótulo OU leitura.
///
/// Também fora do `render`: uma linha vazia não se vê, mas o `gap-2` da coluna se vê — sobrariam
/// 8px acima do trilho em todo medidor sem rótulo (o `p-meter-2`, o caso mais comum).
fn header_needed(label: Option<&SharedString>, reading: Option<&SharedString>) -> bool {
    label.is_some() || reading.is_some()
}

// =================================================================================================
// O componente
// =================================================================================================

/// O medidor. Ver o doc do módulo.
///
/// É um **elemento de render** ([`RenderOnce`]): construa a cada frame.
///
/// ```ignore
/// // p-meter-1: rótulo + porcentagem automática
/// Meter::new(75.0).label("Storage usage").show_value()
///
/// // p-meter-3: faixa de 0 a 5, leitura escrita por quem chama
/// Meter::new(3.0).bounds(0.0, 5.0).label("Rating").value_text("3 / 5")
/// ```
#[derive(IntoElement)]
pub struct Meter {
    value: f32,
    min: f32,
    max: f32,
    label: Option<SharedString>,
    value_text: ValueText,
}

impl Meter {
    /// Um medidor na faixa default do primitivo, `0..=100`.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            min: DEFAULT_MIN,
            max: DEFAULT_MAX,
            label: None,
            value_text: ValueText::default(),
        }
    }

    /// A faixa — o `min`/`max` do `MeterRoot`.
    ///
    /// `min > max` **não** é corrigido, ao contrário do [`crate::slider::Slider::bounds`]: o primitivo
    /// produz um preenchimento espelhado e um valor colapsado em `min`, e é isso que sai aqui. Ver o
    /// doc do módulo.
    pub fn bounds(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// O `MeterLabel`, à esquerda do cabeçalho.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Liga o `MeterValue` com o conteúdo default dele: a porcentagem formatada.
    pub fn show_value(mut self) -> Self {
        self.value_text = ValueText::Auto;
        self
    }

    /// Liga o `MeterValue` com texto de quem chama — o `children` de função da referência.
    ///
    /// É por aqui que passam as casas decimais, a unidade e o locale que o `format`/`locale` do
    /// primitivo resolveriam. Ver "Ausente" no doc do módulo.
    pub fn value_text(mut self, text: impl Into<SharedString>) -> Self {
        self.value_text = ValueText::Custom(text.into());
        self
    }

    /// A porcentagem `[0,100]` que o indicador preenche. Ver a tabela de limites do módulo.
    pub fn percent(&self) -> f32 {
        percent_of(self.value, self.min, self.max)
    }

    /// O valor aparado na faixa — o número que iria em `aria-valuenow`, e o resíduo aproveitável da
    /// semântica de acessibilidade que o GPUI não tem.
    pub fn clamped_value(&self) -> f32 {
        clamp_value(self.value, self.min, self.max)
    }

    /// A leitura default do primitivo: a porcentagem arredondada, com `%`.
    pub fn formatted_value(&self) -> SharedString {
        format_percent(self.percent())
    }
}

impl RenderOnce for Meter {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let percent = self.percent();
        let leitura = value_reading(&self.value_text, percent);

        // `<div className="flex items-center justify-between gap-2">` — as classes que os três
        // particles com rótulo escrevem, iguais. Só existe se houver o que pôr nela.
        let cabecalho = header_needed(self.label.as_ref(), leitura.as_ref()).then(|| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(GAP))
                .when_some(self.label.clone(), |d, t| d.child(meter_label(t)))
                .when_some(leitura, |d, t| d.child(meter_value(t)))
        });

        // `flex w-full flex-col gap-2`.
        div()
            .flex()
            .flex_col()
            .w_full()
            .gap(px(GAP))
            .when_some(cabecalho, |d, c| d.child(c))
            .child(track(percent))
    }
}

/// O `MeterTrack`: `block h-2 w-full overflow-hidden bg-input`, com o indicador dentro.
fn track(percent: f32) -> Div {
    div()
        .h(px(TRACK_HEIGHT))
        .w_full()
        .overflow_hidden()
        .bg(palette().input.hsla())
        .child(indicator(percent))
}

/// O `MeterIndicator`: `bg-primary`, com `width: {percent}%` e `height: inherit`.
///
/// A largura é percentual DE VERDADE ([`gpui::relative`]), resolvida pelo taffy. Trocar por
/// `px(percent)` compila, e dá um indicador de 40px onde deveria ter 40% — o erro que o teste de
/// janela pega.
fn indicator(percent: f32) -> Div {
    div()
        .h_full()
        .w(relative(percent / 100.0))
        .bg(palette().primary.hsla())
}

/// O `MeterLabel` da referência: `font-medium text-foreground text-sm`.
///
/// Público pra quem quiser montar o cabeçalho à mão em vez de usar [`Meter::label`].
pub fn meter_label(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(TEXT_SM))
        .line_height(px(TEXT_SM_LINE))
        .text_color(palette().foreground.hsla())
        .child(text.into())
}

/// O `MeterValue` da referência: `text-foreground text-sm tabular-nums`.
///
/// Público pra quem quiser montar o cabeçalho à mão em vez de usar
/// [`Meter::show_value`]/[`Meter::value_text`].
pub fn meter_value(text: impl Into<SharedString>) -> impl IntoElement {
    let mut d = div()
        .text_size(px(TEXT_SM))
        .line_height(px(TEXT_SM_LINE))
        .text_color(palette().foreground.hsla());

    // `tabular-nums` por [`Styled::text_style`], e não por `.font(...)`: o `.font()` escreveria
    // família E features de uma vez, trocando a família herdada do shell.
    d.text_style()
        .get_or_insert_with(Default::default)
        .font_features = Some(tabular_nums());

    d.child(text.into())
}

// =================================================================================================
// Testes
// =================================================================================================

/// Um teste com janela de verdade, pro único ponto onde a conta encontra o layout: a largura do
/// indicador. `relative(0.4)` e `px(40.0)` compilam igual e produzem coisas diferentes na tela —
/// aritmética pura não pega essa troca, só o taffy pega.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{canvas, Bounds, Context, Pixels, Render, TestAppContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Largura do trilho no teste. Um número redondo e generoso: com 200px, 40% dá 80px exatos, e
    /// não há como um assert passar por arredondamento.
    const LARGURA: f32 = 200.0;

    struct Harness {
        percent: f32,
        medido: Rc<RefCell<Option<Bounds<Pixels>>>>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let alvo = self.medido.clone();
            // Um trilho de dimensões FIXAS, montado no teste: o que está sob medida é o indicador
            // dentro dele, e o `canvas` com `size_full` mede a caixa do pai.
            div().w(px(LARGURA)).h(px(TRACK_HEIGHT)).child(
                indicator(self.percent).child(
                    canvas(
                        move |bounds, _window, _cx| {
                            *alvo.borrow_mut() = Some(bounds);
                        },
                        |_, _, _, _| {},
                    )
                    .size_full(),
                ),
            )
        }
    }

    /// Mede a caixa do indicador pra uma porcentagem.
    fn medir(cx: &mut TestAppContext, percent: f32) -> Bounds<Pixels> {
        let medido: Rc<RefCell<Option<Bounds<Pixels>>>> = Rc::new(RefCell::new(None));
        let cravado = medido.clone();
        let window = cx.add_window(move |_w, _cx| Harness {
            percent,
            medido: cravado,
        });
        let vcx = gpui::VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let caixa = *medido.borrow();
        caixa.expect("o canvas mediu o indicador")
    }

    /// **A largura do indicador é a PORCENTAGEM do trilho, e a altura é a do trilho.**
    ///
    /// Guarda três trocas que compilam: `relative` por `px`, esquecer o `/100`, e `h_full` por uma
    /// altura inventada.
    #[gpui::test]
    fn o_indicador_ocupa_a_fracao_do_trilho(cx: &mut TestAppContext) {
        let quarenta = medir(cx, 40.0);
        assert_eq!(
            f32::from(quarenta.size.width),
            80.0,
            "40% de 200px são 80px — não 40"
        );
        assert_eq!(
            f32::from(quarenta.size.height),
            TRACK_HEIGHT,
            "`height: inherit` = a altura do trilho"
        );

        assert_eq!(f32::from(medir(cx, 0.0).size.width), 0.0, "vazio não pinta");
        assert_eq!(
            f32::from(medir(cx, 100.0).size.width),
            LARGURA,
            "cheio encosta na ponta, e não passa dela"
        );
        assert_eq!(f32::from(medir(cx, 12.5).size.width), 25.0, "fração quebrada");
    }
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// As medidas da referência, contra os literais que as classes do Tailwind valem.
    #[test]
    fn medidas_da_referencia() {
        assert_eq!(TRACK_HEIGHT, 8.0, "h-2");
        assert_eq!(GAP, 8.0, "gap-2, com --spacing = 4px");
        assert_eq!(TEXT_SM, 14.0, "text-sm");
        assert_eq!(TEXT_SM_LINE, 20.0, "o par de entrelinha do text-sm: 1.25rem");
        assert_eq!(DEFAULT_MIN, 0.0, "o min default do MeterRoot");
        assert_eq!(DEFAULT_MAX, 100.0, "o max default do MeterRoot");
    }

    /// **O trilho do medidor NÃO é o do slider.** 8px contra 4px, e é decisão, não coincidência: se
    /// alguém "unificar" os dois num só número, um dos dois componentes sai da referência.
    #[test]
    fn o_trilho_nao_compartilha_medida_com_o_slider() {
        assert_eq!(TRACK_HEIGHT, 8.0, "h-2 aqui");
        assert_ne!(
            TRACK_HEIGHT, 4.0,
            "h-1 é do slider — os dois trilhos têm espessuras diferentes"
        );
    }

    /// A paleta contra os literais dos tokens do coss, não contra as próprias constantes.
    #[test]
    fn paleta_segue_os_tokens_do_coss() {
        assert_eq!(METER_LIGHT.input, Rgba8(0x0000001a), "--input claro: preto 10%");
        assert_eq!(METER_DARK.input, Rgba8(0xffffff14), "--input escuro: branco 8%");
        assert_eq!(METER_LIGHT.primary, Rgba8(0x262626ff), "--primary claro");
        assert_eq!(METER_DARK.primary, Rgba8(0xf5f5f5ff), "--primary escuro");
        assert_eq!(METER_LIGHT.foreground, Rgba8(0x262626ff), "--foreground claro");
        assert_eq!(METER_DARK.foreground, Rgba8(0xf5f5f5ff), "--foreground escuro");
    }

    /// Os três tokens entram **opacos**: nenhum modificador `/N` neste componente. Se alguém
    /// escalar o alfa "pra suavizar", o indicador deixa de bater com a referência.
    #[test]
    fn primary_e_foreground_sao_opacos_e_input_nao_e() {
        assert_eq!(METER_LIGHT.primary.alpha(), 1.0);
        assert_eq!(METER_DARK.primary.alpha(), 1.0);
        assert_eq!(METER_LIGHT.foreground.alpha(), 1.0);
        assert_eq!(METER_DARK.foreground.alpha(), 1.0);
        // O `--input` já NASCE com alfa no `globals.css` — ele não é opaco, e não é um `/N` nosso.
        assert!(METER_LIGHT.input.alpha() < 0.2, "preto 10%");
        assert!(METER_DARK.input.alpha() < 0.2, "branco 8%");
    }

    /// **A conta é linear na faixa**, e não só na faixa `0..100`.
    ///
    /// Os dois últimos casos são os `p-meter-3` e `p-meter-4` da referência: `3` de `0..5` e `700`
    /// de `500..1000`.
    #[test]
    fn a_porcentagem_e_linear_na_faixa() {
        assert_eq!(percent_of(0.0, 0.0, 100.0), 0.0);
        assert_eq!(percent_of(50.0, 0.0, 100.0), 50.0);
        assert_eq!(percent_of(100.0, 0.0, 100.0), 100.0);
        assert_eq!(percent_of(25.0, 0.0, 200.0), 12.5);
        assert_eq!(percent_of(3.0, 0.0, 5.0), 60.0, "p-meter-3: Rating 3 de 5");
        assert_eq!(
            percent_of(700.0, 500.0, 1000.0),
            40.0,
            "p-meter-4: o mínimo NÃO é zero, e a conta desconta ele"
        );
    }

    /// **Valor fora da faixa apara nas pontas** em vez de extrapolar — se não aparasse, o indicador
    /// vazaria do trilho (e o `overflow-hidden` da referência é justo o cinto disso).
    #[test]
    fn a_porcentagem_apara_fora_da_faixa() {
        assert_eq!(percent_of(-50.0, 0.0, 100.0), 0.0, "abaixo do mínimo");
        assert_eq!(percent_of(150.0, 0.0, 100.0), 100.0, "acima do máximo");
        assert_eq!(percent_of(400.0, 500.0, 1000.0), 0.0);
        assert_eq!(percent_of(5000.0, 500.0, 1000.0), 100.0);
    }

    /// **`min == max` não divide por zero, e não devolve 0 sempre.**
    ///
    /// É a linha em que esta conta DIVERGE da `fraction` do slider (que devolve 0 pra faixa
    /// degenerada, sempre). Aqui: acima do teto lê cheio, no teto lê vazio, abaixo lê vazio — porque
    /// `+∞`, `NaN` e `−∞` passam pelos dois fallbacks do primitivo nessa ordem.
    #[test]
    fn faixa_degenerada_segue_o_primitivo_e_nao_o_slider() {
        assert_eq!(percent_of(7.0, 5.0, 5.0), 100.0, "acima do teto: +∞ apara em 100");
        assert_eq!(percent_of(5.0, 5.0, 5.0), 0.0, "no teto: 0/0 é NaN, e NaN vira 0");
        assert_eq!(percent_of(3.0, 5.0, 5.0), 0.0, "abaixo: −∞ apara em 0");
        // E o valor aparado colapsa no ponto único.
        assert_eq!(clamp_value(7.0, 5.0, 5.0), 5.0);
        assert_eq!(clamp_value(3.0, 5.0, 5.0), 5.0);
    }

    /// **Faixa invertida espelha o preenchimento e colapsa o valor.**
    ///
    /// Não é "consertado" trocando min/max como o slider faz: `min = 100, max = 0, valor = 30` dá
    /// 70% de preenchimento (a divisão troca de sinal duas vezes) e valor 100 (o `Math.max(min, …)`
    /// vence). São números diferentes de propósito — trocar min/max daria 30%.
    #[test]
    fn faixa_invertida_espelha_o_preenchimento_e_colapsa_o_valor() {
        assert_eq!(percent_of(30.0, 100.0, 0.0), 70.0);
        assert_eq!(percent_of(0.0, 100.0, 0.0), 100.0);
        assert_eq!(percent_of(100.0, 100.0, 0.0), 0.0);
        assert_eq!(
            clamp_value(30.0, 100.0, 0.0),
            100.0,
            "o clamp do JS deixa o piso vencer; o f32::clamp entraria em pânico aqui"
        );
    }

    /// **`NaN` cai no mínimo, e não propaga.** Sem os dois fallbacks explícitos do primitivo, a
    /// largura do indicador viraria `NaN` e o taffy engoliria o layout em silêncio.
    #[test]
    fn valor_nan_cai_no_minimo() {
        assert_eq!(percent_of(f32::NAN, 0.0, 100.0), 0.0);
        assert_eq!(percent_of(f32::NAN, 500.0, 1000.0), 0.0);
        assert_eq!(clamp_value(f32::NAN, 10.0, 20.0), 10.0, "o fallback é o min");
        assert!(!Meter::new(f32::NAN).percent().is_nan());
    }

    /// Infinito também é aparado — o primitivo comenta que é justo o que o `clamp` resolve.
    #[test]
    fn infinito_nao_escapa_da_faixa() {
        assert_eq!(percent_of(f32::INFINITY, 0.0, 100.0), 100.0);
        assert_eq!(percent_of(f32::NEG_INFINITY, 0.0, 100.0), 0.0);
        assert_eq!(clamp_value(f32::INFINITY, 0.0, 100.0), 100.0);
        assert_eq!(clamp_value(f32::NEG_INFINITY, 0.0, 100.0), 0.0);
    }

    /// **O valor aparado segue o `clamp` do primitivo**, que é o do JS: piso primeiro.
    #[test]
    fn o_valor_aparado_segue_o_clamp_do_primitivo() {
        assert_eq!(clamp_value(42.0, 0.0, 100.0), 42.0, "dentro passa inteiro");
        assert_eq!(clamp_value(-5.0, 0.0, 100.0), 0.0);
        assert_eq!(clamp_value(150.0, 0.0, 100.0), 100.0);
        assert_eq!(clamp_value(700.0, 500.0, 1000.0), 700.0);
    }

    /// **A leitura default é a porcentagem arredondada a inteiro**, com o `%` colado — o
    /// `style: "percent"` do `Intl` a 0 casas, no modo `halfExpand`.
    #[test]
    fn a_leitura_default_e_a_porcentagem_arredondada() {
        assert_eq!(format_percent(0.0), "0%");
        assert_eq!(format_percent(45.0), "45%");
        assert_eq!(format_percent(100.0), "100%");
        assert_eq!(format_percent(99.4), "99%", "arredonda pra baixo");
        assert_eq!(format_percent(45.5), "46%", "meio pra LONGE do zero (halfExpand)");
        assert_eq!(format_percent(12.5), "13%");
    }

    /// A tag do `tabular-nums` é `tnum`, ligada. Uma tag errada não é erro de compilação e não
    /// aparece no render — só some.
    #[test]
    fn tabular_nums_pede_a_tag_tnum() {
        assert_eq!(
            tabular_nums().tag_value_list(),
            &[("tnum".to_string(), 1)],
            "font-variant-numeric: tabular-nums é a feature OpenType `tnum`"
        );
    }

    /// **Os quatro particles da referência, pelo API público.** É o teste de integração da conta com
    /// os construtores: um `bounds` que não guardasse o mínimo, ou um `show_value` que formatasse o
    /// valor cru em vez da porcentagem, cairia aqui.
    #[test]
    fn os_particles_da_referencia_batem() {
        // p-meter-2: `<Meter value={50} />`
        let m = Meter::new(50.0);
        assert_eq!(m.percent(), 50.0);
        assert_eq!(m.clamped_value(), 50.0);
        assert_eq!(m.formatted_value(), "50%");

        // p-meter-1: `<Meter value={75}>` com rótulo e `<MeterValue />`
        let m = Meter::new(75.0).label("Storage usage").show_value();
        assert_eq!(m.percent(), 75.0);
        assert_eq!(m.formatted_value(), "75%");

        // p-meter-3: `<Meter max={5} value={3}>`
        let m = Meter::new(3.0).bounds(0.0, 5.0);
        assert_eq!(m.percent(), 60.0);
        assert_eq!(m.clamped_value(), 3.0);
        assert_eq!(m.formatted_value(), "60%", "a leitura é a PORCENTAGEM, não o valor");

        // p-meter-4: `<Meter max={1000} min={500} value={700}>`
        let m = Meter::new(700.0).bounds(500.0, 1000.0);
        assert_eq!(m.percent(), 40.0);
        assert_eq!(m.clamped_value(), 700.0);
    }

    /// A leitura de quem chama vence a automática, e é ela que sai — é o `children` de função do
    /// `MeterValue`, e a saída de emergência pro `format`/`locale` que não foram portados.
    #[test]
    fn a_leitura_de_quem_chama_vence_a_automatica() {
        let m = Meter::new(3.0).bounds(0.0, 5.0).value_text("3 / 5");
        assert_eq!(m.value_text, ValueText::Custom("3 / 5".into()));
        // E a porcentagem (que o indicador usa) não muda por causa do texto.
        assert_eq!(m.percent(), 60.0);
        assert_eq!(
            m.formatted_value(),
            "60%",
            "a leitura automática continua disponível a quem quiser"
        );
    }

    /// **Os construtores escrevem o que prometem.** Sem isto, um [`Meter::show_value`] que não faz
    /// nada compila e a suíte fica verde — o medidor só perde a leitura na tela.
    #[test]
    fn os_construtores_ligam_o_rotulo_e_a_leitura() {
        assert_eq!(
            Meter::new(75.0).label("Storage usage").label,
            Some(SharedString::from("Storage usage"))
        );
        assert_eq!(Meter::new(75.0).show_value().value_text, ValueText::Auto);
        assert_eq!(
            Meter::new(700.0).bounds(500.0, 1000.0).min,
            500.0,
            "o `bounds` guarda o MÍNIMO, não só o máximo"
        );
        assert_eq!(Meter::new(700.0).bounds(500.0, 1000.0).max, 1000.0);
    }

    /// O default é **sem** rótulo e **sem** leitura: o `p-meter-2` é só trilho e indicador.
    #[test]
    fn o_default_nao_tem_cabecalho() {
        let m = Meter::new(50.0);
        assert!(m.label.is_none());
        assert_eq!(m.value_text, ValueText::None);
    }

    /// **A leitura automática é a PORCENTAGEM, e a de quem chama passa intacta.** O erro plausível
    /// aqui — o [`ValueText::Auto`] formatar o valor cru — dá "3%" onde a referência dá "60%".
    #[test]
    fn a_leitura_resolve_os_tres_casos() {
        assert_eq!(value_reading(&ValueText::None, 60.0), None, "sem leitura");
        assert_eq!(
            value_reading(&ValueText::Auto, 60.0),
            Some("60%".into()),
            "a porcentagem, não o valor"
        );
        assert_eq!(
            value_reading(&ValueText::Custom("3 / 5".into()), 60.0),
            Some("3 / 5".into()),
            "o texto de quem chama passa intacto"
        );
    }

    /// **A linha de cabeçalho só existe se houver o que pôr nela.** Uma linha vazia não se vê, mas o
    /// `gap-2` da coluna se vê: sobrariam 8px acima do trilho em todo medidor sem rótulo.
    #[test]
    fn o_cabecalho_so_existe_com_conteudo() {
        let texto: SharedString = "x".into();
        assert!(!header_needed(None, None), "p-meter-2: nem rótulo nem leitura");
        assert!(header_needed(Some(&texto), None), "só rótulo");
        assert!(header_needed(None, Some(&texto)), "só leitura");
        assert!(header_needed(Some(&texto), Some(&texto)), "os dois");
    }
}

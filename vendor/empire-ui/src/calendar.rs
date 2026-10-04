//! `Calendar` — a **grade de mês** para escolher data. Porte do `calendar.tsx` do coss.
//!
//! ```ignore
//! let cal = cx.new(|cx| Calendar::new(cx).mode(CalendarMode::Range));
//! cx.subscribe(&cal, |_, _, ev: &CalendarEvent, _| match ev {
//!     CalendarEvent::Select { dates } => println!("{dates:?}"),
//! });
//! ```
//!
//! # O que o arquivo da referência é, e o que ele NÃO é
//!
//! O `calendar.tsx` do coss tem 139 linhas e **nenhuma lógica de calendário**: é um mapa de classes
//! Tailwind por *slot* do [`react-day-picker`][1] (que a v10 renomeou pra `@daypicker/react`) mais uma
//! troca do ícone de chevron. Ou seja: a referência entrega o **visual** e delega o resto pra uma
//! biblioteca de 5.000 linhas.
//!
//! Este porte, então, tem duas metades de origem diferente:
//!
//! | | de onde veio |
//! |---|---|
//! | cores, respiros, raios, corpos de texto, geometria de range | do `calendar.tsx`, direto |
//! | grade, dias de fora, modos de seleção, teclado, `data-*` | da FONTE do `react-day-picker` v10.0.1, levantada slot por slot |
//!
//! A segunda metade está registrada em `scratchpad/daypicker-spec.md` (árvore de DOM, tabela de
//! atributo→elemento, e as regras de seleção), lida da fonte e conferida contra DOM renderizado de
//! verdade. Onde este módulo diz "a referência faz X", é de lá que vem.
//!
//! # O par que organiza todo o componente: estado na CÉLULA, pintura no BOTÃO
//!
//! É a descoberta que dá forma ao porte. No `react-day-picker`, **todo** estado de um dia mora no
//! `<td>` (o slot `day`) como `data-selected` / `data-disabled` / `data-outside` / `data-today`, e
//! **toda** a pintura mora no `<button>` de dentro (o slot `day_button`). É por isso que as classes do
//! coss usam `in-data-selected:` (o `in-` do Tailwind v4 = "quando um ANCESTRAL casa") no botão, e
//! `data-selected:` sem `in-` no slot `outside`, que é a própria célula.
//!
//! Aqui isso virou dois elementos com os mesmos papéis: a **célula** ([`CELL`]×[`CELL`] + [`DAY_PAD_Y`]
//! de respiro vertical) carrega o estado e o fundo de `outside`; o **botão** de dentro
//! ([`CELL`]×[`CELL`]) carrega raio, fundo, cor de texto, anel e o pontinho de hoje. Inverter os dois
//! quebraria em silêncio: o fundo de `range-middle` pintaria a fresta de 1px entre as linhas, e o
//! `bg-accent/50` de um dia de fora selecionado desapareceria embaixo do botão.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`focus-visible:z-1`** no botão do dia (e o `z-1`/`z-2` do nav e do caption): o GPUI 0.2 não tem
//!   `z-index`, e a ordem de pintura é a ordem dos filhos. Consequência visível: o anel de foco de 3px
//!   de um dia é coberto pelo VIZINHO DA DIREITA e pelo de BAIXO, que pintam depois. É a mesma
//!   limitação já declarada no [`crate::group`] e no [`crate::toggle_group`]. O nav e o caption não
//!   sofrem: eles não se cruzam (ver [`CAPTION_MARGIN_X`]).
//! - **`transition-[border-radius,box-shadow]`** no dia selecionado: o GPUI não interpola raio nem
//!   sombra. A troca de forma ao estender um range é instantânea.
//! - **`::after`** do pontinho de hoje: virou um filho absoluto de verdade (ver [`today_dot`]) — o
//!   `-translate-x-1/2` que o centralizaria não existe no GPUI, então a centralização é por layout.
//! - **`pointer-coarse:after:min-*-11`** (alvo de 44px pra dedo): consulta de MÍDIA que nunca vale
//!   numa janela de desktop com mouse. Ramo morto, não omissão.
//! - **`<select>` nativo dos dropdowns de mês/ano** (`captionLayout="dropdown"`): ver "Ausente".
//!
//! **Resolvido em número**
//!
//! - `--cell-size` é o ramo `sm:` (`--spacing(9)` = **36px**), não o base de 40: uma janela de desktop
//!   está sempre acima do breakpoint. Mesma decisão do [`crate::Button`] e do [`crate::toggle`].
//! - `py-px` na célula com `size-(--cell-size)` no botão faz a LINHA medir 38 e o botão 36 — a altura
//!   de um `<td>` é mínimo, não teto, então o respiro empurra. Ver [`ROW_HEIGHT`]: é o que abre a
//!   fresta de 1px entre as semanas de um range.
//! - O anel de foco daqui é **3px a 50% de alfa e sem folga** (`ring-[3px] ring-ring/50`), e **não** o
//!   `ring-2 ring-offset-1` opaco do [`crate::Button`]. São dois anéis diferentes no mesmo design
//!   system; copiar o do botão era o erro fácil.
//!
//! **Desvio consciente**
//!
//! - **Semana começa no DOMINGO por default**, como a referência resolvida no locale `en-US`
//!   (`weekStartsOn: 0`) — e não na segunda. Troque com [`Calendar::week_start`].
//! - **Números de semana são ISO 8601**, e a referência usa por default o `getWeek` do locale (semana
//!   1 = a que contém 1º de janeiro), reservando o ISO pra `ISOWeek: true`. Os dois discordam: 1º de
//!   janeiro de 2027 é semana **1** no default da referência e **53** no ISO. Escolhi o ISO porque é o
//!   padrão internacional e o que um usuário fora dos EUA espera; a coluna só aparece com
//!   [`Calendar::show_week_number`], que vem desligada.
//! - **Textos em português**: "Agosto 2026" no caption e `dom seg ter qua qui sex sáb` no cabeçalho. A
//!   referência usa o locale (en-US: "August 2026" e duas letras, `cccccc`). Três letras porque duas
//!   em português são ambíguas — "se" serve pra segunda e "sex" pra sexta. Não há troca de locale (ver
//!   "Ausente").
//! - **Os botões de nav no limite ficam DESABILITADOS de verdade.** Na referência eles nunca recebem
//!   `disabled`: no `startMonth`/`endMonth` o chevron continua opaco, com hover, e só o handler ignora
//!   o clique (`Nav.tsx:67-69`) — ou seja o `disabled:opacity-64` que o próprio coss declara **nunca
//!   dispara**. É a única divergência aqui que é correção e não porte: um botão que parece clicável e
//!   não faz nada é defeito, e a classe existir sem nunca valer é a evidência de que a intenção era
//!   esta.
//!
//! **Superset consciente**
//!
//! - [`Calendar::fixed_weeks`] existe e a referência não a usa: sem ela o número de linhas é o mínimo
//!   necessário (**4 a 6** conforme o mês), e a altura do calendário PULA ao navegar. O default segue a
//!   referência (oscila); quem põe o calendário num popover de largura fixa liga.
//! - `Enter`/`Espaço` selecionam o dia focado. Na referência isso não é código da biblioteca — é o
//!   comportamento nativo do `<button>`, que o GPUI não tem. Sem isto o teclado navegaria sem
//!   conseguir escolher.
//!
//! **Ausente**
//!
//! - **`captionLayout="dropdown"`** (os seletores de mês e ano no lugar do rótulo). A referência os
//!   monta como um `<select>` NATIVO invisível (`opacity-0`) sobre um texto, e é o SO que desenha o
//!   menu; no GPUI não existe `<select>`, então o equivalente é montar dois popups com o
//!   [`crate::Select`] — o que é uma peça própria e uma decisão de desenho separada, não uma linha de
//!   CSS. Aqui está implementado o `captionLayout` **default** (`label`), que é o que o coss usa
//!   quando não se pede nada.
//! - `numberOfMonths` está implementado, mas `pagedNavigation` e `reverseMonths` não.
//! - `min`/`max`/`excludeDisabled`/`resetOnSelect` dos modos de seleção — ver
//!   [`crate::calendar_selection`].
//! - Troca de locale (nomes de mês/dia e ordem). Os textos são os do português, fixos.
//!
//! [1]: https://daypicker.dev

use std::rc::Rc;

use gpui::{
    div, px, App, Context, Div, ElementId, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, Window,
};

use crate::calendar_selection::{CalendarMode, FocusMove, RangeRoles, Selection};
use crate::color::Rgba8;
use crate::date::{month_grid, Date, Weekday};
use crate::theme;

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Convenção da casa: TODO valor é `0xRRGGBBAA`, com o byte de alfa, SEMPRE (ver [`crate::color`]).
// Misturar com os tokens de 6 dígitos de [`crate::theme`] desloca os canais e produz outra cor sem
// erro de compilação.

/// Tokens visuais do calendário, por tema.
#[derive(Clone, Copy, Debug)]
struct CalendarPalette {
    /// `--foreground` — o número de um dia comum, e os chevrons.
    foreground: Rgba8,
    /// `--muted-foreground` — o cabeçalho de dias da semana, os números de semana, os dias de fora e
    /// os desabilitados (estes três a 72%, ver [`MUTED_72`]).
    muted_fg: Rgba8,
    /// `--accent` — o fundo de hover, e o fundo do MEIO de um range.
    accent: Rgba8,
    /// `--primary` — o fundo de um dia selecionado, e o pontinho de hoje.
    primary: Rgba8,
    /// `--primary-foreground` — o número sobre um dia selecionado.
    primary_fg: Rgba8,
    /// `--background` — o pontinho de hoje QUANDO o dia está selecionado (ele inverte pra continuar
    /// visível sobre o fundo `--primary`).
    background: Rgba8,
    /// `--ring` — o anel de foco, aplicado a 50% (ver [`RING_ALPHA`]).
    ring: Rgba8,
}

/// Tema **claro**.
const CALENDAR_LIGHT: CalendarPalette = CalendarPalette {
    foreground: Rgba8(0x262626ff), // neutral-800
    muted_fg: Rgba8(0x686868ff),   // mix(neutral-500 90%, black)
    accent: Rgba8(0x0000000a),     // black 4%
    primary: Rgba8(0x262626ff),    // neutral-800
    primary_fg: Rgba8(0xfafafaff), // neutral-50
    background: Rgba8(0xffffffff),
    ring: Rgba8(0xa3a3a3ff), // neutral-400
};

/// Tema **escuro**.
const CALENDAR_DARK: CalendarPalette = CalendarPalette {
    foreground: Rgba8(0xf5f5f5ff), // neutral-100
    muted_fg: Rgba8(0x818181ff),   // mix(neutral-500 90%, white)
    accent: Rgba8(0xffffff0a),     // white 4%
    primary: Rgba8(0xf5f5f5ff),    // neutral-100
    primary_fg: Rgba8(0x262626ff), // neutral-800
    // `--background` escuro = mix(neutral-950 96%, white) = #141414.
    background: Rgba8(0x141414ff),
    ring: Rgba8(0x737373ff), // neutral-500
};

/// A paleta no tema corrente.
fn palette() -> &'static CalendarPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &CALENDAR_DARK,
        theme::ThemeMode::Light => &CALENDAR_LIGHT,
    }
}

// --- Alfas (os modificadores `/N` do Tailwind) ----------------------------------------------------
//
// ⚠️ No Tailwind v4 o `/N` é `color-mix(in oklab, <cor> N%, transparent)`: ele **multiplica** o alfa
// que a cor já tem. Aqui todas as bases são opacas, então multiplicar é o mesmo que atribuir — mas
// passar por [`Rgba8::scaled`] mantém a regra num lugar só.

/// `text-muted-foreground/72` — o cabeçalho, os números de semana, os dias de fora e os desabilitados.
const MUTED_72: f32 = 0.72;

/// `data-selected:bg-accent/50` no slot `outside` — o fundo de um dia de FORA que está selecionado.
const OUTSIDE_SELECTED_ALPHA: f32 = 0.50;

/// `focus-visible:ring-ring/50` — o anel deste componente é translúcido.
const RING_ALPHA: f32 = 0.50;

/// `[&[data-disabled]>*]:after:bg-foreground/30` — o pontinho de hoje num dia desabilitado.
const TODAY_DOT_DISABLED_ALPHA: f32 = 0.30;

/// `disabled:opacity-64`.
const DISABLED_OPACITY: f32 = 0.64;

/// `[&_svg:not([class*='opacity-'])]:opacity-80` — os chevrons do nav.
const ICON_OPACITY: f32 = 0.80;

// --- Geometria -----------------------------------------------------------------------------------

/// `--cell-size` — o lado de uma célula, e também o do botão de nav.
///
/// É o ramo `sm:` (`--spacing(9)`); o base (`--spacing(10)` = 40) é o de telas estreitas e não vale
/// numa janela de desktop. TUDO neste componente se mede em múltiplos disto: a largura da grade é
/// `7 × CELL` (mais uma coluna se houver número de semana), e as margens do caption reservam
/// exatamente uma célula de cada lado pros chevrons.
const CELL: f32 = 36.0;

/// `py-px` na célula do dia.
///
/// Não é decoração: é ele que abre a fresta de 1px entre as linhas, e é o que faz um range de duas
/// semanas parecer duas barras e não um bloco. Ver [`ROW_HEIGHT`].
const DAY_PAD_Y: f32 = 1.0;

/// A altura de uma linha da grade.
///
/// ⚠️ **Não é [`CELL`].** Na referência a célula é um `<td>` com `size-(--cell-size)` e `py-px`, e o
/// botão de dentro tem `size-(--cell-size)` também. Altura de `<td>` é MÍNIMO, não teto: o botão de 36
/// mais 1px de respiro em cima e embaixo empurram a linha pra **38**. O botão continua com 36 e fica
/// centrado. Reproduzir isso como `CELL` daria uma grade 2px mais curta por linha (12px no mês de seis
/// linhas) e colaria as barras de um range vertical.
const ROW_HEIGHT: f32 = CELL + 2.0 * DAY_PAD_Y;

/// Raio do botão de um dia e do botão de nav — `rounded-lg` = `--radius-lg` = `--radius` = **10px**.
const RADIUS: f32 = 10.0;

/// `mx-(--cell-size)` no caption: reserva uma célula de cada lado, que é onde os chevrons ficam.
///
/// É o que faz o rótulo centralizado nunca cobrir os botões, e é por isso que o `z-1`/`z-2` da
/// referência não faz falta aqui — o nav e o caption não se cruzam.
const CAPTION_MARGIN_X: f32 = CELL;

/// `px-1` no caption.
const CAPTION_PAD_X: f32 = 4.0;

/// `mb-1` no caption.
const CAPTION_MB: f32 = 4.0;

/// `gap-2` entre meses quando há mais de um.
const MONTHS_GAP: f32 = 8.0;

/// `text-base sm:text-sm` do caption e do dia — o ramo `sm:`.
const TEXT_SIZE: f32 = 14.0;

/// Entrelinha do par `text-sm` do Tailwind: 14 de fonte, **20** de linha.
///
/// Fixar é obrigatório (a armadilha nº 1 da casa): o default do GPUI é `relative(1.618_034)`, que daria
/// 22,7px e deslocaria o número do centro do botão.
const TEXT_LINE_HEIGHT: f32 = 20.0;

/// `text-xs` do cabeçalho e dos números de semana.
const SMALL_TEXT: f32 = 12.0;

/// Entrelinha do par `text-xs`: 12 de fonte, **16** de linha.
const SMALL_LINE_HEIGHT: f32 = 16.0;

/// `gap-2` dentro do rótulo do caption.
const CAPTION_GAP: f32 = 8.0;

/// `sm:size-4` dos chevrons.
const ICON_SIZE: f32 = 16.0;

/// `focus-visible:ring-[3px]` — e **sem** `ring-offset`, diferente do [`crate::Button`].
const RING_WIDTH: f32 = 3.0;

/// `size-[3px]` do pontinho de hoje.
const TODAY_DOT: f32 = 3.0;

/// `bottom-1` do pontinho.
const TODAY_DOT_BOTTOM: f32 = 4.0;

/// Os chevrons do nav, servidos pela [`crate::assets::Assets`].
const ICON_PREV: &str = "iconoir/regular/nav-arrow-left.svg";
const ICON_NEXT: &str = "iconoir/regular/nav-arrow-right.svg";

// =================================================================================================
// Textos
// =================================================================================================

/// Os nomes de mês, em português, indexados por `mês − 1`.
///
/// Sem "de": a referência formata `"LLLL yyyy"` ("August 2026"), e "Agosto de 2026" cresceria o
/// rótulo sem ganhar clareza. Ver o desvio declarado no doc do módulo.
const MESES: [&str; 12] = [
    "Janeiro",
    "Fevereiro",
    "Março",
    "Abril",
    "Maio",
    "Junho",
    "Julho",
    "Agosto",
    "Setembro",
    "Outubro",
    "Novembro",
    "Dezembro",
];

/// O nome curto de um dia da semana, na ordem ISO de [`Weekday::ALL`].
///
/// Três letras, e não as duas do `cccccc` da referência: em português duas letras são ambíguas ("se"
/// serve pra segunda **e** para sexta). Cabem folgadas nos 36px da célula a 12px de corpo.
const DIAS: [&str; 7] = ["seg", "ter", "qua", "qui", "sex", "sáb", "dom"];

/// O rótulo do caption — "Agosto 2026".
fn caption_label(mes: Date) -> SharedString {
    SharedString::from(format!(
        "{} {}",
        MESES[(mes.month() - 1) as usize],
        mes.year()
    ))
}

/// O rótulo de uma coluna do cabeçalho.
fn weekday_label(d: Weekday) -> &'static str {
    DIAS[d as usize]
}

// =================================================================================================
// Estado de um dia
// =================================================================================================

/// Tudo que o desenho de um dia precisa saber — os `data-*` que a referência põe no `<td>`, num
/// struct.
///
/// Existe como tipo próprio (e não como seis argumentos) porque é ele que carrega o par declarado no
/// doc do módulo: **isto é o estado da CÉLULA**, e é sobre ele que a pintura do botão decide. Ele
/// também é o que os testes conseguem afirmar sem GPU.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct DayState {
    date: Date,
    /// `data-outside` — o dia é de outro mês.
    outside: bool,
    /// `data-hidden` — a célula existe mas não mostra nada. É o `showOutsideDays: false` da
    /// referência: a célula **continua ocupando espaço** (a grade não colapsa), só fica vazia.
    hidden: bool,
    /// `data-selected`.
    selected: bool,
    /// `data-disabled`.
    disabled: bool,
    /// `data-today`. Pode ser verdadeiro numa célula `outside` — o pontinho aparece no dia do mês
    /// vizinho, e é o que a referência faz.
    today: bool,
    /// `data-focused` — o dia que recebe o teclado (o *roving tabindex* da referência, que tem um só
    /// por calendário).
    focused: bool,
    /// As classes `range-start` / `range-middle` / `range-end`.
    roles: RangeRoles,
}

impl DayState {
    /// **O raio de cada ponta do botão**, em `(início, fim)` — a geometria do range.
    ///
    /// As três regras do coss, e a razão de as duas últimas terem `:not(...)`:
    ///
    /// - `in-[.range-middle]:rounded-none` → o meio é quadrado dos dois lados;
    /// - `in-[.range-start:not(.range-end)]:rounded-e-none` → o início perde a ponta do FIM…
    /// - `in-[.range-end:not(.range-start)]:rounded-s-none` → …e o fim perde a ponta do INÍCIO.
    ///
    /// O `:not` existe porque um range de **um dia** carrega `start` e `end` ao mesmo tempo, e sem ele
    /// as duas regras se aplicariam juntas, deixando o dia único quadrado — quando ele tem que ser
    /// totalmente arredondado, igual a uma seleção simples.
    fn radii(&self) -> (f32, f32) {
        let r = self.roles;
        if r.middle {
            return (0.0, 0.0);
        }
        let inicio = if r.end && !r.start { 0.0 } else { RADIUS };
        let fim = if r.start && !r.end { 0.0 } else { RADIUS };
        (inicio, fim)
    }

    /// O **fundo do botão** em repouso, ou `None` pra transparente.
    ///
    /// A precedência é a da cascata do coss: `in-data-selected:bg-primary` vale pra qualquer dia
    /// selecionado, e `in-[.range-middle]:in-data-selected:bg-accent` sobrescreve no meio do range —
    /// é o que faz as pontas de um range serem sólidas e o meio ser um trilho fraco.
    fn background(&self, p: &CalendarPalette) -> Option<Hsla> {
        if !self.selected {
            return None;
        }
        Some(if self.roles.middle {
            p.accent.hsla()
        } else {
            p.primary.hsla()
        })
    }

    /// A **cor do número**.
    ///
    /// A ordem das perguntas é a ordem de especificidade das classes do coss:
    /// `in-data-selected:in-data-outside:text-primary-foreground` (a mais específica) vem antes de
    /// `in-data-outside:text-muted-foreground/72`, e o meio do range volta pro `--foreground` porque
    /// ali o fundo é fraco e o texto tem que ler como texto comum.
    fn text_color(&self, p: &CalendarPalette) -> Hsla {
        if self.selected {
            if self.roles.middle {
                p.foreground.hsla()
            } else {
                // Vale também pro dia de FORA selecionado — é a regra
                // `in-data-selected:in-data-outside:text-primary-foreground`.
                p.primary_fg.hsla()
            }
        } else if self.disabled || self.outside {
            p.muted_fg.scaled(MUTED_72)
        } else {
            p.foreground.hsla()
        }
    }

    /// A cor do **pontinho de hoje**, ou `None` se este dia não é hoje.
    ///
    /// Ele inverte pra `--background` quando o dia está selecionado (senão desapareceria sobre o fundo
    /// `--primary`), e vai pra `--foreground`/30 quando o dia está desabilitado. No MEIO de um range o
    /// fundo é fraco, então ele continua `--primary` — é o `:not(.range-middle)` da regra do coss.
    fn today_dot(&self, p: &CalendarPalette) -> Option<Hsla> {
        if !self.today {
            return None;
        }
        Some(if self.disabled {
            p.foreground.scaled(TODAY_DOT_DISABLED_ALPHA)
        } else if self.selected && !self.roles.middle {
            p.background.hsla()
        } else {
            p.primary.hsla()
        })
    }
}

// =================================================================================================
// Evento
// =================================================================================================

/// Emitido quando a seleção muda por interação do usuário.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarEvent {
    /// As datas selecionadas **depois** do clique, em ordem crescente.
    ///
    /// Uma lista (e não um `Option<Date>`) porque é a mesma forma nos três modos: vazia quando a
    /// seleção foi limpa, um elemento no `Single`, N no `Multiple`, e duas pontas no `Range`. Quem
    /// precisa das pontas nomeadas lê [`Calendar::selection`].
    Select { dates: Vec<Date> },
}

// =================================================================================================
// O componente
// =================================================================================================

/// Grade de mês para escolher data, com o visual do coss. Ver o doc do módulo.
///
/// É uma **entidade** (`Entity<Calendar>`), como o [`crate::Tabs`] e o
/// [`crate::ToggleGroup`]: ele guarda o mês exibido, a seleção e o dia focado, e o teclado precisa
/// desse estado entre frames.
pub struct Calendar {
    id: ElementId,
    /// O primeiro mês exibido, sempre no dia 1º.
    month: Date,
    selection: Selection,
    week_start: Weekday,
    show_outside_days: bool,
    show_week_number: bool,
    fixed_weeks: bool,
    months: usize,
    /// Limites de navegação — o `startMonth`/`endMonth` da referência.
    start_month: Option<Date>,
    end_month: Option<Date>,
    /// Quais dias não podem ser escolhidos.
    ///
    /// Um predicado, e não uma lista: a referência aceita *matchers* ("antes de hoje", "todo fim de
    /// semana"), e uma `Vec<Date>` não expressa nenhum dos dois sem o call site materializar
    /// milhares de datas.
    disabled: Option<Rc<dyn Fn(Date) -> bool>>,
    /// O dia que recebe o teclado. `None` até o primeiro foco — aí ele nasce no dia selecionado, ou em
    /// hoje, ou no dia 1º (é a prioridade do *roving tabindex* da referência).
    focused: Option<Date>,
    focus_handle: FocusHandle,
    /// [`Date::today`] lido UMA vez na construção.
    ///
    /// Não é cache por performance: é pra o pontinho de hoje não mudar de lugar no meio de um frame se
    /// a meia-noite passar entre duas chamadas. Um calendário aberto na virada do dia mostra o dia
    /// anterior até ser reconstruído, o que é melhor que dois dias marcados no mesmo render.
    today: Date,
}

impl Calendar {
    /// Um calendário no mês de **hoje**, em modo [`CalendarMode::Single`] e sem nada selecionado.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let today = Date::today();
        Self {
            id: ElementId::Name("calendar".into()),
            month: today.first_of_month(),
            selection: Selection::new(CalendarMode::Single),
            // Domingo, como o locale `en-US` que a referência resolve por default.
            week_start: Weekday::Sunday,
            // A referência liga isto por default (a biblioteca, sozinha, tem desligado).
            show_outside_days: true,
            show_week_number: false,
            fixed_weeks: false,
            months: 1,
            start_month: None,
            end_month: None,
            disabled: None,
            focused: None,
            focus_handle: cx.focus_handle(),
            today,
        }
    }

    /// O modo de seleção. Trocar o modo **zera** a seleção: as formas guardadas são diferentes, e
    /// converter um range em três dias soltos (ou o contrário) seria adivinhação.
    pub fn mode(mut self, mode: CalendarMode) -> Self {
        let required = self.selection.is_required();
        self.selection = Selection::new(mode).required(required);
        self
    }

    /// Semeia a seleção. Ver [`Selection::with_dates`] pra como cada modo interpreta a lista.
    pub fn selected(mut self, dates: &[Date]) -> Self {
        self.selection = std::mem::take(&mut self.selection).with_dates(dates);
        self
    }

    /// Impede que um clique LIMPE a seleção — o `required` da referência.
    pub fn required(mut self, required: bool) -> Self {
        self.selection = std::mem::take(&mut self.selection).required(required);
        self
    }

    /// Em que dia a semana começa. O default é **domingo** (ver o desvio declarado no doc do módulo).
    pub fn week_start(mut self, week_start: Weekday) -> Self {
        self.week_start = week_start;
        self
    }

    /// Mostra os dias dos meses vizinhos nas frestas da primeira e da última semana.
    ///
    /// Desligado, as células **continuam ocupando espaço** — a grade não colapsa, ela fica com
    /// buracos. É o `hidden: "invisible"` da referência.
    pub fn show_outside_days(mut self, show: bool) -> Self {
        self.show_outside_days = show;
        self
    }

    /// Acrescenta a coluna de número de semana (ISO 8601 — ver o desvio declarado).
    pub fn show_week_number(mut self, show: bool) -> Self {
        self.show_week_number = show;
        self
    }

    /// Trava a grade em **seis linhas**, pra a altura não mudar ao navegar. Ver o superset declarado.
    pub fn fixed_weeks(mut self, fixed: bool) -> Self {
        self.fixed_weeks = fixed;
        self
    }

    /// Quantos meses lado a lado. Zero é tratado como um — um calendário sem mês não é um estado útil.
    pub fn number_of_months(mut self, n: usize) -> Self {
        self.months = n.max(1);
        self
    }

    /// O mês inicialmente exibido (o dia é ignorado).
    pub fn month(mut self, month: Date) -> Self {
        self.month = month.first_of_month();
        self
    }

    /// Os limites de navegação. É o que desabilita os chevrons nas pontas.
    pub fn nav_limits(mut self, start: Option<Date>, end: Option<Date>) -> Self {
        self.start_month = start.map(Date::first_of_month);
        self.end_month = end.map(Date::first_of_month);
        self
    }

    /// Quais dias não podem ser escolhidos.
    pub fn disabled(mut self, pred: impl Fn(Date) -> bool + 'static) -> Self {
        self.disabled = Some(Rc::new(pred));
        self
    }

    /// A seleção corrente.
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// O primeiro mês exibido.
    pub fn displayed_month(&self) -> Date {
        self.month
    }

    /// Troca o mês exibido, **saturando** nos limites de navegação — é o `goToMonth` da referência,
    /// que faz clamp em vez de recusar.
    pub fn go_to_month(&mut self, month: Date, cx: &mut Context<Self>) {
        let mut alvo = month.first_of_month();
        if let Some(inicio) = self.start_month {
            alvo = alvo.max(inicio);
        }
        if let Some(fim) = self.end_month {
            // O último mês exibível é `fim` menos os meses extras, senão um calendário de dois meses
            // passaria do limite com o segundo painel.
            alvo = alvo.min(fim.add_months(-(self.months as i32 - 1)));
        }
        if alvo != self.month {
            self.month = alvo;
            cx.notify();
        }
    }

    /// Troca a seleção de fora. **Não emite evento** — evento é reação a interação do usuário, e quem
    /// chamou isto já sabe o que fez (mesma regra do [`crate::ToggleGroup`] e do [`crate::Tabs`]).
    pub fn set_selected(&mut self, dates: &[Date], cx: &mut Context<Self>) {
        self.selection = std::mem::take(&mut self.selection).with_dates(dates);
        cx.notify();
    }

    /// Se este dia está desabilitado.
    fn is_disabled(&self, date: Date) -> bool {
        self.disabled.as_ref().is_some_and(|f| f(date))
    }

    /// Se há mês anterior/seguinte disponível — o que decide se o chevron está ativo.
    fn can_go(&self, delta: i32) -> bool {
        let alvo = self.month.add_months(delta);
        match delta {
            d if d < 0 => self.start_month.is_none_or(|inicio| alvo >= inicio),
            _ => self
                .end_month
                .is_none_or(|fim| alvo <= fim.add_months(-(self.months as i32 - 1))),
        }
    }

    /// O dia que o teclado move — o *focus target* da referência, resolvido na primeira vez.
    ///
    /// A prioridade é a dela (`calculateFocusTarget.ts`), reduzida ao que existe aqui: o dia focado,
    /// senão o primeiro selecionado, senão hoje (se estiver no mês), senão o dia 1º. Nunca um dia de
    /// fora nem um desabilitado.
    fn focus_target(&self) -> Date {
        if let Some(f) = self.focused {
            return f;
        }
        let candidatos = [
            self.selection.dates().first().copied(),
            Some(self.today).filter(|t| t.same_month(self.month)),
        ];
        candidatos
            .into_iter()
            .flatten()
            .find(|d| !self.is_disabled(*d))
            .unwrap_or(self.month)
    }

    /// Aplica um clique num dia.
    fn click_day(&mut self, date: Date, cx: &mut Context<Self>) {
        if self.is_disabled(date) {
            // A referência move o foco pro dia desabilitado mas NÃO seleciona — o clique é engolido
            // de propósito, pra o usuário ver onde encostou.
            self.focused = Some(date);
            cx.notify();
            return;
        }
        self.focused = Some(date);
        if self.selection.click(date) {
            cx.emit(CalendarEvent::Select {
                dates: self.selection.dates(),
            });
        }
        cx.notify();
    }

    /// O teclado: as oito teclas da referência, mais `Enter`/`Espaço` (que lá são o `<button>` nativo
    /// e aqui têm que ser código — ver o superset declarado).
    ///
    /// `Shift` troca o eixo do salto, como na referência: nas setas horizontais vira **mês**, nas
    /// verticais e no PageUp/PageDown vira **ano**.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let shift = event.keystroke.modifiers.shift;
        let atual = self.focus_target();
        let mv = match event.keystroke.key.as_str() {
            "left" if shift => FocusMove::PrevMonth,
            "right" if shift => FocusMove::NextMonth,
            "left" => FocusMove::PrevDay,
            "right" => FocusMove::NextDay,
            "up" if shift => FocusMove::PrevYear,
            "down" if shift => FocusMove::NextYear,
            "up" => FocusMove::PrevWeek,
            "down" => FocusMove::NextWeek,
            "pageup" if shift => FocusMove::PrevYear,
            "pagedown" if shift => FocusMove::NextYear,
            "pageup" => FocusMove::PrevMonth,
            "pagedown" => FocusMove::NextMonth,
            "home" => FocusMove::WeekStart,
            "end" => FocusMove::WeekEnd,
            "enter" | "space" => {
                crate::focus_ring::keyboard_used(window);
                self.click_day(atual, cx);
                return;
            }
            _ => return,
        };
        crate::focus_ring::keyboard_used(window);

        let alvo = crate::calendar_selection::move_focus(atual, mv, self.week_start);
        // Pula dias desabilitados na direção do movimento, como a referência faz — mas com teto, pra
        // um predicado que desabilita tudo não virar laço infinito. 365 é o teto dela também.
        let passo = if alvo > atual { 1 } else { -1 };
        let mut alvo = alvo;
        for _ in 0..365 {
            if !self.is_disabled(alvo) {
                break;
            }
            alvo = alvo.add_days(passo);
        }
        if self.is_disabled(alvo) {
            return;
        }
        self.focused = Some(alvo);
        // Mover o foco pra fora da grade NAVEGA o mês — é o `goToDay` da referência.
        if !self.is_displayed(alvo) {
            let delta = if alvo < self.month { -1 } else { 1 };
            self.go_to_month(self.month.add_months(delta), cx);
        }
        cx.notify();
    }

    /// Se a data cai num dos meses exibidos.
    fn is_displayed(&self, date: Date) -> bool {
        (0..self.months).any(|i| date.same_month(self.month.add_months(i as i32)))
    }
}

impl Focusable for Calendar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<CalendarEvent> for Calendar {}

// =================================================================================================
// Desenho
// =================================================================================================

/// O **pontinho de hoje**.
///
/// A referência é um `::after` com `bottom-1 start-1/2 -translate-x-1/2`. O GPUI não tem `transform`,
/// então a centralização é por LAYOUT: uma faixa absoluta que atravessa o botão, com `justify_center`,
/// e o ponto dentro. Sai no mesmo lugar e sem depender de saber a largura.
fn today_dot(color: Hsla) -> Div {
    div()
        .absolute()
        .bottom(px(TODAY_DOT_BOTTOM))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .child(
            div()
                .size(px(TODAY_DOT))
                .rounded_full()
                .flex_none()
                .bg(color),
        )
}

/// O **anel de foco** — `ring-[3px] ring-ring/50`, sem folga.
///
/// ⚠️ Não é o anel do [`crate::Button`] (2px opaco com 1px de folga). Um `ring` do Tailwind é a forma
/// do elemento dilatada pela espessura, então aqui é um overlay 3px maior de cada lado com o raio
/// crescendo junto — e o raio de cada ponta vem da geometria do range, senão o anel de um dia no meio
/// de um range sairia arredondado sobre um botão quadrado.
fn ring_overlay(radii: (f32, f32), color: Hsla) -> Div {
    let (inicio, fim) = radii;
    let crescer = |r: f32| if r > 0.0 { r + RING_WIDTH } else { 0.0 };
    div()
        .absolute()
        .top(px(-RING_WIDTH))
        .bottom(px(-RING_WIDTH))
        .left(px(-RING_WIDTH))
        .right(px(-RING_WIDTH))
        .border(px(RING_WIDTH))
        .border_color(color)
        .rounded_tl(px(crescer(inicio)))
        .rounded_bl(px(crescer(inicio)))
        .rounded_tr(px(crescer(fim)))
        .rounded_br(px(crescer(fim)))
}

impl Calendar {
    /// Um botão de navegação (chevron).
    ///
    /// `ativo` falso **desabilita de verdade** — é a divergência declarada no doc do módulo: na
    /// referência o chevron no limite fica opaco e clicável-sem-efeito.
    fn nav_button(
        &self,
        nome: &'static str,
        icone: &'static str,
        delta: i32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette();
        let ativo = self.can_go(delta);
        let mut cor = p.foreground.hsla();
        cor.a *= ICON_OPACITY;

        let mut el = div()
            .id(ElementId::NamedChild(Box::new(self.id.clone()), nome.into()))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(CELL))
            .rounded(px(RADIUS))
            .child(
                gpui::svg()
                    .path(icone)
                    .size(px(ICON_SIZE))
                    .flex_none()
                    .text_color(cor),
            );
        if ativo {
            el = el
                .cursor(gpui::CursorStyle::PointingHand)
                .hover(|s| s.bg(p.accent.hsla()))
                .on_click(cx.listener(move |this, _e, window, cx| {
                    crate::focus_ring::pointer_used(window);
                    this.go_to_month(this.month.add_months(delta), cx);
                }));
        } else {
            el = el.opacity(DISABLED_OPACITY);
        }
        el
    }

    /// O estado de um dia, montado a partir da célula da grade.
    fn day_state(&self, cell: crate::date::Cell, mes: Date) -> DayState {
        let date = cell.date;
        // `outside` é relativo AO MÊS DESTE PAINEL, não ao primeiro: num calendário de dois meses o
        // mesmo dia é de fora num painel e de dentro no outro.
        let outside = !date.same_month(mes);
        DayState {
            date,
            outside,
            hidden: outside && !self.show_outside_days,
            selected: self.selection.is_selected(date),
            disabled: self.is_disabled(date),
            today: date == self.today,
            focused: self.focused == Some(date),
            roles: self.selection.range_roles(date),
        }
    }

    /// Uma célula da grade: a **célula** carrega o estado e o botão de dentro carrega a pintura (ver o
    /// doc do módulo).
    fn render_day(&self, st: DayState, focado_no_teclado: bool, cx: &mut Context<Self>) -> Div {
        let p = palette();
        // A célula. `size` horizontal é CELL; a altura é ROW_HEIGHT por causa do `py-px`.
        let mut celula = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .w(px(CELL))
            .h(px(ROW_HEIGHT))
            .py(px(DAY_PAD_Y));
        // Célula escondida ocupa espaço e não mostra nada — a grade não colapsa.
        if st.hidden {
            return celula;
        }
        // O `data-selected:bg-accent/50` do slot `outside`: fica na CÉLULA, atrás do botão, e por isso
        // só aparece nas duas frestas de 1px que o `py-px` abre.
        if st.outside && st.selected {
            celula = celula.bg(p.accent.scaled(OUTSIDE_SELECTED_ALPHA));
        }

        let (r_inicio, r_fim) = st.radii();
        let mut botao = div()
            .id(ElementId::NamedChild(
                Box::new(self.id.clone()),
                format!("d{}", st.date.to_days()).into(),
            ))
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(CELL))
            .rounded_tl(px(r_inicio))
            .rounded_bl(px(r_inicio))
            .rounded_tr(px(r_fim))
            .rounded_br(px(r_fim))
            .text_size(px(TEXT_SIZE))
            .line_height(px(TEXT_LINE_HEIGHT))
            .text_color(st.text_color(p))
            .child(SharedString::from(st.date.day().to_string()));

        if let Some(bg) = st.background(p) {
            botao = botao.bg(bg);
        }
        if st.disabled {
            // `disabled:opacity-64` + `in-data-disabled:line-through`, e sai do caminho do ponteiro.
            botao = botao.opacity(DISABLED_OPACITY).line_through();
        } else {
            botao = botao.cursor(gpui::CursorStyle::PointingHand);
            // `not-in-data-selected:hover:bg-accent`: o hover só pinta quando o dia NÃO está
            // selecionado — senão ele apagaria o `--primary` da seleção.
            if !st.selected {
                botao = botao.hover(|s| s.bg(p.accent.hsla()));
            }
            let dia = st.date;
            botao = botao.on_click(cx.listener(move |this, _e, window, cx| {
                crate::focus_ring::pointer_used(window);
                this.click_day(dia, cx);
            }));
        }
        if let Some(cor) = st.today_dot(p) {
            botao = botao.child(today_dot(cor));
        }
        if st.focused && focado_no_teclado {
            botao = botao.child(ring_overlay(st.radii(), p.ring.scaled(RING_ALPHA)));
        }

        celula.child(botao)
    }

    /// O cabeçalho de dias da semana.
    fn render_weekdays(&self) -> Div {
        let p = palette();
        let mut linha = div().flex().flex_none();
        if self.show_week_number {
            // A coluna de número de semana tem cabeçalho VAZIO na referência, e ocupa o espaço.
            linha = linha.child(div().flex_none().size(px(CELL)));
        }
        for d in self.week_start.week_from() {
            linha = linha.child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(CELL))
                    .text_size(px(SMALL_TEXT))
                    .line_height(px(SMALL_LINE_HEIGHT))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(p.muted_fg.scaled(MUTED_72))
                    .child(weekday_label(d)),
            );
        }
        linha
    }

    /// Um mês inteiro: caption + cabeçalho + grade.
    fn render_month(&self, i: usize, window: &Window, cx: &mut Context<Self>) -> Div {
        let p = palette();
        let mes = self.month.add_months(i as i32);
        let focado_no_teclado =
            self.focus_handle.is_focused(window) && crate::focus_ring::visible();

        let caption = div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .h(px(CELL))
            .mx(px(CAPTION_MARGIN_X))
            .px(px(CAPTION_PAD_X))
            .mb(px(CAPTION_MB))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(CAPTION_GAP))
                    .text_size(px(TEXT_SIZE))
                    .line_height(px(TEXT_LINE_HEIGHT))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(p.foreground.hsla())
                    .child(caption_label(mes)),
            );

        let mut grade = div().flex().flex_col().flex_none();
        for semana in month_grid(mes.year(), mes.month(), self.week_start, self.fixed_weeks) {
            let mut linha = div().flex().flex_none();
            if self.show_week_number {
                linha = linha.child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .w(px(CELL))
                        .h(px(ROW_HEIGHT))
                        .text_size(px(SMALL_TEXT))
                        .line_height(px(SMALL_LINE_HEIGHT))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(p.muted_fg.scaled(MUTED_72))
                        // Zero à esquerda, como o `formatWeekNumber` da referência.
                        .child(SharedString::from(format!("{:02}", semana.number.1))),
                );
            }
            for cell in semana.days {
                let st = self.day_state(cell, mes);
                linha = linha.child(self.render_day(st, focado_no_teclado, cx));
            }
            grade = grade.child(linha);
        }

        div()
            .flex()
            .flex_col()
            .child(caption)
            .child(self.render_weekdays())
            .child(grade)
    }
}

impl Render for Calendar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // O nav é IRMÃO dos meses e absoluto sobre a faixa do caption, com um botão em cada ponta do
        // container — inclusive com vários meses, onde existe UM nav só (é o que a referência faz).
        let nav = div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .flex()
            .justify_between()
            .child(self.nav_button("prev", ICON_PREV, -1, cx))
            .child(self.nav_button("next", ICON_NEXT, 1, cx));

        let mut meses = div()
            .relative()
            .flex()
            .flex_none()
            .gap(px(MONTHS_GAP))
            .child(nav);
        for i in 0..self.months {
            meses = meses.child(self.render_month(i, window, cx));
        }

        div()
            .id(self.id.clone())
            .track_focus(&self.focus_handle)
            .flex()
            .flex_none()
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                this.on_key(e, window, cx);
            }))
            .child(meses)
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, dia: u32) -> Date {
        Date::new(y, m, dia).expect("data de teste válida")
    }

    /// Um estado de dia neutro, pros testes mexerem num campo por vez.
    fn estado(date: Date) -> DayState {
        DayState {
            date,
            outside: false,
            hidden: false,
            selected: false,
            disabled: false,
            today: false,
            focused: false,
            roles: RangeRoles::default(),
        }
    }

    /// A geometria em número, contra o `calendar.tsx`.
    #[test]
    fn geometria_e_a_da_referencia() {
        assert_eq!(CELL, 36.0, "--cell-size no ramo sm: (--spacing(9))");
        assert_eq!(RADIUS, 10.0, "rounded-lg = --radius");
        assert_eq!(CAPTION_MARGIN_X, CELL, "mx-(--cell-size)");
        assert_eq!((CAPTION_PAD_X, CAPTION_MB), (4.0, 4.0), "px-1 mb-1");
        assert_eq!(MONTHS_GAP, 8.0, "gap-2");
        assert_eq!(ICON_SIZE, 16.0, "sm:size-4");
        assert_eq!((TODAY_DOT, TODAY_DOT_BOTTOM), (3.0, 4.0), "size-[3px] bottom-1");
        assert_eq!(DISABLED_OPACITY, 0.64, "disabled:opacity-64");
        assert_eq!(ICON_OPACITY, 0.80);
    }

    /// **A linha mede 38, não 36.** O `py-px` da célula com o botão de 36 dentro empurra a altura, e é
    /// isso que abre a fresta entre as semanas de um range. Um porte que usasse `CELL` como altura de
    /// linha encurtaria a grade em 12px num mês de seis linhas.
    #[test]
    fn a_linha_e_dois_pixels_mais_alta_que_a_celula() {
        assert_eq!(DAY_PAD_Y, 1.0, "py-px");
        assert_eq!(ROW_HEIGHT, 38.0, "36 + 1 + 1");
        assert_eq!(ROW_HEIGHT - CELL, 2.0 * DAY_PAD_Y);
        // O erro que isso evita, em número: seis linhas.
        assert_eq!(6.0 * ROW_HEIGHT - 6.0 * CELL, 12.0);
    }

    /// **O anel deste componente NÃO é o do botão.** São dois anéis diferentes no mesmo design system:
    /// aqui `ring-[3px] ring-ring/50` (translúcido, sem folga), lá `ring-2 ring-offset-1` opaco.
    #[test]
    fn o_anel_nao_e_o_do_botao() {
        assert_eq!(RING_WIDTH, 3.0, "ring-[3px]");
        assert_eq!(RING_ALPHA, 0.50, "ring-ring/50");
        // O do botão, pra contraste — se algum dia alguém unificar os dois, este teste cai.
        assert!(
            RING_WIDTH != 2.0,
            "o anel do Button é 2px COM folga de 1; o daqui é 3px sem folga"
        );
    }

    /// **A geometria de um range**, incluindo o caso que o `:not(...)` da referência existe pra
    /// resolver: um range de UM dia carrega `start` e `end` juntos e tem que ficar todo arredondado.
    #[test]
    fn o_raio_de_um_range_segue_os_papeis() {
        let dia = d(2026, 8, 10);
        let com = |start, middle, end| {
            let mut st = estado(dia);
            st.roles = RangeRoles { start, middle, end };
            st.radii()
        };
        assert_eq!(com(false, false, false), (RADIUS, RADIUS), "dia solto");
        assert_eq!(com(true, false, false), (RADIUS, 0.0), "início perde o fim");
        assert_eq!(com(false, false, true), (0.0, RADIUS), "fim perde o início");
        assert_eq!(com(false, true, false), (0.0, 0.0), "meio é quadrado");
        assert_eq!(
            com(true, false, true),
            (RADIUS, RADIUS),
            "range de UM dia: as duas pontas juntas mantêm o arredondamento inteiro — é o que o \
             `:not(.range-end)` / `:not(.range-start)` do coss garante"
        );
    }

    /// O fundo: `--primary` nas pontas e `--accent` no meio. É o que faz um range ler como duas
    /// pastilhas ligadas por um trilho fraco.
    #[test]
    fn o_fundo_de_um_range_e_forte_nas_pontas_e_fraco_no_meio() {
        for p in [&CALENDAR_LIGHT, &CALENDAR_DARK] {
            let mut st = estado(d(2026, 8, 10));
            assert_eq!(st.background(p), None, "dia não selecionado não tem fundo");

            st.selected = true;
            assert_eq!(st.background(p), Some(p.primary.hsla()), "ponta");
            assert_eq!(st.text_color(p), p.primary_fg.hsla());

            st.roles.middle = true;
            assert_eq!(st.background(p), Some(p.accent.hsla()), "meio");
            assert_eq!(
                st.text_color(p),
                p.foreground.hsla(),
                "no meio o texto volta pro --foreground, porque o fundo ali é fraco"
            );
        }
    }

    /// A cor do número, na ordem de especificidade das classes do coss. O caso que importa: um dia de
    /// FORA **selecionado** usa `--primary-foreground`, e não o cinza de dia de fora.
    #[test]
    fn a_cor_do_numero_segue_a_especificidade_do_coss() {
        let p = &CALENDAR_LIGHT;
        let mut st = estado(d(2026, 8, 10));
        assert_eq!(st.text_color(p), p.foreground.hsla(), "dia comum");

        st.outside = true;
        assert_eq!(st.text_color(p), p.muted_fg.scaled(MUTED_72), "de fora");

        st.selected = true;
        assert_eq!(
            st.text_color(p),
            p.primary_fg.hsla(),
            "de fora E selecionado: a regra in-data-selected:in-data-outside vence"
        );

        let mut dis = estado(d(2026, 8, 10));
        dis.disabled = true;
        assert_eq!(dis.text_color(p), p.muted_fg.scaled(MUTED_72), "desabilitado");
    }

    /// **O pontinho de hoje inverte** pra continuar visível: `--primary` num dia comum, `--background`
    /// sobre a seleção, e `--foreground`/30 quando desabilitado. No MEIO de um range ele NÃO inverte,
    /// porque ali o fundo é fraco.
    #[test]
    fn o_pontinho_de_hoje_inverte_sobre_a_selecao() {
        for p in [&CALENDAR_LIGHT, &CALENDAR_DARK] {
            let hoje = d(2026, 8, 4);
            let mut st = estado(hoje);
            assert_eq!(st.today_dot(p), None, "só aparece em hoje");

            st.today = true;
            assert_eq!(st.today_dot(p), Some(p.primary.hsla()));

            st.selected = true;
            assert_eq!(st.today_dot(p), Some(p.background.hsla()), "inverte");

            st.roles.middle = true;
            assert_eq!(
                st.today_dot(p),
                Some(p.primary.hsla()),
                "no meio do range o fundo é fraco: o ponto continua --primary"
            );

            let mut dis = estado(hoje);
            dis.today = true;
            dis.disabled = true;
            assert_eq!(
                dis.today_dot(p),
                Some(p.foreground.scaled(TODAY_DOT_DISABLED_ALPHA))
            );
        }
    }

    /// As entrelinhas são os PARES do Tailwind, e não a razão de ouro do GPUI — a armadilha nº 1 da
    /// casa. O teste guarda os dois pares E a diferença que eles evitam.
    #[test]
    fn entrelinhas_sao_os_pares_do_tailwind() {
        assert_eq!((TEXT_SIZE, TEXT_LINE_HEIGHT), (14.0, 20.0), "text-sm");
        assert_eq!((SMALL_TEXT, SMALL_LINE_HEIGHT), (12.0, 16.0), "text-xs");
        const RAZAO_DE_OURO: f32 = 1.618_034;
        assert!(TEXT_SIZE * RAZAO_DE_OURO > TEXT_LINE_HEIGHT + 2.0);
        assert!(SMALL_TEXT * RAZAO_DE_OURO > SMALL_LINE_HEIGHT + 3.0);
    }

    /// Os textos em português, e o rótulo do caption montado.
    #[test]
    fn rotulos_em_portugues() {
        assert_eq!(caption_label(d(2026, 8, 1)).as_ref(), "Agosto 2026");
        assert_eq!(caption_label(d(2026, 1, 15)).as_ref(), "Janeiro 2026");
        assert_eq!(MESES.len(), 12);
        assert_eq!(DIAS.len(), 7);
        // A ordem de `DIAS` é a de `Weekday::ALL` (ISO, segunda primeiro) — se as duas
        // dessincronizarem, o cabeçalho sai trocado, que é o bug clássico de calendário.
        assert_eq!(weekday_label(Weekday::Monday), "seg");
        assert_eq!(weekday_label(Weekday::Sunday), "dom");
        assert_eq!(weekday_label(Weekday::Saturday), "sáb");
    }

    /// **A largura da grade é `7 × CELL`**, e a coluna de número de semana acrescenta uma célula. É a
    /// conta que o call site precisa pra dimensionar um popover.
    #[test]
    fn a_largura_da_grade_e_sete_celulas() {
        assert_eq!(7.0 * CELL, 252.0);
        assert_eq!(8.0 * CELL, 288.0, "com a coluna de número de semana");
        // E o caption reserva uma célula de cada lado pros chevrons, então ele nunca os cobre.
        assert_eq!(2.0 * CAPTION_MARGIN_X, 2.0 * CELL);
        assert!(2.0 * CAPTION_MARGIN_X < 7.0 * CELL, "sobra espaço pro rótulo");
    }

    /// As paletas decodificam pras cores pretendidas — o teste que pega inversão de canais.
    #[test]
    fn paletas_decodificam_pras_cores_pretendidas() {
        use crate::color::opaque;
        assert_eq!(CALENDAR_LIGHT.foreground, opaque(0x262626), "neutral-800");
        assert_eq!(CALENDAR_DARK.foreground, opaque(0xf5f5f5), "neutral-100");
        assert_eq!(CALENDAR_LIGHT.primary, opaque(0x262626));
        assert_eq!(CALENDAR_LIGHT.primary_fg, opaque(0xfafafa), "neutral-50");
        assert_eq!(CALENDAR_DARK.primary, opaque(0xf5f5f5));
        assert_eq!(CALENDAR_LIGHT.ring, opaque(0xa3a3a3), "neutral-400");
        assert_eq!(CALENDAR_DARK.ring, opaque(0x737373), "neutral-500");
        assert_eq!(CALENDAR_DARK.background, opaque(0x141414));
        // Os dois `--accent` são 4% do preto/branco.
        assert!((CALENDAR_LIGHT.accent.alpha() - 0.04).abs() < 0.01);
        assert!((CALENDAR_DARK.accent.alpha() - 0.04).abs() < 0.01);
        assert_eq!(CALENDAR_LIGHT.accent.0 & 0xffffff00, 0x00000000, "preto");
        assert_eq!(CALENDAR_DARK.accent.0 & 0xffffff00, 0xffffff00, "branco");
    }

    /// O anel cresce o raio **por ponta**, e uma ponta quadrada continua quadrada — senão o anel de um
    /// dia no meio de um range sairia arredondado sobre um botão reto.
    #[test]
    fn o_anel_acompanha_o_raio_de_cada_ponta() {
        let crescer = |r: f32| if r > 0.0 { r + RING_WIDTH } else { 0.0 };
        assert_eq!(crescer(RADIUS), 13.0, "10 + 3");
        assert_eq!(crescer(0.0), 0.0, "ponta quadrada não ganha raio");
    }
}

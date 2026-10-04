//! `calendar_selection` — **quais dias estão escolhidos**, e para onde o teclado anda.
//!
//! É a metade sem pixel do [`crate::calendar`]: tipos e funções puras, sem GPUI. Mora num módulo
//! próprio porque as regras de seleção da referência são cheias de canto (um range de um dia carrega
//! as duas pontas; o primeiro clique já fecha um range; clicar de novo limpa, menos com `required`), e
//! canto se testa melhor sem janela no caminho.
//!
//! ```ignore
//! let mut s = Selection::new(CalendarMode::Range);
//! s.click(Date::new(2026, 8, 10).unwrap());   // vira {10, 10}: um range de UM dia
//! s.click(Date::new(2026, 8, 13).unwrap());   // estende: {10, 13}
//! assert!(s.range_roles(Date::new(2026, 8, 11).unwrap()).middle);
//! ```
//!
//! # De onde vêm as regras
//!
//! Do `useSingle`/`useMulti`/`useRange` e do `utils/addToRange.ts` do `react-day-picker` v10.0.1,
//! levantados na fonte (o registro está em `scratchpad/daypicker-spec.md`, §3 e §9). O `calendar.tsx`
//! do coss não decide nada disso — ele só pinta o resultado.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Resolvido em número**
//!
//! - A **ordem dos ramos** do modo `Range` é a do `addToRange`, e ela não é intuitiva: um clique num
//!   range já completo **ajusta a ponta mais próxima em vez de reiniciar** a seleção. Ver
//!   [`Selection::click`].
//!
//! **Superset consciente**
//!
//! - `RangeRoles` é um struct de três booleanos em vez de um enum, porque um range de **um dia** é
//!   `start` **e** `end` ao mesmo tempo. Um enum forçaria um quarto valor (`StartAndEnd`) e todo call
//!   site a lembrar dele; com booleanos, a regra de raio do coss
//!   (`in-[.range-start:not(.range-end)]:…`) se traduz sozinha.
//!
//! **Ausente**
//!
//! - `min` / `max` (mínimo e máximo de noites num range, e de dias no `Multiple`),
//!   `excludeDisabled` (resetar um range que engoliu um dia desabilitado) e `resetOnSelect` (clicar
//!   num range completo começar um novo em vez de ajustar a ponta). São props da referência que ainda
//!   não têm caso de uso nesta base; cada uma acrescenta um ramo ao [`Selection::click`], e acrescentar
//!   ramo sem chamador é a definição de código não exercitado. `required` está implementado porque é o
//!   que impede um formulário obrigatório ficar vazio.
//! - **Pular dias desabilitados** na navegação por teclado: [`move_focus`] devolve o dia geométrico, e
//!   quem sabe o que está desabilitado é o componente (é ele que tem o predicado). O salto é feito lá,
//!   com teto, como na referência.

use crate::date::{Date, Weekday};

// =================================================================================================
// Modo
// =================================================================================================

/// Quantos dias podem estar escolhidos, e como.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CalendarMode {
    /// **Um dia.** Clicar no dia já escolhido limpa a seleção (menos com `required`).
    #[default]
    Single,
    /// **Vários dias soltos.** Clicar num dia escolhido o remove.
    Multiple,
    /// **Um intervalo.** Ver [`Selection::click`] pro fluxo, que é o menos óbvio dos três.
    Range,
}

/// Os três papéis de um dia dentro de um range — as classes `range-start` / `range-middle` /
/// `range-end` da referência.
///
/// ⚠️ **`start` e `end` podem ser verdadeiros JUNTOS**: é o range de um dia. É por isso que isto é um
/// struct e não um enum, e é o que faz as regras de raio do coss (que usam `:not(...)`) traduzirem
/// direto.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct RangeRoles {
    pub start: bool,
    pub middle: bool,
    pub end: bool,
}

impl RangeRoles {
    /// Se este dia tem algum papel de range.
    pub fn any(self) -> bool {
        self.start || self.middle || self.end
    }
}

// =================================================================================================
// Seleção
// =================================================================================================

/// O que está escolhido, guardado na forma do modo.
///
/// Um enum interno (e não um `Vec<Date>` pra tudo) porque as três formas respondem perguntas
/// diferentes: o `Range` precisa de *ordem* nas pontas, e o `Multiple` precisa de *conjunto*. Um
/// `Vec` servindo os dois viraria uma sequência de convenções ("no modo Range o índice 0 é o from")
/// que nada no tipo garante.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Escolha {
    Single(Option<Date>),
    Multiple(Vec<Date>),
    /// `(from, to)`. `to` só é `None` numa seleção **semeada** com uma data só — o fluxo de cliques
    /// nunca produz esse estado (o primeiro clique já fecha um range de um dia).
    Range(Option<(Date, Option<Date>)>),
}

/// A seleção de um [`crate::calendar::Calendar`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    escolha: Escolha,
    required: bool,
}

impl Default for Selection {
    fn default() -> Self {
        Selection::new(CalendarMode::default())
    }
}

impl Selection {
    /// Uma seleção vazia no modo dado.
    pub fn new(mode: CalendarMode) -> Self {
        Selection {
            escolha: match mode {
                CalendarMode::Single => Escolha::Single(None),
                CalendarMode::Multiple => Escolha::Multiple(Vec::new()),
                CalendarMode::Range => Escolha::Range(None),
            },
            required: false,
        }
    }

    pub fn mode(&self) -> CalendarMode {
        match self.escolha {
            Escolha::Single(_) => CalendarMode::Single,
            Escolha::Multiple(_) => CalendarMode::Multiple,
            Escolha::Range(_) => CalendarMode::Range,
        }
    }

    /// Impede que um clique **limpe** a seleção — o `required` da referência.
    ///
    /// Não é o mesmo que "começa preenchida": uma seleção `required` vazia continua vazia até o
    /// primeiro clique. O que ela garante é que, depois do primeiro, não volta a ficar vazia.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Se ela é obrigatória — o [`crate::calendar::Calendar`] lê isto pra preservar a flag ao trocar de
    /// modo.
    pub fn is_required(&self) -> bool {
        self.required
    }

    /// Semeia a seleção.
    ///
    /// - `Single`: a **primeira** data da lista, se houver.
    /// - `Multiple`: todas, ordenadas e sem repetição.
    /// - `Range`: `[from]` (um range aberto — o único jeito de alcançar esse estado) ou `[from, to]`,
    ///   com as duas **ordenadas entre si**, porque um range invertido não tem meio.
    ///
    /// Lista vazia limpa. Datas além da segunda são ignoradas no `Range`.
    pub fn with_dates(mut self, dates: &[Date]) -> Self {
        self.escolha = match self.escolha {
            Escolha::Single(_) => Escolha::Single(dates.first().copied()),
            Escolha::Multiple(_) => {
                let mut v = dates.to_vec();
                v.sort_unstable();
                v.dedup();
                Escolha::Multiple(v)
            }
            Escolha::Range(_) => Escolha::Range(match dates {
                [] => None,
                [um] => Some((*um, None)),
                [a, b, ..] => Some((*a.min(b), Some(*a.max(b)))),
            }),
        };
        self
    }

    /// O `data-selected` da célula: nas pontas **e** no meio, no modo `Range`.
    pub fn is_selected(&self, date: Date) -> bool {
        match &self.escolha {
            Escolha::Single(sel) => *sel == Some(date),
            Escolha::Multiple(v) => v.contains(&date),
            Escolha::Range(None) => false,
            // Range aberto: só o `from` está selecionado.
            Escolha::Range(Some((from, None))) => *from == date,
            Escolha::Range(Some((from, Some(to)))) => *from <= date && date <= *to,
        }
    }

    /// Os papéis de range deste dia. Tudo falso fora do modo `Range`, e também num range **aberto** —
    /// é o que a referência faz: durante uma seleção em progresso não existe nenhuma classe `range-*`,
    /// só o `data-selected`.
    pub fn range_roles(&self, date: Date) -> RangeRoles {
        let Escolha::Range(Some((from, Some(to)))) = &self.escolha else {
            return RangeRoles::default();
        };
        RangeRoles {
            start: date == *from,
            // Estritamente entre as pontas — o `excludeEnds` do `rangeIncludesDate`.
            middle: date > *from && date < *to,
            end: date == *to,
        }
    }

    /// Aplica um clique. Devolve `true` se a seleção mudou.
    ///
    /// # O fluxo do `Range`, que é o menos óbvio
    ///
    /// 1. **Vazio** → `{from: d, to: d}`. Já é um range COMPLETO de um dia, e o dia recebe `start` e
    ///    `end` juntos. Não existe estado "só o from" no fluxo de cliques (na referência isso só
    ///    apareceria com `min > 0`, que não expomos).
    /// 2. **Completo, clicando em outro dia** → **ajusta a ponta**, nunca reinicia: antes do `from`, o
    ///    `from` anda; senão o `to` anda. É o comportamento do `addToRange`, e é o que surpreende quem
    ///    espera que o segundo par de cliques comece um range novo (isso é o `resetOnSelect`, que a
    ///    referência tem e nós não expomos).
    /// 3. **Clicando no único dia de um range de um dia** → limpa, menos com `required`.
    ///
    /// O `Single` e o `Multiple` seguem a mesma ideia: clicar no que já está escolhido desfaz, e
    /// `required` é o que impede a seleção ficar vazia.
    pub fn click(&mut self, date: Date) -> bool {
        let antes = self.escolha.clone();
        match &mut self.escolha {
            Escolha::Single(sel) => {
                *sel = if *sel == Some(date) && !self.required {
                    None
                } else {
                    Some(date)
                };
            }
            Escolha::Multiple(v) => match v.iter().position(|d| *d == date) {
                // O último não sai se a seleção é obrigatória.
                Some(_) if self.required && v.len() == 1 => {}
                Some(i) => {
                    v.remove(i);
                }
                None => {
                    v.push(date);
                    v.sort_unstable();
                }
            },
            Escolha::Range(range) => {
                *range = match *range {
                    // 1. Primeiro clique: range completo de um dia.
                    None => Some((date, Some(date))),
                    // 3. Clique no único dia de um range de um dia: limpa.
                    Some((from, Some(to))) if from == to && date == from => {
                        if self.required {
                            Some((from, Some(to)))
                        } else {
                            None
                        }
                    }
                    // 2. Ajusta a ponta.
                    Some((from, Some(to))) => {
                        if date < from {
                            Some((date, Some(to)))
                        } else {
                            Some((from, Some(date)))
                        }
                    }
                    // Range semeado aberto: o clique fecha, ordenando as pontas.
                    Some((from, None)) => Some((from.min(date), Some(from.max(date)))),
                };
            }
        }
        antes != self.escolha
    }

    /// As datas escolhidas, em ordem crescente. No `Range` são as duas pontas (uma só, se o range for
    /// de um dia ou estiver aberto).
    pub fn dates(&self) -> Vec<Date> {
        match &self.escolha {
            Escolha::Single(sel) => sel.iter().copied().collect(),
            Escolha::Multiple(v) => v.clone(),
            Escolha::Range(None) => Vec::new(),
            Escolha::Range(Some((from, None))) => vec![*from],
            Escolha::Range(Some((from, Some(to)))) if from == to => vec![*from],
            Escolha::Range(Some((from, Some(to)))) => vec![*from, *to],
        }
    }

    /// As pontas do range, ou `None` fora do modo `Range` (e com o range vazio).
    pub fn range(&self) -> Option<(Date, Option<Date>)> {
        match &self.escolha {
            Escolha::Range(r) => *r,
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.escolha {
            Escolha::Single(sel) => sel.is_none(),
            Escolha::Multiple(v) => v.is_empty(),
            Escolha::Range(r) => r.is_none(),
        }
    }
}

// =================================================================================================
// Foco
// =================================================================================================

/// Um movimento de foco do teclado — a tabela de teclas da referência, nomeada.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FocusMove {
    PrevDay,
    NextDay,
    PrevWeek,
    NextWeek,
    PrevMonth,
    NextMonth,
    PrevYear,
    NextYear,
    /// A primeira coluna da semana — respeita onde a semana começa.
    WeekStart,
    /// A última coluna da semana.
    WeekEnd,
}

/// Para onde o foco vai.
///
/// Geometria pura: não sabe de dia desabilitado nem de limite de navegação (ver "Ausente" no doc do
/// módulo). O salto de mês e de ano passa pelo [`Date::add_months`], que **aproxima** o dia pro fim do
/// mês quando ele não existe lá — 31 de março, um mês atrás, é 28 (ou 29) de fevereiro, e não um erro.
pub fn move_focus(from: Date, mv: FocusMove, week_start: Weekday) -> Date {
    match mv {
        FocusMove::PrevDay => from.add_days(-1),
        FocusMove::NextDay => from.add_days(1),
        FocusMove::PrevWeek => from.add_days(-7),
        FocusMove::NextWeek => from.add_days(7),
        FocusMove::PrevMonth => from.add_months(-1),
        FocusMove::NextMonth => from.add_months(1),
        FocusMove::PrevYear => from.add_months(-12),
        FocusMove::NextYear => from.add_months(12),
        FocusMove::WeekStart => inicio_da_semana(from, week_start),
        FocusMove::WeekEnd => inicio_da_semana(from, week_start).add_days(6),
    }
}

/// O primeiro dia da semana em que `date` cai, dado onde a semana começa.
///
/// É a mesma conta que posiciona o dia 1º na primeira linha da grade (ver `crate::date::month_grid`),
/// e ela vive aqui também porque o `Home`/`End` do teclado é a única outra coisa que precisa dela.
fn inicio_da_semana(date: Date, week_start: Weekday) -> Date {
    date.add_days(-(week_start.days_until(date.weekday()) as i64))
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar a regra que veio da referência É o propósito.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    fn d(dia: u32) -> Date {
        Date::new(2026, 8, dia).expect("agosto de 2026 tem esse dia")
    }

    /// `Single`: clicar escolhe, clicar de novo no MESMO dia limpa.
    #[test]
    fn single_alterna_no_mesmo_dia() {
        let mut s = Selection::new(CalendarMode::Single);
        assert!(s.is_empty());

        assert!(s.click(d(10)), "primeiro clique muda");
        assert!(s.is_selected(d(10)));
        assert_eq!(s.dates(), vec![d(10)]);

        assert!(s.click(d(12)), "outro dia troca");
        assert!(!s.is_selected(d(10)));
        assert!(s.is_selected(d(12)));

        assert!(s.click(d(12)), "o mesmo dia limpa");
        assert!(s.is_empty());
        assert_eq!(s.dates(), Vec::new());
    }

    /// `required` impede LIMPAR, e não impede trocar.
    #[test]
    fn required_impede_esvaziar_nos_tres_modos() {
        let mut s = Selection::new(CalendarMode::Single).required(true);
        s.click(d(10));
        assert!(!s.click(d(10)), "clicar de novo não muda nada");
        assert!(s.is_selected(d(10)), "continua escolhido");
        assert!(s.click(d(11)), "mas trocar de dia funciona");

        let mut m = Selection::new(CalendarMode::Multiple).required(true);
        m.click(d(10));
        assert!(!m.click(d(10)), "o último não sai");
        m.click(d(11));
        assert!(m.click(d(10)), "com dois, o primeiro sai");
        assert_eq!(m.dates(), vec![d(11)]);

        let mut r = Selection::new(CalendarMode::Range).required(true);
        r.click(d(10));
        assert!(!r.click(d(10)), "o range de um dia não se limpa");
        assert_eq!(r.dates(), vec![d(10)]);

        // E sem `required`, os três limpam.
        for modo in [CalendarMode::Single, CalendarMode::Multiple, CalendarMode::Range] {
            let mut s = Selection::new(modo);
            s.click(d(10));
            assert!(s.click(d(10)), "{modo:?}: devia limpar");
            assert!(s.is_empty(), "{modo:?}: devia estar vazio");
        }
    }

    /// `Multiple`: acumula, remove, e a lista sai ORDENADA (é o que o evento carrega).
    #[test]
    fn multiple_acumula_e_ordena() {
        let mut s = Selection::new(CalendarMode::Multiple);
        s.click(d(12));
        s.click(d(3));
        s.click(d(20));
        assert_eq!(s.dates(), vec![d(3), d(12), d(20)], "ordem crescente");
        assert!(s.is_selected(d(3)) && s.is_selected(d(12)) && s.is_selected(d(20)));

        assert!(s.click(d(12)), "clicar num escolhido remove");
        assert_eq!(s.dates(), vec![d(3), d(20)]);
        // E não há papel de range no modo múltiplo.
        assert!(!s.range_roles(d(3)).any());
    }

    /// **O primeiro clique num range já fecha um range de UM dia**, com `start` e `end` juntos. É o
    /// caso que as regras de raio do coss existem pra resolver.
    #[test]
    fn o_primeiro_clique_fecha_um_range_de_um_dia() {
        let mut s = Selection::new(CalendarMode::Range);
        assert!(s.click(d(10)));
        assert_eq!(s.range(), Some((d(10), Some(d(10)))));

        let papeis = s.range_roles(d(10));
        assert!(papeis.start && papeis.end, "as DUAS pontas no mesmo dia");
        assert!(!papeis.middle);
        assert!(s.is_selected(d(10)));
        assert_eq!(s.dates(), vec![d(10)], "um dia só, não dois iguais");
    }

    /// **Um clique num range completo AJUSTA a ponta — não reinicia.** É a ordem dos ramos do
    /// `addToRange`, e é o comportamento que surpreende quem espera um range novo.
    #[test]
    fn range_completo_ajusta_a_ponta_em_vez_de_reiniciar() {
        let mut s = Selection::new(CalendarMode::Range);
        s.click(d(10));
        s.click(d(13));
        assert_eq!(s.range(), Some((d(10), Some(d(13)))), "estendeu pra frente");

        // Um dia ANTES do início: o `from` anda, o `to` fica.
        s.click(d(5));
        assert_eq!(
            s.range(),
            Some((d(5), Some(d(13)))),
            "clicar antes do início move o INÍCIO, e NÃO começa um range novo em 5"
        );

        // Um dia depois: o `to` anda.
        s.click(d(20));
        assert_eq!(s.range(), Some((d(5), Some(d(20)))));

        // Um dia no MEIO: cai no ramo "senão", então o `to` anda pra trás.
        s.click(d(10));
        assert_eq!(s.range(), Some((d(5), Some(d(10)))));
    }

    /// Quem é ponta, quem é meio, e quem não é nada — inclusive as bordas exatas.
    #[test]
    fn os_papeis_de_um_range_de_varios_dias() {
        let s = Selection::new(CalendarMode::Range).with_dates(&[d(10), d(13)]);
        let p = |dia| s.range_roles(d(dia));

        assert!(p(10).start && !p(10).middle && !p(10).end, "10 é só início");
        assert!(p(13).end && !p(13).middle && !p(13).start, "13 é só fim");
        for dia in [11, 12] {
            let r = p(dia);
            assert!(r.middle && !r.start && !r.end, "{dia} é meio");
        }
        assert!(!p(9).any() && !p(14).any(), "fora do range não tem papel");

        // `is_selected` inclui as pontas — é o `excludeEnds: false` da referência.
        for dia in 10..=13 {
            assert!(s.is_selected(d(dia)), "{dia} está selecionado");
        }
        assert!(!s.is_selected(d(9)) && !s.is_selected(d(14)));
    }

    /// **Um range ABERTO (semeado com uma data só) não tem papel nenhum** — só `data-selected` no
    /// `from`. É o que a referência faz durante uma seleção em progresso.
    #[test]
    fn range_aberto_nao_tem_papel_de_range() {
        let s = Selection::new(CalendarMode::Range).with_dates(&[d(10)]);
        assert_eq!(s.range(), Some((d(10), None)));
        assert!(s.is_selected(d(10)));
        assert!(!s.range_roles(d(10)).any(), "nenhuma classe range-*");
        assert!(!s.is_selected(d(11)), "não há intervalo ainda");

        // E um clique FECHA o range, ordenando as pontas mesmo clicando pra trás.
        let mut s2 = s.clone();
        s2.click(d(5));
        assert_eq!(s2.range(), Some((d(5), Some(d(10)))), "as pontas saem ordenadas");
    }

    /// Semear **ordena as pontas**: um range invertido não tem meio, e aceitá-lo daria um calendário
    /// com uma seleção que nenhum clique alcança.
    #[test]
    fn semear_ordena_e_normaliza() {
        let s = Selection::new(CalendarMode::Range).with_dates(&[d(20), d(10)]);
        assert_eq!(s.range(), Some((d(10), Some(d(20)))));
        assert!(s.range_roles(d(15)).middle);

        // Datas além da segunda são ignoradas.
        let s3 = Selection::new(CalendarMode::Range).with_dates(&[d(10), d(12), d(30)]);
        assert_eq!(s3.range(), Some((d(10), Some(d(12)))));

        // No Multiple, semear tira repetição e ordena.
        let m = Selection::new(CalendarMode::Multiple).with_dates(&[d(12), d(3), d(12)]);
        assert_eq!(m.dates(), vec![d(3), d(12)]);

        // No Single, vale a primeira.
        let u = Selection::new(CalendarMode::Single).with_dates(&[d(7), d(9)]);
        assert_eq!(u.dates(), vec![d(7)]);

        // Lista vazia limpa, nos três.
        for modo in [CalendarMode::Single, CalendarMode::Multiple, CalendarMode::Range] {
            let s = Selection::new(modo).with_dates(&[d(5)]).with_dates(&[]);
            assert!(s.is_empty(), "{modo:?}");
        }
    }

    /// `click` só devolve `true` quando algo mudou de verdade — é o que decide se o componente emite
    /// evento, e um `true` gratuito viraria evento fantasma.
    #[test]
    fn click_devolve_se_mudou() {
        let mut s = Selection::new(CalendarMode::Range).with_dates(&[d(10), d(13)]);
        assert!(!s.click(d(13)), "clicar na ponta que já é o fim não muda nada");
        assert!(s.click(d(14)), "mover a ponta muda");

        let mut r = Selection::new(CalendarMode::Single).required(true);
        r.click(d(1));
        assert!(!r.click(d(1)), "required + mesmo dia = sem mudança");
    }

    /// O modo sobrevive às operações — e cada modo só responde o que é dele.
    #[test]
    fn o_modo_e_estavel_e_range_so_existe_no_modo_range() {
        for modo in [CalendarMode::Single, CalendarMode::Multiple, CalendarMode::Range] {
            let mut s = Selection::new(modo);
            s.click(d(10));
            assert_eq!(s.mode(), modo);
            assert_eq!(
                s.range().is_some(),
                modo == CalendarMode::Range,
                "{modo:?}: só o Range tem pontas"
            );
        }
        assert_eq!(Selection::default().mode(), CalendarMode::Single);
    }

    /// O teclado: dia, semana, mês e ano — e o mês **aproximando** o dia, que é o caso que um
    /// `add_days(30)` erraria.
    #[test]
    fn o_foco_anda_por_dia_semana_mes_e_ano() {
        let base = d(15);
        let m = |mv| move_focus(base, mv, Weekday::Sunday);
        assert_eq!(m(FocusMove::PrevDay), d(14));
        assert_eq!(m(FocusMove::NextDay), d(16));
        assert_eq!(m(FocusMove::PrevWeek), d(8));
        assert_eq!(m(FocusMove::NextWeek), d(22));
        assert_eq!(m(FocusMove::PrevMonth), Date::new(2026, 7, 15).unwrap());
        assert_eq!(m(FocusMove::NextMonth), Date::new(2026, 9, 15).unwrap());
        assert_eq!(m(FocusMove::PrevYear), Date::new(2025, 8, 15).unwrap());
        assert_eq!(m(FocusMove::NextYear), Date::new(2027, 8, 15).unwrap());

        // A aproximação de fim de mês: 31 de março, um mês atrás, é 28 de fevereiro (2026 não é
        // bissexto) — e não 3 de março, que um `add_days(-30)` daria.
        let trinta_e_um = Date::new(2026, 3, 31).unwrap();
        assert_eq!(
            move_focus(trinta_e_um, FocusMove::PrevMonth, Weekday::Sunday),
            Date::new(2026, 2, 28).unwrap()
        );
        assert_eq!(
            move_focus(Date::new(2024, 3, 31).unwrap(), FocusMove::PrevMonth, Weekday::Sunday),
            Date::new(2024, 2, 29).unwrap(),
            "em ano bissexto, 29"
        );
    }

    /// **`Home`/`End` respeitam onde a semana começa.** 12 de agosto de 2026 é uma quarta: contando de
    /// domingo, a semana dele vai de 9 a 15; contando de segunda, de 10 a 16.
    #[test]
    fn home_e_end_respeitam_o_inicio_da_semana() {
        let quarta = d(12);
        assert_eq!(quarta.weekday(), Weekday::Wednesday, "premissa do teste");

        assert_eq!(move_focus(quarta, FocusMove::WeekStart, Weekday::Sunday), d(9));
        assert_eq!(move_focus(quarta, FocusMove::WeekEnd, Weekday::Sunday), d(15));

        assert_eq!(move_focus(quarta, FocusMove::WeekStart, Weekday::Monday), d(10));
        assert_eq!(move_focus(quarta, FocusMove::WeekEnd, Weekday::Monday), d(16));

        // A ponta da semana é idempotente: pedir o início de novo não anda.
        for inicio in Weekday::ALL {
            let p = move_focus(quarta, FocusMove::WeekStart, inicio);
            assert_eq!(move_focus(p, FocusMove::WeekStart, inicio), p, "{inicio:?}");
            assert_eq!(p.weekday(), inicio, "{inicio:?}: cai na coluna de origem");
            // E o fim é sempre seis dias depois do início.
            assert_eq!(
                move_focus(quarta, FocusMove::WeekEnd, inicio),
                p.add_days(6)
            );
        }
    }
}

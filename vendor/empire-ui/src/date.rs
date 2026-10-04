//! `date` — a **data civil** de que o [`crate::calendar`] precisa, e nada além dela.
//!
//! # Por que não uma crate de data
//!
//! `chrono` e `time` resolvem fuso horário, horário de verão, parsing, formatação e aritmética de
//! instantes. Um calendário de seleção de dia não usa nada disso: ele precisa saber quantos dias tem
//! fevereiro, em que dia da semana cai o dia 1º, e como andar de mês. São três algoritmos conhecidos e
//! ~150 linhas.
//!
//! A régua é a mesma que esta lib já aplicou à cor: o [`crate::color_picker`] converte sRGB↔OKLCH à
//! mão em vez de puxar uma crate de cor. Dependência nova num crate de UI cobra caro (árvore de
//! build, superfície de API, versão a acompanhar) e aqui pagaria por um décimo do que traz.
//!
//! **O limite disso está declarado**: [`Date::today`] é a única função daqui que olha o mundo, e ela
//! devolve a data em **UTC**, não local — ver o doc dela. É a única coisa que uma crate de data faria
//! melhor, e o dia em que isso incomodar é o dia de reconsiderar este parágrafo.
//!
//! # O algoritmo
//!
//! Toda a aritmética passa por um **número de dias** desde uma época fixa, pelas fórmulas de
//! `days_from_civil`/`civil_from_days` de Howard Hinnant ([chrono-Compatible Low-Level Date
//! Algorithms][1]), que valem pro calendário gregoriano proléptico inteiro e não usam divisão com
//! sinal ambíguo. Com data ↔ inteiro, "dia seguinte", "dia da semana" e "quantos dias entre" viram
//! aritmética de inteiro, e o único lugar que ainda precisa de tabela é [`days_in_month`].
//!
//! [1]: https://howardhinnant.github.io/date_algorithms.html

use std::time::{SystemTime, UNIX_EPOCH};

// =================================================================================================
// Dia da semana
// =================================================================================================

/// Um dia da semana.
///
/// A ordem da declaração é a da **semana ISO** (segunda primeiro), e não a do domingo primeiro: o
/// número ISO é o que o cálculo de número de semana usa, e ter um só sentido de "índice" evita a
/// classe de bug em que uma parte do código conta de domingo e outra de segunda.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    /// Todos, na ordem ISO (segunda → domingo). É a lista ÚNICA: quem varre dias da semana usa esta.
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    /// O número ISO: segunda = 1 … domingo = 7.
    pub fn iso(self) -> u32 {
        self as u32 + 1
    }

    /// Do número ISO. Fora de `1..=7` devolve `None` — a alternativa (saturar) esconderia o erro de
    /// quem chamou.
    pub fn from_iso(n: u32) -> Option<Weekday> {
        Weekday::ALL.get(n.checked_sub(1)? as usize).copied()
    }

    /// Quantos dias andar **pra frente** de `self` até chegar em `outro` (0..=6).
    ///
    /// É o que posiciona o dia 1º dentro da primeira linha da grade: com `week_start` conhecido, o
    /// número de células vazias antes dele é exatamente `week_start.days_until(primeiro_dia)`.
    pub fn days_until(self, outro: Weekday) -> u32 {
        (7 + outro as u32 - self as u32) % 7
    }

    /// A semana começando neste dia, na ordem em que ela aparece na tela.
    ///
    /// É o cabeçalho da grade e a ordem das colunas — os dois saem da MESMA função, que é o que
    /// impede o cabeçalho de dessincronizar das células (o bug clássico de calendário: "seg" em cima
    /// de uma coluna de domingos).
    pub fn week_from(self) -> [Weekday; 7] {
        let mut out = [Weekday::Monday; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = Weekday::ALL[(self as usize + i) % 7];
        }
        out
    }
}

// =================================================================================================
// Data
// =================================================================================================

/// Uma data civil (ano, mês, dia), sem hora e sem fuso.
///
/// `Ord` é derivado e a ordem dos campos é ano → mês → dia **de propósito**: a comparação
/// lexicográfica de campos já é a ordem cronológica, então não há uma segunda implementação de
/// "antes" que possa divergir.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Date {
    year: i32,
    /// 1..=12.
    month: u32,
    /// 1..=`days_in_month`.
    day: u32,
}

impl Date {
    /// Uma data, se ela existir de verdade.
    ///
    /// `None` pra mês fora de `1..=12` e pra dia fora do mês — **incluindo 29 de fevereiro em ano
    /// comum**. Validar na construção é o que deixa todo o resto do módulo poder assumir que uma
    /// [`Date`] é válida, sem checar de novo.
    pub fn new(year: i32, month: u32, day: u32) -> Option<Date> {
        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return None;
        }
        Some(Date { year, month, day })
    }

    /// Aproxima uma data pro dia válido mais próximo do mês pedido — 31 de fevereiro vira 28 (ou 29).
    ///
    /// É o que a navegação de mês precisa: sair de 31 de janeiro e pedir "mês seguinte" tem que dar
    /// 28 de fevereiro, não `None`. É a mesma regra do `addMonths` do `date-fns`, que é o que a
    /// biblioteca da referência usa.
    pub fn clamped(year: i32, month: u32, day: u32) -> Date {
        let month = month.clamp(1, 12);
        Date {
            year,
            month,
            day: day.clamp(1, days_in_month(year, month)),
        }
    }

    pub fn year(self) -> i32 {
        self.year
    }

    pub fn month(self) -> u32 {
        self.month
    }

    pub fn day(self) -> u32 {
        self.day
    }

    /// O **dia de hoje, em UTC**.
    ///
    /// ⚠️ **Não é a data local.** O GPUI não expõe fuso horário e a std não sabe ler o do sistema, e
    /// converter à mão exigiria ler `/etc/localtime` e interpretar a base de dados de fusos — que é
    /// exatamente o trabalho que uma crate de data faz e este módulo escolheu não fazer (ver o doc do
    /// módulo).
    ///
    /// O que isso custa na prática: o pontinho de "hoje" e a data inicial podem ficar **um dia
    /// adiantados ou atrasados** por algumas horas, pra quem está longe de Greenwich. No Brasil
    /// (UTC−3) é entre 21h e a meia-noite; a leste de Greenwich, na madrugada. É o único ponto do
    /// componente que pode discordar do relógio do usuário, e é aqui que ele está declarado.
    ///
    /// Antes da época Unix (relógio do sistema absurdamente atrasado) devolve 1º de janeiro de 1970 —
    /// degrada, não entra em pânico.
    pub fn today() -> Date {
        let dias = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| (d.as_secs() / 86_400) as i64)
            .unwrap_or(0);
        Date::from_days(dias)
    }

    /// O dia da semana.
    pub fn weekday(self) -> Weekday {
        // 1º de janeiro de 1970 foi uma QUINTA (dia 4 na contagem ISO). O `rem_euclid` é o que faz a
        // conta valer pra datas anteriores à época, onde `%` daria resto negativo.
        let iso = (self.to_days() + 3).rem_euclid(7) as u32 + 1;
        Weekday::from_iso(iso).expect("rem_euclid(7) + 1 está em 1..=7")
    }

    /// Quantos dias desde 1º de janeiro de 1970 (negativo antes disso).
    ///
    /// É `days_from_civil` do Hinnant, com a época deslocada da dele (1º de março de 0000) pra a
    /// Unix. A troca de `março` como primeiro mês do "ano interno" é o truque que faz o dia extra do
    /// ano bissexto cair no FIM do ano interno, e é por isso que a fórmula não tem `if` de fevereiro.
    pub fn to_days(self) -> i64 {
        let y = self.year as i64 - i64::from(self.month <= 2);
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400; // 0..=399
        let m = self.month as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.day as i64 - 1; // 0..=365
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // 0..=146096
        era * 146_097 + doe - 719_468
    }

    /// A data que está a `dias` da época Unix — a inversa exata de [`Self::to_days`].
    pub fn from_days(dias: i64) -> Date {
        let z = dias + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097; // 0..=146096
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // 0..=399
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // 0..=365
        let mp = (5 * doy + 2) / 153; // 0..=11, com março = 0
        let d = doy - (153 * mp + 2) / 5 + 1; // 1..=31
        let m = if mp < 10 { mp + 3 } else { mp - 9 }; // 1..=12
        Date {
            year: (y + i64::from(m <= 2)) as i32,
            month: m as u32,
            day: d as u32,
        }
    }

    /// A data `n` dias adiante (ou atrás, com `n` negativo).
    pub fn add_days(self, n: i64) -> Date {
        Date::from_days(self.to_days() + n)
    }

    /// A data `n` meses adiante (ou atrás), com o dia **aproximado** pro fim do mês quando ele não
    /// existe lá — ver [`Self::clamped`].
    pub fn add_months(self, n: i32) -> Date {
        // Conta em meses absolutos desde o ano 0, pra o `rem_euclid` cuidar do sinal: sem isso,
        // recuar de janeiro cairia no mês 0 ou -1.
        let total = self.year as i64 * 12 + (self.month as i64 - 1) + n as i64;
        let year = total.div_euclid(12) as i32;
        let month = total.rem_euclid(12) as u32 + 1;
        Date::clamped(year, month, self.day)
    }

    /// O primeiro dia deste mês.
    pub fn first_of_month(self) -> Date {
        Date {
            day: 1,
            ..self
        }
    }

    /// O último dia deste mês.
    pub fn last_of_month(self) -> Date {
        Date {
            day: days_in_month(self.year, self.month),
            ..self
        }
    }

    /// Se as duas caem no mesmo ano E mês — a pergunta que decide se um dia é "de fora" na grade.
    pub fn same_month(self, outra: Date) -> bool {
        self.year == outra.year && self.month == outra.month
    }

    /// Quantos dias de `self` até `outra` (negativo se `outra` for antes).
    pub fn days_until(self, outra: Date) -> i64 {
        outra.to_days() - self.to_days()
    }

    /// O **número de semana ISO 8601** e o ano a que ele pertence.
    ///
    /// ⚠️ O ano devolvido **não é** necessariamente [`Self::year`]: 1º de janeiro de 2021 é a semana
    /// **53 de 2020**, e 31 de dezembro de 2019 é a semana **1 de 2020**. A regra ISO é que a semana
    /// pertence ao ano em que cai a sua QUINTA-FEIRA — e é por isso que a conta abaixo pula pra
    /// quinta antes de dividir por 7.
    pub fn iso_week(self) -> (i32, u32) {
        // A quinta-feira da semana desta data decide o ano ISO e o número.
        let quinta = self.add_days(4 - self.weekday().iso() as i64);
        let jan1 = Date {
            year: quinta.year,
            month: 1,
            day: 1,
        };
        let semana = (jan1.days_until(quinta) / 7 + 1) as u32;
        (quinta.year, semana)
    }
}

/// Se o ano é bissexto no calendário gregoriano.
///
/// A regra dos 400 anos não é decoração: sem ela, 1900 e 2100 saem com 29 de fevereiro. 2000 é o caso
/// que separa uma implementação correta de uma que só checa `% 4` e `% 100`.
pub fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Quantos dias tem o mês. Mês fora de `1..=12` devolve 0 — quem chama já validou (as duas portas de
/// entrada do módulo são [`Date::new`] e [`Date::clamped`]), e devolver 0 faz um erro futuro aparecer
/// como grade vazia em vez de índice fora de faixa.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

// =================================================================================================
// A grade do mês
// =================================================================================================

/// Uma célula da grade: a data e se ela é de **outro** mês.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub date: Date,
    /// `true` quando o dia pertence ao mês anterior ou ao seguinte — o `data-outside` da referência.
    pub outside: bool,
}

/// Uma linha da grade: sete células e o número de semana ISO.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Week {
    pub days: [Cell; 7],
    /// O par `(ano ISO, número)` de [`Date::iso_week`] da linha, tirado do **primeiro** dia dela.
    pub number: (i32, u32),
}

/// A grade de um mês: as semanas necessárias pra cobri-lo, na ordem da tela.
///
/// - `week_start` escolhe a coluna de origem (a referência usa o do locale; aqui é explícito).
/// - `fixed_weeks` força **seis** linhas sempre. Sem isso a grade tem 4, 5 ou 6 conforme o mês, e a
///   altura do calendário muda ao navegar — um mês de 28 dias começando na segunda ocupa 4 linhas, e
///   fevereiro de 2026 (28 dias, começa domingo) ocupa 5. Com seis, a caixa nunca pula.
///
/// As células fora do mês vêm SEMPRE preenchidas com a data real do mês vizinho, com `outside`
/// marcado. Quem não quer mostrá-las (o `showOutsideDays: false` da referência) esconde o texto —
/// não é a grade que muda, porque a POSIÇÃO das outras células não pode depender disso.
pub fn month_grid(year: i32, month: u32, week_start: Weekday, fixed_weeks: bool) -> Vec<Week> {
    let primeiro = Date::clamped(year, month, 1);
    // Quantas células antes do dia 1º: a distância, em dias, do início da semana até o dia da semana
    // em que ele cai.
    let antes = week_start.days_until(primeiro.weekday()) as i64;
    let inicio = primeiro.add_days(-antes);

    let dias_do_mes = days_in_month(year, month) as i64;
    let linhas = if fixed_weeks {
        6
    } else {
        // Teto da divisão: as células ocupadas (as de antes + as do mês) em blocos de sete.
        ((antes + dias_do_mes) as f64 / 7.0).ceil() as usize
    };

    (0..linhas)
        .map(|linha| {
            let mut days = [Cell {
                date: primeiro,
                outside: false,
            }; 7];
            for (col, slot) in days.iter_mut().enumerate() {
                let date = inicio.add_days(linha as i64 * 7 + col as i64);
                *slot = Cell {
                    date,
                    outside: !date.same_month(primeiro),
                };
            }
            Week {
                days,
                number: days[0].date.iso_week(),
            }
        })
        .collect()
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor de referência É o propósito. Mesma decisão
// em todos os módulos desta lib.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Atalho de teste: uma data que TEM que existir.
    fn d(y: i32, m: u32, dia: u32) -> Date {
        Date::new(y, m, dia).unwrap_or_else(|| panic!("{y}-{m}-{dia} deveria ser válida"))
    }

    /// **A regra dos 400 anos.** Uma implementação que só olha `% 4` erra 1900 e 2100; uma que olha
    /// `% 4` e `% 100` erra 2000. Os quatro casos abaixo separam as três implementações.
    #[test]
    fn ano_bissexto_segue_a_regra_dos_quatrocentos() {
        assert!(is_leap_year(2024), "divisível por 4");
        assert!(!is_leap_year(2023), "não divisível por 4");
        assert!(!is_leap_year(1900), "século NÃO divisível por 400");
        assert!(is_leap_year(2000), "século divisível por 400");
        assert!(!is_leap_year(2100));
        // E fevereiro acompanha.
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
    }

    /// Os tamanhos de mês, e o mês inválido devolvendo 0 em vez de estourar.
    #[test]
    fn tamanhos_de_mes() {
        let esperado = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for (i, dias) in esperado.iter().enumerate() {
            assert_eq!(days_in_month(2023, i as u32 + 1), *dias, "mês {}", i + 1);
        }
        assert_eq!(days_in_month(2023, 0), 0, "mês 0 não existe");
        assert_eq!(days_in_month(2023, 13), 0, "mês 13 não existe");
    }

    /// A construção REJEITA data que não existe — é o que deixa o resto do módulo assumir validade.
    #[test]
    fn construcao_rejeita_data_inexistente() {
        assert!(Date::new(2024, 2, 29).is_some(), "2024 é bissexto");
        assert!(Date::new(2023, 2, 29).is_none(), "2023 não é");
        assert!(Date::new(2023, 4, 31).is_none(), "abril tem 30");
        assert!(Date::new(2023, 13, 1).is_none());
        assert!(Date::new(2023, 0, 1).is_none());
        assert!(Date::new(2023, 1, 0).is_none());
    }

    /// **A ida e a volta são inversas exatas**, e não só em datas simpáticas: o teste varre 40 anos
    /// dia a dia. É o teste que pega erro de sinal nas divisões (o motivo de as fórmulas do Hinnant
    /// existirem) — um `%` no lugar de um `div_euclid` quebra antes de 1970 e passa depois.
    #[test]
    fn dias_e_data_sao_inversas_em_quarenta_anos() {
        let inicio = d(1960, 1, 1).to_days();
        let fim = d(2000, 1, 1).to_days();
        for n in inicio..=fim {
            let data = Date::from_days(n);
            assert_eq!(data.to_days(), n, "ida e volta em {data:?}");
            // E a data reconstruída é válida por construção.
            assert!(Date::new(data.year(), data.month(), data.day()).is_some());
        }
        // A época em si, e a fronteira de era do algoritmo (1º de março de 2000 é o "ano 0" dele).
        assert_eq!(d(1970, 1, 1).to_days(), 0);
        assert_eq!(Date::from_days(0), d(1970, 1, 1));
        assert_eq!(d(1969, 12, 31).to_days(), -1, "antes da época é negativo");
    }

    /// **Dias da semana travados contra o calendário real**, incluindo antes da época Unix — que é
    /// onde um `%` em vez de `rem_euclid` daria índice negativo.
    #[test]
    fn dia_da_semana_contra_o_calendario_real() {
        let casos = [
            (d(1970, 1, 1), Weekday::Thursday),  // a época
            (d(2000, 1, 1), Weekday::Saturday),  // virada do milênio
            (d(2024, 2, 29), Weekday::Thursday), // 29 de fevereiro
            (d(2026, 8, 4), Weekday::Tuesday),   // hoje, quando este módulo foi escrito
            (d(1900, 1, 1), Weekday::Monday),    // antes da época
            (d(1969, 7, 20), Weekday::Sunday),   // Apollo 11
        ];
        for (data, esperado) in casos {
            assert_eq!(data.weekday(), esperado, "{data:?}");
        }
    }

    /// A distância entre dias da semana, que é o que posiciona o dia 1º na primeira linha.
    #[test]
    fn distancia_entre_dias_da_semana() {
        assert_eq!(Weekday::Monday.days_until(Weekday::Monday), 0);
        assert_eq!(Weekday::Monday.days_until(Weekday::Sunday), 6);
        assert_eq!(Weekday::Sunday.days_until(Weekday::Monday), 1, "dá a volta");
        assert_eq!(Weekday::Saturday.days_until(Weekday::Friday), 6);
    }

    /// **O cabeçalho e as colunas saem da MESMA função** — é o que impede "seg" em cima de uma coluna
    /// de domingos.
    #[test]
    fn a_semana_comeca_onde_se_pede() {
        assert_eq!(Weekday::Monday.week_from()[0], Weekday::Monday);
        assert_eq!(Weekday::Monday.week_from()[6], Weekday::Sunday);
        assert_eq!(Weekday::Sunday.week_from()[0], Weekday::Sunday);
        assert_eq!(Weekday::Sunday.week_from()[1], Weekday::Monday);
        // E é sempre uma permutação dos sete, sem repetir.
        for inicio in Weekday::ALL {
            let mut semana = inicio.week_from();
            semana.sort_unstable();
            assert_eq!(semana, Weekday::ALL, "começando em {inicio:?}");
        }
    }

    /// Andar de mês **aproxima** o dia em vez de falhar — 31 de janeiro + 1 mês = 28/29 de fevereiro.
    /// É a regra do `date-fns`, que é o que a biblioteca da referência usa.
    #[test]
    fn andar_de_mes_aproxima_o_dia() {
        assert_eq!(d(2023, 1, 31).add_months(1), d(2023, 2, 28));
        assert_eq!(d(2024, 1, 31).add_months(1), d(2024, 2, 29), "bissexto");
        assert_eq!(d(2023, 3, 31).add_months(-1), d(2023, 2, 28));
        assert_eq!(d(2023, 5, 31).add_months(1), d(2023, 6, 30));
        // Sem aproximação necessária, é exato.
        assert_eq!(d(2023, 1, 15).add_months(1), d(2023, 2, 15));
        // E a virada de ano nos dois sentidos — onde um `%` daria mês 0 ou 13.
        assert_eq!(d(2023, 12, 15).add_months(1), d(2024, 1, 15));
        assert_eq!(d(2023, 1, 15).add_months(-1), d(2022, 12, 15));
        assert_eq!(d(2023, 6, 15).add_months(-18), d(2021, 12, 15));
        assert_eq!(d(2023, 6, 15).add_months(30), d(2025, 12, 15));
    }

    /// **A aproximação não é perda de informação silenciosa** quando não precisa ser: andar e voltar
    /// preserva a data quando o dia existe nos dois meses, e NÃO preserva quando não existe. O
    /// segundo caso é o que quem chama tem que saber — se um dia isso incomodar, é este teste que
    /// documenta o comportamento atual.
    #[test]
    fn andar_e_voltar_um_mes() {
        assert_eq!(d(2023, 1, 15).add_months(1).add_months(-1), d(2023, 1, 15));
        assert_eq!(
            d(2023, 1, 31).add_months(1).add_months(-1),
            d(2023, 1, 28),
            "31 de janeiro vira 28 de fevereiro e volta como 28 de janeiro — a ida aproximou"
        );
    }

    /// **A semana ISO pertence ao ano da QUINTA-FEIRA dela**, e é por isso que o ano devolvido pode
    /// não ser o ano da data. Os casos abaixo são as duas fronteiras clássicas.
    #[test]
    fn semana_iso_segue_a_quinta_feira() {
        // 1º de janeiro de 2021 foi uma sexta: a quinta daquela semana foi 31/12/2020 → semana 53/2020.
        assert_eq!(d(2021, 1, 1).iso_week(), (2020, 53));
        // 31 de dezembro de 2019 foi uma terça: a quinta foi 2/1/2020 → semana 1/2020.
        assert_eq!(d(2019, 12, 31).iso_week(), (2020, 1));
        // 4 de janeiro está SEMPRE na semana 1 (é a definição operacional da ISO).
        for ano in 2015..=2030 {
            assert_eq!(d(ano, 1, 4).iso_week(), (ano, 1), "4 de janeiro de {ano}");
        }
        // Um ano de 53 semanas (2020 começou numa quarta e é bissexto).
        assert_eq!(d(2020, 12, 31).iso_week(), (2020, 53));
    }

    /// O primeiro e o último dia do mês.
    #[test]
    fn pontas_do_mes() {
        assert_eq!(d(2024, 2, 15).first_of_month(), d(2024, 2, 1));
        assert_eq!(d(2024, 2, 15).last_of_month(), d(2024, 2, 29));
        assert_eq!(d(2023, 2, 15).last_of_month(), d(2023, 2, 28));
        assert_eq!(d(2023, 4, 1).last_of_month(), d(2023, 4, 30));
    }

    /// **A grade é retangular e contínua**: toda linha tem sete dias, e cada dia é o seguinte do
    /// anterior sem buraco nem repetição. É a invariante que sustenta a navegação por teclado.
    #[test]
    fn a_grade_e_continua_e_de_sete_em_sete() {
        for (ano, mes) in [(2024, 2), (2023, 2), (2026, 8), (2023, 12), (2024, 1)] {
            for inicio in [Weekday::Monday, Weekday::Sunday] {
                let grade = month_grid(ano, mes, inicio, false);
                let plana: Vec<Cell> = grade.iter().flat_map(|w| w.days).collect();
                assert_eq!(plana.len() % 7, 0);
                for par in plana.windows(2) {
                    assert_eq!(
                        par[0].date.add_days(1),
                        par[1].date,
                        "{ano}-{mes}, começando em {inicio:?}: buraco na grade"
                    );
                }
                // A primeira célula cai na coluna de origem, sempre.
                assert_eq!(plana[0].date.weekday(), inicio);
            }
        }
    }

    /// A grade **cobre o mês inteiro** e nada mais: todo dia do mês aparece exatamente uma vez, e as
    /// bordas são dias de fora marcados.
    #[test]
    fn a_grade_cobre_o_mes_e_marca_o_que_e_de_fora() {
        let grade = month_grid(2024, 2, Weekday::Monday, false);
        let plana: Vec<Cell> = grade.iter().flat_map(|w| w.days).collect();
        for dia in 1..=29 {
            let alvo = d(2024, 2, dia);
            let n = plana.iter().filter(|c| c.date == alvo && !c.outside).count();
            assert_eq!(n, 1, "{alvo:?} aparece {n}× como dia do mês");
        }
        // Fevereiro de 2024 começou numa quinta: 3 células de janeiro antes.
        let de_fora_no_inicio = plana.iter().take_while(|c| c.outside).count();
        assert_eq!(de_fora_no_inicio, 3, "29/30/31 de janeiro");
        for c in plana.iter().filter(|c| c.outside) {
            assert!(!c.date.same_month(d(2024, 2, 1)), "{c:?} está marcado errado");
        }
    }

    /// **O número de linhas depende do mês** — e é por isso que `fixed_weeks` existe: sem ele a
    /// altura do calendário PULA ao navegar.
    #[test]
    fn linhas_variam_com_o_mes_e_fixed_weeks_trava_em_seis() {
        // Fevereiro de 2021 tem 28 dias e começou numa segunda: cabe em 4 linhas exatas.
        assert_eq!(month_grid(2021, 2, Weekday::Monday, false).len(), 4);
        // O mesmo mês, contando de domingo, precisa de 5.
        assert_eq!(month_grid(2021, 2, Weekday::Sunday, false).len(), 5);
        // Maio de 2021 (31 dias, começou num sábado) estoura em 6.
        assert_eq!(month_grid(2021, 5, Weekday::Monday, false).len(), 6);
        // Com `fixed_weeks`, sempre 6 — é o que trava a altura.
        for (ano, mes) in [(2021, 2), (2021, 5), (2024, 2), (2026, 8)] {
            assert_eq!(
                month_grid(ano, mes, Weekday::Monday, true).len(),
                6,
                "{ano}-{mes} com fixed_weeks"
            );
        }
    }

    /// O número de semana da linha sai do PRIMEIRO dia dela, que é o que a coluna de números mostra.
    #[test]
    fn numero_de_semana_da_linha() {
        // Janeiro de 2021 contado de segunda: a primeira linha começa em 28/12/2020, semana 53/2020.
        let grade = month_grid(2021, 1, Weekday::Monday, false);
        assert_eq!(grade[0].number, (2020, 53));
        assert_eq!(grade[0].days[0].date, d(2020, 12, 28));
        assert_eq!(grade[1].number, (2021, 1));
    }

    /// `today` devolve uma data válida e plausível — o que dá pra afirmar sem congelar o relógio. O
    /// desvio de fuso está declarado no doc, não testado aqui (testar exigiria injetar o relógio, e o
    /// valor disso não paga a indireção num componente de UI).
    #[test]
    fn hoje_e_uma_data_valida_e_plausivel() {
        let hoje = Date::today();
        assert!(Date::new(hoje.year(), hoje.month(), hoje.day()).is_some());
        assert!(
            hoje.year() >= 2024 && hoje.year() < 2100,
            "o relógio do sistema devolveu {hoje:?}"
        );
        // E a ida e volta pela contagem de dias fecha, como em qualquer outra data.
        assert_eq!(Date::from_days(hoje.to_days()), hoje);
    }

    /// **O calendário proléptico, antes do ano 1.** O doc do módulo afirma que as fórmulas valem pro
    /// gregoriano proléptico inteiro, e é este teste que cobra a afirmação.
    ///
    /// Ele existe porque uma mutação sobreviveu: trocar o `div_euclid(12)` de [`Date::add_months`] por
    /// `/ 12` passava em todos os outros testes, já que os dois só divergem quando o total de meses é
    /// NEGATIVO — ou seja, em ano ≤ 0. Alcançável na prática: um `add_months(-100_000)` chega lá.
    /// Sem este teste, a promessa do doc não estava travada em lugar nenhum.
    #[test]
    fn a_aritmetica_vale_antes_do_ano_um() {
        // Ano 0 existe no proléptico (é 1 a.C. na contagem histórica) e é bissexto pela regra dos 400.
        assert!(is_leap_year(0));
        assert_eq!(days_in_month(0, 2), 29);

        // Andar de mês atravessando o ano 0 nos dois sentidos: é aqui que `/` daria o ano errado.
        assert_eq!(d(1, 1, 15).add_months(-1), d(0, 12, 15));
        assert_eq!(d(0, 1, 15).add_months(-1), d(-1, 12, 15));
        assert_eq!(d(-1, 12, 15).add_months(1), d(0, 1, 15));
        assert_eq!(d(0, 1, 15).add_months(-24), d(-2, 1, 15));
        assert_eq!(d(-5, 6, 30).add_months(7), d(-4, 1, 30));

        // E a ida e volta pela contagem de dias continua exata lá atrás.
        for data in [d(0, 1, 1), d(0, 12, 31), d(-1, 6, 15), d(-753, 4, 21)] {
            assert_eq!(Date::from_days(data.to_days()), data, "{data:?}");
            assert!(data.to_days() < 0, "{data:?} é antes da época");
        }
    }

    /// A ordem derivada É a ordem cronológica — é o que dispensa uma segunda implementação de
    /// "antes", que poderia divergir.
    #[test]
    fn a_ordem_derivada_e_cronologica() {
        let mut datas = [d(2024, 1, 2), d(2023, 12, 31), d(2024, 1, 1), d(2023, 1, 5)];
        datas.sort_unstable();
        assert_eq!(
            datas,
            [d(2023, 1, 5), d(2023, 12, 31), d(2024, 1, 1), d(2024, 1, 2)]
        );
        // E ela concorda com a contagem de dias, que é a outra forma de comparar.
        for par in datas.windows(2) {
            assert!(par[0].to_days() < par[1].to_days());
            assert!(par[0].days_until(par[1]) > 0);
        }
    }
}

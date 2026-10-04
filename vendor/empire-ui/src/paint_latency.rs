//! `paint_latency` — métrica **real** de latência de preview (M6).
//!
//! ## O que mede (e por que importa)
//! O transporte do preview vive dentro de `EditorShell::render` e o frame do playhead `P` só
//! pinta ~2 ciclos de repaint depois da régua mostrar `P` (push request/response quantizado pelo
//! repaint — ver `Research/findings/07-performance/fable-architecture-review.md` §1.1). O harness
//! de `preview_latency` mede só `request → frame pronto`; ele **para antes** dos saltos
//! slot→poll→pump→paint, exatamente onde mora o atraso do play.
//!
//! Esta métrica fecha o loop: no **instante do paint**, quando um `RenderImage` vai à tela,
//! mede-se
//!
//! ```text
//! lag = playhead_agora(segundos) − tempo_alvo_do_frame_pintado(segundos)
//! ```
//!
//! onde o tempo-alvo viajou pelo pipeline carimbado no `RenderJob`/`RenderResult`
//! (`render_service.rs`): para um frame de vídeo é o `pts_secs` (tempo local do clip do frame
//! efetivamente decodado); para os demais jobs, o playhead-alvo daquele job. É a única régua que
//! enxerga o atraso composto playhead→pixel — e a que vai provar o ganho da inversão pull (#2).
//!
//! ## Custo
//! **Zero quando desligado.** A coleta só roda se `PVE_LATENCY=1` (a meter é `None` caso
//! contrário). O `target_secs` viaja pelo pipeline sempre (um `f64`, custo desprezível), mas nada
//! é gravado/formatado/logado sem o env. Janela deslizante de tamanho fixo (sem alocação por
//! amostra), percentis por cópia+sort só na hora de logar (a cada `LOG_EVERY` paints).

use std::time::Instant;

/// A cada quantos paints registrados a métrica imprime um resumo (p50/p95/máx/média).
const LOG_EVERY: u64 = 120;
/// Tamanho da janela deslizante de amostras (≈ os últimos N paints). 120 ≈ ~2 s @60 Hz de paint.
const WINDOW: usize = 120;

/// Acumulador de latência playhead→pixel sobre uma janela deslizante. Criado **somente** quando
/// `PVE_LATENCY=1`; senão a métrica é desligada (`None`) e tem custo zero.
pub struct PaintLatencyMeter {
    /// Janela circular de amostras de lag (segundos). Pode ser negativa (preview à frente do
    /// playhead, raro — lookahead/jitter), por isso `f64` com sinal.
    samples: Vec<f64>,
    /// Próximo índice a sobrescrever (ring).
    next: usize,
    /// Quantas amostras já entraram (satura em `WINDOW`).
    filled: usize,
    /// Total de paints medidos (dispara o log a cada `LOG_EVERY`).
    count: u64,
    /// Início do processo (pra um timestamp relativo legível no log).
    start: Instant,
}

impl PaintLatencyMeter {
    /// Liga a métrica **apenas** se `PVE_LATENCY=1` (ou `=true`). Caso contrário devolve `None` —
    /// o chamador guarda `Option<PaintLatencyMeter>` e nunca paga nada quando desligado.
    pub fn from_env() -> Option<Self> {
        let on = std::env::var("PVE_LATENCY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        on.then(Self::new)
    }

    fn new() -> Self {
        eprintln!(
            "[PVE_LATENCY] métrica de latência playhead→pixel LIGADA (loga p50/p95 a cada \
             {LOG_EVERY} paints; janela de {WINDOW})"
        );
        Self {
            samples: Vec::with_capacity(WINDOW),
            next: 0,
            filled: 0,
            count: 0,
            start: Instant::now(),
        }
    }

    /// Registra **um** paint: o atraso = `playhead_now_secs − target_secs` (segundos). Ignora
    /// amostras com `target_secs` não-finito (ex.: blank/sem frame — não há "frame pintado" cujo
    /// atraso medir). Devolve `Some(lag_ms)` da amostra aceita (pro overlay), `None` se ignorada.
    pub fn record(&mut self, playhead_now_secs: f64, target_secs: f64) -> Option<f64> {
        if !target_secs.is_finite() || !playhead_now_secs.is_finite() {
            return None;
        }
        let lag = playhead_now_secs - target_secs;
        if self.filled < WINDOW {
            self.samples.push(lag);
            self.filled += 1;
        } else {
            self.samples[self.next] = lag;
        }
        self.next = (self.next + 1) % WINDOW;
        self.count += 1;
        if self.count % LOG_EVERY == 0 {
            self.log();
        }
        Some(lag * 1000.0)
    }

    /// Resumo da janela atual (em milissegundos): `(p50, p95, max, mean)`. `None` se vazia.
    pub fn summary_ms(&self) -> Option<LatencySummary> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted: Vec<f64> = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        let pct = |p: f64| -> f64 {
            // Percentil "nearest-rank" simples (índice = ceil(p*n)-1), barato e suficiente p/ diag.
            let idx = ((p * n as f64).ceil() as usize).saturating_sub(1).min(n - 1);
            sorted[idx]
        };
        let sum: f64 = sorted.iter().sum();
        Some(LatencySummary {
            p50_ms: pct(0.50) * 1000.0,
            p95_ms: pct(0.95) * 1000.0,
            max_ms: sorted[n - 1] * 1000.0,
            mean_ms: (sum / n as f64) * 1000.0,
            window: n,
        })
    }

    fn log(&self) {
        if let Some(s) = self.summary_ms() {
            eprintln!(
                "[PVE_LATENCY] t+{:.1}s · playhead→pixel lag (n={}): p50 {:.1}ms · p95 {:.1}ms · \
                 max {:.1}ms · média {:.1}ms",
                self.start.elapsed().as_secs_f64(),
                s.window,
                s.p50_ms,
                s.p95_ms,
                s.max_ms,
                s.mean_ms,
            );
        }
    }
}

/// Resumo de latência da janela, em milissegundos.
#[derive(Debug, Clone, Copy)]
pub struct LatencySummary {
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub window: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meter() -> PaintLatencyMeter {
        // Constrói direto (sem env) pra testar a lógica do histograma.
        PaintLatencyMeter {
            samples: Vec::with_capacity(WINDOW),
            next: 0,
            filled: 0,
            count: 0,
            start: Instant::now(),
        }
    }

    #[test]
    fn lag_is_playhead_minus_target() {
        let mut m = meter();
        // playhead em 1.000s, frame pintado é de 0.967s (1 frame @30fps atrás) → ~33ms de atraso.
        let lag_ms = m.record(1.000, 0.967).unwrap();
        assert!((lag_ms - 33.0).abs() < 0.5, "lag ≈ 33ms, foi {lag_ms}");
    }

    #[test]
    fn non_finite_target_is_ignored() {
        let mut m = meter();
        assert!(m.record(1.0, f64::NAN).is_none());
        assert!(m.summary_ms().is_none(), "amostra NaN não entra na janela");
    }

    #[test]
    fn percentiles_over_window() {
        let mut m = meter();
        // 100 amostras: lag de 0..99 ms (target = playhead - i/1000).
        for i in 0..100 {
            let ms = i as f64;
            m.record(ms / 1000.0, 0.0);
        }
        let s = m.summary_ms().unwrap();
        assert_eq!(s.window, 100);
        // p50 nearest-rank (ceil(0.5*100)-1 = 49) → 49 ms; p95 → idx 94 → 94 ms; max 99 ms.
        assert!((s.p50_ms - 49.0).abs() < 0.001, "p50 {}", s.p50_ms);
        assert!((s.p95_ms - 94.0).abs() < 0.001, "p95 {}", s.p95_ms);
        assert!((s.max_ms - 99.0).abs() < 0.001, "max {}", s.max_ms);
    }

    #[test]
    fn window_slides_and_caps() {
        let mut m = meter();
        // Mais amostras que a janela: as antigas saem; a janela fica em WINDOW.
        for i in 0..(WINDOW + 50) {
            m.record(i as f64 / 1000.0, 0.0);
        }
        let s = m.summary_ms().unwrap();
        assert_eq!(s.window, WINDOW, "janela satura em WINDOW");
        // As últimas WINDOW amostras vão de 50..(WINDOW+50) ms → max = WINDOW+49.
        assert!(
            (s.max_ms - (WINDOW as f64 + 49.0)).abs() < 0.001,
            "max reflete só as amostras recentes: {}",
            s.max_ms
        );
    }
}

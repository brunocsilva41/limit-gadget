//! Histórico de uso, previsão de esgotamento, orçamento diário e avisos de limite.

use crate::providers::{parse_iso_secs, Provider};
use serde_json::Value;
use std::collections::HashMap;

const SAMPLE_EVERY_MS: u64 = 5 * 60_000;
const KEEP_MS: u64 = 7 * 24 * 3600 * 1000;
const FORECAST_WINDOW_MS: u64 = 6 * 3600 * 1000;

/// Limites de janela longa (semana ou mês), onde faz sentido falar em "gasto de hoje".
fn is_long_window(label: &str) -> bool {
    label.contains("Semana") || label.contains("Mês") || label.contains("dias")
}

#[derive(Default)]
pub struct Insights {
    series: HashMap<String, Vec<(u64, f64)>>,
    levels: HashMap<String, u8>,
    /// chave do limite -> dia (local) em que o aviso do orçamento diário já foi dado
    budget_day: HashMap<String, i64>,
}

fn path() -> Option<std::path::PathBuf> {
    let base = std::env::var("APPDATA").ok()?;
    let dir = std::path::PathBuf::from(base).join("LimitGadget");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("history.json"))
}

/// Parâmetros de uma rodada de registro.
pub struct Params {
    pub now: u64,
    pub warn: f64,
    pub crit: f64,
    /// Pontos percentuais por dia; 0 desliga o aviso.
    pub budget: f64,
    /// Diferença do horário local para UTC, em segundos.
    pub tz: i64,
}

impl Insights {
    pub fn load() -> Self {
        let series = path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|r| serde_json::from_str(&r).ok())
            .unwrap_or_default();
        Self {
            series,
            ..Default::default()
        }
    }

    pub fn persist(&self) {
        if let (Some(p), Ok(j)) = (path(), serde_json::to_string(&self.series)) {
            let _ = std::fs::write(p, j);
        }
    }

    pub fn export(&self) -> Value {
        serde_json::to_value(&self.series).unwrap_or(Value::Null)
    }

    pub fn import(&mut self, v: Value) -> bool {
        match serde_json::from_value(v) {
            Ok(s) => {
                self.series = s;
                self.persist();
                true
            }
            Err(_) => false,
        }
    }

    /// Maior uso dos últimos 7 dias de cada provedor (para o resumo semanal).
    pub fn peaks(&self, list: &[Provider]) -> Vec<String> {
        list.iter()
            .filter(|p| p.state == "ok")
            .filter_map(|p| {
                let m = p
                    .meters
                    .iter()
                    .filter(|m| !m.minor)
                    .max_by(|a, b| a.percent.total_cmp(&b.percent))?;
                let s = self.series.get(&format!("{}|{}", p.id, m.label))?;
                let peak = s.iter().map(|&(_, v)| v).fold(m.percent, f64::max);
                Some(format!("{} {:.0}%", p.name, peak))
            })
            .collect()
    }

    /// Registra as leituras do provedor, calcula previsão e gasto de hoje e acumula os avisos a enviar.
    pub fn record(&mut self, p: &mut Provider, prm: &Params, notes: &mut Vec<(String, String)>) {
        let now = prm.now;
        let mut primary: Option<(String, f64)> = None;
        let day = ((now / 1000) as i64 + prm.tz).div_euclid(86400);
        let day_start_ms = ((day * 86400 - prm.tz) * 1000).max(0) as u64;

        for m in p.meters.iter_mut() {
            let key = format!("{}|{}", p.id, m.label);
            let s = self.series.entry(key.clone()).or_default();

            // queda brusca = o limite renovou; recomeça a série
            if let Some(&(_, last)) = s.last() {
                if m.percent + 20.0 < last {
                    s.clear();
                }
            }
            let push = s
                .last()
                .is_none_or(|&(t, v)| now.saturating_sub(t) >= SAMPLE_EVERY_MS || (m.percent - v).abs() >= 1.0);
            if push {
                s.push((now, m.percent));
            }
            s.retain(|&(t, _)| now.saturating_sub(t) <= KEEP_MS);
            if s.len() > 3000 {
                let cut = s.len() - 3000;
                s.drain(..cut);
            }

            // previsão: ritmo das últimas horas
            let recent: Vec<&(u64, f64)> = s
                .iter()
                .filter(|(t, _)| now.saturating_sub(*t) <= FORECAST_WINDOW_MS)
                .collect();
            if recent.len() >= 2 {
                let (a, b) = (recent[0], recent[recent.len() - 1]);
                let dt = (b.0 - a.0) as f64 / 1000.0;
                if dt >= 600.0 && b.1 > a.1 {
                    let slope = (b.1 - a.1) / dt;
                    let left = (100.0 - m.percent) / slope;
                    let until_reset = m
                        .resets_at
                        .as_deref()
                        .and_then(parse_iso_secs)
                        .map(|r| r as f64 - now as f64 / 1000.0);
                    if left > 0.0 && until_reset.is_none_or(|u| left < u) {
                        m.eta_secs = Some(left as u64);
                    }
                }
            }

            // gasto de hoje (limites de semana ou mês): diferença para o último valor antes da meia-noite local
            if is_long_window(&m.label) {
                let base = s
                    .iter()
                    .rev()
                    .find(|(t, _)| *t < day_start_ms)
                    .or_else(|| s.iter().find(|(t, _)| *t >= day_start_ms));
                if let Some(&(_, b)) = base {
                    let today = (m.percent - b).max(0.0);
                    m.today = Some(today);
                    if prm.budget > 0.0 && !m.minor && today > prm.budget && self.budget_day.get(&key) != Some(&day) {
                        self.budget_day.insert(key.clone(), day);
                        notes.push((
                            format!("{}: orçamento do dia estourou", p.name),
                            format!("{} subiu {:.0} pontos hoje (meta: {:.0}).", m.label, today, prm.budget),
                        ));
                    }
                }
            }

            if !m.minor {
                if primary.as_ref().is_none_or(|(_, v)| m.percent > *v) {
                    primary = Some((key.clone(), m.percent));
                }
                self.level_check(&p.name, &m.label, &key, m.percent, prm, notes);
            }
        }

        // série do limite mais restritivo, reduzida para o gráfico
        p.history = primary
            .and_then(|(k, _)| self.series.get(&k))
            .map(|s| {
                let step = (s.len() / 60).max(1);
                s.iter().step_by(step).cloned().collect()
            })
            .unwrap_or_default();
    }

    fn level_check(
        &mut self,
        name: &str,
        label: &str,
        key: &str,
        pct: f64,
        prm: &Params,
        notes: &mut Vec<(String, String)>,
    ) {
        let level: u8 = if pct >= prm.crit {
            2
        } else if pct >= prm.warn {
            1
        } else {
            0
        };
        let Some(&prev) = self.levels.get(key) else {
            self.levels.insert(key.to_string(), level); // primeira leitura: não avisa
            return;
        };
        if level > prev {
            let title = if level == 2 {
                format!("{name}: limite quase no fim")
            } else {
                format!("{name}: atenção ao limite")
            };
            notes.push((title, format!("{label} em {pct:.0}%")));
            self.levels.insert(key.to_string(), level);
        } else if level < prev {
            if pct < prm.warn - 5.0 {
                notes.push((
                    format!("{name}: limite renovado"),
                    format!("{label} voltou para {pct:.0}%"),
                ));
                self.levels.insert(key.to_string(), level);
            } else if level == 1 && prev == 2 {
                self.levels.insert(key.to_string(), 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Meter;

    fn prov(percent: f64) -> Provider {
        Provider {
            id: "x".into(),
            name: "Teste".into(),
            state: "ok".into(),
            meters: vec![Meter {
                label: "Semana · 7 dias".into(),
                percent,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn prm(now: u64) -> Params {
        Params {
            now,
            warn: 70.0,
            crit: 90.0,
            budget: 0.0,
            tz: 0,
        }
    }

    #[test]
    fn primeira_leitura_nao_avisa() {
        let mut ins = Insights::default();
        let mut notes = vec![];
        ins.record(&mut prov(95.0), &prm(1_000_000), &mut notes);
        assert!(notes.is_empty());
    }

    #[test]
    fn avisa_ao_cruzar_atencao_e_critico() {
        let mut ins = Insights::default();
        let mut notes = vec![];
        let t = 1_000_000u64;
        ins.record(&mut prov(10.0), &prm(t), &mut notes);
        ins.record(&mut prov(75.0), &prm(t + 60_000), &mut notes);
        assert_eq!(notes.len(), 1);
        ins.record(&mut prov(92.0), &prm(t + 120_000), &mut notes);
        assert_eq!(notes.len(), 2);
        assert!(notes[1].0.contains("quase no fim"));
    }

    #[test]
    fn avisa_quando_o_limite_renova() {
        let mut ins = Insights::default();
        let mut notes = vec![];
        let t = 1_000_000u64;
        ins.record(&mut prov(80.0), &prm(t), &mut notes);
        ins.record(&mut prov(75.0), &prm(t + 60_000), &mut notes);
        ins.record(&mut prov(5.0), &prm(t + 120_000), &mut notes);
        assert!(notes.iter().any(|(t, _)| t.contains("renovado")));
    }

    #[test]
    fn previsao_so_com_ritmo_positivo() {
        let mut ins = Insights::default();
        let mut notes = vec![];
        let t = 10_000_000u64;
        ins.record(&mut prov(10.0), &prm(t), &mut notes);
        let mut p = prov(20.0);
        ins.record(&mut p, &prm(t + 3_600_000), &mut notes);
        // 10 pontos em 1 h: faltam 80 pontos, cerca de 8 h
        let eta = p.meters[0].eta_secs.expect("deveria prever");
        assert!((28_000..29_600).contains(&eta), "eta={eta}");
    }

    #[test]
    fn orcamento_diario_avisa_uma_vez_por_dia() {
        let mut ins = Insights::default();
        let mut notes = vec![];
        let mut a = prm(86_400_000 * 5);
        a.budget = 5.0;
        ins.record(&mut prov(10.0), &a, &mut notes);
        let mut b = prm(86_400_000 * 5 + 3_600_000);
        b.budget = 5.0;
        let mut p = prov(18.0);
        ins.record(&mut p, &b, &mut notes);
        assert_eq!(p.meters[0].today.map(|x| x.round() as i64), Some(8));
        assert_eq!(notes.iter().filter(|(t, _)| t.contains("orçamento")).count(), 1);
        let mut c = prm(86_400_000 * 5 + 7_200_000);
        c.budget = 5.0;
        ins.record(&mut prov(25.0), &c, &mut notes);
        assert_eq!(notes.iter().filter(|(t, _)| t.contains("orçamento")).count(), 1);
    }
}

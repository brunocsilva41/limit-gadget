//! Dados de exemplo (nada real) para capturas de tela e documentação.
//! Ativado com a variável de ambiente `LIMITGADGET_DEMO=1`; não faz nenhuma consulta de rede.

use crate::providers::{iso_from_epoch, Extra, Meter, ProjectUse, Provider, SessionUse};

fn at(now_secs: i64, offset_secs: i64) -> Option<String> {
    Some(iso_from_epoch(now_secs + offset_secs))
}

fn meter(label: &str, percent: f64, resets_in: i64, now_secs: i64) -> Meter {
    Meter {
        label: label.into(),
        percent,
        resets_at: at(now_secs, resets_in),
        ..Default::default()
    }
}

/// Série crescente de uso nos últimos dias, para o gráfico.
fn history(now_ms: u64, from: f64, to: f64) -> Vec<(u64, f64)> {
    (0..28)
        .map(|i| {
            let t = now_ms - (27 - i) * 6 * 3_600_000;
            let k = i as f64 / 27.0;
            (t, (from + (to - from) * k * k.sqrt()).round())
        })
        .collect()
}

fn session(
    title: &str,
    project: &str,
    ago_secs: u64,
    now_ms: u64,
    tokens: Option<u64>,
    turns: Option<u32>,
) -> SessionUse {
    SessionUse {
        title: title.into(),
        project: format!("C:\\Projetos\\{project}"),
        updated_ms: now_ms - ago_secs * 1000,
        tokens,
        turns,
    }
}

pub fn providers(now_ms: u64, enabled: &[String]) -> Vec<Provider> {
    let now = (now_ms / 1000) as i64;
    let base = |id: &str, name: &str, plan: &str| Provider {
        id: id.into(),
        name: name.into(),
        plan: Some(plan.into()),
        updated_ms: now_ms,
        state: "ok".into(),
        ..Default::default()
    };

    let mut claude = base("claude", "Claude", "max");
    let mut week = meter("Semana · 7 dias", 38.0, 4 * 86400 + 6 * 3600, now);
    week.today = Some(6.0);
    week.eta_secs = Some(2 * 86400 + 20 * 3600);
    let mut opus = meter("Opus · 7 dias", 21.0, 4 * 86400 + 6 * 3600, now);
    opus.minor = true;
    let mut sonnet = meter("Sonnet · 7 dias", 44.0, 4 * 86400 + 6 * 3600, now);
    sonnet.minor = true;
    claude.meters = vec![meter("Sessão · 5 h", 14.0, 3 * 3600 + 12 * 60, now), week, opus, sonnet];
    claude.extra = Some(Extra {
        percent: 12.0,
        used: 420.0,
        limit: 3500.0,
        currency: "BRL".into(),
    });
    claude.history = history(now_ms, 8.0, 38.0);
    claude.projects = vec![
        ProjectUse {
            name: "api-pagamentos".into(),
            tokens_7d: 4_850_000,
            tokens_today: 1_200_000,
        },
        ProjectUse {
            name: "app-mobile".into(),
            tokens_7d: 2_310_000,
            tokens_today: 640_000,
        },
        ProjectUse {
            name: "site-institucional".into(),
            tokens_7d: 1_120_000,
            tokens_today: 0,
        },
        ProjectUse {
            name: "documentacao".into(),
            tokens_7d: 480_000,
            tokens_today: 95_000,
        },
    ];

    let mut codex = base("codex", "ChatGPT", "plus");
    let mut codex_week = meter("Semana · 7 dias", 76.0, 2 * 86400 + 9 * 3600, now);
    codex_week.today = Some(9.0);
    codex.meters = vec![meter("Sessão · 5 h", 22.0, 2 * 3600 + 40 * 60, now), codex_week];
    codex.history = history(now_ms, 30.0, 76.0);
    codex.sessions = vec![
        session("api-pagamentos", "api-pagamentos", 3 * 3600, now_ms, None, Some(14)),
        session("app-mobile", "app-mobile", 26 * 3600, now_ms, None, Some(9)),
        session("landing-page", "landing-page", 50 * 3600, now_ms, None, Some(22)),
    ];

    let mut copilot = base("copilot", "Copilot", "pro");
    let mut premium = meter("Premium · mês", 94.0, 11 * 86400, now);
    premium.detail = Some("282 de 300 usados".into());
    copilot.meters = vec![premium];
    copilot.history = history(now_ms, 40.0, 94.0);
    copilot.sessions = vec![
        session(
            "Refatorar o módulo de cobrança",
            "api-pagamentos",
            2 * 3600,
            now_ms,
            Some(1_840_000),
            None,
        ),
        session(
            "Corrigir os testes do carrinho",
            "app-mobile",
            9 * 3600,
            now_ms,
            Some(920_000),
            None,
        ),
        session(
            "Explicar o fluxo de autenticação",
            "site-institucional",
            30 * 3600,
            now_ms,
            Some(310_000),
            None,
        ),
    ];

    let mut antigravity = base("antigravity", "Antigravity", "Pro");
    antigravity.meters = vec![
        meter("Gemini 3 Pro", 34.0, 3 * 3600, now),
        meter("Claude Sonnet", 61.0, 3 * 3600, now),
        meter("GPT-OSS", 12.0, 3 * 3600, now),
    ];

    [claude, codex, copilot, antigravity]
        .into_iter()
        .filter(|p| enabled.contains(&p.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respeita_os_provedores_ativos() {
        let todos: Vec<String> = ["claude", "codex", "copilot", "antigravity"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(providers(1_800_000_000_000, &todos).len(), 4);
        assert_eq!(providers(1_800_000_000_000, &todos[..2]).len(), 2);
    }

    #[test]
    fn porcentagens_ficam_entre_0_e_100() {
        let todos: Vec<String> = ["claude", "codex", "copilot", "antigravity"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        for p in providers(1_800_000_000_000, &todos) {
            assert!(p.meters.iter().all(|m| (0.0..=100.0).contains(&m.percent)), "{}", p.id);
            assert!(p.history.iter().all(|(_, v)| (0.0..=100.0).contains(v)), "{}", p.id);
        }
    }
}

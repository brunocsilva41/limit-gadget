//! Consulta de uso de cada provedor. Cada `fetch_*` devolve um `Provider` já pronto para a interface.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Meter {
    pub label: String,
    pub percent: f64,
    pub resets_at: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    /// Limites secundários (ex.: por modelo), exibidos só quando o usuário quer.
    #[serde(default)]
    pub minor: bool,
    /// Segundos até esgotar no ritmo atual (só quando vai esgotar antes do reset).
    #[serde(default)]
    pub eta_secs: Option<u64>,
    /// Pontos percentuais consumidos hoje (só limites de semana ou mês).
    #[serde(default)]
    pub today: Option<f64>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Extra {
    pub percent: f64,
    pub used: f64,
    pub limit: f64,
    pub currency: String,
}

/// Uso do Claude Code em um projeto (tokens somados dos logs locais).
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ProjectUse {
    pub name: String,
    pub tokens_7d: u64,
    pub tokens_today: u64,
}

/// Uma sessão recente de uma ferramenta (Codex ou Copilot).
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SessionUse {
    pub title: String,
    pub project: String,
    pub updated_ms: u64,
    pub tokens: Option<u64>,
    pub turns: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub plan: Option<String>,
    pub meters: Vec<Meter>,
    pub extra: Option<Extra>,
    pub updated_ms: u64,
    /// "ok" | "offline" (sem login ou app fechado) | "error" (falha ao consultar)
    pub state: String,
    pub error: Option<String>,
    /// Série (timestamp ms, %) do limite mais restritivo, para o gráfico.
    #[serde(default)]
    pub history: Vec<(u64, f64)>,
    /// Só para o Claude: projetos mais usados nos últimos 7 dias.
    #[serde(default)]
    pub projects: Vec<ProjectUse>,
    /// Só para Codex e Copilot: últimas sessões dos 7 dias.
    #[serde(default)]
    pub sessions: Vec<SessionUse>,
}

pub struct Fail {
    pub offline: bool,
    pub msg: String,
}

fn offline(msg: &str) -> Fail {
    Fail {
        offline: true,
        msg: msg.into(),
    }
}
fn error(msg: &str) -> Fail {
    Fail {
        offline: false,
        msg: msg.into(),
    }
}

#[derive(Default)]
pub struct Locals {
    pub gh: Option<String>,
    pub ag: Option<(u16, String)>,
    pub ag_next: Option<Instant>,
    pub ins: crate::insights::Insights,
    pub logs: crate::claude_logs::LogScanner,
    pub logs_at: Option<Instant>,
    pub projects: Vec<ProjectUse>,
    pub sess: crate::session_logs::SessionScanner,
    pub sess_at: Option<Instant>,
    pub codex_sessions: Vec<SessionUse>,
    pub copilot_sessions: Vec<SessionUse>,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn new_provider(id: &str, name: &str) -> Provider {
    Provider {
        id: id.into(),
        name: name.into(),
        updated_ms: now_ms(),
        state: "ok".into(),
        ..Default::default()
    }
}

fn home() -> Option<String> {
    std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok()
}

pub fn iso_from_epoch(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 {
        y += 1;
    }
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(windows)]
fn hidden(c: &mut std::process::Command) -> &mut std::process::Command {
    use std::os::windows::process::CommandExt;
    c.creation_flags(0x0800_0000)
}
#[cfg(not(windows))]
fn hidden(c: &mut std::process::Command) -> &mut std::process::Command {
    c
}

// ---------------------------------------------------------------- Claude

fn claude_meter(v: &Value, key: &str, label: &str, minor: bool) -> Option<Meter> {
    let o = v.get(key)?;
    if o.is_null() {
        return None;
    }
    Some(Meter {
        label: label.into(),
        percent: o.get("utilization")?.as_f64()?,
        resets_at: o.get("resets_at").and_then(|r| r.as_str()).map(String::from),
        detail: None,
        eta_secs: None,
        today: None,
        minor,
    })
}

pub async fn fetch_claude(client: &reqwest::Client) -> Result<Provider, Fail> {
    let home = home().ok_or_else(|| offline("Sem pasta de usuário"))?;
    let raw = std::fs::read_to_string(format!("{home}/.claude/.credentials.json"))
        .map_err(|_| offline("Faça login no Claude Code"))?;
    let j: Value = serde_json::from_str(&raw).map_err(|_| error("Credenciais ilegíveis"))?;
    let o = j
        .get("claudeAiOauth")
        .ok_or_else(|| offline("Faça login no Claude Code"))?;
    let token = o
        .get("accessToken")
        .and_then(|t| t.as_str())
        .ok_or_else(|| offline("Faça login no Claude Code"))?;
    let exp = o.get("expiresAt").and_then(|e| e.as_u64()).unwrap_or(u64::MAX);
    if exp < now_ms() {
        return Err(error("Sessão expirada. Abra o Claude Code para renovar"));
    }
    let plan = o
        .get("subscriptionType")
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();

    let resp = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", "claude-code/2.0")
        .send()
        .await
        .map_err(|_| error("Sem conexão"))?;
    match resp.status().as_u16() {
        401 => return Err(error("Sessão expirada. Abra o Claude Code para renovar")),
        429 => return Err(error("Muitas consultas. Tentando de novo")),
        c if !(200..300).contains(&c) => return Err(error(&format!("Erro {c}"))),
        _ => {}
    }
    let v: Value = resp.json().await.map_err(|_| error("Resposta inválida"))?;

    let mut p = new_provider("claude", "Claude");
    p.plan = if plan.is_empty() { None } else { Some(plan) };
    for (k, l, minor) in [
        ("five_hour", "Sessão · 5 h", false),
        ("seven_day", "Semana · 7 dias", false),
        ("seven_day_opus", "Opus · 7 dias", true),
        ("seven_day_sonnet", "Sonnet · 7 dias", true),
    ] {
        if let Some(m) = claude_meter(&v, k, l, minor) {
            p.meters.push(m);
        }
    }
    p.extra = v
        .get("extra_usage")
        .filter(|e| e.get("is_enabled").and_then(|b| b.as_bool()) == Some(true))
        .and_then(|e| {
            let d = 10f64.powi(e.get("decimal_places").and_then(|d| d.as_i64()).unwrap_or(2) as i32);
            Some(Extra {
                percent: e.get("utilization")?.as_f64()?,
                used: e.get("used_credits")?.as_f64()? / d,
                limit: e.get("monthly_limit")?.as_f64()? / d,
                currency: e.get("currency").and_then(|c| c.as_str()).unwrap_or("").into(),
            })
        });
    Ok(p)
}

// ---------------------------------------------------------------- ChatGPT / Codex

fn window_label(secs: i64) -> String {
    match secs {
        18000 => "Sessão · 5 h".into(),
        604800 => "Semana · 7 dias".into(),
        s if s >= 2_400_000 => "Mês · 30 dias".into(),
        s if s >= 86400 => format!("{} dias", s / 86400),
        s => format!("{} h", (s / 3600).max(1)),
    }
}

fn codex_windows(v: Option<&Value>, prefix: &str, minor: bool, out: &mut Vec<Meter>) {
    let Some(v) = v.filter(|v| !v.is_null()) else { return };
    for key in ["primary_window", "secondary_window"] {
        let Some(w) = v.get(key).filter(|w| !w.is_null()) else {
            continue;
        };
        let Some(pct) = w.get("used_percent").and_then(|p| p.as_f64()) else {
            continue;
        };
        let secs = w.get("limit_window_seconds").and_then(|s| s.as_i64()).unwrap_or(0);
        out.push(Meter {
            label: format!("{prefix}{}", window_label(secs)),
            percent: pct,
            resets_at: w.get("reset_at").and_then(|r| r.as_i64()).map(iso_from_epoch),
            detail: None,
            eta_secs: None,
            today: None,
            minor,
        });
    }
}

pub async fn fetch_codex(client: &reqwest::Client) -> Result<Provider, Fail> {
    let home = home().ok_or_else(|| offline("Sem pasta de usuário"))?;
    let raw = std::fs::read_to_string(format!("{home}/.codex/auth.json"))
        .map_err(|_| offline("Faça login no Codex (codex login)"))?;
    let j: Value = serde_json::from_str(&raw).map_err(|_| error("Credenciais ilegíveis"))?;
    let tokens = j
        .get("tokens")
        .filter(|t| !t.is_null())
        .ok_or_else(|| offline("Codex sem login ChatGPT"))?;
    let token = tokens
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| offline("Codex sem login ChatGPT"))?;

    let mut req = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .bearer_auth(token)
        .header("User-Agent", "codex_cli_rs/0.50.0");
    if let Some(acc) = tokens.get("account_id").and_then(|a| a.as_str()) {
        req = req.header("chatgpt-account-id", acc);
    }
    let resp = req.send().await.map_err(|_| error("Sem conexão"))?;
    match resp.status().as_u16() {
        401 | 403 => return Err(error("Sessão do ChatGPT expirou. Abra o Codex para renovar")),
        429 => return Err(error("Muitas consultas. Tentando de novo")),
        c if !(200..300).contains(&c) => return Err(error(&format!("Erro {c}"))),
        _ => {}
    }
    let v: Value = resp.json().await.map_err(|_| error("Resposta inválida"))?;

    let mut p = new_provider("codex", "ChatGPT");
    p.plan = v.get("plan_type").and_then(|s| s.as_str()).map(String::from);
    codex_windows(v.get("rate_limit"), "", false, &mut p.meters);
    codex_windows(v.get("code_review_rate_limit"), "Revisão · ", true, &mut p.meters);
    Ok(p)
}

// ---------------------------------------------------------------- GitHub Copilot

fn copilot_token_from_files() -> Option<String> {
    let mut dirs = vec![];
    if let Ok(l) = std::env::var("LOCALAPPDATA") {
        dirs.push(std::path::PathBuf::from(l).join("github-copilot"));
    }
    if let Some(h) = home() {
        dirs.push(std::path::PathBuf::from(h).join(".config").join("github-copilot"));
    }
    for d in dirs {
        for f in ["apps.json", "hosts.json"] {
            let Ok(raw) = std::fs::read_to_string(d.join(f)) else {
                continue;
            };
            let Ok(v) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            if let Some(obj) = v.as_object() {
                for entry in obj.values() {
                    if let Some(t) = entry.get("oauth_token").and_then(|t| t.as_str()) {
                        return Some(t.to_string());
                    }
                }
            }
        }
    }
    None
}

fn copilot_token_from_gh() -> Option<String> {
    let out = hidden(std::process::Command::new("gh").args(["auth", "token"]))
        .output()
        .ok()?;
    let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() && !t.is_empty() {
        Some(t)
    } else {
        None
    }
}

pub async fn fetch_copilot(client: &reqwest::Client, cache: &mut Option<String>) -> Result<Provider, Fail> {
    if cache.is_none() {
        *cache = tauri::async_runtime::spawn_blocking(|| copilot_token_from_files().or_else(copilot_token_from_gh))
            .await
            .ok()
            .flatten();
    }
    let Some(tok) = cache.clone() else {
        return Err(offline("Entre no GitHub (gh auth login) para ver o Copilot"));
    };
    let resp = client
        .get("https://api.github.com/copilot_internal/user")
        .header("Authorization", format!("token {tok}"))
        .header("User-Agent", "GitHubCopilotChat/0.26.7")
        .header("Editor-Version", "vscode/1.99.0")
        .send()
        .await
        .map_err(|_| error("Sem conexão"))?;
    match resp.status().as_u16() {
        401 | 403 => {
            *cache = None;
            return Err(error("GitHub recusou o login. Rode gh auth login"));
        }
        404 => return Err(offline("Esta conta do GitHub não tem Copilot")),
        c if !(200..300).contains(&c) => return Err(error(&format!("Erro {c}"))),
        _ => {}
    }
    let v: Value = resp.json().await.map_err(|_| error("Resposta inválida"))?;

    let mut p = new_provider("copilot", "Copilot");
    p.plan = v.get("copilot_plan").and_then(|s| s.as_str()).map(String::from);
    let reset = v.get("quota_reset_date_utc").and_then(|s| s.as_str()).map(String::from);
    if let Some(snaps) = v.get("quota_snapshots").and_then(|s| s.as_object()) {
        for (key, label, minor) in [
            ("premium_interactions", "Premium · mês", false),
            ("chat", "Chat · mês", true),
            ("completions", "Sugestões · mês", true),
        ] {
            let Some(s) = snaps.get(key) else { continue };
            let unlimited = s.get("unlimited").and_then(|u| u.as_bool()).unwrap_or(false);
            let ent = s.get("entitlement").and_then(|e| e.as_f64()).unwrap_or(0.0);
            if unlimited || ent <= 0.0 {
                continue;
            }
            let remaining_pct = s.get("percent_remaining").and_then(|r| r.as_f64()).unwrap_or(100.0);
            let used = s
                .get("credits_used")
                .and_then(|c| c.as_f64())
                .filter(|c| *c > 0.0)
                .unwrap_or(ent - s.get("quota_remaining").and_then(|r| r.as_f64()).unwrap_or(ent));
            p.meters.push(Meter {
                label: label.into(),
                percent: (100.0 - remaining_pct).clamp(0.0, 100.0),
                resets_at: reset.clone(),
                detail: Some(format!("{:.0} de {:.0} usados", used, ent)),
                eta_secs: None,
                today: None,
                minor,
            });
        }
    }
    Ok(p)
}

// ---------------------------------------------------------------- Antigravity (IDE local)

/// Procura o language server do Antigravity (token CSRF e portas locais). Só existe com a IDE aberta.
fn discover_antigravity() -> Vec<(String, Vec<u16>)> {
    let script = "Get-CimInstance Win32_Process | Where-Object { $_.Name -like 'language_server*' -and $_.CommandLine -like '*csrf_token*' } | ForEach-Object { \"$($_.ProcessId)`t$($_.CommandLine)\" }";
    let Ok(out) =
        hidden(std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", script]))
            .output()
    else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let net = hidden(std::process::Command::new("netstat").args(["-ano", "-p", "tcp"]))
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut found = vec![];
    for line in text.lines() {
        let Some((pid, cmd)) = line.split_once('\t') else {
            continue;
        };
        let mut token = None;
        let mut it = cmd.split_whitespace();
        while let Some(t) = it.next() {
            if t == "--csrf_token" {
                token = it.next().map(String::from);
            } else if let Some(rest) = t.strip_prefix("--csrf_token=") {
                token = Some(rest.to_string());
            }
        }
        let Some(token) = token else { continue };
        let pid = pid.trim();
        let mut ports: Vec<u16> = net
            .lines()
            .filter(|l| l.contains("LISTENING") && l.split_whitespace().last() == Some(pid))
            .filter_map(|l| l.split_whitespace().nth(1)?.rsplit(':').next()?.parse().ok())
            .collect();
        ports.sort_unstable();
        ports.dedup();
        found.push((token, ports));
    }
    found
}

async fn query_antigravity(client: &reqwest::Client, port: u16, token: &str) -> Option<Provider> {
    let body = json!({"metadata": {"ideName": "antigravity", "extensionName": "antigravity", "ideVersion": "unknown", "locale": "en"}});
    for scheme in ["https", "http"] {
        let url = format!("{scheme}://127.0.0.1:{port}/exa.language_server_pb.LanguageServerService/GetUserStatus");
        let Ok(resp) = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Connect-Protocol-Version", "1")
            .header("X-Codeium-Csrf-Token", token)
            .json(&body)
            .send()
            .await
        else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(v) = resp.json::<Value>().await else { continue };
        let Some(us) = v.get("userStatus") else { continue };

        let mut p = new_provider("antigravity", "Antigravity");
        p.plan = us
            .pointer("/planStatus/planInfo/planName")
            .or_else(|| us.pointer("/userTier/name"))
            .and_then(|s| s.as_str())
            .map(String::from);
        if let Some(models) = us
            .pointer("/cascadeModelConfigData/clientModelConfigs")
            .and_then(|m| m.as_array())
        {
            for m in models {
                let Some(q) = m.get("quotaInfo") else { continue };
                let label = m.get("label").and_then(|l| l.as_str()).unwrap_or("Modelo");
                let remaining = q.get("remainingFraction").and_then(|r| r.as_f64()).unwrap_or(1.0);
                p.meters.push(Meter {
                    label: label.into(),
                    percent: ((1.0 - remaining) * 100.0).clamp(0.0, 100.0),
                    resets_at: q.get("resetTime").and_then(|r| r.as_str()).map(String::from),
                    detail: None,
                    eta_secs: None,
                    today: None,
                    minor: false,
                });
            }
        }
        return Some(p);
    }
    None
}

pub async fn fetch_antigravity(l: &mut Locals) -> Result<Provider, Fail> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // o servidor local usa certificado autoassinado
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| error("Falha interna"))?;

    if let Some((port, tok)) = l.ag.clone() {
        if let Some(p) = query_antigravity(&client, port, &tok).await {
            return Ok(p);
        }
        l.ag = None;
    }
    if l.ag_next.is_some_and(|t| Instant::now() < t) {
        return Err(offline("Abra o Antigravity para ver o uso"));
    }
    let found = tauri::async_runtime::spawn_blocking(discover_antigravity)
        .await
        .unwrap_or_default();
    if found.is_empty() {
        l.ag_next = Some(Instant::now() + Duration::from_secs(60));
        return Err(offline("Abra o Antigravity para ver o uso"));
    }
    for (token, ports) in found {
        for port in ports {
            if let Some(p) = query_antigravity(&client, port, &token).await {
                l.ag = Some((port, token));
                return Ok(p);
            }
        }
    }
    l.ag_next = Some(Instant::now() + Duration::from_secs(30));
    Err(error("Antigravity aberto, mas sem resposta"))
}

/// Converte "2026-11-11T00:00:00.093909+00:00" ou "...Z" em segundos desde 1970 (UTC).
pub fn parse_iso_secs(s: &str) -> Option<i64> {
    if s.len() < 19 {
        return None;
    }
    let n = |a: usize, l: usize| -> Option<i64> { s.get(a..a + l)?.parse().ok() };
    let (y, mo, d, h, mi, se) = (n(0, 4)?, n(5, 2)?, n(8, 2)?, n(11, 2)?, n(14, 2)?, n(17, 2)?);
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let mut secs = days * 86400 + h * 3600 + mi * 60 + se;
    if let Some(pos) = s[19..].find(['+', '-']) {
        let o = &s[19 + pos..];
        let sign = if o.starts_with('-') { -1 } else { 1 };
        let oh: i64 = o.get(1..3)?.parse().ok()?;
        let om: i64 = o.get(4..6).and_then(|x| x.parse().ok()).unwrap_or(0);
        secs -= sign * (oh * 3600 + om * 60);
    }
    Some(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_para_epoch_conhecido() {
        assert_eq!(parse_iso_secs("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_iso_secs("2000-03-01T00:00:00Z"), Some(951_868_800));
    }

    #[test]
    fn iso_com_fuso_e_fracao() {
        let utc = parse_iso_secs("2026-01-01T06:00:00Z");
        assert_eq!(parse_iso_secs("2026-01-01T03:00:00-03:00"), utc);
        assert_eq!(
            parse_iso_secs("2026-11-11T00:00:00.093909+00:00"),
            parse_iso_secs("2026-11-11T00:00:00Z")
        );
    }

    #[test]
    fn iso_invalido_vira_none() {
        assert_eq!(parse_iso_secs("amanhã"), None);
        assert_eq!(parse_iso_secs(""), None);
    }

    #[test]
    fn epoch_e_iso_fazem_ida_e_volta() {
        for secs in [0_i64, 951_868_800, 1_795_805_477, 1_760_000_123] {
            assert_eq!(parse_iso_secs(&iso_from_epoch(secs)), Some(secs));
        }
    }

    #[test]
    fn rotulos_das_janelas_do_codex() {
        assert_eq!(window_label(18_000), "Sessão · 5 h");
        assert_eq!(window_label(604_800), "Semana · 7 dias");
        assert_eq!(window_label(2_592_000), "Mês · 30 dias");
        assert_eq!(window_label(3 * 86_400), "3 dias");
        assert_eq!(window_label(7_200), "2 h");
    }
}

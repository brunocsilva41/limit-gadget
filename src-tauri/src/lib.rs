mod claude_logs;
mod insights;
mod providers;
mod session_logs;
#[cfg(windows)]
mod win;

use providers::*;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;

/// Endereço do latest.json das releases do GitHub, injetado pela pipeline (LIMITGADGET_UPDATE_URL).
const DEFAULT_UPDATE_URL: Option<&str> = option_env!("LIMITGADGET_UPDATE_URL");

fn effective_update_url(g: &Settings) -> String {
    let own = g.update_url.trim();
    if own.is_empty() {
        DEFAULT_UPDATE_URL.unwrap_or("").to_string()
    } else {
        own.to_string()
    }
}

type Shared = Arc<Mutex<Vec<Provider>>>;
struct LocalsState(tokio::sync::Mutex<Locals>);

const ALL: [(&str, &str); 4] = [
    ("claude", "Claude"),
    ("codex", "ChatGPT"),
    ("copilot", "Copilot"),
    ("antigravity", "Antigravity"),
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    interval_secs: u64,
    mode: String,  // "normal" | "mini" | "bar"
    theme: String, // "auto" | "dark" | "light"
    opacity: f64,
    always_on_top: bool,
    show_models: bool,
    show_extra: bool,
    warn_at: f64,
    crit_at: f64,
    autostart: bool,
    remember_pos: bool,
    pos: Option<(i32, i32)>,
    bar_match_taskbar: bool, // barra usa a cor real da barra de tarefas
    auto_update: bool,
    update_url: String,
    mono_icons: bool,       // ícones dos provedores sem cor
    monitor: usize,         // monitor onde o gadget aparece (0 = primeiro da lista)
    daily_budget_pct: f64,  // meta de consumo por dia (pontos percentuais); 0 desliga
    weekly_summary: bool,   // resumo na segunda-feira
    last_summary_day: i64,  // controle interno do resumo semanal
    notify: bool,           // avisos ao cruzar os limites
    close_to_bar: bool,     // fechar o gadget = continuar na barra de tarefas
    bar_auto: bool,         // posiciona a barra ao lado da bandeja sozinha
    bar_worst_only: bool,   // barra compacta: só o limite mais crítico
    providers: Vec<String>, // ids ativos: claude, codex, copilot, antigravity
    bar_offset: i32,        // distância (px lógicos) da borda direita da tela, no modo barra
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_secs: 30,
            mode: "normal".into(),
            theme: "auto".into(),
            opacity: 1.0,
            always_on_top: true,
            show_models: true,
            show_extra: true,
            warn_at: 70.0,
            crit_at: 90.0,
            autostart: false,
            remember_pos: true,
            pos: None,
            bar_match_taskbar: true,
            auto_update: true,
            update_url: String::new(),
            mono_icons: false,
            monitor: 0,
            daily_budget_pct: 0.0,
            weekly_summary: true,
            last_summary_day: 0,
            notify: true,
            close_to_bar: true,
            bar_auto: true,
            bar_worst_only: false,
            providers: ALL.iter().map(|(id, _)| id.to_string()).collect(),
            bar_offset: 340,
        }
    }
}

struct SettingsState(Arc<Mutex<Settings>>);
struct Wake(Arc<Notify>);

fn settings_path() -> Option<std::path::PathBuf> {
    let base = std::env::var("APPDATA").ok()?;
    let dir = std::path::PathBuf::from(base).join("LimitGadget");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("settings.json"))
}

fn load_settings() -> Settings {
    settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|r| serde_json::from_str(&r).ok())
        .unwrap_or_default()
}

fn persist(s: &Settings) {
    if let (Some(p), Ok(j)) = (settings_path(), serde_json::to_string_pretty(s)) {
        let _ = std::fs::write(p, j);
    }
}

fn cache_path() -> Option<std::path::PathBuf> {
    settings_path().map(|p| p.with_file_name("usage.json"))
}

fn load_cache() -> Option<Vec<Provider>> {
    let raw = std::fs::read_to_string(cache_path()?).ok()?;
    let mut list: Vec<Provider> = serde_json::from_str(&raw).ok()?;
    for p in &mut list {
        p.error = None;
    }
    Some(list)
}

fn save_cache(list: &[Provider]) {
    if let (Some(p), Ok(j)) = (cache_path(), serde_json::to_string(list)) {
        let _ = std::fs::write(p, j);
    }
}

fn set_autostart(on: bool) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
        let mut c = std::process::Command::new("reg");
        if on {
            if let Ok(exe) = std::env::current_exe() {
                c.args([
                    "add",
                    key,
                    "/v",
                    "LimitGadget",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &format!("\"{}\"", exe.display()),
                    "/f",
                ]);
            }
        } else {
            c.args(["delete", key, "/v", "LimitGadget", "/f"]);
        }
        let _ = c.creation_flags(0x0800_0000).status();
    }
    #[cfg(not(windows))]
    let _ = on;
}

fn ring_icon(pct: f64, warn: f64, crit: f64) -> Image<'static> {
    use std::f64::consts::TAU;
    const N: usize = 32;
    let color: [f64; 3] = if pct >= crit {
        [255.0, 69.0, 58.0]
    } else if pct >= warn {
        [255.0, 159.0, 10.0]
    } else {
        [255.0, 150.0, 112.0]
    };
    let c = (N as f64 - 1.0) / 2.0;
    let (radius, half) = (12.3, 2.25);
    let end = pct.clamp(1.0, 100.0) / 100.0 * TAU;
    // cobertura com borda suave de 1 px; `d` é a distância além da borda
    let cov = |d: f64| (0.5 - d).clamp(0.0, 1.0);
    let at = |ang: f64| (c + radius * ang.sin(), c - radius * ang.cos());
    let (sx, sy) = at(0.0);
    let (ex, ey) = at(end);
    let over = |dst: [f64; 4], src: [f64; 3], a: f64| -> [f64; 4] {
        let oa = a + dst[3] * (1.0 - a);
        if oa <= 0.0 {
            return [0.0; 4];
        }
        let mix = |s: f64, d: f64| (s * a + d * dst[3] * (1.0 - a)) / oa;
        [mix(src[0], dst[0]), mix(src[1], dst[1]), mix(src[2], dst[2]), oa]
    };
    let mut px = vec![0u8; N * N * 4];
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f64, y as f64);
            let (dx, dy) = (fx - c, fy - c);
            let d = (dx * dx + dy * dy).sqrt();
            let ring = cov((d - radius).abs() - half);
            let mut ang = dx.atan2(-dy);
            if ang < 0.0 {
                ang += TAU;
            }
            let cap = |cx: f64, cy: f64| cov(((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt() - half);
            let arc = (if ang <= end { ring } else { 0.0 }).max(cap(sx, sy)).max(cap(ex, ey));
            let mut out = [0.0; 4];
            out = over(out, [210.0, 210.0, 218.0], ring * 0.3);
            out = over(out, color, arc);
            out = over(out, [245.0, 245.0, 247.0], cov(d - 2.9) * 0.95);
            let i = (y * N + x) * 4;
            px[i] = out[0].round() as u8;
            px[i + 1] = out[1].round() as u8;
            px[i + 2] = out[2].round() as u8;
            px[i + 3] = (out[3] * 255.0).round() as u8;
        }
    }
    Image::new_owned(px, N as u32, N as u32)
}

/// Percentual mais restritivo de um provedor (ignora limites secundários).
fn primary_pct(p: &Provider) -> Option<f64> {
    if p.state == "offline" {
        return None;
    }
    p.meters
        .iter()
        .filter(|m| !m.minor)
        .map(|m| m.percent)
        .fold(None, |a, b| Some(a.map_or(b, |a: f64| a.max(b))))
}

async fn refresh(app: &AppHandle, client: &reqwest::Client) -> bool {
    let enabled = app.state::<SettingsState>().0.lock().unwrap().providers.clone();
    let prev = app.state::<Shared>().lock().unwrap().clone();
    let mut list: Vec<Provider> = Vec::new();
    let mut all_ok = true;

    {
        let locals_state = app.state::<LocalsState>();
        let mut loc = locals_state.0.lock().await;
        for (id, name) in ALL {
            if !enabled.iter().any(|e| e == id) {
                continue;
            }
            let res = match id {
                "claude" => fetch_claude(client).await,
                "codex" => fetch_codex(client).await,
                "copilot" => fetch_copilot(client, &mut loc.gh).await,
                _ => fetch_antigravity(&mut loc).await,
            };
            list.push(match res {
                Ok(p) => p,
                Err(f) => {
                    if !f.offline {
                        all_ok = false;
                    }
                    // mantém o último dado válido enquanto a falha durar
                    let mut old = prev.iter().find(|p| p.id == id).cloned().unwrap_or_else(|| Provider {
                        id: id.into(),
                        name: name.into(),
                        ..Default::default()
                    });
                    let keep_stale = f.offline && id == "antigravity" && !old.meters.is_empty();
                    if f.offline && !keep_stale {
                        old.meters.clear();
                        old.extra = None;
                    }
                    old.state = if keep_stale {
                        "stale".into()
                    } else if f.offline {
                        "offline".into()
                    } else {
                        "error".into()
                    };
                    old.error = Some(if keep_stale {
                        "IDE fechada. Mostrando a última leitura".to_string()
                    } else {
                        f.msg
                    });
                    old
                }
            });
        }

        // histórico, previsão de esgotamento, orçamento diário e avisos
        let (warn, crit, notify_on, budget, summary_on, last_day) = {
            let st = app.state::<SettingsState>();
            let g = st.0.lock().unwrap();
            (
                g.warn_at,
                g.crit_at,
                g.notify,
                g.daily_budget_pct,
                g.weekly_summary,
                g.last_summary_day,
            )
        };
        let now = now_ms();
        #[cfg(windows)]
        let tz = win::local_offset_secs();
        #[cfg(not(windows))]
        let tz = 0i64;
        let prm = insights::Params {
            now,
            warn,
            crit,
            budget,
            tz,
        };
        let mut notes: Vec<(String, String)> = Vec::new();
        for p in list.iter_mut().filter(|p| p.state == "ok") {
            loc.ins.record(p, &prm, &mut notes);
        }

        // resumo semanal: segunda-feira, a partir das 9h
        let local_secs = (now / 1000) as i64 + tz;
        let day = local_secs.div_euclid(86400);
        if summary_on && (day + 4).rem_euclid(7) == 1 && local_secs.rem_euclid(86400) >= 9 * 3600 && day != last_day {
            let peaks = loc.ins.peaks(&list);
            if !peaks.is_empty() {
                notes.push(("Resumo da semana".to_string(), peaks.join(" · ")));
            }
            let st = app.state::<SettingsState>();
            let mut g = st.0.lock().unwrap();
            g.last_summary_day = day;
            persist(&g);
        }

        // uso do Claude Code por projeto (varredura a cada 5 minutos)
        if enabled.iter().any(|e| e == "claude") {
            if loc
                .logs_at
                .is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(300))
            {
                let mut scanner = std::mem::take(&mut loc.logs);
                let secs = (now / 1000) as i64;
                let done = tauri::async_runtime::spawn_blocking(move || {
                    let found = scanner.scan(secs, tz);
                    (scanner, found)
                })
                .await;
                if let Ok((scanner, found)) = done {
                    loc.logs = scanner;
                    loc.projects = found;
                }
                loc.logs_at = Some(std::time::Instant::now());
            }
            let projects = loc.projects.clone();
            if let Some(p) = list.iter_mut().find(|p| p.id == "claude") {
                p.projects = projects;
            }
        }
        // últimas sessões do Codex e do Copilot
        if enabled.iter().any(|e| e == "codex" || e == "copilot") {
            if loc
                .sess_at
                .is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(120))
            {
                let mut scanner = std::mem::take(&mut loc.sess);
                let secs = (now / 1000) as i64;
                let done = tauri::async_runtime::spawn_blocking(move || {
                    let codex = scanner.scan_codex(secs);
                    let copilot = scanner.scan_copilot(secs);
                    (scanner, codex, copilot)
                })
                .await;
                if let Ok((scanner, codex, copilot)) = done {
                    loc.sess = scanner;
                    loc.codex_sessions = codex;
                    loc.copilot_sessions = copilot;
                }
                loc.sess_at = Some(std::time::Instant::now());
            }
            for p in list.iter_mut() {
                match p.id.as_str() {
                    "codex" => p.sessions = loc.codex_sessions.clone(),
                    "copilot" => p.sessions = loc.copilot_sessions.clone(),
                    _ => {}
                }
            }
        }
        loc.ins.persist();
        if notify_on {
            for (title, body) in notes {
                let _ = app.notification().builder().title(title).body(body).show();
            }
        }
    }

    save_cache(&list);
    *app.state::<Shared>().lock().unwrap() = list.clone();

    if let Some(tray) = app.tray_by_id("main") {
        let (warn, crit) = {
            let st = app.state::<SettingsState>();
            let g = st.0.lock().unwrap();
            (g.warn_at, g.crit_at)
        };
        let worst = list.iter().filter_map(primary_pct).fold(0.0, f64::max);
        let _ = tray.set_icon(Some(ring_icon(worst, warn, crit)));
        let parts: Vec<String> = list
            .iter()
            .filter_map(|p| primary_pct(p).map(|v| format!("{} {:.0}%", p.name, v)))
            .collect();
        let tip = if parts.is_empty() {
            "LimitGadget".to_string()
        } else {
            parts.join(" · ")
        };
        let _ = tray.set_tooltip(Some(tip));
    }
    let _ = app.emit("usage", list);
    all_ok
}

#[tauri::command]
fn get_usage(s: State<Shared>) -> Vec<Provider> {
    s.lock().unwrap().clone()
}

#[tauri::command]
fn refresh_now(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        refresh(
            &app,
            &reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        )
        .await;
    });
}

/// Redimensiona mantendo o canto inferior direito fixo (o widget cresce para cima e para a esquerda).
#[tauri::command]
fn set_size(app: AppHandle, width: f64, height: f64) {
    if let Some(w) = app.get_webview_window("main") {
        let scale = w.scale_factor().unwrap_or(1.0);
        if let (Ok(pos), Ok(size)) = (w.outer_position(), w.outer_size()) {
            let (nw, nh) = ((width * scale).round() as i32, (height * scale).round() as i32);
            if nw == size.width as i32 && nh == size.height as i32 {
                return;
            }
            let right = pos.x + size.width as i32;
            let bottom = pos.y + size.height as i32;
            let (mut x, mut y) = (right - nw, bottom - nh);
            // nunca deixa a janela sair da área útil do monitor (acima da barra de tarefas)
            if let Ok(Some(m)) = w.current_monitor() {
                let wa = m.work_area();
                let (ax, ay, aw, ah) = (
                    wa.position.x,
                    wa.position.y,
                    wa.size.width as i32,
                    wa.size.height as i32,
                );
                x = x.min(ax + aw - nw).max(ax);
                y = y.min(ay + ah - nh).max(ay);
            }
            let _ = w.set_size(LogicalSize::new(width, height));
            let _ = w.set_position(PhysicalPosition::new(x, y));
        }
    }
}

#[tauri::command]
fn get_settings(s: State<SettingsState>) -> Settings {
    s.0.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(app: AppHandle, s: State<SettingsState>, wake: State<Wake>, settings: Settings) {
    let (prev_auto, relayout, theme_changed) = {
        let mut g = s.0.lock().unwrap();
        let prev = g.autostart;
        let theme_changed = g.theme != settings.theme;
        let relayout = g.mode != settings.mode
            || (settings.mode == "bar"
                && (g.bar_offset != settings.bar_offset
                    || g.providers != settings.providers
                    || g.bar_auto != settings.bar_auto
                    || g.bar_worst_only != settings.bar_worst_only));
        let pos = g.pos;
        let last_summary = g.last_summary_day;
        *g = settings.clone();
        g.pos = pos;
        g.last_summary_day = last_summary;
        persist(&g);
        (prev, relayout, theme_changed)
    };
    if prev_auto != settings.autostart {
        set_autostart(settings.autostart);
    }
    if relayout {
        apply_layout(&app);
    }
    sync_topmost(&app);
    if theme_changed {
        if let Some(w) = app.get_webview_window("settings") {
            let _ = w.set_effects(settings_effects(settings_is_dark(&app)));
        }
    }
    let _ = app.emit("settings", settings);
    wake.0.notify_one();
}

/// 170 px de margens, pino e botões + 162 px por provedor (célula de 150 px mais o espaçamento).
fn bar_width(n: usize) -> f64 {
    (170.0 + 162.0 * n.max(1) as f64).min(1300.0)
}

/// Monitor escolhido nas configurações (cai para o principal se o índice não existir mais).
fn target_monitor(w: &tauri::WebviewWindow, idx: usize) -> Option<tauri::Monitor> {
    w.available_monitors()
        .ok()
        .and_then(|v| v.into_iter().nth(idx))
        .or_else(|| w.primary_monitor().ok().flatten())
}

/// Posiciona a janela conforme o modo: no modo barra, cola sobre a barra de tarefas.
fn apply_layout(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    let (mode, off, saved, bw, auto, mon_idx) = {
        let st = app.state::<SettingsState>();
        let g = st.0.lock().unwrap();
        let n = if g.bar_worst_only { 1 } else { g.providers.len() };
        // barra compacta com mais de um provedor ganha espaço para as setas
        let arrows = if g.bar_worst_only && g.providers.len() > 1 {
            64.0
        } else {
            0.0
        };
        (
            g.mode.clone(),
            g.bar_offset,
            g.pos,
            bar_width(n) + arrows,
            g.bar_auto,
            g.monitor,
        )
    };
    let Some(m) = target_monitor(&w, mon_idx) else { return };
    let sc = m.scale_factor();
    let (ms, mp) = (m.size(), m.position());
    if mode == "bar" {
        let mon_rect = (mp.x, mp.y, mp.x + ms.width as i32, mp.y + ms.height as i32);
        #[cfg(windows)]
        let (tb, tray) = {
            let tb = win::taskbar_rect_for(mon_rect);
            // a bandeja (relógio e ícones) só existe na barra de tarefas principal
            let tray = win::taskbar_rect()
                .filter(|r| (r.0 + r.2) / 2 >= mon_rect.0 && (r.0 + r.2) / 2 < mon_rect.2)
                .and_then(|_| win::tray_left());
            (tb, tray)
        };
        #[cfg(not(windows))]
        let (tb, tray): (Option<(i32, i32, i32, i32)>, Option<i32>) = (None, None);

        let wa = m.work_area();
        let (top, th) = match tb {
            Some((l, t, r, b)) if r - l >= b - t && b - t >= 20 => (t, b - t),
            _ => {
                let mut th = ms.height as i32 - wa.size.height as i32;
                if th < 20 {
                    th = (48.0 * sc) as i32;
                }
                (
                    if wa.position.y > mp.y {
                        mp.y
                    } else {
                        mp.y + ms.height as i32 - th
                    },
                    th,
                )
            }
        };
        let wp = (bw * sc) as i32;
        let gap = (8.0 * sc) as i32;
        let x = match (auto, tray) {
            (true, Some(tx)) => tx - wp - gap,
            _ => mp.x + ms.width as i32 - (off as f64 * sc) as i32 - wp,
        }
        .max(mp.x);
        let _ = w.set_size(LogicalSize::new(bw, th as f64 / sc));
        let _ = w.set_position(PhysicalPosition::new(x, top));
    } else {
        let (x, y) = saved
            .filter(|&(x, y)| {
                x >= mp.x - 50 && y >= mp.y - 50 && x < mp.x + ms.width as i32 - 20 && y < mp.y + ms.height as i32 - 20
            })
            .unwrap_or((
                mp.x + ms.width as i32 - (330.0 * sc) as i32,
                mp.y + ms.height as i32 - (300.0 * sc) as i32 - (56.0 * sc) as i32,
            ));
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
    sync_topmost(app);
}

/// Mantém a janela sempre acima (inclusive da barra de tarefas) no modo barra ou quando a opção está ligada.
fn sync_topmost(app: &AppHandle) {
    let active = {
        let st = app.state::<SettingsState>();
        let g = st.0.lock().unwrap();
        g.mode == "bar" || g.always_on_top
    };
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_always_on_top(active);
        #[cfg(windows)]
        {
            if let Ok(h) = w.hwnd() {
                win::HWND_MAIN.store(h.0 as isize, std::sync::atomic::Ordering::Relaxed);
            }
            win::KEEP.store(active, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Reerguer a janela acima da barra de tarefas (só faz algo no Windows).
fn keep_on_top() {
    #[cfg(windows)]
    {
        use std::sync::atomic::Ordering::Relaxed;
        if win::KEEP.load(Relaxed) {
            let h = win::HWND_MAIN.load(Relaxed);
            if h != 0 && win::is_visible(h) {
                win::raise(h);
            }
        }
    }
}

fn switch_mode(app: &AppHandle, mode: &str) {
    let snapshot = {
        let st = app.state::<SettingsState>();
        let mut g = st.0.lock().unwrap();
        g.mode = mode.to_string();
        persist(&g);
        g.clone()
    };
    apply_layout(app);
    let _ = app.emit("settings", snapshot);
}

/// "Fechar" não encerra o app: continua mostrando os limites na barra de tarefas (ou só oculta, se configurado).
fn close_action(app: &AppHandle) {
    let (close_to_bar, mode) = {
        let st = app.state::<SettingsState>();
        let g = st.0.lock().unwrap();
        (g.close_to_bar, g.mode.clone())
    };
    if close_to_bar {
        if mode != "bar" {
            switch_mode(app, "bar");
        }
    } else if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

/// Solta ou fixa a barra. Solta: pode ser arrastada pela barra de tarefas. Fixa: encosta na bandeja.
#[tauri::command]
fn bar_pin(app: AppHandle, pinned: bool) {
    let snapshot = {
        let st = app.state::<SettingsState>();
        let mut g = st.0.lock().unwrap();
        if !pinned {
            // mantém onde está: guarda a distância atual até a borda direita
            if let Some(w) = app.get_webview_window("main") {
                if let (Ok(Some(m)), Ok(pos), Ok(size)) = (w.current_monitor(), w.outer_position(), w.outer_size()) {
                    let right_gap = (m.position().x + m.size().width as i32) - (pos.x + size.width as i32);
                    g.bar_offset = ((right_gap as f64) / m.scale_factor()).round().max(0.0) as i32;
                }
            }
        }
        g.bar_auto = pinned;
        persist(&g);
        g.clone()
    };
    apply_layout(&app);
    let _ = app.emit("settings", snapshot);
}

/// Arrasto horizontal da barra solta (a posição vertical fica presa à barra de tarefas).
#[tauri::command]
fn bar_drag(app: AppHandle, dx: f64) {
    let st = app.state::<SettingsState>();
    let (mode, auto) = {
        let g = st.0.lock().unwrap();
        (g.mode.clone(), g.bar_auto)
    };
    if mode != "bar" || auto {
        return;
    }
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    let (Ok(Some(m)), Ok(pos), Ok(size)) = (w.current_monitor(), w.outer_position(), w.outer_size()) else {
        return;
    };
    let sc = m.scale_factor();
    let (mp, ms) = (m.position(), m.size());
    let nx =
        ((pos.x as f64 + dx * sc).round() as i32).clamp(mp.x, (mp.x + ms.width as i32 - size.width as i32).max(mp.x));
    let _ = w.set_position(PhysicalPosition::new(nx, pos.y));
    st.0.lock().unwrap().bar_offset = (((mp.x + ms.width as i32) - (nx + size.width as i32)) as f64 / sc)
        .round()
        .max(0.0) as i32;
}

#[tauri::command]
fn bar_drop(app: AppHandle) {
    let snapshot = {
        let st = app.state::<SettingsState>();
        let g = st.0.lock().unwrap();
        persist(&g);
        g.clone()
    };
    let _ = app.emit("settings", snapshot);
}

/// Cor real da barra de tarefas, amostrada logo à esquerda da barra do gadget.
#[tauri::command]
fn taskbar_color(app: AppHandle) -> Option<[u8; 3]> {
    #[cfg(windows)]
    {
        let w = app.get_webview_window("main")?;
        let pos = w.outer_position().ok()?;
        let m = w.current_monitor().ok()??;
        let (mp, ms) = (m.position(), m.size());
        let (l, t, _r, _b) = win::taskbar_rect_for((mp.x, mp.y, mp.x + ms.width as i32, mp.y + ms.height as i32))?;
        let sc = w.scale_factor().unwrap_or(1.0);
        let y = t + (2.0 * sc) as i32;
        let x_to = pos.x - (8.0 * sc) as i32;
        let x_from = (x_to - (320.0 * sc) as i32).max(l);
        win::sample_row(x_from, x_to, y)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        None
    }
}

#[derive(Serialize)]
struct MonitorInfo {
    index: usize,
    name: String,
    width: u32,
    height: u32,
    primary: bool,
}

#[tauri::command]
fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    let Some(w) = app.get_webview_window("main") else {
        return vec![];
    };
    let primary = w.primary_monitor().ok().flatten().map(|m| *m.position());
    w.available_monitors()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(i, m)| MonitorInfo {
            index: i,
            name: m.name().cloned().unwrap_or_else(|| format!("Monitor {}", i + 1)),
            width: m.size().width,
            height: m.size().height,
            primary: primary == Some(*m.position()),
        })
        .collect()
}

#[tauri::command]
fn default_update_url() -> String {
    DEFAULT_UPDATE_URL.unwrap_or("").to_string()
}

fn backup_path() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("USERPROFILE").map_err(|_| "Sem pasta de usuário".to_string())?;
    let dir = std::path::PathBuf::from(home).join("Documents");
    let _ = std::fs::create_dir_all(&dir);
    Ok(dir.join("LimitGadget-backup.json"))
}

/// Grava configurações e histórico em Documentos\LimitGadget-backup.json.
#[tauri::command]
async fn export_backup(app: AppHandle) -> Result<String, String> {
    let settings = app.state::<SettingsState>().0.lock().unwrap().clone();
    let history = app.state::<LocalsState>().0.lock().await.ins.export();
    let doc = serde_json::json!({
        "app": "LimitGadget",
        "version": app.package_info().version.to_string(),
        "settings": settings,
        "history": history,
    });
    let path = backup_path()?;
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("Não foi possível gravar: {e}"))?;
    Ok(path.display().to_string())
}

/// Restaura o backup de Documentos\LimitGadget-backup.json (mantém posição da janela).
#[tauri::command]
async fn import_backup(app: AppHandle) -> Result<String, String> {
    let path = backup_path()?;
    let raw = std::fs::read_to_string(&path).map_err(|_| format!("Arquivo não encontrado: {}", path.display()))?;
    let doc: serde_json::Value = serde_json::from_str(&raw).map_err(|_| "Arquivo de backup inválido".to_string())?;
    if doc.get("app").and_then(|a| a.as_str()) != Some("LimitGadget") {
        return Err("Este arquivo não é um backup do LimitGadget".into());
    }
    let imported: Settings = serde_json::from_value(doc.get("settings").cloned().unwrap_or_default())
        .map_err(|_| "Configurações do backup inválidas".to_string())?;
    let snapshot = {
        let st = app.state::<SettingsState>();
        let mut g = st.0.lock().unwrap();
        let (pos, last_summary) = (g.pos, g.last_summary_day);
        *g = imported;
        g.pos = pos;
        g.last_summary_day = last_summary;
        persist(&g);
        g.clone()
    };
    if let Some(h) = doc.get("history") {
        app.state::<LocalsState>().0.lock().await.ins.import(h.clone());
    }
    apply_layout(&app);
    let _ = app.emit("settings", snapshot);
    Ok(path.display().to_string())
}

#[derive(Serialize)]
struct UpdateInfo {
    available: bool,
    version: Option<String>,
    notes: Option<String>,
    current: String,
}

fn build_updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    use tauri_plugin_updater::UpdaterExt;
    let url = effective_update_url(&app.state::<SettingsState>().0.lock().unwrap());
    if url.is_empty() {
        return Err("Informe o endereço das atualizações.".into());
    }
    let endpoint = url.parse().map_err(|_| "Endereço inválido.".to_string())?;
    app.updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let updater = build_updater(&app)?;
    let current = app.package_info().version.to_string();
    match updater.check().await {
        Ok(Some(u)) => Ok(UpdateInfo {
            available: true,
            version: Some(u.version.clone()),
            notes: u.body.clone(),
            current,
        }),
        Ok(None) => Ok(UpdateInfo {
            available: false,
            version: None,
            notes: None,
            current,
        }),
        Err(e) => Err(format!("Não foi possível verificar: {e}")),
    }
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = build_updater(&app)?;
    match updater.check().await {
        Ok(Some(u)) => {
            u.download_and_install(|_, _| {}, || {})
                .await
                .map_err(|e| format!("Falha ao instalar: {e}"))?;
            app.restart();
        }
        Ok(None) => Ok(()),
        Err(e) => Err(format!("Não foi possível verificar: {e}")),
    }
}

#[tauri::command]
fn minimize_to_bar(app: AppHandle) {
    close_action(&app);
}

/// Reaplica o encaixe na barra (o frontend pode ter redimensionado a janela antes de carregar o modo).
#[tauri::command]
fn relayout(app: AppHandle) {
    let is_bar = app.state::<SettingsState>().0.lock().unwrap().mode == "bar";
    if is_bar {
        apply_layout(&app);
    }
}

#[tauri::command]
fn set_mode(app: AppHandle, mode: String) {
    switch_mode(&app, &mode);
}

#[tauri::command]
async fn open_settings(app: AppHandle) {
    open_settings_window(&app);
}

/// Vidro fosco da janela de configurações (acrílico), com tom escuro ou claro conforme o tema.
fn settings_effects(dark: bool) -> tauri::utils::config::WindowEffectsConfig {
    let (r, g, b, a) = if dark { (20, 20, 24, 160) } else { (244, 244, 248, 170) };
    tauri::window::EffectsBuilder::new()
        .effect(tauri::window::Effect::Acrylic)
        .color(tauri::window::Color(r, g, b, a))
        .build()
}

fn settings_is_dark(app: &AppHandle) -> bool {
    let theme = app.state::<SettingsState>().0.lock().unwrap().theme.clone();
    match theme.as_str() {
        "dark" => true,
        "light" => false,
        _ => app
            .get_webview_window("main")
            .and_then(|w| w.theme().ok())
            .is_none_or(|t| t == tauri::Theme::Dark),
    }
}

fn open_settings_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        bring_to_front(&w);
        return;
    }
    match WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("Configurações · LimitGadget")
        .inner_size(660.0, 720.0)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .effects(settings_effects(settings_is_dark(app)))
        .resizable(false)
        .maximizable(false)
        .center()
        .build()
    {
        Ok(w) => bring_to_front(&w),
        Err(e) => eprintln!("falha ao abrir configurações: {e}"),
    }
}

/// O Windows impede janelas novas de roubar o foco; alternar "sempre no topo" traz a janela para frente.
fn bring_to_front(w: &tauri::WebviewWindow) {
    let _ = w.unminimize();
    let _ = w.show();
    let _ = w.set_always_on_top(true);
    let _ = w.set_always_on_top(false);
    let _ = w.set_focus();
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

fn toggle(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

pub fn run() {
    let shared: Shared = Arc::new(Mutex::new(load_cache().unwrap_or_default()));
    let settings = Arc::new(Mutex::new(load_settings()));
    let loop_settings = settings.clone();
    let wake = Arc::new(Notify::new());
    let loop_wake = wake.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle(app);
                    }
                })
                .build(),
        )
        .manage(shared)
        .manage(LocalsState(tokio::sync::Mutex::new(Locals {
            ins: insights::Insights::load(),
            ..Default::default()
        })))
        .manage(SettingsState(settings))
        .manage(Wake(wake))
        .on_window_event(|w, ev| {
            if w.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = ev {
                    api.prevent_close();
                    close_action(w.app_handle());
                }
                if let WindowEvent::Moved(p) = ev {
                    let st = w.app_handle().state::<SettingsState>();
                    let mut g = st.0.lock().unwrap();
                    if g.mode != "bar" {
                        g.pos = Some((p.x, p.y));
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_usage,
            refresh_now,
            set_size,
            hide_window,
            get_settings,
            save_settings,
            set_mode,
            relayout,
            minimize_to_bar,
            bar_pin,
            bar_drag,
            bar_drop,
            taskbar_color,
            list_monitors,
            default_update_url,
            export_backup,
            import_backup,
            check_update,
            install_update,
            open_settings
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // posição inicial: a salva (se ainda cabe em algum monitor) ou o canto inferior direito
            if let Some(w) = app.get_webview_window("main") {
                let saved = {
                    let g = loop_settings.lock().unwrap();
                    let _ = w.set_always_on_top(g.always_on_top);
                    if g.remember_pos {
                        g.pos
                    } else {
                        None
                    }
                };
                let fits = saved.is_some_and(|(x, y)| {
                    w.available_monitors().unwrap_or_default().iter().any(|m| {
                        let (p, sz) = (m.position(), m.size());
                        x >= p.x - 50
                            && y >= p.y - 50
                            && x < p.x + sz.width as i32 - 20
                            && y < p.y + sz.height as i32 - 20
                    })
                });
                if let (true, Some((x, y))) = (fits, saved) {
                    let _ = w.set_position(PhysicalPosition::new(x, y));
                } else if let (Ok(Some(m)), Ok(sz)) = (w.primary_monitor(), w.outer_size()) {
                    let (ms, mp) = (m.size(), m.position());
                    let _ = w.set_position(PhysicalPosition::new(
                        mp.x + ms.width as i32 - sz.width as i32 - 12,
                        mp.y + ms.height as i32 - sz.height as i32 - (56.0 * m.scale_factor()) as i32,
                    ));
                }
            }

            // mantém o início automático apontando para este executável (o caminho pode ter mudado)
            if loop_settings.lock().unwrap().autostart {
                set_autostart(true);
            }

            if std::env::args().any(|a| a == "--settings") {
                open_settings_window(&handle);
            }

            if loop_settings.lock().unwrap().mode == "bar" {
                apply_layout(&handle);
            }
            #[cfg(windows)]
            win::install_hook();
            sync_topmost(&handle);
            tauri::async_runtime::spawn(async move {
                // reforço do gancho de foco: reergue a janela acima da barra de tarefas
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    keep_on_top();
                }
            });
            let _ = app.global_shortcut().register("ctrl+alt+l");

            let upd_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(90)).await;
                let mut notified: Option<String> = None;
                loop {
                    let (on, has_url) = {
                        let st = upd_handle.state::<SettingsState>();
                        let g = st.0.lock().unwrap();
                        (g.auto_update, !effective_update_url(&g).is_empty())
                    };
                    if on && has_url {
                        if let Ok(info) = check_update(upd_handle.clone()).await {
                            if info.available && info.version != notified {
                                let v = info.version.clone().unwrap_or_default();
                                let _ = upd_handle
                                    .notification()
                                    .builder()
                                    .title("Atualização do LimitGadget")
                                    .body(format!("A versão {v} está disponível. Instale em Configurações."))
                                    .show();
                                notified = info.version;
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await;
                }
            });

            let show = MenuItem::with_id(app, "toggle", "Mostrar / ocultar", true, None::<&str>)?;
            let cfg = MenuItem::with_id(app, "settings", "Configurações…", true, None::<&str>)?;
            let upd = MenuItem::with_id(app, "refresh", "Atualizar agora", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &cfg, &upd, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(ring_icon(0.0, 70.0, 90.0))
                .tooltip("LimitGadget · carregando…")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, ev| match ev.id.as_ref() {
                    "toggle" => toggle(app),
                    "refresh" => {
                        refresh_now(app.clone());
                    }
                    "settings" => open_settings_window(app),
                    "quit" => {
                        persist(&app.state::<SettingsState>().0.lock().unwrap());
                        app.exit(0)
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = ev
                    {
                        toggle(tray.app_handle());
                    }
                })
                .build(app)?;

            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .unwrap();
                loop {
                    let ok = refresh(&handle, &client).await;
                    let secs = {
                        let g = loop_settings.lock().unwrap();
                        persist(&g);
                        let base = g.interval_secs.clamp(10, 600);
                        if ok {
                            base
                        } else {
                            base.max(60)
                        }
                    };
                    // dorme até o próximo ciclo ou até as configurações mudarem
                    let _ = tokio::time::timeout(std::time::Duration::from_secs(secs), loop_wake.notified()).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao rodar o LimitGadget");
}

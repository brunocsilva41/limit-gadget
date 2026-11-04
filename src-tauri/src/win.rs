//! Integração com a barra de tarefas do Windows: manter a janela sempre acima e achar a bandeja.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
use windows_sys::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowRect, IsWindowVisible, SetWindowPos, EVENT_SYSTEM_FOREGROUND, HWND_TOPMOST,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WINEVENT_OUTOFCONTEXT,
};

/// Janela principal (HWND como inteiro) e se ela deve ser mantida acima de tudo.
pub static HWND_MAIN: AtomicIsize = AtomicIsize::new(0);
pub static KEEP: AtomicBool = AtomicBool::new(false);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Reergue a janela para o topo da pilha "sempre à frente", sem ativá-la (não rouba o foco).
pub fn raise(hwnd: isize) {
    unsafe {
        SetWindowPos(
            hwnd as HWND,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

pub fn is_visible(hwnd: isize) -> bool {
    unsafe { IsWindowVisible(hwnd as HWND) != 0 }
}

unsafe extern "system" fn on_foreground(_h: HWINEVENTHOOK, _e: u32, _w: HWND, _o: i32, _c: i32, _t: u32, _ms: u32) {
    // a barra de tarefas se reergue quando recebe clique; reafirmamos o topo na hora
    if KEEP.load(Ordering::Relaxed) {
        let h = HWND_MAIN.load(Ordering::Relaxed);
        if h != 0 && is_visible(h) {
            raise(h);
        }
    }
}

/// Deve ser chamado na thread principal (a que tem o laço de mensagens).
pub fn install_hook() {
    unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            std::ptr::null_mut(),
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
    }
}

fn rect_of(h: HWND) -> Option<(i32, i32, i32, i32)> {
    if h.is_null() {
        return None;
    }
    let mut r = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        if GetWindowRect(h, &mut r) == 0 {
            return None;
        }
    }
    Some((r.left, r.top, r.right, r.bottom))
}

/// Retângulo da barra de tarefas principal: (esquerda, topo, direita, base) em pixels físicos.
pub fn taskbar_rect() -> Option<(i32, i32, i32, i32)> {
    unsafe { rect_of(FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null())) }
}

/// Borda esquerda da área de notificação (relógio e ícones), em pixels físicos.
pub fn tray_left() -> Option<i32> {
    unsafe {
        let tray = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
        if tray.is_null() {
            return None;
        }
        rect_of(FindWindowExW(
            tray,
            std::ptr::null_mut(),
            wide("TrayNotifyWnd").as_ptr(),
            std::ptr::null(),
        ))
        .map(|r| r.0)
    }
}

/// Cor real da barra de tarefas: mediana de pixels de uma faixa fina no topo dela (onde não há ícones).
pub fn sample_row(x_from: i32, x_to: i32, y: i32) -> Option<[u8; 3]> {
    if x_to - x_from < 20 {
        return None;
    }
    let (mut rs, mut gs, mut bs) = (Vec::new(), Vec::new(), Vec::new());
    unsafe {
        let dc = GetDC(std::ptr::null_mut());
        if dc.is_null() {
            return None;
        }
        let step = ((x_to - x_from) / 24).max(1);
        let mut x = x_from;
        while x < x_to {
            let c = GetPixel(dc, x, y);
            if c != 0xFFFF_FFFF {
                rs.push((c & 0xFF) as u8);
                gs.push(((c >> 8) & 0xFF) as u8);
                bs.push(((c >> 16) & 0xFF) as u8);
            }
            x += step;
        }
        ReleaseDC(std::ptr::null_mut(), dc);
    }
    if rs.is_empty() {
        return None;
    }
    let med = |v: &mut Vec<u8>| {
        v.sort_unstable();
        v[v.len() / 2]
    };
    Some([med(&mut rs), med(&mut gs), med(&mut bs)])
}

/// Diferença do horário local para UTC, em segundos (inclui horário de verão).
pub fn local_offset_secs() -> i64 {
    unsafe {
        let mut tz: TIME_ZONE_INFORMATION = std::mem::zeroed();
        let id = GetTimeZoneInformation(&mut tz);
        let extra = if id == 2 { tz.DaylightBias } else { tz.StandardBias };
        -((tz.Bias + extra) as i64) * 60
    }
}

fn center_in(r: (i32, i32, i32, i32), m: (i32, i32, i32, i32)) -> bool {
    let (cx, cy) = ((r.0 + r.2) / 2, (r.1 + r.3) / 2);
    cx >= m.0 && cx < m.2 && cy >= m.1 && cy < m.3
}

/// Barra de tarefas que está dentro do monitor `mon` (esquerda, topo, direita, base). Inclui as secundárias.
pub fn taskbar_rect_for(mon: (i32, i32, i32, i32)) -> Option<(i32, i32, i32, i32)> {
    if let Some(r) = taskbar_rect() {
        if center_in(r, mon) {
            return Some(r);
        }
    }
    unsafe {
        let mut prev: HWND = std::ptr::null_mut();
        loop {
            let h = FindWindowExW(
                std::ptr::null_mut(),
                prev,
                wide("Shell_SecondaryTrayWnd").as_ptr(),
                std::ptr::null(),
            );
            if h.is_null() {
                return None;
            }
            if let Some(r) = rect_of(h) {
                if center_in(r, mon) {
                    return Some(r);
                }
            }
            prev = h;
        }
    }
}

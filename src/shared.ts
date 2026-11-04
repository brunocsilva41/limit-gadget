import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface Meter { label: string; percent: number; resets_at: string | null; detail: string | null; minor: boolean; eta_secs: number | null; today: number | null }
export interface Extra { percent: number; used: number; limit: number; currency: string }
export interface SessionUse { title: string; project: string; updated_ms: number; tokens: number | null; turns: number | null }
export interface ProjectUse { name: string; tokens_7d: number; tokens_today: number }
export interface Provider {
  id: string
  name: string
  plan: string | null
  meters: Meter[]
  extra: Extra | null
  updated_ms: number
  state: 'ok' | 'offline' | 'error' | 'stale'
  error: string | null
  history: [number, number][]
  projects: ProjectUse[]
  sessions: SessionUse[]
}

/** Limite mais restritivo do provedor (ignora os secundários). */
export function primaryMeter(p: Provider): Meter | null {
  if (p.state === 'offline') return null
  const list = p.meters.filter((m) => !m.minor)
  return list.length ? list.reduce((a, b) => (b.percent > a.percent ? b : a)) : null
}

export const PROVIDER_INFO: { id: string; name: string; hint: string }[] = [
  { id: 'claude', name: 'Claude', hint: 'Usa o login do Claude Code.' },
  { id: 'codex', name: 'ChatGPT / Codex', hint: 'Usa o login do Codex (codex login).' },
  { id: 'copilot', name: 'GitHub Copilot', hint: 'Usa o login do GitHub (gh auth login).' },
  { id: 'antigravity', name: 'Antigravity', hint: 'Ao vivo com a IDE aberta; fechada, mostra a última leitura.' },
]
export interface Settings {
  interval_secs: number
  mode: 'normal' | 'mini' | 'bar'
  theme: 'auto' | 'dark' | 'light'
  opacity: number
  always_on_top: boolean
  show_models: boolean
  show_extra: boolean
  warn_at: number
  crit_at: number
  autostart: boolean
  remember_pos: boolean
  pos: [number, number] | null
  mono_icons: boolean
  monitor: number
  daily_budget_pct: number
  weekly_summary: boolean
  bar_match_taskbar: boolean
  auto_update: boolean
  update_url: string
  notify: boolean
  close_to_bar: boolean
  bar_auto: boolean
  bar_worst_only: boolean
  providers: string[]
  bar_offset: number
}

export const DEFAULTS: Settings = {
  interval_secs: 30, mode: 'normal', theme: 'auto', opacity: 1, always_on_top: true,
  show_models: true, show_extra: true, warn_at: 70, crit_at: 90, autostart: false,
  remember_pos: true, pos: null, mono_icons: false, monitor: 0, daily_budget_pct: 0, weekly_summary: true,
  bar_match_taskbar: true, auto_update: true, update_url: '',
  notify: true, close_to_bar: true, bar_auto: true, bar_worst_only: false,
  providers: ['claude', 'codex', 'copilot', 'antigravity'], bar_offset: 340,
}

export function useSettings(): Settings {
  const [s, setS] = useState<Settings>(DEFAULTS)
  useEffect(() => {
    invoke<Settings>('get_settings').then(setS).catch(() => {})
    const un = listen<Settings>('settings', (e) => setS(e.payload))
    return () => { un.then((f) => f()) }
  }, [])
  // aplica tema e opacidade no documento
  useEffect(() => {
    document.documentElement.dataset.theme = s.theme
    document.documentElement.style.setProperty('--alpha', String(s.opacity))
  }, [s.theme, s.opacity])
  return s
}

export const levelColor = (p: number, s: Settings) =>
  p >= s.crit_at ? 'var(--crit)' : p >= s.warn_at ? 'var(--warn)' : 'var(--accent)'

export function until(iso: string | null, now: number): string {
  if (!iso) return ''
  const s = Math.max(0, Math.floor((Date.parse(iso) - now) / 1000))
  const d = Math.floor(s / 86400)
  const h = Math.floor((s % 86400) / 3600)
  const m = Math.floor((s % 3600) / 60)
  if (d > 0) return `${d}d ${h}h`
  if (h > 0) return `${h}h ${m}min`
  return m > 0 ? `${m} min` : '< 1 min'
}

export function ago(ms: number, now: number): string {
  if (!ms) return 'ainda não'
  const s = Math.max(0, Math.floor((now - ms) / 1000))
  if (s < 5) return 'agora'
  if (s < 60) return `${s}s atrás`
  if (s < 3600) return `${Math.floor(s / 60)} min atrás`
  if (s < 86400) return `${Math.floor(s / 3600)} h atrás`
  return `${Math.floor(s / 86400)} d atrás`
}

export function fmtSecs(total: number): string {
  const d = Math.floor(total / 86400)
  const h = Math.floor((total % 86400) / 3600)
  const m = Math.floor((total % 3600) / 60)
  if (d > 0) return `${d}d ${h}h`
  if (h > 0) return `${h}h ${m}min`
  return m > 0 ? `${m} min` : '< 1 min'
}

export function fmtTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1).replace('.', ',')} mi`
  if (n >= 1_000) return `${Math.round(n / 1_000)} mil`
  return String(n)
}

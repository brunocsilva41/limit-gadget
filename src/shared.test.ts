import { describe, expect, it } from 'vitest'
import { ago, DEFAULTS, fmtSecs, fmtTokens, levelColor, primaryMeter, until, type Meter, type Provider } from './shared'

const meter = (label: string, percent: number, minor = false): Meter => ({
  label, percent, resets_at: null, detail: null, minor, eta_secs: null, today: null,
})

const provider = (meters: Meter[], state: Provider['state'] = 'ok'): Provider => ({
  id: 'claude', name: 'Claude', plan: null, meters, extra: null, updated_ms: 0, state, error: null, history: [], projects: [], sessions: [],
})

describe('fmtTokens', () => {
  it('abaixo de mil mostra o número', () => expect(fmtTokens(850)).toBe('850'))
  it('milhares', () => expect(fmtTokens(12_400)).toBe('12 mil'))
  it('milhões com vírgula', () => expect(fmtTokens(2_350_000)).toBe('2,4 mi'))
})

describe('fmtSecs', () => {
  it('formata menos de 1 minuto', () => expect(fmtSecs(30)).toBe('< 1 min'))
  it('formata minutos', () => expect(fmtSecs(5 * 60)).toBe('5 min'))
  it('formata horas e minutos', () => expect(fmtSecs(3600 + 90 * 60)).toBe('2h 30min'))
  it('formata dias e horas', () => expect(fmtSecs(86400 + 3 * 3600)).toBe('1d 3h'))
})

describe('until', () => {
  const now = Date.parse('2026-01-01T00:00:00Z')
  it('retorna vazio sem data', () => expect(until(null, now)).toBe(''))
  it('não fica negativo para datas passadas', () => expect(until('2025-12-31T00:00:00Z', now)).toBe('< 1 min'))
  it('formata minutos', () => expect(until('2026-01-01T00:10:00Z', now)).toBe('10 min'))
  it('formata horas', () => expect(until('2026-01-01T02:05:00Z', now)).toBe('2h 5min'))
  it('formata dias', () => expect(until('2026-01-03T04:00:00Z', now)).toBe('2d 4h'))
})

describe('ago', () => {
  const now = 1_000_000
  it('sem leitura', () => expect(ago(0, now)).toBe('ainda não'))
  it('agora', () => expect(ago(now - 2000, now)).toBe('agora'))
  it('segundos', () => expect(ago(now - 30_000, now)).toBe('30s atrás'))
  it('minutos', () => expect(ago(now - 150_000, now)).toBe('2 min atrás'))
  it('horas', () => expect(ago(now - 3 * 3600_000 - 60_000, now)).toBe('3 h atrás'))
  it('dias', () => expect(ago(now - 2 * 86_400_000 - 5_000, now)).toBe('2 d atrás'))
})

describe('primaryMeter', () => {
  it('retorna null quando offline', () => expect(primaryMeter(provider([meter('a', 50)], 'offline'))).toBeNull())
  it('retorna null sem medidores principais', () => expect(primaryMeter(provider([meter('a', 50, true)]))).toBeNull())
  it('escolhe o maior percentual ignorando os secundários', () => {
    const p = provider([meter('a', 20), meter('b', 80), meter('c', 99, true)])
    expect(primaryMeter(p)?.label).toBe('b')
  })
})

describe('levelColor', () => {
  it('usa accent abaixo do aviso', () => expect(levelColor(10, DEFAULTS)).toBe('var(--accent)'))
  it('usa warn a partir do limite de aviso', () => expect(levelColor(DEFAULTS.warn_at, DEFAULTS)).toBe('var(--warn)'))
  it('usa crit a partir do limite crítico', () => expect(levelColor(DEFAULTS.crit_at, DEFAULTS)).toBe('var(--crit)'))
})

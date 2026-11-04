import { levelColor, Settings } from './shared'

/**
 * Anel de progresso: a porcentagem fica no centro e o arco em volta mostra quanto do limite já foi usado.
 * `pct` nulo desenha só o trilho (sem dados).
 */
export function PctRing({ pct, size, s, big = false }: { pct: number | null; size: number; s: Settings; big?: boolean }) {
  const stroke = Math.max(2.5, Math.round(size / 11))
  const r = (size - stroke) / 2
  const c = 2 * Math.PI * r
  const has = pct !== null
  const value = has ? Math.max(0, Math.min(100, pct)) : 0
  const color = levelColor(value, s)
  const warn = has && value >= s.warn_at
  return (
    <div className="pring" style={{ width: size, height: size }} role="img" aria-label={has ? `${Math.round(value)} por cento do limite` : 'sem dados'}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden="true">
        <circle className="rtrack" cx={size / 2} cy={size / 2} r={r} fill="none" strokeWidth={stroke} />
        {has && (
          <circle
            className="rbar"
            cx={size / 2}
            cy={size / 2}
            r={r}
            fill="none"
            strokeWidth={stroke}
            strokeLinecap="round"
            stroke={color}
            strokeDasharray={c}
            strokeDashoffset={c * (1 - value / 100)}
          />
        )}
      </svg>
      <span className="rnum" style={{ fontSize: size * (big ? 0.3 : 0.36), color: warn ? color : undefined }}>
        {has ? Math.round(value) : '–'}
        {big && has && <small>%</small>}
      </span>
    </div>
  )
}

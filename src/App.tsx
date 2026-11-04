import { ReactNode, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { PinIcon, ProviderIcon } from './icons'
import { PctRing } from './ring'
import { ago, fmtSecs, fmtTokens, levelColor, Meter, primaryMeter, Provider, Settings, until, useSettings } from './shared'

const hot = (p: number, s: Settings) => (p >= s.warn_at ? levelColor(p, s) : undefined)

function MeterRow({ m, now, s }: { m: Meter; now: number; s: Settings }) {
  const today = m.today != null && m.today >= 0.5 ? `hoje +${Math.round(m.today)}%` : null
  const sub = [m.resets_at ? `reseta em ${until(m.resets_at, now)}` : 'sem uso', m.detail, today].filter(Boolean).join(' · ')
  return (
    <div className="meter">
      <div className="mtop">
        <div className="mname">
          <b>{m.label}</b>
          <span title={sub}>{sub}</span>
          {m.eta_secs != null && <em className="eta">No ritmo atual, acaba em {fmtSecs(m.eta_secs)}</em>}
        </div>
        <PctRing pct={m.percent} size={46} s={s} big />
      </div>
    </div>
  )
}

function Trend({ pts, now }: { pts: [number, number][]; now: number }) {
  const t0 = pts[0][0]
  const t1 = Math.max(pts[pts.length - 1][0], t0 + 1)
  const path = pts.map(([t, v]) => `${(((t - t0) / (t1 - t0)) * 100).toFixed(1)},${(23 - (Math.min(100, v) / 100) * 21).toFixed(1)}`).join(' ')
  const days = (now - t0) / 86400000
  const label = days >= 1 ? `últimos ${Math.round(days)} dias` : `últimas ${Math.max(1, Math.round(days * 24))} h`
  return (
    <div className="trend">
      <svg viewBox="0 0 100 24" preserveAspectRatio="none" role="img" aria-label={`Uso recente, ${label}`}>
        <polyline points={path} fill="none" stroke="var(--accent)" strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
      </svg>
      <span>{label}</span>
    </div>
  )
}

/** Seção que abre e fecha com um botão discreto; a escolha fica guardada. */
function Fold({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  const key = `lg.fold.${id}`
  const [open, setOpen] = useState(() => localStorage.getItem(key) !== '0')
  const toggle = () => {
    const next = !open
    setOpen(next)
    localStorage.setItem(key, next ? '1' : '0')
  }
  return (
    <div className="fold">
      <button className="foldhead" aria-expanded={open} onClick={toggle} title={open ? 'Recolher' : 'Expandir'}>
        <span>{title}</span>
        <svg className={`fchev ${open ? 'open' : ''}`} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M4 6.5l4 4 4-4" />
        </svg>
      </button>
      <div className={`foldwrap ${open ? 'open' : ''}`} aria-hidden={!open}>
        <div className="foldin">{children}</div>
      </div>
    </div>
  )
}

function summary(p: Provider): string {
  if (p.state === 'offline') return p.error ?? 'indisponível'
  const m = primaryMeter(p)
  const base = m ? `${p.name}: ${m.label}, ${Math.round(m.percent)}%` : p.name
  return p.state === 'stale' ? `${base} (última leitura, IDE fechada)` : base
}

export default function App() {
  const s = useSettings()
  const [list, setList] = useState<Provider[]>([])
  const [openId, setOpenId] = useState<string | null>(null)
  const [now, setNow] = useState(Date.now())
  const [spin, setSpin] = useState(false)
  const [pick, setPick] = useState<string | null>(null) // provedor escolhido na barra compacta
  const card = useRef<HTMLDivElement>(null)
  const dragX = useRef<number | null>(null)
  const [tb, setTb] = useState<[number, number, number] | null>(null)

  useEffect(() => {
    invoke<Provider[]>('get_usage').then(setList).catch(() => {})
    const un = listen<Provider[]>('usage', (e) => setList(e.payload))
    const t = setInterval(() => setNow(Date.now()), 1000)
    return () => { clearInterval(t); un.then((f) => f()) }
  }, [])

  // abre o primeiro provedor só uma vez; depois quem manda é o clique do usuário
  const opened = useRef(false)
  useEffect(() => {
    if (!opened.current && list.length) {
      opened.current = true
      setOpenId(list[0].id)
    }
  }, [list])

  const mini = s.mode === 'mini'
  const bar = s.mode === 'bar'

  useEffect(() => {
    document.body.classList.toggle('bar-mode', bar)
    if (bar) invoke('relayout')
  }, [bar])

  // a janela acompanha o tamanho real do cartão (na barra, o Rust define o tamanho)
  useLayoutEffect(() => {
    const el = card.current
    if (!el || bar) return
    const fit = () => {
      const r = el.getBoundingClientRect()
      invoke('set_size', { width: Math.ceil(r.width) + 16, height: Math.ceil(r.height) + 16 })
    }
    const ro = new ResizeObserver(fit)
    ro.observe(el)
    fit()
    return () => ro.disconnect()
  }, [bar, mini])

  // cor real da barra de tarefas, relida de tempos em tempos
  useEffect(() => {
    if (!bar || !s.bar_match_taskbar) {
      setTb(null)
      return
    }
    let alive = true
    const read = () => invoke<[number, number, number] | null>('taskbar_color').then((c) => alive && setTb(c)).catch(() => {})
    read()
    const t = setInterval(read, 5000)
    return () => { alive = false; clearInterval(t) }
  }, [bar, s.bar_match_taskbar])

  // arrasto horizontal da barra quando está solta
  const barDown = (e: React.PointerEvent) => {
    if (s.bar_auto || e.button !== 0 || (e.target as HTMLElement).closest('button')) return
    dragX.current = e.screenX
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }
  const barMove = (e: React.PointerEvent) => {
    if (dragX.current === null) return
    const dx = e.screenX - dragX.current
    if (dx) {
      dragX.current = e.screenX
      invoke('bar_drag', { dx })
    }
  }
  const barUp = () => {
    if (dragX.current !== null) {
      dragX.current = null
      invoke('bar_drop')
    }
  }

  const drag = (e: React.MouseEvent) => {
    if (e.button === 0 && !(e.target as HTMLElement).closest('button')) getCurrentWindow().startDragging()
  }
  const refresh = () => {
    setSpin(true)
    invoke('refresh_now')
    setTimeout(() => setSpin(false), 800)
  }

  const nextMode = s.mode === 'normal' ? 'mini' : s.mode === 'mini' ? 'bar' : 'normal'
  const modeName = { normal: 'Normal', mini: 'Mini', bar: 'Barra' }[nextMode]
  const modeBtn = (cls: string) => (
    <button className={cls} onClick={() => invoke('set_mode', { mode: nextMode })} title={`Mudar para ${modeName}`} aria-label={`Mudar para o modo ${modeName}`}>
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <rect x="2" y="3" width="12" height="10" rx="2" /><path d="M2 10h12" />
      </svg>
    </button>
  )
  const refreshBtn = (cls: string) => (
    <button className={`${cls} ${spin ? 'spin' : ''}`} onClick={refresh} title="Atualizar agora" aria-label="Atualizar agora">
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2.5v3h-3" /></svg>
    </button>
  )
  const closeBtn = (cls: string) => (
    <button className={cls} onClick={() => invoke('minimize_to_bar')} title="Fechar (o gadget continua na barra de tarefas)" aria-label="Fechar o gadget, mantendo os limites na barra de tarefas">
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round"><path d="M4 4l8 8M12 4l-8 8" /></svg>
    </button>
  )
  const gearBtn = (cls: string) => (
    <button className={cls} onClick={() => invoke('open_settings')} title="Configurações" aria-label="Abrir configurações">
      <GearIcon />
    </button>
  )

  const loading = list.length === 0

  const worst = [...list].filter((p) => primaryMeter(p)).sort((a, b) => primaryMeter(b)!.percent - primaryMeter(a)!.percent)[0]
  // barra compacta: um provedor por vez (o mais perto do limite, até você escolher outro com as setas)
  const compact = s.bar_worst_only
  const shown = compact ? (list.find((p) => p.id === pick) ?? worst ?? list[0]) : undefined
  const barList = compact ? (shown ? [shown] : []) : list
  const step = (d: number) => {
    if (list.length < 2 || !shown) return
    const i = list.findIndex((p) => p.id === shown.id)
    setPick(list[(i + d + list.length) % list.length].id)
  }
  const navBtn = (d: -1 | 1) => (
    <button
      className="btn xs cnav"
      onClick={() => step(d)}
      title={d < 0 ? 'Provedor anterior' : 'Próximo provedor'}
      aria-label={d < 0 ? 'Mostrar o provedor anterior' : 'Mostrar o próximo provedor'}
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d={d < 0 ? 'M10 3.5L5.5 8 10 12.5' : 'M6 3.5L10.5 8 6 12.5'} />
      </svg>
    </button>
  )

  // ------------------------------------------------------------ barra
  if (bar) {
    const lum = tb ? (0.2126 * tb[0] + 0.7152 * tb[1] + 0.0722 * tb[2]) / 255 : 0
    return (
      <div
        className={`card bar ${tb ? (lum > 0.6 ? 'tone-light' : 'tone-dark') : ''} ${s.bar_auto ? '' : 'free'}`}
        style={tb ? ({ '--bar-rgb': tb.join(',') } as React.CSSProperties) : undefined}
        onPointerDown={barDown}
        onPointerMove={barMove}
        onPointerUp={barUp}
      >
        <button
          className={`btn xs pin ${s.bar_auto ? 'on' : ''}`}
          aria-pressed={s.bar_auto}
          onClick={() => invoke('bar_pin', { pinned: !s.bar_auto })}
          title={s.bar_auto ? 'Fixada ao lado da bandeja. Clique para soltar e mover pela barra' : 'Solta: arraste para mover pela barra. Clique para fixar'}
          aria-label={s.bar_auto ? 'Soltar a barra para mover' : 'Fixar a barra ao lado da bandeja'}
        >
          <PinIcon filled={s.bar_auto} />
        </button>
        {compact && list.length > 1 && navBtn(-1)}
        {barList.map((p) => {
          const m = primaryMeter(p)
          const off = p.state === 'offline'
          const sub = off ? 'indisponível' : p.state === 'stale' ? 'última leitura' : m?.resets_at ? until(m.resets_at, now) : m ? 'sem uso' : 'sem limite'
          return (
            <div key={p.id} className={`bcell ${off ? 'off' : ''} ${p.state === 'stale' ? 'stale' : ''}`} title={summary(p)}>
              <ProviderIcon id={p.id} size={15} mono={s.mono_icons} />
              <div className="bmain">
                <div className="blabel">
                  <b>{p.name}</b>
                  <span>{sub}</span>
                </div>
              </div>
              <PctRing pct={m ? m.percent : null} size={28} s={s} />
            </div>
          )
        })}
        {compact && list.length > 1 && navBtn(1)}
        {loading && <span className="muted">carregando…</span>}
        <span className="sp" />
        {refreshBtn('btn')}
        {modeBtn('btn')}
        {gearBtn('btn')}
      </div>
    )
  }

  // ------------------------------------------------------------ mini (lista compacta)
  if (mini) {
    return (
      <div className="card mini" ref={card} onMouseDown={drag}>
        <div className="mhead">
          <span className="mtitle">Limites</span>
          <div className="mtools">
            {modeBtn('btn xs')}
            {gearBtn('btn xs')}
            {refreshBtn('btn xs')}
            {closeBtn('btn xs')}
          </div>
        </div>
        {loading && <span className="muted">carregando…</span>}
        {list.map((p) => {
          const m = primaryMeter(p)
          return (
            <div key={p.id} className={`mrow ${p.state === 'offline' ? 'off' : ''} ${p.state === 'stale' ? 'stale' : ''}`} title={summary(p)}>
              <ProviderIcon id={p.id} mono={s.mono_icons} />
              <span className="mrname">{p.name}</span>
              <PctRing pct={m ? m.percent : null} size={26} s={s} />
            </div>
          )
        })}
      </div>
    )
  }

  // ------------------------------------------------------------ normal
  const updated = Math.max(0, ...list.filter((p) => p.state !== 'offline').map((p) => p.updated_ms))
  return (
    <div className="card" ref={card} onMouseDown={drag}>
      <div className="head">
        <span className="title">Limites de uso</span>
        <span className="sp" />
        {modeBtn('btn')}
        {gearBtn('btn')}
        <span className="gap" />
        {closeBtn('btn')}
      </div>

      <div className="plist">
        {loading && <div className="empty">Carregando os limites…</div>}
        {list.map((p) => {
          const m = primaryMeter(p)
          const off = p.state === 'offline'
          const open = openId === p.id
          const metersShown = p.meters.filter((x) => s.show_models || !x.minor)
          return (
            <div key={p.id} className={`prov ${open ? 'open' : ''} ${p.state === 'stale' ? 'stale' : ''}`}>
              <button className="prow" aria-expanded={open} onClick={() => setOpenId(open ? null : p.id)} title={summary(p)}>
                <ProviderIcon id={p.id} size={15} mono={s.mono_icons} />
                <span className="pname">{p.name}</span>
                {off ? (
                  <span className="pmsg">indisponível</span>
                ) : (
                  <PctRing pct={m ? m.percent : null} size={34} s={s} />
                )}
                {p.state === 'error' && <span className="dot" role="img" aria-label={p.error ?? 'Falha ao atualizar'} />}
                <svg className="chev" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><path d="M4 6.5l4 4 4-4" /></svg>
              </button>

              {open && (
                <div className="pbody">
                  {p.plan && <span className="plan">{p.plan}</span>}
                  {off && <div className="pnote">{p.error}</div>}
                  {metersShown.map((x) => <MeterRow key={x.label} m={x} now={now} s={s} />)}
                  {p.id === 'claude' && (p.projects ?? []).length > 0 && (
                    <Fold id={`${p.id}.projects`} title="Projetos · 7 dias">
                      {p.projects.map((x) => (
                        <div key={x.name} className="pproj" title={`${x.name}: ${x.tokens_7d.toLocaleString('pt-BR')} tokens em 7 dias`}>
                          <span className="ppname">{x.name}</span>
                          {x.tokens_today > 0 && <span className="pptoday">hoje {fmtTokens(x.tokens_today)}</span>}
                          <span className="ppnum">{fmtTokens(x.tokens_7d)}</span>
                        </div>
                      ))}
                    </Fold>
                  )}
                  {(p.sessions ?? []).length > 0 && (
                    <Fold id={`${p.id}.sessions`} title="Sessões · 7 dias">
                      {p.sessions.map((x, i) => {
                        const amount = x.tokens != null ? `${fmtTokens(x.tokens)} tokens` : x.turns != null ? `${x.turns} ${x.turns === 1 ? 'mensagem' : 'mensagens'}` : ''
                        const where = x.project ? x.project.split(/[\\/]/).filter(Boolean).pop() : ''
                        return (
                          <div key={i} className="psess" title={`${x.title}${x.project ? ` · ${x.project}` : ''}`}>
                            <div className="psmain">
                              <span className="ppname">{x.title}</span>
                              <span className="ppnum">{amount}</span>
                            </div>
                            <div className="pssub">{[where && where !== x.title ? where : '', ago(x.updated_ms, now)].filter(Boolean).join(' · ')}</div>
                          </div>
                        )
                      })}
                    </Fold>
                  )}
                  {!off && p.history.length >= 3 && <Trend pts={p.history} now={now} />}
                  {!off && metersShown.length === 0 && <div className="pnote">Sem limite a exibir neste plano.</div>}
                  {s.show_extra && p.extra && (
                    <div className="meter">
                      <div className="mtop">
                        <div className="mname">
                          <b>Uso extra · mês</b>
                          <span>{p.extra.currency} {p.extra.used.toLocaleString('pt-BR', { minimumFractionDigits: 2 })} de {p.extra.limit.toLocaleString('pt-BR', { minimumFractionDigits: 2 })}</span>
                        </div>
                        <PctRing pct={p.extra.percent} size={46} s={s} big />
                      </div>
                    </div>
                  )}
                  {p.state === 'stale' && <div className="pnote warn">{p.error}. Última leitura {ago(p.updated_ms, now)}.</div>}
                  {p.state === 'error' && <div className="pnote warn">{p.error}. Mostrando o último valor.</div>}
                </div>
              )}
            </div>
          )
        })}
      </div>

      <div className="foot">
        <span>{updated ? `Atualizado ${ago(updated, now)}` : 'Aguardando a primeira leitura'}</span>
        {refreshBtn('btn xs')}
      </div>
    </div>
  )
}

// engrenagem de 8 dentes: polígono com dentes e vales, mais o furo central
const GEAR_POINTS = (() => {
  const pts: string[] = []
  for (let i = 0; i < 8; i++) {
    const center = (i * 45 * Math.PI) / 180
    for (const [off, r] of [[-17, 5.1], [-9, 6.9], [9, 6.9], [17, 5.1]] as const) {
      const a = center + (off * Math.PI) / 180
      pts.push(`${(8 + r * Math.sin(a)).toFixed(2)},${(8 - r * Math.cos(a)).toFixed(2)}`)
    }
  }
  return pts.join(' ')
})()

function GearIcon() {
  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
      <polygon points={GEAR_POINTS} />
      <circle cx="8" cy="8" r="2.1" />
    </svg>
  )
}

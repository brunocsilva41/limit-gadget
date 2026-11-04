import { useEffect, useLayoutEffect, useRef, useState, ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { DEFAULTS, PROVIDER_INFO, Settings, useSettings } from './shared'

type TabId = 'look' | 'prov' | 'bar' | 'alerts' | 'system'

const icon = (children: ReactNode) => (
  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {children}
  </svg>
)

const TABS: { id: TabId; label: string; icon: ReactNode }[] = [
  {
    id: 'look',
    label: 'Aparência',
    icon: icon(<><circle cx="8" cy="8" r="2.8" /><path d="M8 1.6v1.7M8 12.7v1.7M1.6 8h1.7M12.7 8h1.7M3.5 3.5l1.2 1.2M11.3 11.3l1.2 1.2M3.5 12.5l1.2-1.2M11.3 4.7l1.2-1.2" /></>),
  },
  { id: 'prov', label: 'Provedores', icon: icon(<><path d="M8 2l6 3-6 3-6-3z" /><path d="M2 8l6 3 6-3M2 11l6 3 6-3" /></>) },
  { id: 'bar', label: 'Barra', icon: icon(<><rect x="1.5" y="9" width="13" height="4.5" rx="1.5" /><path d="M4 5.5h8" /></>) },
  { id: 'alerts', label: 'Avisos', icon: icon(<><path d="M4 11V7a4 4 0 018 0v4l1 1.5H3z" /><path d="M6.7 14a1.4 1.4 0 002.6 0" /></>) },
  { id: 'system', label: 'Sistema', icon: icon(<><path d="M2 4.5h7M12 4.5h2M2 11.5h2M7 11.5h7" /><circle cx="10.5" cy="4.5" r="1.5" /><circle cx="5.5" cy="11.5" r="1.5" /></>) },
]

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h2>{title}</h2>
      <div className="group">{children}</div>
    </section>
  )
}

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="srow">
      <div className="stext"><b>{label}</b>{hint && <span>{hint}</span>}</div>
      <div className="sctl">{children}</div>
    </div>
  )
}

function Toggle({ on, set, label }: { on: boolean; set: (v: boolean) => void; label: string }) {
  return (
    <button role="switch" aria-checked={on} aria-label={label} className={`switch ${on ? 'on' : ''}`} onClick={() => set(!on)}>
      <i />
    </button>
  )
}

function Seg<T extends string | number>({ value, options, set }: { value: T; options: [T, string][]; set: (v: T) => void }) {
  return (
    <div className="seg">
      {options.map(([v, l]) => (
        <button key={String(v)} aria-pressed={v === value} className={v === value ? 'act' : ''} onClick={() => set(v)}>{l}</button>
      ))}
    </div>
  )
}

export default function SettingsView() {
  const saved = useSettings()
  const [s, setS] = useState<Settings>(DEFAULTS)
  const [ready, setReady] = useState(false)
  const [tab, setTab] = useState<TabId>('look')
  const [ind, setInd] = useState({ left: 0, width: 0 })
  const tabRefs = useRef<Partial<Record<TabId, HTMLButtonElement | null>>>({})
  const [bmsg, setBmsg] = useState('')
  const [mons, setMons] = useState<{ index: number; name: string; width: number; height: number; primary: boolean }[]>([])
  const [defUrl, setDefUrl] = useState('')
  const [uinfo, setUinfo] = useState<{ msg: string; version?: string; busy?: boolean }>({ msg: 'Ainda não verificado.' })

  // carrega uma vez e acompanha mudanças feitas pelo gadget (ex.: botão de modo)
  useEffect(() => {
    invoke<Settings>('get_settings').then((v) => { setS(v); setReady(true) })
  }, [])
  useEffect(() => { if (ready) setS(saved) }, [saved])

  useEffect(() => {
    invoke<typeof mons>('list_monitors').then(setMons).catch(() => {})
    invoke<string>('default_update_url').then(setDefUrl).catch(() => {})
  }, [])

  // o indicador deslizante acompanha a aba ativa
  useLayoutEffect(() => {
    const el = tabRefs.current[tab]
    if (el) setInd({ left: el.offsetLeft, width: el.offsetWidth })
  }, [tab, ready])

  const backup = async (cmd: 'export_backup' | 'import_backup') => {
    try {
      const path = await invoke<string>(cmd)
      setBmsg(cmd === 'export_backup' ? `Backup salvo em ${path}` : `Backup restaurado de ${path}`)
    } catch (e) {
      setBmsg(String(e))
    }
  }

  const upd = <K extends keyof Settings>(k: K, v: Settings[K]) => {
    const next = { ...s, [k]: v }
    setS(next)
    invoke('save_settings', { settings: next })
  }

  const checkUpdate = async () => {
    setUinfo({ msg: 'Verificando…', busy: true })
    try {
      await invoke('save_settings', { settings: s })
      const r = await invoke<{ available: boolean; version: string | null; current: string }>('check_update')
      setUinfo(r.available ? { msg: `Versão ${r.version} disponível (instalada: ${r.current}).`, version: r.version ?? undefined } : { msg: `Você está na versão mais recente (${r.current}).` })
    } catch (e) {
      setUinfo({ msg: String(e) })
    }
  }
  const installUpdate = async () => {
    setUinfo((u) => ({ ...u, msg: 'Baixando e instalando…', busy: true }))
    try {
      await invoke('install_update')
    } catch (e) {
      setUinfo({ msg: String(e) })
    }
  }

  const win = getCurrentWindow()
  const drag = (e: React.MouseEvent) => {
    if (e.button === 0 && e.target === e.currentTarget) win.startDragging()
  }

  if (!ready) return null
  return (
    <div className="shell">
      <div className="top" onMouseDown={drag}>
        <div className="winctl">
          <button onClick={() => win.minimize()} title="Minimizar" aria-label="Minimizar">
            <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round"><path d="M3 8h10" /></svg>
          </button>
          <button className="close" onClick={() => win.close()} title="Fechar" aria-label="Fechar configurações">
            <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
          </button>
        </div>

        <nav className="tabs" role="tablist" aria-label="Categorias das configurações">
          <span className="tind" style={{ transform: `translateX(${ind.left}px)`, width: ind.width }} />
          {TABS.map((t) => (
            <button
              key={t.id}
              ref={(el) => { tabRefs.current[t.id] = el }}
              role="tab"
              aria-selected={tab === t.id}
              className={`tab ${tab === t.id ? 'act' : ''}`}
              onClick={() => setTab(t.id)}
            >
              {t.icon}
              <span>{t.label}</span>
            </button>
          ))}
        </nav>
      </div>

      <main className="pane" key={tab} role="tabpanel">
        {tab === 'look' && (
          <>
            <Section title="Gadget">
              <Row label="Modo" hint="Mini: lista compacta. Barra: faixa sobre a barra de tarefas.">
                <Seg value={s.mode} options={[['normal', 'Normal'], ['mini', 'Mini'], ['bar', 'Barra']]} set={(v) => upd('mode', v)} />
              </Row>
              <Row label="Tema">
                <Seg value={s.theme} options={[['auto', 'Auto'], ['dark', 'Escuro'], ['light', 'Claro']]} set={(v) => upd('theme', v)} />
              </Row>
              <Row label="Opacidade" hint={`${Math.round(s.opacity * 100)}%`}>
                <input aria-label="Opacidade" type="range" min={50} max={100} value={Math.round(s.opacity * 100)} onChange={(e) => upd('opacity', Number(e.target.value) / 100)} />
              </Row>
              <Row label="Ícones sem cor" hint="Usa a cor do texto em vez da cor de cada ferramenta.">
                <Toggle label="Ícones sem cor" on={s.mono_icons} set={(v) => upd('mono_icons', v)} />
              </Row>
            </Section>
            {mons.length > 1 && (
              <Section title="Tela">
                <Row label="Monitor" hint="Onde o gadget e a barra aparecem.">
                  <Seg
                    value={s.monitor}
                    options={mons.map((m): [number, string] => [m.index, `${m.index + 1}${m.primary ? ' (principal)' : ''}`])}
                    set={(v) => upd('monitor', v)}
                  />
                </Row>
              </Section>
            )}
          </>
        )}

        {tab === 'prov' && (
          <>
            <Section title="Provedores">
              {PROVIDER_INFO.map((p) => {
                const on = s.providers.includes(p.id)
                return (
                  <Row key={p.id} label={p.name} hint={p.hint}>
                    <Toggle
                      label={p.name}
                      on={on}
                      set={(v) => {
                        const next = v ? [...s.providers, p.id] : s.providers.filter((x) => x !== p.id)
                        if (next.length === 0) return
                        upd('providers', PROVIDER_INFO.map((i) => i.id).filter((id) => next.includes(id)))
                      }}
                    />
                  </Row>
                )
              })}
            </Section>
            <Section title="Conteúdo">
              <Row label="Limites secundários" hint="Por modelo (Opus, Sonnet), chat e sugestões do Copilot.">
                <Toggle label="Limites secundários" on={s.show_models} set={(v) => upd('show_models', v)} />
              </Row>
              <Row label="Uso extra do mês" hint="Gasto adicional do Claude além do plano.">
                <Toggle label="Uso extra do mês" on={s.show_extra} set={(v) => upd('show_extra', v)} />
              </Row>
            </Section>
          </>
        )}

        {tab === 'bar' && (
          <>
            {s.mode !== 'bar' && (
              <div className="note">
                <span>Estas opções valem para o modo Barra.</span>
                <button className="sbtn primary" onClick={() => upd('mode', 'bar')}>Usar o modo Barra</button>
              </div>
            )}
            <Section title="Barra de tarefas">
              <Row label="Barra compacta" hint="Mostra um provedor por vez, com setas para alternar.">
                <Toggle label="Barra compacta" on={s.bar_worst_only} set={(v) => upd('bar_worst_only', v)} />
              </Row>
              <Row label="Fixar na bandeja" hint="Desligado, você arrasta a barra pela barra de tarefas (ou use o pino).">
                <Toggle label="Fixar na bandeja" on={s.bar_auto} set={(v) => invoke('bar_pin', { pinned: v })} />
              </Row>
              {!s.bar_auto && (
                <Row label="Posição" hint="Distância até a borda direita da tela.">
                  <span className="val">{s.bar_offset}px</span>
                  <input aria-label="Posição na barra" type="range" min={0} max={1400} step={10} value={s.bar_offset} onChange={(e) => upd('bar_offset', Number(e.target.value))} />
                </Row>
              )}
              <Row label="Cor da barra de tarefas" hint="A barra copia a cor real da barra de tarefas.">
                <Toggle label="Cor da barra de tarefas" on={s.bar_match_taskbar} set={(v) => upd('bar_match_taskbar', v)} />
              </Row>
            </Section>
            <Section title="Janela">
              <Row label="Sempre à frente" hint="Fica acima de tudo, inclusive da barra de tarefas.">
                <Toggle label="Sempre à frente" on={s.always_on_top} set={(v) => upd('always_on_top', v)} />
              </Row>
              <Row label="Fechar mantém na barra" hint="O X minimiza para a barra de tarefas em vez de sair.">
                <Toggle label="Fechar mantém na barra" on={s.close_to_bar} set={(v) => upd('close_to_bar', v)} />
              </Row>
              <Row label="Lembrar posição" hint="Reabre onde você deixou.">
                <Toggle label="Lembrar posição" on={s.remember_pos} set={(v) => upd('remember_pos', v)} />
              </Row>
            </Section>
          </>
        )}

        {tab === 'alerts' && (
          <>
            <Section title="Cores de alerta">
              <Row label="Atenção a partir de" hint="O anel fica âmbar.">
                <span className="val">{s.warn_at}%</span>
                <input aria-label="Limite de atenção" type="range" min={30} max={95} step={5} value={s.warn_at} onChange={(e) => { const v = Number(e.target.value); upd('warn_at', v); if (v >= s.crit_at) upd('crit_at', Math.min(100, v + 5)) }} />
              </Row>
              <Row label="Crítico a partir de" hint="O anel fica vermelho.">
                <span className="val">{s.crit_at}%</span>
                <input aria-label="Limite crítico" type="range" min={35} max={100} step={5} value={s.crit_at} onChange={(e) => { const v = Number(e.target.value); upd('crit_at', Math.max(v, s.warn_at + 5)) }} />
              </Row>
            </Section>
            <Section title="Notificações">
              <Row label="Avisos de limite" hint="Ao passar dos níveis de atenção e crítico e quando o limite renovar.">
                <Toggle label="Avisos de limite" on={s.notify} set={(v) => upd('notify', v)} />
              </Row>
              <Row label="Meta de consumo por dia" hint="Avisa quando um limite de semana ou mês sobe mais que isso em um dia.">
                <Seg value={s.daily_budget_pct} options={[[0, 'Desligada'], [5, '5%'], [10, '10%'], [15, '15%'], [20, '20%']]} set={(v) => upd('daily_budget_pct', v)} />
              </Row>
              <Row label="Resumo da semana" hint="Na segunda-feira, a partir das 9h, mostra o pico de cada provedor.">
                <Toggle label="Resumo da semana" on={s.weekly_summary} set={(v) => upd('weekly_summary', v)} />
              </Row>
            </Section>
          </>
        )}

        {tab === 'system' && (
          <>
            <Section title="Geral">
              <Row label="Consultar a cada" hint="Frequência de atualização dos limites.">
                <Seg value={s.interval_secs} options={[[15, '15s'], [30, '30s'], [60, '1min'], [120, '2min']]} set={(v) => upd('interval_secs', v)} />
              </Row>
              <Row label="Iniciar com o Windows">
                <Toggle label="Iniciar com o Windows" on={s.autostart} set={(v) => upd('autostart', v)} />
              </Row>
            </Section>
            <Section title="Atualizações">
              <Row label="Endereço" hint={defUrl ? 'Em branco, usa as releases do GitHub deste projeto.' : 'Link do latest.json publicado a cada versão.'}>
                <input
                  className="txt"
                  aria-label="Endereço das atualizações"
                  type="text"
                  spellCheck={false}
                  value={s.update_url}
                  placeholder={defUrl || 'https://exemplo.com/latest.json'}
                  onChange={(e) => setS({ ...s, update_url: e.target.value })}
                  onBlur={() => invoke('save_settings', { settings: s })}
                />
              </Row>
              <Row label="Verificar sozinho" hint="A cada 6 horas, com aviso quando houver versão nova.">
                <Toggle label="Verificar sozinho" on={s.auto_update} set={(v) => upd('auto_update', v)} />
              </Row>
              <Row label="Verificação manual" hint={uinfo.msg}>
                <button className="sbtn" disabled={uinfo.busy} onClick={checkUpdate}>Verificar</button>
                {uinfo.version && <button className="sbtn primary" disabled={uinfo.busy} onClick={installUpdate}>Instalar {uinfo.version}</button>}
              </Row>
            </Section>
            <Section title="Dados">
              <Row label="Backup" hint={bmsg || 'Salva e restaura em Documentos\\LimitGadget-backup.json.'}>
                <button className="sbtn" onClick={() => backup('export_backup')}>Exportar</button>
                <button className="sbtn" onClick={() => backup('import_backup')}>Importar</button>
              </Row>
              <Row label="Restaurar padrões" hint="Volta todas as opções ao início (a posição da janela é mantida).">
                <button className="sbtn" onClick={() => { const d = { ...DEFAULTS, pos: s.pos }; setS(d); invoke('save_settings', { settings: d }) }}>Restaurar</button>
              </Row>
            </Section>
            <p className="about">LimitGadget 0.5 · Atalho global Ctrl+Alt+L mostra ou oculta o gadget</p>
          </>
        )}
      </main>
    </div>
  )
}

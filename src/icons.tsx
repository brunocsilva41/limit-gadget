/** Marcas simples de cada ferramenta (traço fino), na cor própria de cada uma. */
export function ProviderIcon({ id, size = 14, mono = false }: { id: string; size?: number; mono?: boolean }) {
  const base = {
    width: size,
    height: size,
    viewBox: '0 0 16 16',
    fill: 'none',
    strokeWidth: 1.6,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    'aria-hidden': true,
    className: 'picon',
  }
  switch (id) {
    case 'claude':
      return (
        <svg {...base} stroke={mono ? 'currentColor' : '#d97757'}>
          <path d="M8 1.8v12.4M1.8 8h12.4M3.6 3.6l8.8 8.8M12.4 3.6l-8.8 8.8" />
        </svg>
      )
    case 'codex':
      return (
        <svg {...base} stroke={mono ? 'currentColor' : '#10a37f'}>
          <path d="M8 1.6l5.5 3.2v6.4L8 14.4l-5.5-3.2V4.8z" />
          <circle cx="8" cy="8" r="1.9" />
        </svg>
      )
    case 'copilot':
      return (
        <svg {...base} stroke={mono ? 'currentColor' : '#a371f7'}>
          <rect x="1.8" y="5" width="5.2" height="6" rx="2.4" />
          <rect x="9" y="5" width="5.2" height="6" rx="2.4" />
          <path d="M7 8h2" />
        </svg>
      )
    default:
      return (
        <svg {...base} stroke={mono ? 'currentColor' : '#4285f4'}>
          <path d="M2.4 13.6L8 2.4l5.6 11.2M5 10.2h6" />
        </svg>
      )
  }
}

export function PinIcon({ filled }: { filled: boolean }) {
  return (
    <svg viewBox="0 0 16 16" fill={filled ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M10 2.2l3.8 3.8-2.4.9-2.3 2.3.2 3-1 1-2.7-2.7-3.2 3.2M5.6 5.4l1 1" />
    </svg>
  )
}

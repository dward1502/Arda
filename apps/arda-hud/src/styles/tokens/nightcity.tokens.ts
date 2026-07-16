// sigil: REPAIR
export const nightcityTokens = {
  colors: {
    orange: '#ff6b35',
    orangeDim: 'rgba(255, 107, 53, 0.4)',
    orangeGlow: 'rgba(255, 107, 53, 0.13)',
    purple: '#9b59b6',
    purpleDim: 'rgba(155, 89, 182, 0.4)',
    purpleGlow: 'rgba(155, 89, 182, 0.07)',
    violet: '#e056fd',
    violetDim: 'rgba(224, 86, 253, 0.27)',
    white: '#e8e8f0',
    whiteDim: 'rgba(232, 232, 240, 0.4)',
    alert: '#ff3333',
    alertGlow: 'rgba(255, 51, 51, 0.13)',
    gold: '#f9ca24',
    goldGlow: 'rgba(249, 202, 36, 0.13)',
    bg: '#0a0010',
    bg2: '#0f0018',
    bg3: '#140020',
  },
  fonts: {
    display: "'Orbitron', monospace",
    mono: "'Share Tech Mono', monospace",
    body: "'Rajdhani', sans-serif",
  },
} as const

export type NightcityColor = keyof typeof nightcityTokens.colors

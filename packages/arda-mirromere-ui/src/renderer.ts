import type { MirromereSurface } from './contract'

export type MirromereVisualMode = 'wave' | 'radar' | 'handoff' | 'presence' | 'research' | 'veil' | 'offline'
export type MirromereVisualTone = 'cyan' | 'amber' | 'violet' | 'mint' | 'dim'
export type MirromereTruthState = 'live' | 'stale' | 'unavailable' | 'veiled'

export interface MirromereVisualModel {
  mode: MirromereVisualMode
  tone: MirromereVisualTone
  glyph: string
  truthState: MirromereTruthState
  accent: string
  secondary: string
  urgency: MirromereSurface['accessibility']['urgency']
}

const COLOR_BY_TONE: Record<MirromereVisualTone, [string, string]> = {
  cyan: ['#59e8ff', '#2f82ff'],
  amber: ['#ffc75a', '#ff5f76'],
  violet: ['#bd8cff', '#ff61ba'],
  mint: ['#72ffc5', '#50b8ff'],
  dim: ['#506779', '#233646'],
}

export function deriveMirromereVisualModel(surface: MirromereSurface): MirromereVisualModel {
  const sceneId = surface.scene.scene_id
  let mode: MirromereVisualMode = 'wave'
  let tone: MirromereVisualTone = 'cyan'
  let glyph = '∿'
  if (sceneId === 'system.starting') {
    mode = 'radar'; tone = 'cyan'; glyph = '⋯'
  } else if (sceneId === 'system.degraded') {
    mode = 'radar'; tone = 'amber'; glyph = '△'
  } else if (sceneId === 'continuity.handoff-ready') {
    mode = 'handoff'; tone = 'violet'; glyph = '⇄'
  } else if (sceneId === 'conversation.presence') {
    mode = 'presence'; tone = 'mint'; glyph = '◉'
  } else if (sceneId === 'research.focus') {
    mode = 'research'; tone = 'violet'; glyph = '⌖'
  } else if (sceneId === 'privacy.veil') {
    mode = 'veil'; tone = 'dim'; glyph = '◫'
  } else if (sceneId === 'offline.local') {
    mode = 'offline'; tone = 'dim'; glyph = '—'
  }
  const truthState: MirromereTruthState = sceneId === 'privacy.veil'
    ? 'veiled'
    : surface.availability === 'unavailable' || surface.freshness === 'unavailable'
      ? 'unavailable'
      : surface.freshness === 'stale'
        ? 'stale'
        : 'live'
  const [accent, secondary] = COLOR_BY_TONE[tone]
  return { mode, tone, glyph, truthState, accent, secondary, urgency: surface.accessibility.urgency }
}

export function resolveMirromereMotion(
  surface: MirromereSurface,
  motionEnabled: boolean,
  prefersReducedMotion: boolean,
): boolean {
  return motionEnabled
    && !prefersReducedMotion
    && surface.accessibility.reduced_motion !== 'freeze'
}

export function isMirromereInspectAllowed(surface: MirromereSurface): boolean {
  return surface.allowed_interactions.includes('inspect_provenance')
}

function alpha(color: string, opacity: number): string {
  return `${color}${Math.round(Math.max(0, Math.min(1, opacity)) * 255).toString(16).padStart(2, '0')}`
}

function drawGrid(context: CanvasRenderingContext2D, width: number, height: number, color: string) {
  context.strokeStyle = alpha(color, 0.055)
  context.lineWidth = 1
  for (let x = 0; x <= width; x += 32) {
    context.beginPath(); context.moveTo(x, 0); context.lineTo(x, height); context.stroke()
  }
  for (let y = 0; y <= height; y += 32) {
    context.beginPath(); context.moveTo(0, y); context.lineTo(width, y); context.stroke()
  }
}

function drawWave(context: CanvasRenderingContext2D, width: number, height: number, model: MirromereVisualModel, time: number) {
  context.strokeStyle = alpha(model.accent, model.truthState === 'stale' ? 0.42 : 0.78)
  context.lineWidth = model.truthState === 'unavailable' ? 1 : 2
  context.shadowColor = model.accent
  context.shadowBlur = model.truthState === 'live' ? 14 : 3
  context.beginPath()
  for (let index = 0; index <= 160; index += 1) {
    const progress = index / 160
    const x = 30 + progress * (width - 60)
    const amplitude = model.mode === 'presence' ? 58 : 34
    const y = height / 2
      + Math.sin(progress * Math.PI * 5 + time * 1.2) * amplitude
      + Math.sin(progress * Math.PI * 17 - time * 0.4) * 8
    if (index === 0) context.moveTo(x, y)
    else context.lineTo(x, y)
  }
  context.stroke()
}

function drawRadar(context: CanvasRenderingContext2D, width: number, height: number, model: MirromereVisualModel, time: number) {
  const x = width / 2
  const y = height / 2
  const radius = Math.min(width, height) * 0.32
  context.save(); context.translate(x, y)
  context.strokeStyle = alpha(model.accent, 0.42)
  context.lineWidth = 1
  for (const scale of [0.28, 0.52, 0.76, 1]) {
    context.beginPath(); context.arc(0, 0, radius * scale, 0, Math.PI * 2); context.stroke()
  }
  for (let ray = 0; ray < 12; ray += 1) {
    const angle = ray / 12 * Math.PI * 2
    context.beginPath(); context.moveTo(0, 0); context.lineTo(Math.cos(angle) * radius, Math.sin(angle) * radius); context.stroke()
  }
  context.rotate(time * 0.35)
  const sweep = context.createLinearGradient(0, 0, radius, 0)
  sweep.addColorStop(0, alpha(model.secondary, 0.12)); sweep.addColorStop(1, alpha(model.secondary, 0.86))
  context.strokeStyle = sweep; context.lineWidth = 3
  context.beginPath(); context.moveTo(0, 0); context.lineTo(radius, 0); context.stroke()
  context.restore()
}

function drawHandoff(context: CanvasRenderingContext2D, width: number, height: number, model: MirromereVisualModel, time: number) {
  const centerY = height / 2
  const travel = (Math.sin(time * 1.1) + 1) * 0.5
  context.strokeStyle = alpha(model.accent, 0.6); context.lineWidth = 2
  context.beginPath(); context.moveTo(width * 0.2, centerY); context.lineTo(width * 0.8, centerY); context.stroke()
  for (const direction of [-1, 1]) {
    const x = direction < 0 ? width * (0.2 + travel * 0.28) : width * (0.8 - travel * 0.28)
    context.fillStyle = direction < 0 ? model.accent : model.secondary
    context.shadowColor = context.fillStyle as string; context.shadowBlur = 16
    context.beginPath(); context.arc(x, centerY + direction * 22, 6, 0, Math.PI * 2); context.fill()
  }
}

function drawVeil(context: CanvasRenderingContext2D, width: number, height: number, model: MirromereVisualModel, time: number) {
  context.strokeStyle = alpha(model.accent, 0.18); context.lineWidth = 10
  for (let index = -8; index < 18; index += 1) {
    const offset = ((time * 8) % 48)
    context.beginPath()
    context.moveTo(index * 48 + offset, height)
    context.lineTo(index * 48 + width * 0.5 + offset, 0)
    context.stroke()
  }
}

export function drawMirromereFrame(
  canvas: HTMLCanvasElement,
  surface: MirromereSurface,
  elapsedSeconds: number,
  animate: boolean,
): void {
  const context = canvas.getContext('2d')
  if (!context) return
  const model = deriveMirromereVisualModel(surface)
  const width = canvas.width
  const height = canvas.height
  const time = animate ? elapsedSeconds : 2.4
  context.clearRect(0, 0, width, height)
  const background = context.createRadialGradient(width / 2, height / 2, 12, width / 2, height / 2, width * 0.62)
  background.addColorStop(0, '#07131d'); background.addColorStop(1, '#010205')
  context.fillStyle = background; context.fillRect(0, 0, width, height)
  drawGrid(context, width, height, model.accent)
  if (model.mode === 'radar' || model.mode === 'research') drawRadar(context, width, height, model, time)
  else if (model.mode === 'handoff') drawHandoff(context, width, height, model, time)
  else if (model.mode === 'veil' || model.mode === 'offline') drawVeil(context, width, height, model, time)
  else drawWave(context, width, height, model, time)

  context.shadowBlur = 0
  context.fillStyle = alpha(model.accent, model.truthState === 'unavailable' ? 0.34 : 0.88)
  context.font = '600 34px ui-monospace, monospace'
  context.textAlign = 'center'; context.textBaseline = 'middle'
  context.fillText(model.glyph, width / 2, height / 2)
  context.fillStyle = alpha(model.secondary, 0.42)
  if (surface.transition.attention_budget > 0) {
    context.fillRect(24, height - 18, (width - 48) * Math.min(1, surface.transition.attention_budget / 3), 2)
  }
}

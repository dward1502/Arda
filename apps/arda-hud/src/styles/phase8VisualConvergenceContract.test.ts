import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

function css(path: string): string {
  return readFileSync(resolve(process.cwd(), path), 'utf8')
}

function luminance(hex: string): number {
  const channels = hex.match(/[a-f\d]{2}/gi)!.map((value) => Number.parseInt(value, 16) / 255)
    .map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4)
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
}

function contrast(foreground: string, background: string): number {
  const [light, dark] = [luminance(foreground), luminance(background)].sort((left, right) => right - left)
  return (light + 0.05) / (dark + 0.05)
}

describe('Phase 8 shared visual convergence contract', () => {
  it('defines one physical, focus, type, truth, and motion token grammar', () => {
    const tokens = css('src/styles/foundation/tokens.css')

    for (const token of [
      '--arda-console-material',
      '--arda-vector-line',
      '--arda-vector-line-strong',
      '--arda-focus-outline',
      '--arda-text-min-focused',
      '--arda-motion-fast',
      '--arda-truth-live',
      '--arda-truth-snapshot',
      '--arda-truth-projected',
      '--arda-truth-stale',
      '--arda-truth-unavailable',
      '--arda-truth-missing',
    ]) expect(tokens).toContain(token)
  })

  it('applies the shared substrate without homogenizing domain composition', () => {
    const fleet = css('src/styles/adapters/fleet.css')
    const routing = css('src/styles/adapters/routing.css')
    const continuity = css('src/styles/adapters/continuity.css')
    const governance = css('src/styles/components/modules.css')

    for (const surface of [fleet, routing, continuity, governance]) {
      expect(surface).toContain('var(--arda-console-material)')
      expect(surface).toContain('var(--arda-vector-line)')
      expect(surface).toContain('var(--arda-text-min-focused)')
    }

    expect(fleet).toContain('.fleet-focused-view__topology')
    expect(routing).toContain('.routing-focused-view__flow')
    expect(continuity).toContain('.continuity-focused-view__horizons')
    expect(governance).toContain('.governance-master-detail')
  })

  it('gives source truth a consistent corner marker, non-color cue, focus ring, and reduced-motion stop', () => {
    const convergence = css('src/styles/adapters/convergence.css')

    expect(convergence).toContain('.arda-source-corner')
    expect(convergence).toContain('[data-truth-state]::before')
    expect(convergence).toContain('[data-truth-state="missing"]')
    expect(convergence).toContain('content: "×"')
    expect(convergence).toContain(':focus-visible')
    expect(convergence).toContain('var(--arda-focus-outline)')
    expect(convergence).toContain('@media (prefers-reduced-motion: reduce)')
    expect(convergence).toMatch(/prefers-reduced-motion:[\s\S]*animation-duration: 0\.01ms/)
  })

  it('keeps every truth-state color above WCAG AA against the console background', () => {
    const tokens = css('src/styles/foundation/tokens.css')
    const truthColors = [...tokens.matchAll(/--arda-truth-[a-z]+:\s*(#[a-f\d]{6})/gi)].map((match) => match[1])

    expect(truthColors).toHaveLength(6)
    expect(Math.min(...truthColors.map((color) => contrast(color, '#02070c')))).toBeGreaterThanOrEqual(4.5)
  })

  it('keeps the command core tactile instead of applying a focused-workstation template', () => {
    const controls = css('src/styles/components/controls.css')
    expect(controls).toContain('var(--arda-console-material)')
    expect(controls).toContain('.command-core-terminal__buttons')
    expect(controls).toContain('.command-core-terminal__scope')
    expect(controls).not.toContain('routing-focused-view__flow')
  })
})

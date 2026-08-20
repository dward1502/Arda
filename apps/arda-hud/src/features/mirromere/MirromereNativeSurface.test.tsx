import { describe, expect, it, vi, afterEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ambientIdleFixture } from './fixtures'
import MirromereNativeSurface from './MirromereNativeSurface'
import type { MirromereSurface } from './types'

const drawCalls: string[] = []
vi.mock('./MirromereAperture', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./MirromereAperture')>()
  return {
    ...actual,
    drawMirromereFrame: vi.fn((canvas: HTMLCanvasElement, surface: MirromereSurface, _elapsed: number, animate: boolean) => {
      drawCalls.push(`${surface.scene.scene_id}:${animate}`)
      canvas.dataset.drawn = surface.scene.scene_id
    }),
  }
})

afterEach(() => {
  drawCalls.length = 0
  vi.restoreAllMocks()
})

describe('MirromereNativeSurface', () => {
  it('renders unavailable semantics instead of moving content to a fallback display', () => {
    render(<MirromereNativeSurface surface={null} loading={false} error="selected display unavailable" />)

    expect(screen.getByRole('status').textContent).toContain('selected display unavailable')
    expect(screen.getByText(/Mirromere unavailable/i)).toBeInTheDocument()
  })

  it('renders veiled semantics explicitly', () => {
    render(<MirromereNativeSurface surface={{
      ...ambientIdleFixture,
      source_mode: 'runtime',
      scene: { ...ambientIdleFixture.scene, scene_id: 'privacy.veil' },
    }} loading={false} error={null} />)

    expect(screen.getByText(/Privacy veil active/i)).toBeInTheDocument()
  })

  it('reuses the shared Mirromere frame drawing path and honors reduced motion', () => {
    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      value: vi.fn(() => ({ matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() })),
    })

    const { container } = render(<MirromereNativeSurface surface={{ ...ambientIdleFixture, source_mode: 'runtime' }} loading={false} error={null} motionEnabled />)

    expect(container.querySelector('canvas')?.getAttribute('data-drawn')).toBe('ambient.idle')
    expect(drawCalls).toEqual(['ambient.idle:false'])
  })
})

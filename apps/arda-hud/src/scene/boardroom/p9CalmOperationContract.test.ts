import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { parseOperatorProjection, projectionMonitorSignals } from '../../lib/operatorProjection'
import { resolveBoardroomRenderProfile } from './boardroomPerformance'
import { parseMonitorSessionWorkstationId } from './monitorSessionWorkstationRoute'

function projectionFixture(): unknown {
  return JSON.parse(readFileSync(
    resolve(process.cwd(), '../../spec/operator-projection/v1/fixtures/valid-operator-projection.json'),
    'utf8',
  ))
}

describe('P9.4 calm-operation contract', () => {
  it('keeps default projection output concise while preserving readable approval scope', () => {
    const projection = parseOperatorProjection(projectionFixture())
    const signals = projectionMonitorSignals(projection)

    expect(Object.keys(signals)).toHaveLength(7)
    expect(Object.keys(signals)).not.toContain('actions')
    expect(projection.pending_approvals[0]).toMatchObject({
      approval_id: 'approval-p9',
      run_id: 'run-p9',
      node_id: 'implement',
      scope: 'write repository files',
      status: 'pending',
    })
  })

  it('turns motion off without erasing the boardroom and pauses decorative instrument animation', () => {
    expect(resolveBoardroomRenderProfile({
      active: true,
      prefersReducedMotion: true,
      hardwareConcurrency: 16,
      deviceMemoryGb: 16,
    })).toMatchObject({
      id: 'reduced-motion',
      frameloop: 'demand',
      motionEnabled: false,
    })

    const css = readFileSync(resolve(process.cwd(), 'src/styles/scene/hud-instruments.css'), 'utf8')
    expect(css).toContain('@media (prefers-reduced-motion: reduce)')
    expect(css).toMatch(/prefers-reduced-motion:[\s\S]*animation: none/)
    expect(css).toContain('.hud-instrument__status')
    expect(css).toContain('text-transform: uppercase')
  })

  it('retains a stable recovery key after an attention or window-context shift', () => {
    expect(parseMonitorSessionWorkstationId('monitor-session:surface-session-p9'))
      .toBe('surface-session-p9')
    expect(parseMonitorSessionWorkstationId('world:district-p9')).toBeNull()
  })
})

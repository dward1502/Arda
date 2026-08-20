import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

describe('Mirromere boardroom integration', () => {
  it('mounts the proving aperture only after session and claim ownership branches', () => {
    const source = readFileSync(resolve(process.cwd(), 'src/scene/boardroom/BoardroomViewport.tsx'), 'utf8')
    const session = source.indexOf("displayMode === 'session'")
    const claim = source.indexOf("displayMode === 'claim'")
    const mirromere = source.indexOf('renderMirromere && mirromereSurface')
    expect(session).toBeGreaterThan(0)
    expect(claim).toBeGreaterThan(session)
    expect(mirromere).toBeGreaterThan(claim)
    expect(source).toContain("shouldRenderMirromereAperture(monitorSlotId, displayMode, mirromereSurface)")
  })

  it('passes the backend-owned surface from the bundle into the boardroom', () => {
    const source = readFileSync(resolve(process.cwd(), 'src/App.tsx'), 'utf8')
    expect(source).toContain('mirromereSurface={bundle?.mirromereSurface ?? null}')
  })

  it('routes Mirromere provenance inspection through backend receipts before opening workstation', () => {
    const app = readFileSync(resolve(process.cwd(), 'src/App.tsx'), 'utf8')
    const viewport = readFileSync(resolve(process.cwd(), 'src/scene/boardroom/BoardroomViewport.tsx'), 'utf8')
    expect(app).toContain('requestMirromereInteraction(')
    expect(app).toContain('onMirromereInteraction={handleMirromereInteraction}')
    expect(viewport).toContain("receipt.outcome === 'accepted' && receipt.status === 'requested'")
    expect(viewport).toContain('requestMirromereInspection(')
    expect(viewport).toContain('onInspectMirromere={props.mirromereSurface')
    expect(viewport).not.toContain('onOpenWorkstation(workstationZoneId) : undefined')
    expect(viewport).not.toContain('if (zoneId) props.onOpenWorkstation(zoneId)')
  })
})

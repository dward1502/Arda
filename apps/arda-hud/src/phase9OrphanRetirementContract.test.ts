import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const fromHud = (path: string) => resolve(process.cwd(), path)

function source(path: string): string {
  return readFileSync(fromHud(path), 'utf8')
}

describe('Phase 9 orphan retirement contract', () => {
  it('retires the empty provider routing placeholder', () => {
    expect(existsSync(fromHud('src/lib/providerRouting.ts'))).toBe(false)
    expect(source('BREAKDOWN.md')).not.toContain('src/lib/providerRouting.ts')
  })

  it('retires the disconnected legacy Fleet module', () => {
    expect(existsSync(fromHud('src/components/arda/modules/fleet/FleetWorkstation.tsx'))).toBe(false)
  })

  it('retains the tested Fleet owner selected by App composition', () => {
    expect(existsSync(fromHud('src/scene/workstations/fleetWorkstationView.tsx'))).toBe(true)
    expect(existsSync(fromHud('src/scene/workstations/fleetWorkstationView.test.tsx'))).toBe(true)
    expect(source('src/App.tsx')).toContain("import { FleetFocusedWorkstationView } from './scene/workstations/fleetWorkstationView'")
    expect(source('src/App.tsx')).toContain('<FleetFocusedWorkstationView fleetViewModel={fleetViewModel}')
    expect(source('src/scene/workstations/fleetWorkstationView.tsx')).not.toContain('getFloatingWorkstationTileLayout')
    expect(source('src/lib/bundleDerivation.ts')).toContain('export function getFloatingWorkstationTileLayout')
  })
})

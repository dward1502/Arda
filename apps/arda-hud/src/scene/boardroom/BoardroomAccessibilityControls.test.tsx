import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { SceneAnchorDefinition, WorkstationManifestDefinition } from '../systems/runtimeTypes'
import BoardroomAccessibilityControls from './BoardroomAccessibilityControls'
import { ambientIdleFixture } from '../../features/mirromere/fixtures'

const anchors: SceneAnchorDefinition[] = [{
  id: 'boardroom.anchor.world',
  scene: 'boardroom',
  type: 'gate',
  label: 'World view',
  zoneId: 'world',
  activationBehavior: 'transition_world',
  dataBinding: [],
}]

const workstations: WorkstationManifestDefinition[] = [{
  id: 'arda.service-health',
  title: 'Service Health',
  sourceZoneId: 'service-health',
  entryAnchorId: 'boardroom.anchor.service-health',
  moduleIds: ['service-health'],
  presentationModes: ['in_scene'],
}]

describe('BoardroomAccessibilityControls', () => {
  it('exposes keyboard-operable boardroom controls outside the WebGL canvas', () => {
    const onActivate = vi.fn()
    const onOpenWorkstation = vi.fn()
    const onOpenSettings = vi.fn()
    const onOpenHermesDashboard = vi.fn()
    const onOpenHermesCli = vi.fn()

    render(<BoardroomAccessibilityControls
      anchors={anchors}
      workstations={workstations}
      onActivate={onActivate}
      onOpenWorkstation={onOpenWorkstation}
      onOpenSettings={onOpenSettings}
      onOpenHermesDashboard={onOpenHermesDashboard}
      onOpenHermesCli={onOpenHermesCli}
    />)

    expect(screen.getByRole('navigation', { name: 'Boardroom controls' })).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Navigate to World view' }))
    fireEvent.click(screen.getByRole('button', { name: 'Open Service Health workstation' }))
    fireEvent.click(screen.getByRole('button', { name: 'Open Hermes dashboard' }))
    fireEvent.click(screen.getByRole('button', { name: 'Open Hermes CLI' }))
    fireEvent.click(screen.getByRole('button', { name: 'Open settings' }))

    expect(onActivate).toHaveBeenCalledWith('boardroom.anchor.world')
    expect(onOpenWorkstation).toHaveBeenCalledWith('service-health')
    expect(onOpenHermesDashboard).toHaveBeenCalledOnce()
    expect(onOpenHermesCli).toHaveBeenCalledOnce()
    expect(onOpenSettings).toHaveBeenCalledOnce()
  })

  it('distinguishes scene navigation from opening a same-named workstation', () => {
    render(<BoardroomAccessibilityControls
      anchors={[{ ...anchors[0], label: 'Sovereign World Workstation' }]}
      workstations={[{ ...workstations[0], title: 'Sovereign World Workstation' }]}
      onActivate={vi.fn()}
      onOpenWorkstation={vi.fn()}
      onOpenSettings={vi.fn()}
      onOpenHermesDashboard={vi.fn()}
      onOpenHermesCli={vi.fn()}
    />)

    expect(screen.getByRole('button', { name: 'Navigate to Sovereign World Workstation' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Open Sovereign World Workstation' })).toBeTruthy()
  })

  it('exposes allowlisted Mirromere inspection with the contract description', () => {
    const onInspectMirromere = vi.fn()
    render(<BoardroomAccessibilityControls
      anchors={[]}
      workstations={[]}
      onActivate={vi.fn()}
      onOpenWorkstation={vi.fn()}
      onOpenSettings={vi.fn()}
      onOpenHermesDashboard={vi.fn()}
      onOpenHermesCli={vi.fn()}
      mirromereSurface={{ ...ambientIdleFixture, source_mode: 'runtime' }}
      onInspectMirromere={onInspectMirromere}
    />)

    const button = screen.getByRole('button', { name: 'Inspect Mirromere ambient.idle' })
    expect(button.getAttribute('aria-description')).toBe(ambientIdleFixture.accessibility.description)
    expect(screen.getByText('fixture://mirromere/ambient-idle', { exact: false })).toBeTruthy()
    expect(screen.getByText('fresh')).toBeTruthy()
    fireEvent.click(button)
    expect(onInspectMirromere).toHaveBeenCalledOnce()
  })
})

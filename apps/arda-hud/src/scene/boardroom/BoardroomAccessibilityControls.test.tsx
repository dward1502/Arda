import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import BoardroomAccessibilityControls from './BoardroomAccessibilityControls'

const anchors = [{
  id: 'boardroom.anchor.world',
  scene: 'boardroom',
  type: 'portal',
  label: 'World view',
  zoneId: 'world',
  activationBehavior: 'switch_scene',
  dataBinding: null,
}]

const workstations = [{
  id: 'arda.service-health',
  title: 'Service Health',
  sourceZoneId: 'service-health',
  entryAnchorId: 'boardroom.anchor.service-health',
  moduleIds: ['service-health'],
  presentationModes: ['scene_overlay'],
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
    fireEvent.click(screen.getByRole('button', { name: 'Open World view' }))
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
})

import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import SettingsModule from './SettingsModule'

const routingAssignment = {
  slot: 'monitor_2',
  label: 'Routing and Communication',
  sourceZoneId: 'routing_and_comms',
  componentId: 'routing-surface',
  visualization: {
    preset_id: 'routes' as const,
    config: { density: 'medium' as const, timespan_minutes: 15, alert_threshold: null },
  },
}

describe('SettingsModule boardroom visualization profiles', () => {
  it('offers only compatible presets and emits a live visualization update', () => {
    const onUpdateVisualization = vi.fn()
    render(
      <SettingsModule
        theme="cyberpunk"
        editMode={false}
        viewMode="boardroom"
        themeOptions={[{ id: 'cyberpunk', label: 'Cyberpunk' }]}
        monitorAssignments={[routingAssignment]}
        futureDomains={[]}
        configWalkthrough={null}
        rootPath={null}
        onUpdateVisualization={onUpdateVisualization}
        onToggleEditMode={() => undefined}
      />,
    )

    const select = screen.getByLabelText('Visualization for monitor_2')
    expect(Array.from((select as HTMLSelectElement).options).map((option) => option.value)).toEqual([
      'standby', 'topology', 'routes', 'pulse',
    ])
    fireEvent.change(select, { target: { value: 'topology' } })
    expect(onUpdateVisualization).toHaveBeenCalledWith('monitor_2', {
      preset_id: 'topology',
      config: routingAssignment.visualization.config,
    })
    expect(screen.getByText('Routes · medium · 15m')).toBeTruthy()
  })

  it('supports recoverable export, import, and reset actions', () => {
    const onExportProfile = vi.fn(() => '{"profile":true}')
    const onImportProfile = vi.fn(() => ({ ok: true, message: 'Imported 8 boardroom slots' }))
    const onResetProfile = vi.fn()
    render(
      <SettingsModule
        theme="cyberpunk"
        editMode={false}
        viewMode="boardroom"
        themeOptions={[]}
        monitorAssignments={[routingAssignment]}
        futureDomains={[]}
        configWalkthrough={null}
        rootPath={null}
        onExportProfile={onExportProfile}
        onImportProfile={onImportProfile}
        onResetProfile={onResetProfile}
        onToggleEditMode={() => undefined}
      />,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Export profile' }))
    const profileText = screen.getByLabelText('Boardroom profile JSON') as HTMLTextAreaElement
    expect(profileText.value).toBe('{"profile":true}')
    fireEvent.click(screen.getByRole('button', { name: 'Import profile' }))
    expect(onImportProfile).toHaveBeenCalledWith('{"profile":true}')
    expect(screen.getByText('Imported 8 boardroom slots')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Reset profile' }))
    expect(onResetProfile).toHaveBeenCalledOnce()
  })
})

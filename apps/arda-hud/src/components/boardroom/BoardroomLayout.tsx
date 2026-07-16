export interface BoardroomLayoutProps {
  zones?: BoardroomZone[]
}

export interface BoardroomZone {
  id: string
  label: string
  kind: string
  interaction: string
  binding?: string
  slotId?: string
  detail?: string
  primary?: boolean
}

const DEFAULT_ZONES: BoardroomZone[] = [
  {
    id: 'boardroom.monitor.left',
    label: 'Monitor 01',
    kind: 'upper_monitor',
    interaction: 'open_workstation',
    slotId: 'monitor_left_1',
  },
  {
    id: 'boardroom.monitor.center_left',
    label: 'Monitor 02',
    kind: 'upper_monitor',
    interaction: 'open_workstation',
    slotId: 'monitor_left_2',
  },
  {
    id: 'boardroom.monitor.center_right',
    label: 'Monitor 03',
    kind: 'upper_monitor',
    interaction: 'open_workstation',
    slotId: 'monitor_left_3',
  },
  {
    id: 'boardroom.monitor.right',
    label: 'Monitor 04',
    kind: 'upper_monitor',
    interaction: 'open_workstation',
    slotId: 'monitor_left_4',
  },
  {
    id: 'boardroom.lower.left_wrap',
    label: 'Governance Console',
    kind: 'desk_surface',
    interaction: 'open_workstation',
    slotId: 'view_desk_l',
  },
  {
    id: 'boardroom.lower.left_inner',
    label: 'Systems Console',
    kind: 'desk_surface',
    interaction: 'open_workstation',
    slotId: 'view_desk_control_panel',
  },
  {
    id: 'boardroom.lower.right_inner',
    label: 'Network Console',
    kind: 'desk_surface',
    interaction: 'open_workstation',
    slotId: 'view_desk_r',
  },
  {
    id: 'boardroom.lower.right_wrap',
    label: 'Human Console',
    kind: 'desk_surface',
    interaction: 'open_workstation',
    slotId: 'view_desk_aux',
  },
  {
    id: 'boardroom.button.hermes',
    label: 'Hermes Dashboard',
    kind: 'physical_button',
    interaction: 'open_hermes',
    binding: 'human_control',
    primary: true,
    detail: 'Tools + Abilities',
  },
  {
    id: 'boardroom.button.settings',
    label: 'Settings',
    kind: 'physical_button',
    interaction: 'open_settings',
    binding: 'settings_control',
  },
  {
    id: 'boardroom.control.center',
    label: 'Control Core',
    kind: 'control_panel',
    interaction: 'open_settings',
    primary: true,
    detail: 'Command Core',
  },
  {
    id: 'boardroom.avatar.emitter',
    label: 'Avatar Emitter',
    kind: 'avatar_emitter',
    interaction: 'presence_focus',
  },
  {
    id: 'boardroom.world.window',
    label: 'Enter World',
    kind: 'world_window',
    interaction: 'transition_world',
    primary: true,
    detail: 'City Window',
  },
]

import { useMemo } from 'react'
import BoardroomZone from './BoardroomZone'

export default function BoardroomLayout({ zones }: BoardroomLayoutProps) {
  const zonesToRender = useMemo(() => zones ?? DEFAULT_ZONES, [zones])
  const monitors = zonesToRender.filter((zone) => zone.kind === 'upper_monitor')
  const lowerDesks = zonesToRender.filter((zone) => zone.kind === 'desk_surface' && Boolean(zone.slotId))
  const controls = zonesToRender.filter((zone) => zone.kind === 'control_panel' || zone.kind === 'physical_button')
  const avatarAndWorld = zonesToRender.filter((zone) => zone.kind === 'avatar_emitter' || zone.kind === 'world_window')

  return (
    <div className="boardroom-layout">
      <div className="boardroom-layout__row boardroom-layout__row--monitors">
        {monitors.map((zone) => (
          <BoardroomZone key={zone.id} zone={zone} />
        ))}
      </div>
      <div className="boardroom-layout__row boardroom-layout__row--lower">
        {lowerDesks.map((zone) => (
          <BoardroomZone key={zone.id} zone={zone} />
        ))}
      </div>
      <div className="boardroom-layout__row boardroom-layout__row--control">
        {controls.map((zone) => (
          <BoardroomZone key={zone.id} zone={zone} />
        ))}
      </div>
      <div className="boardroom-layout__row">
        {avatarAndWorld.map((zone) => (
          <BoardroomZone key={zone.id} zone={zone} />
        ))}
      </div>
    </div>
  )
}

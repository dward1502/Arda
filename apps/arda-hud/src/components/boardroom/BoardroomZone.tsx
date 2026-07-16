import '../styles/scene/boardroom.css'
import type { BoardroomZone } from './BoardroomLayout'

export interface BoardroomZoneProps {
  zone: BoardroomZone
}

export default function BoardroomZone({ zone }: BoardroomZoneProps) {
  return (
    <div
      className={
        'boardroom-zone' +
        (zone.primary ? ' boardroom-zone--primary' : '') +
        (zone.kind === 'upper_monitor' ? ' boardroom-zone--monitor' : '') +
        (zone.kind === 'desk_surface' ? ' boardroom-zone--desk' : '') +
        (zone.kind === 'world_window' ? ' boardroom-zone--portal' : '')
      }
    >
      <div className="boardroom-zone__header">
        <div>
          <div className="boardroom-zone__title">{zone.label}</div>
          {zone.detail ? <div className="boardroom-zone__detail">{zone.detail}</div> : null}
        </div>
      </div>
      <div className="boardroom-zone__body">
        <div>Zone: {zone.id}</div>
        {zone.slotId ? <div>Slot: {zone.slotId}</div> : null}
        {zone.binding ? <div>Binding: {zone.binding}</div> : null}
      </div>
      <div className="boardroom-zone__meta">
        <span>{zone.kind.replaceAll('_', ' ')}</span>
        <span>·</span>
        <span>{zone.interaction.replaceAll('_', ' ')}</span>
      </div>
    </div>
  )
}

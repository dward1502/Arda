import type { SceneAnchorDefinition, WorkstationManifestDefinition } from '../systems/runtimeTypes'

interface BoardroomAccessibilityControlsProps {
  anchors: SceneAnchorDefinition[]
  workstations: WorkstationManifestDefinition[]
  onActivate: (anchorId: string) => void
  onOpenWorkstation: (zoneId: string) => void
  onOpenHermesDashboard: () => void
  onOpenHermesCli: () => void
  onOpenSettings: () => void
}

export default function BoardroomAccessibilityControls({
  anchors,
  workstations,
  onActivate,
  onOpenWorkstation,
  onOpenHermesDashboard,
  onOpenHermesCli,
  onOpenSettings,
}: BoardroomAccessibilityControlsProps) {
  const accessibleAnchors = anchors.filter((anchor) => anchor.label.trim().length > 0)
  const accessibleWorkstations = workstations.filter(
    (workstation, index, collection) => collection.findIndex(
      (candidate) => candidate.sourceZoneId === workstation.sourceZoneId,
    ) === index,
  )

  return (
    <nav className="boardroom-accessibility-controls" aria-label="Boardroom controls">
      <h2>Boardroom controls</h2>
      <ul>
        {accessibleAnchors.map((anchor) => (
          <li key={anchor.id}>
            <button type="button" onClick={() => onActivate(anchor.id)}>
              Navigate to {anchor.label}
            </button>
          </li>
        ))}
        {accessibleWorkstations.map((workstation) => (
          <li key={workstation.id}>
            <button type="button" onClick={() => onOpenWorkstation(workstation.sourceZoneId)}>
              Open {workstation.title}{workstation.title.toLowerCase().endsWith('workstation') ? '' : ' workstation'}
            </button>
          </li>
        ))}
        <li><button type="button" onClick={onOpenHermesDashboard}>Open Hermes dashboard</button></li>
        <li><button type="button" onClick={onOpenHermesCli}>Open Hermes CLI</button></li>
        <li><button type="button" onClick={onOpenSettings}>Open settings</button></li>
      </ul>
    </nav>
  )
}

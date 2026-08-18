import type { SceneAnchorDefinition, WorkstationManifestDefinition } from '../systems/runtimeTypes'
import type { MirromereSurface } from '../../features/mirromere/types'
import { isMirromereInspectAllowed } from '../../features/mirromere/MirromereAperture'

interface BoardroomAccessibilityControlsProps {
  anchors: SceneAnchorDefinition[]
  workstations: WorkstationManifestDefinition[]
  onActivate: (anchorId: string) => void
  onOpenWorkstation: (zoneId: string) => void
  onOpenHermesDashboard: () => void
  onOpenHermesCli: () => void
  onOpenSettings: () => void
  mirromereSurface?: MirromereSurface | null
  onInspectMirromere?: () => void
}

export default function BoardroomAccessibilityControls({
  anchors,
  workstations,
  onActivate,
  onOpenWorkstation,
  onOpenHermesDashboard,
  onOpenHermesCli,
  onOpenSettings,
  mirromereSurface = null,
  onInspectMirromere,
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
        {mirromereSurface && onInspectMirromere && isMirromereInspectAllowed(mirromereSurface) ? (
          <li>
            <details>
              <summary>Mirromere {mirromereSurface.scene.scene_id} details</summary>
              <dl>
                <div><dt>Freshness</dt><dd>{mirromereSurface.freshness}</dd></div>
                <div><dt>Availability</dt><dd>{mirromereSurface.availability}</dd></div>
                <div><dt>Purpose</dt><dd>{mirromereSurface.scene.purpose}</dd></div>
              </dl>
              <ul aria-label="Mirromere evidence references">
                {mirromereSurface.evidence.map((evidence) => (
                  <li key={`${evidence.source_id}:${evidence.evidence_ref}`}>
                    {evidence.source_id}: {evidence.evidence_ref}
                  </li>
                ))}
              </ul>
            </details>
            <button
              type="button"
              onClick={onInspectMirromere}
              aria-description={mirromereSurface.accessibility.description}
            >
              Inspect Mirromere {mirromereSurface.scene.scene_id}
            </button>
          </li>
        ) : null}
        <li><button type="button" onClick={onOpenHermesDashboard}>Open Hermes dashboard</button></li>
        <li><button type="button" onClick={onOpenHermesCli}>Open Hermes CLI</button></li>
        <li><button type="button" onClick={onOpenSettings}>Open settings</button></li>
      </ul>
    </nav>
  )
}

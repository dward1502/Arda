// sigil: REPAIR
import type { ReactNode } from 'react'
import LineList from './primitives/LineList'

interface BoardroomEditOverlayProps {
  boardroomSlotAssignmentMode: string
  boardroomSlotSaveStatus: string
  boardroomSlotAssignmentMessage: string
  worldSurfaceAssignmentMode: string
  worldSurfaceSaveStatus: string
  worldSurfaceAssignmentMessage: string
  boardroomSceneSlotAssignments: Record<string, string>
  sectionTitles: Array<{
    title: string
    status: string
  }>
  moduleOrder: string[]
  moduleTitles: Record<string, string>
  onSlotAssignmentChange: (slotId: string, value: string) => void
  sceneSlotAssignmentOptions: Array<{
    id: string
    label: string
  }>
}

export default function BoardroomEditOverlay({
  boardroomSlotAssignmentMode,
  boardroomSlotSaveStatus,
  boardroomSlotAssignmentMessage,
  worldSurfaceAssignmentMode,
  worldSurfaceSaveStatus,
  worldSurfaceAssignmentMessage,
  boardroomSceneSlotAssignments,
  sectionTitles,
  moduleOrder,
  moduleTitles,
  onSlotAssignmentChange,
  sceneSlotAssignmentOptions,
}: BoardroomEditOverlayProps) {
  return (
    <section className="boardroom-edit-overlay">
      <div className="boardroom-edit-console">
        <div className="boardroom-edit-console__header">
          <span className="boardroom-edit-console__eyebrow">Operator Edit</span>
          <strong className="boardroom-edit-console__title">Scene Slot Assignment</strong>
          <span className="module-subtitle">
            Persistence: {boardroomSlotAssignmentMode} / {boardroomSlotSaveStatus} — {boardroomSlotAssignmentMessage}
          </span>
          <span className="module-subtitle">
            World surfaces: {worldSurfaceAssignmentMode} / {worldSurfaceSaveStatus} — {worldSurfaceAssignmentMessage}
          </span>
        </div>
        <div className="monitor-config">
          {sectionTitles.map((section)=>
              <label className="monitor-config__row" key={section.title}>
                <span className="monitor-config__label">{section.title}</span>
                <span className="monitor-config__status">{section.status}</span>
              </label>
          )}
        </div>
        <div className="boardroom-edit-console__lists">
          <div>
            <div className="module-subtitle">Section Focus</div>
            <LineList
              items={sectionTitles.slice(0, 8).map((section) => ({ label: section.title, value: section.status }))}
            />
          </div>
          <div>
            <div className="module-subtitle">Panel Order</div>
            <LineList
              items={moduleOrder.map((moduleId, index) => ({ label: `${index + 1}. ${moduleTitles[moduleId] ?? moduleId}`, value: moduleId }))}
            />
          </div>
        </div>
      </div>
    </section>
  )
}

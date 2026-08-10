import { Canvas } from '@react-three/fiber'
import type { MonitorSurfaceSessionRecord } from '../../lib/monitorSurfaceContract'
import { BoardroomApertureSurface } from './BoardroomApertureSurface'

interface MonitorSessionWorkstationProps {
  sessionId: string
  record: MonitorSurfaceSessionRecord | null
  rootPath: string | null
}

export default function MonitorSessionWorkstation({ sessionId, record, rootPath }: MonitorSessionWorkstationProps) {
  if (!record) {
    return (
      <main className="monitor-session-workstation monitor-session-workstation--unavailable">
        <h1>Monitor session unavailable</h1>
        <p>{sessionId}</p>
        <p>The session was released, expired, or is not present in the authoritative registry.</p>
      </main>
    )
  }

  const title = 'title' in record.content && typeof record.content.title === 'string'
    ? record.content.title
    : `${record.owner} monitor session`

  return (
    <main className="monitor-session-workstation" data-session-id={record.surface_session_id}>
      <header className="monitor-session-workstation__header">
        <div>
          <span className="monitor-session-workstation__eyebrow">{record.slot_id}</span>
          <h1>{title}</h1>
        </div>
        <div className="monitor-session-workstation__ownership" aria-label="Session ownership">
          <strong>{record.owner} · r{record.revision}</strong>
          <span>{record.surface_session_id}</span>
        </div>
      </header>
      <section className="monitor-session-workstation__surface" aria-label="Live monitor session content">
        <Canvas orthographic camera={{ position: [0, 0, 10], zoom: 70 }} dpr={[1, 1.5]}>
          <ambientLight intensity={1.2} />
          <BoardroomApertureSurface
            zoneId={record.slot_id}
            previewMode="monitor_surface"
            size={[12, 7.2, 1]}
            model={{
              title,
              eyebrow: `${record.slot_id} · ${record.owner}`,
              status: 'nominal',
              tone: 'cyan',
              glyph: '◇',
              preset: 'standby',
              nodes: [],
              links: [],
              rings: [],
              source: {
                freshness: 'fresh',
                sourceId: record.surface_session_id,
                sourcePaths: [],
                observedAtUtc: record.updated_at_utc,
              },
            }}
            descriptor={record.content}
            rootPath={rootPath}
            motionEnabled
            active
            onActivate={() => undefined}
          />
        </Canvas>
      </section>
    </main>
  )
}

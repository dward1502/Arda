import type { MonitorSurfaceSessionRecord } from '../../lib/monitorSurfaceContract'
import type { MonitorRecordsBySlot } from '../../lib/monitorSurfaceRegistryBridge'
import type { WindowConfig } from '../../utils/multiWindow'

const MONITOR_SESSION_WORKSTATION_PREFIX = 'monitor-session:'

export function parseMonitorSessionWorkstationId(workstationId: string | null): string | null {
  if (!workstationId?.startsWith(MONITOR_SESSION_WORKSTATION_PREFIX)) return null
  const sessionId = workstationId.slice(MONITOR_SESSION_WORKSTATION_PREFIX.length)
  return sessionId.length > 0 ? sessionId : null
}

export function createMonitorSessionWindowConfig(record: MonitorSurfaceSessionRecord): WindowConfig {
  const sessionId = record.surface_session_id
  return {
    id: `monitor-workstation-${sessionId}`,
    title: `ARDA ${record.slot_id} · ${record.owner}`,
    subtitle: sessionId,
    windowRole: 'workstation',
    workstationId: `${MONITOR_SESSION_WORKSTATION_PREFIX}${sessionId}`,
    sourceZoneId: record.slot_id,
    originAnchorId: record.slot_id,
    presentationMode: 'native_window',
    width: 1280,
    height: 800,
    position: 'center',
  }
}

export function findMonitorSessionRecord(
  records: MonitorRecordsBySlot,
  sessionId: string,
): MonitorSurfaceSessionRecord | null {
  return Object.values(records).find((record) => record?.surface_session_id === sessionId) ?? null
}

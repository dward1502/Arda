// sigil: REPAIR
import { Canvas, useFrame, useThree, type ThreeEvent } from '@react-three/fiber'
import { Environment, Html, OrbitControls, useGLTF, useTexture } from '@react-three/drei'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { Suspense, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import * as THREE from 'three'
import type { Group } from 'three'
import {
  BOARDROOM_MONITOR_SLOT_IDS,
  type BoardroomAgentClaim,
  type BoardroomMonitorSlotSource,
  type BoardroomSceneSlotId,
  type BoardroomSurfaceLayout,
  type MonitorSurfaceRequest,
} from '../../lib/boardroomSlotSettings'
import type { ArdaSourceProvenance } from '../../lib/ardaProvenance'
import type { SceneAnchorDefinition, SceneZoneDefinition, WorkstationManifestDefinition } from '../systems/runtimeTypes'
import type { FleetViewModel, WorkstationStatus } from '../workstations/viewModels'
import { getSurfaceAdapterManifest } from '../../lib/surfaceAdapterManifests'
import SceneRuntimeCard from '../systems/SceneRuntimeCard'
import { getSceneAssetByBinding, getWindowAssetUrl } from '../systems/sceneAssets'
import { DEFAULT_AGENT_PRESENCE_STATE } from '../systems/presenceState'
import type { AgentPresenceState, PresenceLedgerStatus } from '../systems/presenceTypes'
import { useSceneMaterial } from '../systems/sceneMaterials'
import BoardroomMissionCue from './BoardroomMissionCue'
import {
  BOARDROOM_CONTROL_ZONES,
  BOARDROOM_MONITOR_ZONES,
  getBoardroomSpatialZone,
  normalizeBoardroomZonePositionOverrides,
  serializeBoardroomZonePositionOverrides,
  type BoardroomPreviewMode,
  type BoardroomSpatialZone,
  type BoardroomVec3,
  type BoardroomZonePositionOverrides,
} from './boardroomSpatialLayout'
import {
  previewPresetForSource,
  previewTitleForSource,
  resolveBoardroomHudInstrument,
  type BoardroomHudInstrumentMap,
  type HudInstrumentModel,
  type HudTone,
} from './boardroomHudInstruments'

import { AvatarPresenceLayer } from './AvatarPresenceLayer'
import { BoardroomInstrumentScreen } from './BoardroomInstrumentScreen'
import { CommandCoreInstrumentScreen } from './CommandCoreInstrumentScreen'
import { LowerInstrumentScreen } from './LowerInstrumentScreen'
import { resolveLowerInstrumentRole } from './lowerInstrumentSignal'
import { UpperAmbientMonitorScreen } from './UpperAmbientMonitorScreen'
import { isUpperMonitorInteractive, resolveUpperMonitorDisplayMode } from './upperAmbientSignal'
import { MonitorOwnershipRail } from './MonitorOwnershipRail'
import { BoardroomApertureSurface } from './BoardroomApertureSurface'
import type { MonitorRecordsBySlot } from '../../lib/monitorSurfaceRegistryBridge'
import type { MonitorSurfaceSessionRecord } from '../../lib/monitorSurfaceContract'
import { parseJsonOrNull } from '../../lib/jsonParse'
import { deriveBoardroomPresenceStatusView } from './boardroomPresenceStatus'
import { resolveSceneSlotWorkstationZoneId } from '../workstations/sceneSlotWorkstationTemplates'
import {
  BOARDROOM_CAMERA_COMPOSITION,
  deriveAvatarEmitterGeometry,
} from './boardroomComposition'
import {
  BOARDROOM_COMMAND_CORE_CONTROL_BANKS,
  deriveBoardroomPhysicalControlState,
  dispatchBoardroomCommandCoreControl,
  getBoardroomPhysicalControlAction,
  resolveBoardroomPhysicalControlInteraction,
  type BoardroomPhysicalControlAction,
  type BoardroomPhysicalControlState,
} from './boardroomPhysicalControls'
import {
  resolveBoardroomRenderProfile,
  type BoardroomRenderProfile,
} from './boardroomPerformance'
import {
  formatMonitorSurfaceStream,
  resolveMonitorContractSlotId,
  resolveMonitorSurfaceOpenRequest,
  type MonitorSurfacePayloadEvent,
} from './monitorSurfaceRuntime'
import BoardroomAccessibilityControls from './BoardroomAccessibilityControls'

interface BoardroomViewportProps {
  active: boolean
  debug?: boolean
  zones: SceneZoneDefinition[]
  anchors: SceneAnchorDefinition[]
  workstations: WorkstationManifestDefinition[]
  slotAssignments: Record<string, string>
  surfaceLayouts?: Record<string, BoardroomSurfaceLayout>
  monitorSlotSources?: Record<string, BoardroomMonitorSlotSource | null>
  monitorRecordsBySlot?: MonitorRecordsBySlot
  agentClaims?: Record<string, BoardroomAgentClaim | null>
  onReleaseMonitor?: (slotId: BoardroomSceneSlotId, owner: string) => void
  onRefreshMonitor?: (slotId: BoardroomSceneSlotId, owner: string) => void
  sourceProvenance?: ArdaSourceProvenance[]
  instruments?: BoardroomHudInstrumentMap
  fleetViewModel?: FleetViewModel | null
  presenceState?: AgentPresenceState
  presenceStatus?: PresenceLedgerStatus
  rootPath?: string | null
  sceneOverlay?: ReactNode
  onActivate: (anchorId: string) => void
  onOpenWorkstation: (zoneId: string) => void
  onOpenMonitorSurface?: (request: MonitorSurfaceRequest) => void
  onOpenMonitorSession?: (record: MonitorSurfaceSessionRecord) => void
  onOpenHermesDashboard: () => void
  onOpenHermesCli: () => void
  onOpenSettings: () => void
}

function SceneAssetModel({
  binding,
  fallback,
  ...props
}: {
  binding: string
  fallback: ReactNode
  position?: [number, number, number]
  rotation?: [number, number, number]
  scale?: number | [number, number, number]
  onClick?: () => void
}) {
  const asset = getSceneAssetByBinding(binding)
  if (!asset?.glbUrl) return <>{fallback}</>
  return <LoadedSceneAsset url={asset.glbUrl} {...props} />
}

function LoadedSceneAsset({
  url,
  ...props
}: {
  url: string
  position?: [number, number, number]
  rotation?: [number, number, number]
  scale?: number | [number, number, number]
  onClick?: () => void
}) {
  const gltf = useGLTF(url)
  const scene = useMemo(() => gltf.scene.clone(true) as Group, [gltf.scene])
  return <primitive object={scene} {...props} />
}


function CyberpunkCityWindow({ url }: { url: string }) {
  const texture = useTexture(url)
  useEffect(() => {
    texture.colorSpace = THREE.SRGBColorSpace
  }, [texture])

  return (
    <group position={[0, 3.15, -4.92]} name="boardroom-reference-atmosphere">
      <mesh position={[0, 0, 0]}>
        <planeGeometry args={[13.4, 5.025]} />
        <meshBasicMaterial map={texture} toneMapped={false} transparent opacity={1} />
      </mesh>
      <mesh position={[0, 0, 0.025]}>
        <planeGeometry args={[13.4, 5.025]} />
        <meshBasicMaterial color="#7adfff" transparent opacity={0.025} blending={THREE.AdditiveBlending} />
      </mesh>
    </group>
  )
}

function BoardroomConsoleShell() {
  const deskShape = useMemo(() => {
    const shape = new THREE.Shape()
    const points: Array<[number, number]> = [
      [-5.65, 2.75], [-5.48, 0.78], [-4.2, -0.08], [-2.35, -0.38], [0, -0.48],
      [2.35, -0.38], [4.2, -0.08], [5.48, 0.78], [5.65, 2.75], [4.42, 2.5],
      [3.08, 2.12], [1.68, 1.84], [0, 1.73], [-1.68, 1.84], [-3.08, 2.12], [-4.42, 2.5],
    ]
    shape.moveTo(points[0][0], points[0][1])
    for (const [x, y] of points.slice(1)) shape.lineTo(x, y)
    shape.closePath()
    return shape
  }, [])
  const frontRail = useMemo(
    () => new THREE.CatmullRomCurve3([
      new THREE.Vector3(-4.42, 0.24, 2.5),
      new THREE.Vector3(-3.08, 0.24, 2.12),
      new THREE.Vector3(-1.68, 0.24, 1.84),
      new THREE.Vector3(0, 0.24, 1.73),
      new THREE.Vector3(1.68, 0.24, 1.84),
      new THREE.Vector3(3.08, 0.24, 2.12),
      new THREE.Vector3(4.42, 0.24, 2.5),
    ]),
    [],
  )
  const rearRail = useMemo(
    () => new THREE.CatmullRomCurve3([
      new THREE.Vector3(-5.48, 0.22, 0.78),
      new THREE.Vector3(-4.2, 0.22, -0.08),
      new THREE.Vector3(-2.35, 0.22, -0.38),
      new THREE.Vector3(0, 0.22, -0.48),
      new THREE.Vector3(2.35, 0.22, -0.38),
      new THREE.Vector3(4.2, 0.22, -0.08),
      new THREE.Vector3(5.48, 0.22, 0.78),
    ]),
    [],
  )

  return (
    <group name="boardroom-command-console-shell">
      <mesh position={[0, 0.18, 0]} rotation={[Math.PI / 2, 0, 0]} receiveShadow castShadow>
        <extrudeGeometry args={[deskShape, {
          depth: 0.36,
          bevelEnabled: true,
          bevelSegments: 3,
          bevelSize: 0.065,
          bevelThickness: 0.055,
          curveSegments: 32,
        }]} />
        <meshStandardMaterial
          color="#04080d"
          emissive="#071621"
          emissiveIntensity={0.18}
          metalness={0.88}
          roughness={0.2}
          envMapIntensity={1.35}
        />
      </mesh>
      <mesh>
        <tubeGeometry args={[frontRail, 64, 0.035, 8, false]} />
        <meshStandardMaterial color="#8bf8ff" emissive="#3eeeff" emissiveIntensity={2.4} metalness={0.5} roughness={0.2} />
      </mesh>
      <mesh>
        <tubeGeometry args={[rearRail, 64, 0.025, 8, false]} />
        <meshStandardMaterial color="#ff62bf" emissive="#ff2f9e" emissiveIntensity={1.8} metalness={0.45} roughness={0.24} />
      </mesh>
    </group>
  )
}

function setPointerCursor(active: boolean) {
  document.body.style.cursor = active ? 'pointer' : ''
}

type Vec3 = BoardroomVec3

const BOARDROOM_ZONE_POSITION_OVERRIDES_STORAGE_KEY = 'arda.boardroom.zone_positions.v3'

function localStorageOrNull(): Storage | null {
  return typeof window === 'undefined' ? null : window.localStorage
}

function readZonePositionOverrides(): BoardroomZonePositionOverrides {
  try {
    const raw = localStorageOrNull()?.getItem(BOARDROOM_ZONE_POSITION_OVERRIDES_STORAGE_KEY)
    if (!raw) return {}
    return normalizeBoardroomZonePositionOverrides(parseJsonOrNull<unknown>(raw))
  } catch {
    return {}
  }
}

function writeZonePositionOverrides(overrides: BoardroomZonePositionOverrides) {
  try {
    const normalized = normalizeBoardroomZonePositionOverrides(overrides)
    localStorageOrNull()?.setItem(BOARDROOM_ZONE_POSITION_OVERRIDES_STORAGE_KEY, JSON.stringify(normalized))
  } catch {
    // Local editing persistence is a convenience; failure should not break the scene.
  }
}

function clearZonePositionOverrides() {
  try {
    localStorageOrNull()?.removeItem(BOARDROOM_ZONE_POSITION_OVERRIDES_STORAGE_KEY)
  } catch {
    // Local editing persistence is a convenience; failure should not break the scene.
  }
}

function withPositionOverride(zone: BoardroomSpatialZone, overrides: BoardroomZonePositionOverrides): BoardroomSpatialZone {
  return overrides[zone.id] ? { ...zone, position: overrides[zone.id] } : zone
}

function InteractionPad({
  slotId,
  label,
  detail,
  position,
  rotation = [0, 0, 0],
  size,
  color = '#5defff',
  primary = false,
  showLabel = true,
  showHitbox = true,
  draggable = false,
  onMovePosition,
  onActivate,
  children,
}: {
  slotId: string
  label: string
  detail?: string
  position: [number, number, number]
  rotation?: [number, number, number]
  size: [number, number, number]
  color?: string
  primary?: boolean
  showLabel?: boolean
  showHitbox?: boolean
  draggable?: boolean
  onMovePosition?: (position: Vec3) => void
  onActivate?: () => void
  children?: ReactNode
}) {
  const dragRef = useRef<{ pointerId: number; startPoint: THREE.Vector3; basePosition: Vec3; moved: boolean } | null>(null)
  const suppressNextClickRef = useRef(false)

  const handleActivate = () => {
    onActivate?.()
  }

  const handlePointerDown = (event: ThreeEvent<PointerEvent>) => {
    if (!draggable) return
    event.stopPropagation()
    const target = event.target as EventTarget & { setPointerCapture?: (pointerId: number) => void }
    target.setPointerCapture?.(event.pointerId)
    dragRef.current = {
      pointerId: event.pointerId,
      startPoint: event.point.clone(),
      basePosition: [...position],
      moved: false,
    }
    setPointerCursor(true)
  }

  const handlePointerMove = (event: ThreeEvent<PointerEvent>) => {
    if (!draggable || !dragRef.current || dragRef.current.pointerId !== event.pointerId) return
    event.stopPropagation()
    const delta = event.point.clone().sub(dragRef.current.startPoint)
    if (delta.length() > 0.015) dragRef.current.moved = true
    onMovePosition?.([
      Number((dragRef.current.basePosition[0] + delta.x).toFixed(3)),
      Number((dragRef.current.basePosition[1] + delta.y).toFixed(3)),
      Number((dragRef.current.basePosition[2] + delta.z).toFixed(3)),
    ])
  }

  const handlePointerUp = (event: ThreeEvent<PointerEvent>) => {
    if (!dragRef.current || dragRef.current.pointerId !== event.pointerId) return
    event.stopPropagation()
    const target = event.target as EventTarget & { releasePointerCapture?: (pointerId: number) => void }
    target.releasePointerCapture?.(event.pointerId)
    suppressNextClickRef.current = dragRef.current.moved
    dragRef.current = null
    setPointerCursor(false)
  }

  const handleClick = (event: ThreeEvent<MouseEvent>) => {
    if (suppressNextClickRef.current) {
      suppressNextClickRef.current = false
      event.stopPropagation()
      return
    }
    handleActivate()
  }

  return (
    <group
      position={position}
      rotation={rotation}
      userData={{ sceneSlotId: slotId }}
      onClick={onActivate ? handleClick : undefined}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerUp}
    >
      {children}
      {showHitbox ? (
        <mesh
          onPointerOver={() => setPointerCursor(true)}
          onPointerOut={() => setPointerCursor(false)}
        >
          <boxGeometry args={size} />
          <meshStandardMaterial
            color={color}
            emissive={color}
            emissiveIntensity={primary ? 0.42 : 0.2}
            transparent
            opacity={primary ? 0.22 : 0.12}
            roughness={0.35}
            metalness={0.35}
          />
        </mesh>
      ) : null}
      {showLabel ? (
        <Html center distanceFactor={primary ? 7 : 8.5}>
          <button
            type="button"
            className={`scene-anchor-label${primary ? ' scene-anchor-label--primary' : ''}`}
            onClick={handleActivate}
          >
            {label}
            {detail ? <span>{detail}</span> : null}
          </button>
        </Html>
      ) : null}
    </group>
  )
}


function getSlotAssignment(
  workstations: WorkstationManifestDefinition[],
  slotAssignments: Record<string, string>,
  slot: BoardroomSpatialZone,
): WorkstationManifestDefinition | null {
  const assignedZoneId = slot.assignmentSlotId ? slotAssignments[slot.assignmentSlotId] : null
  if (assignedZoneId) {
    return workstations.find((workstation) => workstation.sourceZoneId === assignedZoneId) ?? null
  }
  return typeof slot.assignmentIndex === 'number' ? workstations[slot.assignmentIndex] ?? null : null
}

function getSlotDetail(assignment: WorkstationManifestDefinition | null): string {
  return assignment ? 'Open Workstation' : 'Placeholder'
}

function getSlotWorkstationZoneId(
  slot: BoardroomSpatialZone,
  assignment: WorkstationManifestDefinition | null,
): string {
  return resolveSceneSlotWorkstationZoneId(slot.assignmentSlotId, slot.id, assignment?.sourceZoneId)
}

export function resolveMonitorFocus(
  slotId: string,
  slotAssignments: Record<string, string>,
  monitorSlotSources: Record<string, BoardroomMonitorSlotSource | null>,
  surfaceLayouts: Record<string, BoardroomSurfaceLayout>,
  agentClaims: Record<string, BoardroomAgentClaim | null>,
  _nowUtc: string,
): { sourceZoneId: string | null; focusMode: string; hasActiveClaim: boolean } | null {
  if (!BOARDROOM_MONITOR_SLOT_IDS.includes(slotId as typeof BOARDROOM_MONITOR_SLOT_IDS[number])) return null
  const source = monitorSlotSources[slotId] ?? null
  const activeClaim = source?.claim ?? agentClaims[slotId] ?? null
  if (activeClaim && source?.active) {
    const layout = surfaceLayouts[slotId]
    const focusMode = layout?.focus.mode ?? 'remote_preview'
    return {
      sourceZoneId: activeClaim.payload_binding,
      focusMode,
      hasActiveClaim: true,
    }
  }
  const persistedSourceZoneId = slotAssignments[slotId] ?? null
  const layout = surfaceLayouts[slotId]
  const fallbackLayout = layout ?? createDefaultSurfaceLayoutForMonitor(slotId, persistedSourceZoneId)
  return {
    sourceZoneId: persistedSourceZoneId,
    focusMode: fallbackLayout.focus.mode,
    hasActiveClaim: false,
  }
}

export function shouldRenderActiveMonitorClaim(
  focus: { hasActiveClaim: boolean } | null,
): boolean {
  return focus?.hasActiveClaim === true
}

function createDefaultSurfaceLayoutForMonitor(slotId: string, sourceZoneId: string | null): BoardroomSurfaceLayout {
  const effectiveSource = sourceZoneId ?? slotId
  if (effectiveSource === 'hermes_runtime') {
    return {
      enabled: true,
      adapter_type: 'agent_activity',
      preview: { mode: 'agent_activity', refresh_ms: 1000, widgets: [] },
      focus: { mode: 'remote_preview', target: effectiveSource, refresh_ms: 1000 },
      embed: { url: null, allow_inline: false },
    }
  }
  return {
    enabled: true,
    adapter_type: 'component_grid',
    preview: { mode: 'component_grid', refresh_ms: 3000, widgets: [] },
    focus: { mode: 'in_scene_workstation', target: effectiveSource, refresh_ms: 1000 },
    embed: { url: null, allow_inline: false },
  }
}

function isFleetWorkstationAssignment(assignment: WorkstationManifestDefinition | null): boolean {
  if (!assignment) return false
  return assignment.sourceZoneId === 'systems_health'
    || assignment.sourceZoneId === 'routing_health'
    || assignment.sourceZoneId === 'sovereign_world'
    || assignment.moduleIds.includes('systems')
}

function formatFleetValue(value: number | string | null | undefined, fallback = 'n/a'): string {
  if (typeof value === 'number' && Number.isFinite(value)) return String(value)
  if (typeof value === 'string' && value.length > 0) return value
  return fallback
}

const BOARDROOM_WORKSTATION_ZONE_IDS = new Set(['sovereign_world', 'settings'])

function getSceneWorkstations(workstations: WorkstationManifestDefinition[]): WorkstationManifestDefinition[] {
  return workstations.filter((workstation) => !BOARDROOM_WORKSTATION_ZONE_IDS.has(workstation.sourceZoneId))
}

const BOARDROOM_DESK_ASSIGNMENTS = {
  left: 0,
  center: 1,
  right: 2,
}

const BOARDROOM_DESK_SLOT_IDS = {
  left: 'view_desk_l',
  center: 'view_desk_control_panel',
  right: 'view_desk_r',
}

const BOARDROOM_DESK_FALLBACK_IDS = {
  left: 'systems_table',
  center: 'systems_table',
  right: 'operations',
}

type BoardroomDeskRegion = keyof typeof BOARDROOM_DESK_ASSIGNMENTS

function getDeskActivationId(
  region: BoardroomDeskRegion,
  workstations: WorkstationManifestDefinition[],
): string {
  return workstations[BOARDROOM_DESK_ASSIGNMENTS[region]]?.entryAnchorId
    ?? BOARDROOM_DESK_FALLBACK_IDS[region]
    ?? BOARDROOM_DESK_SLOT_IDS[region]
}


function toneForAssignment(assignment: WorkstationManifestDefinition | null, persistedSourceZoneId?: string): HudTone {
  const source = assignment?.sourceZoneId ?? persistedSourceZoneId ?? ''
  if (source.startsWith('service_')) return 'violet'
  if (source.includes('governance')) return 'gold'
  if (source.includes('human') || source.includes('memory')) return 'mint'
  if (source.includes('planning')) return 'rose'
  return 'cyan'
}

function instrumentModelForAssignment(
  zone: BoardroomSpatialZone,
  assignment: WorkstationManifestDefinition | null,
  persistedSourceZoneId?: string,
): HudInstrumentModel {
  const sourceZoneId = assignment?.sourceZoneId ?? persistedSourceZoneId
  const serviceManifest = getSurfaceAdapterManifest(sourceZoneId)
  const tone = toneForAssignment(assignment, persistedSourceZoneId)
  const isHermes = sourceZoneId === 'hermes_runtime'
  const seed = (sourceZoneId ?? zone.id).split('').reduce((sum, char) => sum + char.charCodeAt(0), 0)
  const nodes = Array.from({ length: 9 }, (_, index) => {
    const angle = (Math.PI * 2 * index) / 9 + (seed % 7) * 0.08
    const radius = index % 3 === 0 ? 31 : index % 2 === 0 ? 42 : 52
    return {
      id: `${zone.id}-${index}`,
      x: 50 + Math.cos(angle) * radius,
      y: 50 + Math.sin(angle) * radius * 0.68,
      state: 'dim' as const,
    }
  })

  return {
    title: serviceManifest?.provider ?? assignment?.title.replace(/\s+Workstation$/, '') ?? previewTitleForSource(sourceZoneId) ?? zone.label,
    eyebrow: isHermes ? 'TERMINAL SURFACE' : serviceManifest ? 'EXTERNAL SURFACE' : 'STANDBY SCHEMATIC',
    tone: isHermes ? 'violet' : tone,
    status: isHermes || serviceManifest ? 'external' : 'offline',
    glyph: isHermes ? 'HMS' : serviceManifest ? 'EXT' : assignment?.moduleIds[0]?.slice(0, 3).toUpperCase() ?? 'NUL',
    preset: previewPresetForSource(sourceZoneId),
    nodes,
    links: [[0, 2], [2, 5], [5, 7], [1, 4], [4, 8], [3, 6]],
    rings: [22, 35, 49],
  }
}

function HudInstrumentSurface({
  zone,
  assignment,
  persistedSourceZoneId,
  instrument,
  motionEnabled,
  onActivate,
}: {
  zone: BoardroomSpatialZone
  assignment: WorkstationManifestDefinition | null
  persistedSourceZoneId?: string
  instrument?: HudInstrumentModel
  motionEnabled?: boolean
  onActivate: () => void
}) {
  const model = instrument ?? instrumentModelForAssignment(zone, assignment, persistedSourceZoneId)
  const lowerRole = resolveLowerInstrumentRole(zone.id)

  if (lowerRole) {
    return (
      <LowerInstrumentScreen
        slotId={zone.id}
        role={lowerRole}
        size={zone.size}
        model={model}
        motionEnabled={motionEnabled}
        onActivate={onActivate}
      />
    )
  }

  return (
    <BoardroomInstrumentScreen
      slotId={zone.id}
      previewMode={zone.previewMode}
      size={zone.size}
      model={model}
      onActivate={onActivate}
    />
  )
}

function FleetPreviewSurface({
  zone,
  assignment,
  fleetViewModel,
  motionEnabled,
  onActivate,
}: {
  zone: BoardroomSpatialZone
  assignment: WorkstationManifestDefinition | null
  fleetViewModel: FleetViewModel
  motionEnabled?: boolean
  onActivate: () => void
}) {
  const liveMetric = fleetViewModel.metrics.find((metric) => metric.id === 'live_targets')
  const totalMetric = fleetViewModel.metrics.find((metric) => metric.id === 'total_targets')
  const offlineMetric = fleetViewModel.metrics.find((metric) => metric.id === 'unexpected_offline')
  const fallbackModel = instrumentModelForAssignment(zone, assignment)
  const offlineCount = Number(offlineMetric?.value ?? 0)
  const model: HudInstrumentModel = {
    ...fallbackModel,
    eyebrow: 'Fleet',
    title: assignment?.title.replace(/\s+Workstation$/, '') ?? fleetViewModel.title,
    glyph: `${formatFleetValue(liveMetric?.value)}/${formatFleetValue(totalMetric?.value)}`,
    tone: offlineCount > 0 ? 'gold' : 'cyan',
    status: fleetViewModel.status === 'ok'
      ? 'nominal'
      : fleetViewModel.status === 'attention'
        ? 'watch'
        : 'offline',
  }
  const lowerRole = resolveLowerInstrumentRole(zone.id)

  if (lowerRole) {
    return (
      <LowerInstrumentScreen
        slotId={zone.id}
        role={lowerRole}
        size={zone.size}
        model={model}
        motionEnabled={motionEnabled}
        onActivate={onActivate}
      />
    )
  }

  return (
    <BoardroomInstrumentScreen
      slotId={zone.id}
      previewMode={zone.previewMode}
      size={zone.size}
      model={model}
      onActivate={onActivate}
    />
  )
}

function CommandCoreSurface({
  zone,
  onControl,
  nowInstrument,
  healthInstrument,
  routingInstrument,
}: {
  zone: BoardroomSpatialZone
  onControl: (action: BoardroomPhysicalControlAction) => void
  nowInstrument?: HudInstrumentModel
  healthInstrument?: HudInstrumentModel
  routingInstrument?: HudInstrumentModel
}) {
  const openAction = getBoardroomPhysicalControlAction('open_command_core')
  const model: HudInstrumentModel = nowInstrument ?? {
    title: 'ARDA Control',
    eyebrow: 'Command Core',
    tone: 'cyan',
    status: healthInstrument?.status ?? 'offline',
    glyph: routingInstrument?.glyph ?? 'NO DATA',
    preset: 'pulse',
    nodes: [],
    links: [],
    rings: [],
  }
  const commandPositions: Vec3[] = [
    [0.36, 0.075, -0.2],
    [0.58, 0.075, -0.2],
    [0.36, 0.075, 0.02],
    [0.58, 0.075, 0.02],
  ]
  const commandColors = ['#8cffc7', '#ff789c', '#5defff', '#b98cff']
  const utilityPositions: Vec3[] = [
    [0.34, 0.075, 0.27],
    [0.48, 0.075, 0.27],
    [0.62, 0.075, 0.27],
  ]
  const utilityColors = ['#d8e7ff', '#22d3ee', '#a855f7']

  return (
    <>
      <group position={[-0.27, 0, -0.02]}>
        <CommandCoreInstrumentScreen
          slotId={zone.id}
          size={[zone.size[0] * 0.62, zone.size[1], zone.size[2] * 0.82]}
          model={model}
          onActivate={() => onControl(openAction)}
        />
      </group>
      <group name="command-core-command-bank">
        {BOARDROOM_COMMAND_CORE_CONTROL_BANKS.command.map((actionId, index) => {
          const action = getBoardroomPhysicalControlAction(actionId)
          const state = deriveBoardroomPhysicalControlState(actionId, null)
          return (
            <group key={actionId} position={commandPositions[index]}>
              <PhysicalControlButtonSurface
                label={action.shortLabel}
                size={[0.17, 0.04, 0.17]}
                color={commandColors[index]}
                controlState={state}
                title={`${action.authority} · verify ${action.verificationPath}`}
                onClick={() => onControl(action)}
              />
            </group>
          )
        })}
      </group>
      <group name="command-core-utility-bank">
        {BOARDROOM_COMMAND_CORE_CONTROL_BANKS.utility.map((actionId, index) => {
          const action = getBoardroomPhysicalControlAction(actionId)
          const state = deriveBoardroomPhysicalControlState(actionId, null)
          return (
            <group key={actionId} position={utilityPositions[index]}>
              <PhysicalControlButtonSurface
                label={action.shortLabel}
                size={[0.12, 0.04, 0.12]}
                color={utilityColors[index]}
                controlState={state}
                title={`${action.authority} · verify ${action.verificationPath}`}
                onClick={() => onControl(action)}
              />
            </group>
          )
        })}
      </group>
    </>
  )
}

function PhysicalControlButtonSurface({
  label,
  size,
  onClick,
  color = '#22d3ee',
  controlState,
  title,
  onBlocked,
}: {
  label: string
  size: BoardroomVec3
  onClick: () => void
  color?: string
  controlState?: BoardroomPhysicalControlState
  title?: string
  onBlocked?: () => void
}) {
  const [hovered, setHovered] = useState(false)
  const [pressed, setPressed] = useState(false)
  const disabled = controlState?.disabled ?? false
  const surfaceColor = disabled ? '#26333c' : color
  const activate = () => {
    if (disabled) onBlocked?.()
    else onClick()
  }

  return (
    <group position={[0, pressed ? -0.04 : 0, 0]} name={`kinetic-control.${label.toLowerCase().split(' ').join('_')}`}>
      <mesh
        userData={{ controlLabel: label, title: controlState?.error ?? title, disabled }}
        onClick={(event) => { event.stopPropagation(); activate() }}
        onPointerDown={(event) => { event.stopPropagation(); if (!disabled) setPressed(true) }}
        onPointerUp={(event) => { event.stopPropagation(); setPressed(false) }}
        onPointerOver={(event) => { event.stopPropagation(); setHovered(true); setPointerCursor(!disabled) }}
        onPointerOut={(event) => { event.stopPropagation(); setHovered(false); setPressed(false); setPointerCursor(false) }}
      >
        <boxGeometry args={size} />
        <meshStandardMaterial
          color={surfaceColor}
          emissive={disabled ? '#4b1f2c' : color}
          emissiveIntensity={pressed ? 1.15 : hovered ? 0.78 : controlState?.state === 'attention' ? 0.62 : 0.34}
          roughness={0.2}
          metalness={0.64}
        />
      </mesh>
      <mesh position={[0, size[1] / 2 + 0.012, 0]}>
        <boxGeometry args={[size[0] * 0.5, 0.018, size[2] * 0.5]} />
        <meshBasicMaterial color={surfaceColor} transparent opacity={disabled ? 0.18 : hovered ? 0.95 : 0.62} />
      </mesh>
    </group>
  )
}

function AvatarEmitterBase({
  zone,
  presenceState,
  motionEnabled,
  rootPath,
}: {
  zone: BoardroomSpatialZone
  presenceState: AgentPresenceState
  motionEnabled: boolean
  rootPath?: string | null
}) {
  const geometry = deriveAvatarEmitterGeometry(zone.size)
  const pulseRef = useRef<THREE.Group>(null)
  const isActive = presenceState.phase !== 'idle'
  const isAlert = presenceState.scenario === 'alert' || presenceState.urgency === 'high'
  const emitterColor = isAlert ? '#ff4f9d' : isActive ? '#5defff' : zone.color

  useFrame(({ clock }) => {
    if (!pulseRef.current || !motionEnabled) return
    const elapsed = clock.getElapsedTime()
    const pulse = (Math.sin(elapsed * (isActive ? 2.4 : 1.15)) + 1) * 0.5
    pulseRef.current.scale.setScalar(0.985 + pulse * (isActive ? 0.045 : 0.018))
    pulseRef.current.rotation.y = elapsed * (isActive ? 0.22 : 0.08)
  })

  return (
    <group position={zone.position} name="arda-presence-emitter">
      <mesh position={[0, -0.02, 0]}>
        <cylinderGeometry args={[geometry.baseTopRadius, geometry.baseBottomRadius, 0.11, 12]} />
        <meshStandardMaterial color="#050b12" emissive="#102638" emissiveIntensity={0.54} roughness={0.26} metalness={0.78} />
      </mesh>
      <mesh position={[0, 0.045, 0]}>
        <cylinderGeometry args={[geometry.coreTopRadius, geometry.baseTopRadius, 0.04, 12]} />
        <meshStandardMaterial color="#0b1721" emissive={emitterColor} emissiveIntensity={0.38} roughness={0.22} metalness={0.7} />
      </mesh>
      <group ref={pulseRef}>
        <mesh position={[0, 0.07, 0]} rotation={[Math.PI / 2, 0, 0]}>
          <torusGeometry args={[geometry.ringRadius, geometry.ringTubeRadius, 10, 72]} />
          <meshStandardMaterial color={emitterColor} emissive={emitterColor} emissiveIntensity={isActive ? 2.5 : 1.1} roughness={0.14} metalness={0.35} />
        </mesh>
        {Array.from({ length: 6 }, (_, index) => {
          const angle = (index / 6) * Math.PI * 2
          return (
            <mesh
              key={index}
              position={[Math.sin(angle) * geometry.ringRadius, 0.052, Math.cos(angle) * geometry.ringRadius]}
              rotation={[0, angle, 0]}
            >
              <boxGeometry args={[0.105, 0.035, 0.045]} />
              <meshStandardMaterial color="#122431" emissive={index % 2 === 0 ? emitterColor : '#f06dd7'} emissiveIntensity={0.9} metalness={0.62} roughness={0.24} />
            </mesh>
          )
        })}
      </group>
      <mesh position={[0, 0.24, 0]}>
        <cylinderGeometry args={[geometry.coreTopRadius * 0.65, geometry.ringRadius * 0.82, 0.34, 32, 1, true]} />
        <meshBasicMaterial color={emitterColor} transparent opacity={isActive ? 0.09 : 0.035} side={THREE.DoubleSide} depthWrite={false} blending={THREE.AdditiveBlending} />
      </mesh>
      <pointLight position={[0, 0.42, 0]} intensity={isActive ? 1.25 : 0.58} distance={geometry.lightDistance} color={emitterColor} />
      <group position={[0, 0.18, 0]}>
        <AvatarPresenceLayer
          presenceState={presenceState}
          motionEnabled={motionEnabled}
          rootPath={rootPath}
        />
      </group>
    </group>
  )
}

function PresenceLedgerStatusBadge({
  status,
  state,
}: {
  status?: PresenceLedgerStatus
  state: AgentPresenceState
}) {
  const view = deriveBoardroomPresenceStatusView(status, state)
  return (
    <Html position={[0, 2.25, -0.08]} center distanceFactor={7.5}>
      <div className={view.className} title={view.title}>
        <span className="presence-ledger-status__label">{view.label}</span>
        <span className="presence-ledger-status__detail">{view.detail}</span>
      </div>
    </Html>
  )
}

function BoardroomScene({
  zones,
  anchors,
  workstations,
  slotAssignments,
  surfaceLayouts = {},
  monitorSlotSources = {},
  monitorRecordsBySlot,
  agentClaims = {},
  onReleaseMonitor,
  onRefreshMonitor,
  sourceProvenance = [],
  instruments = {},
  fleetViewModel = null,
  presenceState = DEFAULT_AGENT_PRESENCE_STATE,
  presenceStatus,
  rootPath = null,
  debug = false,
  onActivate,
  onOpenWorkstation,
  onOpenMonitorSurface,
  onOpenMonitorSession,
  onOpenHermesDashboard,
  onOpenHermesCli,
  onOpenSettings,
  renderProfile,
}: Omit<BoardroomViewportProps, 'active'> & { renderProfile: BoardroomRenderProfile }) {
  const sceneWorkstations = getSceneWorkstations(workstations)
  const [zonePositionOverrides, setZonePositionOverrides] = useState<BoardroomZonePositionOverrides>(() => readZonePositionOverrides())
  const [layoutExportStatus, setLayoutExportStatus] = useState('No exported layout yet')
  const [controlFeedback, setControlFeedback] = useState<{
    message: string
    state: BoardroomPhysicalControlState
  } | null>(null)
  const [monitorPayloads, setMonitorPayloads] = useState<Record<string, MonitorSurfacePayloadEvent>>({})

  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window)) return
    let unlisten: (() => void) | null = null
    let cancelled = false
    void import('@tauri-apps/api/event').then(({ listen }) => listen<MonitorSurfacePayloadEvent>(
      'monitor-surface-payload',
      ({ payload }) => setMonitorPayloads((current) => ({ ...current, [payload.slotId]: payload })),
    )).then((dispose) => {
      if (cancelled) dispose()
      else unlisten = dispose
    })
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])
  const monitorZones = useMemo(
    () => BOARDROOM_MONITOR_ZONES.map((zone) => withPositionOverride(zone, zonePositionOverrides)),
    [zonePositionOverrides],
  )
  const controlZones = useMemo(
    () => BOARDROOM_CONTROL_ZONES.map((zone) => withPositionOverride(zone, zonePositionOverrides)),
    [zonePositionOverrides],
  )
  const commandCoreZone = withPositionOverride(getBoardroomSpatialZone('boardroom.control.center')!, zonePositionOverrides)
  const avatarEmitterZone = withPositionOverride(getBoardroomSpatialZone('boardroom.avatar.emitter')!, zonePositionOverrides)
  const worldWindowZone = withPositionOverride(getBoardroomSpatialZone('boardroom.world.window')!, zonePositionOverrides)

  const moveZone = (zoneId: string, position: Vec3) => {
    setZonePositionOverrides((current) => {
      const next = normalizeBoardroomZonePositionOverrides({ ...current, [zoneId]: position })
      writeZonePositionOverrides(next)
      return next
    })
  }

  const resetEditedLayout = () => {
    clearZonePositionOverrides()
    setZonePositionOverrides({})
    setLayoutExportStatus('Cleared local boardroom position overrides')
  }

  const copyEditedLayout = async () => {
    const serialized = serializeBoardroomZonePositionOverrides(zonePositionOverrides)
    if (Object.keys(zonePositionOverrides).length === 0) {
      setLayoutExportStatus('No local boardroom position overrides to export')
      return
    }

    try {
      await navigator.clipboard.writeText(serialized)
      setLayoutExportStatus('Copied accepted boardroom positions to clipboard')
    } catch {
      console.info(serialized)
      setLayoutExportStatus('Clipboard unavailable; wrote accepted boardroom positions to console')
    }
  }

  const floorMaterial = useSceneMaterial('boardroom_floor')
  const deskMaterial = useSceneMaterial('boardroom_desk')
  const wallMaterial = useSceneMaterial('boardroom_wall')
  const terminalMaterial = useSceneMaterial('world_terminal_housing')
  const boardroomEnvironmentUrl = getWindowAssetUrl(
    'window_boardroom_environment',
    'hdri',
    'boardroom_environment.hdr',
  )
  const skylinePlateUrl = getWindowAssetUrl(
    'window_boardroom_reference_atmosphere',
    'plate',
    'boardroom_reference_atmosphere.jpg',
  )

  const activateControl = (
    action: BoardroomPhysicalControlAction,
    callback: () => void,
    sourceStatus: WorkstationStatus | null | undefined = null,
  ) => {
    const state = deriveBoardroomPhysicalControlState(action.id, sourceStatus)
    const interaction = resolveBoardroomPhysicalControlInteraction(action, state)
    setControlFeedback({ message: interaction.message, state })
    if (interaction.kind === 'dispatch') callback()
  }

  const activateCommandCoreControl = (action: BoardroomPhysicalControlAction) => activateControl(
    action,
    () => dispatchBoardroomCommandCoreControl(action, {
      onOpenSettings,
      onOpenHermesCli,
      onOpenHermesDashboard,
      onEnterWorld: () => onActivate(worldWindowZone.binding ?? worldWindowZone.id),
      onOpenWorkstation,
    }),
  )

  return (
    <>
      {renderProfile.environmentEnabled && boardroomEnvironmentUrl ? <Environment files={boardroomEnvironmentUrl} /> : null}
      <ambientLight intensity={0.32} />
      <directionalLight position={[6, 10, 4]} intensity={1.25} color="#f8fbff" castShadow={renderProfile.shadows} />
      <fog attach="fog" args={['#071018', 8, 24]} />

      <mesh
        rotation={[-Math.PI / 2, 0, 0]}
        position={[0, -1.4, 0]}
        receiveShadow
        material={floorMaterial}
      >
        <planeGeometry args={[24, 24]} />
      </mesh>

      <SceneAssetModel binding="boardroom_physical_stage" fallback={<BoardroomConsoleShell />} />

      {debug ? (
        <SceneAssetModel
          binding="controlled_arda_workstation"
          position={[0, 0.02, 0.42]}
          scale={0.58}
          onClick={() => onActivate(getDeskActivationId('center', sceneWorkstations))}
          fallback={(
            <mesh position={[0, 0.26, 0.42]} material={deskMaterial} onClick={() => onActivate(getDeskActivationId('center', sceneWorkstations))}>
              <boxGeometry args={[2.5, 0.28, 1.1]} />
            </mesh>
          )}
        />
      ) : null}


      <mesh position={[0, 3.0, -5]} material={wallMaterial}>
        <planeGeometry args={[14, 6]} />
      </mesh>

      <mesh position={[0, 2.8, -5.08]}>
        <planeGeometry args={[12.8, 4.35]} />
        <meshStandardMaterial color="#02060c" roughness={0.9} metalness={0.1} />
      </mesh>

      {skylinePlateUrl ? <CyberpunkCityWindow url={skylinePlateUrl} /> : null}

      {monitorZones.map((slot) => {
        const monitorSlotId = resolveMonitorContractSlotId(slot.id, slot.assignmentSlotId)
        const assignment = getSlotAssignment(sceneWorkstations, slotAssignments, slot)
        const persistedSourceZoneId = slot.assignmentSlotId ? slotAssignments[slot.assignmentSlotId] : undefined
        const focus = resolveMonitorFocus(monitorSlotId, slotAssignments, monitorSlotSources, surfaceLayouts, agentClaims, new Date().toISOString())
        const effectiveSourceZoneId = focus?.sourceZoneId ?? persistedSourceZoneId
        const workstationZoneId = getSlotWorkstationZoneId(slot, assignment)
        const typedRecord = monitorRecordsBySlot?.[monitorSlotId as keyof MonitorRecordsBySlot] ?? null
        const activeClaim = shouldRenderActiveMonitorClaim(focus)
          ? (agentClaims[monitorSlotId] ?? (monitorSlotSources[monitorSlotId]?.claim ?? null))
          : null
        const displayMode = resolveUpperMonitorDisplayMode(Boolean(typedRecord), Boolean(activeClaim))
        const handleMonitorActivate = () => {
          if (typedRecord && onOpenMonitorSession) {
            onOpenMonitorSession(typedRecord)
            return
          }
          const request = resolveMonitorSurfaceOpenRequest(monitorSlotId, effectiveSourceZoneId ?? null, focus?.focusMode ?? 'native_window')
          if (request && onOpenMonitorSurface) {
            onOpenMonitorSurface(request)
            return
          }
          onOpenWorkstation(workstationZoneId)
        }
        return (
        <InteractionPad
          key={slot.id}
          slotId={slot.id}
          label={slot.label}
          detail={getSlotDetail(assignment)}
          position={slot.position}
          rotation={slot.rotation}
          size={slot.size}
          color={slot.color}
          showLabel={false}
          showHitbox={false}
          draggable={debug}
          onMovePosition={(position) => moveZone(slot.id, position)}
          onActivate={isUpperMonitorInteractive(displayMode) ? handleMonitorActivate : undefined}
        >
          {displayMode === 'session' && typedRecord ? (
            <BoardroomApertureSurface
              zoneId={monitorSlotId}
              previewMode={slot.previewMode}
              size={slot.size}
              model={{
                eyebrow: typedRecord.owner,
                title: typedRecord.content.kind,
                glyph: `R${typedRecord.revision}`,
                tone: 'cyan',
                status: 'nominal',
                preset: 'routes',
                nodes: [],
                links: [],
                rings: [],
                source: {
                  freshness: 'fresh',
                  sourceId: typedRecord.surface_session_id,
                  sourceLabel: typedRecord.owner,
                  sourcePaths: [],
                  observedAtUtc: typedRecord.updated_at_utc,
                  sourceKind: 'live',
                  truthState: 'live',
                },
              }}
              descriptor={typedRecord.content}
              playback={typedRecord.playback}
              rootPath={rootPath}
              motionEnabled={renderProfile.motionEnabled}
              active
              onActivate={() => onOpenMonitorSession?.(typedRecord)}
            />
          ) : displayMode === 'claim' && activeClaim ? (
            <BoardroomApertureSurface
              zoneId={monitorSlotId}
              previewMode={slot.previewMode}
              size={slot.size}
              model={{
                eyebrow: 'Agent Monitor',
                title: activeClaim.payload_binding,
                glyph: formatMonitorSurfaceStream(monitorPayloads[monitorSlotId] ?? null, !renderProfile.motionEnabled) || activeClaim.payload_binding,
                tone: 'cyan',
                status: 'nominal',
                preset: 'routes',
                nodes: [],
                links: [],
                rings: [],
                source: {
                  freshness: 'fresh',
                  sourceId: activeClaim.owner,
                  sourceLabel: activeClaim.owner,
                  sourcePaths: [],
                  observedAtUtc: new Date().toISOString(),
                  sourceKind: 'live',
                  truthState: 'live',
                },
              }}
              payload={monitorPayloads[monitorSlotId] ?? null}
              motionEnabled={renderProfile.motionEnabled}
              active={!!activeClaim}
              onActivate={handleMonitorActivate}
            />
          ) : (
            <UpperAmbientMonitorScreen
              slotId={monitorSlotId}
              size={slot.size}
              motionEnabled={false}
            />
          )}
          <MonitorOwnershipRail
            slotId={monitorSlotId}
            size={slot.size}
            session={typedRecord}
            claim={activeClaim}
            motionEnabled={false}
          />
        </InteractionPad>
        )
      })}

      {controlZones.map((slot) => {
        const assignment = getSlotAssignment(sceneWorkstations, slotAssignments, slot)
        const persistedSourceZoneId = slot.assignmentSlotId ? slotAssignments[slot.assignmentSlotId] : undefined
        const workstationZoneId = getSlotWorkstationZoneId(slot, assignment)
        const instrument = resolveBoardroomHudInstrument(instruments, slot.id, slot.assignmentSlotId)
        return (
        <InteractionPad
          key={slot.id}
          slotId={slot.id}
          label={slot.label}
          detail={getSlotDetail(assignment)}
          position={slot.position}
          rotation={slot.rotation}
          size={slot.size}
          color={slot.color}
          primary={slot.primary}
          showLabel={debug}
          showHitbox={false}
          draggable={debug}
          onMovePosition={(position) => moveZone(slot.id, position)}
          onActivate={() => onOpenWorkstation(workstationZoneId)}
        >
          {fleetViewModel && isFleetWorkstationAssignment(assignment) ? (
            <FleetPreviewSurface
              zone={slot}
              assignment={assignment}
              fleetViewModel={fleetViewModel}
              motionEnabled={false}
              onActivate={() => onOpenWorkstation(workstationZoneId)}
            />
          ) : (
            <HudInstrumentSurface
              zone={slot}
              assignment={assignment}
              persistedSourceZoneId={persistedSourceZoneId}
              instrument={instrument}
              motionEnabled={false}
              onActivate={() => onOpenWorkstation(workstationZoneId)}
            />
          )}
        </InteractionPad>
        )
      })}

      <group position={commandCoreZone.position} rotation={commandCoreZone.rotation}>
        <CommandCoreSurface
          zone={commandCoreZone}
          onControl={activateCommandCoreControl}
          nowInstrument={instruments.command_core}
          healthInstrument={instruments.view_desk_control_panel}
          routingInstrument={instruments.view_desk_r}
        />
      </group>



      {controlFeedback ? (
        <Html position={[0, 0.56, 1.92]} center distanceFactor={6.2}>
          <button
            type="button"
            className={`boardroom-control-feedback boardroom-control-feedback--${controlFeedback.state.state}`}
            role="status"
            aria-live="polite"
            onClick={() => setControlFeedback(null)}
          >
            <strong>{controlFeedback.state.statusLabel}</strong>
            <span>{controlFeedback.message}</span>
          </button>
        </Html>
      ) : null}


      <AvatarEmitterBase
        zone={avatarEmitterZone}
        presenceState={presenceState}
        motionEnabled={renderProfile.motionEnabled}
        rootPath={rootPath}
      />
      {debug ? (
        <>
          <BoardroomMissionCue presenceState={presenceState} />
          <PresenceLedgerStatusBadge state={presenceState} status={presenceStatus} />
        </>
      ) : null}

      {debug ? (
        <Html position={[-5.8, 3.8, 0]} transform>
          <SceneRuntimeCard
            eyebrow="Scene Debug"
            title="Boardroom Runtime"
            metrics={[
              { label: 'Anchors', value: anchors.length },
              { label: 'Zones', value: zones.length },
              { label: 'Slots', value: monitorZones.length + controlZones.length },
              { label: 'Dragged', value: Object.keys(zonePositionOverrides).length },
            ]}
            actions={[
              { label: 'Copy layout', onClick: copyEditedLayout },
              { label: 'Reset layout', onClick: resetEditedLayout },
              { label: 'Settings', onClick: onOpenSettings },
            ]}
          >
            <p>{layoutExportStatus}</p>
          </SceneRuntimeCard>
        </Html>
      ) : null}

      <OrbitControls
        enablePan={false}
        enableZoom={false}
        minPolarAngle={1.38}
        maxPolarAngle={1.48}
        minAzimuthAngle={-0.2}
        maxAzimuthAngle={0.2}
        target={BOARDROOM_CAMERA_COMPOSITION.target}
      />
    </>
  )
}

function BoardroomFrameRateProbe() {
  const frameCount = useRef(0)
  const sampledAt = useRef(performance.now())
  const gl = useThree((state) => state.gl)
  const rendererReported = useRef(false)
  useFrame(() => {
    if (!rendererReported.current) {
      const context = gl.getContext()
      const extension = context.getExtension('WEBGL_debug_renderer_info')
      const reportedRenderer = extension
        ? String(context.getParameter(extension.UNMASKED_RENDERER_WEBGL))
        : 'renderer unavailable'
      const renderer = reportedRenderer === 'Apple GPU' && navigator.userAgent.includes('Linux')
        ? 'WebKitGTK masked GPU (Linux)'
        : reportedRenderer
      const rendererOutput = document.getElementById('boardroom-renderer-probe')
      if (rendererOutput) {
        rendererOutput.textContent = renderer
        rendererReported.current = true
      }
    }
    frameCount.current += 1
    const now = performance.now()
    const elapsed = now - sampledAt.current
    if (elapsed < 1000) return
    const output = document.getElementById('boardroom-frame-rate-probe')
    if (output) output.textContent = `Scene ${(frameCount.current * 1000 / elapsed).toFixed(1)} FPS`
    frameCount.current = 0
    sampledAt.current = now
  })
  return null
}

export default function BoardroomViewport(props: BoardroomViewportProps) {
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(false)
  const [softwareRenderer, setSoftwareRenderer] = useState(false)
  const acceptanceEnabled = import.meta.env.DEV && import.meta.env.VITE_MONITOR_ACCEPTANCE === '1'

  useEffect(() => {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)')
    const update = () => setPrefersReducedMotion(query.matches)
    update()
    query.addEventListener('change', update)
    return () => query.removeEventListener('change', update)
  }, [])

  useEffect(() => {
    if (!isTauri()) return
    let cancelled = false
    void invoke<{ software_renderer: boolean }>('get_hud_render_context').then((context) => {
      if (!cancelled) setSoftwareRenderer(context.software_renderer)
    }).catch(() => undefined)
    return () => { cancelled = true }
  }, [])

  const deviceMemoryGb = (navigator as Navigator & { deviceMemory?: number }).deviceMemory
  const renderProfile = resolveBoardroomRenderProfile({
    active: props.active,
    prefersReducedMotion,
    hardwareConcurrency: navigator.hardwareConcurrency,
    deviceMemoryGb,
    nativeWebKit: isTauri(),
    softwareRenderer,
  })

  return (
    <div
      className={`scene-runtime-canvas${props.active ? '' : ' scene-runtime-canvas--inactive'}`}
      data-boardroom-render-profile={renderProfile.id}
    >
      <Canvas
        key={`boardroom-camera-${BOARDROOM_CAMERA_COMPOSITION.fov}`}
        camera={{ position: BOARDROOM_CAMERA_COMPOSITION.position, fov: BOARDROOM_CAMERA_COMPOSITION.fov }}
        dpr={renderProfile.dpr}
        frameloop={renderProfile.frameloop}
        shadows={renderProfile.shadows}
      >
        <color attach="background" args={['#05080d']} />
        {acceptanceEnabled ? <BoardroomFrameRateProbe /> : null}
        <Suspense fallback={null}>
          <BoardroomScene {...props} renderProfile={renderProfile} />
        </Suspense>
      </Canvas>
      <BoardroomAccessibilityControls
        anchors={props.anchors}
        workstations={props.workstations}
        onActivate={props.onActivate}
        onOpenWorkstation={props.onOpenWorkstation}
        onOpenHermesDashboard={props.onOpenHermesDashboard}
        onOpenHermesCli={props.onOpenHermesCli}
        onOpenSettings={props.onOpenSettings}
      />
      {acceptanceEnabled ? (
        <div style={{ position: 'fixed', right: '1rem', top: '3.5rem', zIndex: 10000, color: '#8cffc7', textAlign: 'right' }}>
          <output id="boardroom-frame-rate-probe">Scene measuring…</output>
          <br />
          <output id="boardroom-renderer-probe">renderer measuring…</output>
        </div>
      ) : null}
      {props.sceneOverlay ? (
        <div className="scene-runtime-workstation-layer">
          {props.sceneOverlay}
        </div>
      ) : null}
    </div>
  )
}

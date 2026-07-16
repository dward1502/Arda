// sigil: REPAIR
import { Canvas, type ThreeEvent } from '@react-three/fiber'
import { Environment, Html, OrbitControls, useGLTF, useTexture } from '@react-three/drei'
import { Suspense, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import * as THREE from 'three'
import type { Group } from 'three'
import type { BoardroomSurfaceLayout } from '../../lib/boardroomSlotSettings'
import type { ArdaSourceProvenance } from '../../lib/ardaProvenance'
import type { SceneAnchorDefinition, SceneZoneDefinition, WorkstationManifestDefinition } from '../systems/runtimeTypes'
import type { FleetViewModel } from '../workstations/viewModels'
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
  type BoardroomSpatialZone,
  type BoardroomVec3,
  type BoardroomZonePositionOverrides,
} from './boardroomSpatialLayout'
import { deriveBoardroomMonitorModelBinding } from './boardroomMonitorModels'
import type { BoardroomHudInstrumentMap, HudInstrumentModel, HudTone } from './boardroomHudInstruments'
import { deriveBoardroomScreenVisualRefinement } from './boardroomVisualRefinement'
import PresenceAvatar from './PresenceAvatar'
import { parseJsonOrNull } from '../../lib/jsonParse'

interface BoardroomViewportProps {
  active: boolean
  debug?: boolean
  zones: SceneZoneDefinition[]
  anchors: SceneAnchorDefinition[]
  workstations: WorkstationManifestDefinition[]
  slotAssignments: Record<string, string>
  surfaceLayouts?: Record<string, BoardroomSurfaceLayout>
  sourceProvenance?: ArdaSourceProvenance[]
  instruments?: BoardroomHudInstrumentMap
  fleetViewModel?: FleetViewModel | null
  presenceState?: AgentPresenceState
  presenceStatus?: PresenceLedgerStatus
  sceneOverlay?: ReactNode
  onActivate: (anchorId: string) => void
  onOpenWorkstation: (zoneId: string) => void
  onOpenHermesDashboard: () => void
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

function FittedSceneAssetModel({
  binding,
  fitSize,
  fallback,
  surfaceOffset = [0, 0, 0],
  ...props
}: {
  binding: string
  fitSize: BoardroomVec3
  fallback: ReactNode
  surfaceOffset?: BoardroomVec3
  onClick?: () => void
}) {
  const asset = getSceneAssetByBinding(binding)
  if (!asset?.glbUrl) return <>{fallback}</>
  return <LoadedFittedSceneAsset url={asset.glbUrl} fitSize={fitSize} surfaceOffset={surfaceOffset} {...props} />
}

function LoadedFittedSceneAsset({
  url,
  fitSize,
  surfaceOffset,
  ...props
}: {
  url: string
  fitSize: BoardroomVec3
  surfaceOffset: BoardroomVec3
  onClick?: () => void
}) {
  const gltf = useGLTF(url)
  const { scene, scale, position } = useMemo(() => {
    const clonedScene = gltf.scene.clone(true) as Group
    const box = new THREE.Box3().setFromObject(clonedScene)
    const size = new THREE.Vector3()
    const center = new THREE.Vector3()
    box.getSize(size)
    box.getCenter(center)

    const safeScale = Math.min(
      fitSize[0] / Math.max(size.x, 0.001),
      fitSize[1] / Math.max(size.y, 0.001),
      fitSize[2] / Math.max(size.z, 0.001),
    )

    return {
      scene: clonedScene,
      scale: safeScale,
      position: [
        surfaceOffset[0] - center.x * safeScale,
        surfaceOffset[1] - center.y * safeScale,
        surfaceOffset[2] - center.z * safeScale,
      ] as BoardroomVec3,
    }
  }, [fitSize, gltf.scene, surfaceOffset])

  return (
    <group {...props}>
      <primitive object={scene} scale={scale} position={position} />
    </group>
  )
}

function CyberpunkCityWindow({ url }: { url: string }) {
  const texture = useTexture(url)
  useEffect(() => {
    texture.colorSpace = THREE.SRGBColorSpace
  }, [texture])

  return (
    <group position={[0, 2.92, -4.92]}>
      <mesh position={[0, 0, 0]}>
        <planeGeometry args={[8.9, 2.9]} />
        <meshBasicMaterial map={texture} toneMapped={false} transparent opacity={1} />
      </mesh>
      <mesh position={[0, 0, 0.025]}>
        <planeGeometry args={[8.9, 2.9]} />
        <meshBasicMaterial color="#2ff6ff" transparent opacity={0.08} blending={THREE.AdditiveBlending} />
      </mesh>
      {[-3.9, -2.8, -1.7, -0.85, 0.35, 1.45, 2.65, 3.55].map((x, index) => (
        <mesh key={x} position={[x, -0.18 + (index % 3) * 0.16, 0.05]}>
          <boxGeometry args={[0.08 + (index % 2) * 0.04, 1.2 + (index % 4) * 0.28, 0.02]} />
          <meshBasicMaterial color={index % 2 === 0 ? '#ff3eb5' : '#32e8ff'} transparent opacity={0.35} blending={THREE.AdditiveBlending} />
        </mesh>
      ))}
      <mesh position={[0, -1.42, 0.06]}>
        <planeGeometry args={[8.9, 0.18]} />
        <meshBasicMaterial color="#ff3eb5" transparent opacity={0.22} blending={THREE.AdditiveBlending} />
      </mesh>
    </group>
  )
}

function setPointerCursor(active: boolean) {
  document.body.style.cursor = active ? 'pointer' : ''
}

type Vec3 = BoardroomVec3

const BOARDROOM_ZONE_POSITION_OVERRIDES_STORAGE_KEY = 'arda.boardroom.zone_positions.v1'

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
  draggable?: boolean
  onMovePosition?: (position: Vec3) => void
  onActivate: () => void
  children?: ReactNode
}) {
  const dragRef = useRef<{ pointerId: number; startPoint: THREE.Vector3; basePosition: Vec3; moved: boolean } | null>(null)
  const suppressNextClickRef = useRef(false)

  const handleActivate = () => {
    onActivate()
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
      onClick={handleClick}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerUp}
    >
      {children}
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

function ScreenSurface({
  zone,
}: {
  zone: BoardroomSpatialZone
}) {
  const refinement = useMemo(() => deriveBoardroomScreenVisualRefinement(zone), [zone])
  const glassColor = new THREE.Color('#06131c')
  const trimColor = new THREE.Color(zone.color)

  return (
    <group>
      <mesh
        onPointerOver={() => setPointerCursor(true)}
        onPointerOut={() => setPointerCursor(false)}
      >
        <boxGeometry args={refinement.paneArgs} />
        <meshPhysicalMaterial
          color={glassColor}
          emissive={trimColor}
          emissiveIntensity={0.28}
          transparent
          opacity={refinement.paneOpacity}
          roughness={refinement.paneRoughness}
          metalness={refinement.paneMetalness}
          clearcoat={refinement.paneClearcoat}
          clearcoatRoughness={0.18}
        />
      </mesh>
      {refinement.trimBars.map((bar) => (
        <mesh key={bar.id} position={bar.position}>
          <boxGeometry args={bar.args} />
          <meshStandardMaterial
            color="#101923"
            emissive={zone.color}
            emissiveIntensity={refinement.trimEmissiveIntensity}
            roughness={refinement.trimRoughness}
            metalness={refinement.trimMetalness}
          />
        </mesh>
      ))}
    </group>
  )
}

function BoardroomMonitorModelSurface({
  zone,
  onActivate,
}: {
  zone: BoardroomSpatialZone
  onActivate: () => void
}) {
  const modelBinding = useMemo(() => deriveBoardroomMonitorModelBinding(zone), [zone])
  if (!modelBinding) return <ScreenSurface zone={zone} />

  return (
    <FittedSceneAssetModel
      binding={modelBinding.binding}
      fitSize={modelBinding.fitSize}
      surfaceOffset={modelBinding.surfaceOffset}
      onClick={onActivate}
      fallback={<ScreenSurface zone={zone} />}
    />
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

function getSlotWorkstationZoneId(slot: BoardroomSpatialZone, assignment: WorkstationManifestDefinition | null): string {
  return assignment?.sourceZoneId ?? `scene_slot:${slot.assignmentSlotId ?? slot.id}`
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


function toneForAssignment(assignment: WorkstationManifestDefinition | null): HudTone {
  const source = assignment?.sourceZoneId ?? ''
  if (source.startsWith('service_')) return 'violet'
  if (source.includes('governance')) return 'gold'
  if (source.includes('human') || source.includes('memory')) return 'mint'
  if (source.includes('planning')) return 'rose'
  return 'cyan'
}

function instrumentModelForAssignment(
  zone: BoardroomSpatialZone,
  assignment: WorkstationManifestDefinition | null,
): HudInstrumentModel {
  const serviceManifest = getSurfaceAdapterManifest(assignment?.sourceZoneId)
  const tone = toneForAssignment(assignment)
  const isHermes = assignment?.sourceZoneId === 'hermes_runtime'
  const seed = (assignment?.sourceZoneId ?? zone.id).split('').reduce((sum, char) => sum + char.charCodeAt(0), 0)
  const states: Array<'good' | 'warn' | 'alert' | 'dim'> = ['good', 'good', 'warn', 'dim', 'good', seed % 5 === 0 ? 'alert' : 'good']
  const nodes = Array.from({ length: 9 }, (_, index) => {
    const angle = (Math.PI * 2 * index) / 9 + (seed % 7) * 0.08
    const radius = index % 3 === 0 ? 31 : index % 2 === 0 ? 42 : 52
    return {
      id: `${zone.id}-${index}`,
      x: 50 + Math.cos(angle) * radius,
      y: 50 + Math.sin(angle) * radius * 0.68,
      state: states[(index + seed) % states.length],
    }
  })

  return {
    title: serviceManifest?.provider ?? assignment?.title.replace(/\s+Workstation$/, '') ?? zone.label,
    eyebrow: isHermes ? 'TERMINAL SURFACE' : serviceManifest ? 'EXTERNAL SURFACE' : zone.previewMode === 'desk_surface' ? 'DESK INSTRUMENT' : 'TACTICAL SURFACE',
    tone: isHermes ? 'violet' : tone,
    status: isHermes ? 'external' : serviceManifest ? 'external' : assignment ? 'nominal' : 'offline',
    glyph: isHermes ? 'HMS' : serviceManifest ? 'EXT' : assignment?.moduleIds[0]?.slice(0, 3).toUpperCase() ?? 'NUL',
    nodes,
    links: [[0, 2], [2, 5], [5, 7], [1, 4], [4, 8], [3, 6]],
    rings: [22, 35, 49],
  }
}

function HudInstrumentSurface({
  zone,
  assignment,
  instrument,
  onActivate,
}: {
  zone: BoardroomSpatialZone
  assignment: WorkstationManifestDefinition | null
  instrument?: HudInstrumentModel
  onActivate: () => void
}) {
  const model = instrument ?? instrumentModelForAssignment(zone, assignment)
  const className = `hud-instrument hud-instrument--${zone.previewMode} hud-instrument--${model.tone}`
  const glowId = `${zone.id.replace(/[^a-z0-9_-]/gi, '-')}-glow`

  return (
    <Html
      center
      transform
      distanceFactor={zone.previewMode === 'monitor_surface' ? 4.1 : 5.6}
      position={[0, 0, zone.previewMode === 'monitor_surface' ? 0.12 : 0.28]}
    >
      <button type="button" className={className} onClick={onActivate} aria-label={`Open ${model.title}`}>
        <span className="hud-instrument__header">
          <span>
            <b>{model.eyebrow}</b>
            <strong>{model.title}</strong>
          </span>
          <i>{model.glyph}</i>
        </span>
        <svg className="hud-instrument__scope" viewBox="0 0 100 100" role="img" aria-hidden="true">
          <defs>
            <radialGradient id={glowId} cx="50%" cy="50%" r="50%">
              <stop offset="0%" stopColor="currentColor" stopOpacity="0.34" />
              <stop offset="68%" stopColor="currentColor" stopOpacity="0.08" />
              <stop offset="100%" stopColor="currentColor" stopOpacity="0" />
            </radialGradient>
          </defs>
          <circle className="hud-instrument__glow" cx="50" cy="50" r="48" fill={`url(#${glowId})`} />
          {model.rings.map((ring, index) => (
            <circle className={`hud-instrument__ring hud-instrument__ring--${index}`} key={ring} cx="50" cy="50" r={ring} />
          ))}
          <path className="hud-instrument__axis" d="M 8 50 H 92 M 50 12 V 88" />
          {model.links.map(([from, to]) => (
            <line
              className="hud-instrument__link"
              key={`${from}-${to}`}
              x1={model.nodes[from].x}
              y1={model.nodes[from].y}
              x2={model.nodes[to].x}
              y2={model.nodes[to].y}
            />
          ))}
          {model.nodes.map((node, index) => (
            <g className={`hud-instrument__node hud-instrument__node--${node.state}`} key={node.id}>
              <circle cx={node.x} cy={node.y} r={index % 3 === 0 ? 2.7 : 2.1} />
              {index % 4 === 0 ? <circle cx={node.x} cy={node.y} r="5.2" /> : null}
            </g>
          ))}
          <path className="hud-instrument__sweep" d="M 50 50 L 82 24 A 41 41 0 0 1 88 42" />
        </svg>
        <span className="hud-instrument__footer">
          <span className={`hud-instrument__status hud-instrument__status--${model.status}`}>{model.status}</span>
          <span className="hud-instrument__pips">
            <i />
            <i />
            <i />
            <i />
            <i />
          </span>
        </span>
      </button>
    </Html>
  )
}

function FleetPreviewSurface({
  zone,
  assignment,
  fleetViewModel,
  onActivate,
}: {
  zone: BoardroomSpatialZone
  assignment: WorkstationManifestDefinition | null
  fleetViewModel: FleetViewModel
  onActivate: () => void
}) {
  const provider = fleetViewModel.providers.find((candidate) => candidate.enabled && candidate.healthy)
    ?? fleetViewModel.providers[0]
    ?? null
  const primaryRoute = fleetViewModel.laneOwnership.find((lane) => lane.route)?.route ?? null
  const liveMetric = fleetViewModel.metrics.find((metric) => metric.id === 'live_targets')
  const totalMetric = fleetViewModel.metrics.find((metric) => metric.id === 'total_targets')
  const offlineMetric = fleetViewModel.metrics.find((metric) => metric.id === 'unexpected_offline')
  const modelCount = fleetViewModel.providers.reduce((count, item) => count + item.models.length, 0)

  return (
    <Html
      center
      transform
      distanceFactor={zone.previewMode === 'monitor_surface' ? 4.2 : 5.4}
      position={[0, 0, zone.previewMode === 'monitor_surface' ? 0.14 : 0.3]}
    >
      <button
        type="button"
        className={`fleet-preview-surface fleet-preview-surface--${fleetViewModel.status}`}
        onClick={onActivate}
        aria-label={`Open ${assignment?.title ?? fleetViewModel.title} fleet workstation`}
      >
        <span className="fleet-preview-surface__header">
          <b>FLEET</b>
          <i>{fleetViewModel.status}</i>
        </span>
        <strong>{assignment?.title.replace(/\s+Workstation$/, '') ?? fleetViewModel.title}</strong>
        <span className="fleet-preview-surface__metrics">
          <span><b>{formatFleetValue(liveMetric?.value)}</b><small>live</small></span>
          <span><b>{formatFleetValue(totalMetric?.value)}</b><small>total</small></span>
          <span><b>{formatFleetValue(offlineMetric?.value, '0')}</b><small>offline</small></span>
        </span>
        <span className="fleet-preview-surface__route">
          <span>{primaryRoute ? `${primaryRoute.providerId} / ${primaryRoute.modelId}` : 'routing unassigned'}</span>
          <small>{provider ? `${provider.providerName} · ${modelCount} models` : 'no provider projection'}</small>
        </span>
      </button>
    </Html>
  )
}

function CommandCoreSurface({
  onOpenWorkstation,
  onOpenHermesDashboard,
  onOpenSettings,
  onEnterWorld,
}: {
  onOpenWorkstation: (zoneId: string) => void
  onOpenHermesDashboard: () => void
  onOpenSettings: () => void
  onEnterWorld: () => void
}) {
  return (
    <Html center transform distanceFactor={5.2} position={[0, 0, 0.32]}>
      <div className="command-core-terminal" aria-label="Boardroom command core">
        <button type="button" className="command-core-terminal__screen" onClick={() => onOpenWorkstation('sovereign_world')}>
          <span className="command-core-terminal__eyebrow">Command Core</span>
          <strong>ARDA CONTROL</strong>
          <span className="command-core-terminal__scope">
            <i />
            <i />
            <i />
            <i />
          </span>
          <small>mode / health / routes</small>
        </button>
        <div className="command-core-terminal__buttons">
          <button type="button" className="command-core-terminal__button command-core-terminal__button--go" onClick={() => onOpenWorkstation('planning_and_queue')}>GO</button>
          <button type="button" className="command-core-terminal__button command-core-terminal__button--stop" onClick={() => onOpenWorkstation('governance_guardhouse')}>STOP</button>
          <button type="button" className="command-core-terminal__button" onClick={() => onOpenWorkstation('routing_and_comms')}>ROUTE</button>
          <button type="button" className="command-core-terminal__button" onClick={onOpenHermesDashboard}>HERMES</button>
          <button type="button" className="command-core-terminal__button" onClick={onEnterWorld}>WORLD</button>
          <button type="button" className="command-core-terminal__button" onClick={onOpenSettings}>SET</button>
        </div>
      </div>
    </Html>
  )
}

function HermesTerminalSurface({ onOpenHermesDashboard }: { onOpenHermesDashboard: () => void }) {
  return (
    <Html center transform distanceFactor={5.2} position={[0, 0, 0.28]}>
      <button type="button" className="hermes-desk-terminal" onClick={onOpenHermesDashboard} aria-label="Open Hermes Dashboard">
        <span className="hermes-desk-terminal__bar">
          <b>HERMES</b>
          <i>9119</i>
        </span>
        <span className="hermes-desk-terminal__lines">
          <i />
          <i />
          <i />
          <i />
        </span>
        <strong>DASHBOARD TERMINAL</strong>
      </button>
    </Html>
  )
}

function AvatarEmitterBase({ zone }: { zone: BoardroomSpatialZone }) {
  return (
    <group position={zone.position}>
      <mesh position={[0, -0.08, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <torusGeometry args={[0.58, 0.045, 16, 96]} />
        <meshStandardMaterial color={zone.color} emissive={zone.color} emissiveIntensity={1.15} roughness={0.18} metalness={0.42} />
      </mesh>
      <mesh position={[0, -0.1, 0]}>
        <cylinderGeometry args={[0.48, 0.56, 0.12, 72]} />
        <meshStandardMaterial color="#071018" emissive="#12344a" emissiveIntensity={0.75} roughness={0.34} metalness={0.6} />
      </mesh>
      <mesh position={[0, 0.02, 0]}>
        <cylinderGeometry args={[0.24, 0.34, 0.04, 72]} />
        <meshStandardMaterial color="#7df2ff" emissive="#5defff" emissiveIntensity={1.8} transparent opacity={0.58} />
      </mesh>
      <pointLight position={[0, 0.42, 0]} intensity={0.95} distance={2.8} color={zone.color} />
    </group>
  )
}

function getPresenceStatusLabel(status: PresenceLedgerStatus | undefined): string {
  if (!status) return 'Presence fallback'
  if (status.source === 'fallback_default') return 'Presence fallback'
  return status.freshness === 'fresh' ? 'Presence live' : 'Presence stale'
}

function getPresenceStatusDetail(status: PresenceLedgerStatus | undefined, state: AgentPresenceState): string {
  if (!status) return 'Default ARDA state'
  if (status.source === 'fallback_default') return status.summary
  const agent = state.primaryAgent.toUpperCase()
  const age = typeof status.ageSeconds === 'number' ? `${status.ageSeconds}s` : 'age unknown'
  return `${agent} · ${age} · ${status.validEventCount} ledger row${status.validEventCount === 1 ? '' : 's'}`
}

function getPresenceStatusClassName(status: PresenceLedgerStatus | undefined): string {
  if (!status || status.source === 'fallback_default') return 'presence-ledger-status presence-ledger-status--fallback'
  return `presence-ledger-status presence-ledger-status--${status.freshness}`
}

function PresenceLedgerStatusBadge({
  status,
  state,
}: {
  status?: PresenceLedgerStatus
  state: AgentPresenceState
}) {
  return (
    <Html position={[0, 2.25, -0.08]} center distanceFactor={7.5}>
      <div className={getPresenceStatusClassName(status)} title={status?.summary ?? 'Default ARDA state'}>
        <span className="presence-ledger-status__label">{getPresenceStatusLabel(status)}</span>
        <span className="presence-ledger-status__detail">{getPresenceStatusDetail(status, state)}</span>
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
  sourceProvenance = [],
  instruments = {},
  fleetViewModel = null,
  presenceState = DEFAULT_AGENT_PRESENCE_STATE,
  presenceStatus,
  debug = false,
  onActivate,
  onOpenWorkstation,
  onOpenHermesDashboard,
  onOpenSettings,
}: Omit<BoardroomViewportProps, 'active'>) {
  const sceneWorkstations = getSceneWorkstations(workstations)
  const [zonePositionOverrides, setZonePositionOverrides] = useState<BoardroomZonePositionOverrides>(() => readZonePositionOverrides())
  const [layoutExportStatus, setLayoutExportStatus] = useState('No exported layout yet')
  const monitorZones = useMemo(
    () => BOARDROOM_MONITOR_ZONES.map((zone) => withPositionOverride(zone, zonePositionOverrides)),
    [zonePositionOverrides],
  )
  const controlZones = useMemo(
    () => BOARDROOM_CONTROL_ZONES.map((zone) => withPositionOverride(zone, zonePositionOverrides)),
    [zonePositionOverrides],
  )
  const hermesButtonZone = withPositionOverride(getBoardroomSpatialZone('boardroom.button.hermes')!, zonePositionOverrides)
  const settingsButtonZone = withPositionOverride(getBoardroomSpatialZone('boardroom.control.center')!, zonePositionOverrides)
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
    'window_boardroom_skyline_plate',
    'plate',
    'boardroom_skyline_plate.jpg',
  )

  return (
    <>
      {boardroomEnvironmentUrl ? <Environment files={boardroomEnvironmentUrl} /> : null}
      <ambientLight intensity={0.32} />
      <directionalLight position={[6, 10, 4]} intensity={1.25} color="#f8fbff" castShadow />
      <pointLight position={[0, 2.4, 0.6]} intensity={1.2} distance={8} color="#8bdcff" />
      <fog attach="fog" args={['#071018', 8, 24]} />

      <mesh
        rotation={[-Math.PI / 2, 0, 0]}
        position={[0, -1.4, 0]}
        receiveShadow
        material={floorMaterial}
      >
        <planeGeometry args={[24, 24]} />
      </mesh>

      <mesh position={[0, -0.18, 0.42]} material={deskMaterial} receiveShadow castShadow>
        <boxGeometry args={[6.8, 0.32, 2.2]} />
      </mesh>

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


      <mesh position={[0, 3.2, -5]} material={wallMaterial}>
        <planeGeometry args={[10, 5]} />
      </mesh>

      <mesh position={[0, 2.85, -5.08]}>
        <planeGeometry args={[7.25, 2.35]} />
        <meshStandardMaterial color="#02060c" roughness={0.9} metalness={0.1} />
      </mesh>

      {skylinePlateUrl ? <CyberpunkCityWindow url={skylinePlateUrl} /> : null}

      {monitorZones.map((slot) => {
        const assignment = getSlotAssignment(sceneWorkstations, slotAssignments, slot)
        const workstationZoneId = getSlotWorkstationZoneId(slot, assignment)
        const surfaceLayout = slot.assignmentSlotId ? surfaceLayouts[slot.assignmentSlotId] : undefined
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
          draggable={debug}
          onMovePosition={(position) => moveZone(slot.id, position)}
          onActivate={() => onOpenWorkstation(workstationZoneId)}
        >
          {fleetViewModel && isFleetWorkstationAssignment(assignment) ? (
            <FleetPreviewSurface
              zone={slot}
              assignment={assignment}
              fleetViewModel={fleetViewModel}
              onActivate={() => onOpenWorkstation(workstationZoneId)}
            />
          ) : (
            <ScreenSurface zone={slot} />
          )}
        </InteractionPad>
        )
      })}

      {controlZones.map((slot) => {
        const assignment = getSlotAssignment(sceneWorkstations, slotAssignments, slot)
        const workstationZoneId = getSlotWorkstationZoneId(slot, assignment)
        const surfaceLayout = slot.assignmentSlotId ? surfaceLayouts[slot.assignmentSlotId] : undefined
        const isHermesSlot = assignment?.sourceZoneId === 'hermes_runtime'
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
          draggable={debug}
          onMovePosition={(position) => moveZone(slot.id, position)}
          onActivate={() => onOpenWorkstation(workstationZoneId)}
        >
          {fleetViewModel && isFleetWorkstationAssignment(assignment) ? (
            <FleetPreviewSurface
              zone={slot}
              assignment={assignment}
              fleetViewModel={fleetViewModel}
              onActivate={() => onOpenWorkstation(workstationZoneId)}
            />
          ) : (
            <ScreenSurface zone={slot} />
          )}
        </InteractionPad>
        )
      })}

      <InteractionPad
        slotId={settingsButtonZone.id}
        label={settingsButtonZone.label}
        detail={settingsButtonZone.detail}
        position={settingsButtonZone.position}
        rotation={settingsButtonZone.rotation}
        size={settingsButtonZone.size}
        color={settingsButtonZone.color}
        primary={settingsButtonZone.primary}
        showLabel={debug}
        draggable={debug}
        onMovePosition={(position) => moveZone(settingsButtonZone.id, position)}
        onActivate={onOpenSettings}
      >
        <ScreenSurface zone={settingsButtonZone} />
      </InteractionPad>

      <InteractionPad
        slotId={hermesButtonZone.id}
        label={hermesButtonZone.label}
        detail={hermesButtonZone.detail}
        position={hermesButtonZone.position}
        rotation={hermesButtonZone.rotation}
        size={hermesButtonZone.size}
        color={hermesButtonZone.color}
        primary={hermesButtonZone.primary}
        draggable={debug}
        onMovePosition={(position) => moveZone(hermesButtonZone.id, position)}
        onActivate={onOpenHermesDashboard}
      >
        <SceneAssetModel
          binding="human_control"
          scale={0.46}
          onClick={onOpenHermesDashboard}
          fallback={(
            <mesh onClick={onOpenHermesDashboard} material={terminalMaterial}>
              <boxGeometry args={[1.28, 0.18, 0.42]} />
            </mesh>
          )}
        />
      </InteractionPad>

      <AvatarEmitterBase zone={avatarEmitterZone} />
      <PresenceAvatar position={avatarEmitterZone.position} scale={0.82} presenceState={presenceState} />
      <BoardroomMissionCue presenceState={presenceState} />
      <PresenceLedgerStatusBadge state={presenceState} status={presenceStatus} />

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
        minPolarAngle={Math.PI / 2.55}
        maxPolarAngle={Math.PI / 2.55}
        minAzimuthAngle={-0.72}
        maxAzimuthAngle={0.72}
        target={[0, 0.92, -1.45]}
      />
    </>
  )
}

export default function BoardroomViewport(props: BoardroomViewportProps) {
  return (
    <div className="scene-runtime-canvas">
      <Canvas camera={{ position: [0, 3.15, 8.2], fov: 43 }} dpr={[1, 2]} frameloop={props.active ? 'always' : 'never'}>
        <color attach="background" args={['#05080d']} />
        <Suspense fallback={null}>
          <BoardroomScene {...props} />
        </Suspense>
      </Canvas>
      {props.sceneOverlay ? (
        <div className="scene-runtime-workstation-layer">
          {props.sceneOverlay}
        </div>
      ) : null}
    </div>
  )
}

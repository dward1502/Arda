// sigil: REPAIR
import { Canvas, useFrame, type ThreeEvent } from '@react-three/fiber'
import { Environment, Html, OrbitControls, useGLTF, useTexture } from '@react-three/drei'
import { Suspense, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import * as THREE from 'three'
import type { Group } from 'three'
import type { BoardroomSurfaceLayout } from '../../lib/boardroomSlotSettings'
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

import PresenceAvatar from './PresenceAvatar'
import { parseJsonOrNull } from '../../lib/jsonParse'
import { deriveBoardroomPresenceStatusView } from './boardroomPresenceStatus'
import { resolveSceneSlotWorkstationZoneId } from '../workstations/sceneSlotWorkstationTemplates'
import {
  BOARDROOM_CAMERA_COMPOSITION,
  deriveAvatarEmitterGeometry,
} from './boardroomComposition'
import {
  deriveBoardroomPhysicalControlState,
  getBoardroomPhysicalControlAction,
  resolveBoardroomPhysicalControlInteraction,
  type BoardroomPhysicalControlAction,
  type BoardroomPhysicalControlState,
} from './boardroomPhysicalControls'
import {
  resolveBoardroomRenderProfile,
  type BoardroomRenderProfile,
} from './boardroomPerformance'

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
const isNativeTauriRuntime = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

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

function HudInstrumentVisualization({ model, glowId }: { model: HudInstrumentModel; glowId: string }) {
  const renderNodes = (withLinks: boolean) => (
    <>
      {withLinks ? model.links.map(([from, to]) => (
        <line
          className="hud-instrument__link"
          key={`${from}-${to}`}
          x1={model.nodes[from].x}
          y1={model.nodes[from].y}
          x2={model.nodes[to].x}
          y2={model.nodes[to].y}
        />
      )) : null}
      {model.nodes.map((node, index) => (
        <g className={`hud-instrument__node hud-instrument__node--${node.state}`} key={node.id}>
          <circle cx={node.x} cy={node.y} r={index % 3 === 0 ? 2.7 : 2.1} />
          {index % 4 === 0 ? <circle cx={node.x} cy={node.y} r="5.2" /> : null}
        </g>
      ))}
    </>
  )

  return (
    <svg className={`hud-instrument__scope hud-instrument__scope--${model.preset}`} viewBox="0 0 100 100" role="img" aria-hidden="true">
      <defs>
        <radialGradient id={glowId} cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="currentColor" stopOpacity="0.34" />
          <stop offset="68%" stopColor="currentColor" stopOpacity="0.08" />
          <stop offset="100%" stopColor="currentColor" stopOpacity="0" />
        </radialGradient>
      </defs>
      <path className="hud-instrument__grid" d="M 8 25 H 92 M 8 50 H 92 M 8 75 H 92 M 25 8 V 92 M 50 8 V 92 M 75 8 V 92" />
      <circle className="hud-instrument__glow" cx="50" cy="50" r="48" fill={`url(#${glowId})`} />

      {model.preset === 'topology' ? (
        <>
          {model.rings.map((ring, index) => (
            <circle className={`hud-instrument__ring hud-instrument__ring--${index}`} key={ring} cx="50" cy="50" r={ring} />
          ))}
          <path className="hud-instrument__axis" d="M 8 50 H 92 M 50 12 V 88" />
          {renderNodes(true)}
          <path className="hud-instrument__sweep" d="M 50 50 L 82 24 A 41 41 0 0 1 88 42" />
        </>
      ) : null}

      {model.preset === 'routes' ? (
        <>
          {[24, 50, 76].map((y, index) => (
            <g className="hud-instrument__route" key={y}>
              <path d={`M 8 ${y} C 30 ${y - 18}, 66 ${y + 18}, 92 ${y}`} />
              <circle className={`hud-instrument__packet hud-instrument__packet--${index}`} cx={28 + index * 20} cy={y + (index - 1) * 5} r="2.5" />
            </g>
          ))}
          {renderNodes(false)}
        </>
      ) : null}

      {model.preset === 'lanes' ? (
        <>
          {model.nodes.slice(0, 5).map((node, index) => (
            <g className="hud-instrument__lane" key={node.id}>
              <rect x="10" y={14 + index * 16} width="80" height="7" rx="3.5" />
              <rect className="hud-instrument__lane-fill" x="10" y={14 + index * 16} width={Math.max(16, Math.min(80, node.x * 0.8))} height="7" rx="3.5" />
              <circle cx={Math.max(18, Math.min(86, node.x * 0.8 + 8))} cy={17.5 + index * 16} r="2" />
            </g>
          ))}
        </>
      ) : null}

      {model.preset === 'constellation' ? (
        <>
          <path className="hud-instrument__constellation-orbit" d="M 12 58 C 26 16, 74 16, 88 58 C 73 87, 27 87, 12 58 Z" />
          {renderNodes(true)}
        </>
      ) : null}

      {model.preset === 'pulse' ? (
        <>
          <path className="hud-instrument__pulse-guide" d="M 8 50 H 92" />
          <polyline
            className="hud-instrument__pulse-wave"
            points={model.nodes.map((node, index) => `${8 + index * (84 / Math.max(1, model.nodes.length - 1))},${20 + node.y * 0.58}`).join(' ')}
          />
          <path className="hud-instrument__scan" d="M 12 16 V 84" />
        </>
      ) : null}

      {model.preset === 'standby' ? (
        <>
          <path className="hud-instrument__standby-frame" d="M 14 18 H 86 V 82 H 14 Z M 22 26 L 78 74 M 78 26 L 22 74" />
          <circle className="hud-instrument__standby-glyph" cx="50" cy="50" r="8" />
        </>
      ) : null}
    </svg>
  )
}

function HudInstrumentSurface({
  zone,
  assignment,
  persistedSourceZoneId,
  instrument,
  onActivate,
}: {
  zone: BoardroomSpatialZone
  assignment: WorkstationManifestDefinition | null
  persistedSourceZoneId?: string
  instrument?: HudInstrumentModel
  onActivate: () => void
}) {
  const model = instrument ?? instrumentModelForAssignment(zone, assignment, persistedSourceZoneId)
  const deskRole = zone.id.includes('wrap') ? 'outer' : zone.id.includes('inner') ? 'inner' : 'standard'
  const className = `hud-instrument hud-instrument--${zone.previewMode} hud-instrument--desk-${deskRole} hud-instrument--${model.tone} hud-instrument--${model.status}`
  const glowId = `${zone.id.replace(/[^a-z0-9_-]/gi, '-')}-glow`
  const sourceTime = model.source?.observedAtUtc?.slice(11, 16) ?? '--:--'
  const sourceTitle = model.source
    ? `${(model.source.sourceIds ?? [model.source.sourceId]).join(', ')} · ${model.source.sourcePaths.join(', ')} · observed ${model.source.observedAtUtc ?? 'unknown'}`
    : undefined
  const distanceFactor = zone.previewMode === 'monitor_surface' ? 4.1 : 4.3
  const surfacePosition: Vec3 = zone.previewMode === 'monitor_surface'
    ? [0, 0, 0.12]
    : [0, zone.size[1] / 2 + 0.045, 0]

  return (
    <>
      <Html
        center
        transform
        distanceFactor={distanceFactor}
        position={surfacePosition}
        rotation={zone.previewMode === 'monitor_surface' ? [0, 0, 0] : [-Math.PI / 2, 0, 0]}
      >
        <button type="button" className={className} onClick={onActivate} aria-label={`Open ${model.title}`}>
          <span className="hud-instrument__header">
            <span>
              <b>{model.eyebrow}</b>
              <strong>{model.title}</strong>
            </span>
            <i>{model.glyph}</i>
          </span>
          <HudInstrumentVisualization model={model} glowId={glowId} />
          <span className="hud-instrument__footer">
            <span className={`hud-instrument__status hud-instrument__status--${model.status}`}>
              {model.status === 'nominal' ? 'live' : model.status === 'offline' ? 'no data' : model.status}
            </span>
            {model.source ? (
              <span className={`hud-instrument__source hud-instrument__source--${model.source.freshness}`} title={sourceTitle}>
                {model.source.freshness} {sourceTime}Z
              </span>
            ) : null}
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
      {isNativeTauriRuntime ? (
        <Html center distanceFactor={distanceFactor} position={surfacePosition} pointerEvents="auto">
          <button
            type="button"
            className={`boardroom-scene__monitor-native-target boardroom-scene__monitor-native-target--${zone.previewMode}`}
            aria-label={`Open ${model.title}`}
            onClick={onActivate}
          />
        </Html>
      ) : null}
    </>
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
  const distanceFactor = zone.previewMode === 'monitor_surface' ? 4.2 : 4.3
  const surfacePosition: Vec3 = zone.previewMode === 'monitor_surface'
    ? [0, 0, 0.14]
    : [0, zone.size[1] / 2 + 0.05, 0]

  return (
    <>
      <Html
        center
        transform
        distanceFactor={distanceFactor}
        position={surfacePosition}
        rotation={zone.previewMode === 'monitor_surface' ? [0, 0, 0] : [-Math.PI / 2, 0, 0]}
      >
        <button
          type="button"
          className={`fleet-preview-surface fleet-preview-surface--${zone.previewMode} fleet-preview-surface--${fleetViewModel.status}`}
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
      {isNativeTauriRuntime ? (
        <Html center distanceFactor={distanceFactor} position={surfacePosition} pointerEvents="auto">
          <button
            type="button"
            className={`boardroom-scene__monitor-native-target boardroom-scene__monitor-native-target--${zone.previewMode}`}
            aria-label={`Open ${assignment?.title ?? fleetViewModel.title} fleet workstation`}
            onClick={onActivate}
          />
        </Html>
      ) : null}
    </>
  )
}

function CommandCoreSurface({
  onControl,
}: {
  onControl: (action: BoardroomPhysicalControlAction) => void
}) {
  const buttonProps = (actionId: string) => {
    const action = getBoardroomPhysicalControlAction(actionId)
    const state = deriveBoardroomPhysicalControlState(actionId, null)
    return {
      'aria-label': action.label,
      'data-authority': action.authority,
      disabled: state.disabled,
      onClick: () => onControl(action),
      title: `${action.authority} · verify ${action.verificationPath}`,
    }
  }

  return (
    <>
      <Html center transform distanceFactor={4.3} position={[0, 0.05, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <div className="command-core-terminal" aria-label="Boardroom command core">
          <button type="button" className="command-core-terminal__screen" {...buttonProps('open_command_core')}>
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
            <button type="button" className="command-core-terminal__button command-core-terminal__button--go" {...buttonProps('open_approval_queue')}>GO</button>
            <button type="button" className="command-core-terminal__button command-core-terminal__button--stop" {...buttonProps('open_emergency_stop')}>STOP</button>
            <button type="button" className="command-core-terminal__button" {...buttonProps('open_route_selector')}>ROUTE</button>
            <button type="button" className="command-core-terminal__button" {...buttonProps('enter_world')}>WORLD</button>
          </div>
        </div>
      </Html>
      {isNativeTauriRuntime ? (
        <Html center distanceFactor={4.3} position={[0, 0.05, 0]} pointerEvents="auto">
          <button
            type="button"
            className="command-core-terminal__native-screen-target"
            aria-label="Open ARDA Control"
            {...buttonProps('open_command_core')}
          />
        </Html>
      ) : null}
    </>
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

function HermesCliButtonSurface({
  action,
  controlState,
  onClick,
  zone,
}: {
  action: BoardroomPhysicalControlAction
  controlState: BoardroomPhysicalControlState
  onClick: () => void
  zone: BoardroomSpatialZone
}) {
  return (
    <group position={zone.position} rotation={zone.rotation}>
      <PhysicalControlButtonSurface
        label={action.label}
        size={zone.size}
        color={zone.color}
        controlState={controlState}
        title={`${action.authority} · verify ${action.verificationPath}`}
        onClick={onClick}
      />
    </group>
  )
}

function AvatarEmitterBase({
  zone,
  presenceState,
  motionEnabled,
}: {
  zone: BoardroomSpatialZone
  presenceState: AgentPresenceState
  motionEnabled: boolean
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
      <mesh position={[0, -0.08, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <torusGeometry args={[geometry.ringRadius, geometry.ringTubeRadius, 16, 96]} />
        <meshStandardMaterial color={emitterColor} emissive={emitterColor} emissiveIntensity={isActive ? 2.1 : 0.82} roughness={0.18} metalness={0.42} />
      </mesh>
      <mesh position={[0, -0.1, 0]}>
        <cylinderGeometry args={[geometry.baseTopRadius, geometry.baseBottomRadius, 0.12, 72]} />
        <meshStandardMaterial color="#071018" emissive="#12344a" emissiveIntensity={0.75} roughness={0.34} metalness={0.6} />
      </mesh>
      <group ref={pulseRef}>
        <mesh position={[0, 0.02, 0]}>
          <cylinderGeometry args={[geometry.coreTopRadius, geometry.coreBottomRadius, 0.04, 72]} />
          <meshStandardMaterial color="#7df2ff" emissive={emitterColor} emissiveIntensity={isActive ? 2.8 : 1.25} transparent opacity={isActive ? 0.72 : 0.46} />
        </mesh>
        {[0, 1, 2].map((index) => (
          <mesh key={index} position={[0, 0.13 + index * 0.075, 0]} rotation={[Math.PI / 2, 0, 0]}>
            <torusGeometry args={[geometry.coreTopRadius * (0.78 + index * 0.24), geometry.ringTubeRadius * 0.22, 8, 48]} />
            <meshBasicMaterial color={emitterColor} transparent opacity={isActive ? 0.52 - index * 0.1 : 0.16 - index * 0.03} />
          </mesh>
        ))}
      </group>
      <pointLight position={[0, 0.42, 0]} intensity={isActive ? 1.25 : 0.58} distance={geometry.lightDistance} color={emitterColor} />
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
  sourceProvenance = [],
  instruments = {},
  fleetViewModel = null,
  presenceState = DEFAULT_AGENT_PRESENCE_STATE,
  presenceStatus,
  debug = false,
  onActivate,
  onOpenWorkstation,
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
  const monitorZones = useMemo(
    () => BOARDROOM_MONITOR_ZONES.map((zone) => withPositionOverride(zone, zonePositionOverrides)),
    [zonePositionOverrides],
  )
  const controlZones = useMemo(
    () => BOARDROOM_CONTROL_ZONES.map((zone) => withPositionOverride(zone, zonePositionOverrides)),
    [zonePositionOverrides],
  )
  const hermesButtonZone = withPositionOverride(getBoardroomSpatialZone('boardroom.button.hermes')!, zonePositionOverrides)
  const hermesCliButtonZone = withPositionOverride(getBoardroomSpatialZone('boardroom.button.hermes_cli')!, zonePositionOverrides)
  const commandCoreZone = withPositionOverride(getBoardroomSpatialZone('boardroom.control.center')!, zonePositionOverrides)
  const settingsButtonZone = withPositionOverride(getBoardroomSpatialZone('boardroom.button.settings')!, zonePositionOverrides)
  const serviceHealthAction = getBoardroomPhysicalControlAction('service_health_status')
  const settingsAction = getBoardroomPhysicalControlAction('open_settings')
  const hermesCliAction = getBoardroomPhysicalControlAction('open_hermes_cli')
  const hermesDashboardAction = getBoardroomPhysicalControlAction('open_hermes_dashboard')
  const serviceHealthButtonZone = withPositionOverride(getBoardroomSpatialZone(serviceHealthAction.zoneId)!, zonePositionOverrides)
  const serviceHealthState = deriveBoardroomPhysicalControlState(serviceHealthAction.id, fleetViewModel?.status)
  const settingsState = deriveBoardroomPhysicalControlState(settingsAction.id, null)
  const hermesCliState = deriveBoardroomPhysicalControlState(hermesCliAction.id, null)
  const hermesDashboardState = deriveBoardroomPhysicalControlState(hermesDashboardAction.id, null)
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

  const activateServiceHealth = () => activateControl(
    serviceHealthAction,
    () => onOpenWorkstation(serviceHealthAction.targetZoneId),
    fleetViewModel?.status,
  )

  const activateCommandControl = (action: BoardroomPhysicalControlAction) => activateControl(
    action,
    () => action.id === 'enter_world'
      ? onActivate(worldWindowZone.binding ?? worldWindowZone.id)
      : onOpenWorkstation(action.targetZoneId),
  )
  const activateSettings = () => activateControl(settingsAction, onOpenSettings)
  const activateHermesCli = () => activateControl(hermesCliAction, onOpenHermesCli)
  const activateHermesDashboard = () => activateControl(hermesDashboardAction, onOpenHermesDashboard)

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
          showLabel={false}
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
              onActivate={() => onOpenWorkstation(workstationZoneId)}
            />
          ) : (
            <HudInstrumentSurface
              zone={slot}
              assignment={assignment}
              persistedSourceZoneId={persistedSourceZoneId}
              instrument={instrument}
              onActivate={() => onOpenWorkstation(workstationZoneId)}
            />
          )}
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
              onActivate={() => onOpenWorkstation(workstationZoneId)}
            />
          ) : (
            <HudInstrumentSurface
              zone={slot}
              assignment={assignment}
              persistedSourceZoneId={persistedSourceZoneId}
              instrument={instrument}
              onActivate={() => onOpenWorkstation(workstationZoneId)}
            />
          )}
        </InteractionPad>
        )
      })}

      <group position={commandCoreZone.position} rotation={commandCoreZone.rotation}>
        <CommandCoreSurface onControl={activateCommandControl} />
      </group>


      <InteractionPad
        slotId={serviceHealthButtonZone.id}
        label={serviceHealthButtonZone.label}
        detail={serviceHealthButtonZone.detail}
        position={serviceHealthButtonZone.position}
        rotation={serviceHealthButtonZone.rotation}
        size={serviceHealthButtonZone.size}
        color={serviceHealthButtonZone.color}
        showLabel={debug}
        draggable={debug}
        onMovePosition={(position) => moveZone(serviceHealthButtonZone.id, position)}
        onActivate={activateServiceHealth}
      >
        <PhysicalControlButtonSurface
          label={serviceHealthAction.shortLabel}
          size={serviceHealthButtonZone.size}
          color={serviceHealthButtonZone.color}
          controlState={serviceHealthState}
          title={`${serviceHealthAction.authority} · verify ${serviceHealthAction.verificationPath}`}
          onClick={activateServiceHealth}
          onBlocked={activateServiceHealth}
        />
      </InteractionPad>

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
        onActivate={activateSettings}
      >
        <PhysicalControlButtonSurface
          label={settingsAction.label}
          size={settingsButtonZone.size}
          color="#b98cff"
          controlState={settingsState}
          title={`${settingsAction.authority} · verify ${settingsAction.verificationPath}`}
          onClick={activateSettings}
        />
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
        showLabel={debug}
        draggable={debug}
        onMovePosition={(position) => moveZone(hermesButtonZone.id, position)}
        onActivate={activateHermesDashboard}
      >
        <PhysicalControlButtonSurface
          label={hermesDashboardAction.label}
          size={hermesButtonZone.size}
          color="#b98cff"
          controlState={hermesDashboardState}
          title={`${hermesDashboardAction.authority} · verify ${hermesDashboardAction.verificationPath}`}
          onClick={activateHermesDashboard}
        />
      </InteractionPad>

      <HermesCliButtonSurface
        action={hermesCliAction}
        controlState={hermesCliState}
        onClick={activateHermesCli}
        zone={hermesCliButtonZone}
      />

      <AvatarEmitterBase
        zone={avatarEmitterZone}
        presenceState={presenceState}
        motionEnabled={renderProfile.motionEnabled}
      />
      {debug ? (
        <>
          <PresenceAvatar position={avatarEmitterZone.position} scale={0.82} presenceState={presenceState} />
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

export default function BoardroomViewport(props: BoardroomViewportProps) {
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(false)

  useEffect(() => {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)')
    const update = () => setPrefersReducedMotion(query.matches)
    update()
    query.addEventListener('change', update)
    return () => query.removeEventListener('change', update)
  }, [])

  const deviceMemoryGb = (navigator as Navigator & { deviceMemory?: number }).deviceMemory
  const renderProfile = resolveBoardroomRenderProfile({
    active: props.active,
    prefersReducedMotion,
    hardwareConcurrency: navigator.hardwareConcurrency,
    deviceMemoryGb,
  })

  return (
    <div
      className={`scene-runtime-canvas${props.active ? '' : ' scene-runtime-canvas--inactive'}`}
      data-boardroom-render-profile={renderProfile.id}
    >
      <Canvas
        camera={{ position: BOARDROOM_CAMERA_COMPOSITION.position, fov: BOARDROOM_CAMERA_COMPOSITION.fov }}
        dpr={renderProfile.dpr}
        frameloop={renderProfile.frameloop}
        shadows={renderProfile.shadows}
      >
        <color attach="background" args={['#05080d']} />
        <Suspense fallback={null}>
          <BoardroomScene {...props} renderProfile={renderProfile} />
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

// sigil: REPAIR
import { Canvas } from '@react-three/fiber'
import { Html, OrbitControls, Stars, useGLTF } from '@react-three/drei'
import { Suspense, useMemo, type CSSProperties, type ReactNode } from 'react'
import type { Group } from 'three'
import type { SceneAnchorDefinition, SceneZoneDefinition } from '../systems/runtimeTypes'
import { DEFAULT_AGENT_PRESENCE_STATE } from '../systems/presenceState'
import type { AgentPresenceState } from '../systems/presenceTypes'
import SceneRuntimeCard from '../systems/SceneRuntimeCard'
import { getSceneAssetByBinding } from '../systems/sceneAssets'
import { useSceneMaterial } from '../systems/sceneMaterials'
import WorldDistrictPresenceCue from './WorldDistrictPresenceCue'
import { resolveWorldDistrictPresentation } from './worldDistrictPresentation'
import type { WorldDistrictUrgency } from './worldDistrictUrgency'
import type { WorldSurfaceLayout } from '../../lib/worldSurfaceSettings'
import WorldTerminalSurfacePreview from './WorldTerminalSurfacePreview'
import { deriveWorldTerminalSurfacePreviewModel } from './worldTerminalSurfacePreviewModel'

interface WorldRuntimeViewportProps {
  active: boolean
  debug?: boolean
  zones: SceneZoneDefinition[]
  anchors: SceneAnchorDefinition[]
  districtUrgencies?: Record<string, WorldDistrictUrgency>
  surfaceLayouts?: Record<string, WorldSurfaceLayout>
  presenceState?: AgentPresenceState
  onExit: () => void
}

const DISTRICT_ASSET_BINDINGS = [
  'district_command',
  'district_knowledge',
  'district_operations',
  'district_finance',
  'district_communications',
  'district_governance',
  'district_monitoring',
]

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
}) {
  const gltf = useGLTF(url)
  const scene = useMemo(() => gltf.scene.clone(true) as Group, [gltf.scene])
  return <primitive object={scene} {...props} />
}

function WorldRuntimeScene({
  zones,
  anchors,
  districtUrgencies = {},
  surfaceLayouts = {},
  presenceState = DEFAULT_AGENT_PRESENCE_STATE,
  debug = false,
  onExit,
}: Omit<WorldRuntimeViewportProps, 'active'>) {
  const districts = zones.filter((zone) => zone.scene === 'world')
  const terminals = anchors.filter((anchor) => anchor.type === 'terminal' || anchor.type === 'workstation_spawn').slice(0, 3)
  const groundMaterial = useSceneMaterial('world_ground_plate')
  const districtMaterial = useSceneMaterial('world_district_structure')
  const terminalMaterial = useSceneMaterial('world_terminal_housing')

  return (
    <>
      <ambientLight intensity={0.3} />
      <directionalLight position={[8, 12, 6]} intensity={1.15} color="#f2f7ff" castShadow />
      <pointLight position={[0, 5, 0]} intensity={1.4} distance={24} color="#5ec9ff" />
      <fog attach="fog" args={['#03070c', 12, 52]} />
      <Stars radius={80} depth={35} count={1400} factor={3} fade speed={0.7} />

      <SceneAssetModel
        binding="world_ground_plate"
        position={[0, -1.2, 0]}
        scale={1.8}
        fallback={(
          <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, -1.2, 0]} material={groundMaterial}>
            <planeGeometry args={[50, 50]} />
          </mesh>
        )}
      />

      {districts.map((zone, index) => {
        const columnHeight = 2.8 + (index % 4) * 1.35
        const binding = DISTRICT_ASSET_BINDINGS[index % DISTRICT_ASSET_BINDINGS.length]
        const presentation = resolveWorldDistrictPresentation(zone, districtUrgencies[zone.id])
        return (
          <group
            key={zone.id}
            position={[-10 + index * 3.2, columnHeight / 2 - 1.2, -2 - (index % 3) * 2.8]}
            userData={{
              sceneWorldDistrictId: zone.id,
              sceneWorldDistrictUrgency: presentation.tone,
              sceneWorldDisplayOnly: true,
            }}
          >
            <SceneAssetModel
              binding={binding}
              scale={0.9 + (index % 3) * 0.08}
              fallback={(
                <mesh material={districtMaterial}>
                  <boxGeometry args={[1.4, columnHeight, 1.4]} />
                </mesh>
              )}
            />
            <mesh position={[0, columnHeight / 2 + 0.12, 0]} rotation={[-Math.PI / 2, 0, 0]}>
              <ringGeometry args={[0.72, 0.86, 48]} />
              <meshStandardMaterial
                color={presentation.color}
                emissive={presentation.color}
                emissiveIntensity={presentation.tone === 'critical' || presentation.tone === 'blocked' ? 1.2 : 0.72}
                transparent
                opacity={0.58}
              />
            </mesh>
            <Html center distanceFactor={11}>
              <div
                className={`scene-anchor-label scene-anchor-label--display-only scene-anchor-label--urgency-${presentation.tone}`}
                style={{ '--district-urgency-color': presentation.color } as CSSProperties}
                title={presentation.detail}
              >
                {presentation.title}
                <span>{presentation.badge} · DISPLAY ONLY</span>
              </div>
            </Html>
          </group>
        )
      })}

      {terminals.map((anchor, index) => {
        const terminalSurfaceId = ['terminal_queue', 'terminal_tools', 'terminal_status'][index] ?? anchor.id
        const surfaceLayout = surfaceLayouts[terminalSurfaceId]
        const terminalPreview = deriveWorldTerminalSurfacePreviewModel({ terminalId: terminalSurfaceId, layout: surfaceLayout })
        return (
          <group
            key={anchor.id}
            position={[6 + index * 2.2, -0.2, 4 - index * 2]}
            userData={{
              sceneWorldTerminalId: terminalSurfaceId,
              sceneWorldTerminalSurfaceAdapter: terminalPreview.adapterType,
              sceneWorldTerminalPreviewWidgets: terminalPreview.widgets.length,
              sceneWorldDisplayOnly: true,
            }}
          >
            <SceneAssetModel
              binding={['center_console', 'systems_control', 'network_control'][index] ?? 'settings_control'}
              scale={0.7}
              fallback={(
                <mesh material={terminalMaterial}>
                  <cylinderGeometry args={[0.35, 0.35, 1.3, 24]} />
                </mesh>
              )}
            />
            <WorldTerminalSurfacePreview
              terminalId={terminalSurfaceId}
              layout={surfaceLayout}
              label={anchor.label}
            />
          </group>
        )
      })}

      <WorldDistrictPresenceCue presenceState={presenceState} zones={zones} />

      {debug ? (
        <Html position={[-12, 7, 0]} transform>
          <SceneRuntimeCard
            eyebrow="Scene Debug"
            title="World Runtime"
            variant="world"
            metrics={[
              { label: 'Districts', value: districts.length },
              { label: 'Anchors', value: anchors.length },
              { label: 'Terminals', value: terminals.length },
            ]}
            actions={[
              { label: 'Boardroom', onClick: onExit },
            ]}
          />
        </Html>
      ) : null}

      <OrbitControls enablePan={false} minDistance={10} maxDistance={24} maxPolarAngle={Math.PI / 2.05} target={[0, 0.5, 0]} />
    </>
  )
}

export default function WorldRuntimeViewport(props: WorldRuntimeViewportProps) {
  return (
    <div className="scene-runtime-canvas scene-runtime-canvas--world">
      <Canvas camera={{ position: [0, 9, 16], fov: 48 }} dpr={[1, 2]} frameloop={props.active ? 'always' : 'never'}>
        <color attach="background" args={['#03070c']} />
        <Suspense fallback={null}>
          <WorldRuntimeScene {...props} />
        </Suspense>
      </Canvas>
    </div>
  )
}

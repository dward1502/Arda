// sigil: REPAIR

export interface SceneAssetMetadata {
  id: string
  domain: 'window' | 'hologram' | 'world' | 'materials'
  scene_binding: string
  material_family: string
  source: string
  license: string
  files?: Record<string, string>
}

const worldMetadataModules = import.meta.glob<SceneAssetMetadata>(
  '/src/assets/scene/world/*/metadata.json',
  { eager: true, import: 'default' },
)

const hologramMetadataModules = import.meta.glob<SceneAssetMetadata>(
  '/src/assets/scene/hologram/*/metadata.json',
  { eager: true, import: 'default' },
)

const windowMetadataModules = import.meta.glob<SceneAssetMetadata>(
  '/src/assets/scene/window/*/metadata.json',
  { eager: true, import: 'default' },
)

const glbUrls = import.meta.glob<string>(
  [
    '/src/assets/scene/{world,hologram}/*/*.glb',
    '!/src/assets/scene/world/boardroom_main_desk/*.glb',
    '!/src/assets/scene/world/boardroom_main_desk_flux2/*.glb',
  ],
  { eager: true, import: 'default', query: '?url' },
)

const windowUrls = import.meta.glob<string>(
  '/src/assets/scene/window/*/*.{hdr,jpg,png}',
  { eager: true, import: 'default', query: '?url' },
)

interface SceneAssetEntry {
  metadata: SceneAssetMetadata
  folder: string
  glbUrl: string | null
}

const ASSETS_BY_BINDING: Record<string, SceneAssetEntry> = {}
const WINDOW_ASSETS_BY_ID: Record<string, { metadata: SceneAssetMetadata; folder: string }> = {}

for (const [path, metadata] of Object.entries({
  ...worldMetadataModules,
  ...hologramMetadataModules,
})) {
  const folder = path.replace(/\/metadata\.json$/, '')
  const glbUrl = glbUrls[`${folder}/${metadata.scene_binding}.glb`] ?? glbUrls[`${folder}/${metadata.id}.glb`] ?? null
  ASSETS_BY_BINDING[metadata.scene_binding] = {
    metadata,
    folder,
    glbUrl,
  }
}

for (const [path, metadata] of Object.entries(windowMetadataModules)) {
  WINDOW_ASSETS_BY_ID[metadata.id] = {
    metadata,
    folder: path.replace(/\/metadata\.json$/, ''),
  }
}

export function getSceneAssetByBinding(binding: string): SceneAssetEntry | null {
  return ASSETS_BY_BINDING[binding] ?? null
}

export function getWindowAssetUrl(assetId: string, fileKey: string, fallbackFilename: string): string | null {
  const entry = WINDOW_ASSETS_BY_ID[assetId]
  if (!entry) return null
  const filename = entry.metadata.files?.[fileKey] ?? fallbackFilename
  return windowUrls[`${entry.folder}/${filename}`] ?? null
}

// sigil: REPAIR
import * as THREE from 'three'
import { useMemo } from 'react'

export type TextureChannel =
  | 'albedo'
  | 'normal'
  | 'roughness'
  | 'metalness'
  | 'ao'
  | 'emissive'
  | 'mask'

export interface MaterialMetadata {
  id: string
  domain: 'window' | 'hologram' | 'world' | 'materials'
  material_family: string
  scene_binding: string
  source: string
  license: string
  channels: Partial<Record<TextureChannel, string>>
  status: 'pending_textures' | 'active'
  notes?: string
}

const metadataModules = import.meta.glob<MaterialMetadata>(
  '/src/assets/scene/materials/*/metadata.json',
  { eager: true, import: 'default' },
)

const textureUrls = import.meta.glob<string>(
  [
    '/src/assets/scene/materials/*/*.{png,jpg,ktx2}',
    '!/src/assets/scene/materials/boardroom_wall/boardroom_wall_ao.png',
    '!/src/assets/scene/materials/boardroom_wall/boardroom_wall_emissive.png',
    '!/src/assets/scene/materials/boardroom_floor/boardroom_floor_ao.jpg',
    '!/src/assets/scene/materials/boardroom_monitor_bezel/boardroom_monitor_bezel_ao.jpg',
    '!/src/assets/scene/materials/boardroom_monitor_screen/boardroom_monitor_screen_ao.jpg',
  ],
  { eager: true, import: 'default', query: '?url' },
)

interface FamilyEntry {
  meta: MaterialMetadata
  folder: string
}

const FAMILY_INDEX: Record<string, FamilyEntry> = {}
for (const [path, meta] of Object.entries(metadataModules)) {
  if (!meta || typeof meta !== 'object' || !('material_family' in meta)) continue
  FAMILY_INDEX[meta.material_family] = {
    meta,
    folder: path.replace(/\/metadata\.json$/, ''),
  }
}

interface TransitionalFallback {
  color: string
  roughness: number
  metalness: number
  emissive?: string
  emissiveIntensity?: number
}

const TRANSITIONAL_FALLBACKS: Record<string, TransitionalFallback> = {
  boardroom_floor: { color: '#10161f', roughness: 0.88, metalness: 0.15 },
  boardroom_wall: {
    color: '#0a1320',
    roughness: 0.72,
    metalness: 0.0,
    emissive: '#123654',
    emissiveIntensity: 0.4,
  },
  boardroom_desk: { color: '#1a2430', roughness: 0.52, metalness: 0.45 },
}

const textureCache = new Map<string, THREE.Texture>()
const textureLoader = new THREE.TextureLoader()

function loadTexture(url: string, isColor: boolean): THREE.Texture {
  const cached = textureCache.get(url)
  if (cached) return cached
  const tex = textureLoader.load(url)
  if (isColor) {
    tex.colorSpace = THREE.SRGBColorSpace
  }
  textureCache.set(url, tex)
  return tex
}

export function getSceneMaterial(familyId: string): THREE.MeshStandardMaterial {
  const entry = FAMILY_INDEX[familyId]
  if (!entry) {
    throw new Error(
      `[sceneMaterials] No metadata sidecar for material family "${familyId}". ` +
        `Add src/assets/scene/materials/${familyId}/metadata.json before requesting this family.`,
    )
  }
  const { meta, folder } = entry

  if (meta.status === 'pending_textures') {
    const fallback = TRANSITIONAL_FALLBACKS[familyId]
    if (!fallback) {
      throw new Error(
        `[sceneMaterials] Family "${familyId}" is pending_textures but has no transitional fallback entry.`,
      )
    }
    return new THREE.MeshStandardMaterial({
      color: fallback.color,
      roughness: fallback.roughness,
      metalness: fallback.metalness,
      emissive: fallback.emissive ? new THREE.Color(fallback.emissive) : undefined,
      emissiveIntensity: fallback.emissiveIntensity ?? 0,
    })
  }

  const params: THREE.MeshStandardMaterialParameters = {}
  for (const [channel, filename] of Object.entries(meta.channels)) {
    if (!filename) continue
    const url = textureUrls[`${folder}/${filename}`]
    if (!url) {
      throw new Error(
        `[sceneMaterials] Family "${familyId}" declares ${channel} = "${filename}" but the file is missing in ${folder}/.`,
      )
    }
    switch (channel as TextureChannel) {
      case 'albedo':
        params.map = loadTexture(url, true)
        break
      case 'normal':
        params.normalMap = loadTexture(url, false)
        break
      case 'roughness':
        params.roughnessMap = loadTexture(url, false)
        break
      case 'metalness':
        params.metalnessMap = loadTexture(url, false)
        break
      case 'ao':
        params.aoMap = loadTexture(url, false)
        break
      case 'emissive':
        params.emissiveMap = loadTexture(url, true)
        params.emissive = new THREE.Color('#ffffff')
        break
      case 'mask':
        params.alphaMap = loadTexture(url, false)
        params.transparent = true
        break
    }
  }
  return new THREE.MeshStandardMaterial(params)
}

export function useSceneMaterial(familyId: string): THREE.MeshStandardMaterial {
  return useMemo(() => getSceneMaterial(familyId), [familyId])
}

export function listSceneMaterialFamilies(): string[] {
  return Object.keys(FAMILY_INDEX)
}

import * as THREE from 'three';

// Desk Material - Semi-reflective Obsidian
export const boardroomDeskMaterial = new THREE.MeshPhysicalMaterial({
  color: 0x111111,                    // Deep obsidian black
  metalness: 0.92,
  roughness: 0.18,                    // Glossy but not mirror
  clearcoat: 0.6,
  clearcoatRoughness: 0.1,
  envMapIntensity: 1.2,
  
  // Maps (loaded via your asset pipeline)
  map: deskAlbedoTexture,
  normalMap: deskNormalTexture,
  roughnessMap: deskRoughnessTexture,
  metalnessMap: deskMetalnessTexture,
  emissiveMap: deskEmissiveTexture,
  emissive: 0x00ffff,                 // Base cyan glow
  emissiveIntensity: 2.5,
  
  side: THREE.DoubleSide,
});

// Monitor Bezel
export const monitorBezelMaterial = new THREE.MeshPhysicalMaterial({
  color: 0x0a0a0a,
  metalness: 0.85,
  roughness: 0.35,
  normalMap: bezelNormalTexture,
  emissive: 0x22ffcc,
  emissiveIntensity: 1.8,
});

// Holographic Screen (for content projection)
export const holographicScreenMaterial = new THREE.MeshPhysicalMaterial({
  color: 0x000000,
  metalness: 0.1,
  roughness: 0.05,
  transmission: 0.85,                 // Glass-like
  thickness: 0.1,
  envMapIntensity: 2.0,
  emissive: 0x88ffff,
  emissiveIntensity: 0.8,
  // Will blend with React HTML overlays or CanvasTexture for live content
});
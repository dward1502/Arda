import { useState } from 'react';
import { useThree } from '@react-three/fiber';
import * as THREE from 'three';

function loadTexture(url: string): THREE.Texture | null {
  try {
    const loader = new THREE.TextureLoader();
    return loader.load(url, undefined, undefined, () => null);
  } catch {
    return null;
  }
}

export default function Background() {
  const { gl } = useThree();
  const [skyTexture] = useState(() => loadTexture('/artifacts/bg-milkyway.jpg'));
  if (!skyTexture) return null;
  skyTexture.minFilter = THREE.LinearMipmapLinearFilter;
  skyTexture.magFilter = THREE.LinearFilter;
  skyTexture.anisotropy = gl.capabilities.getMaxAnisotropy() || 16;
  skyTexture.wrapS = skyTexture.wrapT = THREE.RepeatWrapping;
  skyTexture.colorSpace = THREE.SRGBColorSpace;

  return (
    <group>
      <mesh position={[0, 0, -27]} rotation={[0.02, 0, 0]}>
        <planeGeometry args={[54, 36]} />
        <meshBasicMaterial map={skyTexture} />
      </mesh>
    </group>
  );
}
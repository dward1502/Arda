import { useTexture } from "@react-three/drei";
import { useThree, useFrame } from "@react-three/fiber";
import * as THREE from "three";

export default function Background() {
  const { gl } = useThree();

  const skyTexture = useTexture("/artifacts/bg-milkyway.jpg");   // ← full moving sky
  // const landTexture = useTexture("/artifacts/bg-ground.png");    // ← transparent ground (must be PNG)

  // Sky texture settings + slow movement
  skyTexture.minFilter = THREE.LinearMipmapLinearFilter;
  skyTexture.magFilter = THREE.LinearFilter;
  skyTexture.anisotropy = gl.capabilities.getMaxAnisotropy() || 16;
  skyTexture.wrapS = skyTexture.wrapT = THREE.RepeatWrapping;
  skyTexture.colorSpace = THREE.SRGBColorSpace;

  // Land texture settings
  // landTexture.minFilter = THREE.LinearFilter;
  // landTexture.magFilter = THREE.LinearFilter;
  // landTexture.anisotropy = 8;
  // landTexture.colorSpace = THREE.SRGBColorSpace;

  // Animate only the sky
  // useFrame((state) => {
  //   skyTexture.offset.x = state.clock.elapsedTime * 0.001;
  // });

  return (
    <group>
      {/* Moving Sky - Far back */}
      <mesh position={[0, 0, -27]} rotation={[0.02, 0, 0]}>
        <planeGeometry args={[54, 36]} />
        <meshBasicMaterial map={skyTexture} />
      </mesh>
    </group>
  );
}
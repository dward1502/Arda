export type BoardroomVisualLandmark =
  | 'five-upper-monitors'
  | 'five-lower-console-zones'
  | 'four-physical-controls'
  | 'center-avatar-emitter'
  | 'fail-closed-control-state'

export interface BoardroomVisualRegressionScenario {
  id: 'web-full-fidelity' | 'web-degraded-state'
  runtimeState: 'default-profile' | 'degraded'
  viewport: { width: 1278; height: 513 }
  captureKind: 'webgl-canvas'
  baselinePath: string
  requiredLandmarks: BoardroomVisualLandmark[]
}

const STRUCTURAL_LANDMARKS: BoardroomVisualLandmark[] = [
  'five-upper-monitors',
  'five-lower-console-zones',
  'four-physical-controls',
  'center-avatar-emitter',
]

export const BOARDROOM_VISUAL_REGRESSION_SCENARIOS: BoardroomVisualRegressionScenario[] = [
  {
    id: 'web-full-fidelity',
    runtimeState: 'default-profile',
    viewport: { width: 1278, height: 513 },
    captureKind: 'webgl-canvas',
    baselinePath: '__visual_baselines__/boardroom-default-web.png',
    requiredLandmarks: STRUCTURAL_LANDMARKS,
  },
  {
    id: 'web-degraded-state',
    runtimeState: 'degraded',
    viewport: { width: 1278, height: 513 },
    captureKind: 'webgl-canvas',
    baselinePath: '__visual_baselines__/boardroom-degraded-web.png',
    requiredLandmarks: [...STRUCTURAL_LANDMARKS, 'fail-closed-control-state'],
  },
]

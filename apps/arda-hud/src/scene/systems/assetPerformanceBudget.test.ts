// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { analyzeArdaAssetBudget } from './assetPerformanceBudget'

describe('asset performance budget', () => {
  it('flags oversized runtime model and texture assets', () => {
    const report = analyzeArdaAssetBudget([
      { path: 'dist/assets/boardroom_main_desk-ZfJB1Q42.glb', bytes: 29_575_352 },
      { path: 'dist/assets/boardroom_main_desk_flux2-BW9jZosy.glb', bytes: 22_105_868 },
      { path: 'dist/assets/boardroom_wall_ao-H8Bri2t8.png', bytes: 7_978_084 },
      { path: 'dist/assets/vendor-three-core-C4SSXDVm.js', bytes: 724_936 },
    ])

    expect(report.totalSeverity).toBe('fail')
    expect(report.findings.slice(0, 3).map((finding) => finding.path)).toEqual([
      'dist/assets/boardroom_main_desk-ZfJB1Q42.glb',
      'dist/assets/boardroom_main_desk_flux2-BW9jZosy.glb',
      'dist/assets/boardroom_wall_ao-H8Bri2t8.png',
    ])
    expect(report.findings[0]).toMatchObject({
      kind: 'model',
      severity: 'fail',
      thresholdBytes: 5_000_000,
    })
    expect(report.findings[2].recommendation).toContain('compressed GPU texture')
    expect(report.findings.find((finding) => finding.kind === 'script')).toMatchObject({
      severity: 'warn',
      thresholdBytes: 750_000,
    })
  })

  it('keeps small split assets below warning thresholds', () => {
    const report = analyzeArdaAssetBudget([
      { path: 'dist/assets/district_command.glb', bytes: 40_000 },
      { path: 'dist/assets/world_ground_plate_albedo.png', bytes: 820_000 },
      { path: 'dist/assets/boardroom_environment.hdr', bytes: 2_000_000 },
      { path: 'dist/assets/index.js', bytes: 450_000 },
    ])

    expect(report.totalSeverity).toBe('ok')
    expect(report.findings).toEqual([])
    expect(report.countsByKind).toMatchObject({
      model: 1,
      texture: 1,
      environment: 1,
      script: 1,
    })
  })

  it('keeps the optimized default runtime asset profile below failure thresholds after starter deferral', () => {
    const report = analyzeArdaAssetBudget([
      { path: 'dist/assets/center_console-D4RPipqJ.glb', bytes: 2_586_524 },
      { path: 'dist/assets/boardroom_environment-CJDuDhPb.hdr', bytes: 2_097_202 },
      { path: 'dist/assets/world_route_marker_albedo-SxYJFI31.png', bytes: 989_654 },
      { path: 'dist/assets/world_terminal_housing_albedo-B1sNTqqs.png', bytes: 939_186 },
      { path: 'dist/assets/vendor-three-core-C4SSXDVm.js', bytes: 724_936 },
      { path: 'dist/assets/index-J5-POJAL.js', bytes: 448_956 },
    ])

    expect(report.findings.filter((finding) => finding.severity === 'fail')).toEqual([])
    expect(report.totalBytes).toBeLessThan(65_000_000)
  })

  it('flags total build payload even when individual assets are below hard limits', () => {
    const report = analyzeArdaAssetBudget(
      Array.from({ length: 40 }, (_, index) => ({
        path: `dist/assets/surface_${index}.png`,
        bytes: 1_800_000,
      })),
    )

    expect(report.totalBytes).toBe(72_000_000)
    expect(report.totalSeverity).toBe('fail')
    expect(report.findings.every((finding) => finding.severity === 'warn')).toBe(true)
  })
})

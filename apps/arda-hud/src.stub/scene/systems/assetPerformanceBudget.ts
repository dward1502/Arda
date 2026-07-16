// sigil: REPAIR
export type ArdaAssetKind = 'model' | 'texture' | 'environment' | 'script' | 'style' | 'other'
export type ArdaAssetBudgetSeverity = 'ok' | 'warn' | 'fail'

export interface ArdaBuildAssetEntry {
  path: string
  bytes: number
}

export interface ArdaAssetBudgetThresholds {
  maxModelBytes: number
  maxTextureBytes: number
  maxEnvironmentBytes: number
  maxScriptBytes: number
  maxTotalBuildAssetBytes: number
}

export interface ArdaAssetBudgetFinding {
  path: string
  bytes: number
  kind: ArdaAssetKind
  severity: ArdaAssetBudgetSeverity
  thresholdBytes: number
  recommendation: string
}

export interface ArdaAssetBudgetReport {
  totalBytes: number
  totalSeverity: ArdaAssetBudgetSeverity
  findings: ArdaAssetBudgetFinding[]
  countsByKind: Record<ArdaAssetKind, number>
}

export const ARDA_ASSET_BUDGET_THRESHOLDS: ArdaAssetBudgetThresholds = {
  maxModelBytes: 5_000_000,
  maxTextureBytes: 2_000_000,
  maxEnvironmentBytes: 3_000_000,
  maxScriptBytes: 750_000,
  maxTotalBuildAssetBytes: 65_000_000,
}

function assetKind(path: string): ArdaAssetKind {
  const lower = path.toLowerCase()
  if (lower.endsWith('.glb') || lower.endsWith('.gltf')) return 'model'
  if (lower.endsWith('.hdr')) return 'environment'
  if (lower.endsWith('.png') || lower.endsWith('.jpg') || lower.endsWith('.jpeg') || lower.endsWith('.ktx2') || lower.endsWith('.webp')) return 'texture'
  if (lower.endsWith('.js')) return 'script'
  if (lower.endsWith('.css')) return 'style'
  return 'other'
}

function thresholdForKind(kind: ArdaAssetKind, thresholds: ArdaAssetBudgetThresholds): number {
  switch (kind) {
    case 'model':
      return thresholds.maxModelBytes
    case 'texture':
      return thresholds.maxTextureBytes
    case 'environment':
      return thresholds.maxEnvironmentBytes
    case 'script':
      return thresholds.maxScriptBytes
    default:
      return Number.POSITIVE_INFINITY
  }
}

function severityForBytes(bytes: number, thresholdBytes: number): ArdaAssetBudgetSeverity {
  if (!Number.isFinite(thresholdBytes)) return 'ok'
  if (bytes > thresholdBytes) return 'fail'
  if (bytes > thresholdBytes * 0.75) return 'warn'
  return 'ok'
}

function recommendationFor(kind: ArdaAssetKind, severity: ArdaAssetBudgetSeverity): string {
  if (severity === 'ok') return 'within budget'
  switch (kind) {
    case 'model':
      return 'decimate mesh, remove embedded high-res texture payloads, or split/lazy-load focused model'
    case 'texture':
      return 'resize, convert to compressed GPU texture, or replace full-resolution channel with procedural/material fallback'
    case 'environment':
      return 'downsample HDRI or ship a preview LDR plate for non-reflection use'
    case 'script':
      return 'split chunk or defer non-critical scene/runtime code'
    default:
      return 'review asset before promoting to runtime bundle'
  }
}

function maxSeverity(a: ArdaAssetBudgetSeverity, b: ArdaAssetBudgetSeverity): ArdaAssetBudgetSeverity {
  const rank: Record<ArdaAssetBudgetSeverity, number> = { ok: 0, warn: 1, fail: 2 }
  return rank[b] > rank[a] ? b : a
}

export function analyzeArdaAssetBudget(
  assets: ArdaBuildAssetEntry[],
  thresholds: ArdaAssetBudgetThresholds = ARDA_ASSET_BUDGET_THRESHOLDS,
): ArdaAssetBudgetReport {
  const countsByKind: Record<ArdaAssetKind, number> = {
    model: 0,
    texture: 0,
    environment: 0,
    script: 0,
    style: 0,
    other: 0,
  }

  const findings = assets
    .map((asset): ArdaAssetBudgetFinding => {
      const kind = assetKind(asset.path)
      const thresholdBytes = thresholdForKind(kind, thresholds)
      const severity = severityForBytes(asset.bytes, thresholdBytes)
      countsByKind[kind] += 1
      return {
        path: asset.path,
        bytes: asset.bytes,
        kind,
        severity,
        thresholdBytes,
        recommendation: recommendationFor(kind, severity),
      }
    })
    .filter((finding) => finding.severity !== 'ok')
    .sort((a, b) => b.bytes - a.bytes)

  const totalBytes = assets.reduce((sum, asset) => sum + Math.max(0, asset.bytes), 0)
  const totalSeverity = maxSeverity(
    severityForBytes(totalBytes, thresholds.maxTotalBuildAssetBytes),
    findings.reduce<ArdaAssetBudgetSeverity>((severity, finding) => maxSeverity(severity, finding.severity), 'ok'),
  )

  return {
    totalBytes,
    totalSeverity,
    findings,
    countsByKind,
  }
}

import type { ReactNode } from 'react'

export interface ArdaRuntimeModeStatus {
  label: string
  mode: string
  detail: string
  dataSource: string
  isTauri: boolean
  isDev: boolean
  nativeProofRequired: boolean
}

export function detectArdaRuntimeMode(): ArdaRuntimeModeStatus {
  const dataSource = 'env'
  const isTauri = typeof window !== 'undefined' && !!(window as unknown as Record<string, unknown>).__TAURI__
  const isDev = process.env.NODE_ENV !== 'production'
  const nativeProofRequired = isTauri && !isDev

  let mode = 'native'
  let detail = 'Tauri desktop shell detected'

  if (mode !== 'native') {
    mode = 'web'
    detail = 'Detected runtime uses web transport.'
  }

  if (isDev) {
    detail = `${detail} (development profile active) `
  }

  if (nativeProofRequired) {
    detail = `${detail} Native proof required.`
  }

  return {
    label: `${mode.toUpperCase()} runtime`,
    mode,
    detail,
    dataSource,
    isTauri,
    isDev,
    nativeProofRequired,
  }
}


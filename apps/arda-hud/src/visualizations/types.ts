import type { ComponentType, ReactNode } from 'react'
import type { ModuleId } from '../components/arda/core/types' // adjust path if needed

export interface VisualizationTemplateProps<T = unknown> {
  data: T
  moduleId: ModuleId
  title?: string
  config?: Record<string, unknown>
  onAction?: (action: string, payload?: unknown) => void
}

export interface VisualizationTemplate<T = unknown> {
  id: string
  label: string
  description?: string
  supportedDataTypes?: string[]
  component: ComponentType<VisualizationTemplateProps<T>>
}

export interface ModuleVisualizationConfig {
  moduleId: ModuleId
  defaultTemplateId: string
  userOverrideTemplateId?: string | null
}
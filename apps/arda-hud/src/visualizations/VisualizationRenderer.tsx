import type { ModuleId } from '../components/arda/core/types' // adjust path if needed
import './register'
import { getTemplate, resolveTemplateId } from './registry'
import type { ModuleVisualizationConfig } from './types'

// Temporary in-memory configs. Later move this to a Zustand store or settings.
const defaultConfigs: ModuleVisualizationConfig[] = [
  // Add real module defaults here later
  // { moduleId: 'governance_controls', defaultTemplateId: 'structured' },
]

interface VisualizationRendererProps {
  moduleId: ModuleId
  data: unknown
  title?: string
  config?: Record<string, unknown>
  onAction?: (action: string, payload?: unknown) => void
  /** Optional: pass live user configs from a store */
  visualizationConfigs?: ModuleVisualizationConfig[]
}

export function VisualizationRenderer({
  moduleId,
  data,
  title,
  config,
  onAction,
  visualizationConfigs = defaultConfigs,
}: VisualizationRendererProps) {
  const templateId = resolveTemplateId(moduleId, visualizationConfigs)
  const template = getTemplate(templateId)

  if (!template) {
    return (
      <div style={{ padding: '1rem', color: 'var(--text-muted)' }}>
        No visualization template found for “{templateId}”.
      </div>
    )
  }

  const TemplateComponent = template.component

  return (
    <TemplateComponent
      data={data}
      moduleId={moduleId}
      title={title}
      config={config}
      onAction={onAction}
    />
  )
}
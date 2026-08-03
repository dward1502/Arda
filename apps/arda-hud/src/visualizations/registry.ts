import type { VisualizationTemplate } from './types'

const templateRegistry = new Map<string, VisualizationTemplate>()

export function registerTemplate(template: VisualizationTemplate) {
  if (templateRegistry.has(template.id)) {
    console.warn(`[visualization] Template "${template.id}" is already registered. Overwriting.`)
  }
  templateRegistry.set(template.id, template)
}

export function getTemplate(id: string): VisualizationTemplate | undefined {
  return templateRegistry.get(id)
}

export function listTemplates(): VisualizationTemplate[] {
  return Array.from(templateRegistry.values())
}

export function resolveTemplateId(
  moduleId: string,
  configs: { moduleId: string; defaultTemplateId: string; userOverrideTemplateId?: string | null }[],
  fallback = 'structured',
): string {
  const config = configs.find((c) => c.moduleId === moduleId)
  return config?.userOverrideTemplateId || config?.defaultTemplateId || fallback
}
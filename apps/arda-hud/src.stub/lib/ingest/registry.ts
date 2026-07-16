import { ComponentType, ComponentPayload } from './types'

export interface ComponentRegistryEntry {
  type: ComponentType
  label: string
  description: string
  defaultOptions: Record<string, unknown>
}

export const COMPONENT_REGISTRY: ComponentRegistryEntry[] = [
  {
    type: 'kanban',
    label: 'Kanban Board',
    description: 'Swimlanes with draggable cards',
    defaultOptions: {
      lanes: ['todo', 'in-progress', 'done'],
      showTags: true,
      showAssignee: true,
    },
  },
  {
    type: 'grid',
    label: 'Data Grid',
    description: 'Sortable, filterable table',
    defaultOptions: {
      pageSize: 20,
      sortable: true,
      filterable: true,
    },
  },
  {
    type: 'metric',
    label: 'Metric Pills',
    description: 'Glowing stat cards',
    defaultOptions: {
      layout: 'auto',
      showTrend: true,
    },
  },
  {
    type: 'timeline',
    label: 'Timeline',
    description: 'Chronological event stream',
    defaultOptions: {
      direction: 'vertical',
      showCategories: true,
    },
  },
  {
    type: 'graph3d',
    label: '3D Graph',
    description: 'Force-directed network visualization',
    defaultOptions: {
      layout: 'force',
      showLabels: true,
      physics: true,
    },
  },
  {
    type: 'document',
    label: 'Document Reader',
    description: 'Markdown/content viewer',
    defaultOptions: {
      toc: true,
      codeHighlighting: true,
    },
  },
  {
    type: 'status-matrix',
    label: 'Status Matrix',
    description: 'Grid with state indicators',
    defaultOptions: {
      showLegend: true,
    },
  },
  {
    type: 'stream-log',
    label: 'Stream Log',
    description: 'Terminal-style live feed',
    defaultOptions: {
      maxLines: 100,
      autoScroll: true,
    },
  },
  {
    type: 'chart',
    label: 'Chart',
    description: 'Bar, line, pie, area charts',
    defaultOptions: {
      chartType: 'bar',
      responsive: true,
    },
  },
  {
    type: 'list',
    label: 'List',
    description: 'Simple item list',
    defaultOptions: {
      sortable: false,
      selectable: false,
    },
  },
]

export function getComponentEntry(type: ComponentType): ComponentRegistryEntry | undefined {
  return COMPONENT_REGISTRY.find(entry => entry.type === type)
}

export function getAllComponentTypes(): ComponentType[] {
  return COMPONENT_REGISTRY.map(entry => entry.type)
}

export function getComponentLabel(type: ComponentType): string {
  return getComponentEntry(type)?.label || type
}

export function getComponentDescription(type: ComponentType): string {
  return getComponentEntry(type)?.description || ''
}

// sigil: INGEST
export type DataFormat = 'auto' | 'json' | 'markdown' | 'yaml' | 'csv' | 'text'

export type ComponentType = 
  | 'kanban'
  | 'grid'
  | 'metric'
  | 'timeline'
  | 'graph3d'
  | 'document'
  | 'status-matrix'
  | 'stream-log'
  | 'chart'
  | 'list'

export interface IngestRequest {
  source: string
  format: DataFormat
  componentType?: ComponentType | 'auto'
  options?: RenderOptions
}

export interface RenderOptions {
  title?: string
  refreshInterval?: number
  maxItems?: number
  columns?: string[]
  sortable?: boolean
  filterable?: boolean
}

export interface ParsedData {
  success: boolean
  format: DataFormat
  data: unknown
  metadata: DataMetadata
  componentPayload: ComponentPayload
}

export interface DataMetadata {
  source: string
  parsedAt: number
  itemCount: number
  schema?: string
}

export type ComponentPayload = 
  | KanbanPayload
  | GridPayload
  | MetricPayload
  | TimelinePayload
  | GraphPayload
  | DocumentPayload
  | MatrixPayload
  | StreamPayload
  | ChartPayload
  | ListPayload

export interface KanbanPayload {
  type: 'kanban'
  lanes: KanbanLane[]
}

export interface KanbanLane {
  id: string
  title: string
  items: KanbanItem[]
}

export interface KanbanItem {
  id: string
  title: string
  description?: string
  tags?: string[]
  priority?: 'low' | 'medium' | 'high'
  assignee?: string
}

export interface GridPayload {
  type: 'grid'
  columns: GridColumn[]
  rows: Record<string, unknown>[]
}

export interface GridColumn {
  key: string
  label: string
  sortable?: boolean
  filterable?: boolean
  width?: number
}

export interface MetricPayload {
  type: 'metric'
  items: MetricItem[]
}

export interface MetricItem {
  id: string
  label: string
  value: string | number
  trend?: 'up' | 'down' | 'stable'
  trendValue?: number
  unit?: string
}

export interface TimelinePayload {
  type: 'timeline'
  events: TimelineEvent[]
  direction?: 'vertical' | 'horizontal'
}

export interface TimelineEvent {
  id: string
  timestamp: number
  title: string
  description?: string
  category?: string
}

export interface GraphPayload {
  type: 'graph3d'
  nodes: GraphNode[]
  edges: GraphEdge[]
}

export interface GraphNode {
  id: string
  label: string
  group?: string
  value?: number
}

export interface GraphEdge {
  source: string
  target: string
  label?: string
  value?: number
}

export interface DocumentPayload {
  type: 'document'
  title: string
  content: string
  sections?: DocumentSection[]
}

export interface DocumentSection {
  id: string
  title: string
  level: number
  content: string
}

export interface MatrixPayload {
  type: 'status-matrix'
  items: MatrixItem[]
  states: string[]
}

export interface MatrixItem {
  id: string
  label: string
  states: Record<string, boolean>
}

export interface StreamPayload {
  type: 'stream-log'
  lines: StreamLine[]
  maxLines?: number
}

export interface StreamLine {
  id: string
  timestamp: number
  level: 'info' | 'warn' | 'error' | 'debug'
  message: string
}

export interface ChartPayload {
  type: 'chart'
  chartType: 'bar' | 'line' | 'pie' | 'area'
  data: ChartData
}

export interface ChartData {
  labels: string[]
  datasets: ChartDataset[]
}

export interface ChartDataset {
  label: string
  data: number[]
  color?: string
}

export interface ListPayload {
  type: 'list'
  items: ListItem[]
}

export interface ListItem {
  id: string
  label: string
  value?: string
  icon?: string
  action?: string
}

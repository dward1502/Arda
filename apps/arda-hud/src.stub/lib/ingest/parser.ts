import {
  DataFormat,
  ComponentType,
  IngestRequest,
  ParsedData,
  ComponentPayload,
  KanbanPayload,
  GridPayload,
  MetricPayload,
  TimelinePayload,
  GraphPayload,
  DocumentPayload,
  MatrixPayload,
  StreamPayload,
  ListPayload,
} from './types'

function detectFormat(content: string, filename?: string): DataFormat {
  if (filename) {
    const ext = filename.toLowerCase()
    if (ext.endsWith('.json')) return 'json'
    if (ext.endsWith('.yaml') || ext.endsWith('.yml')) return 'yaml'
    if (ext.endsWith('.md') || ext.endsWith('.markdown')) return 'markdown'
    if (ext.endsWith('.csv') || ext.endsWith('.tsv')) return 'csv'
  }
  const trimmed = content.trim()
  if (trimmed.startsWith('{') || trimmed.startsWith('[')) return 'json'
  if (trimmed.startsWith('#') || trimmed.includes('\n## ')) return 'markdown'
  if (trimmed.includes(':') && !trimmed.includes(',')) return 'yaml'
  if (trimmed.includes(',') && trimmed.includes('\n')) return 'csv'
  return 'text'
}

function parseJSON(content: string): unknown {
  return JSON.parse(content)
}

function parseYAML(content: string): unknown {
  const lines = content.split('\n')
  const result: Record<string, unknown> = {}
  let currentKey = ''
  let currentArray: unknown[] = []
  let inArray = false
  
  for (const line of lines) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue
    
    const keyMatch = trimmed.match(/^(\w+):/)
    if (keyMatch) {
      if (inArray && currentKey) {
        result[currentKey] = currentArray
        currentArray = []
        inArray = false
      }
      currentKey = keyMatch[1]
      const valueMatch = trimmed.match(/:\s*(.+)$/)
      if (valueMatch) {
        const value = valueMatch[1].trim()
        if (value === '[') {
          inArray = true
        } else {
          result[currentKey] = value.replace(/['"]/g, '')
          currentKey = ''
        }
      }
    } else if (trimmed.startsWith('-')) {
      const value = trimmed.slice(1).trim().replace(/['"]/g, '')
      currentArray.push(value)
    }
  }
  if (inArray && currentKey) {
    result[currentKey] = currentArray
  }
  return result
}

function parseCSV(content: string): { headers: string[], rows: string[][] } {
  const lines = content.trim().split('\n')
  if (lines.length === 0) return { headers: [], rows: [] }
  
  const headers = lines[0].split(',').map(h => h.trim())
  const rows = lines.slice(1).map(line => 
    line.split(',').map(cell => cell.trim())
  )
  
  return { headers, rows }
}

function parseMarkdown(content: string): { title: string, sections: { level: number, title: string, content: string }[] } {
  const lines = content.split('\n')
  const result = { title: '', sections: [] as { level: number, title: string, content: string }[] }
  
  let currentSection = { level: 1, title: '', content: '' }
  let inCodeBlock = false
  
  for (const line of lines) {
    if (line.trim().startsWith('```')) {
      inCodeBlock = !inCodeBlock
      currentSection.content += line + '\n'
      continue
    }
    
    if (!inCodeBlock) {
      const headingMatch = line.match(/^(#{1,6})\s+(.+)$/)
      if (headingMatch) {
        if (currentSection.title || currentSection.content) {
          result.sections.push(currentSection)
        }
        currentSection = {
          level: headingMatch[1].length,
          title: headingMatch[2],
          content: ''
        }
        if (currentSection.level === 1 && !result.title) {
          result.title = currentSection.title
        }
      } else {
        currentSection.content += line + '\n'
      }
    }
  }
  
  if (currentSection.title || currentSection.content) {
    result.sections.push(currentSection)
  }
  
  return result
}

function inferComponentType(data: unknown, preferred?: ComponentType): ComponentType {
  if (preferred) return preferred
  
  if (Array.isArray(data)) {
    if (data.length === 0) return 'list'
    const first = data[0]
    if (typeof first === 'object' && first !== null) {
      const keys = Object.keys(first)
      if (keys.includes('status') || keys.includes('lane')) return 'kanban'
      if (keys.includes('timestamp') || keys.includes('time')) return 'timeline'
      if (keys.includes('source') && keys.includes('target')) return 'graph3d'
      if (keys.includes('level')) return 'stream-log'
    }
    if (typeof first === 'number' || typeof first === 'string') return 'metric'
    return 'list'
  }
  
  if (typeof data === 'object' && data !== null) {
    const obj = data as Record<string, unknown>
    if (obj.nodes && obj.edges) return 'graph3d'
    if (obj.events) return 'timeline'
    if (obj.lines) return 'stream-log'
    if (obj.sections || obj.content) return 'document'
    if (obj.items && obj.states) return 'status-matrix'
    
    const keys = Object.keys(obj)
    const values = Object.values(obj)
    if (keys.length <= 6 && values.every(v => typeof v === 'number' || typeof v === 'string')) {
      return 'metric'
    }
    return 'grid'
  }
  
  return 'list'
}

function transformToKanban(data: unknown): KanbanPayload {
  const arr = Array.isArray(data) ? data : [data]
  const lanes: KanbanPayload['lanes'] = []
  
  const statusGroups = new Map<string, KanbanPayload['lanes'][0]['items']>()
  
  for (const item of arr) {
    const obj = item as Record<string, unknown>
    const status = String(obj.status || obj.lane || 'todo')
    if (!statusGroups.has(status)) {
      statusGroups.set(status, [])
    }
    statusGroups.get(status)!.push({
      id: String(obj.id || Math.random().toString(36).slice(2)),
      title: String(obj.title || obj.name || obj.label || 'Untitled'),
      description: String(obj.description || obj.summary || ''),
      tags: Array.isArray(obj.tags) ? obj.tags.map(String) : [],
      priority: obj.priority as 'low' | 'medium' | 'high' | undefined,
      assignee: obj.assignee ? String(obj.assignee) : undefined,
    })
  }
  
  for (const [status, items] of statusGroups) {
    lanes.push({ id: status, title: status, items })
  }
  
  return { type: 'kanban', lanes }
}

function transformToGrid(data: unknown): GridPayload {
  const arr = Array.isArray(data) ? data : [data]
  if (arr.length === 0) return { type: 'grid', columns: [], rows: [] }
  
  const first = arr[0] as Record<string, unknown>
  const columns = Object.keys(first).map(key => ({
    key,
    label: key.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase()),
    sortable: true,
    filterable: true,
  }))
  
  return { type: 'grid', columns, rows: arr as Record<string, unknown>[] }
}

function transformToMetric(data: unknown): MetricPayload {
  const items: MetricPayload['items'] = []
  
  if (Array.isArray(data)) {
    for (const item of data) {
      const obj = item as Record<string, unknown>
      items.push({
        id: String(obj.id || Math.random().toString(36).slice(2)),
        label: String(obj.label || obj.name || 'Metric'),
        value: Number(obj.value ?? obj.count ?? obj.total ?? 0),
        trend: obj.trend as 'up' | 'down' | 'stable' | undefined,
        trendValue: obj.trendValue ? Number(obj.trendValue) : undefined,
        unit: obj.unit ? String(obj.unit) : undefined,
      })
    }
  } else if (typeof data === 'object' && data !== null) {
    const obj = data as Record<string, unknown>
    for (const [key, value] of Object.entries(obj)) {
      if (typeof value === 'number') {
        items.push({
          id: key,
          label: key.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase()),
          value,
        })
      }
    }
  }
  
  return { type: 'metric', items }
}

function transformToTimeline(data: unknown): TimelinePayload {
  const arr = Array.isArray(data) ? data : [data]
  const events = arr.map((item, index) => {
    const obj = item as Record<string, unknown>
    return {
      id: String(obj.id || index),
      timestamp: obj.timestamp ? Number(obj.timestamp) * 1000 : Date.now() - index * 3600000,
      title: String(obj.title || obj.name || 'Event'),
      description: String(obj.description || obj.summary || ''),
      category: obj.category ? String(obj.category) : undefined,
    }
  }).sort((a, b) => a.timestamp - b.timestamp)
  
  return { type: 'timeline', events }
}

function transformToGraph(data: unknown): GraphPayload {
  const obj = data as Record<string, unknown>
  const nodes = Array.isArray(obj.nodes) ? obj.nodes : []
  const edges = Array.isArray(obj.edges) ? obj.edges : []
  
  return {
    type: 'graph3d',
    nodes: nodes.map((n: unknown, i: number) => {
      const node = n as Record<string, unknown>
      return {
        id: String(node.id || i),
        label: String(node.label || node.name || node.id || i),
        group: node.group ? String(node.group) : undefined,
        value: node.value ? Number(node.value) : undefined,
      }
    }),
    edges: edges.map((e: unknown) => {
      const edge = e as Record<string, unknown>
      return {
        source: String(edge.source),
        target: String(edge.target),
        label: edge.label ? String(edge.label) : undefined,
        value: edge.value ? Number(edge.value) : undefined,
      }
    }),
  }
}

function transformToDocument(data: unknown): DocumentPayload {
  if (typeof data === 'string') {
    const md = parseMarkdown(data)
    return {
      type: 'document',
      title: md.title || 'Document',
      content: data,
      sections: md.sections.map((s, i) => ({
        id: String(i),
        title: s.title,
        level: s.level,
        content: s.content.trim(),
      })),
    }
  }
  
  const obj = data as Record<string, unknown>
  return {
    type: 'document',
    title: String(obj.title || obj.name || 'Document'),
    content: String(obj.content || obj.body || JSON.stringify(obj)),
  }
}

function transformToMatrix(data: unknown): MatrixPayload {
  const arr = Array.isArray(data) ? data : [data]
  const states = new Set<string>()
  const items: MatrixPayload['items'] = []
  
  for (const item of arr) {
    const obj = item as Record<string, unknown>
    const statesObj: Record<string, boolean> = {}
    
    for (const [key, value] of Object.entries(obj)) {
      if (key === 'id' || key === 'name' || key === 'label') continue
      if (typeof value === 'boolean') {
        states.add(key)
        statesObj[key] = value
      }
    }
    
    items.push({
      id: String(obj.id || Math.random().toString(36).slice(2)),
      label: String(obj.name || obj.label || 'Item'),
      states: statesObj,
    })
  }
  
  return { type: 'status-matrix', items, states: Array.from(states) }
}

function transformToStream(data: unknown): StreamPayload {
  const arr = Array.isArray(data) ? data : [data]
  const lines = arr.map((item, index) => {
    const obj = item as Record<string, unknown>
    return {
      id: String(obj.id || index),
      timestamp: obj.timestamp ? Number(obj.timestamp) * 1000 : Date.now() - index * 1000,
      level: (obj.level === 'error' || obj.level === 'warn' || obj.level === 'debug') 
        ? obj.level as 'info' | 'warn' | 'error' | 'debug'
        : 'info',
      message: String(obj.message || obj.text || obj.line || JSON.stringify(obj)),
    }
  })
  
  return { type: 'stream-log', lines }
}

function transformToList(data: unknown): ListPayload {
  const arr = Array.isArray(data) ? data : [data]
  const items = arr.map((item, index) => {
    const obj = item as Record<string, unknown>
    return {
      id: String(obj.id || index),
      label: String(obj.label || obj.name || obj.title || item),
      value: obj.value ? String(obj.value) : undefined,
      icon: obj.icon ? String(obj.icon) : undefined,
    }
  })
  
  return { type: 'list', items }
}

function transformData(data: unknown, componentType: ComponentType): ComponentPayload {
  switch (componentType) {
    case 'kanban':
      return transformToKanban(data)
    case 'grid':
      return transformToGrid(data)
    case 'metric':
      return transformToMetric(data)
    case 'timeline':
      return transformToTimeline(data)
    case 'graph3d':
      return transformToGraph(data)
    case 'document':
      return transformToDocument(data)
    case 'status-matrix':
      return transformToMatrix(data)
    case 'stream-log':
      return transformToStream(data)
    case 'list':
      return transformToList(data)
    default:
      return transformToList(data)
  }
}

function getItemCount(data: unknown): number {
  if (Array.isArray(data)) return data.length
  if (typeof data === 'object' && data !== null) {
    const obj = data as Record<string, unknown>
    if (obj.nodes && Array.isArray(obj.nodes)) return obj.nodes.length
    if (obj.items && Array.isArray(obj.items)) return obj.items.length
    return Object.keys(obj).length
  }
  return 1
}

export async function ingest(request: IngestRequest): Promise<ParsedData> {
  const { source, format: requestedFormat, componentType: preferredType, options } = request
  
  let content = source
  
  if (source.startsWith('http://') || source.startsWith('https://')) {
    try {
      const response = await fetch(source)
      content = await response.text()
    } catch {
      return {
        success: false,
        format: 'text',
        data: null,
        metadata: {
          source,
          parsedAt: Date.now(),
          itemCount: 0,
        },
        componentPayload: { type: 'list', items: [] },
      }
    }
  }
  
  const detectedFormat = requestedFormat === 'auto' ? detectFormat(content) : requestedFormat
  const parsed = detectFormat(content) === 'json' ? parseJSON(content) 
    : detectedFormat === 'yaml' ? parseYAML(content)
    : detectedFormat === 'csv' ? parseCSV(content)
    : detectedFormat === 'markdown' ? parseMarkdown(content)
    : content
  
  const componentType = preferredType && preferredType !== 'auto' ? preferredType : inferComponentType(parsed)
  const componentPayload = transformData(parsed, componentType)
  
  return {
    success: true,
    format: detectedFormat,
    data: parsed,
    metadata: {
      source,
      parsedAt: Date.now(),
      itemCount: getItemCount(parsed),
    },
    componentPayload,
  }
}

export function detectFormatFn(content: string, filename?: string): DataFormat {
  return detectFormat(content, filename)
}

export function inferComponent(data: unknown, preferred?: ComponentType): ComponentType {
  return inferComponentType(data, preferred)
}

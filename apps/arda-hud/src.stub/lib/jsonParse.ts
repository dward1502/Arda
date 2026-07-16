// sigil: REPAIR

export interface JsonParseOk<T = unknown> {
  ok: true
  value: T
}

export interface JsonParseErr {
  ok: false
  error: string
  preview: string
}

export type JsonParseResult<T = unknown> = JsonParseOk<T> | JsonParseErr

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function compactJsonPreview(content: string, max = 180): string {
  const compact = content.replace(/\s+/g, ' ').trim()
  if (!compact) return ''
  return compact.length > max ? `${compact.slice(0, max - 1)}…` : compact
}

export function parseJsonResult<T = unknown>(content: string): JsonParseResult<T> {
  try {
    return { ok: true, value: JSON.parse(content) as T }
  } catch (error) {
    return {
      ok: false,
      error: errorMessage(error),
      preview: compactJsonPreview(content),
    }
  }
}

export function parseJsonOrNull<T = unknown>(content: string | null | undefined): T | null {
  if (!content) return null
  const parsed = parseJsonResult<T>(content)
  return parsed.ok ? parsed.value : null
}

export function parseJsonOrDefault<T>(content: string | null | undefined, fallback: T): T {
  if (!content) return fallback
  const parsed = parseJsonResult<T>(content)
  return parsed.ok ? parsed.value : fallback
}

export function jsonParseFailureMessage(label: string, result: JsonParseErr): string {
  const preview = result.preview ? `; preview: ${result.preview}` : ''
  return `${label} JSON parse failed: ${result.error}${preview}`
}

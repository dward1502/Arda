const CAPTURE_WIDTH = 1280
const CAPTURE_HEIGHT = 720

export function normalizeBrowserAddress(value: string): string {
  const trimmed = value.trim()
  if (!trimmed) throw new Error('Browser address is required')
  const candidate = /^[a-z][a-z\d+.-]*:/i.test(trimmed) ? trimmed : `https://${trimmed}`
  let parsed: URL
  try {
    parsed = new URL(candidate)
  } catch {
    throw new Error('Browser address must be a valid HTTP(S) URL')
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    throw new Error('Browser address must use HTTP(S)')
  }
  return parsed.toString().replace(/\/$/, parsed.pathname === '/' && !parsed.search && !parsed.hash ? '' : '/')
}

interface PointerMappingRequest {
  clientX: number
  clientY: number
  bounds: { left: number; top: number; width: number; height: number }
}

export function mapWorkstationPointerToCapture(
  request: PointerMappingRequest,
): { x: number; y: number } | null {
  const { bounds } = request
  if (bounds.width <= 0 || bounds.height <= 0) return null
  const scale = Math.min(bounds.width / CAPTURE_WIDTH, bounds.height / CAPTURE_HEIGHT)
  const renderedWidth = CAPTURE_WIDTH * scale
  const renderedHeight = CAPTURE_HEIGHT * scale
  const left = bounds.left + (bounds.width - renderedWidth) / 2
  const top = bounds.top + (bounds.height - renderedHeight) / 2
  const localX = request.clientX - left
  const localY = request.clientY - top
  if (localX < 0 || localY < 0 || localX > renderedWidth || localY > renderedHeight) return null
  return {
    x: Math.min(CAPTURE_WIDTH, Math.max(0, localX / scale)),
    y: Math.min(CAPTURE_HEIGHT, Math.max(0, localY / scale)),
  }
}

export const SCHEMA_VERSION = 'arda.project-adapter.v1'
export const PROTOCOL_VERSION = '1'
export const MAX_LINE_BYTES = 64 * 1024

export class ProtocolError extends Error {
  constructor(code, message) {
    super(message)
    this.name = 'ProtocolError'
    this.code = code
  }
}

export function validateEnvelope(frame) {
  if (frame === null || typeof frame !== 'object' || Array.isArray(frame)) {
    throw new ProtocolError('invalid_message', 'frame must be a JSON object')
  }
  if (frame.schema_version !== SCHEMA_VERSION) {
    throw new ProtocolError('unsupported_schema', 'unsupported schema_version')
  }
  if (typeof frame.id !== 'string' || frame.id.length === 0) {
    throw new ProtocolError('invalid_id', 'frame id must be a non-empty string')
  }
  if (typeof frame.type !== 'string' || frame.type.length === 0) {
    throw new ProtocolError('invalid_type', 'frame type must be a non-empty string')
  }
  return frame
}

export function decodeFrame(line) {
  const bytes = Buffer.byteLength(line, 'utf8')
  if (bytes > MAX_LINE_BYTES) {
    throw new ProtocolError('line_too_large', `frame exceeds ${MAX_LINE_BYTES} bytes`)
  }
  if (!line.endsWith('\n')) {
    throw new ProtocolError('partial_frame', 'stream ended with a partial frame')
  }
  let frame
  try {
    frame = JSON.parse(line)
  } catch {
    throw new ProtocolError('invalid_json', 'frame is not valid JSON')
  }
  return validateEnvelope(frame)
}

export function encodeFrame(frame) {
  validateEnvelope(frame)
  const line = `${JSON.stringify(frame)}\n`
  if (Buffer.byteLength(line, 'utf8') > MAX_LINE_BYTES) {
    throw new ProtocolError('line_too_large', `frame exceeds ${MAX_LINE_BYTES} bytes`)
  }
  return line
}

export function negotiateCapabilities(requested, available) {
  const allow = new Set(available)
  return requested.filter((capability) => allow.has(capability))
}

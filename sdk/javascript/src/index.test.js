import assert from 'node:assert/strict'
import test from 'node:test'

import {
  MAX_LINE_BYTES,
  ProtocolError,
  SCHEMA_VERSION,
  decodeFrame,
  encodeFrame,
  negotiateCapabilities,
} from './index.js'

const frame = () => ({
  schema_version: SCHEMA_VERSION,
  id: 'request-1',
  type: 'health',
})

test('valid frame round trips', () => {
  assert.deepEqual(decodeFrame(encodeFrame(frame())), frame())
})

test('oversized input is rejected before parsing', () => {
  assert.throws(
    () => decodeFrame(`${'x'.repeat(MAX_LINE_BYTES)}\n`),
    (error) => error instanceof ProtocolError && error.code === 'line_too_large',
  )
})

test('partial frame fails closed', () => {
  assert.throws(
    () => decodeFrame(JSON.stringify(frame())),
    (error) => error instanceof ProtocolError && error.code === 'partial_frame',
  )
})

test('capability negotiation preserves request order and denies unknown capabilities', () => {
  assert.deepEqual(
    negotiateCapabilities(['read', 'network', 'write'], ['read', 'write']),
    ['read', 'write'],
  )
})

#!/usr/bin/env node
import { createHash } from 'node:crypto'
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { createInterface } from 'node:readline'
import { pathToFileURL } from 'node:url'

const sdk = await import(pathToFileURL(process.env.ARDA_JAVASCRIPT_SDK).href)
const adapter = 'arda-javascript-golden'
const adapterVersion = '1.0.0'
const digest = (value) => `sha256:${createHash('sha256').update(value).digest('hex')}`
const send = (frame) => process.stdout.write(sdk.encodeFrame(frame))
const response = (request, type, fields = {}) => ({
  schema_version: sdk.SCHEMA_VERSION,
  id: `${request.id}-${type}`,
  type,
  request_id: request.id,
  ...fields,
})

async function handle(request) {
  if (request.type === 'initialize') {
    send(response(request, 'initialized', {
      adapter,
      adapter_version: adapterVersion,
      capabilities: sdk.negotiateCapabilities(request.allowed_capabilities, ['mutate_and_test']),
      recovery_supported: true,
    }))
    return
  }
  if (request.type === 'health') {
    send(response(request, 'health_status', { status: 'ready', detail: 'javascript reference adapter ready' }))
    return
  }
  if (request.type !== 'request') throw new Error(`unsupported message type: ${request.type}`)

  const startedAt = new Date().toISOString()
  let status = 'succeeded'
  let output
  try {
    if (request.operation !== 'mutate_and_test') throw new Error(`unsupported operation: ${request.operation}`)
    if (request.arguments?.before !== 'hello' || request.arguments?.after !== 'hello, Arda'
      || Object.keys(request.arguments).length !== 2) {
      throw new Error('golden mutation arguments did not match the approved plan')
    }
    const paths = ['src/greeting.js', 'test/greeting.test.js']
    let changed = false
    for (const path of paths) {
      const before = readFileSync(path, 'utf8')
      const after = before.replaceAll("'hello'", "'hello, Arda'")
      if (after !== before) { writeFileSync(path, after); changed = true }
    }
    if (changed) {
      const countPath = process.env.ARDA_GOLDEN_MUTATION_COUNT
      const count = existsSync(countPath) ? Number(readFileSync(countPath, 'utf8')) : 0
      writeFileSync(countPath, String(count + 1))
    }
    const check = spawnSync(process.execPath, ['--test'], { encoding: 'utf8' })
    if (check.status !== 0) throw new Error(`${check.stdout}${check.stderr}`)
    output = {
      mutation: { files: paths, observable_count: 1, source_digest: digest(readFileSync(paths[0])) },
      test: { command: 'node --test', exit_code: check.status, output_digest: digest(`${check.stdout}${check.stderr}`) },
      route: { adapter: 'javascript-reference', provider: null, model: null },
      cost_usd: 0,
    }
  } catch (error) {
    status = 'failed'
    output = { error: error.message }
  }
  send(response(request, 'result', {
    status,
    output,
    provenance: {
      adapter,
      adapter_version: adapterVersion,
      cwd: process.cwd(),
      started_at: startedAt,
      finished_at: new Date().toISOString(),
      request_digest: digest(JSON.stringify(request)),
    },
    recovery_token: request.recovery_token ?? null,
  }))
}

const lines = createInterface({ input: process.stdin, crlfDelay: Infinity })
for await (const line of lines) {
  if (!line) continue
  try { await handle(sdk.decodeFrame(`${line}\n`)) }
  catch (error) {
    const request = JSON.parse(line)
    send(response(request, 'error', { code: 'adapter_error', message: error.message, retryable: false }))
  }
}

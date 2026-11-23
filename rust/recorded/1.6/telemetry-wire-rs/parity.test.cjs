const assert = require('node:assert/strict')
const { test } = require('node:test')
const original = require('../../.review-ardia/packages/core/src/telemetryWire.ts')
const port = require('./index.cjs')

test('logRecord uses the original timestamp coercion on both transports', t => {
  t.mock.method(Date, 'now', () => 1000)
  process.env.RIA_NOW_MS = '1000'
  for (const timestamp of [undefined, null, '\u0085', '\uFEFF', new Date(1000), [['0b11']],
    '0x100000000000008000000000000000001', NaN, Infinity, -0]) {
    const args = ['warn', 'test', {}, timestamp]
    const expected = original.logRecord(...args)
    assert.deepEqual(port.logRecord(...args), expected)
    assert.deepEqual(port.logRecord(...args, null), expected)
  }
  assert.throws(() => port.logRecord('warn', 'test', {}, 1n), TypeError)
})

test('telemetry preserves ordinary keys that match transport tags', () => {
  for (const key of ['$num', '$undefined', '$utf16', '$date', '$$num']) {
    const value = { [key]: { $num: 'NaN' } }
    const attributes = { ...value, requestId: 'r1', skipped: undefined, skippedNumber: NaN }
    const e = { uuid: 'u', event: 'test', distinct_id: 'd', timestamp: '2025-01-01', properties: attributes }
    assert.deepEqual(port.eventRecord(e), original.eventRecord(e))
    assert.deepEqual(port.eventRecord(e, null), original.eventRecord(e))
    const args = ['warn', 'test', attributes, 42]
    assert.deepEqual(port.logRecord(...args), original.logRecord(...args))
    assert.deepEqual(port.logRecord(...args, null), original.logRecord(...args))
    const record = original.logRecord(...args)
    record.attributes[0].value = value
    const request = [value, 'test', [record]]
    assert.equal(port.logsRequest(...request), original.logsRequest(...request))
    assert.equal(port.logsRequest(...request, null), original.logsRequest(...request))
  }
  const attributes = { $num: 'NaN', requestId: 'r1' }
  const args = ['warn', 'test', attributes, 42]
  assert.deepEqual(port.logRecord(...args), original.logRecord(...args))
  assert.deepEqual(port.logRecord(...args, null), original.logRecord(...args))
  for (const records of [undefined, null, 42, attributes]) {
    assert.equal(port.logsRequest(attributes, 'test', records), original.logsRequest(attributes, 'test', records))
  }
})

test('telemetry indexes string attributes by UTF-16 code units', () => {
  for (const text of ['', 'é', 'a😀z', '\uD800', '\uDC00']) {
    const event = { uuid: 'u', event: 'test', distinct_id: 'd', timestamp: '2025-01-01', properties: text }
    const log = ['warn', 'test', text, 42]
    const request = [text, 'test', [original.logRecord('warn', 'test', {}, 42)]]
    assert.deepEqual(port.eventRecord(event), original.eventRecord(event))
    assert.deepEqual(port.eventRecord(event, null), original.eventRecord(event))
    assert.deepEqual(port.logRecord(...log), original.logRecord(...log))
    assert.deepEqual(port.logRecord(...log, null), original.logRecord(...log))
    assert.equal(port.logsRequest(...request), original.logsRequest(...request))
    assert.equal(port.logsRequest(...request, null), original.logsRequest(...request))
    assert.equal(port.logsRequest(text, 'test'), original.logsRequest(text, 'test'))
  }
})

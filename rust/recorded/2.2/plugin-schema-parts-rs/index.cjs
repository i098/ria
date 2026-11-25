// Loads the native module built by `rust-anything build`.
const native = require('./pluginschemaparts_rs.node')

// Structured values cross as JSON text. JSON has no `undefined`, NaN, -0, bigint, Date, Map, Set
// or bytes, and Rust strings no lone surrogates: those travel as one-key marker objects
// (`{"$undefined": true}`, `{"$num": "NaN"}`, `{"$map": [[k, v]]}`, `{"$bytes": [...]}`, ...),
// the same markers recordings use.
const replacer = function (k, v) {
  const raw = this[k]
  if (raw instanceof Date) return { $date: Number.isNaN(raw.getTime()) ? 'Invalid' : raw.toISOString() }
  if (typeof Buffer !== 'undefined' && Buffer.isBuffer(raw)) return { $buffer: Array.from(raw) }
  if (raw instanceof Uint8Array) return { $bytes: Array.from(raw) }
  if (raw instanceof Map) return { $map: [...raw] }
  if (raw instanceof Set) return { $set: [...raw] }
  return v === undefined ? { $undefined: true }
    : typeof v === 'bigint' ? { $bigint: String(v) }
    : typeof v === 'number' ? (Number.isFinite(v) && !Object.is(v, -0) ? v : { $num: Object.is(v, -0) ? '-0' : String(v) })
    : typeof v === 'string' && !v.isWellFormed() ? { $utf16: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) }
    : v
}
const encode = v => JSON.stringify(v, replacer)
const single = (v, key) => Object.keys(v).length === 1 && key in v
const revive = v => Array.isArray(v) ? v.map(revive)
  : v !== null && typeof v === 'object'
    ? (single(v, '$undefined') ? undefined
      : single(v, '$utf16') ? fromUnits(v.$utf16)
      : single(v, '$num') ? Number(v.$num)
      : single(v, '$bigint') ? BigInt(v.$bigint)
      : single(v, '$date') ? new Date(v.$date)
      : single(v, '$bytes') ? Uint8Array.from(v.$bytes)
      : single(v, '$buffer') ? Buffer.from(v.$buffer)
      : single(v, '$map') ? new Map(v.$map.map(([k, x]) => [revive(k), revive(x)]))
      : single(v, '$set') ? new Set(v.$set.map(revive))
      : Object.fromEntries(Object.entries(v).map(([k, x]) => [k, revive(x)])))
    : v
// Most results carry no marker: skip the walk unless the text has one.
const decode = text => {
  const value = JSON.parse(text)
  return text.includes('"$') ? revive(value) : value
}
const fromUnits = (units) => {
  let out = ''
  for (let i = 0; i < units.length; i += 8192)
    out += String.fromCharCode.apply(null, Array.prototype.slice.call(units, i, i + 8192))
  return out
}

module.exports = { ...native }
const specs = {
  textOf: [[0, 0], 'json', false],
  closed: [[1, 1], 'json', false],
  pluginPath: [[0], 'json', false],
  rowSchema: [[0], 'json', false],
}
const classes = {

}
const convert = result => result === 'json' ? decode : result === 'units' ? fromUnits : x => x
// An omitted structured argument is `undefined`, not a missing value.
const prepare = (a, structured) => Array.from({ length: Math.max(a.length, structured.length) }, (_, i) => structured[i] ? encode(a[i]) : a[i])
// Functions: [structured arguments, result kind, async].
for (const [name, [structured, result, isAsync]] of Object.entries(specs)) {
  const call = native[name]
  const out = convert(result)
  module.exports[name] = isAsync
    ? async (...a) => out(await call(...prepare(a, structured)))
    : (...a) => out(call(...prepare(a, structured)))
}
// Classes: a JS class around the native one, converting arguments and results.
const INNER = Symbol('native')
for (const [name, [ctor, methods]] of Object.entries(classes)) {
  const Native = native[name]
  const Wrapped = { [name]: class { constructor(...a) { this[INNER] = new Native(...prepare(a, ctor)) } } }[name]
  for (const [method, [structured, result]] of Object.entries(methods)) {
    const out = convert(result)
    Wrapped.prototype[method] = function (...a) { return out(this[INNER][method](...prepare(a, structured))) }
  }
  module.exports[name] = Wrapped
}

// The binary wire (see rust-anything's `wire.rs`): arguments are written into `wIn`, results read
// from `wOut`; both are registered with the native module once.
let wIn = new Uint8Array(1 << 16), wOut = new Uint8Array(1 << 16)
// Its own ArrayBuffer: Bun (JavaScriptCore) may relocate a small typed array whose `.buffer`
// nobody read, and Rust then reads a length of 0 and reports `wire data ended early`.
const wLen = new Uint32Array(new ArrayBuffer(4))
let iv = new DataView(wIn.buffer), ov = new DataView(wOut.buffer), at = 0, p = 0
native.rustAnythingWireInit(wIn, wOut, wLen)
const regrow = () => native.rustAnythingWireInit(wIn, wOut, wLen)
// Runs a wire function; a result larger than `wOut` is fetched once `wOut` has grown.
const wireCall = fn => {
  wLen[0] = at
  const n = fn()
  if (Number.isNaN(n)) return false
  if (n === Infinity) {
    p = 0
    throw new Error(rStr())
  }
  if (n < 0) {
    wOut = new Uint8Array(Math.max(wOut.length * 2, -n))
    ov = new DataView(wOut.buffer)
    native.rustAnythingWireInit(wIn, wOut, wLen)
    native.rustAnythingWireFlush()
  }
  p = 0
  return true
}

const need = n => {
  if (at + n <= wIn.length) return
  const grown = new Uint8Array(Math.max(wIn.length * 2, at + n))
  grown.set(wIn)
  wIn = grown
  iv = new DataView(wIn.buffer)
  regrow()
}
const wU8 = v => { need(1); iv.setUint8(at++, v) }
const wU32 = v => { need(4); iv.setUint32(at, v, true); at += 4 }
const wF64 = v => { need(8); iv.setFloat64(at, v, true); at += 8 }
const wUnits = s => {
  need(4 + 2 * s.length)
  iv.setUint32(at, s.length, true)
  at += 4
  for (let i = 0; i < s.length; i++) { iv.setUint16(at, s.charCodeAt(i), true); at += 2 }
}
const wStr = s => { wU8(4); wUnits(s) }
const wDyn = v => {
  switch (typeof v) {
    case 'undefined': wU8(0); return true
    case 'boolean': wU8(v ? 3 : 2); return true
    case 'number': wU8(4); wF64(v); return true
    case 'string': wU8(5); wUnits(v); return true
    case 'object':
      if (v === null) { wU8(1); return true }
      if (Array.isArray(v)) {
        wU8(6); wU32(v.length)
        for (let i = 0; i < v.length; i++) if (!wDyn(v[i])) return false
        return true
      }
      if (Object.getPrototypeOf(v) !== Object.prototype && Object.getPrototypeOf(v) !== null) return false
      {
        const keys = Object.keys(v)
        wU8(7); wU32(keys.length)
        for (const k of keys) { wUnits(k); if (!wDyn(v[k])) return false }
        return true
      }
  }
  return false
}
const rU8 = () => ov.getUint8(p++)
const rU32 = () => { const v = ov.getUint32(p, true); p += 4; return v }
const rF64 = () => { const v = ov.getFloat64(p, true); p += 8; return v }
// ignoreBOM: a leading U+FEFF is part of the string, not a byte-order mark.
const utf16 = new TextDecoder('utf-16le', { ignoreBOM: true })
const rUnits = (wellFormed) => {
  const n = rU32()
  let s
  if (n > 24 && wellFormed) s = utf16.decode(wOut.subarray(p, p + 2 * n))
  else {
    s = ''
    for (let i = 0; i < n; i++) s += String.fromCharCode(ov.getUint16(p + 2 * i, true))
  }
  p += 2 * n
  return s
}
// Tag 1: a string from Rust (well-formed); 4: raw code units, lone surrogates included.
const rStr = () => rUnits(rU8() === 1)
const rDyn = () => {
  switch (rU8()) {
    case 0: return undefined
    case 1: return null
    case 2: return false
    case 3: return true
    case 4: return rF64()
    case 5: return rUnits(false)
    case 6: { const n = rU32(); const a = new Array(n); for (let i = 0; i < n; i++) a[i] = rDyn(); return a }
    case 7: { const n = rU32(); const o = {}; for (let i = 0; i < n; i++) { const k = rUnits(false); o[k] = rDyn() } return o }
  }
}

// `textOf`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.textOf
const call = native.textOfWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'number') return false
wF64(a0)
if (a1 === undefined) {
wU8(0)
} else if (a1 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a1 !== 'string') return false
wStr(a1)
}
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rDyn()
o0["type"] = v0
break
}
case 1: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["enum"] = v0
break
}
case 2: {
let v0
v0 = rDyn()
o0["const"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["default"] = v0
break
}
case 4: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["description"] = v0
break
}
case 5: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["minimum"] = v0
break
}
case 6: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["maximum"] = v0
break
}
case 7: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["exclusiveMinimum"] = v0
break
}
case 8: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["pattern"] = v0
break
}
case 9: {
let v0
v0 = rDyn()
o0["items"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v0 = a2
}
}
}
o0["prefixItems"] = v0
break
}
case 11: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["minItems"] = v0
break
}
case 12: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["maxItems"] = v0
break
}
case 13: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["uniqueItems"] = v0
break
}
case 14: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
v6 = rDyn()
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
v6 = rDyn()
o6["description"] = v6
break
}
case 5: {
let v6
v6 = rDyn()
o6["minimum"] = v6
break
}
case 6: {
let v6
v6 = rDyn()
o6["maximum"] = v6
break
}
case 7: {
let v6
v6 = rDyn()
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
v6 = rDyn()
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
v6 = rDyn()
o6["prefixItems"] = v6
break
}
case 11: {
let v6
v6 = rDyn()
o6["minItems"] = v6
break
}
case 12: {
let v6
v6 = rDyn()
o6["maxItems"] = v6
break
}
case 13: {
let v6
v6 = rDyn()
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
v6 = rDyn()
o6["propertyNames"] = v6
break
}
case 17: {
let v6
v6 = rDyn()
o6["required"] = v6
break
}
case 18: {
let v6
v6 = rDyn()
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
v7 = rDyn()
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["properties"] = v0
break
}
case 15: {
let v0
v0 = rDyn()
o0["additionalProperties"] = v0
break
}
case 16: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rDyn()
o2["type"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rDyn()
a4[i4] = v4
}
v2 = a4
}
}
}
o2["enum"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["const"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["default"] = v2
break
}
case 4: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["description"] = v2
break
}
case 5: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["minimum"] = v2
break
}
case 6: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["maximum"] = v2
break
}
case 7: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["exclusiveMinimum"] = v2
break
}
case 8: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["pattern"] = v2
break
}
case 9: {
let v2
v2 = rDyn()
o2["items"] = v2
break
}
case 10: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
a4[i4] = v4
}
v2 = a4
}
}
}
o2["prefixItems"] = v2
break
}
case 11: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["minItems"] = v2
break
}
case 12: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["maxItems"] = v2
break
}
case 13: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rU8() === 1
}
}
o2["uniqueItems"] = v2
break
}
case 14: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const o4 = {}
for (let i4 = 0; i4 < n4; i4++) {
const k4 = rStr()
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
v7 = rDyn()
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
o4[k4] = v4
}
v2 = o4
}
}
}
o2["properties"] = v2
break
}
case 15: {
let v2
v2 = rDyn()
o2["additionalProperties"] = v2
break
}
case 16: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
v4 = rDyn()
o4["type"] = v4
break
}
case 1: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
v6 = rDyn()
a6[i6] = v6
}
v4 = a6
}
}
}
o4["enum"] = v4
break
}
case 2: {
let v4
v4 = rDyn()
o4["const"] = v4
break
}
case 3: {
let v4
v4 = rDyn()
o4["default"] = v4
break
}
case 4: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rStr()
}
}
o4["description"] = v4
break
}
case 5: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["minimum"] = v4
break
}
case 6: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["maximum"] = v4
break
}
case 7: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["exclusiveMinimum"] = v4
break
}
case 8: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rStr()
}
}
o4["pattern"] = v4
break
}
case 9: {
let v4
v4 = rDyn()
o4["items"] = v4
break
}
case 10: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
a6[i6] = v6
}
v4 = a6
}
}
}
o4["prefixItems"] = v4
break
}
case 11: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["minItems"] = v4
break
}
case 12: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["maxItems"] = v4
break
}
case 13: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rU8() === 1
}
}
o4["uniqueItems"] = v4
break
}
case 14: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const o6 = {}
for (let i6 = 0; i6 < n6; i6++) {
const k6 = rStr()
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
o6[k6] = v6
}
v4 = o6
}
}
}
o4["properties"] = v4
break
}
case 15: {
let v4
v4 = rDyn()
o4["additionalProperties"] = v4
break
}
case 16: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v4 = o6
}
}
}
o4["propertyNames"] = v4
break
}
case 17: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
v6 = rStr()
a6[i6] = v6
}
v4 = a6
}
}
}
o4["required"] = v4
break
}
case 18: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
a6[i6] = v6
}
v4 = a6
}
}
}
o4["anyOf"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v2 = o4
}
}
}
o2["propertyNames"] = v2
break
}
case 17: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rStr()
a4[i4] = v4
}
v2 = a4
}
}
}
o2["required"] = v2
break
}
case 18: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
a4[i4] = v4
}
v2 = a4
}
}
}
o2["anyOf"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v0 = o2
}
}
}
o0["propertyNames"] = v0
break
}
case 17: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rStr()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["required"] = v0
break
}
case 18: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v0 = a2
}
}
}
o0["anyOf"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.textOf = function textOf (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of textOf has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `closed`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.closed
const call = native.closedWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rDyn()
o0["type"] = v0
break
}
case 1: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["enum"] = v0
break
}
case 2: {
let v0
v0 = rDyn()
o0["const"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["default"] = v0
break
}
case 4: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["description"] = v0
break
}
case 5: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["minimum"] = v0
break
}
case 6: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["maximum"] = v0
break
}
case 7: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["exclusiveMinimum"] = v0
break
}
case 8: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["pattern"] = v0
break
}
case 9: {
let v0
v0 = rDyn()
o0["items"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v0 = a2
}
}
}
o0["prefixItems"] = v0
break
}
case 11: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["minItems"] = v0
break
}
case 12: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["maxItems"] = v0
break
}
case 13: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["uniqueItems"] = v0
break
}
case 14: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
v6 = rDyn()
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
v6 = rDyn()
o6["description"] = v6
break
}
case 5: {
let v6
v6 = rDyn()
o6["minimum"] = v6
break
}
case 6: {
let v6
v6 = rDyn()
o6["maximum"] = v6
break
}
case 7: {
let v6
v6 = rDyn()
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
v6 = rDyn()
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
v6 = rDyn()
o6["prefixItems"] = v6
break
}
case 11: {
let v6
v6 = rDyn()
o6["minItems"] = v6
break
}
case 12: {
let v6
v6 = rDyn()
o6["maxItems"] = v6
break
}
case 13: {
let v6
v6 = rDyn()
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
v6 = rDyn()
o6["propertyNames"] = v6
break
}
case 17: {
let v6
v6 = rDyn()
o6["required"] = v6
break
}
case 18: {
let v6
v6 = rDyn()
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
v7 = rDyn()
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["properties"] = v0
break
}
case 15: {
let v0
v0 = rDyn()
o0["additionalProperties"] = v0
break
}
case 16: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rDyn()
o2["type"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rDyn()
a4[i4] = v4
}
v2 = a4
}
}
}
o2["enum"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["const"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["default"] = v2
break
}
case 4: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["description"] = v2
break
}
case 5: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["minimum"] = v2
break
}
case 6: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["maximum"] = v2
break
}
case 7: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["exclusiveMinimum"] = v2
break
}
case 8: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["pattern"] = v2
break
}
case 9: {
let v2
v2 = rDyn()
o2["items"] = v2
break
}
case 10: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
a4[i4] = v4
}
v2 = a4
}
}
}
o2["prefixItems"] = v2
break
}
case 11: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["minItems"] = v2
break
}
case 12: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["maxItems"] = v2
break
}
case 13: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rU8() === 1
}
}
o2["uniqueItems"] = v2
break
}
case 14: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const o4 = {}
for (let i4 = 0; i4 < n4; i4++) {
const k4 = rStr()
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
v7 = rDyn()
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
o4[k4] = v4
}
v2 = o4
}
}
}
o2["properties"] = v2
break
}
case 15: {
let v2
v2 = rDyn()
o2["additionalProperties"] = v2
break
}
case 16: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
v4 = rDyn()
o4["type"] = v4
break
}
case 1: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
v6 = rDyn()
a6[i6] = v6
}
v4 = a6
}
}
}
o4["enum"] = v4
break
}
case 2: {
let v4
v4 = rDyn()
o4["const"] = v4
break
}
case 3: {
let v4
v4 = rDyn()
o4["default"] = v4
break
}
case 4: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rStr()
}
}
o4["description"] = v4
break
}
case 5: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["minimum"] = v4
break
}
case 6: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["maximum"] = v4
break
}
case 7: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["exclusiveMinimum"] = v4
break
}
case 8: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rStr()
}
}
o4["pattern"] = v4
break
}
case 9: {
let v4
v4 = rDyn()
o4["items"] = v4
break
}
case 10: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
a6[i6] = v6
}
v4 = a6
}
}
}
o4["prefixItems"] = v4
break
}
case 11: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["minItems"] = v4
break
}
case 12: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["maxItems"] = v4
break
}
case 13: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rU8() === 1
}
}
o4["uniqueItems"] = v4
break
}
case 14: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const o6 = {}
for (let i6 = 0; i6 < n6; i6++) {
const k6 = rStr()
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
o6[k6] = v6
}
v4 = o6
}
}
}
o4["properties"] = v4
break
}
case 15: {
let v4
v4 = rDyn()
o4["additionalProperties"] = v4
break
}
case 16: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v4 = o6
}
}
}
o4["propertyNames"] = v4
break
}
case 17: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
v6 = rStr()
a6[i6] = v6
}
v4 = a6
}
}
}
o4["required"] = v4
break
}
case 18: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
a6[i6] = v6
}
v4 = a6
}
}
}
o4["anyOf"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v2 = o4
}
}
}
o2["propertyNames"] = v2
break
}
case 17: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rStr()
a4[i4] = v4
}
v2 = a4
}
}
}
o2["required"] = v2
break
}
case 18: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
a4[i4] = v4
}
v2 = a4
}
}
}
o2["anyOf"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v0 = o2
}
}
}
o0["propertyNames"] = v0
break
}
case 17: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rStr()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["required"] = v0
break
}
case 18: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v0 = a2
}
}
}
o0["anyOf"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.closed = function closed (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `pluginPath`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.pluginPath
const call = native.pluginPathWire
const encode = (a0) => {
at = 0
if (a0 === undefined) {
wU8(0)
} else if (a0 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a0 !== 'string') return false
wStr(a0)
}
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rDyn()
o0["type"] = v0
break
}
case 1: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["enum"] = v0
break
}
case 2: {
let v0
v0 = rDyn()
o0["const"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["default"] = v0
break
}
case 4: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["description"] = v0
break
}
case 5: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["minimum"] = v0
break
}
case 6: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["maximum"] = v0
break
}
case 7: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["exclusiveMinimum"] = v0
break
}
case 8: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["pattern"] = v0
break
}
case 9: {
let v0
v0 = rDyn()
o0["items"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v0 = a2
}
}
}
o0["prefixItems"] = v0
break
}
case 11: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["minItems"] = v0
break
}
case 12: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["maxItems"] = v0
break
}
case 13: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["uniqueItems"] = v0
break
}
case 14: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
v6 = rDyn()
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
v6 = rDyn()
o6["description"] = v6
break
}
case 5: {
let v6
v6 = rDyn()
o6["minimum"] = v6
break
}
case 6: {
let v6
v6 = rDyn()
o6["maximum"] = v6
break
}
case 7: {
let v6
v6 = rDyn()
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
v6 = rDyn()
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
v6 = rDyn()
o6["prefixItems"] = v6
break
}
case 11: {
let v6
v6 = rDyn()
o6["minItems"] = v6
break
}
case 12: {
let v6
v6 = rDyn()
o6["maxItems"] = v6
break
}
case 13: {
let v6
v6 = rDyn()
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
v6 = rDyn()
o6["propertyNames"] = v6
break
}
case 17: {
let v6
v6 = rDyn()
o6["required"] = v6
break
}
case 18: {
let v6
v6 = rDyn()
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
v7 = rDyn()
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["properties"] = v0
break
}
case 15: {
let v0
v0 = rDyn()
o0["additionalProperties"] = v0
break
}
case 16: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rDyn()
o2["type"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rDyn()
a4[i4] = v4
}
v2 = a4
}
}
}
o2["enum"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["const"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["default"] = v2
break
}
case 4: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["description"] = v2
break
}
case 5: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["minimum"] = v2
break
}
case 6: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["maximum"] = v2
break
}
case 7: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["exclusiveMinimum"] = v2
break
}
case 8: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["pattern"] = v2
break
}
case 9: {
let v2
v2 = rDyn()
o2["items"] = v2
break
}
case 10: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
a4[i4] = v4
}
v2 = a4
}
}
}
o2["prefixItems"] = v2
break
}
case 11: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["minItems"] = v2
break
}
case 12: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["maxItems"] = v2
break
}
case 13: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rU8() === 1
}
}
o2["uniqueItems"] = v2
break
}
case 14: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const o4 = {}
for (let i4 = 0; i4 < n4; i4++) {
const k4 = rStr()
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
v7 = rDyn()
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
o4[k4] = v4
}
v2 = o4
}
}
}
o2["properties"] = v2
break
}
case 15: {
let v2
v2 = rDyn()
o2["additionalProperties"] = v2
break
}
case 16: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
v4 = rDyn()
o4["type"] = v4
break
}
case 1: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
v6 = rDyn()
a6[i6] = v6
}
v4 = a6
}
}
}
o4["enum"] = v4
break
}
case 2: {
let v4
v4 = rDyn()
o4["const"] = v4
break
}
case 3: {
let v4
v4 = rDyn()
o4["default"] = v4
break
}
case 4: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rStr()
}
}
o4["description"] = v4
break
}
case 5: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["minimum"] = v4
break
}
case 6: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["maximum"] = v4
break
}
case 7: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["exclusiveMinimum"] = v4
break
}
case 8: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rStr()
}
}
o4["pattern"] = v4
break
}
case 9: {
let v4
v4 = rDyn()
o4["items"] = v4
break
}
case 10: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
a6[i6] = v6
}
v4 = a6
}
}
}
o4["prefixItems"] = v4
break
}
case 11: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["minItems"] = v4
break
}
case 12: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["maxItems"] = v4
break
}
case 13: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rU8() === 1
}
}
o4["uniqueItems"] = v4
break
}
case 14: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const o6 = {}
for (let i6 = 0; i6 < n6; i6++) {
const k6 = rStr()
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
o6[k6] = v6
}
v4 = o6
}
}
}
o4["properties"] = v4
break
}
case 15: {
let v4
v4 = rDyn()
o4["additionalProperties"] = v4
break
}
case 16: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v4 = o6
}
}
}
o4["propertyNames"] = v4
break
}
case 17: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
v6 = rStr()
a6[i6] = v6
}
v4 = a6
}
}
}
o4["required"] = v4
break
}
case 18: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
a6[i6] = v6
}
v4 = a6
}
}
}
o4["anyOf"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v2 = o4
}
}
}
o2["propertyNames"] = v2
break
}
case 17: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rStr()
a4[i4] = v4
}
v2 = a4
}
}
}
o2["required"] = v2
break
}
case 18: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
a4[i4] = v4
}
v2 = a4
}
}
}
o2["anyOf"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v0 = o2
}
}
}
o0["propertyNames"] = v0
break
}
case 17: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rStr()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["required"] = v0
break
}
case 18: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v0 = a2
}
}
}
o0["anyOf"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.pluginPath = function pluginPath (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of pluginPath has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `rowSchema`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.rowSchema
const call = native.rowSchemaWire
const encode = (a0) => {
at = 0
if (!Array.isArray(a0)) return false
wU8(1)
wU32(a0.length)
for (let i0 = 0; i0 < a0.length; i0++) {
const v0 = a0[i0]
if (typeof v0 !== 'string') return false
wStr(v0)
}
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rDyn()
o0["type"] = v0
break
}
case 1: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["enum"] = v0
break
}
case 2: {
let v0
v0 = rDyn()
o0["const"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["default"] = v0
break
}
case 4: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["description"] = v0
break
}
case 5: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["minimum"] = v0
break
}
case 6: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["maximum"] = v0
break
}
case 7: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["exclusiveMinimum"] = v0
break
}
case 8: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["pattern"] = v0
break
}
case 9: {
let v0
v0 = rDyn()
o0["items"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v0 = a2
}
}
}
o0["prefixItems"] = v0
break
}
case 11: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["minItems"] = v0
break
}
case 12: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["maxItems"] = v0
break
}
case 13: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["uniqueItems"] = v0
break
}
case 14: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
v6 = rDyn()
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
v6 = rDyn()
o6["description"] = v6
break
}
case 5: {
let v6
v6 = rDyn()
o6["minimum"] = v6
break
}
case 6: {
let v6
v6 = rDyn()
o6["maximum"] = v6
break
}
case 7: {
let v6
v6 = rDyn()
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
v6 = rDyn()
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
v6 = rDyn()
o6["prefixItems"] = v6
break
}
case 11: {
let v6
v6 = rDyn()
o6["minItems"] = v6
break
}
case 12: {
let v6
v6 = rDyn()
o6["maxItems"] = v6
break
}
case 13: {
let v6
v6 = rDyn()
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
v6 = rDyn()
o6["propertyNames"] = v6
break
}
case 17: {
let v6
v6 = rDyn()
o6["required"] = v6
break
}
case 18: {
let v6
v6 = rDyn()
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
v7 = rDyn()
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["properties"] = v0
break
}
case 15: {
let v0
v0 = rDyn()
o0["additionalProperties"] = v0
break
}
case 16: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rDyn()
o2["type"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rDyn()
a4[i4] = v4
}
v2 = a4
}
}
}
o2["enum"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["const"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["default"] = v2
break
}
case 4: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["description"] = v2
break
}
case 5: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["minimum"] = v2
break
}
case 6: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["maximum"] = v2
break
}
case 7: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["exclusiveMinimum"] = v2
break
}
case 8: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["pattern"] = v2
break
}
case 9: {
let v2
v2 = rDyn()
o2["items"] = v2
break
}
case 10: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
a4[i4] = v4
}
v2 = a4
}
}
}
o2["prefixItems"] = v2
break
}
case 11: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["minItems"] = v2
break
}
case 12: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["maxItems"] = v2
break
}
case 13: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rU8() === 1
}
}
o2["uniqueItems"] = v2
break
}
case 14: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const o4 = {}
for (let i4 = 0; i4 < n4; i4++) {
const k4 = rStr()
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
v7 = rDyn()
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
o4[k4] = v4
}
v2 = o4
}
}
}
o2["properties"] = v2
break
}
case 15: {
let v2
v2 = rDyn()
o2["additionalProperties"] = v2
break
}
case 16: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
v4 = rDyn()
o4["type"] = v4
break
}
case 1: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
v6 = rDyn()
a6[i6] = v6
}
v4 = a6
}
}
}
o4["enum"] = v4
break
}
case 2: {
let v4
v4 = rDyn()
o4["const"] = v4
break
}
case 3: {
let v4
v4 = rDyn()
o4["default"] = v4
break
}
case 4: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rStr()
}
}
o4["description"] = v4
break
}
case 5: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["minimum"] = v4
break
}
case 6: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["maximum"] = v4
break
}
case 7: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["exclusiveMinimum"] = v4
break
}
case 8: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rStr()
}
}
o4["pattern"] = v4
break
}
case 9: {
let v4
v4 = rDyn()
o4["items"] = v4
break
}
case 10: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
a6[i6] = v6
}
v4 = a6
}
}
}
o4["prefixItems"] = v4
break
}
case 11: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["minItems"] = v4
break
}
case 12: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rF64()
}
}
o4["maxItems"] = v4
break
}
case 13: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
v4 = rU8() === 1
}
}
o4["uniqueItems"] = v4
break
}
case 14: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const o6 = {}
for (let i6 = 0; i6 < n6; i6++) {
const k6 = rStr()
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
v7 = rDyn()
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
v7 = rDyn()
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
o6[k6] = v6
}
v4 = o6
}
}
}
o4["properties"] = v4
break
}
case 15: {
let v4
v4 = rDyn()
o4["additionalProperties"] = v4
break
}
case 16: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v4 = o6
}
}
}
o4["propertyNames"] = v4
break
}
case 17: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
v6 = rStr()
a6[i6] = v6
}
v4 = a6
}
}
}
o4["required"] = v4
break
}
case 18: {
let v4
{
const g5 = rU8()
if (g5 === 0) v4 = undefined
else if (g5 === 2) v4 = null
else {
if (rU8() === 0) v4 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v6 = o7
}
a6[i6] = v6
}
v4 = a6
}
}
}
o4["anyOf"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v2 = o4
}
}
}
o2["propertyNames"] = v2
break
}
case 17: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rStr()
a4[i4] = v4
}
v2 = a4
}
}
}
o2["required"] = v2
break
}
case 18: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
a4[i4] = v4
}
v2 = a4
}
}
}
o2["anyOf"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v0 = o2
}
}
}
o0["propertyNames"] = v0
break
}
case 17: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rStr()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["required"] = v0
break
}
case 18: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["type"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["enum"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["const"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["default"] = v3
break
}
case 4: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 5: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minimum"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maximum"] = v3
break
}
case 7: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["exclusiveMinimum"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["pattern"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["items"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["prefixItems"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["minItems"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["maxItems"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["uniqueItems"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const o5 = {}
for (let i5 = 0; i5 < n5; i5++) {
const k5 = rStr()
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
v6 = rDyn()
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
o5[k5] = v5
}
v3 = o5
}
}
}
o3["properties"] = v3
break
}
case 15: {
let v3
v3 = rDyn()
o3["additionalProperties"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["type"] = v5
break
}
case 1: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rDyn()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["enum"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["const"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["default"] = v5
break
}
case 4: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["description"] = v5
break
}
case 5: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minimum"] = v5
break
}
case 6: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maximum"] = v5
break
}
case 7: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["exclusiveMinimum"] = v5
break
}
case 8: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rStr()
}
}
o5["pattern"] = v5
break
}
case 9: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 10: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["prefixItems"] = v5
break
}
case 11: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["minItems"] = v5
break
}
case 12: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rF64()
}
}
o5["maxItems"] = v5
break
}
case 13: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
v5 = rU8() === 1
}
}
o5["uniqueItems"] = v5
break
}
case 14: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const o7 = {}
for (let i7 = 0; i7 < n7; i7++) {
const k7 = rStr()
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
v8 = rDyn()
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
v8 = rDyn()
o8["description"] = v8
break
}
case 5: {
let v8
v8 = rDyn()
o8["minimum"] = v8
break
}
case 6: {
let v8
v8 = rDyn()
o8["maximum"] = v8
break
}
case 7: {
let v8
v8 = rDyn()
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
v8 = rDyn()
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
v8 = rDyn()
o8["prefixItems"] = v8
break
}
case 11: {
let v8
v8 = rDyn()
o8["minItems"] = v8
break
}
case 12: {
let v8
v8 = rDyn()
o8["maxItems"] = v8
break
}
case 13: {
let v8
v8 = rDyn()
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
v8 = rDyn()
o8["required"] = v8
break
}
case 18: {
let v8
v8 = rDyn()
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
o7[k7] = v7
}
v5 = o7
}
}
}
o5["properties"] = v5
break
}
case 15: {
let v5
v5 = rDyn()
o5["additionalProperties"] = v5
break
}
case 16: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["type"] = v7
break
}
case 1: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["enum"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["const"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["default"] = v7
break
}
case 4: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["description"] = v7
break
}
case 5: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minimum"] = v7
break
}
case 6: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maximum"] = v7
break
}
case 7: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["exclusiveMinimum"] = v7
break
}
case 8: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rStr()
}
}
o7["pattern"] = v7
break
}
case 9: {
let v7
v7 = rDyn()
o7["items"] = v7
break
}
case 10: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["prefixItems"] = v7
break
}
case 11: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["minItems"] = v7
break
}
case 12: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rF64()
}
}
o7["maxItems"] = v7
break
}
case 13: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
v7 = rU8() === 1
}
}
o7["uniqueItems"] = v7
break
}
case 14: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const o9 = {}
for (let i9 = 0; i9 < n9; i9++) {
const k9 = rStr()
let v9
v9 = rDyn()
o9[k9] = v9
}
v7 = o9
}
}
}
o7["properties"] = v7
break
}
case 15: {
let v7
v7 = rDyn()
o7["additionalProperties"] = v7
break
}
case 16: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v7 = o9
}
}
}
o7["propertyNames"] = v7
break
}
case 17: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rStr()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["required"] = v7
break
}
case 18: {
let v7
{
const g8 = rU8()
if (g8 === 0) v7 = undefined
else if (g8 === 2) v7 = null
else {
if (rU8() === 0) v7 = null
else {
const n9 = rU32()
const a9 = new Array(n9)
for (let i9 = 0; i9 < n9; i9++) {
let v9
v9 = rDyn()
a9[i9] = v9
}
v7 = a9
}
}
}
o7["anyOf"] = v7
break
}
default: {
const k7 = rStr()
o7[k7] = rDyn()
}
}
}
v5 = o7
}
}
}
o5["propertyNames"] = v5
break
}
case 17: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
v7 = rStr()
a7[i7] = v7
}
v5 = a7
}
}
}
o5["required"] = v5
break
}
case 18: {
let v5
{
const g6 = rU8()
if (g6 === 0) v5 = undefined
else if (g6 === 2) v5 = null
else {
if (rU8() === 0) v5 = null
else {
const n7 = rU32()
const a7 = new Array(n7)
for (let i7 = 0; i7 < n7; i7++) {
let v7
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v7 = o8
}
a7[i7] = v7
}
v5 = a7
}
}
}
o5["anyOf"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["propertyNames"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rStr()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["required"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["type"] = v6
break
}
case 1: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rDyn()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["enum"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["const"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["default"] = v6
break
}
case 4: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["description"] = v6
break
}
case 5: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minimum"] = v6
break
}
case 6: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maximum"] = v6
break
}
case 7: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["exclusiveMinimum"] = v6
break
}
case 8: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rStr()
}
}
o6["pattern"] = v6
break
}
case 9: {
let v6
v6 = rDyn()
o6["items"] = v6
break
}
case 10: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["prefixItems"] = v6
break
}
case 11: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["minItems"] = v6
break
}
case 12: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rF64()
}
}
o6["maxItems"] = v6
break
}
case 13: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
v6 = rU8() === 1
}
}
o6["uniqueItems"] = v6
break
}
case 14: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const o8 = {}
for (let i8 = 0; i8 < n8; i8++) {
const k8 = rStr()
let v8
v8 = rDyn()
o8[k8] = v8
}
v6 = o8
}
}
}
o6["properties"] = v6
break
}
case 15: {
let v6
v6 = rDyn()
o6["additionalProperties"] = v6
break
}
case 16: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
{
const o8 = {}
const n8 = rU8()
for (let j8 = 0; j8 < n8; j8++) {
switch (rU8()) {
case 0: {
let v8
v8 = rDyn()
o8["type"] = v8
break
}
case 1: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["enum"] = v8
break
}
case 2: {
let v8
v8 = rDyn()
o8["const"] = v8
break
}
case 3: {
let v8
v8 = rDyn()
o8["default"] = v8
break
}
case 4: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["description"] = v8
break
}
case 5: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minimum"] = v8
break
}
case 6: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maximum"] = v8
break
}
case 7: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["exclusiveMinimum"] = v8
break
}
case 8: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rStr()
}
}
o8["pattern"] = v8
break
}
case 9: {
let v8
v8 = rDyn()
o8["items"] = v8
break
}
case 10: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["prefixItems"] = v8
break
}
case 11: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["minItems"] = v8
break
}
case 12: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rF64()
}
}
o8["maxItems"] = v8
break
}
case 13: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
v8 = rU8() === 1
}
}
o8["uniqueItems"] = v8
break
}
case 14: {
let v8
v8 = rDyn()
o8["properties"] = v8
break
}
case 15: {
let v8
v8 = rDyn()
o8["additionalProperties"] = v8
break
}
case 16: {
let v8
v8 = rDyn()
o8["propertyNames"] = v8
break
}
case 17: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["required"] = v8
break
}
case 18: {
let v8
{
const g9 = rU8()
if (g9 === 0) v8 = undefined
else if (g9 === 2) v8 = null
else {
if (rU8() === 0) v8 = null
else {
const n10 = rU32()
const a10 = new Array(n10)
for (let i10 = 0; i10 < n10; i10++) {
let v10
v10 = rDyn()
a10[i10] = v10
}
v8 = a10
}
}
}
o8["anyOf"] = v8
break
}
default: {
const k8 = rStr()
o8[k8] = rDyn()
}
}
}
v6 = o8
}
}
}
o6["propertyNames"] = v6
break
}
case 17: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
v8 = rStr()
a8[i8] = v8
}
v6 = a8
}
}
}
o6["required"] = v6
break
}
case 18: {
let v6
{
const g7 = rU8()
if (g7 === 0) v6 = undefined
else if (g7 === 2) v6 = null
else {
if (rU8() === 0) v6 = null
else {
const n8 = rU32()
const a8 = new Array(n8)
for (let i8 = 0; i8 < n8; i8++) {
let v8
{
const o9 = {}
const n9 = rU8()
for (let j9 = 0; j9 < n9; j9++) {
switch (rU8()) {
case 0: {
let v9
v9 = rDyn()
o9["type"] = v9
break
}
case 1: {
let v9
v9 = rDyn()
o9["enum"] = v9
break
}
case 2: {
let v9
v9 = rDyn()
o9["const"] = v9
break
}
case 3: {
let v9
v9 = rDyn()
o9["default"] = v9
break
}
case 4: {
let v9
v9 = rDyn()
o9["description"] = v9
break
}
case 5: {
let v9
v9 = rDyn()
o9["minimum"] = v9
break
}
case 6: {
let v9
v9 = rDyn()
o9["maximum"] = v9
break
}
case 7: {
let v9
v9 = rDyn()
o9["exclusiveMinimum"] = v9
break
}
case 8: {
let v9
v9 = rDyn()
o9["pattern"] = v9
break
}
case 9: {
let v9
v9 = rDyn()
o9["items"] = v9
break
}
case 10: {
let v9
v9 = rDyn()
o9["prefixItems"] = v9
break
}
case 11: {
let v9
v9 = rDyn()
o9["minItems"] = v9
break
}
case 12: {
let v9
v9 = rDyn()
o9["maxItems"] = v9
break
}
case 13: {
let v9
v9 = rDyn()
o9["uniqueItems"] = v9
break
}
case 14: {
let v9
v9 = rDyn()
o9["properties"] = v9
break
}
case 15: {
let v9
v9 = rDyn()
o9["additionalProperties"] = v9
break
}
case 16: {
let v9
v9 = rDyn()
o9["propertyNames"] = v9
break
}
case 17: {
let v9
v9 = rDyn()
o9["required"] = v9
break
}
case 18: {
let v9
v9 = rDyn()
o9["anyOf"] = v9
break
}
default: {
const k9 = rStr()
o9[k9] = rDyn()
}
}
}
v8 = o9
}
a8[i8] = v8
}
v6 = a8
}
}
}
o6["anyOf"] = v6
break
}
default: {
const k6 = rStr()
o6[k6] = rDyn()
}
}
}
v5 = o6
}
a5[i5] = v5
}
v3 = a5
}
}
}
o3["anyOf"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v0 = a2
}
}
}
o0["anyOf"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.rowSchema = function rowSchema (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of rowSchema has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}
const guards = {

}
const JS_ERRORS = /^(TypeError|RangeError|SyntaxError|ReferenceError|URIError|EvalError): ([\s\S]*)$/
const jsError = e => {
  const message = typeof e?.message === 'string' ? e.message : ''
  const typed = JS_ERRORS.exec(message)
  if (typed) return new globalThis[typed[1]](typed[2])
  if (/Expected$/.test(e?.code)) return Object.assign(new TypeError(message), { code: e.code })
  if (message.startsWith('Rust panic:')) e.code = 'RUST_PANIC'
  return e
}
const illFormed = v => typeof v === 'string' ? !v.isWellFormed() : Array.isArray(v) && v.some(illFormed)
for (const [name, [isAsync, strings]] of Object.entries(guards)) {
  const f = module.exports[name]
  if (typeof f !== 'function') continue
  const check = a => {
    for (const i of strings) {
      if (illFormed(a[i])) throw new TypeError(`argument ${i + 1} of ${name} has a lone surrogate, which a Rust String cannot hold`)
    }
  }
  module.exports[name] = isAsync
    ? { [name]: async function (...a) { check(a); try { return await f(...a) } catch (e) { throw jsError(e) } } }[name]
    : { [name]: function (...a) { check(a); try { return f(...a) } catch (e) { throw jsError(e) } } }[name]
}

// Loads the native module built by `rust-anything build`.
const native = require('./theme_rs.node')

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
  padColor: [[0, 0, 1], 'json', false],
  padPosition: [[1], 'json', false],
  oklab: [[1], 'json', false],
  fromOklab: [[1], 'json', false],
  colorHex: [[1], 'raw', false],
  hexColor: [[0], 'json', false],
  mixOklab: [[1, 1, 0], 'json', false],
  lookAngle: [[1], 'raw', false],
  themeArea: [[1, 0, 0, 0, 0], 'json', false],
  themeDark: [[1, 0], 'raw', false],
  resolveTheme: [[1, 0], 'json', false],
  darkenedLook: [[1, 1], 'json', false],
  requestedDarkFor: [[1, 0, 0], 'raw', false],
  shownLook: [[1, 0, 0], 'json', false],
  themeLayerAlpha: [[1], 'raw', false],
  themeUnderlayAlpha: [[1, 0], 'raw', false],
  themeGrainAlpha: [[1], 'raw', false],
  flatColors: [[1], 'raw', false],
  themeAccent: [[1], 'raw', false],
  themePalette: [[1], 'raw', false],
  themeUnderColor: [[1, 0], 'raw', false],
  opaqueThemeTint: [[1, 0], 'json', false],
  themeLuminance: [[1, 0], 'raw', false],
  readableDark: [[1, 0], 'raw', false],
  stopLightness: [[1], 'raw', false],
  withStopAt: [[1, 0, 0, 0], 'json', false],
  withStopLightness: [[1, 0, 0], 'json', false],
  withAngle: [[1, 0], 'json', false],
  withAddedStop: [[1], 'json', false],
  withoutStop: [[1, 0], 'json', false],
  withTexture: [[1, 1], 'json', false],
  startingTheme: [[0], 'json', false],
  sanitizeTheme: [[1], 'json', false],
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

// `padColor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.padColor
const call = native.padColorWire
const encode = (a0, a1, a2) => {
at = 0
if (typeof a0 !== 'number') return false
wF64(a0)
if (typeof a1 !== 'number') return false
wF64(a1)
if (!wDyn(a2)) return false
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
v0 = rF64()
o0["r"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["g"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["b"] = v0
break
}
case 3: {
let v0
v0 = rF64()
o0["a"] = v0
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
module.exports.padColor = function padColor (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `padPosition`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.padPosition
const call = native.padPositionWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
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
v0 = rF64()
o0["x"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["y"] = v0
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
module.exports.padPosition = function padPosition (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `oklab`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.oklab
const call = native.oklabWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.oklab = function oklab (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `fromOklab`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.fromOklab
const call = native.fromOklabWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.fromOklab = function fromOklab (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `colorHex`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.colorHex
const call = native.colorHexWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.colorHex = function colorHex (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `hexColor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.hexColor
const call = native.hexColorWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
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
v0 = rF64()
o0["r"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["g"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["b"] = v0
break
}
case 3: {
let v0
v0 = rF64()
o0["a"] = v0
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
module.exports.hexColor = function hexColor (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of hexColor has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mixOklab`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mixOklab
const call = native.mixOklabWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
if (typeof a2 !== 'number') return false
wF64(a2)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.mixOklab = function mixOklab (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `lookAngle`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.lookAngle
const call = native.lookAngleWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.lookAngle = function lookAngle (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `themeArea`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.themeArea
const call = native.themeAreaWire
const encode = (a0, a1, a2, a3, a4) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'number') return false
wF64(a1)
if (typeof a2 !== 'number') return false
wF64(a2)
if (typeof a3 !== 'number') return false
wF64(a3)
if (typeof a4 !== 'number') return false
wF64(a4)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.themeArea = function themeArea (a0, a1, a2, a3, a4) {
  try {
    if (arguments.length <= 5 && encode(a0, a1, a2, a3, a4) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `themeDark`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.themeDark
const call = native.themeDarkWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.themeDark = function themeDark (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveTheme`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveTheme
const call = native.resolveThemeWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["r"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["g"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["b"] = v3
break
}
case 3: {
let v3
v3 = rF64()
o3["a"] = v3
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
o2["color"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["x"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["y"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
a1[i1] = v1
}
v0 = a1
}
o0["stops"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["intensity"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["grain"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["texture"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["translucency"] = v0
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
o0["angle"] = v0
break
}
case 6: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 7: {
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
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rF64()
o5["r"] = v5
break
}
case 1: {
let v5
v5 = rF64()
o5["g"] = v5
break
}
case 2: {
let v5
v5 = rF64()
o5["b"] = v5
break
}
case 3: {
let v5
v5 = rF64()
o5["a"] = v5
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
o4["color"] = v4
break
}
case 1: {
let v4
v4 = rF64()
o4["x"] = v4
break
}
case 2: {
let v4
v4 = rF64()
o4["y"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v3 = o4
}
a3[i3] = v3
}
v2 = a3
}
o2["stops"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["intensity"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["grain"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["texture"] = v2
break
}
case 4: {
let v2
v2 = rF64()
o2["translucency"] = v2
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
o2["angle"] = v2
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
o0["dark"] = v0
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
module.exports.resolveTheme = function resolveTheme (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `darkenedLook`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.darkenedLook
const call = native.darkenedLookWire
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["r"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["g"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["b"] = v3
break
}
case 3: {
let v3
v3 = rF64()
o3["a"] = v3
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
o2["color"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["x"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["y"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
a1[i1] = v1
}
v0 = a1
}
o0["stops"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["intensity"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["grain"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["texture"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["translucency"] = v0
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
o0["angle"] = v0
break
}
case 6: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 7: {
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
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rF64()
o5["r"] = v5
break
}
case 1: {
let v5
v5 = rF64()
o5["g"] = v5
break
}
case 2: {
let v5
v5 = rF64()
o5["b"] = v5
break
}
case 3: {
let v5
v5 = rF64()
o5["a"] = v5
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
o4["color"] = v4
break
}
case 1: {
let v4
v4 = rF64()
o4["x"] = v4
break
}
case 2: {
let v4
v4 = rF64()
o4["y"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v3 = o4
}
a3[i3] = v3
}
v2 = a3
}
o2["stops"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["intensity"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["grain"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["texture"] = v2
break
}
case 4: {
let v2
v2 = rF64()
o2["translucency"] = v2
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
o2["angle"] = v2
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
o0["dark"] = v0
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
module.exports.darkenedLook = function darkenedLook (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requestedDarkFor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requestedDarkFor
const call = native.requestedDarkForWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
if (typeof a2 !== 'string') return false
wStr(a2)
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.requestedDarkFor = function requestedDarkFor (a0, a1, a2) {
  if (illFormed(a2)) throw new TypeError('argument 3 of requestedDarkFor has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `shownLook`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.shownLook
const call = native.shownLookWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
if (typeof a2 !== 'string') return false
wStr(a2)
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
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
if (rU8() === 0) v1 = null
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
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
v4 = rF64()
o4["r"] = v4
break
}
case 1: {
let v4
v4 = rF64()
o4["g"] = v4
break
}
case 2: {
let v4
v4 = rF64()
o4["b"] = v4
break
}
case 3: {
let v4
v4 = rF64()
o4["a"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v3 = o4
}
o3["color"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["x"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["y"] = v3
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
v1 = a2
}
o1["stops"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["intensity"] = v1
break
}
case 2: {
let v1
v1 = rF64()
o1["grain"] = v1
break
}
case 3: {
let v1
v1 = rDyn()
o1["texture"] = v1
break
}
case 4: {
let v1
v1 = rF64()
o1["translucency"] = v1
break
}
case 5: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rF64()
}
}
o1["angle"] = v1
break
}
case 6: {
let v1
v1 = rStr()
o1["mode"] = v1
break
}
case 7: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
if (rU8() === 0) v3 = null
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
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rDyn()
o6["r"] = v6
break
}
case 1: {
let v6
v6 = rDyn()
o6["g"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["b"] = v6
break
}
case 3: {
let v6
v6 = rDyn()
o6["a"] = v6
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
o5["color"] = v5
break
}
case 1: {
let v5
v5 = rF64()
o5["x"] = v5
break
}
case 2: {
let v5
v5 = rF64()
o5["y"] = v5
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
v3 = a4
}
o3["stops"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["intensity"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["grain"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["texture"] = v3
break
}
case 4: {
let v3
v3 = rF64()
o3["translucency"] = v3
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
o3["angle"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v1 = o3
}
}
}
o1["dark"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["look"] = v0
break
}
case 1: {
let v0
v0 = rU8() === 1
o0["requestedDark"] = v0
break
}
case 2: {
let v0
v0 = rU8() === 1
o0["dark"] = v0
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
module.exports.shownLook = function shownLook (a0, a1, a2) {
  if (illFormed(a2)) throw new TypeError('argument 3 of shownLook has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `arcLayerAlpha`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.arcLayerAlpha
const call = native.arcLayerAlphaWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'number') return false
wF64(a0)
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.arcLayerAlpha = function arcLayerAlpha (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `themeLayerAlpha`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.themeLayerAlpha
const call = native.themeLayerAlphaWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.themeLayerAlpha = function themeLayerAlpha (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `themeUnderlayAlpha`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.themeUnderlayAlpha
const call = native.themeUnderlayAlphaWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.themeUnderlayAlpha = function themeUnderlayAlpha (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `themeGrainAlpha`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.themeGrainAlpha
const call = native.themeGrainAlphaWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.themeGrainAlpha = function themeGrainAlpha (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `flatColors`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.flatColors
const call = native.flatColorsWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
if (rU8() === 0) t = null
else {
const n0 = rU32()
const a0 = new Array(n0)
for (let i0 = 0; i0 < n0; i0++) {
let v0
v0 = rF64()
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.flatColors = function flatColors (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `themeAccent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.themeAccent
const call = native.themeAccentWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.themeAccent = function themeAccent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `themePalette`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.themePalette
const call = native.themePaletteWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
if (rU8() === 0) t = null
else {
const n0 = rU32()
const a0 = new Array(n0)
for (let i0 = 0; i0 < n0; i0++) {
let v0
v0 = rStr()
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.themePalette = function themePalette (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `backdropUnderColor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.backdropUnderColor
const call = native.backdropUnderColorWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'number') return false
wF64(a0)
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.backdropUnderColor = function backdropUnderColor (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `themeUnderColor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.themeUnderColor
const call = native.themeUnderColorWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.themeUnderColor = function themeUnderColor (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `opaqueThemeTint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.opaqueThemeTint
const call = native.opaqueThemeTintWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.opaqueThemeTint = function opaqueThemeTint (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `themeLuminance`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.themeLuminance
const call = native.themeLuminanceWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.themeLuminance = function themeLuminance (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readableDark`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readableDark
const call = native.readableDarkWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.readableDark = function readableDark (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `stopLightness`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.stopLightness
const call = native.stopLightnessWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.stopLightness = function stopLightness (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withStopAt`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withStopAt
const call = native.withStopAtWire
const encode = (a0, a1, a2, a3) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'number') return false
wF64(a1)
if (typeof a2 !== 'number') return false
wF64(a2)
if (typeof a3 !== 'number') return false
wF64(a3)
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["r"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["g"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["b"] = v3
break
}
case 3: {
let v3
v3 = rF64()
o3["a"] = v3
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
o2["color"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["x"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["y"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
a1[i1] = v1
}
v0 = a1
}
o0["stops"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["intensity"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["grain"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["texture"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["translucency"] = v0
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
o0["angle"] = v0
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
module.exports.withStopAt = function withStopAt (a0, a1, a2, a3) {
  try {
    if (arguments.length <= 4 && encode(a0, a1, a2, a3) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withStopLightness`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withStopLightness
const call = native.withStopLightnessWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'number') return false
wF64(a1)
if (typeof a2 !== 'number') return false
wF64(a2)
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["r"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["g"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["b"] = v3
break
}
case 3: {
let v3
v3 = rF64()
o3["a"] = v3
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
o2["color"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["x"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["y"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
a1[i1] = v1
}
v0 = a1
}
o0["stops"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["intensity"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["grain"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["texture"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["translucency"] = v0
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
o0["angle"] = v0
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
module.exports.withStopLightness = function withStopLightness (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withAngle`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withAngle
const call = native.withAngleWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'number') return false
wF64(a1)
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["r"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["g"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["b"] = v3
break
}
case 3: {
let v3
v3 = rF64()
o3["a"] = v3
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
o2["color"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["x"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["y"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
a1[i1] = v1
}
v0 = a1
}
o0["stops"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["intensity"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["grain"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["texture"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["translucency"] = v0
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
o0["angle"] = v0
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
module.exports.withAngle = function withAngle (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withAddedStop`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withAddedStop
const call = native.withAddedStopWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["r"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["g"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["b"] = v3
break
}
case 3: {
let v3
v3 = rF64()
o3["a"] = v3
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
o2["color"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["x"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["y"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
a1[i1] = v1
}
v0 = a1
}
o0["stops"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["intensity"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["grain"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["texture"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["translucency"] = v0
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
o0["angle"] = v0
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
module.exports.withAddedStop = function withAddedStop (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withoutStop`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withoutStop
const call = native.withoutStopWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'number') return false
wF64(a1)
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["r"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["g"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["b"] = v3
break
}
case 3: {
let v3
v3 = rF64()
o3["a"] = v3
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
o2["color"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["x"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["y"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
a1[i1] = v1
}
v0 = a1
}
o0["stops"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["intensity"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["grain"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["texture"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["translucency"] = v0
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
o0["angle"] = v0
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
module.exports.withoutStop = function withoutStop (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withTexture`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withTexture
const call = native.withTextureWire
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["r"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["g"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["b"] = v3
break
}
case 3: {
let v3
v3 = rF64()
o3["a"] = v3
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
o2["color"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["x"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["y"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
a1[i1] = v1
}
v0 = a1
}
o0["stops"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["intensity"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["grain"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["texture"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["translucency"] = v0
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
o0["angle"] = v0
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
module.exports.withTexture = function withTexture (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `startingTheme`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.startingTheme
const call = native.startingThemeWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["r"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["g"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["b"] = v3
break
}
case 3: {
let v3
v3 = rF64()
o3["a"] = v3
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
o2["color"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["x"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["y"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
a1[i1] = v1
}
v0 = a1
}
o0["stops"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["intensity"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["grain"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["texture"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["translucency"] = v0
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
o0["angle"] = v0
break
}
case 6: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 7: {
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
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rF64()
o5["r"] = v5
break
}
case 1: {
let v5
v5 = rF64()
o5["g"] = v5
break
}
case 2: {
let v5
v5 = rF64()
o5["b"] = v5
break
}
case 3: {
let v5
v5 = rF64()
o5["a"] = v5
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
o4["color"] = v4
break
}
case 1: {
let v4
v4 = rF64()
o4["x"] = v4
break
}
case 2: {
let v4
v4 = rF64()
o4["y"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v3 = o4
}
a3[i3] = v3
}
v2 = a3
}
o2["stops"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["intensity"] = v2
break
}
case 2: {
let v2
v2 = rF64()
o2["grain"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["texture"] = v2
break
}
case 4: {
let v2
v2 = rF64()
o2["translucency"] = v2
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
o2["angle"] = v2
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
o0["dark"] = v0
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
module.exports.startingTheme = function startingTheme (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of startingTheme has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `sanitizeTheme`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.sanitizeTheme
const call = native.sanitizeThemeWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
if (rU8() === 0) v1 = null
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
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
v4 = rF64()
o4["r"] = v4
break
}
case 1: {
let v4
v4 = rF64()
o4["g"] = v4
break
}
case 2: {
let v4
v4 = rF64()
o4["b"] = v4
break
}
case 3: {
let v4
v4 = rF64()
o4["a"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v3 = o4
}
o3["color"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["x"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["y"] = v3
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
v1 = a2
}
o1["stops"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["intensity"] = v1
break
}
case 2: {
let v1
v1 = rF64()
o1["grain"] = v1
break
}
case 3: {
let v1
v1 = rDyn()
o1["texture"] = v1
break
}
case 4: {
let v1
v1 = rF64()
o1["translucency"] = v1
break
}
case 5: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rF64()
}
}
o1["angle"] = v1
break
}
case 6: {
let v1
v1 = rStr()
o1["mode"] = v1
break
}
case 7: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
if (rU8() === 0) v3 = null
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
{
const o6 = {}
const n6 = rU8()
for (let j6 = 0; j6 < n6; j6++) {
switch (rU8()) {
case 0: {
let v6
v6 = rF64()
o6["r"] = v6
break
}
case 1: {
let v6
v6 = rF64()
o6["g"] = v6
break
}
case 2: {
let v6
v6 = rF64()
o6["b"] = v6
break
}
case 3: {
let v6
v6 = rF64()
o6["a"] = v6
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
o5["color"] = v5
break
}
case 1: {
let v5
v5 = rF64()
o5["x"] = v5
break
}
case 2: {
let v5
v5 = rF64()
o5["y"] = v5
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
v3 = a4
}
o3["stops"] = v3
break
}
case 1: {
let v3
v3 = rF64()
o3["intensity"] = v3
break
}
case 2: {
let v3
v3 = rF64()
o3["grain"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["texture"] = v3
break
}
case 4: {
let v3
v3 = rF64()
o3["translucency"] = v3
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
o3["angle"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v1 = o3
}
}
}
o1["dark"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
t = o1
}
}
}
return t
}
module.exports.sanitizeTheme = function sanitizeTheme (a0) {
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

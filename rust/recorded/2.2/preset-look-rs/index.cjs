// Loads the native module built by `rust-anything build`.
const native = require('./preset_look_rs.node')

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
  presetStartingTheme: [[0], 'json', false],
  presetThemeOf: [[1, 1], 'json', false],
  snapPointToStart: [[1, 1, 0], 'json', false],
  isUntouched: [[1, 1], 'raw', false],
  sameTheme: [[1, 1], 'raw', false],
  withOwnLook: [[1, 1], 'json', false],
  withPresetChosen: [[1, 1], 'json', false],
  withSyncedColor: [[1, 1, 1], 'json', false],
  editedPresetCount: [[1], 'raw', false],
  withOwnLooksSeeded: [[1], 'json', false],
  profilesWithOwnLooks: [[1], 'json', false],
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

// `presetStartingTheme`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.presetStartingTheme
const call = native.presetStartingThemeWire
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
module.exports.presetStartingTheme = function presetStartingTheme (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of presetStartingTheme has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `presetThemeOf`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.presetThemeOf
const call = native.presetThemeOfWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
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
module.exports.presetThemeOf = function presetThemeOf (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `snapToStart`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.snapToStart
const call = native.snapToStartWire
const encode = (a0, a1, a2) => {
at = 0
if (typeof a0 !== 'number') return false
wF64(a0)
if (typeof a1 !== 'number') return false
wF64(a1)
if (typeof a2 !== 'number') return false
wF64(a2)
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.snapToStart = function snapToStart (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `snapPointToStart`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.snapPointToStart
const call = native.snapPointToStartWire
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
module.exports.snapPointToStart = function snapPointToStart (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isUntouched`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isUntouched
const call = native.isUntouchedWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.isUntouched = function isUntouched (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `sameTheme`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.sameTheme
const call = native.sameThemeWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.sameTheme = function sameTheme (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withOwnLook`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withOwnLook
const call = native.withOwnLookWire
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
v5 = rDyn()
o5["r"] = v5
break
}
case 1: {
let v5
v5 = rDyn()
o5["g"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["b"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
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
case 6: {
let v2
v2 = rStr()
o2["mode"] = v2
break
}
case 7: {
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
if (rU8() === 0) v4 = null
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
o6["color"] = v6
break
}
case 1: {
let v6
v6 = rF64()
o6["x"] = v6
break
}
case 2: {
let v6
v6 = rF64()
o6["y"] = v6
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
v4 = a5
}
o4["stops"] = v4
break
}
case 1: {
let v4
v4 = rF64()
o4["intensity"] = v4
break
}
case 2: {
let v4
v4 = rF64()
o4["grain"] = v4
break
}
case 3: {
let v4
v4 = rDyn()
o4["texture"] = v4
break
}
case 4: {
let v4
v4 = rF64()
o4["translucency"] = v4
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
o4["angle"] = v4
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
o2["dark"] = v2
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
o0["theme"] = v0
break
}
case 1: {
let v0
v0 = rDyn()
o0["presetThemes"] = v0
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
module.exports.withOwnLook = function withOwnLook (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withPresetChosen`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withPresetChosen
const call = native.withPresetChosenWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
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
v1 = rDyn()
o1["color"] = v1
break
}
case 1: {
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
v5 = rDyn()
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
case 6: {
let v3
v3 = rStr()
o3["mode"] = v3
break
}
case 7: {
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
if (rU8() === 0) v5 = null
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
o7["color"] = v7
break
}
case 1: {
let v7
v7 = rDyn()
o7["x"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["y"] = v7
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
v5 = a6
}
o5["stops"] = v5
break
}
case 1: {
let v5
v5 = rF64()
o5["intensity"] = v5
break
}
case 2: {
let v5
v5 = rF64()
o5["grain"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["texture"] = v5
break
}
case 4: {
let v5
v5 = rF64()
o5["translucency"] = v5
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
o5["angle"] = v5
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
o3["dark"] = v3
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
o1["theme"] = v1
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
module.exports.withPresetChosen = function withPresetChosen (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withSyncedColor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withSyncedColor
const call = native.withSyncedColorWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
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
v1 = rDyn()
o1["color"] = v1
break
}
case 1: {
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
v5 = rDyn()
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
case 6: {
let v3
v3 = rStr()
o3["mode"] = v3
break
}
case 7: {
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
if (rU8() === 0) v5 = null
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
o7["color"] = v7
break
}
case 1: {
let v7
v7 = rDyn()
o7["x"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["y"] = v7
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
v5 = a6
}
o5["stops"] = v5
break
}
case 1: {
let v5
v5 = rF64()
o5["intensity"] = v5
break
}
case 2: {
let v5
v5 = rF64()
o5["grain"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["texture"] = v5
break
}
case 4: {
let v5
v5 = rF64()
o5["translucency"] = v5
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
o5["angle"] = v5
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
o3["dark"] = v3
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
o1["theme"] = v1
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
module.exports.withSyncedColor = function withSyncedColor (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `editedPresetCount`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.editedPresetCount
const call = native.editedPresetCountWire
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
module.exports.editedPresetCount = function editedPresetCount (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withOwnLooksSeeded`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withOwnLooksSeeded
const call = native.withOwnLooksSeededWire
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
v0 = rDyn()
o0["color"] = v0
break
}
case 1: {
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
v4 = rDyn()
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
case 6: {
let v2
v2 = rStr()
o2["mode"] = v2
break
}
case 7: {
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
if (rU8() === 0) v4 = null
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
o6["color"] = v6
break
}
case 1: {
let v6
v6 = rDyn()
o6["x"] = v6
break
}
case 2: {
let v6
v6 = rDyn()
o6["y"] = v6
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
v4 = a5
}
o4["stops"] = v4
break
}
case 1: {
let v4
v4 = rF64()
o4["intensity"] = v4
break
}
case 2: {
let v4
v4 = rF64()
o4["grain"] = v4
break
}
case 3: {
let v4
v4 = rDyn()
o4["texture"] = v4
break
}
case 4: {
let v4
v4 = rF64()
o4["translucency"] = v4
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
o4["angle"] = v4
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
o2["dark"] = v2
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
o0["theme"] = v0
break
}
case 2: {
let v0
v0 = rDyn()
o0["presetThemes"] = v0
break
}
case 3: {
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
case 6: {
let v2
v2 = rStr()
o2["mode"] = v2
break
}
case 7: {
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
if (rU8() === 0) v4 = null
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
{
const o7 = {}
const n7 = rU8()
for (let j7 = 0; j7 < n7; j7++) {
switch (rU8()) {
case 0: {
let v7
v7 = rDyn()
o7["r"] = v7
break
}
case 1: {
let v7
v7 = rDyn()
o7["g"] = v7
break
}
case 2: {
let v7
v7 = rDyn()
o7["b"] = v7
break
}
case 3: {
let v7
v7 = rDyn()
o7["a"] = v7
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
o6["color"] = v6
break
}
case 1: {
let v6
v6 = rF64()
o6["x"] = v6
break
}
case 2: {
let v6
v6 = rF64()
o6["y"] = v6
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
v4 = a5
}
o4["stops"] = v4
break
}
case 1: {
let v4
v4 = rF64()
o4["intensity"] = v4
break
}
case 2: {
let v4
v4 = rF64()
o4["grain"] = v4
break
}
case 3: {
let v4
v4 = rDyn()
o4["texture"] = v4
break
}
case 4: {
let v4
v4 = rF64()
o4["translucency"] = v4
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
o4["angle"] = v4
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
o2["dark"] = v2
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
o0["seenTheme"] = v0
break
}
case 4: {
let v0
v0 = rDyn()
o0["seenColor"] = v0
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
module.exports.withOwnLooksSeeded = function withOwnLooksSeeded (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `profilesWithOwnLooks`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.profilesWithOwnLooks
const call = native.profilesWithOwnLooksWire
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
const o0 = {}
for (let i0 = 0; i0 < n0; i0++) {
const k0 = rStr()
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rDyn()
o1["color"] = v1
break
}
case 1: {
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
v4 = rDyn()
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
case 6: {
let v3
v3 = rStr()
o3["mode"] = v3
break
}
case 7: {
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
if (rU8() === 0) v5 = null
else {
const n6 = rU32()
const a6 = new Array(n6)
for (let i6 = 0; i6 < n6; i6++) {
let v6
v6 = rDyn()
a6[i6] = v6
}
v5 = a6
}
o5["stops"] = v5
break
}
case 1: {
let v5
v5 = rF64()
o5["intensity"] = v5
break
}
case 2: {
let v5
v5 = rF64()
o5["grain"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["texture"] = v5
break
}
case 4: {
let v5
v5 = rF64()
o5["translucency"] = v5
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
o5["angle"] = v5
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
o3["dark"] = v3
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
o1["theme"] = v1
break
}
case 2: {
let v1
v1 = rDyn()
o1["presetThemes"] = v1
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
o0[k0] = v0
}
t = o0
}
return t
}
module.exports.profilesWithOwnLooks = function profilesWithOwnLooks (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `trackValueAt`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.trackValueAt
const call = native.trackValueAtWire
const encode = (a0, a1, a2, a3) => {
at = 0
if (typeof a0 !== 'number') return false
wF64(a0)
if (typeof a1 !== 'number') return false
wF64(a1)
if (typeof a2 !== 'number') return false
wF64(a2)
if (a3 === undefined) {
wU8(0)
} else if (a3 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a3 !== 'number') return false
wF64(a3)
}
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rF64()
}
}
return t
}
module.exports.trackValueAt = function trackValueAt (a0, a1, a2, a3) {
  try {
    if (arguments.length <= 4 && encode(a0, a1, a2, a3) && wireCall(call)) return decode()
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

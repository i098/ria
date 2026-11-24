// Ria original, no Ardia source.
// Makes the typed engine API of `ria-engine` from `packages/ardiacore/src/engine/api.ts` (Ardia, commit 551d3c13c).
//
// Usage: node gen.mjs <ardia checkout> [--check]
//   writes  rust/crates/ria-engine/src/api/*.rs   (one file for each call domain, `events.rs`, `mod.rs`)
//           rust/crates/ria-engine/src/types/*.rs (one file for each TypeScript file that holds a used type)
//   --check writes nothing and exits 1 when a file would change.
//
// The TypeScript compiler resolves each type, so the Rust types follow `api.ts` and the files it imports.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import ts from "typescript";

const args = process.argv.slice(2);
const check = args.includes("--check");
const ardia = args.find((a) => !a.startsWith("--"));
if (!ardia) throw new Error("usage: node gen.mjs <ardia checkout> [--check]");
const here = path.dirname(fileURLToPath(import.meta.url));
const crate = path.resolve(here, "../../crates/ria-engine/src");
const apiFile = path.join(ardia, "packages/ardiacore/src/engine/api.ts");

const program = ts.createProgram([apiFile], {
  strict: true,
  noEmit: true,
  skipLibCheck: true,
  target: ts.ScriptTarget.ES2022,
  module: ts.ModuleKind.ESNext,
  moduleResolution: ts.ModuleResolutionKind.Bundler,
  jsx: ts.JsxEmit.Preserve,
  types: [],
});
const checker = program.getTypeChecker();
const isProject = (sf) => !sf.isDeclarationFile && !sf.fileName.includes("/node_modules/");

// ---- naming helpers
const pascal = (s) => {
  const parts = s.split(/[^A-Za-z0-9]+/).filter(Boolean).map((p) => p[0].toUpperCase() + p.slice(1));
  let name = parts.join("");
  if (!name) name = "Empty";
  return /^[0-9]/.test(name) ? "V" + name : name;
};
const snake = (s) => s.replace(/([a-z0-9])([A-Z])/g, "$1_$2").replace(/([A-Z]+)([A-Z][a-z])/g, "$1_$2").toLowerCase();
const RESERVED = new Set("as break const continue crate else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while async await dyn abstract become box do final macro override priv typeof unsized virtual yield try gen".split(" "));
const fieldIdent = (name) => {
  const id = snake(name).replace(/[^a-z0-9_]/g, "_");
  const safe = /^[0-9]/.test(id) ? "f_" + id : id;
  return RESERVED.has(safe) ? (["self", "Self", "super", "crate"].includes(safe) ? safe + "_" : "r#" + safe) : safe;
};
const fieldRename = (name, ident) => (ident.replace(/^r#/, "").replace(/_$/, "") === name ? null : name);

// ---- project aliases by type identity (the alias symbol is lost for mapped types such as `Partial<Omit<..>>`)
const aliasByType = new Map();
const fileKey = (sf) => "types/" + snake(path.basename(sf.fileName).replace(/\.tsx?$/, "")).replace(/^api$/, "engine_api");
for (const sf of program.getSourceFiles()) {
  if (!isProject(sf)) continue;
  for (const st of sf.statements) {
    if (ts.isTypeAliasDeclaration(st) && !st.typeParameters) {
      const type = checker.getTypeAtLocation(st.name);
      if (!aliasByType.has(type)) aliasByType.set(type, { name: st.name.text, file: fileKey(sf) });
    }
  }
}

// ---- declarations
const decls = new Map(); // name -> { file, code }
const RUST_RESERVED_NAMES = new Set(["Number", "Value", "Option", "Vec", "String", "HashMap", "Call", "Event", "NoArgs"]);

function declare(name, file, build) {
  if (RUST_RESERVED_NAMES.has(name)) name += "Type";
  // A name is unique among the types; the inline types of the call files are unique in their file.
  const key = file.startsWith("api/") ? `${file}/${name}` : name;
  const existing = decls.get(key);
  if (existing) {
    if (existing.file !== file) throw new Error(`type name ${name} is declared in ${existing.file} and ${file}`);
    return name;
  }
  decls.set(key, { file, code: "" }); // stops a recursive type from looping
  decls.get(key).code = build(name);
  return name;
}

const ref = (name, file, from) => (file.startsWith("types/") && from.startsWith("api/") ? `crate::types::${name}` : name);

const isLit = (t) => t.flags & ts.TypeFlags.StringLiteral;
const isBoolLit = (t) => t.flags & ts.TypeFlags.BooleanLiteral;

function fail(type, why) {
  throw new Error(`${why}: ${checker.typeToString(type)}`);
}

/** The Rust type of a TypeScript type. `hint` names a new type; `from` is the file that holds the reference. */
function rust(type, hint, from) {
  const named = aliasByType.get(type);
  const lib = type.aliasSymbol && !(type.aliasSymbol.declarations ?? []).some((d) => isProject(d.getSourceFile()));
  const name = named?.name ?? (type.aliasSymbol && !lib ? type.aliasSymbol.name : null);
  const file = named?.file ?? from;
  const parts = type.isUnion() ? type.types : [type];
  const rest = parts.filter((t) => !(t.flags & (ts.TypeFlags.Undefined | ts.TypeFlags.Void)));
  const nullable = rest.some((t) => t.flags & ts.TypeFlags.Null);
  const core = rest.filter((t) => !(t.flags & ts.TypeFlags.Null));
  let out;
  if (core.length === 0) out = "()";
  else if (core.every(isBoolLit)) out = "bool";
  else out = restCore(core, name ?? hint, file, from, !!name);
  return nullable && core.length ? `Option<${out}>` : out;
}

function restCore(core, hint, file, from, named) {
  if (core.length === 1) return single(core[0], hint, file, from, named);
  if (core.every(isLit)) return literalEnum(core, hint, file, from);
  if (core.every(objectish)) return taggedEnum(core, hint, file, from);
  return untaggedEnum(core, hint, file, from);
}

const objectish = (t) => t.flags & (ts.TypeFlags.Object | ts.TypeFlags.Intersection);

/** A union of unlike types, such as `number | false`: each variant is tried in order. */
function untaggedEnum(core, hint, file, from) {
  const name = declare(hint, file, (n) => {
    const variants = [];
    const used = new Set();
    let bool = false;
    core.forEach((t, i) => {
      if (isBoolLit(t)) {
        if (bool) return;
        bool = true;
        variants.push("    Bool(bool),");
        return;
      }
      const rt = single(t, `${n}Variant${i}`, file, file, false);
      let id = t.flags & ts.TypeFlags.Number ? "Number" : t.flags & (ts.TypeFlags.String | ts.TypeFlags.StringLiteral) ? "String" : `Other${i}`;
      if (used.has(id)) return;
      used.add(id);
      variants.push(`    ${id}(${rt}),`);
    });
    return `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n#[serde(untagged)]\npub enum ${n} {\n${variants.join("\n")}\n}`;
  });
  return ref(name, file, from);
}

function single(t, hint, file, from, named) {
  const F = ts.TypeFlags;
  if (t.flags & F.String) return "String";
  if (t.flags & F.Number) return "Number";
  if (t.flags & F.Boolean || isBoolLit(t)) return "bool";
  if (t.flags & (F.Unknown | F.Any)) return "serde_json::Value";
  if (t.flags & F.Never) return "serde_json::Value";
  if (isLit(t)) return "String";
  if (!objectish(t)) fail(t, "unsupported type");
  if (checker.isArrayType(t)) return `Vec<${rust(checker.getTypeArguments(t)[0], hint + "Item", from)}>`;
  if (checker.isTupleType(t)) return `(${checker.getTypeArguments(t).map((e, i) => rust(e, hint + "Item" + i, from)).join(", ")})`;
  const index = checker.getIndexInfoOfType(t, ts.IndexKind.String);
  if (index && checker.getPropertiesOfType(t).length === 0) return `std::collections::HashMap<String, ${rust(index.type, hint + "Value", from)}>`;
  if (t.aliasSymbol?.name === "Record" && t.aliasTypeArguments) {
    const [k, v] = t.aliasTypeArguments;
    return `std::collections::HashMap<${rust(k, hint + "Key", from)}, ${rust(v, hint + "Value", from)}>`;
  }
  return struct(t, hint, file, from);
}

function literalEnum(types, hint, file, from) {
  const values = types.map((t) => t.value);
  const name = declare(hint, file, (n) => {
    const used = new Set();
    const variants = values.map((v) => {
      const id = pascal(v);
      if (used.has(id)) throw new Error(`enum ${n}: two values give the variant ${id}`);
      used.add(id);
      return `    #[serde(rename = ${JSON.stringify(v)})]\n    ${id},`;
    });
    return `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub enum ${n} {\n${variants.join("\n")}\n}`;
  });
  return ref(name, file, from);
}

// Fields that `api.ts` declares required but the engine omits on some replies. `search.suggest` sends only
// `{ stopped: true }` on stop (ac_search_suggest.cc:85).
const ENGINE_OPTIONAL = new Set(["SuggestReply.status"]);

function fieldsOf(type, hint, file, from, indent) {
  const out = [];
  for (const prop of checker.getPropertiesOfType(type)) {
    const decl = prop.valueDeclaration ?? prop.declarations?.[0];
    const pt = checker.getTypeOfSymbol(prop);
    const optional = !!(prop.flags & ts.SymbolFlags.Optional) || ENGINE_OPTIONAL.has(`${hint}.${prop.name}`);
    let rt = rust(pt, hint + pascal(prop.name), file);
    if (optional && !rt.startsWith("Option<")) rt = `Option<${rt}>`;
    const ident = fieldIdent(prop.name);
    const rename = fieldRename(prop.name, ident);
    const attrs = [];
    if (rename) attrs.push(`rename = ${JSON.stringify(rename)}`);
    if (optional) attrs.push(`default`, `skip_serializing_if = "Option::is_none"`);
    out.push(`${indent}${attrs.length ? `#[serde(${attrs.join(", ")})] ` : ""}pub ${ident}: ${rt},`);
  }
  return out;
}

function struct(type, hint, file, from) {
  const name = declare(hint, file, (n) => {
    const fields = fieldsOf(type, n, file, from, "    ");
    return `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\npub struct ${n} {${fields.length ? "\n" + fields.join("\n") + "\n" : ""}}`;
  });
  return ref(name, file, from);
}

/** The string values of a literal or a union of literals, else null. */
function tagValues(t) {
  const parts = t.isUnion() ? t.types : [t];
  return parts.every(isLit) ? parts.map((x) => x.value) : null;
}

function taggedEnum(types, hint, file, from) {
  const props = checker.getPropertiesOfType(types[0]);
  const tag = props.find((p) =>
    types.every((t) => {
      const q = checker.getPropertyOfType(t, p.name);
      return q && tagValues(checker.getTypeOfSymbol(q));
    }),
  );
  if (!tag) return untaggedEnum(types, hint, file, from);
  const name = declare(hint, file, (n) => {
    const entries = types.flatMap((t) => tagValues(checker.getTypeOfSymbol(checker.getPropertyOfType(t, tag.name))).map((value) => ({ t, value })));
    const variants = entries.map(({ t, value }) => {
      const variantName = pascal(value);
      const members = checker.getPropertiesOfType(t).filter((p) => p.name !== tag.name);
      const body = [];
      for (const prop of members) {
        const pt = checker.getTypeOfSymbol(prop);
        const optional = !!(prop.flags & ts.SymbolFlags.Optional);
        let rt = rust(pt, n + variantName + pascal(prop.name), file);
        if (optional && !rt.startsWith("Option<")) rt = `Option<${rt}>`;
        const ident = fieldIdent(prop.name);
        const rename = fieldRename(prop.name, ident);
        const attrs = [];
        if (rename) attrs.push(`rename = ${JSON.stringify(rename)}`);
        if (optional) attrs.push(`default`, `skip_serializing_if = "Option::is_none"`);
        body.push(`        ${attrs.length ? `#[serde(${attrs.join(", ")})] ` : ""}${ident}: ${rt},`);
      }
      return `    #[serde(rename = ${JSON.stringify(value)})]\n    ${variantName}${body.length ? ` {\n${body.join("\n")}\n    }` : ""},`;
    });
    return `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n#[serde(tag = ${JSON.stringify(tag.name)})]\npub enum ${n} {\n${variants.join("\n")}\n}`;
  });
  return ref(name, file, from);
}

// ---- read api.ts
const sf = program.getSourceFile(apiFile);
const group = (aliasName) => {
  const alias = sf.statements.find((s) => ts.isTypeAliasDeclaration(s) && s.name.text === aliasName);
  if (!alias) throw new Error(`api.ts has no type ${aliasName}`);
  return checker.getPropertiesOfType(checker.getTypeAtLocation(alias.name));
};

const domainOf = (n) => n.slice(0, n.indexOf("."));
const restOf = (n) => n.slice(n.indexOf(".") + 1);
const HEADER = "// Ria original, no Ardia source.\n// Generated by rust/tools/engine-api-gen/gen.mjs from packages/ardiacore/src/engine/api.ts at 551d3c13c. Do not edit.\n\n";

const calls = group("EngineCalls").map((p) => {
  const tuple = checker.getTypeOfSymbol(p);
  const [argsType, replyType] = checker.getTypeArguments(tuple);
  return { name: p.name, argsType, replyType };
});

const files = new Map(); // relative path -> text
const domains = [...new Set(calls.map((c) => domainOf(c.name)))];
const domainFile = (d) => "api/" + snake(d);

for (const d of domains) {
  const body = [];
  for (const c of calls.filter((c) => domainOf(c.name) === d)) {
    const marker = pascal(restOf(c.name));
    const from = domainFile(d);
    let args;
    if (checker.getPropertiesOfType(c.argsType).length === 0 && !c.argsType.aliasSymbol) args = "super::NoArgs";
    else {
      args = rust(c.argsType, marker + "Args", from);
    }
    const reply = rust(c.replyType, marker + "Reply", from);
    body.push(
      `/// \`${c.name}\`.\npub struct ${marker};\n\nimpl Call for ${marker} {\n    const NAME: &'static str = ${JSON.stringify(c.name)};\n    type Args = ${args};\n    type Reply = ${reply};\n}`,
    );
  }
  files.set(`api/${snake(d)}`, { head: `//! Calls of the \`${d}\` domain.\n`, uses: "Call", body, key: domainFile(d) });
}

const eventGroups = [
  ["EngineEvents", "ENGINE_EVENTS", "Events that the engine sends (`EngineEvents` of `api.ts`)."],
  ["WindowsHostEvents", "WINDOWS_HOST_EVENTS", "Events of the Windows host (`WindowsHostEvents`); the Mac host has none of them."],
  ["LinuxHostEvents", "LINUX_HOST_EVENTS", "Events of the Linux host (`LinuxHostEvents`); the Mac host has none of them."],
];
const eventBody = [];
const eventCounts = {};
for (const [alias, constant, doc] of eventGroups) {
  const names = [];
  for (const p of group(alias)) {
    const marker = pascal(domainOf(p.name)) + pascal(restOf(p.name));
    const data = rust(checker.getTypeOfSymbol(p), marker + "Data", "api/events");
    eventBody.push(`/// \`${p.name}\`.\npub struct ${marker};\n\nimpl Event for ${marker} {\n    const NAME: &'static str = ${JSON.stringify(p.name)};\n    type Data = ${data};\n}`);
    names.push(p.name);
  }
  eventCounts[alias] = names.length;
  eventBody.push(`/// ${doc}\npub const ${constant}: &[&str] = &[\n${names.map((n) => `    ${JSON.stringify(n)},`).join("\n")}\n];`);
}
files.set("api/events", { head: "//! Events of `api.ts`.\n", uses: "Event", body: eventBody, key: "api/events" });

if (calls.length !== 184) throw new Error(`api.ts has ${calls.length} calls, the plan says 184`);
if (eventCounts.EngineEvents !== 34) throw new Error(`api.ts has ${eventCounts.EngineEvents} engine events, the plan says 34`);

// ---- write out
const out = new Map();
const imports = (text, trait) => {
  const lines = [];
  if (/\b(Serialize|Deserialize)\b/.test(text)) lines.push("use serde::{Deserialize, Serialize};");
  if (/(^|[^:\w])Number\b/m.test(text)) lines.push("use crate::types::Number;");
  if (trait) lines.push(`use super::${trait};`);
  return lines.length ? "\n" + lines.join("\n") + "\n" : "";
};
for (const [file, { head, uses, body, key }] of files) {
  const local = [...decls.values()].filter((d) => d.file === key).map((d) => d.code);
  const text = (local.length ? "\n" + local.join("\n\n") + "\n" : "") + "\n" + body.join("\n\n") + "\n";
  out.set(file + ".rs", HEADER + head + imports(text, uses) + text);
}

const typeFiles = [...new Set([...decls.values()].map((d) => d.file).filter((f) => f.startsWith("types/")))].sort();
for (const f of typeFiles) {
  const code = [...decls.values()].filter((d) => d.file === f).map((d) => d.code).join("\n\n");
  out.set(`${f}.rs`, `${HEADER}use serde::{Deserialize, Serialize};\n\n#[allow(unused_imports)]\nuse super::*;\n\n${code}\n`);
}
out.set(
  "types/mod.rs",
  `${HEADER}//! The wire types of the engine API. One file for each TypeScript file of Ardia that holds a used type.\n\nmod number;\n${typeFiles.map((f) => `mod ${path.basename(f)};`).join("\n")}\n\npub use number::Number;\n${typeFiles.map((f) => `pub use ${path.basename(f)}::*;`).join("\n")}\n`,
);
out.set(
  "types/number.rs",
  `${HEADER}use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A TypeScript \`number\`. It serializes as an integer when it has no fraction, because the engine reads ids and counts as integers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Number(pub f64);

impl Serialize for Number {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let n = self.0;
        if n.is_finite() && n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
            serializer.serialize_i64(n as i64)
        } else {
            serializer.serialize_f64(n)
        }
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Any;
        impl Visitor<'_> for Any {
            type Value = Number;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a number")
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Number, E> {
                Ok(Number(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Number, E> {
                Ok(Number(v as f64))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Number, E> {
                Ok(Number(v as f64))
            }
        }
        deserializer.deserialize_any(Any)
    }
}

impl From<f64> for Number {
    fn from(v: f64) -> Self {
        Self(v)
    }
}

impl From<i32> for Number {
    fn from(v: i32) -> Self {
        Self(f64::from(v))
    }
}

#[cfg(test)]
mod tests {
    use super::Number;

    #[test]
    fn whole_numbers_serialize_without_a_fraction() {
        assert_eq!(serde_json::to_string(&Number(5.0)).unwrap(), "5");
        assert_eq!(serde_json::to_string(&Number(1_700_000_000_000.0)).unwrap(), "1700000000000");
        assert_eq!(serde_json::to_string(&Number(1.5)).unwrap(), "1.5");
        assert_eq!(serde_json::to_string(&Number(-0.0)).unwrap(), "0");
    }

    #[test]
    fn integers_and_floats_deserialize() {
        assert_eq!(serde_json::from_str::<Number>("7").unwrap(), Number(7.0));
        assert_eq!(serde_json::from_str::<Number>("-2.25").unwrap(), Number(-2.25));
        assert!(serde_json::from_str::<Number>("\\"7\\"").is_err());
    }
}
`,
);

const domainMods = domains.map((d) => snake(d)).sort();
out.set(
  "api/mod.rs",
  `${HEADER}//! The typed engine API: ${calls.length} calls and ${eventCounts.EngineEvents} engine events of \`api.ts\`.
//!
//! A [\`Call\`] marker names one call of \`api.ts\` and gives its argument and reply types. The map from a call name
//! to an engine symbol (\`ac_*\`) and the change of the argument shape belong to the domain code of slices 3.3 and 3.4;
//! [\`call_symbol\`] sends a typed call through a symbol that the caller gives.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::call::{CallError, CallResult, Engine};

${domainMods.map((m) => `pub mod ${m};`).join("\n")}
pub mod events;

/// One call of \`EngineCalls\` in \`api.ts\`.
pub trait Call {
    /// The name in \`api.ts\`, for example \`engine.info\`.
    const NAME: &'static str;
    type Args: Serialize;
    type Reply: DeserializeOwned;
}

/// One event of \`api.ts\`.
pub trait Event {
    /// The name in \`api.ts\`, for example \`tabs.strips\`.
    const NAME: &'static str;
    type Data: DeserializeOwned;
}

/// The arguments of a call that has none (\`{}\` in \`api.ts\`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, serde::Deserialize)]
pub struct NoArgs {}

#[derive(Debug, PartialEq)]
pub enum ApiError {
    Call(CallError),
    /// The reply or the event data does not have the type of \`api.ts\`.
    Decode(String),
}

/// The JSON object that a call sends. A \`profile\` field is not sent: the profile folder goes in the call itself, as \`call.ts\` does.
pub fn encode_args<C: Call>(args: &C::Args) -> Result<Value, CallError> {
    let mut value = serde_json::to_value(args).map_err(|_| CallError::BadArguments)?;
    match &mut value {
        Value::Object(object) => {
            object.remove("profile");
            Ok(value)
        }
        _ => Err(CallError::BadArguments),
    }
}

/// Reads the reply of call \`C\`.
pub fn decode_reply<C: Call>(result: CallResult) -> Result<C::Reply, ApiError> {
    let value = result.map_err(ApiError::Call)?;
    serde_json::from_value(value).map_err(|e| ApiError::Decode(e.to_string()))
}

/// Sends call \`C\` through the engine symbol \`symbol\` with its arguments as they are (the \`passing\` calls of Ardia).
/// \`completion\` runs on a drain, once.
pub fn call_symbol<C: Call>(
    engine: &Engine,
    symbol: &str,
    profile_dir: &str,
    private: bool,
    args: &C::Args,
    completion: impl FnOnce(Result<C::Reply, ApiError>) + 'static,
) where
    C::Reply: 'static,
{
    match encode_args::<C>(args) {
        Ok(value) => engine.call(symbol, profile_dir, private, value, move |r| completion(decode_reply::<C>(r))),
        Err(error) => engine.call(symbol, profile_dir, private, Value::Null, move |_| completion(Err(ApiError::Call(error)))),
    }
}

/// Runs \`handler\` for each event \`E\` that arrives on topic \`topic\`. The topic is the C name of the event, which differs from
/// \`E::NAME\` for a few events (\`tabs.strip\` and \`tabs.strips\`); slice 3.4 keeps the map.
pub fn subscribe<E: Event>(engine: &Engine, topic: &str, handler: impl Fn(Result<E::Data, ApiError>) + 'static) {
    engine.on_event(topic, move |value| {
        handler(serde_json::from_value(value.clone()).map_err(|e| ApiError::Decode(e.to_string())))
    });
}
`,
);

// The files are written in the form of `rustfmt`, so a check after `cargo fmt` finds no change.
const format = (text) => {
  const r = spawnSync("rustfmt", ["--edition", "2024", "--emit", "stdout"], { input: text, encoding: "utf8" });
  if (r.status !== 0) throw new Error(`rustfmt failed: ${r.stderr}`);
  // `--emit stdout` prints a file name line before the text for stdin input on some versions
  return r.stdout.replace(/^.*<stdin>:\n\n?/, "");
};

let changed = false;
for (const [rel, formatted] of out) {
  const text = format(formatted);
  const target = path.join(crate, rel);
  const old = fs.existsSync(target) ? fs.readFileSync(target, "utf8") : null;
  if (old === text) continue;
  changed = true;
  if (check) console.error(`would change: ${rel}`);
  else {
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, text);
  }
}
console.log(`${calls.length} calls in ${domains.length} domains, ${eventCounts.EngineEvents}+${eventCounts.WindowsHostEvents}+${eventCounts.LinuxHostEvents} events, ${decls.size} types, ${out.size} files${check ? (changed ? ", out of date" : ", up to date") : ""}`);
if (check && changed) process.exit(1);

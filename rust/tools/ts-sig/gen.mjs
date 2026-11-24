// Ria original, no Ardia source.
// Makes the skeleton of `ria-state` from the TypeScript stores of Ardia (commit 551d3c13c): one module for each of the
// 41 folders of `apps/browser/src/state`, with `State`, `Action` and `apply`, and the kernel list of the modules.
//
// Usage: node gen.mjs <ardia checkout> [--check]
//   writes  rust/crates/ria-state/src/lib.rs
//           rust/crates/ria-state/src/kernel/modules.rs      (`AppState`, `AppAction`, one `Module` impl for each module)
//           rust/crates/ria-state/src/<module>/mod.rs         (41 files)
//   --check writes nothing and exits 1 when a file would change.
//
// Rules (README.md has the reasons):
//   store      a top-level variable of a state file that has `getState` and `setState` (a zustand store). Its state type is
//              the return type of `getState`.
//   State      the data properties (no function) of the store state types. A property belongs to the folder of the file
//              that declares it, so the slices of `useBrowser` go to the folders that declare them.
//   Action     the function properties of the store state types (store methods), and each exported function of a state
//              file that writes a store: it calls `setState`, a store method, or an exported function that does.
//   types      one Rust type for each TypeScript type, in the module of the folder that declares it.
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
const crate = path.resolve(here, "../../crates/ria-state/src");
const stateDir = path.join(path.resolve(ardia), "apps/browser/src/state");

/** The slice of the plan (section 8.7) that fills each module. */
const SLICE = {
  browser: "2.5", telemetry: "2.6", settings: "2.7", tabs: "2.8", windows: "2.9", app: "2.10", commands: "2.11",
  profiles: "2.12", pins: "2.13", splits: "2.14", omnibox: "2.15", bookmarks: "2.16", plugins: "2.17", sidebar: "2.18",
  peek: "2.19", site: "2.20", engines: "2.21", extensions: "2.22", groups: "2.23", pages: "2.24", config: "2.25",
  favicons: "2.26", history: "2.27", onboarding: "2.28", automation: "2.29", media: "2.30", sync: "2.31", drag: "2.32",
  session: "2.33", downloads: "2.34", focus: "2.35", littleArdia: "2.36", live: "2.37", reader: "2.38", airTraffic: "2.39",
  import: "2.40", sharedFolders: "2.41", find: "2.42", presets: "2.43", releaseNotes: "2.44", report: "2.45",
};

// ---- naming helpers (the rules of engine-api-gen)
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
const moduleName = (folder) => snake(folder);

// ---- the TypeScript program over the state files
const PLATFORM_VARIANT = /\.(windows|linux|android|web)\.tsx?$/;
const folders = fs.readdirSync(stateDir, { withFileTypes: true }).filter((d) => d.isDirectory()).map((d) => d.name).sort();
if (folders.length !== 41) throw new Error(`${stateDir} has ${folders.length} folders, the plan says 41`);
for (const f of folders) if (!SLICE[f]) throw new Error(`no slice for the folder ${f}`);
const stateFiles = folders.flatMap((f) =>
  fs.readdirSync(path.join(stateDir, f))
    .filter((n) => /\.tsx?$/.test(n) && !/\.d\.ts$/.test(n) && !/\.test\./.test(n) && !PLATFORM_VARIANT.test(n))
    .map((n) => path.join(stateDir, f, n)),
);
const stubs = fs.readdirSync(path.join(here, "stubs")).map((n) => path.join(here, "stubs", n));

const program = ts.createProgram([...stateFiles, ...stubs], {
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
const isProject = (sf) => !sf.isDeclarationFile && !sf.fileName.includes("/node_modules/") && sf.fileName.startsWith(path.resolve(ardia));
const folderOfFile = (file) => {
  const rel = path.relative(stateDir, file);
  return rel.startsWith("..") ? null : rel.split(path.sep)[0];
};
const stateFileSet = new Set(stateFiles);
const warnings = [];
const warn = (msg) => warnings.push(msg);

// ---- stores
const stores = []; // { folder, name, state }
const methodDecls = new Set(); // the declaration nodes of store methods
const methods = []; // { folder, name, sig, decl }
const data = new Map(); // folder -> Map(name -> { type, optional, decl })
for (const f of folders) data.set(f, new Map());

const callSig = (type) => checker.getSignaturesOfType(type, ts.SignatureKind.Call);
const isFunctionType = (type) => callSig(type).length > 0 && checker.getPropertiesOfType(type).length === 0;

for (const file of stateFiles) {
  const sf = program.getSourceFile(file);
  if (!sf) continue;
  const folder = folderOfFile(file);
  for (const st of sf.statements) {
    if (!ts.isVariableStatement(st)) continue;
    for (const d of st.declarationList.declarations) {
      if (!ts.isIdentifier(d.name)) continue;
      const type = checker.getTypeAtLocation(d.name);
      const get = type.getProperty("getState");
      if (!get || !type.getProperty("setState")) continue;
      const sig = callSig(checker.getTypeOfSymbolAtLocation(get, d))[0];
      if (!sig) continue;
      const state = checker.getReturnTypeOfSignature(sig);
      stores.push({ folder, name: d.name.text, state });
    }
  }
}

for (const store of stores) {
  for (const prop of checker.getPropertiesOfType(store.state)) {
    const decl = prop.valueDeclaration ?? prop.declarations?.[0];
    const declFolder = (decl && folderOfFile(decl.getSourceFile().fileName)) || store.folder;
    const type = checker.getTypeOfSymbol(prop);
    if (isFunctionType(type)) {
      if (decl && methodDecls.has(decl)) continue;
      if (decl) methodDecls.add(decl);
      methods.push({ folder: declFolder, name: prop.name, sig: callSig(type)[0], decl, store: store.name });
    } else {
      const bucket = data.get(declFolder);
      if (bucket.has(prop.name)) {
        if (bucket.get(prop.name).store !== store.name) warn(`${declFolder}: field ${prop.name} is in the stores ${bucket.get(prop.name).store} and ${store.name}`);
        continue;
      }
      bucket.set(prop.name, { type, optional: !!(prop.flags & ts.SymbolFlags.Optional), store: store.name });
    }
  }
}

// ---- exported functions that write a store
const fnDecls = new Map(); // symbol -> { node, folder, name, exported, file }
const resolve = (node) => {
  let sym = checker.getSymbolAtLocation(node);
  if (sym && sym.flags & ts.SymbolFlags.Alias) sym = checker.getAliasedSymbol(sym);
  return sym;
};
const exportedNames = (sf) => {
  const names = new Set();
  for (const st of sf.statements) {
    if (ts.isExportDeclaration(st) && !st.moduleSpecifier && st.exportClause && ts.isNamedExports(st.exportClause)) {
      for (const el of st.exportClause.elements) names.add((el.propertyName ?? el.name).text);
    }
  }
  return names;
};
for (const file of stateFiles) {
  const sf = program.getSourceFile(file);
  if (!sf) continue;
  const folder = folderOfFile(file);
  const named = exportedNames(sf);
  const isExported = (st, name) => !!ts.getCombinedModifierFlags(st) && (ts.getCombinedModifierFlags(st) & ts.ModifierFlags.Export) !== 0 || named.has(name);
  for (const st of sf.statements) {
    if (ts.isFunctionDeclaration(st) && st.name && st.body) {
      const sym = resolve(st.name);
      if (sym) fnDecls.set(sym, { node: st, folder, name: st.name.text, exported: isExported(st, st.name.text), file });
    } else if (ts.isVariableStatement(st)) {
      for (const d of st.declarationList.declarations) {
        if (!ts.isIdentifier(d.name) || !d.initializer) continue;
        if (!(ts.isArrowFunction(d.initializer) || ts.isFunctionExpression(d.initializer))) continue;
        const sym = resolve(d.name);
        if (sym) fnDecls.set(sym, { node: d.initializer, folder, name: d.name.text, exported: isExported(st, d.name.text), file });
      }
    }
  }
}

const memo = new Map();
function writes(sym) {
  if (memo.has(sym)) return memo.get(sym);
  memo.set(sym, false); // a cycle adds nothing
  const info = fnDecls.get(sym);
  let result = false;
  const visit = (node) => {
    if (result) return;
    if (ts.isCallExpression(node)) {
      const callee = node.expression;
      if (ts.isPropertyAccessExpression(callee) && callee.name.text === "setState") {
        result = true;
        return;
      }
      const target = resolve(ts.isPropertyAccessExpression(callee) ? callee.name : callee);
      if (target) {
        if ((target.declarations ?? []).some((d) => methodDecls.has(d))) {
          result = true;
          return;
        }
        if (fnDecls.has(target) && writes(target)) {
          result = true;
          return;
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  if (info) ts.forEachChild(info.node, visit);
  memo.set(sym, result);
  return result;
}

const functionActions = [];
for (const [sym, info] of fnDecls) {
  if (!info.exported) continue;
  if (/^use[A-Z]/.test(info.name)) continue; // a React hook reads
  const params = info.node.parameters.map((p) => p.name.getText());
  if (params[0] === "set" && params[1] === "get") continue; // a store creator
  if (!writes(sym)) continue;
  const sig = checker.getSignatureFromDeclaration(info.node);
  functionActions.push({ folder: info.folder, name: info.name, sig, decl: info.node, file: info.file });
}

// ---- Rust types
const decls = new Map(); // `${folder}::${name}` -> { folder, name, code, building }
const byType = new Map(); // ts type or declaration key -> { folder, name }
const usedNames = new Map(); // folder -> Set(name)
for (const f of folders) usedNames.set(f, new Set(["State", "Action", "Number", "Serialize", "Deserialize", "AppState", "Effects", "Option", "Vec", "String", "Box", "HashMap", "Value"]));

function uniqueName(folder, base) {
  const used = usedNames.get(folder);
  let name = base;
  for (let i = 2; used.has(name); i++) name = base + i;
  used.add(name);
  return name;
}

const isLit = (t) => t.flags & ts.TypeFlags.StringLiteral;
const isNumLit = (t) => t.flags & ts.TypeFlags.NumberLiteral;
const isBoolLit = (t) => t.flags & ts.TypeFlags.BooleanLiteral;
const objectish = (t) => t.flags & (ts.TypeFlags.Object | ts.TypeFlags.Intersection);
const declFolderOf = (decl) => {
  const sf = decl?.getSourceFile();
  return sf ? folderOfFile(sf.fileName) : null;
};

/** The project declaration that names a type: an alias or an interface or a class. Null for an anonymous type. */
function declarationOf(type) {
  const alias = type.aliasSymbol;
  if (alias && !type.aliasTypeArguments) {
    const d = alias.declarations?.find((x) => isProject(x.getSourceFile()));
    if (d) return { name: alias.name, decl: d };
  }
  const sym = type.symbol;
  if (sym && !type.aliasSymbol && !(type.flags & ts.TypeFlags.Intersection)) {
    const d = sym.declarations?.find((x) => (ts.isInterfaceDeclaration(x) || ts.isClassDeclaration(x)) && isProject(x.getSourceFile()));
    const generic = d && (d.typeParameters?.length ?? 0) > 0;
    if (d && !generic && sym.name !== "__type" && sym.name !== "__object") return { name: sym.name, decl: d };
  }
  return null;
}

/** Declares one Rust type. The module is the folder of the TypeScript declaration, or `from` for an anonymous type. */
function declare(key, hint, folder, build) {
  const known = byType.get(key);
  if (known) return known;
  const name = uniqueName(folder, pascal(hint));
  const entry = { folder, name, code: "", building: true };
  byType.set(key, entry);
  decls.set(`${folder}::${name}`, entry);
  entry.code = build(name);
  entry.building = false;
  return entry;
}

const refPath = (entry, from) => (entry.folder === from ? entry.name : `crate::${moduleName(entry.folder)}::${entry.name}`);
/** A reference to a type that is still being built goes through a `Box`. */
const refTo = (entry, from) => {
  const p = refPath(entry, from);
  return entry.building ? { text: `Box<${p}>`, boxed: true } : { text: p, boxed: false };
};

function fail(type, why, hint) {
  warn(`${why}: ${checker.typeToString(type)} (${hint})`);
  return "serde_json::Value";
}

/** The Rust type of a TypeScript type. `hint` names a new type; `from` is the folder that holds the reference. */
function rust(type, hint, from) {
  const named = declarationOf(type);
  const parts = type.isUnion() ? type.types : [type];
  const rest = parts.filter((t) => !(t.flags & (ts.TypeFlags.Undefined | ts.TypeFlags.Void)));
  const nullable = rest.some((t) => t.flags & ts.TypeFlags.Null);
  const core = rest.filter((t) => !(t.flags & ts.TypeFlags.Null));
  let out;
  if (core.length === 0) out = "()";
  else if (core.every(isBoolLit)) out = "bool";
  else out = restCore(core, named, hint, from, type);
  return nullable && core.length ? `Option<${out}>` : out;
}

function restCore(core, named, hint, from, whole) {
  if (core.length === 1) return single(core[0], core[0] === whole ? named : declarationOf(core[0]), hint, from);
  const folder = (named && declFolderOf(named.decl)) || from;
  const name = named?.name ?? hint;
  const key = named ? named.decl : whole.aliasSymbol ? whole : core.map((t) => t.id).join(",") + "@" + folder + "/" + name;
  if (core.every((t) => isLit(t))) return literalEnum(core, key, name, folder, from);
  if (core.every(objectish)) return taggedEnum(core, key, name, folder, from);
  if (core.every((t) => isNumLit(t) || t.flags & ts.TypeFlags.Number)) return "Number";
  return untaggedEnum(core, key, name, folder, from);
}

function untaggedEnum(core, key, name, folder, from) {
  const entry = declare(key, name, folder, (n) => {
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
      const rt = single(t, declarationOf(t), `${n}Variant${i}`, folder);
      const id = t.flags & (ts.TypeFlags.Number | ts.TypeFlags.NumberLiteral) ? "Number" : t.flags & (ts.TypeFlags.String | ts.TypeFlags.StringLiteral) ? "String" : `Other${i}`;
      if (used.has(id)) return;
      used.add(id);
      variants.push(`    ${id}(${rt}),`);
    });
    const first = variants[0].trim().match(/^(\w+)\(/)[1];
    return `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n#[serde(untagged)]\npub enum ${n} {\n${variants.join("\n")}\n}\n\nimpl Default for ${n} {\n    fn default() -> Self {\n        Self::${first}(Default::default())\n    }\n}`;
  });
  return refTo(entry, from).text;
}

function single(t, named, hint, from) {
  const F = ts.TypeFlags;
  if (t.flags & (F.String | F.TemplateLiteral | F.StringMapping)) return "String";
  if (t.flags & F.Number || isNumLit(t)) return "Number";
  if (t.flags & F.BigInt || t.flags & F.BigIntLiteral) return fail(t, "bigint", hint);
  if (t.flags & F.Boolean || isBoolLit(t)) return "bool";
  if (t.flags & (F.Unknown | F.Any | F.Never)) return "serde_json::Value";
  if (t.flags & (F.Enum | F.EnumLiteral)) return fail(t, "enum", hint);
  if (isLit(t)) return "String";
  if (t.flags & F.NonPrimitive) return "serde_json::Value";
  if (t.flags & F.ESSymbolLike) return fail(t, "symbol", hint);
  if (t.flags & F.TypeParameter) return fail(t, "type parameter", hint);
  if (!objectish(t)) return fail(t, "unsupported type", hint);
  const sym = t.symbol?.name;
  if (sym === "Date") return "String";
  const targs = t.objectFlags & ts.ObjectFlags.Reference ? checker.getTypeArguments(t) : [];
  if (sym === "Set" && targs.length === 1) return `Vec<${rust(targs[0], hint + "Item", from)}>`;
  if (sym === "Map" && targs.length === 2) return `std::collections::HashMap<String, ${rust(targs[1], hint + "Value", from)}>`;
  if (sym === "Promise" || sym === "Function") return "serde_json::Value";
  if (checker.isArrayType(t)) return `Vec<${rust(checker.getTypeArguments(t)[0], hint + "Item", from)}>`;
  if (checker.isTupleType(t)) {
    const elems = checker.getTypeArguments(t);
    return elems.length === 0 ? "()" : `(${elems.map((e, i) => rust(e, hint + "Item" + i, from)).join(", ")}${elems.length === 1 ? "," : ""})`;
  }
  const index = checker.getIndexInfoOfType(t, ts.IndexKind.String);
  if (index && checker.getPropertiesOfType(t).length === 0) return `std::collections::HashMap<String, ${rust(index.type, hint + "Value", from)}>`;
  if (callSig(t).length > 0 && checker.getPropertiesOfType(t).length === 0) return "serde_json::Value";
  return struct(t, named, hint, from);
}

function literalEnum(types, key, name, folder, from) {
  const values = types.map((t) => t.value);
  const entry = declare(key, name, folder, (n) => {
    const used = new Set();
    const variants = values.map((v, i) => {
      const id = pascal(String(v)) + (used.has(pascal(String(v))) ? `V${i}` : "");
      used.add(id);
      return `    ${i === 0 ? "#[default]\n    " : ""}#[serde(rename = ${JSON.stringify(v)})]\n    ${id},`;
    });
    return `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]\npub enum ${n} {\n${variants.join("\n")}\n}`;
  });
  return refTo(entry, from).text;
}

function propList(type) {
  return checker.getPropertiesOfType(type).filter((p) => {
    const t = checker.getTypeOfSymbol(p);
    return !isFunctionType(t);
  });
}

function fieldsOf(type, owner, folder, indent, ownerEntry) {
  const out = [];
  for (const prop of propList(type)) {
    const pt = checker.getTypeOfSymbol(prop);
    const optional = !!(prop.flags & ts.SymbolFlags.Optional);
    let rt = rust(pt, owner + pascal(prop.name), folder);
    if (optional && !rt.startsWith("Option<")) rt = `Option<${rt}>`;
    rt = boxIfBuilding(rt, folder);
    const ident = fieldIdent(prop.name);
    const rename = fieldRename(prop.name, ident);
    const attrs = [];
    if (rename) attrs.push(`rename = ${JSON.stringify(rename)}`);
    if (optional) attrs.push(`default`, `skip_serializing_if = "Option::is_none"`);
    out.push(`${indent}${attrs.length ? `#[serde(${attrs.join(", ")})] ` : ""}pub ${ident}: ${rt},`);
  }
  return out;
}

/** A field whose type is a type that is still being built (a cycle) goes through a `Box`. */
function boxIfBuilding(rt, folder) {
  const inner = rt.startsWith("Option<") ? rt.slice(7, -1) : rt;
  for (const e of decls.values()) {
    if (!e.building) continue;
    if (inner === refPath(e, folder)) return rt.startsWith("Option<") ? `Option<Box<${inner}>>` : `Box<${inner}>`;
  }
  return rt;
}

function struct(type, named, hint, from) {
  const folder = (named && declFolderOf(named.decl)) || from;
  const key = named ? named.decl : type;
  const entry = declare(key, named?.name ?? hint, folder, (n) => {
    const fields = fieldsOf(type, n, folder, "    ");
    return `#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]\npub struct ${n} {${fields.length ? "\n" + fields.join("\n") + "\n" : ""}}`;
  });
  return refTo(entry, from).text;
}

const tagValues = (t) => {
  const parts = t.isUnion() ? t.types : [t];
  return parts.every(isLit) ? parts.map((x) => x.value) : null;
};

function taggedEnum(types, key, name, folder, from) {
  const props = checker.getPropertiesOfType(types[0]);
  const tag = props.find((p) =>
    types.every((t) => {
      const q = checker.getPropertyOfType(t, p.name);
      return q && tagValues(checker.getTypeOfSymbol(q));
    }),
  );
  if (!tag) return untaggedEnum(types, key, name, folder, from);
  const entry = declare(key, name, folder, (n) => {
    const entries = types.flatMap((t) => tagValues(checker.getTypeOfSymbol(checker.getPropertyOfType(t, tag.name))).map((value) => ({ t, value })));
    const used = new Set();
    const variants = entries.map(({ t, value }) => {
      let variantName = pascal(value);
      for (let k = 0; used.has(variantName); k++) variantName = pascal(value) + "Again" + (k ? k + 1 : "");
      used.add(variantName);
      const members = propList(t).filter((p) => p.name !== tag.name);
      const body = members.map((prop) => {
        const pt = checker.getTypeOfSymbol(prop);
        const optional = !!(prop.flags & ts.SymbolFlags.Optional);
        let rt = rust(pt, n + variantName + pascal(prop.name), folder);
        if (optional && !rt.startsWith("Option<")) rt = `Option<${rt}>`;
        rt = boxIfBuilding(rt, folder);
        const ident = fieldIdent(prop.name);
        const rename = fieldRename(prop.name, ident);
        const attrs = [];
        if (rename) attrs.push(`rename = ${JSON.stringify(rename)}`);
        if (optional) attrs.push(`default`, `skip_serializing_if = "Option::is_none"`);
        return `        ${attrs.length ? `#[serde(${attrs.join(", ")})] ` : ""}${ident}: ${rt},`;
      });
      return { variantName, value, body, dup: variantName !== pascal(value) };
    });
    const text = variants.map((v) => `    ${v.dup ? "/// Same tag value as an earlier variant; serde cannot tell them apart.\n    #[serde(skip)]" : `#[serde(rename = ${JSON.stringify(v.value)})]`}\n    ${v.variantName}${v.body.length ? ` {\n${v.body.join("\n")}\n    }` : ""},`);
    const first = variants[0];
    const dflt = first.body.length
      ? `Self::${first.variantName} {\n${first.body.map((l) => `            ${l.trim().replace(/^#\[serde\([^)]*\)\] /, "").split(":")[0]}: Default::default(),`).join("\n")}\n        }`
      : `Self::${first.variantName}`;
    return `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n#[serde(tag = ${JSON.stringify(tag.name)})]\npub enum ${n} {\n${text.join("\n")}\n}\n\nimpl Default for ${n} {\n    fn default() -> Self {\n        ${dflt}\n    }\n}`;
  });
  return refTo(entry, from).text;
}

// ---- State and Action of each module
const byFolder = (list) => {
  const m = new Map(folders.map((f) => [f, []]));
  for (const x of list) m.get(x.folder)?.push(x);
  return m;
};
const methodsBy = byFolder(methods);
const functionsBy = byFolder(functionActions);

function actionVariants(folder) {
  const variants = [];
  const used = new Set();
  const names = new Set();
  const all = [
    ...methodsBy.get(folder).map((m) => ({ ...m, kind: "method" })),
    ...functionsBy.get(folder).map((m) => ({ ...m, kind: "function" })),
  ].sort((a, b) => a.name.localeCompare(b.name) || a.kind.localeCompare(b.kind));
  for (const a of all) {
    const dupName = names.has(a.name);
    names.add(a.name);
    let id = pascal(a.name);
    if (used.has(id)) {
      const stem = a.file ? path.basename(a.file).replace(/\.tsx?$/, "") : a.kind;
      id = pascal(stem) + id;
      for (let i = 2; used.has(id); i++) id += i;
    }
    used.add(id);
    const sig = a.sig;
    const fields = [];
    const skipped = [];
    (sig?.parameters ?? []).forEach((p, i) => {
      const pdecl = p.valueDeclaration;
      const ptype = checker.getTypeOfSymbolAtLocation(p, pdecl ?? a.decl);
      if (isFunctionType(ptype) || (ptype.isUnion() && ptype.types.some((u) => isFunctionType(u)))) {
        skipped.push(p.name);
        return;
      }
      const optional = !!(pdecl && (ts.isParameter(pdecl) && (pdecl.questionToken || pdecl.initializer)));
      const rest = !!(pdecl && ts.isParameter(pdecl) && pdecl.dotDotDotToken);
      const pname = p.name === "type" ? "typeArg" : /^[A-Za-z_$][\w$]*$/.test(p.name) ? p.name : `arg${i}`; // "type" is the Action tag
      let rt = rest ? rust(ptype, id + pascal(pname), folder) : rust(ptype, id + pascal(pname), folder);
      if (optional && !rt.startsWith("Option<")) rt = `Option<${rt}>`;
      const ident = fieldIdent(pname);
      const rename = fieldRename(pname, ident);
      const attrs = [];
      if (rename) attrs.push(`rename = ${JSON.stringify(rename)}`);
      if (optional) attrs.push(`default`);
      fields.push(`        ${attrs.length ? `#[serde(${attrs.join(", ")})] ` : ""}${ident}: ${rt},`);
    });
    const doc = `    /// \`${a.name}\`${a.kind === "method" ? " (a store method)" : " (an exported function that writes a store)"}.${skipped.length ? ` The callback parameters ${skipped.map((s) => `\`${s}\``).join(", ")} are not in the JSON.` : ""}`;
    variants.push(`${doc}\n    ${dupName ? "#[serde(skip)] // same name as an earlier action" : `#[serde(rename = ${JSON.stringify(a.name)})]`}\n    ${id}${fields.length ? ` {\n${fields.join("\n")}\n    }` : ""},`);
  }
  return variants;
}

const modules = new Map();
for (const folder of folders) {
  const stateData = [...data.get(folder)].sort(([a], [b]) => a.localeCompare(b));
  const stateFieldsText = [];
  for (const [name, { type, optional }] of stateData) {
    let rt = rust(type, "State" + pascal(name), folder);
    if (optional && !rt.startsWith("Option<")) rt = `Option<${rt}>`;
    const ident = fieldIdent(name);
    const rename = fieldRename(name, ident);
    const attrs = [];
    if (rename) attrs.push(`rename = ${JSON.stringify(rename)}`);
    if (optional) attrs.push(`default`, `skip_serializing_if = "Option::is_none"`);
    stateFieldsText.push(`    ${attrs.length ? `#[serde(${attrs.join(", ")})] ` : ""}pub ${ident}: ${rt},`);
  }
  modules.set(folder, { stateFieldsText, variants: actionVariants(folder) });
}

// ---- write out
const HEADER = (rel) => `// Ported from ardia ${rel} at 551d3c13c.\n// Copyright 2026 Mantra Patel, Apache-2.0.\n// This file is a Rust port of that file, with changes.\n\n`;
const ORIGINAL = "// Ria original, no Ardia source.\n// Generated by rust/tools/ts-sig/gen.mjs from the TypeScript stores of Ardia at 551d3c13c.\n\n";

const out = new Map();
for (const folder of folders) {
  const { stateFieldsText, variants } = modules.get(folder);
  const local = [...decls.values()].filter((d) => d.folder === folder).map((d) => d.code);
  const slice = SLICE[folder];
  const body = [];
  body.push(`//! State module \`${folder}\` (\`apps/browser/src/state/${folder}\`).\n//!\n//! \`State\` has the data fields of the TypeScript store, \`Action\` has one variant for each exported store action, and\n//! \`apply\` is the reducer. Slice ${slice} writes the body of \`apply\` and proves it with recorded cases.`);
  const text0 = local.join("\n\n") + stateFieldsText.join("\n") + variants.join("\n");
  const uses = [];
  if (/\b(Serialize|Deserialize)\b/.test(text0) || true) uses.push("use serde::{Deserialize, Serialize};");
  uses.push("");
  uses.push("#[allow(unused_imports)]\nuse crate::kernel::Number;\nuse crate::kernel::{AppState, Effects};");
  body.push(uses.join("\n"));
  if (local.length) body.push(local.join("\n\n"));
  body.push(`/// The data fields of the TypeScript store.\n#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]\npub struct State {${stateFieldsText.length ? "\n" + stateFieldsText.join("\n") + "\n" : ""}}`);
  body.push(`/// One variant for each exported store action.\n#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n#[serde(tag = "type")]\npub enum Action {${variants.length ? "\n" + variants.join("\n") + "\n" : ""}}`);
  body.push(`/// Applies \`action\` to \`app\`. Slice ${slice} writes the body.\npub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {\n    let _ = (app, action, fx);\n    todo!("slice ${slice}")\n}`);
  out.set(`${moduleName(folder)}/mod.rs`, HEADER(`apps/browser/src/state/${folder}/`) + body.join("\n\n") + "\n");
}

const order = folders.map((f) => ({ folder: f, mod: moduleName(f), variant: pascal(f) }));
out.set(
  "lib.rs",
  `${ORIGINAL}//! Ria state: one module for each of the 41 state folders of Ardia, and the store kernel (plan section 6.5).\n#![allow(clippy::large_enum_variant, clippy::derivable_impls)] // shortcut: generated skeleton types; the slice of each folder may tidy them\n\n//! A state crate has no Rune type. Slice 2.4 writes the skeleton; slices 2.5 to 2.45 fill the bodies of \`apply\`.\n\npub mod kernel;\npub mod platform;\n\n${order.map((m) => `pub mod ${m.mod};`).join("\n")}\n`,
);
out.set(
  "kernel/modules.rs",
  `${ORIGINAL}use serde::{Deserialize, Serialize};

use super::Module;
use crate::{${order.map((m) => m.mod).join(", ")}};

/// The one state value of the process: one private field and one change counter for each state module.
#[derive(Debug, Default)]
pub struct AppState {
${order.map((m) => `    ${m.mod}: ${m.mod}::State,\n    ${m.mod}_changes: u64,`).join("\n")}
}

/// One variant for each module \`Action\`. The JSON is \`{"<folder>": <action>}\`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppAction {
${order.map((m) => `    #[serde(rename = ${JSON.stringify(m.folder)})]\n    ${m.variant}(${m.mod}::Action),`).join("\n")}
}

${order
  .map(
    (m) => `impl Module for ${m.mod}::State {
    fn slot(app: &AppState) -> &Self {
        &app.${m.mod}
    }
    fn slot_mut(app: &mut AppState) -> &mut Self {
        &mut app.${m.mod}
    }
    fn changes(app: &AppState) -> u64 {
        app.${m.mod}_changes
    }
    fn count_change(app: &mut AppState) {
        app.${m.mod}_changes += 1;
    }
}`,
  )
  .join("\n\n")}
`,
);

const format = (text) => {
  const r = spawnSync("rustfmt", ["--edition", "2024", "--emit", "stdout"], { input: text, encoding: "utf8" });
  if (r.status !== 0) throw new Error(`rustfmt failed: ${r.stderr.slice(0, 2000)}`);
  return r.stdout.replace(/^.*<stdin>:\n\n?/, "");
};

let changed = false;
for (const [rel, text0] of out) {
  const text = format(text0);
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
const actionCount = [...modules.values()].reduce((n, m) => n + m.variants.length, 0);
const fieldCount = [...modules.values()].reduce((n, m) => n + m.stateFieldsText.length, 0);
console.log(`${folders.length} modules, ${stores.length} stores, ${fieldCount} state fields, ${actionCount} actions, ${decls.size} types, ${out.size} files${check ? (changed ? ", out of date" : ", up to date") : ""}`);
for (const w of warnings) console.error(`warning: ${w}`);
if (check && changed) process.exit(1);

import { BUILT_IN_ENGINES, allEngines } from "../../../../packages/core/src/engines.ts";
import { offeredSiteSearch } from "../../../../packages/core/src/siteSearch.ts";
const out = { engines: [], site: [], ext: [] };
const E = (fn, ...args) => out.engines.push({ fn, args });
const S = (fn, ...args) => out.site.push({ fn, args });
const X = (fn, ...args) => out.ext.push({ fn, args });
const custom = [
  { id: "c1", name: "Kagi", keyword: "kagi", url: "https://kagi.com/search?q=%s" },
  { id: "c2", name: "MDN", keyword: "MDN", url: "https://developer.mozilla.org/search?q=%s" },
  { id: "c3", name: "Bad", keyword: "bad", url: "ftp://x/%s" },
  { id: "c4", name: "NoPlaceholder", keyword: "np", url: "https://example.com/search" },
  { id: 5, name: "x", keyword: "y", url: "https://a.b/%s" },
];
const extEngines = [
  { profile: "p", extensionId: "ext1", extensionName: "Ecosia Ext", name: "Ecosia", keyword: "eco", url: "https://www.ecosia.org/search?q=%s", suggestUrl: "https://ac.ecosia.org/?q=%s", isDefault: false },
  { profile: "p", extensionId: "ext1", extensionName: "Ecosia Ext", name: "Ecosia 2", keyword: "eco2", url: "https://www.ecosia.org/search?q=%s", isDefault: true },
  { profile: "p", extensionId: "ext2", extensionName: "Startpage", name: "Startpage", keyword: "sp", url: "https://www.startpage.com/do/search?q=%s", suggestUrl: "", isDefault: true },
];
for (const c of custom) { E("customEngineFrom", c); E("validSearchTemplate", c.url); }
E("customEnginesFrom", custom); E("customEnginesFrom", [null, 1, "x", custom[0], [custom[1]]]);
E("validSearchTemplate", "HTTPS://Example.com/?q=%s"); E("validSearchTemplate", "https:// /?q=%s"); E("validSearchTemplate", "https://a/?q=%S");
E("allEngines", custom, extEngines); E("allEngines", [], extEngines.slice(1)); E("allEngines", custom);
E("controllingExtensionEngine", extEngines); E("controllingExtensionEngine", extEngines.slice(0, 1)); E("controllingExtensionEngine", []);
const all = allEngines(custom, extEngines);
for (const id of ["google", "bing", "c1", "extension:ext1", "missing", ""]) E("engineById", all, id);
for (const e of all) { E("searchUrl", e, "  rust & cargo?  "); E("suggestRequestUrl", e, "héllo wörld"); E("suggestRequestUrl", e, "   "); E("engineHost", e); }
const mapsReply = ["maps", ["maps google", "https://maps.google.com/", "maps directions", "2+2"], ["", "Google Maps", "", ""], [], { "google:suggesttype": ["QUERY", "NAVIGATION", "QUERY", "CALCULATOR"] }];
const odd = ["x", ["javascript:alert(1)", "maps.google.com"], [], [], { "google:suggesttype": ["NAVIGATION", "NAVIGATION"] }];
const more = [
  mapsReply, odd,
  ["q", [" padded ", "", "   ", 7, null, "https://x.com/a b", "HTTP://Example.ORG"], [1, " Title ", null, null, null, "  X  ", 5], [], { "google:suggesttype": ["QUERY", "QUERY", "QUERY", "QUERY", "QUERY", "PERSONALIZED_NAVIGATION", "NAVIGATION"] }],
  ["q", ["a", "b"], "not array", [], { "google:suggesttype": "NAVIGATION" }],
  ["q", ["a", "b"], [], [], null], ["q", ["a"], [], [], { "google:suggesttype": [["NAVIGATION"], 1] }],
  ["q", ["a"]], ["q", "a"], {}, null, "x",
];
for (const b of more) { E("parseRemoteSuggestions", b); E("parseSuggestions", b); }
for (const t of [JSON.stringify(mapsReply), ")]}'\n" + JSON.stringify(odd), ")]}'  []", '["a",["b"]]garbage', '{"a":1,"a":2}', '[1e400, -0, 0.1, "\u00e9\ud83d\ude00"]']) E("suggestReplyJson", t);
const inputs = [
  { name: " Kagi ", keyword: " KAGI ", url: " https://kagi.com/search?q=%s " },
  { name: "", keyword: "k", url: "https://k/%s" },
  { name: "n", keyword: "  ", url: "https://k/%s" },
  { name: "n", keyword: "a b", url: "https://k/%s" },
  { name: "n", keyword: "k", url: "ftp://k/%s" },
  { name: "n", keyword: "k", url: "https://k/search" },
  { name: "n", keyword: "Google.com", url: "https://k/%s" },
  { name: "n", keyword: "İstanbul", url: "https://k/%s" },
];
for (const i of inputs) { E("validateEngine", i, all); E("validateEngine", i, all, "google"); E("makeCustomEngine", i, "id-1"); }
E("validateEngine", inputs[0], [null]); E("validateEngine", inputs[0], null); E("validateEngine", inputs[6], [null]);
// siteSearch
const providers = [{ id: "gh", title: "GitHub repos", keyword: "GH", host: "github.com", url: "https://github.com/search?q=%s" }, { id: "notes", title: "Notes", keyword: "notes" }];
const sources = { engines: all, hosts: ["news.example.com", "localhost"], providers };
const words = ["google", "goo", "Google.com", "https://www.bing.com/", "kagi", "KAGI", "maps", "Maps", "map", "x.com", "x", "gh", "notes", "en.wikipedia.org", "wikipedia", "news.example.com", "localhost", "hello world", "", "  youtube  ", "duckduckgo", "github", "mdn", "drive", "translate"];
for (const w of words) { S("findScope", w, sources); S("findScope", w, { engines: BUILT_IN_ENGINES }); S("providerScope", w, providers); }
S("findScope", "news.example.com", { engines: [], hosts: "a news.example.com b" }); S("findScope", "news.example.com", { engines: [], hosts: null }); S("findScope", "x.com", { engines: [], sites: null });
S("providerScope", "gh", [{ id: "a", title: "A", keyword: null }]); S("providerScope", "gh", [{ id: "a", title: "A", keyword: 5 }]); S("providerScope", "gh", "providers");
for (const [t, c] of [["goo", "gle.com"], ["google", ".com"], ["x.com", ""], ["exampl", "e.org"], ["gh", "x"], ["hello world", ""], ["", ""]]) S("offeredSiteSearch", t, c, sources);
const scopes = [offeredSiteSearch("google", "", sources), offeredSiteSearch("x.com", "", sources), offeredSiteSearch("gh", "", sources), offeredSiteSearch("notes", "", sources), { kind: "history", name: "news.example.com", host: "news.example.com", url: "" }];
for (const s of scopes) { S("scopedSearchUrl", s, "  rust lang ", BUILT_IN_ENGINES[1]); S("asksForBackspace", s); }
S("scopedSearchUrl", { kind: "provider", url: 5 }, "q", BUILT_IN_ENGINES[0]); S("scopedSearchUrl", { kind: "site", url: 0 }, "q", BUILT_IN_ENGINES[0]); S("scopedSearchUrl", { kind: "history", host: 42 }, "q", { url: "https://s/?q=%s" });
const bar = (typed, completion = "", scope = null, scopeFrom = "") => ({ typed, completion, scope, offered: scope ? null : offeredSiteSearch(typed, completion, sources), scopeFrom });
for (const st of [bar("google", ".com"), bar("goo", "gle.com"), bar("x.com"), bar("maps"), bar("exampl", "e.org"), bar("hello world"), bar(""), bar("", "", scopes[0], "google.com"), bar("", "", scopes[1], "x.com ")])
  for (const m of [{}, { shiftKey: true }, { ctrlKey: true }, { altKey: true }, { metaKey: 1 }]) for (const k of ["Tab", "Backspace", "Enter"]) S("siteSearchKey", k, m, st);
for (const [a, b] of [["x.com", "x.com "], ["x.com ", "x.com  "], ["x.com", "x.coma"], ["x.com", " x.com"], ["", " "], ["gh", "gh "], ["a\u00a0", "a\u00a0 "], ["  ", "   "]]) S("keywordBySpace", a, b);
const ext = (id, title, action = true, options = true) => ({ id, title, icon: "data:image/png;base64," + id, action, options });
const EX = [ext("ublock", "uBlock Origin"), ext("onepassword", "1Password – Password Manager", true, false), ext("darkreader", "Dark Reader"), ext("nopopup", "Video Speed Controller", false, true), { id: "noicon", title: "Tab Session Manager", icon: null, action: true, options: true }];
for (const q of ["ublock", "password", "reader", "video speed", "ub", "zzq", "Dark Reader", "dark rea", "  session  ", "tab", "manager", "u", ""]) for (const l of [undefined, 1, 3, -1, 0]) X("extensionSuggestions", q, EX, l);
X("extensionSuggestions", "ublock", []); X("extensionSuggestions", "ab", [ext("ab", "AB")]); X("extensionSuggestions", "ab", [ext("ab", "AB", false, false)]);
const fs = await import("node:fs");
const enc = (v) => JSON.stringify(v, (k, x) => x === undefined ? { "$undefined": true } : x);
for (const [k, f] of [["engines", "engines"], ["site", "site-search"], ["ext", "extension-rows"]]) fs.writeFileSync("./" + f + ".jsonl", out[k].map(enc).join("\n") + "\n");
console.log(out.engines.length, out.site.length, out.ext.length);

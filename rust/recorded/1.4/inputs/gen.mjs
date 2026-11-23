// Generates the --inputs JSONL for bangs-rs and bangs-harness-rs from the real bang list sample.
// Usage: node gen.mjs (from this folder)  -> ./bangs-inputs.jsonl, ./harness-inputs.jsonl
import { writeFileSync } from "node:fs";
const { BANG_LIST, BANG_LIST_UNICODE, BANG_LIST_ETAG } = await import("../../../../packages/core/src/bangList.ts");

let seed = 0x5eed1234;
const rand = () => ((seed = (Math.imul(seed, 1103515245) + 12345) >>> 0) / 2 ** 32);
const LIST = BANG_LIST.split("\n");
const UNI = BANG_LIST_UNICODE.split("\n");
const hasTrigger = (t) => (l) => l.split("\t")[0].split(" ").includes(t);
const REQUIRED = ["gh", "w", "yt"].map((t) => LIST.find(hasTrigger(t)) ?? UNI.find(hasTrigger(t)));
if (REQUIRED.some((l) => !l)) throw new Error("sample lacks gh/w/yt");

/** `n` real slim lines, the gh/w/yt lines among them, in list order. */
function realLines(n, from = LIST) {
  const picked = new Set(REQUIRED.filter((l) => from.includes(l)));
  while (picked.size < n) picked.add(from[Math.floor(rand() * from.length)]);
  return from.filter((l) => picked.has(l)).concat(REQUIRED.filter((l) => !from.includes(l)));
}
const expand = (stored) => (/^https?:\/\//i.test(stored) ? stored : `https://${stored}`).split("{s}").join("{searchTerms}");
const toEntry = (line) => {
  const [triggers, s, stored] = line.split("\t");
  const e = { s, ts: triggers.split(" "), u: expand(stored) };
  if (rand() < 0.2) e.sc = "Online Services";
  if (rand() < 0.1) e.f = 1;
  return e;
};
function heliumFile(entries) {
  const body = JSON.stringify(entries, null, 1).replace(/"\n(\s*)([\]}])/g, '",\n$1$2');
  return `// Generated at 2026-10-14T11:11:02.825Z\n// MIT License\n//\n\n${body}\n`;
}

const slim1100 = realLines(1100).join("\n");
const slim1000 = realLines(1000).join("\n");
const uniSlim = UNI.join("\n");
const fixtureEntries = [
  { s: "GitHub", ts: ["gh", "git", "github"], u: "https://github.com/search?q={searchTerms}" },
  { s: "GitHub Repo", ts: ["ghr"], f: 1, u: "https://github.com/{searchTerms}" },
  { s: "Wikipedia", ts: ["w", "wiki"], u: "https://wikipedia.org/w/index.php?search={searchTerms}" },
  { s: "YouTube", ts: ["yt", "ют"], u: "https://www.youtube.com/results?search_query={searchTerms}" },
  { s: "Zig", ts: ["zig"], u: "https://ziglang.org/documentation/master/std/#?{searchTerms}" },
  { s: "Gmail", ts: ["gmail"], u: "https://mail.google.com/mail/u/0/#search/{searchTerms}" },
  { s: "T3 Chat", ts: ["t3"], u: "https://www.t3.chat/new?q={searchTerms}", sc: "ai" },
  { s: "Not a web page", ts: ["ftp"], u: "ftp://example.com/{searchTerms}" },
  { s: "Read the Docs", ts: ["rtfd"], u: "https://{searchTerms}.rtfd.io/" },
  { s: "hypestat", ts: ["hypestat"], u: "https://www.{searchTerms}.hypestat.com/" },
  { s: "No slash", ts: ["noslash"], u: "https://example.com{searchTerms}" },
  { s: "Port", ts: ["port"], u: "https://example.com:{searchTerms}/" },
  { s: "User", ts: ["user"], u: "https://{searchTerms}@example.com/" },
  { s: "Taken", ts: ["gh"], u: "https://taken.example/?q={searchTerms}" },
  { s: "  Spaced\u3000\u00a0 name\t\n ", ts: [" SpAcEd ", "spaced", "two words", "bang!", "", 7, null, "\u0130x", "\u00df"], u: "HTTPS://Spaced.example/?q={searchTerms}" },
  { s: "Plain http", ts: ["plain"], u: "http://plain.example/{searchTerms}/x?y={searchTerms}" },
  { s: "Stored placeholder", ts: ["stored"], u: "https://a.example/{s}" },
  { s: "\u0085nel", ts: ["nel"], u: "https://nel.example/?q={searchTerms}" },
  { s: "", ts: ["empty"], u: "https://e.example/" },
  { s: " \t ", ts: ["blank"], u: "https://e.example/" },
  { s: 5, ts: ["num"], u: "https://e.example/" },
  { s: "No ts", ts: "x", u: "https://e.example/" },
  { s: "Ctl", ts: ["ctl"], u: "https://ctl.example/\u0001{searchTerms}" },
  ["array", "entry"],
  null,
  42,
  "string entry",
  { s: "No u", ts: ["nou"] },
  { s: "Hosts", ts: ["hosts"], u: "https://h.example/?q={searchTerms}", __proto__x: 1 },
];
const fixtureFile = heliumFile([...fixtureEntries, ...realLines(1000, LIST.slice(200)).map(toEntry)]);

const out = [];
const add = (fn, ...args) => out.push({ fn, args });

// ---- parseLooseJson ----
const jsonCases = [
  '// head\n[\n {"u": "https://a//b", /* c */ "ts": ["x",],},\n // tail\n]\n',
  '["a\\\\", "b,]", "c\\"//"]',
  "", " ", "{", "[", "[1", "[1,", '{"a"', '{"a":', '{"a":1', '{"a":1,', '{"a" 1}', "{1}", "{,}", '{"a":1,}', "[1,]", "[1,,]", "[,]", "[1 , ]", "[1,\n]", "[1,/* c */]", "[1,// c\n]", "[1, /* a */ /* b */ ]", "[1,\u3000]", "[1,\ufeff]", "[1,\u2028]", "{\"a\":1,\u00a0}",
  "tru", "trx", "t1", 't"', "nul", "fals", "falsey", "nullx", "true false", "-", "-x", "-a", "01", "-01", "1.", "1.x", "1e", "1e+", "1ex", "1 2", "-0", "[-0]", "0", "-0.0e0", "1E5", "1e-7", "1.5e+300", "1e400", "-1e400", "123456789012345678901234567890", "0.1", "9007199254740993", "4.9e-324", "1e-400", "[0,1,-1,10,1.25]", "00", "0x10", "+1", ".5", "1.e5",
  '"abc', '"a\\', '"a\\x"', '"a\\u12"', '"a\\u12', '"\u0001"', '"\\\u0101"', '"\\é"', '"\\😀"', '"\\u00e9\\n\\t\\b\\f\\r\\/\\\\\\""', '"\\uD83D\\uDE00"', '"\\ud83d\\ude00"', '"\\uDE00\\uD83D"', '"\\ud800"', '"\\udc00x"', '"a\\ud800\\u0041"', '"\\ud800\\ud800\\udc00"', '{"\\ud800":1,"\\udc00":2}', '"\u007f\u0080\u2028"', '"tab\there"', '"new\nline"',
  "undefined", "NaN", "Infinity", "[object Object]", "-Infinity", "abcdefghijklmnopqrstuvwxyz", "abcdefghijklmnopqrst", "abcdefghijklmnopqrstu", "[1,2,3,4,5,6,7,8,9,10,11,x]", "[1,2,3,4,5,6,7,8,9,10,11,12,13,14,x]", "[1,2,3,4,5,6,7,8,9,10,11,12,13,14,x,1,2,3,4,5,6,7,8,9]", "\n\r\n\r[\nx", "]", "😀", "[1,2,3,4,5,6,7,8,9,😀 1,2,3,4,5,6,7,8,9]", "[\"日本語のテキスト\", 1, 2, 3, x, \"é\"]",
  "\r\n[1 x]", "\r\r[1 x]", "[\r\n\n1\r\n x]", '{"a":1\r\n\r\n}}', "[1]]", "[1]x", '{"a":1}}', "[1e5x]", '["a" "b"]', '{"a":"b" "c":1}', '{"a" : 1, "b"}', "\t\n\r 1 \t\n\r", "\u000b1", "\ufeff1", "1\u3000", "\u00a0 1", "[1,,2]", ":", ",", "}", "/", "/ 1", "1 /", "1 // c", "1 /* c", "/* c */ 1", "/* c * / 1", "// only", "[1, 2 // c\n, 3]", "[1, \"//\", '/*']",
  '{"0":1,"b":2,"1":3,"b":4}', '{"__proto__":1,"a":{"__proto__":[2]}}', '{"4294967295":1,"4294967294":2,"01":3,"-1":4,"1":5}', '{"a":1,"a":2,"a":3}', '{"b":1,"a":2,"b":3,"c":4,"a":5}', '{"":0,"":1}', "[[],{},[[]],{\"a\":{}}]", "[true,false,null]",
  '"unterminated // comment', '"a" // "b"', '"a\\\\" , ]', '["x\\\\\\"y", ]', '[",", ",]"]', '{"s":"x",/* , */}', "[1,/**/]", "[1,/* unterminated", "[1,// unterminated", '"\\"', "[\"\\\\\\\\\"]",
  "[" + "[".repeat(4999) + "]".repeat(5000), "{\"a\":".repeat(3000) + "1" + "}".repeat(3000), "[".repeat(3000) + "]".repeat(2999),
  heliumFile(realLines(40).map(toEntry)),
];
for (const t of jsonCases) add("parseLooseJson", t);
add("parseLooseJson", heliumFile(realLines(1000).map(toEntry)));

// ---- isValidBangTemplate ----
for (const t of ["https://github.com/search?q={searchTerms}", "HTTPS://GitHub.com/x", "http://a.example", "http://a.example:8080", "http://a.example:8080/x", "https://a.example:", "https://a.example:x/", "https://a_b.example/", "https://a.example/ x", "https://a.example/\u3000", "https://a.example/\u0085", "https://a.example/x\u2028", "https://é.example/", "ftp://a.example/", "https://a.example?{searchTerms}", "https://a.example#x\ny", "https://", "https:///x", "https://a.example\\x", "https://a.example/\u0001", "hTtPs://a.example#", "https:/a.example", " https://a.example/", "https://a.example/ ", "https://{searchTerms}.rtfd.io/", "https://a.example:{searchTerms}/", "https://a-b.c.example.:1/?", "https://..../", "https://a.example/😀", "httpss://a.example/", "https://a.example:99999999999999/x", ""]) add("isValidBangTemplate", t);

// ---- staysOnSite ----
const gh = "https://github.com/{searchTerms}";
for (const u of ["https://github.com/search?q=a", "https://GitHub.com/a", "https://github.com\\@evil.example/", "https://github.com@evil.example/", "https://evil.example/github.com/", "http://github.com/a", "https://github.com:444/a", "https://github.com.evil.example/", "https://github.com/a\tb", "https://github.com/a\nb", "https://github.com/a\u007f", "https://github.com/a\u3000", "https://github.com/a\u0085", "HTTPS://GITHUB.COM", "https://github.com", "github.com/a", "1https://github.com/", "https:/github.com/", "https://github.com#x", "https://github.com?x", "https://GİTHUB.com/", "https://ΣΑΣ.example/"]) add("staysOnSite", u, gh);
add("staysOnSite", "https://github.com/search?q=a", "https://github.com/search?q={searchTerms}");
add("staysOnSite", "https://x/", "https:///x");
add("staysOnSite", "https:///x", "https:///x");
add("staysOnSite", "a+b.c-d://h/", "A+B.C-D://H/");
add("staysOnSite", "https://ΣΑΣ.example/", "https://σας.example/x");
add("staysOnSite", "https://x.example/", "nope");

// ---- slimBangs ----
for (const b of ["", "<html>oops</html>", "{}", "[]", '[{"s":"A","ts":["a"],"u":"https://a.example/{searchTerms}"}]', heliumFile([]).slice(0, 5000), "[1,2,", "null"]) add("slimBangs", b);
add("slimBangs", fixtureFile);
add("slimBangs", heliumFile(realLines(999, LIST.slice(100)).map(toEntry)));
add("slimBangs", heliumFile(realLines(1000, LIST.slice(100)).map(toEntry)));
add("slimBangs", heliumFile([...realLines(1000, LIST.slice(100)).map(toEntry), { s: "Lone \\ud800 kept", ts: ["lone"], u: "https://l.example/" }]).replace("\\\\ud800", "\\ud800"));
add("slimBangs", "[" + "[".repeat(200000) + "]".repeat(200001));
add("slimBangs", JSON.stringify(realLines(1000, LIST.slice(300)).map(toEntry)));

// ---- bangDocument ----
add("bangDocument", '"v2"', "gh\tGitHub\tgithub.com/search?q={s}");
add("bangDocument", null, "");
add("bangDocument", "", "x");
add("bangDocument", BANG_LIST_ETAG, slim1000);

// ---- escapeSearchTerms / fillBangTemplate ----
for (const [t, q] of [["é ü", false], ["é ü", true], ["a+b & c/d #x 100%", true], ["a+b & c/d #x 100%", false], ["it's (ok)*!~", true], ["it's (ok)*!~", false], ["\"#%:<>?[\\]^`{|}", false], ["\"#%:<>?[\\]^`{|}", true], ["\u0000\u001f\u007f \t\n", false], ["日本 語 😀", false], ["%20", true], ["", true], ["", false], ["a/b;c=d@e$f,g&h", false], ["\u2028", false]]) add("escapeSearchTerms", t, q);
for (const [tpl, terms] of [["https://a.example/{searchTerms}/x?q={searchTerms}", "p q"], ["https://github.com/search?q={searchTerms}", "react hooks"], ["https://github.com/{searchTerms}", "a b?c#d%e"], ["https://mail.google.com/mail/u/0/#search/{searchTerms}", "from:me"], ["https://ziglang.org/documentation/master/std/#?{searchTerms}", "std.mem a b"], ["https://x.example/", "q"], ["https://x.example/?a=1&q={searchtrms}", "q"], ["{searchTerms}?", "a b"], ["?{searchTerms}", "a b"], ["https://x.example/{searchTerms}{searchTerms}", "é"], ["https://x.example/?q={searchTerms}&r={searchTerms}", "1+1=2"], ["https://x.example/{searchTerms", "x"], ["", ""]]) add("fillBangTemplate", tpl, terms);

writeFileSync("./bangs-inputs.jsonl", out.map((c) => JSON.stringify(c)).join("\n") + "\n");

// ---- harness ----
const h = [];
const hadd = (fn, ...args) => h.push({ fn, args });
const triggers = ["gh", "GH", "gh", "w", "yt", "git", "github", "ют", "ЮТ", "g", "п", "amazon", "nope", "", "!gh", "gh ", "ghr", "rtfd", "hypestat", "hs", "us", "İ", "ß", "x"];
const stored = "rtfd\tRead the Docs\t{s}.rtfd.io/\nhs\thypestat\twww.{s}.hypestat.com/\nus\tUser\t{s}@example.com/";
hadd("bangTableView", stored, ["rtfd", "hs", "us"]);
hadd("bangTableView", slim1100, triggers);
hadd("bangTableView", uniSlim, triggers);
hadd("bangTableView", "", triggers);
hadd("bangTableView", "\n\n\n", ["", "\n"]);
hadd("bangTableView", "a b  c \tA\ta.example/{s}\n\tno trigger\tx.example/\nno tab\nd\t\td.example/\ne\tE\n f\tF\tf.example/{s}/x?q={s}\nB\tUpper\tb.example\na\tSecond a\tsecond.example/\ng\tG\thttp://g.example/{s}\nh\tH\tHTTPS://H.example\ni\tI\tftp://i.example/\nk\tK\tk.example/{s} x\nl\tL\tl.example:{s}\nm\tM\tm.example:8080\n\u0130\tDotted\tdot.example\ni\u0307\tLower dotted\tdot2.example/\nσ\tSigma\ts.example/\nq\tQ\tq.example/\tmore\tfields\nr\tR\tr.example/\r\nend\tEnd\tend.example/", ["a", "A", "b", "c", "", "d", "e", "f", "B", "g", "h", "i", "k", "l", "m", "İ", "i̇", "Σ", "σ", "q", "r", "end", "no", "a", "f"]);
hadd("bangTableView", "x\tX\tx.example/{s}\n", ["x", "x", "X"]);
const fixture = [...fixtureEntries.filter((e) => e && typeof e.s === "string" && e.s.trim() && Array.isArray(e.ts) && typeof e.u === "string" && /^https?:\/\/[a-z0-9.-]+(?::\d+)?(?:[/?#]\S*)?$/i.test(e.u) && !e.u.includes("{s}")).map((e) => `${e.ts.filter((t) => typeof t === "string" && t.trim() && !/[\s!]/.test(t.trim())).map((t) => t.trim().toLowerCase()).join(" ")}\t${e.s.replace(/\s+/g, " ").trim()}\t${e.u.replace(/^https:\/\//i, "").split("{searchTerms}").join("{s}")}`).filter((l) => !l.startsWith("\t")), ...realLines(1000, LIST.slice(400))].join("\n");
const fixtureNoDup = fixture.split("\n").filter((l) => !l.endsWith("taken.example/?q={s}")).join("\n");
const ck = ["gh", "w", "yt", "GH", "git", "ют", "zig", "nope", "t3", "rtfd", "plain", "ctl"];
hadd("checkedBangTableView", slim1100, ck);
hadd("checkedBangTableView", slim1000, ck);
hadd("checkedBangTableView", fixtureNoDup, ck);
hadd("checkedBangTableView", realLines(999).join("\n"), ck);
hadd("checkedBangTableView", Array.from({ length: 1200 }, (_, i) => `bad${i}\tBroken\thttps://bad host/`).join("\n"), ck);
hadd("checkedBangTableView", `${slim1000}\nbad\tBroken\thttps://bad host/`, ck);
hadd("checkedBangTableView", `${slim1000}\nbad\tBroken`, ck);
hadd("checkedBangTableView", `${slim1000}\nbad\tBroken\tx.example/{s}\textra`, ck);
hadd("checkedBangTableView", `${slim1000}\n`, ck);
hadd("checkedBangTableView", `${slim1000}\n\tNo trigger\tx.example/`, ck);
hadd("checkedBangTableView", `${slim1000}\nb  c\tDouble space\tx.example/`, ck);
hadd("checkedBangTableView", `${slim1000}\nb!\tBang\tx.example/`, ck);
hadd("checkedBangTableView", `${slim1000}\nb\u3000c\tIdeographic space\tx.example/`, ck);
hadd("checkedBangTableView", `${slim1000}\nb\u0085c\tNEL is no space\tx.example/`, ck);
hadd("checkedBangTableView", `${slim1000}\nb\t\tx.example/`, ck);
hadd("checkedBangTableView", `${slim1000}\nb\tB\t`, ck);
hadd("checkedBangTableView", `${slim1000}\nb\tB\tftp://x.example/`, ck);
hadd("checkedBangTableView", `${slim1000}\nb\tB\tHTTP://x.example/{s}`, ck);
hadd("checkedBangTableView", `${slim1000}\r`, ck);
hadd("checkedBangTableView", slim1000.split("\n").filter((l) => !hasTrigger("yt")(l)).join("\n") + "\nfill\tFill\tfill.example/", ck);
hadd("checkedBangTableView", slim1000.split("\n").filter((l) => !hasTrigger("gh")(l)).join("\n") + "\nfill\tFill\tfill.example/\ngh\tBroken gh\tgh.example:{s}/", ck);
hadd("checkedBangTableView", slim1000.split("\n").slice(0, 900).join("\n"), ck);
// size limit: one valid list just over and one exactly at 4_000_000 UTF-16 units (more bytes than units).
const atLimit = (base, units, char = "x") => {
  const head = `${base}\n`;
  const tail = "\tBig\tbig.example/{s}";
  const fill = units - head.length - tail.length;
  return `${head}${char.repeat(100000)}${"x".repeat(fill - 100000)}${tail}`;
};
hadd("checkedBangTableView", atLimit(slim1000, 4_000_001), ck);
hadd("checkedBangTableView", atLimit(slim1000, 4_000_000, "é"), ck);

const doc = (etag, slim) => `ardia-bangs 1\t${etag ?? ""}\n${slim}`;
const rk = ["gh", "w", "yt", "ют", "nope", "GitHub"];
for (const d of [doc('"v2"', fixtureNoDup), doc(null, slim1000), doc("", slim1000), doc(BANG_LIST_ETAG, slim1100), null, "", fixtureNoDup, doc('"v2"', fixtureNoDup).slice(0, 2000), `ardia-bangs 2\t"v2"\n${slim1000}`, `ardia-bangs 1\t"v2"\tx\n${slim1000}`, `ardia-bangs 1\n${slim1000}`, `ardia-bangs 1 \t"v2"\n${slim1000}`, "ardia-bangs 1\t\"v2\"", `ardia-bangs 1\t"v2"\r\n${slim1000}`, doc("\u00e9tag", slim1000)]) hadd("readBangDocumentView", d, rk);
// size limit of the document: an over-long ETag pushes a valid list past 4_001_024 units, and exactly at it.
const bigSlim = atLimit(slim1000, 3_999_000, "é");
hadd("readBangDocumentView", doc("e".repeat(4_001_025 - 15 - bigSlim.length), bigSlim), rk);
hadd("readBangDocumentView", doc("e".repeat(4_001_024 - 15 - bigSlim.length), bigSlim), rk);

hadd("joinBangTablesView", [slim1100, uniSlim], ["gh", "g", "п", "ют", "w", "nope", "yt", "GH"]);
hadd("joinBangTablesView", [], ["gh"]);
hadd("joinBangTablesView", ["gh\tFirst\tfirst.example/\nx\tX\tx.example/", "gh\tSecond\tsecond.example/\nbad\t\tx.example/", "bad\tGood\tgood.example/"], ["gh", "x", "bad", "GH"]);
hadd("joinBangTablesView", [stored, stored], ["rtfd"]);

const texts = ["!gh react", "react !w", "!GH react hooks", "  react   hooks   !Wiki ", "!ют cats", "!yt", "  !W  ", "!nope react", "react !nope", "hello!", "wow ! nice", "!", "! gh react", "hi!gh", "say !gh in the middle", "example.com/!gh", "?!gh react", "a!w", "!gh react !w", "!nope react !w", "!gh a+b & c/d #x 100%", "!gh 日本 語", "!gh it's (ok)*!~", "!zig std.mem a b", "!ghr facebook/react", "!ghr a b?c#d%e", "!gmail from:me", "!rtfd evil.example/", "!hypestat @evil.example/", "!rtfd", "!noslash @evil.example", "?!gh react !w", "? react !w", "  ?react !w", "!copilot hi", "!t3 hi", "!ctl x", "!ctl", "!plain a/b c", "\u3000!gh\u00a0x\u2028y\u0085z", "!gh\n", "!!gh x", "x !!", "!gh😀", "!😀", "\ufeff?!gh", "!spaced x", "!x", "!X", "!GİT"];
hadd("findBangIn", texts, [fixture]);
hadd("findBangIn", texts, [slim1100, uniSlim]);
hadd("findBangIn", ["!gh react", "!x @evil.example", "!x"], null);
hadd("findBangIn", ["!gh react", ""], []);
hadd("findBangIn", ["!x @evil.example", "!x", "!x a", "!c q", "!c", "!h a b"], ["x\tX\tx.example{s}\nc\tCtl\tc.example/\u0001{s}\nh\tH\th.example/#{s}"]);

writeFileSync("./harness-inputs.jsonl", h.map((c) => JSON.stringify(c)).join("\n") + "\n");
console.log(out.length, h.length);

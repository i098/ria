"""Inputs for the suggestion-rows, candidate-pool, open-tabs and suggest-fetcher recordings."""

import json
import random
import sys

OUT = sys.argv[1]
rng = random.Random(1303)

INF = {"$num": "Infinity"}
NINF = {"$num": "-Infinity"}
UNDEF = {"$undefined": True}

HOSTS = [
    "a.test", "www.example.com", "EXAMPLE.org", "docs.google.com", "xn--bcher-kva.example",
    "bücher.example", "münchen.de", "例え.jp", "127.0.0.1", "[::1]", "user:pw@host.test",
    "host.test:8080", "sub.a.test", "github.com", "news.ycombinator.com", "ΣΑΣ.gr", "İstanbul.tr",
]
PATHS = ["", "/", "/a", "/a/", "/A/b", "/path?q=1", "/?q=x#frag", "#top", "/#", "/x#y/", "?a=b",
         "/ünï/çødé", "/emoji/😀", "/%20space", "/a//b/", "/a#x\ny", "/very/" + "long" * 20]
SCHEMES = ["https://", "http://", "HTTPS://", "Http://"]
ODD = ["", "about:blank", "chrome://settings", "file:///home/u/a.txt", "javascript:alert(1)",
       "ardia://settings", "ardia://history/", "ardia://newtab#x", "ARDIA://Settings/", "mailto:a@b.c",
       "data:text/html,hi", "a.test", "localhost:3000/x", " https://space.test ", "https://", "http://#",
       "ardia://", "ardia:///x"]
BAD = ["ardia://settings#a\nb", "ardia://history?q#\u2028x"]
TITLES = ["", "Home", "GitHub", "Docs – Google", "Straße", "İstanbul", "ΣΑΣ", "日本語のタイトル", "😀 Emoji",
          "a.b  c", "Mixed CASE Title", "  padded  ", "x" * 80, "new\nline", "tab\there", "ǅ", "K"]
FAVICONS = [None, None, "https://a.test/favicon.ico", "data:image/png;base64,AAA", ""]


def url(bad_rate=0.0):
    if rng.random() < bad_rate:
        return rng.choice(BAD)
    if rng.random() < 0.18:
        return rng.choice(ODD)
    return rng.choice(SCHEMES) + rng.choice(HOSTS) + rng.choice(PATHS)


def page_key(u):
    last = max((u.rfind(c) for c in "\n\r\u2028\u2029"), default=-1)
    i = u.find("#", last + 1)
    if i >= 0:
        u = u[:i]
    return u[:-1] if u.endswith("/") else u


def num(kind="visits"):
    r = rng.random()
    if kind == "visits":
        if r < 0.25:
            return 0
        if r < 0.7:
            return rng.randint(1, 50)
        if r < 0.8:
            return rng.randint(1000, 10**6)
        if r < 0.88:
            return round(rng.uniform(0, 20), 3)
        if r < 0.94:
            return -rng.randint(1, 5)
        return rng.choice([INF, 0.5, 1023, 1024, 2**53])
    if r < 0.1:
        return 0
    if r < 0.9:
        return 1_700_000_000_000 + rng.randint(-10**10, 10**10)
    return rng.choice([-1, 1.5, INF, NINF, 2**53])


def text(pool=TITLES):
    return rng.choice(pool)


def history_row(urls, bad=0.0):
    return {"url": rng.choice(urls) if urls and rng.random() < 0.5 else url(bad), "title": text(),
            "favicon": rng.choice(FAVICONS), "visits": num(), "lastVisit": num("time")}


def bookmark_row(urls, bad=0.0):
    return {"url": rng.choice(urls) if urls and rng.random() < 0.6 else url(bad), "title": text(),
            "favicon": rng.choice(FAVICONS)}


def tab_row(urls, ids, bad=0.0):
    row = {"id": rng.choice(ids), "url": rng.choice(urls) if urls and rng.random() < 0.6 else url(bad),
           "title": text()}
    r = rng.random()
    if r < 0.35:
        row["customTitle"] = rng.choice(["Mine", "", "Renamed Ü", "😀"])
    elif r < 0.45:
        row["customTitle"] = None
    row["favicon"] = rng.choice(FAVICONS)
    return row


def write(name, cases):
    with open(f"{OUT}/inputs-{name}.jsonl", "w", encoding="utf-8") as f:
        for c in cases:
            f.write(json.dumps(c, ensure_ascii=False) + "\n")
    print(name, len(cases))


def shared_urls(n=4):
    return [url() for _ in range(n)]


# ---- suggestionRows ------------------------------------------------------------------------------

def suggestion():
    k = rng.choice(["page", "page", "search", "calc", "action", "create", "extension", "provider"])
    if k == "page":
        s = {"kind": "page", "url": url(), "title": text(), "favicon": rng.choice(FAVICONS)}
        if rng.random() < 0.5:
            s["tabId"] = rng.choice(["1", "", "tab-2", "ü"])
        if rng.random() < 0.3:
            s["bookmarked"] = True
        if rng.random() < 0.3:
            s["visited"] = True
        return s
    if k == "search":
        s = {"kind": "search", "query": text(), "url": url(), "engine": rng.choice(["Google", "DuckDuckGo", ""])}
        if rng.random() < 0.3:
            s["suggested"] = True
        if rng.random() < 0.2:
            s["bang"] = True
        return s
    if k == "calc":
        return {"kind": "calc", "expression": rng.choice(["1+1", "2*3", "", "sqrt(2)"]),
                "value": rng.choice(["2", "6", "1.41421356237"]), "url": url()}
    if k == "action":
        s = {"kind": "action", "id": rng.choice(["new-tab", "settings", "", "ü"]), "title": text()}
        if rng.random() < 0.5:
            s["icon"] = rng.choice(["plus", "gear"])
        if rng.random() < 0.5:
            s["hint"] = rng.choice(["⌘T", "Ctrl+,"])
        return s
    if k == "create":
        return {"kind": "create", "id": rng.choice(["doc", "sheet", "slide"]), "title": text(),
                "url": url(), "site": rng.choice(["docs.google.com", "figma.com"])}
    if k == "extension":
        return {"kind": "extension", "id": rng.choice(["abc", "xyz", ""]), "title": text(),
                "icon": rng.choice([None, "data:x"]), "open": rng.choice(["action", "options"])}
    s = {"kind": "provider", "providerId": rng.choice(["p1", "p2", ""]), "provider": text(),
         "title": text(), "url": url(), "icon": rng.choice([None, "https://i.test/a.png"])}
    if rng.random() < 0.4:
        s["subtitle"] = text()
    return s


def page_row():
    row = {"url": url(), "title": text(), "favicon": rng.choice(FAVICONS), "visits": num(),
           "bookmarked": rng.random() < 0.4}
    if rng.random() < 0.5:
        row["tabId"] = rng.choice(["7", "", "tab"])
    return row


rows = []
for _ in range(400):
    args = [page_row()]
    r = rng.random()
    if r < 0.3:
        args.append(rng.choice(["t1", "", None]))
    elif r < 0.5:
        args.append(UNDEF)
    rows.append({"fn": "pageSuggestionOf", "args": args})
for _ in range(500):
    rows.append({"fn": "suggestionKeyOf", "args": [suggestion()]})
for _ in range(300):
    base = [suggestion() for _ in range(rng.randint(0, 6))]
    batches = [[rng.choice(base) if base and rng.random() < 0.4 else suggestion() for _ in range(rng.randint(0, 5))]
               for _ in range(rng.randint(0, 4))]
    limit = rng.choice([0, 1, 2, 3, 8, 8, 100, 2.5, -1, 0.5, INF])
    rows.append({"fn": "outputItems", "args": [limit, batches]})
for _ in range(100):
    rows.append({"fn": "createRowOf", "args": [{"id": text(), "title": text(), "url": url(), "site": text()}]})
for _ in range(150):
    a = {"id": rng.choice(["a", "b", ""]), "title": text()}
    if rng.random() < 0.3:
        a["keywords"] = ["k", "w"]
    for field, values in (("icon", ["x", "", None]), ("hint", ["⌘K", "", None])):
        if rng.random() < 0.6:
            v = rng.choice(values)
            if v is not None:
                a[field] = v
    rows.append({"fn": "actionRowOf", "args": [a]})
WORDS = ["foo", "Foo", " foo ", "FOO", "", "  ", "bar", "ß", "SS", "İ", "i̇", "ΣΑΣ", "σας", "\u00a0x\u00a0",
         "x", "\ufeffx", "\u200bx", "日本", "Ǆ", "ǆ", "K", "k"]
for _ in range(250):
    remote = [rng.choice(WORDS) for _ in range(rng.randint(0, 8))]
    rows.append({"fn": "dedupeSuggestionsOf", "args": [remote, rng.choice(WORDS)]})
write("suggestionRows", rows)

# ---- candidatePool -------------------------------------------------------------------------------

cases = []
ids = ["1", "2", "3", "4", "5", "", "ü"]
for _ in range(450):
    urls = shared_urls(rng.randint(1, 5))
    bad = 0.02
    histories = [[history_row(urls, bad) for _ in range(rng.randint(0, 7))] for _ in range(rng.randint(1, 2))]
    lists = [None if rng.random() < 0.25 else [bookmark_row(urls, bad) for _ in range(rng.randint(0, 5))]
             for _ in range(rng.randint(1, 3))]
    keys = [page_key(u) for u in rng.sample(urls, min(len(urls), 3))] + [rng.choice(["", "nope", urls[0]])]
    cases.append({"fn": "poolGrid", "args": [histories, lists, keys]})
for i in range(150):
    urls = shared_urls(rng.randint(2, 30))
    n = rng.choice([0, 5, 63, 64, 65, 130, 200]) if i % 3 == 0 else rng.randint(0, 10)
    source = {"tabs": [], "history": [history_row(urls, 0.005) for _ in range(n)]}
    if rng.random() < 0.8:
        source["bookmarks"] = [bookmark_row(urls, 0.005) for _ in range(rng.choice([0, 3, 64, 70, 140]))]
    keys = [page_key(u) for u in urls[:3]] + [""]
    cases.append({"fn": "preparedPool", "args": [source, rng.choice([0, 0, 1e300, INF]), keys]})
for _ in range(500):
    urls = shared_urls(rng.randint(1, 5))
    bad = 0.02
    source = {"tabs": [tab_row(urls, ids, bad) for _ in range(rng.randint(0, 8))],
              "history": [history_row(urls, bad) for _ in range(rng.randint(0, 5))]}
    if rng.random() < 0.7:
        source["bookmarks"] = [bookmark_row(urls, bad) for _ in range(rng.randint(0, 4))]
    cases.append({"fn": "tabsOverView", "args": [source, rng.choice([None, "1", "2", "", "9"])]})
for _ in range(250):
    urls = shared_urls(rng.randint(1, 5))
    history = [history_row(urls) for _ in range(rng.randint(0, 6))]
    probe = [rng.choice(urls) + rng.choice(["", "/", "#x", "?"]) for _ in range(rng.randint(1, 4))] + [url()]
    cases.append({"fn": "savedVisits", "args": [history, probe]})
for _ in range(250):
    urls = shared_urls(rng.randint(1, 5))
    marks = [bookmark_row(urls) for _ in range(rng.randint(0, 6))]
    probe = [rng.choice(urls) + rng.choice(["", "/", "#x"]) for _ in range(rng.randint(1, 4))] + [url()]
    cases.append({"fn": "savedBookmarks", "args": [marks, probe]})
write("candidatePool", cases)

# ---- openTabs ------------------------------------------------------------------------------------

cases = []
for _ in range(1600):
    n = rng.choice([0, 1, 2, 3, 5, 8, 9, 12, 25])
    times = [rng.choice([1, 2, 3, 3, 5, 100, 0, -1, 1.5]) if rng.random() < 0.5 else rng.randint(0, 2**40)
             for _ in range(n)]
    if rng.random() < 0.05 and n:
        times[rng.randrange(n)] = rng.choice([INF, NINF])
    tabs = [{"id": rng.choice(ids + [f"t{i}"]), "url": rng.choice(["", url(), url()]), "title": text(),
             "favicon": rng.choice(FAVICONS), "lastActiveAt": t} for i, t in enumerate(times)]
    args = [tabs]
    r = rng.random()
    if r < 0.75:
        opts = {}
        if rng.random() < 0.5:
            opts["exclude"] = rng.choice(ids + ["t1", "t3"])
        if rng.random() < 0.6:
            opts["limit"] = rng.choice([0, 1, 2, 3, 8, 100, 2.5, 0.5, INF])
        args.append(opts)
    elif r < 0.85:
        args.append(UNDEF)
    cases.append({"fn": "recentTabSuggestions", "args": args})
write("openTabs", cases)

# ---- suggestFetcher ------------------------------------------------------------------------------

ENGINES = [
    {"id": "google", "name": "Google", "keyword": "g", "url": "https://www.google.com/search?q=%s",
     "suggestUrl": "https://suggest.test/complete?q=%s"},
    {"id": "ddg", "name": "DuckDuckGo", "keyword": "d", "url": "https://duckduckgo.com/?q=%s",
     "suggestUrl": "https://ddg.test/ac/?q=%s&type=list"},
    {"id": "custom", "name": "Mine", "keyword": "m", "url": "https://mine.test/?q=%s",
     "suggestUrl": "https://mine.test/s?q=%s", "custom": True},
    {"id": "nosuggest", "name": "Plain", "keyword": "p", "url": "https://plain.test/?q=%s"},
    {"id": "empty", "name": "Empty", "keyword": "e", "url": "https://e.test/?q=%s", "suggestUrl": ""},
    {"id": "ext", "name": "Ext", "keyword": "x", "url": "https://x.test/?q=%s",
     "suggestUrl": "https://x.test/s?q=%s", "custom": False, "extension": {"id": "abc", "name": "A"}},
]
QUERIES = ["a", "ab", "abc", "a b", "", " ", "\u00a0", "ü", "日本", "😀", "a&b=c", "100%", "a+b", "#x", "a/b"]


def reply_body():
    q = rng.choice(QUERIES)
    sug = [rng.choice(["foo", " foo ", "", "https://a.test/", "http://b.test", "ftp://c.test", "https://",
                       "ünï", "1+1", "news.test", 3, None, True]) for _ in range(rng.randint(0, 6))]
    titles = [rng.choice(["A", " B ", "", None, 4]) for _ in range(rng.randint(0, 6))]
    types = [rng.choice(["QUERY", "NAVIGATION", "PERSONALIZED_NAVIGATION", "CALCULATOR", ["NAVIGATION"],
                         [["CALCULATOR"]], ["NAVIGATION", "x"], 1, None, {}, True])
             for _ in range(rng.randint(0, 6))]
    shape = rng.random()
    if shape < 0.55:
        body = [q, sug, titles, [], {"google:suggesttype": types}]
    elif shape < 0.65:
        body = [q, sug]
    elif shape < 0.7:
        body = [q, sug, "titles", [], {"google:suggesttype": "NAVIGATION"}]
    elif shape < 0.75:
        body = [q, sug, titles, [], ["google:suggesttype"]]
    elif shape < 0.8:
        body = {"0": q, "1": sug}
    elif shape < 0.85:
        body = [q, "not-a-list"]
    else:
        body = [q, sug, titles, None, {"google:suggesttype": types, "google:suggesttype ": []}]
    text = json.dumps(body, ensure_ascii=rng.random() < 0.3)
    r = rng.random()
    if r < 0.15:
        text = ")]}'" + rng.choice(["", "\n", " \t\n", "\u00a0\ufeff"]) + text
    elif r < 0.22:
        text = rng.choice(["", "{", "[1,]", "<html>", "nul", "[\"a\",[\"b\"]] x", "// c\n[]", ")]}'", "  [] "])
    return text


def steps():
    out = []
    for _ in range(rng.randint(1, 14)):
        r = rng.random()
        if r < 0.35:
            out.append({"op": "request", "engine": rng.choice(ENGINES), "query": rng.choice(QUERIES)})
        elif r < 0.45:
            out.append({"op": "cancel"})
        elif r < 0.65:
            out.append({"op": "fire"})
        elif r < 0.85:
            out.append({"op": "reply", "ok": rng.random() < 0.85, "body": reply_body()})
        elif r < 0.92:
            out.append({"op": "unreadable"})
        else:
            out.append({"op": "fail"})
    return out


cases = []
for i in range(1500):
    s = steps()
    if i % 4 == 0:
        # a full round trip, then the same query again from the cache
        e, q = rng.choice(ENGINES[:2]), rng.choice(QUERIES[:4])
        s = [{"op": "request", "engine": e, "query": q}, {"op": "fire"},
             {"op": "reply", "ok": True, "body": reply_body()}, {"op": "request", "engine": e, "query": q}] + s
    args = [s]
    if rng.random() < 0.4:
        args.append(rng.choice([0, 1, 2, 64, -1, 1.5, INF]))
    cases.append({"fn": "runFetcher", "args": args})
write("suggestFetcher", cases)

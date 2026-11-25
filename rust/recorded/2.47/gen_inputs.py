#!/usr/bin/env python3
"""Writes import.inputs.jsonl: the recorded inputs of slice 2.47.

Each line is one replay of `import.harness.ts`: a list of steps `[export, ...args, script]`.
Run from anywhere: `python3 rust/recorded/2.47/gen_inputs.py`.
"""
import json
import pathlib

JOB = "$job"
OK = lambda text: {"reply": {"text": json.dumps(text, separators=(",", ":")) if not isinstance(text, str) else text}}
NIL = {"reply": {"text": None}}
FAIL = lambda message, code=None: {"reply": {"error": {"message": message, **({"code": code} if code else {})}}}
VALUE = lambda v: {"reply": {"value": v}}


def progress(job, kind="history", phase="progress", processed=10, total=None):
    event = {"type": "progress", "jobId": job, "kind": kind, "phase": phase, "processed": processed}
    if total is not None:
        event["total"] = total
    return {"emit": event}


def history(job, entries):
    return {"emit": {"type": "history", "jobId": job, "entries": json.dumps(entries, separators=(",", ":"))}}


tab = {"url": "https://a.test/", "title": "A", "pinned": True, "customTitle": "Mine", "groupId": "g1", "splitId": "s1",
       "windowIndex": 0, "active": True, "lastActive": 1700000000.5, "container": 2}
tab_min = {"url": "https://b.test/", "title": "B", "pinned": False, "windowIndex": 1, "active": False}
entry = {"url": "https://h.test/", "title": "H", "visits": 3, "lastVisit": 1700000000123, "typedCount": 1}
entry_min = {"url": "https://i.test/", "title": "I", "visits": 1, "lastVisit": 5}
# Keys follow the field order of the Rust types: the diff compares objects in key order.
mark = {"type": "folder", "title": "Bar", "children": [
    {"type": "url", "title": "T", "url": "https://t.test/", "dateAdded": 1700000000, "pageTitle": "Page", "icon": "i.png"},
    {"type": "folder", "title": "Empty", "children": []}], "role": "toolbar", "color": "blue"}
cred = {"url": "https://c.test/", "username": "u", "password": "p", "realm": "r", "title": "t", "note": "n",
        "otpAuth": "otpauth://totp/x", "created": 1, "lastUsed": 2, "timesUsed": 3}
cred_min = {"url": "https://d.test/", "username": "v", "password": "q"}
warn = {"kind": "tabs", "code": "locked", "message": "Browser is open"}
warn_file = {"code": "unreadableFile", "message": "bad zip"}
dest = {"kind": "space", "spaceId": "sp1", "spaceName": "Work"}
space = {"id": "sp1", "name": "Work", "color": "red", "colors": ["red", "blue"], "emoji": "W", "icon": "briefcase",
         "profileId": "p1", "pinned": [mark], "tabs": [tab, tab_min], "theme": "dark", "container": 1,
         "openGroups": [mark], "favorites": [tab]}
space_min = {"id": "sp2", "name": "Home", "profileId": "p1", "pinned": [], "tabs": []}
result_full = {
    "browserId": "chrome", "profileId": "Default", "profile": {"name": "Me", "color": "green", "avatarPath": "/a.png"},
    "bookmarks": mark, "history": [entry, entry_min], "historyCount": 2, "tabs": [tab, tab_min],
    "tabGroups": [{"id": "g1", "title": "G", "color": "purple", "collapsed": True}],
    "credentials": [cred, cred_min], "cookieCount": 4, "addressCount": 5, "cardCount": 6, "vaultToken": "tok",
    "spaces": [space, space_min], "favorites": [tab],
    "favicons": [{"page": "https://a.test/", "icon": "https://a.test/i.ico", "png": "AAAA"}],
    "searchEngine": {"name": "S", "keyword": "s", "url": "https://s.test/?q=%s", "prepopulateId": 7},
    "separateEssentials": True, "diaSidebar": {"primarySpaceId": "sp1"},
    "arcRouting": {"rules": [{"id": "r1", "match": "contains", "text": "github", "to": dest},
                             {"id": "r2", "match": "equals", "text": "x", "to": {"kind": "littleArc"}}],
                   "skipped": 1, "fallback": {"kind": "mostRecent"}},
    "arcArchive": {"tabs": [{"url": "https://z.test/", "title": "Z", "customTitle": "ZZ", "archivedAt": 1700000000,
                             "spaceId": "sp1", "spaceName": "Work", "auto": True}], "dropped": 2, "cleanUpAfterMinutes": 720},
    "arcCachedIcons": 9, "failed": ["passwords", "cookies"], "warnings": [warn, warn_file],
}
result_min = {"browserId": "firefox", "profileId": "x", "history": [], "historyCount": 0, "tabs": [], "tabGroups": [],
              "credentials": [], "cookieCount": 0, "addressCount": 0, "cardCount": 0, "spaces": [], "favorites": [],
              "favicons": [], "failed": [], "warnings": []}
safari = {"bookmarks": mark, "credentials": [cred], "profiles": [{"name": "Work", "history": [entry], "extensions": ["a", "b"]},
                                                              {"history": [], "extensions": []}],
          "tabs": [tab], "cookieCount": 3, "cardCount": 1, "vaultToken": "vt", "warnings": [warn]}
safari_min = {"credentials": [], "profiles": [], "tabs": [], "cookieCount": 0, "cardCount": 0, "warnings": []}
sources = [
    {"id": "chrome", "name": "Chrome", "family": "chromium", "appPath": "/Applications/Chrome.app", "iconPath": "/i.png",
     "requiresExport": False, "needsKeychain": True, "needsFullDiskAccess": False, "keychainService": "Chrome Safe Storage",
     "profiles": [{"id": "Default", "name": "Me", "path": "/p", "email": "me@x.test", "avatarPath": "/av.png", "color": "blue",
                   "isDefault": True, "available": ["bookmarks", "history", "tabs", "passwords", "cookies", "autofill"],
                   "spaces": [{"id": "s", "name": "S", "color": "red", "emoji": "E", "icon": "i", "pinnedCount": 1, "tabCount": 2,
                               "theme": "t", "container": 3, "containerName": "Work"}],
                   "tabsNeedKey": True, "diaSidebar": True}],
     "sidebar": "arc", "asksAppForTabs": True},
    {"id": "safari", "name": "Safari", "family": "safari", "requiresExport": True, "needsKeychain": False, "profiles": []},
    {"id": "zen", "name": "Zen", "family": "firefox", "requiresExport": False, "needsKeychain": False,
     "profiles": [{"id": "z", "name": "Z", "path": "/z", "isDefault": False,
                   "available": ["spaces", "pinnedTabs", "favorites", "linkRules", "archive"]}], "sidebar": "zen"},
    {"id": "arc", "name": "Arc", "family": "arc", "requiresExport": False, "needsKeychain": False, "profiles": []},
]
dia = {"profiles": [{"id": "d", "name": "D", "index": 0, "pinned": [tab], "tabs": [tab_min]}], "windowCount": 2, "skipped": 1,
       "duplicates": 0, "favicons": []}
exts = [{"id": "e1", "name": "E", "webStoreUrl": "https://chromewebstore.google.com/detail/e1", "version": "1.0", "enabled": True,
         "pinnedIndex": 0},
        {"id": "e2", "name": "F", "webStoreUrl": "", "skip": "manifestV2"}, {"id": "e3", "name": "G", "webStoreUrl": "", "skip": "notWebStore"}]
counts = {"cookies": 3, "cookiesRejected": 1, "cookiesExisting": 2, "addresses": 4, "cards": 5}
counts_failed = {**counts, "cookiesFailed": 1, "autofillFailed": 1, "error": "disk full"}


def import_opts(**kw):
    return kw


cases = []


def case(*steps):
    cases.append([list(s) for s in steps])


# importData: events of the job, of another job, and callbacks on or off.
events = [progress(JOB, "bookmarks", "start", 0, 3), history(JOB, [entry, entry_min]), progress("import-99", "tabs", "end", 1),
          history("import-99", [entry]), progress(JOB, "bookmarks", "end", 3, 3)]
case(["importData", "chrome", "Default", ["bookmarks", "history"],
      {"historyLimit": 100, "historySince": 1700000000, "spaceIds": ["a", "b"], "chunkSize": 50, "onProgress": True,
       "onHistoryChunk": True, "signal": "live"}, {"actions": events, "reply": OK(result_full)["reply"]}])
case(["importData", "chrome", "Default", ["history"], {"onProgress": True}, {"actions": events, "reply": OK(result_min)["reply"]}])
case(["importData", "chrome", "Default", ["history"], {"onHistoryChunk": True}, {"actions": events, "reply": OK(result_min)["reply"]}])
case(["importData", "chrome", "Default", [], {}, {"actions": events, "reply": OK(result_min)["reply"]}])
case(["importData", "safari", "p", ["tabs", "pinnedTabs", "linkRules", "archive"], {"historyLimit": 0, "chunkSize": 0},
      {"reply": NIL["reply"]}])
case(["importData", "chrome", "Default", ["cookies"], {"onProgress": True, "onHistoryChunk": True, "signal": "live"},
      {"actions": [history(JOB, []), progress(JOB, "cookies", "progress", 2)], "reply": {"error": {"code": "ERR_LOCKED", "message": "Keychain locked"}}}])
case(["importData", "chrome", "Default", ["cookies"], {"signal": "live"}, FAIL("plain failure")])
# A signal that is already aborted: no native call, and the next job is import-2.
case(["importData", "chrome", "Default", ["history"], {"onProgress": True, "signal": "aborted"}, {"reply": OK(result_min)["reply"]}],
     ["importData", "chrome", "Default", ["history"], {"signal": "live"}, {"reply": OK(result_min)["reply"]}],
     ["importSafariDirect", "aborted", NIL], ["importSafariExport", "/x.zip", "aborted", NIL],
     ["importSafariExport", "/x.zip", "none", OK(safari)])
# An abort during the job: cancelImport, then the listeners are gone.
case(["importData", "chrome", "Default", ["history"], {"onProgress": True, "signal": "live"},
      {"actions": [progress(JOB, "history", "start", 0), {"abort": True}, {"abort": True}, progress(JOB, "history", "end", 1)],
       "reply": FAIL("cancelled by native", "cancelled")["reply"]}])
case(["importSafariDirect", "live", {"actions": [{"abort": True}], "reply": OK(safari_min)["reply"]}],
     ["importSafariExport", "/y.zip", "live", {"actions": [{"abort": True}], "reply": FAIL("x", "cancelled")["reply"]}],
     ["importSafariDirect", "none", {"actions": events, "reply": OK(safari)["reply"]}])
# Each other export.
case(["listBrowsers", OK(sources)], ["listBrowsers", NIL], ["listBrowsers", OK([])], ["listBrowsers", FAIL("no", "ERR_X")], ["listBrowsers", FAIL("boom")])
case(["unlockBrowser", "firefox", None, NIL], ["unlockBrowser", "firefox", "pw", NIL], ["unlockBrowser", "firefox", "", NIL],
     ["unlockBrowser", "firefox", "bad", FAIL("Wrong primary password", "ERR_PASSWORD")], ["unlockBrowser", "firefox", None, OK("null")])
case(["isBrowserUnlocked", "chrome", VALUE(True)], ["isBrowserUnlocked", "chrome", VALUE(False)], ["forgetUnlockedKeys", VALUE(None)],
     ["safariHasFullDiskAccess", VALUE(True)], ["safariHasFullDiskAccess", VALUE(False)], ["discardImported", "tok", VALUE(None)])
case(["writeImported", "tok", "prof", ["cookies", "autofill"], OK(counts)], ["writeImported", "tok", "prof", ["cookies"], OK(counts_failed)],
     ["writeImported", "tok", "prof", [], NIL], ["writeImported", "tok", "prof", ["autofill"], FAIL("gone", "ERR_TOKEN")])
case(["listExtensions", "chrome", "Default", OK(exts)], ["listExtensions", "chrome", "Default", OK([])], ["listExtensions", "x", "y", NIL],
     ["listExtensions", "x", "y", FAIL("none", "ERR_NO")])
case(["importBookmarksHTML", "/b.html", OK(mark)], ["importBookmarksHTML", "/b.html", NIL], ["importBookmarksHTML", "/c.html", FAIL("parse", "ERR_PARSE")],
     ["importPasswordsCSV", "/p.csv", OK([cred, cred_min])], ["importPasswordsCSV", "/p.csv", OK([])], ["importPasswordsCSV", "/q.csv", FAIL("csv")])
case(["chooseImportFile", "safariExport", OK("/Users/me/export.zip")], ["chooseImportFile", "bookmarksHTML", NIL],
     ["chooseImportFile", "passwordsCSV", FAIL("panel closed")], ["chooseImportFile", "passwordsCSV", FAIL("denied", "ERR_DENIED")],
     ["openFullDiskAccessSettings", NIL], ["openFullDiskAccessSettings", FAIL("no settings", "ERR_SETTINGS")],
     ["readDiaTabs", OK(dia)], ["readDiaTabs", NIL], ["readDiaTabs", FAIL("Dia is not running", "ERR_NOT_RUNNING")])
# Unicode and a long run of jobs.
case(*[["importData", "b\u00e9\U0001f600", "p\u2028", ["history"], {"onProgress": True}, {"actions": [progress(JOB, "history", "progress", n, n + 1)], "reply": OK(result_min)["reply"]}] for n in range(12)])
case(["nope", NIL])
# Malformed text: the message is the JSON parser's, so it is in known-diffs.jsonl.
case(["listBrowsers", {"reply": {"text": "{"}}])

pathlib.Path(__file__).with_name("import.inputs.jsonl").write_text(
    "".join(json.dumps({"fn": "replayImport", "args": [json.dumps(c, separators=(",", ":"), ensure_ascii=False)]}, ensure_ascii=False) + "\n" for c in cases)
)
print(len(cases), "replays")

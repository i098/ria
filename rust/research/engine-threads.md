# The `ac_engine` C interface: threads, lifetimes, what Rust keeps alive

Scope: which thread each call and each event runs on, who owns each pointer, and what Rust must keep alive. Mac only (decision C3). Research only. No code.

## Source and pin

- Ardia: `551d3c13c1c9ede130724ea9f1056d4b586fbe55` (`git rev-parse HEAD` in the read-only clone). Short name `ardia@551d3c1`.
  - `PUB:` = `engine/chromium/src/chrome/browser/ardia/public/ac_engine.h` (42 lines).
  - `PRIV:` = `engine/chromium/src/chrome/browser/ardia/ac_engine.h` (191 lines, the C++ side).
  - `IMPL:` = `engine/chromium/src/chrome/browser/ardia/ac_engine.cc`.
  - `DISP:` = `engine/chromium/src/chrome/browser/ardia/ac_engine_dispatch.cc`.
  - `GN:` = `engine/chromium/src/chrome/browser/ardia/BUILD.gn`.
  - `BR:` = `packages/ardiacore/ios/ArdiaEngineBridge.mm` (today's C bridge).
  - `C:` = `engine/ardiacore/src/ardia/core/` (the `ArdiaCore` ObjC layer).
- `docs/engine-build.md` is `DOC:`.
- Anything without a line cite is `[INFERENCE]`.

## 1. The interface

`PUB:` is a plain C header. It has 4 function-pointer types, 2 exported functions, and nothing else.

| Item | Line | Shape |
| --- | --- | --- |
| `ac_engine_reply_t` | `PUB:12` | `void (*)(void* context, const char* json)` |
| `ac_engine_call_t` | `PUB:13-16` | `void (*)(const char* profile_dir, const char* args_json, ac_engine_reply_t reply, void* context)` |
| `ac_engine_event_t` | `PUB:17-19` | `void (*)(void* context, const char* topic, const char* json)` |
| `ac_engine_dispatch_t` | `PUB:21-25` | Same as `ac_engine_call_t` with a leading `const char* name`. |
| `ac_engine_set_event_sink_t` | `PUB:27-28` | `void (*)(ac_engine_event_t sink, void* context)` |
| `ac_engine_call` | `PUB:30-34` | The dispatch function. |
| `ac_engine_set_event_sink` | `PUB:35-36` | Sets the one event sink. |

The rules in `DOC:330-340` (my words):

- Every export is named `ac_<domain>_<verb>` and has the `ac_engine_call_t` signature.
- Arguments and results are JSON objects. A failure is `{"error": ...}`.
- UI thread only.
- The reply runs once, maybe before the call returns.
- Events go to one sink as `("<domain>.<what>", {profile, ...})`.

### Which entry point Rust calls on macOS

- On macOS, `ac_engine_call` does **not** exist. `DISP:` is in the Linux source list only (`GN:90-92`), as is the generated registry (`GN:104-107`, `GN:211-212`). The Mac build exports every symbol that starts with `_ac_` through an exports list (`GN:226-230`; the list is one line, `_ac_*`, in `engine/chromium/src/chrome/browser/ardia/ac_engine.exports:1`).
- Today's Mac host therefore finds each call by name: `dlsym(RTLD_DEFAULT, name)` cast to `ac_engine_call_t` (`BR:50`; used at `BR:134, 155`). Rust does the same, or declares each `extern "C"` function. [INFERENCE] The dynamic way costs one lookup per call; cache the pointer.
- Size of the surface: 109 distinct names match `AC_(ENGINE|PROFILE|ARGS)_CALL(ac_...)` in `ac_*.cc` (`grep -rhoE` and `sort -u`, run in the `ardia` tree). The TypeScript type `EngineCalls` lists 138 calls and `EngineEvents` lists 27 events (`packages/ardiacore/src/engine/api.ts:30-232` and `:234-269`; counted with `awk`). The two numbers differ because the TS list includes calls that other Swift modules provide. [INFERENCE] Not verified call by call.
- `ac_engine_set_event_sink` is in `IMPL:` and exists on both platforms (`IMPL:291-299`).

## 2. Threads

### 2.1 Calls

| Fact | Evidence |
| --- | --- |
| The doc rule says UI thread only. | `DOC:332` |
| The Linux dispatcher checks it with a hard `CHECK`. | `DISP:24` |
| The Mac call has no dispatcher, but the shared argument parser checks it with `DCHECK_CURRENTLY_ON(UI)` in debug builds. | `IMPL:114` |
| `Reply::Send` checks it too, in debug builds. | `IMPL:63, 73, 84` |
| `ac_engine_set_event_sink` has a hard `CHECK` on Linux only. | `IMPL:294-296` |
| The Swift/ObjC++ host calls every engine function from blocks run on the main thread. `WithProfile(...)` runs the completion either at once or after `loadProfile:`, which completes on the UI thread. | `BR:146-149`; `packages/ardiacore/ios/ArdiaProfiles.mm:179-201` |

- On macOS the UI thread is the process main thread. [INFERENCE] The evidence: Chromium runs `MessagePumpNSApplication` and `[NSApp run]` itself (`docs/ardiacore-spike.md:40-42`), and the engine hops to `dispatch_get_main_queue()` before it calls the host (`C:ardiacore_hooks.mm:111-113`, `C:ardiacore_tab_commands.mm:218`).
- **Rule for Rust: call `ac_*` only on the main thread.** Release builds do not check it on Mac, so a violation gives a data race, not a clean crash. [INFERENCE] From `DCHECK` being compiled out in release builds.
- To call from a Rust worker thread, post to the main queue: `dispatch_async(dispatch_get_main_queue(), ...)`. Ardia does this to defer its own forwarding (`BR:102-104`).

### 2.2 Replies

- `Reply::Send` takes the callback and the context out of the object with `std::exchange` before it calls `reply(context, json)` (`IMPL:62-70`). So the reply fires once. [Evidence for "once": lines 67-69.]
- The reply runs on the UI thread (`DCHECK` at `IMPL:63`). It runs either:
  - **before the call returns** (synchronous). Examples: bad JSON (`IMPL:116-119, 126-134`), unknown profile (`IMPL:219-223`), refused private profile (`IMPL:138-140`); or
  - **later**, on the UI thread, from a task or callback the call started.
- A `Reply` that is destroyed without a send reports `{"error":"no reply"}` (`IMPL:56-60`, move-assign at `IMPL:45-54`). [INFERENCE] So every call gets exactly one reply even when the handler fails or drops its `Reply`. Rust must still time out: if the engine never destroys the `Reply` (a leaked callback), no reply comes. Ardia's own `callForSecretReply:` has a timer that settles with `nil` (`BR:175-184`).
- The JSON pointer passed to `reply` is a temporary: `ToJSON(result).c_str()` (`IMPL:69`) or the caller's string for secrets (`IMPL:77`). It is valid only for the duration of the callback. Rust must copy it (`CStr::to_str` then `to_owned`), never store the pointer. Ardia copies it with `@(json)` (`BR:55`).

### 2.3 Events

- `Emit` has `DCHECK_CURRENTLY_ON(UI)` (`IMPL:276`). So the sink runs on the UI thread, which is the main thread.
- `Emit` drops the event when no sink is set (`IMPL:277-279`). Events that fire before `set_event_sink` are lost.
- `Emit` adds `profile` (the original profile path) and, for a private profile, `offTheRecord: true` to the payload (`IMPL:280-285`). Then it calls `g_sink(g_sink_context, topic, json)` with temporaries (`IMPL:286`). Both strings are valid only during the callback.
- The sink is a single global: `g_sink` and `g_sink_context` (`IMPL:29-30`). A second `set_event_sink` replaces the first (`IMPL:297-298`). Rust must own one sink and fan out by topic itself. Ardia's bridge does that: an `Observers()` map of topic to handler array, with `InstallSink` called once (`BR:79, 110, 120, 191-197`).
- The sink is called **inside** engine code. It can run while an `ac_*` call is on the stack. [INFERENCE] Ardia defers some events with `dispatch_async` and merges repeated ones (`BR:95-105`), which is a sign that the authors hit re-entrancy. A Rust sink must not borrow a `RefCell` that a call site already holds.

### 2.4 `ArdiaCore` delegate callbacks and completion blocks

- Engine delegate callbacks come on the main thread: either directly from the engine's UI-thread code (`C:ardiacore_engine.mm:126-141`) or through the main queue (`C:ardiacore_hooks.mm:111-113`).
- Tab delegate callbacks come from the engine's tab observer, sent by `notify:` on the UI thread (`C:ardiacore_tab.mm:319-324`).
- `loadProfile:completion:` calls its block on the UI thread, either right away (`C:ardiacore_engine.mm:196-200`) or from the profile manager's callback (`C:ardiacore_engine.mm:201-208`).
- The engine uses `base::ThreadPool` only for file deletion (`C:ardiacore_engine.mm:326-331`). No callback to the host comes from a pool thread in the code I read. [INFERENCE] I read these call sites only. I did not read every `.mm` file for off-thread callbacks.

## 3. Lifetimes

| Object | Who owns it | Valid until | Rust action |
| --- | --- | --- | --- |
| `profile_dir` (`const char*`) | Caller | The call returns. The engine reads it in the `Call` constructor and copies nothing: `ProfileAt(profile_dir ? profile_dir : "")` (`IMPL:219`) is a lookup. | A `CString` on the stack or in the call frame is enough. |
| `args_json` | Caller | The call returns. `ArgsCall` parses it into its own `DictValue` in the constructor (`IMPL:126-135`). Later work uses the parsed copy. [INFERENCE] I did not read all 109 handlers; a handler that stores the `const char*` itself would break this. | Same. Do not free before the call returns. |
| `context` | Caller | Until the reply fires. | A `Box` turned into a raw pointer with `into_raw`. The trampoline reclaims it with `from_raw`. Ardia uses `__bridge_retained` to send and `__bridge_transfer` to take it back (`BR:148, 54`). Because a reply always comes (section 2.2), the box is freed. |
| `reply` function pointer | Caller | Always valid. | A plain `extern "C" fn`. It has no captured state; the state is in `context`. |
| `json` in the reply | Engine | The reply returns (section 2.2). | Copy it. |
| Event sink function and `context` | Caller | Until replaced or until the engine shuts down. The engine does not clear them on quit. `IMPL:297-298` is the only writer. [INFERENCE] | A `static` or a leaked `Box`. Clear it in `engineWillShutDown` so a late event cannot hit a freed context. |
| Event `topic`/`json` | Engine | The sink returns. | Copy it. |
| Secret arguments | Caller buffer | The call returns, then Rust must zero it. `ArgsCall` wipes its parsed copy on destruction (`IMPL:145-149`). Ardia zeroes its own buffer right after the call (`BR:152-171`). The buffer must end in a NUL byte (`BR:157`). | A `Zeroizing` buffer. A call with secrets limits size with `ardia::Sensitive{max_bytes}` (`PRIV:78-80`, `IMPL:116-125`). |

`Call` also refuses a call for a private profile unless the handler opted in. The host marks private calls with `offTheRecord: true` in the args (`BR:136-141`; refusal at `IMPL:137-141`). Rust has to add the same key for a private session.

## 4. What Rust must keep alive

1. **The main thread.** `runWithArgc` blocks it for the whole run (`C:ardiacore_engine.mm:170`). Rust code runs inside callbacks. Do not return from `main` before the engine ends.
2. **The `argv` strings** passed to `runWithArgc` (`C:ardiacore_engine.mm:153-173`). Ardia keeps them in a `static` vector (`packages/ardiacore/ios/ArdiaHost+Launch.mm:528-534`).
3. **The engine delegate object.** The engine holds it strongly (`C:ardiacore_engine.mm:62`) and drops it when `ChromeMain` returns (line 171). Rust may keep its own `Retained` as well.
4. **Window and tab delegate objects.** The engine holds them weakly (`ArdiaCore.h:188, 287`). A dropped delegate stops callbacks without an error. See `objc2-ardiacore.md` section 4.
5. **The event sink context.** A process-wide static (section 3).
6. **Reply contexts.** Boxes in flight. One per outstanding call.
7. **`Retained` handles to `ArdiaCoreWindow`, `ArdiaCoreTab`, `ArdiaCoreProfile` and the `NSView`s.** A tab wrapper outlives its contents and then reports `closed` (`C:ardiacore_tab.mm:315-317, 326-329`).
8. **The Rune `LayerHost`, its `CAMetalLayer`, and the `NSView` that holds the layer.** `LayerHost` requires the layer to outlive it and every call to run on the UI thread (`crates/rune/src/platform/embed.rs:209-211` in Rune `ada31dc`; see `rune-embed-host.md`).
9. **The `Waker`** that Rune takes (`Arc<dyn Fn() + Send + Sync>`, `crates/rune-core/src/lib.rs:50`). It can run on any thread, so it must post to the main queue and call `frame()` there.
10. **The display link.** `LayerHost` has none (`rune-embed-host.md` section 7). The host owns it, and it must run on the main thread or post to it.

## 5. Shutdown order

1. `engineWillShutDown` is sent from `OnEngineShuttingDown` after `CloseAllExtensionViews()` (`C:ardiacore_engine.mm:138-141`).
2. At this point Rust should: stop the display link, drop the `LayerHost`, clear the event sink, and cancel its own pending replies. [INFERENCE] The engine does not clear the sink (section 3), so a late `Emit` would use a freed context.
3. `quit` asks Chromium to end (`C:ardiacore_engine.mm:211-213`). `engineQuitCancelled` is sent if a page vetoes it (`ArdiaCore.h:15`; today's host posts `ArdiaHostQuitCancelled`, `packages/ardiacore/ios/ArdiaHostEngineDelegate.mm:115-119`).
4. `runWithArgc` returns the exit code (`C:ardiacore_engine.mm:170-172`).

## 6. Patterns the current host already uses (copy them)

- **Wait for `engineDidStart` before any `ac_*` call.** `ArdiaHost.isStarted` is `engineRunning` (`packages/ardiacore/ios/ArdiaHost.mm:458-460`), and every bridge call checks it (`BR:147, 164, 184`). `engineRunning` is set in `engineDidStart` (`packages/ardiacore/ios/ArdiaHostEngineDelegate.mm:35-38`).
- **Single-flight profile loads.** `WithProfile` keeps a waiter list per profile name, so two callers share one `loadProfile:` (`packages/ardiacore/ios/ArdiaProfiles.mm:197-201`).
- **Private sessions** go through `WithPrivateProfile` and a flag in the args (`ArdiaProfiles.mm:185`; `BR:136-141`).

## 7. Open risks

### Proof A (Rust starts the engine, opens a window and a tab, loads a page, quits clean)

1. **Main thread is blocked inside `runWithArgc`.** Everything Rust does must start from a delegate callback or a main-queue block. The first real call is likely in `engineDidStart` (`C:ardiacore_engine.mm:126-136`). Test that a `dispatch_async(main)` block runs before and after the first window opens.
2. **No `ac_engine_call` on Mac** (section 1). Proof A does not need one `ac_*` call. It needs `ArdiaCore` ObjC calls only. Keep one `ac_*` call out of A. Put the first `ac_*` call, for example a cookies or bookmarks call, in B or in a follow-up, if you want the C path covered.
3. **Debug versus release.** The `DCHECK`s in section 2.1 are off in a release engine. [INFERENCE] A wrong-thread call in the proof may pass by luck. Run one wrong-thread test against a build that has `DCHECK` on, if one exists. I did not check whether the prebuilt engine has them on.
4. **Clean quit.** Section 5. The sink and delegates must be gone before `runWithArgc` returns.

### Proof B (Rune `LayerHost` beside the page view)

1. **Frame driver thread.** A `CVDisplayLink` calls back on its own thread. `LayerHost::frame()` and the layer calls must run on the UI thread (`embed.rs:209-211`). The callback must post to main, and it must not queue a frame per tick if main is slow. Measure missed ticks.
2. **`Waker` hop.** Same rule (item 9 of section 4).
3. **Re-entrancy.** An event sink call can arrive while Rust runs `frame()` or an input handler on the main thread (section 2.3). Rune's `Runtime` is not re-entrant. [INFERENCE] Defer sink work with `dispatch_async` and merge repeated events, as `BR:95-105` does.
4. **Reply that never comes.** Add a timeout to every call, as `callForSecretReply:` does (`BR:178`).

### Proof C (Rune layer above the page view, glass under, VoiceOver)

1. **No engine-thread risk of its own.** C is a view and accessibility test. The shared risk is that VoiceOver actions arrive on the main thread and must reach `Runtime::accessibility_action` without crossing a `RefCell` borrow (`rune-embed-host.md` section 8).

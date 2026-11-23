# Calling `ArdiaCore.h` from Rust with `objc2`

Scope: each class and protocol in `ArdiaCore.h`, the ownership rules, and the three delegate protocols. Research only. No code.

## Source and pin

- Ardia: `551d3c13c1c9ede130724ea9f1056d4b586fbe55` (`git rev-parse HEAD` in the read-only clone). Short name `ardia@551d3c1`.
  - `H:` = `engine/ardiacore/src/ardia/core/public/ArdiaCore.h` (417 lines; the last line, 417, is `NS_ASSUME_NONNULL_END`).
  - `C:` = `engine/ardiacore/src/ardia/core/` (the `.mm` implementation).
  - `HOST:` = `packages/ardiacore/ios/` (today's Swift/ObjC++ host).
- `objc2` 0.6.5, `block2` 0.6.2 (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`). Rune's `Cargo.lock` also pulls `objc2` 0.5.3 and 0.6.5 (`Cargo.lock:2627-2638` in the Rune clone at `ada31dc`). Ria picks 0.6.x. Cites for `objc2` use `objc2/` for that registry directory.
- Anything without a line cite is `[INFERENCE]`.

## 1. How the header is built

- Header imports `AppKit` and is wrapped in `NS_ASSUME_NONNULL_BEGIN/END` (`H:1`, `H:5`, `H:417`). Unmarked pointers are non-null. `nullable` marks the exceptions. Rust maps non-null to `&T` or `Retained<T>` and `nullable` to `Option<..>`.
- Classes that cross the framework boundary carry `ARDIACORE_EXPORT` (default visibility, `H:3`). Five classes have it: `ArdiaCoreEngine` (`H:53-54`), `ArdiaCoreProfile` (`H:115-116`), `ArdiaCoreWindow` (`H:180-181`), `ArdiaCoreTab` (`H:274-275`), `ArdiaCorePageView` (`H:408-409`).
- The framework exports `_ArdiaCoreMain`, `_OBJC_CLASS_$_ArdiaCore*` and `_OBJC_METACLASS_$_ArdiaCore*` (`C:ardiacore.exports:1-3`).
- The framework is Chrome's own, named `Chromium Framework.framework` (`docs/ardiacore-spike.md:36-39`, `docs/engine-build.md:9`). The app stages `out/Release_GN_arm64/Chromium Framework.framework` (`docs/engine-build.md:140`).
- The header has 180 method lines in total (`grep -c '^[-+] (' ArdiaCore.h`).

How Rust reaches the classes:

1. Link the framework (`-framework "Chromium Framework"` or an explicit path) or `dlopen` it. Then `AnyClass::get(c"ArdiaCoreEngine")` finds the class. `extern_class!` binds a Rust type to a class by name (`objc2/src/macros/extern_class.rs:175`).
2. `objc2` is the right layer because the header is pure ObjC with no C++ in it. [INFERENCE] No `bindgen` step is needed for it.
3. Ardia's own C bridge finds engine exports with `dlsym(RTLD_DEFAULT, name)` (`HOST:ArdiaEngineBridge.mm:49-51`). That is for `ac_*` C symbols, not for these classes. See `engine-threads.md`.

## 2. Type mapping

| ObjC | Rust with `objc2` | Rule |
| --- | --- | --- |
| class | `extern_class!` (`objc2/src/macros/extern_class.rs:175`) | One Rust type per class. Mark `MainThreadOnly` (see section 5). |
| category | `extern_methods!` block on the same type (`objc2/src/macros/extern_methods.rs:213`) | `ArdiaCoreEngine (IdleProfiles)` at `H:103`, `(Tracing)` at `H:109`; `ArdiaCoreProfile (Icons)` at `H:137`; `ArdiaCoreWindow (Picture)` at `H:219`; `ArdiaCoreTab (Navigation)` `H:327`, `(Page)` `H:341`, `(Commands)` `H:356`, `(Media)` `H:373`, `(Extensions)` `H:390`. |
| protocol Ardia calls | `extern_protocol!` (`objc2/src/macros/extern_protocol.rs:156`) | Used as `&ProtocolObject<dyn P>`. |
| protocol Rust implements | `define_class!` (`objc2/src/macros/define_class.rs:446`) with `impl P for Class` | Rust owns the object and passes it as `ProtocolObject::from_ref` (`objc2/src/runtime/protocol_object.rs:88`) or `from_retained` (line 112). |
| `nullable T*` | `Option<&T>` / `Option<Retained<T>>` | |
| `BOOL` | `objc2::runtime::Bool` in block signatures | Block arguments require `Encode`, which Rust `bool` does not implement. Convert received block values with `as_bool()`. For direct method arguments and results, `extern_methods!` handles the conversion. See the [Mac host bindings](../crates/ria-host-mac/src/core/bindings.rs). |
| `NSString*` | `NSString` from `objc2-foundation` | |
| `NSDictionary<NSString*, id>*` | `NSDictionary<NSString, AnyObject>` | Untyped JSON-like payload. No schema in the header (`H:19, 23, 26, 29, 36, 43, 46`). |
| block `void (^)(...)` | `&block2::Block<dyn Fn(..)>`, built with `RcBlock::new` (`block2-0.6.2/src/rc_block.rs:38`) | Escaping: Ardia copies the block and keeps it (see section 4). |
| `const char* _Nonnull* _Nonnull` | `*const *const c_char` | `runWithArgc:argv:delegate:` at `H:55-57`. |
| class property (`@property(class)`) | associated function | `sharedEngine` `H:58`, `videoUpscalingState` `H:76`, `testExternalLaunches` `H:86`, `desktopCaptureAudio` `H:93`, `pagesPickDisplayMedia` `H:94`, `isTracing` `H:112`. |
| `NSRect`, `NSSize`, `CGImageRef` | `objc2-foundation` types, `objc2-core-graphics` | `initWithContentRect:` `H:185`, `devToolsLayoutForSize:devTools:page:` `H:363-365`, `copyPicture:` `H:375`. |

## 3. Classes

### `ArdiaCoreEngine` (`H:53-101`, plus `H:103-113`)

- `+runWithArgc:argv:delegate:` (`H:55-57`) starts Chromium. In `C:ardiacore_engine.mm:153-173` it:
  - scans `argv` for `--type=`; a helper process goes straight to `ChromeMain` (lines 157-164);
  - sets `g_delegate = delegate` and `g_engine = [[ArdiaCoreEngine alloc] init]` (lines 166-167);
  - calls `ChromeMain(argc, argv)` and returns only when it ends (line 170);
  - sets `g_delegate = nil` after it (line 171).
- `+sharedEngine` (`H:58`) returns `g_engine` (`C:ardiacore_engine.mm:175-177`).
- `defaultProfile` (`H:60`) returns `wrapperFor:` the last used loaded profile, or `nil` (`C:ardiacore_engine.mm:179-182`). The header says non-null, but the code can return `nil`. Rust must treat it as `Option`.
- `-loadProfile:completion:` (`H:61-62`, `C:ardiacore_engine.mm:184-209`) can call `completion(nil)` at once (lines 196-200) or later after the profile loads or after a dying profile is gone (lines 189-194). The header types the completion argument `nullable`.
- `-quit` (`H:63`) calls `QuitEngine()` (`C:ardiacore_engine.mm:211-213`).
- The class methods (`resolvePermission`, `startCasting`, `selectDevice`, ...) are static functions that act on global state (`H:65-93`).
- `-deleteProfile:completion:` and `-releaseProfile:` are at `H:82-84`.

### `ArdiaCoreProfile` (`H:115-135`)

- Read-only `name`, `path`, `offTheRecord`, `destroyed` (`H:117-120`).
- Settings properties `passwordsOffered`, `addressesAutofilled`, `cardsAutofilled`, `preloadPages` (`H:128-131`) are read/write.
- Rust never creates one. It gets it from the engine or from a completion block.

### `ArdiaCoreWindow` (`H:180-217`)

- `-initWithContentRect:` creates the window (`H:185`); it builds a C++ `WindowHost` (`C:ardiacore_window.mm:64-69`).
- Read-only `window` (the `NSWindow`) and `hostView` (`H:186-187`). Class helpers: `+embeddedRootOfWindow:` (`H:182`, `C:ardiacore_window.mm:47-51`), `+contentViewClass` (`H:183`, returns `BridgedContentView`, `C:ardiacore_window.mm:54-55`), `+isChromeFramelessWindow:` (`H:184`).
- Tab control: `openTab:profile:foreground:` (`H:190`), `activateTab:` (`H:197`), `adoptTab:`/`canAdoptTab:` (`H:206-207`), `restoreTab:`, `duplicateTab:`, `placeTab:index:pinned:` (`H:208-214`), `close` (`H:203`, `C:ardiacore_window.mm:376-378` calls `_host->Close()`).
- `delegate` is `weak, nullable` (`H:188`).

### `ArdiaCoreTab` (`H:274-325`, categories to `H:394`)

- Read-only state: `view` (non-null `NSView`, `H:276`), `profile` (`weak`, `H:277`), `tabId`, `url`, `title`, `loading`, `progress`, `favicon`, `canGoBack`, `canGoForward`, `closed` (`H:278-286`).
- `delegate` is `weak, nullable` (`H:287`).
- Navigation: `loadURL:userInitiated:` (`H:332`), `goBack`, `goForward`, `reload`, `stop`, `goToOffset:` (`H:328-333`).
- Page scripting: `evaluate:completion:` gives JSON in a block (`H:343`); `callPage:json:` (`H:345`).
- Devtools: `devToolsView` (`H:362`), `devToolsLayoutForSize:devTools:page:` fills two `NSRect*` out parameters (`H:363-365`). Rust passes `*mut NSRect`.

### `ArdiaCorePageView` (`H:408-415`)

- Only class methods. `+isPageView:` (`H:410`), `+setScrollObserver:` (`H:411`), `+takeSwipes:`/`+takesSwipes:` (`H:412-413`), `+isEditingText:` (`H:414`).
- The scroll observer protocol `ArdiaCorePageScrollObserver` has 2 methods (`H:401-406`). It is the fourth protocol in the header but it is not one of the three delegate protocols.

## 4. The three delegate protocols

Rust has to implement all three as `define_class!` types. The rules differ.

| Protocol | Lines | Methods | Optional? | Held by Ardia as | Return values Ardia reads |
| --- | --- | --- | --- | --- | --- |
| `ArdiaCoreEngineDelegate` | `H:12-51` | 16 | **No** (no `@optional`) | **strong**: `id<ArdiaCoreEngineDelegate> __strong g_delegate` (`C:ardiacore_engine.mm:62`) | `engineWindowForNewBrowserOfProfile:type:` `H:16`; `extensionActionPopup` `H:25`; `extensionActionShortcut` `H:28`; `engineIsReservedKeyEvent:` `H:31`; `engineIsReservedKey:...` `H:32` |
| `ArdiaCoreWindowDelegate` | `H:149-178` | 14 | **Yes** (`@optional`, `H:150`) | **weak** (`H:188`) | `windowShouldClose:` `H:170`; `window:preHandleKeyEvent:` `H:176`; `window:handleKeyEvent:` `H:177` |
| `ArdiaCoreTabDelegate` | `H:224-272` | 36 | **Yes** (`@optional`, `H:225`) | **weak** (`H:287`) | `contextMenuItemsForSelection:` `H:257`; `runContextMenu:` `H:265` |

(Method counts: `awk` over each range for lines that start with `- (`.)

### 4.1 Engine delegate

- Required. The engine sends its calls with no `respondsToSelector:` guard, for example `[g_delegate engineDidStart]` (`C:ardiacore_engine.mm:135`), `[g_delegate engineWillShutDown]` (`C:ardiacore_engine.mm:140`) and `[g_delegate engine:g_engine extensionInstallPrompt:... ]` (`C:ardiacore_hooks.mm:111-113`). [INFERENCE] A missing method on the Rust class is an `unrecognized selector` crash. So the Rust class must implement all 16.
- Strong by the engine: Rust may drop its own `Retained` after `runWithArgc`. `runWithArgc` clears `g_delegate` only after `ChromeMain` returns (`C:ardiacore_engine.mm:171`). The Rust object lives for the whole run.
- Calls arrive from the engine on the main thread. Some are posted with `dispatch_async(dispatch_get_main_queue(), ...)` (`C:ardiacore_hooks.mm:111`). See `engine-threads.md`.
- Sync results the engine uses: the four `BOOL`s tell the engine whether the host handled a popup, shortcut or key. A wrong default changes Chromium behaviour. For `engineWindowForNewBrowserOfProfile:type:` returning `nil` means "no window" (`nullable`, `H:16`).
- Dismissal pairs: `permissionRequest:tab:` (`H:18-20`) pairs with `permissionRequestDismissed:` (`H:21`) and `ArdiaCoreEngine +resolvePermission:result:remember:` (`H:65-67`). The `requestId` string ties them.

### 4.2 Window delegate

- Optional and weak. Rust must keep a `Retained` of the delegate object alive for as long as the window lives, or the callbacks stop silently. Weak load in `objc2` is `Weak<T>` from `Retained` (`objc2/src/rc/weak.rs:76`) for any Rust back-pointer.
- Calls are guarded: `[delegate respondsToSelector:@selector(window:showRequestedActive:)]` (`C:ac_browser_window.mm:173`) and `window:autofillPrompt:forTab:` (`C:ac_autofill_prompt.mm:157`). A partial implementation is safe.
- Close is two-step: `windowShouldClose:` can veto (`H:170`), then `windowDidCancelClose:` (`H:171`) if the close is cancelled; `confirmCloseWithDownloads:completion:` takes a block that may run later (`H:173-175`).

### 4.3 Tab delegate

- Optional and weak. `notify:` loads the weak delegate and sends the selector only if it responds (`C:ardiacore_tab.mm:319-324`).
- 36 methods. The commonly needed ones are title, URL, loading, progress, favicon, navigation state (`H:226-231`), find results (`H:232`), page messages (`H:233-238`), `tabWillClose:` (`H:239`) and `rendererGone:` (`H:240`).
- Some calls are re-entrant. `DeferWhileAnyNotifies(...)` makes `closeNow` and `close` wait while the engine is inside a notify (`C:ardiacore_tab.mm:437-441, 468-472`; `C:ac_browser.mm:80-86`). [INFERENCE] A Rust handler can call `tab.close()` inside a callback. The engine will defer it. The `Retained<ArdiaCoreTab>` the handler holds stays valid.
- `contextMenuItemsForSelection:` (`H:257`) and `runContextMenu:` (`H:265`) return values to the engine on the main thread, so they must not block.

## 5. Ownership rules for Rust

1. **Hold `Retained<T>` for every object Rust keeps.** `objc2` `Retained` is an owning pointer (`objc2/src/rc/retained.rs:129`). `Retained::retain` retains a raw pointer (line 428) and `retain_autoreleased` takes the value that a method returned autoreleased (line 463). [INFERENCE] `msg_send!` / `extern_methods!` pick the right one by the ObjC method family; I did not verify the macro source.
2. **All delegate properties are weak or are cleared at the end** (section 4). Rust must own the delegate objects separately: the engine delegate for the whole run, the window and tab delegates for the life of each window and tab.
3. **Profile wrappers** are strong in a global map until the profile is destroyed (`C:ardiacore_profile.mm:46-48, 197-205`). On destroy `profileWillBeDestroyed` erases the entry and clears the weak pointer (`C:ardiacore_profile.mm:209-213`). A `Retained<ArdiaCoreProfile>` in Rust stays valid memory, but `destroyed` (`H:120`) becomes true. `wrapperFor:` returns `nil` for a profile that is dying (`C:ardiacore_profile.mm:194-196`).
4. **A tab wrapper outlives its web contents.** The C++ `TabBridge` holds the `ArdiaCoreTab` strongly (`C:ac_browser.h:386`; created at `C:ac_tab_bridge.mm:43`). The tab keeps a raw `content::WebContents` pointer (`C:ardiacore_tab_internal.h:20`) and sets it to null in `contentsDestroyed` (`C:ardiacore_tab.mm:315-317`). `closed` returns `YES` when the pointer is null (`C:ardiacore_tab.mm:326-329`). Rust must check `closed` and must expect `tabWillClose:` (`C:ac_tab_bridge.mm:154`) before the contents go. [INFERENCE] The wrapper is then a husk: later calls act on `nil` contents and do nothing.
5. **The window owns its host.** `ArdiaCoreWindow` holds a `std::unique_ptr<WindowHost>` (`C:ardiacore_window.mm:43-45`). `WindowHost` holds the owner weakly (`C:ac_browser.h:158`). `close` ends the host (`C:ardiacore_window.mm:376-378`). The `NSWindow` and `hostView` Rust reads are owned by the window object, so keep the `ArdiaCoreWindow` as long as the `NSWindow` is in use. [INFERENCE]
6. **Blocks that Ardia keeps.** `loadProfile:completion:` captures the block into a retry (`C:ardiacore_engine.mm:189-193`) and into `BindOnce` (lines 201-208). So Rust passes a heap block (`RcBlock`), not a stack block, and every closure it captures must be `'static`.
7. **A completion may run before the call returns.** `loadProfile` can call `completion(nil)` in the same stack frame (`C:ardiacore_engine.mm:196-200`). Rust code that holds a `RefCell` across the call will panic on re-entry.
8. **`argv` lives for the whole run.** `runWithArgc` passes `argv` to `ChromeMain` and does not return until the end (`C:ardiacore_engine.mm:170`). Ardia keeps its argument vector in a function-level `static` (`HOST:ArdiaHost+Launch.mm:528-534`). [INFERENCE] Rust must keep `CString`s alive for the whole process.
9. **`runWithArgc` does not return while the app runs.** Chromium runs its own loop: `MessagePumpNSApplication` and `[NSApp run]` (`docs/ardiacore-spike.md:40-42`). The Rust `main` is stuck inside that call. Rust code runs only in delegate callbacks and in blocks posted to the main queue. See `engine-threads.md` for the thread facts.
10. **Main thread only.** `MainThreadMarker::new()` returns `Some` only on the main thread (`objc2/src/main_thread_marker.rs:230-232`). `define_class!` types that are `MainThreadOnly` are not `Send` or `Sync` (`objc2/src/macros/define_class.rs:250-262`). Use this to stop a `Retained<ArdiaCoreTab>` from crossing to a worker thread.

## 6. Today's host, for comparison

- The Swift/ObjC++ host calls `[ArdiaCoreEngine runWithArgc:argc argv:args.data() delegate:engineDelegate]` (`HOST:ArdiaHost+Launch.mm:534`). The delegate class is `ArdiaHostEngineDelegate` (`HOST:ArdiaHostEngineDelegate.mm:35, 111`).
- Window creation in the Swift host: `WindowHost.makeContentView` and `ChromeWindows.makeWindow` (`packages/shell/ios/WindowManager.swift:96-135`). The window gets `ArdiaCoreWindow.contentViewClass` as its content view [INFERENCE from the class helper at `H:183`; I did not read the call site].
- Root embed: `embedRootView:inWindow:` (`HOST:ArdiaChromeWindowHost.mm:377-388`).

## 7. Open risks

### Proof A (Rust starts the engine, opens one window and one tab, loads a page, quits clean)

1. **Linking.** The framework is `Chromium Framework.framework`, staged by the Xcode build (`docs/engine-build.md:140`; `apps/browser/macos/Ardia.xcodeproj/project.pbxproj:137,288-298`). A Rust binary needs the framework at a path the loader finds, with `@rpath` and the helper apps (`Chromium Helper`) beside it. The helper names stay "Chromium" (`docs/ardiacore-spike.md:166`). Work out the bundle layout first. `blaze` is the build tool, so check it can link a framework.
2. **Prebuilt engine.** The plan assumes a prebuilt engine. I did not check whether one exists on this host. Proof A is blocked without it.
3. **Subprocess entry.** The same binary must support the `--type=` path (`C:ardiacore_engine.mm:157-164`) if helpers re-enter the main executable. Today helpers are separate bundles, so [INFERENCE] no Rust runs in them.
4. **Required engine delegate.** 16 methods, no guard (section 4.1). Stub all of them or the first unhandled call crashes.
5. **Window creation order.** `engineWindowForNewBrowserOfProfile:type:` (`H:16`) is how the engine asks the host for a window. Without a real answer a tab has no window. Check `delegate.engineDidStart` before the first call (`C:ardiacore_engine.mm:126-136`).
6. **Clean quit.** `engineWillShutDown` (`H:14`), `engineQuitCancelled` (`H:15`) and `quit` (`H:63`). `runWithArgc` returns the exit code (`C:ardiacore_engine.mm:170-172`). Rust must tear down its own objects before it returns.

### Proof B (Rune `LayerHost` beside the page view)

1. **Page view placement.** `tab.view` is a plain `NSView` (`H:276`). The Rust host adds it to `window.hostView` (`H:187`). Page views are added with `addSubview:page` in the Swift host (`HOST:ArdiaWebView+Tab.mm:109`). Rust must reproduce the same order and autoresize rules.
2. **Key routing.** `engineIsReservedKeyEvent:` and the window's `preHandleKeyEvent:` / `handleKeyEvent:` (`H:31, 176-177`) decide who gets a key. The Rune view and the page view race for first responder.
3. **Page scroll and swipes.** `ArdiaCorePageView` and the scroll observer (`H:401-415`) exist to coordinate swipes with the page. Rune's view must not eat page gestures.
4. **Delegate lifetime.** A dropped delegate object stops callbacks with no error (weak, section 4.2).

### Proof C (Rune layer above the page view, glass below, VoiceOver)

1. **Layer order against `tab.view`.** See `rune-embed-host.md` risk 4. The header gives no API for z-order; `hostView` is a plain `NSView` (`H:187`).
2. **`ArdiaCoreWindow.window` is a Chromium widget window** (`BridgedContentView`, `C:ardiacore_window.mm:47-55`). Chromium's own accessibility code runs in it. A Rune accessibility node next to it may conflict. [INFERENCE] Needs a test.

### Cross-cutting

- **Payload schemas are not in the header.** Every `NSDictionary<NSString*, id>` (`H:19, 23, 26, 29, 36, 43, 46, 49, 250, 252, 254, 261-266, 271`) has keys that only the consumers know. They live in the Swift/ObjC++ host. Phase 1 needs a port of those key lists. Not needed for A, B or C.
- **`objc2` exceptions.** [INFERENCE] If Chromium raises an `NSException`, `objc2` unwinds as a Rust panic only with the `catch-all` feature. I did not check whether the engine raises any.

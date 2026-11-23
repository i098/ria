# Ria animation inventory

Source: Ardia at frozen commit `551d3c13c`, Mac app only. This is the checklist for the animation acceptance rule: every animation must match the original, and a missed animation is a defect.
Paths in rows are relative to the Ardia repo root. Part A covers React components, part B covers state, lib and platform TypeScript, and part C covers native Swift, ObjC and Metal code.
Spec numbers refer to `docs/dia-spec.md`. Each part ends with its own "How I searched" section.

Column `Slice` is the slice that records and proves the row (port plan, section 8).
A part A row belongs to the Phase 4 slice of its component folder.
A part B or part C row belongs to the slice that owns its source file in `rust/MAP.md`.
Slice 4.31 records the reference of each row from Ardia.

---

# Animation inventory A: `apps/browser/src/components/**` and shared ui motion helpers

Source: Ardia repo at commit 551d3c13c (read-only). Scope: TS/TSX under `apps/browser/src/components/**`, plus `apps/browser/src/lib/pressMotion.ts`. `*.test.*` files are skipped. Native-only motion (PeekStage, FieldGlowView, Meadow, PowerUp) is outside this part; this part names only the call sites.

All `path:line` citations are relative to the Ardia repo root. Nothing was run; every value comes from reading source. Items not read in source are marked [INFERENCE].

Method: rows A-001..A-031 (ui helpers) were written from direct reads. Rows A-032 onward came from four read-only sub-inventories (chrome, sidebar, content, onboarding slices). Each of them read every file with a regex hit. An automatic check confirmed that every cited file exists, and that the first cited line of each row holds motion code (`setValue`, `Animated`, `spring`, `interpolate`, and similar).

## Shared tokens

|Token|Kind|Resolved value|Source|
|---|---|---|---|---|
|curve.linear|Easing.linear|t|`apps/browser/src/components/ui/motion.ts:7`|
|curve.enter|Easing.out(Easing.cubic)|1-(1-t)^3|`apps/browser/src/components/ui/motion.ts:8`|
|curve.exit|Easing.in(Easing.quad)|t^2|`apps/browser/src/components/ui/motion.ts:9`|
|curve.move|Easing.inOut(Easing.quad)|2t² for t<0.5, else 1-2(1-t)² [INFERENCE: RN Easing.inOut definition, not read]|`apps/browser/src/components/ui/motion.ts:10`|
|curve.pulse|Easing.inOut(Easing.sin)|sine in-out|`apps/browser/src/components/ui/motion.ts:11`|
|curve.outExpo|Easing.out(Easing.exp)|1-2^(-10t) [INFERENCE: RN Easing.exp = 2^(10(t-1)), not read]|`apps/browser/src/components/ui/motion.ts:12`|
|curve.fade|Easing.bezier|cubic-bezier(0.25, 0.46, 0.45, 0.94)|`apps/browser/src/components/ui/motion.ts:13`|
|curve.cssEase|Easing.bezier|cubic-bezier(0.25, 0.1, 0.25, 1)|`apps/browser/src/components/ui/motion.ts:14`|
|curve.cssEaseOut|Easing.bezier|cubic-bezier(0, 0, 0.58, 1)|`apps/browser/src/components/ui/motion.ts:15`|
|duration.fast / base / slow / long|ms|100 / 150 / 250 / 400|`apps/browser/src/components/ui/motion.ts:47-52`|
|period.spinner / pulse|ms per loop|900 / 1100|`apps/browser/src/components/ui/motion.ts:54`|
|buttonMotion.iconPressScale / controlPressScale|scale|0.96 / 0.985|`apps/browser/src/components/ui/motion.ts:79`|
|bouncePressMotion|scale, ms, rest|icon 0.9, control 0.97, row 0.975, tile 0.94, shortestMs 90, restDisplacement 0.0001, restSpeed 0.0001|`apps/browser/src/components/ui/motion.ts:81-89`|
|pressReleaseDelay(heldMs)|ms|max(0, 90 - heldMs)|`apps/browser/src/components/ui/motion.ts:91`|
|bounceDip(scale)|scale|0.96 -> 0.9; 0.985 -> 0.97; other -> same|`apps/browser/src/components/ui/motion.ts:93-97`|
|hoverFade.show / hide|ms + curve|50 ms curve.fade / 200 ms curve.fade|`apps/browser/src/components/ui/motion.ts:101-104`|
|spring({response, damping})|RN spring physics|omega = 2π/response; stiffness = omega²; damping = 2·damping·omega; mass 1|`apps/browser/src/components/ui/motion.ts:128-131`|
|springs.snappy|response 0.2, damping 0.9|stiffness 986.96, damping 56.55, mass 1|`apps/browser/src/components/ui/motion.ts:134, :142`|
|springs.smooth|response 0.4, damping 0.85|stiffness 246.74, damping 26.70, mass 1|`apps/browser/src/components/ui/motion.ts:135, :143`|
|springs.bouncy|response 0.3, damping 0.6|stiffness 438.65, damping 25.13, mass 1|`apps/browser/src/components/ui/motion.ts:136, :144`|
|springs.press|response 0.12, damping 1|stiffness 2741.56, damping 104.72, mass 1|`apps/browser/src/components/ui/motion.ts:137, :145`|
|springs.release|response 0.28, damping 0.55|stiffness 503.55, damping 24.68, mass 1|`apps/browser/src/components/ui/motion.ts:138, :146`|
|liftSpring()|spring choice|bouncy if pressMotion is "bounce" and reduceMotion() === false, else snappy|`apps/browser/src/components/ui/motion.ts:149`|
|POPOVER_MOTION.open / close|ms + curve|150 curve.enter / 100 curve.exit|`apps/browser/src/components/ui/Popover.tsx:60-63`|
|TOAST_FADE.in / out|ms + curve|150 curve.enter / 250 curve.exit|`apps/browser/src/components/ui/Toast.tsx:15`|
|TooltipAbove SHOW_DELAY_MS / FADE_MS / FADE_EASING|ms + curve|500 / 100 / curve.fade|`apps/browser/src/components/ui/TooltipAbove.tsx:8-10`|
|Switch USER_CHANGE_WINDOW_MS|ms|2000|`apps/browser/src/components/ui/Switch.tsx:97`|
|Swatch LIVELY|pt, opacity|restInset 4, hoverInset 3, selectedInset 6, restOpacity 0.85 (swatch size 20, inset 2, ring 2 at tokens.ts:348)|`apps/browser/src/components/ui/Swatch.tsx:9`|
|PINCH_SPRING|spring|snappy + overshootClamping true, restDisplacement 0.001, restSpeed 0.01, native|`apps/browser/src/components/ui/PinchArea.tsx:7`|
|PINCH_OPEN_AT / PINCH_CLOSE_AT / SPREAD_TO_TAB|magnification|0.3 / -0.25 / 0.4|`apps/browser/src/components/ui/pinch.ts:1-3`|
|cssMotion CURVE_STEPS / SPRING_STEPS / SPRING_REST / SPRING_LONGEST_MS / REACH_STEPS|samples, thresholds|32 / 60 / 0.001 / 10000 / 40|`apps/browser/src/components/ui/cssMotion.ts:6-10`|
|pressMotion mode|"still" | "bounce"|default "still"; `setPressMotion` notifies subscribers|`apps/browser/src/lib/pressMotion.ts:1-19`|
|reduceMotion()|boolean | null|null until AccessibilityInfo answers; then live value|`apps/browser/src/components/ui/reduceMotion.ts:3-11`|

Reduce Motion note: `reduceMotion()` is `null` until the first answer (`apps/browser/src/components/ui/reduceMotion.ts:3`). `usePressScaleMotion` and `useHoverFill` test `reduceMotion() !== false` (`apps/browser/src/components/ui/motion.ts:199`, `:222`), so they do not animate until the answer arrives. `useSlidingThumb` tests truthiness (`apps/browser/src/components/ui/SlidingThumb.tsx:18`), so it animates while the value is `null`. `liftSpring` uses bouncy only when the value is exactly `false` (`apps/browser/src/components/ui/motion.ts:149`).

## 1. Shared ui components and helpers (`apps/browser/src/components/ui/`)

|ID|Surface|Trigger|Properties animated|Duration ms|Curve / spring|Delay / loop|Native driver|Source|Spec| Slice |
|---|---|---|---|---|---|---|---|---|---|---|
|A-001|Press scale helper `usePressScaleMotion` (every press call site below)|onPressIn / onPressOut; off when `enabled` is false|transform scale 1 -> pressedScale -> 1|spring (no fixed ms)|still mode: `springs.snappy` k=986.96 c=56.55 m=1 to 0.96 (icon) or 0.985 (control). bounce mode: `springs.press` k=2741.56 c=104.72 to `bounceDip` (0.96->0.9, 0.985->0.97, others unchanged), then `springs.release` k=503.55 c=24.68; rest thresholds 0.0001|bounce: release waits max(0, 90 - heldMs) ms (`motion.ts:91`). Reduce Motion: stop, set 1, no motion (`motion.ts:199-202`)|yes|apps/browser/src/components/ui/motion.ts:151-214 (springs :173, :177)|docs/dia-spec.md:350-356 (§Press and hover feedback, Press); :368 (Reduce Motion)| 4.1 |
|A-002|Squish wrapper `useSquish` / `<Squish>` (sidebar rows and tiles)|press, only when `pressMotion` is "bounce"|transform scale on wrapper `Animated.View`|spring|same as press helper: row 0.975, tile 0.94 (no `bounceDip` change); `springs.press` then `springs.release`|90 ms minimum hold|yes|apps/browser/src/components/ui/Squish.tsx:9-17; scales apps/browser/src/components/ui/motion.ts:84-85|docs/dia-spec.md:352-355 (§Press and hover feedback: squish off by default)| 4.1 |
|A-003|Hover highlight `useHoverFill` / `<HoverFill>`|`shown` true/false (hover or pressed)|opacity 0 <-> 1 of a fill layer|show 50, hide 200 (`hoverFade`)|curve.fade = bezier(0.25, 0.46, 0.45, 0.94) both ways|`instant` or Reduce Motion: set at once, no fade. Layer is not created until first shown (lazy)|yes|apps/browser/src/components/ui/motion.ts:216-232 (timing :227); apps/browser/src/components/ui/HoverFill.tsx:5-9; tokens apps/browser/src/components/ui/motion.ts:101-104|docs/dia-spec.md:362-368 (§Press and hover feedback, Hover)| 4.1 |
|A-004|Hover cross-fade `<HoverSwap>` (controls with a rest fill)|`shown` true/false|rest fill opacity 1 -> 0 (interpolate, :13) while hover fill 0 -> 1|show 50, hide 200|curve.fade bezier(0.25, 0.46, 0.45, 0.94)|created from mount (`fromMount`); `instant` skips the fade|yes|apps/browser/src/components/ui/HoverFill.tsx:11-22|docs/dia-spec.md:365-366 (§Hover: cross-fades from rest fill)| 4.1 |
|A-005|Icon button plate (`Button variant="icon"`)|hover, press, `active`|HoverFill opacity; colour swaps hover -> pressed at once|50 / 200|curve.fade|`active` -> instant. No press scale in this view (no `usePressScaleMotion` in `Button.tsx:121-205`)|yes|apps/browser/src/components/ui/Button.tsx:163|docs/dia-spec.md:351 says the glyph dips to 0.96; code does not scale it [mismatch]| 4.1 |
|A-006|Text button (`Button variant="text"`)|press; hover|press scale 0.985 (plate or label); HoverFill on `toolbar` plate|spring; fill 50/200|press helper only when pressMotion is "bounce" (`:217`); fill curve.fade|still mode: no scale|yes|apps/browser/src/components/ui/Button.tsx:217 (scale), :243 (HoverFill)|docs/dia-spec.md:351 says 0.985 on snappy; code scales only in bounce mode [mismatch]| 4.1 |
|A-007|Stepper button|press; hover|glyph scale 0.96 (`ScaledGlyph` :12-14); HoverSwap control -> hover/pressed fill|spring; fill 50/200|press helper (icon scale); curve.fade|-|yes|apps/browser/src/components/ui/ButtonKinds.tsx:30, :46, :47|docs/dia-spec.md:351, 365| 4.1 |
|A-008|Pill button (chip and prompt pill)|press; hover|plate scale 0.985; HoverSwap rest -> hover fill|spring; fill 50/200|press helper (control scale); curve.fade|-|yes|apps/browser/src/components/ui/ButtonKinds.tsx:61, :84, :86, :111, :112|docs/dia-spec.md:351, 365| 4.1 |
|A-009|Close button (panel / art / hud looks)|press; hover|glyph scale 0.96; panel look HoverFill, art look HoverSwap, hud look scale only|spring; fill 50/200|press helper (icon scale); curve.fade|-|yes|apps/browser/src/components/ui/ButtonKinds.tsx:128, :134, :135, :148, :149, :163|docs/dia-spec.md:351, 365| 4.1 |
|A-010|Push button (default `Button`)|press (not disabled); hover|plate scale 0.985; HoverSwap (not for plain or primary kinds)|spring; fill 50/200|press helper (control scale); curve.fade|-|yes|apps/browser/src/components/ui/ButtonKinds.tsx:181, :211, :213|docs/dia-spec.md:351, 365| 4.1 |
|A-011|SearchField clear (x) glyph|press|scale 0.96|spring|press helper (icon scale)|only while field has text and is not `bare`|yes|apps/browser/src/components/ui/Field.tsx:53, :104|-| 4.1 |
|A-012|PopUp (select) control|press|plate scale 0.985|spring|press helper (control scale)|off for native-form look and disabled|yes|apps/browser/src/components/ui/Field.tsx:217, :256|-| 4.1 |
|A-013|Checkbox (box and ring looks)|press|scale 0.96 of box / ring frame|spring|press helper (icon scale)|off without `onChange` or when disabled|yes|apps/browser/src/components/ui/Field.tsx:306, :320, :333|-| 4.1 |
|A-014|Ring checkbox mark (`RingCheck`)|`on` changes|opacity 0 <-> 1, scale 0.6 <-> 1 of the filled disc|150 (duration.base)|curve.enter = Easing.out(cubic), 1-(1-t)^3|-|yes|apps/browser/src/components/ui/Field.tsx:362 (timing), :389-390|-| 4.1 |
|A-015|Settings `Row` with `onPress`|hover|HoverFill opacity (radius 0)|50 / 200|curve.fade|-|yes|apps/browser/src/components/ui/Row.tsx:26|docs/dia-spec.md:363 (settings rows)| 4.1 |
|A-016|`ListRow`|hover; selected|HoverFill opacity; selected -> instant|50 / 200|curve.fade|selected: no fade|yes|apps/browser/src/components/ui/ListRow.tsx:58|docs/dia-spec.md:363-364 (list rows; selected drops highlight)| 4.1 |
|A-017|Sliding thumb `useSlidingThumb` / `<SlidingThumb>` (Segmented, ProfileSwitcher)|`selected` key changes after first placement|thumb translateX to slot x; thumb width to slot width|spring|`springs.snappy` k=986.96 c=56.55 for both|first placement and Reduce Motion: set at once (`:18-23`)|x: yes; width: no (JS)|apps/browser/src/components/ui/SlidingThumb.tsx:25 (x), :26 (width), :56-57 (view)|-| 4.1 |
|A-018|Segmented control segment|press|segment scale 0.985|spring|press helper (control scale)|-|yes|apps/browser/src/components/ui/Segmented.tsx:33, :56 (thumb :17, :21)|-| 4.1 |
|A-019|Switch / `Toggle`|value changes (press, or external change)|on-fill opacity 0 <-> 1 (:77); knob translateX 0 <-> travel (:89). Travel: panel 30-14-3=13; form 32-15-3=14; native form 26-13-2=11|150 (duration.base)|curve.enter = Easing.out(cubic), 1-(1-t)^3|Form look sets `animateExternalChanges=false`: an external change jumps unless within 2000 ms of a press (`:56`, `:97`, `:137`). Panel look animates all changes (default true, `:25`)|yes|apps/browser/src/components/ui/Switch.tsx:46 (timing); looks :127 (panel), :138 (form)|-| 4.1 |
|A-020|Theme swatch (`Swatch animated`)|`selected`; hover|ring opacity 0 <-> 1 (:48); disc size 12 (rest) or 14 (hover) -> 8 (selected) (:45). Hover changes size and disc opacity 0.85 -> 1 at once|150 (duration.base)|curve.enter = Easing.out(cubic), 1-(1-t)^3|-|no (width/height)|apps/browser/src/components/ui/Swatch.tsx:43; LIVELY :9 (swatch tokens apps/browser/src/components/ui/tokens.ts:348)|-| 4.1 |
|A-021|Popover open/close (`usePopoverMotion` + `usePopoverMotionStyle`, used by `<Popover>`)|`closing` false/true (`ready` gate)|opacity 0 <-> 1; scale 0.94 <-> 1; translateY -4 (or +4 upward) <-> 0; origin top left / top right|open 150 (duration.base); close 100 (duration.fast)|open curve.enter 1-(1-t)^3; close curve.exit = Easing.in(quad), t^2|`onClosed` (unmount) after close finishes|yes|apps/browser/src/components/ui/Popover.tsx:92 (timing), :102-106 (style), :60-63 (POPOVER_MOTION), :138-139|docs/dia-spec.md:152 (§Panels from the address field: geometry only, no motion values)| 4.1 |
|A-022|Dialog (`animated` prop)|mount|scrim opacity 0 -> 1 (:76); surface opacity 0 -> 1 and scale 0.96 -> 1 (:79)|150 (duration.base)|curve.enter = Easing.out(cubic), 1-(1-t)^3|no exit motion; not animated when `animated` is false or `place` is set (`:60-73`). `Sheet` (:107-113) never sets `animated`|yes|apps/browser/src/components/ui/Dialog.tsx:40|-| 4.1 |
|A-023|Toast|mount (`closing` false); `closing` true|opacity 0 <-> 1|in 150 (duration.base); out 250 (duration.slow)|in curve.enter 1-(1-t)^3; out curve.exit t^2|`onClosed` after fade out|yes|apps/browser/src/components/ui/Toast.tsx:39; TOAST_FADE :15|docs/dia-spec.md:359-361 (§Press and hover feedback, Toast)| 4.1 |
|A-024|TooltipAbove|`active` with text for 500 ms|opacity 0 -> 1|100 (duration.fast)|curve.fade bezier(0.25, 0.46, 0.45, 0.94)|show delay 500 ms (`SHOW_DELAY_MS` :8, timer :21); hide is instant (:32-34)|yes|apps/browser/src/components/ui/TooltipAbove.tsx:36|-| 4.1 |
|A-025|Eased colour `useEasedColor` (toolbar, strip, bars)|colour target changes|interpolated colour from old to new (null -> same hex with alpha 00)|default 150 (`durations.base`), caller may pass another|curve.enter 1-(1-t)^3|-|no (colour)|apps/browser/src/components/ui/toolbarColors.ts:77-92 (timing :89)|docs/peek-motion.md:34 (§Design: toolbar colour fade)| 4.1 |
|A-026|Presence fade `usePresence` (helper)|value becomes non-null / null|opacity `appear` 0 <-> 1|caller's `show` / `hide` Fade|caller's curve; optional delay|keeps the last value mounted until hide finishes (:120)|yes|apps/browser/src/components/ui/motion.ts:108-124 (timing :119)|-| 4.1 |
|A-027|Looped legs `nativeLoop` (helper)|caller `start()`|one Animated.timing 0 -> 1 with a piecewise easing built from legs|sum of leg ms|each leg's easing, default curve.linear (`legsEasing` :58-71)|`Animated.loop`, iterations default -1 (infinite); `isInteraction: false`|yes|apps/browser/src/components/ui/motion.ts:73-77|-| 4.1 |
|A-028|Lift spring `liftSpring()` (helper)|caller|-|spring|`springs.bouncy` k=438.65 c=25.13 when pressMotion is "bounce" and Reduce Motion is false; else `springs.snappy` k=986.96 c=56.55|-|caller|apps/browser/src/components/ui/motion.ts:149|docs/dia-spec.md:356 (New Tab card lift on snappy)| 4.1 |
|A-029|Pinch settle `pinchSettle` / `<PinchArea>` (Little Ardia)|trackpad magnify gesture (native `MagnifyArea`) -> `pinchStep`|caller values follow fingers, then spring to target|spring|`springs.snappy` k=986.96 c=56.55, overshootClamping true, restDisplacement 0.001, restSpeed 0.01|outcome: open if magnification >= 0.3, close if <= -0.25, else settle (`pinch.ts:1-2`, `:8-12`, `:36-47`); follow maps spread over 0.4 (`pinch.ts:16-24`); Reduce Motion skips begin/follow/settle|yes|apps/browser/src/components/ui/PinchArea.tsx:7 (PINCH_SPRING), :10 (parallel springs), :16|-| 4.1 |
|A-030|Native spinner `ActivitySpinner`|mounted in a window|ring layer rotation.z one full turn|1880 per turn|CAMediaTimingFunction linear|repeatCount infinity|native (Core Animation)|packages/shell/ios/ActivitySpinner.swift:60-67; export apps/browser/src/components/ui/native.ts:2|-| 5.7 |
|A-031|CSS curve export `cssEasing` / `cssSpring` / `timeToReach` (helper, used by onboarding intro page)|build time of a web page style|-|cssSpring: simulates the spring by 1 ms steps until \|1-x\| < 0.001 and \|v\| < 0.001 (max 10000 ms)|cssEasing samples a curve at 32 steps into CSS `linear(...)`; cssSpring uses 60 steps; timeToReach bisects 40 times|-|n/a (CSS)|apps/browser/src/components/ui/cssMotion.ts:6-10, :14-17, :37-47, :49-58|-| 4.1 |

## 2. Window chrome: tabstrip, window, split, littleArdia, peek, find, omnibox, toolbar

Slice notes:

I found 71 animation rows in 53 files. Every file with a regex hit has a row or a reason in the coverage list. Peek has no React animation in this slice: `PeekStage` animates natively. Several rows depend on values computed in shared helpers outside the slice, so they are marked [INFERENCE].

|ID|Surface|Trigger|Properties animated|Duration ms|Curve / spring|Delay / loop|Native driver|Source|Spec| Slice |
|---|---|---|---|---|---|---|---|---|---|---|
|A-032|Strip FadeIn overlay (used by PinnedDock:111, TabChip:117)|Mount|opacity 0→1|100 (FADE_IN_MS, apps/browser/src/components/tabstrip/tokens.ts:76)|curve.enter|none|no|apps/browser/src/components/tabstrip/FadeIn.tsx:9|-| 4.7 |
|A-033|Group member tuck fade|`shown` changes|opacity 0↔1 (gated by `useGated` :76)|120 (TUCK_FADE_MS, tokens.ts:77)|curve.enter|none|no|apps/browser/src/components/tabstrip/GroupMember.tsx:68|docs/dia-spec.md:840 (§Groups, "–" fade ~0.12 s)| 4.7 |
|A-034|Group member slide|group `open` value (StripGroup glide)|`left` from start to start+offset; width clipped by `clip`|180 (from StripGroup)|toggleEase (cubic out)|none|no|apps/browser/src/components/tabstrip/GroupMember.tsx:71,74-75|docs/dia-spec.md:834 (§Groups, Expand / collapse)| 4.7 |
|A-035|Incoming tab gap|`open` (a tab drags in from another window)|width 0→w, marginRight −GAP→0|100 (INCOMING_GAP_MS, tokens.ts:75)|curve.enter|none|no|apps/browser/src/components/tabstrip/IncomingTab.tsx:48|docs/dia-spec.md:904 (§Dragging tabs, tab-wide gap 0.12 s)| 4.7 |
|A-036|New Tab button reorder shift|reorder registry (`registerSlot` :41)|translateX|spring|SLIDE k=8100 c=180 m1, rest 0.25/2 (tokens.ts:82)|none|no|apps/browser/src/components/tabstrip/NewTabButton.tsx:41,51 → apps/browser/src/components/tabstrip/stripReorder.ts:36|docs/dia-spec.md:908 (§Dragging tabs, Reordering)| 4.7 |
|A-037|Pinned dock grow|tab crosses into the dock (`growDock`)|dock box width, opacity (0→1 over PINNED_CELL/2)|spring|SLIDE k=8100 c=180|none|no|apps/browser/src/components/tabstrip/dockCross.ts:12; apps/browser/src/components/tabstrip/PinnedDock.tsx:40-41|docs/dia-spec.md:925 (§Dragging tabs, In the strip)| 4.7 |
|A-038|Dock tile hold|drag dx past limit|interpolated offset (follows the drag)|follows gesture|-|none|no|apps/browser/src/components/tabstrip/dockCross.ts:114|-| 4.7 |
|A-039|Pinned tile hover/press fill|hover / press|opacity of the fill|show 50 / hide 200; instant when active|hoverFade (curve.fade)|none|yes|apps/browser/src/components/tabstrip/PinnedFace.tsx:66|docs/dia-spec.md:350 (§Press and hover feedback)| 4.7 |
|A-040|Strip slot reorder (all chips)|neighbour passed during drag|`shift` translateX|spring|SLIDE k=8100 c=180|none|no|apps/browser/src/components/tabstrip/stripReorder.ts:36|docs/dia-spec.md:908 (§Dragging tabs, Reordering)| 4.7 |
|A-041|Strip drag lift-out|tab dragged below strip + LIFT_SLOP|opacity 1→0, marginRight 0→−(w+gap)|150 (LIFT_MS, tokens.ts:73)|curve.enter|none|no|apps/browser/src/components/tabstrip/useStripDrag.ts:34,254-255|docs/dia-spec.md:874 (§Dragging tabs, Leaving the strip)| 4.7 |
|A-042|Strip drag follow|pan move|translateX = dx+shift (setValue per move)|follows gesture|-|none|no|apps/browser/src/components/tabstrip/useStripDrag.ts:118,256|docs/dia-spec.md:873| 4.7 |
|A-043|Strip drag drop settle|release / cross-strip landing|dx to its slot|spring|SLIDE k=8100 c=180|none|no|apps/browser/src/components/tabstrip/useStripDrag.ts:88|docs/dia-spec.md:917-918 (§Dragging tabs)| 4.7 |
|A-044|Strip drag auto-scroll|drag near the strip edge|ScrollView offset, one step per rAF|per frame|-|rAF repeats while in the edge zone|n/a|apps/browser/src/components/tabstrip/useStripDrag.ts:138|-| 4.7 |
|A-045|Strip reveal of the active tab|activeId changes|ScrollView `scrollTo({animated:true})` after rAF|native scroll|native ScrollView|rAF; retries after USER_SCROLL_QUIET_MS|n/a|apps/browser/src/components/tabstrip/StripPage.tsx:218,222,131|docs/dia-spec.md:851 (§Many tabs)| 4.7 |
|A-046|Strip left clip on scroll|scroll offset crosses 0.5|marginLeft/left clip (setValue, a step, not an animation)|instant|-|-|no|apps/browser/src/components/tabstrip/StripPage.tsx:189|-| 4.7 |
|A-047|Group expand/collapse|chip toggle or member shift|`open` 0↔1, `size` width|180 (TOGGLE_MS, stripToggle.ts:6)|toggleEase 1−(1−t)³ (stripToggle.ts:7)|parallel; reduceMotion → setValue|no|apps/browser/src/components/tabstrip/StripGroup.tsx:84,96|docs/dia-spec.md:834 (§Groups; Dia uses a spring r0.30 d0.82, ours is a timing)| 4.7 |
|A-048|Group divider/members fade|`open` value|opacity, tail left (`Animated.add` size)|180|toggleEase|none|no|apps/browser/src/components/tabstrip/StripGroup.tsx:105,125-127,141,167|docs/dia-spec.md:834| 4.7 |
|A-049|Group fill width|`size` value|width|180|toggleEase|none|no|apps/browser/src/components/tabstrip/StripGroup.tsx:236-242|-| 4.7 |
|A-050|Chip width change|width change while `animate()`|width|180 (TOGGLE_MS)|toggleEase|reduceMotion → setValue|no|apps/browser/src/components/tabstrip/TabChip.tsx:147|docs/dia-spec.md:834| 4.7 |
|A-051|Chip band colour|site colour changes|backgroundColor interpolation|150 (durations.base default)|curve.enter|none|no|apps/browser/src/components/tabstrip/TabChip.tsx:137 (useEasedColor, apps/browser/src/components/ui/toolbarColors.ts:89)|docs/peek-motion.md:34 (§Design, toolbar fades 150 ms)| 4.7 |
|A-052|Chip cross-strip hide|`drag.crossed`|opacity 0/1 (a step)|instant|-|-|no|apps/browser/src/components/tabstrip/TabChip.tsx:113|-| 4.7 |
|A-053|Mute toggle press|press in/out|scale|spring|still mode: snappy k=986.96 c=56.55 to 0.96; bounce mode: press→0.9, release spring|90 ms min hold (bounce)|yes|apps/browser/src/components/tabstrip/TabChip.tsx:238|docs/dia-spec.md:351 (§Press and hover feedback)| 4.7 |
|A-054|Tab chip hover/press fill|hover / press|fill opacity|50 / 200|hoverFade|none|yes|apps/browser/src/components/tabstrip/TabChip.tsx:310|docs/dia-spec.md:350| 4.7 |
|A-055|Top-strip peek panel|hover at the top edge|opacity = slide, translateY −PEEK_HEIGHT−8→0|in 150 (PEEK_IN_MS); out 150 (SLIDE_MS)|in easeOutCubic; out curve.enter|hide after PEEK_CLOSE_DELAY_MS|yes|apps/browser/src/components/tabstrip/TopTabStrip.tsx:126,164-165 → apps/browser/src/components/window/usePeek.ts:30,33|docs/dia-spec.md:309 (§Arc note: hiding the sidebar)| 4.7 |
|A-056|Window drop glow|another window's tab drag is over this window|opacity 0↔1|150 (DROP_GLOW_MS, tokens.ts:74)|curve.enter|none|yes|apps/browser/src/components/tabstrip/WindowDropGlow.tsx:17|docs/dia-spec.md:893 (§Dragging tabs, Outside the window)| 4.7 |
|A-057|Strip hover card|chip hover (via `useStripCardHover`)|opacity|100 (SIDEBAR_MOTION.hoverCardFade, apps/browser/src/components/sidebar/tokens.ts:155)|curve.fade|open/close delays live in apps/browser/src/components/sidebar/hoverMotion|yes|apps/browser/src/components/tabstrip/StripHoverCard.tsx:38,45|docs/dia-spec.md:230 (§Tab hover card motion)| 4.6 |
|A-058|Sidebar dock slide|sidebar open/close (`sidebarSlide` switch on, no reduceMotion)|`dock` 0↔1 → card edge, top, stripY, translateX slideOut|200 (DOCK_SLIDE, apps/browser/src/components/window/tokens.ts:95)|bezier(0.42,0,0.58,1)|"jump" → setValue|no|apps/browser/src/components/window/SidebarDock.tsx:91,135-143,295|docs/dia-spec.md:280 (§Hiding and showing the sidebar; Dia uses a spring r0.215 d0.79)| 4.5 |
|A-059|Sidebar arc hide|hide with the address bar in the sidebar|`dock`→0|100 (ARC_HIDE_MS)|curve.enter|-|no|apps/browser/src/components/window/SidebarDock.tsx:90|docs/dia-spec.md:321 (§Arc note)| 4.5 |
|A-060|Sidebar peek panel|pointer at the window edge|translateX (−12·out → w·out), opacity = slide (non-arc)|in 150 (PEEK_IN_MS); out 150 (arc: ARC_PEEK_OUT_MS) or 150 (SLIDE_MS)|in easeOutCubic (dockMotion.ts:6); arc out easeInOutCubic (:7); else out curve.enter|close delay PEEK_CLOSE_DELAY_MS|yes|apps/browser/src/components/window/SidebarDock.tsx:239,297,323-328 → apps/browser/src/components/window/usePeek.ts:30-33|docs/dia-spec.md:302-315 (§Arc note)| 4.5 |
|A-061|Peek panel backdrop|`peeking`|opacity 0/1 (layout-effect setValue, a step)|instant|-|-|no|apps/browser/src/components/window/SidebarDock.tsx:334|-| 4.5 |
|A-062|Traffic lights follow dock/peek|dock/slide listeners|native traffic-light centre per frame|follows the dock value|-|-|n/a|apps/browser/src/components/window/useDockLights.ts:17,39|docs/dia-spec.md:289-290| 4.5 |
|A-063|Control pill|program control shown|opacity; translateY 8→0; scale 0.96→1|show 150 / hide 150|enter / exit|unmount after hide|yes|apps/browser/src/components/window/ControlPill.tsx:22,32|-| 4.5 |
|A-064|Loading pill show|page loading|opacity 0→1|150|curve.enter|-|yes|apps/browser/src/components/window/LoadingPill.tsx:41|-| 4.5 |
|A-065|Loading pill sweep|while loading|sweep translateX (grow to FULL, wipe to WIPED)|330 + 330|curve.move|sequence + delay 330 rest, restarts on finish (loop); reduceMotion → grow only, once|yes|apps/browser/src/components/window/LoadingPill.tsx:33-35,39|-| 4.5 |
|A-066|Loading pill leave|loading ends|opacity →0, scale →0.55|250|curve.exit|waits for the rest of the current grow (setTimeout)|yes|apps/browser/src/components/window/LoadingPill.tsx:48-53|-| 4.5 |
|A-067|New tab notice|notice live|opacity; translateY −6→0; scale 0.96→1|show 150 / hide 150|enter / exit|unmount after hide|yes|apps/browser/src/components/window/NewTabNotice.tsx:12,27|-| 4.5 |
|A-068|Status bubble|link status text|opacity|show 100 / hide 250|enter / exit|hide delay 180 (apps/browser/src/components/window/tokens.ts:52)|yes|apps/browser/src/components/window/PaneOverlays.tsx:15|-| 4.5 |
|A-069|Status layer shift (auto-hide)|toolbar motion|translateY (bottomStyle)|150|curve.enter|-|yes|apps/browser/src/components/window/PageOverlays.tsx:64 (via apps/browser/src/components/window/TabPane.tsx:171)|-| 4.5 |
|A-070|Profile tint crossfade|profile pager position (native pager event)|per-page opacity|follows gesture / pager settle|Animated.event, native; settle spring in state/windows/profilePager [INFERENCE]|-|yes|apps/browser/src/components/window/ProfileSwipe.tsx:30,72|docs/dia-spec.md:606 (§Profile paging)| 4.5 |
|A-071|Swipe capsule layout|confirm / list / unconfirm|w, h, top, borderRadius|spring|springSpec.snappy k=986.96 c=56.55|stopTogether false|no|apps/browser/src/components/window/SwipeOverlay.tsx:113-118|-| 4.5 |
|A-072|Swipe icon pop|confirmed|icon scale 1↔34/28|spring|snappy|-|yes|apps/browser/src/components/window/SwipeOverlay.tsx:129|-| 4.5 |
|A-073|Swipe confirm wash|confirmed|confirm 0↔1 → wash opacity, icon opacity 0.5→1|150|curve.fade|-|yes|apps/browser/src/components/window/SwipeOverlay.tsx:131,231,266|-| 4.5 |
|A-074|Swipe history list|list presented / folded|list 0↔1 → list opacity, scale 0.9→1, icon fade|250|curve.fade|idle timer LIST_IDLE_MS 300 before present|yes|apps/browser/src/components/window/SwipeOverlay.tsx:149,158,302|-| 4.5 |
|A-075|Swipe list selection|dy selects a row|top, pill translateY|spring|SWIPE_PILL_SPRING r0.27 d0.9 → k≈541.5 c≈41.89|nudge setValue follows the gesture|no|apps/browser/src/components/window/SwipeOverlay.tsx:171-172|-| 4.5 |
|A-076|Swipe dismiss|swipe ends|fade →0, scale →0.01|150|curve.exit|`delay` argument|yes|apps/browser/src/components/window/SwipeOverlay.tsx:181-182|-| 4.5 |
|A-077|Window controls fade (auto-hide)|toolbar `fade` value|opacity row [1,.45,.15,0,0], strip [0,0,1]|150|curve.enter|-|yes|apps/browser/src/components/window/BrowserWindowControls.tsx:56-57; apps/browser/src/components/window/WindowControls.tsx:104|-| 4.5 |
|A-078|Pane group / cover (auto-hide)|toolbar mode|translateY of the group and the cover|150|curve.enter|-|yes|apps/browser/src/components/window/TabPane.tsx:151,180|-| 4.5 |
|A-079|Split drop zone geometry|zone stage / rect change|left, top, width, height|spring|snappy k=986.96 c=56.55|parallel|no|apps/browser/src/components/split/SplitChrome.tsx:262-266|docs/dia-spec.md:898-900 (§Dragging tabs, Ardia: 0.2 s / 0.9 spring)| 4.11 |
|A-080|Split drop zone glow|zone active|backgroundColor → accent|100|curve.enter|-|no|apps/browser/src/components/split/SplitChrome.tsx:270,292|docs/dia-spec.md:884-887| 4.11 |
|A-081|Split drop zone slide|shown / hidden|translateX from off-pane|110 (ZONE_SLIDE_MS)|in enter / out exit|grow stage after a 110 ms setTimeout (:199)|no|apps/browser/src/components/split/SplitChrome.tsx:273,295|docs/dia-spec.md:874-875 (~0.11 s)| 4.11 |
|A-082|Split toast|toast closing|Toast's own motion (ui) [INFERENCE]|ui-owned|ui-owned|auto-close TOAST_MS 2600|ui|apps/browser/src/components/split/SplitChrome.tsx:334|-| 4.11 |
|A-083|Little Ardia pinch follow|trackpad magnify|scale (setValue 0.6…1.04)|follows gesture|-|-|n/a|apps/browser/src/components/littleArdia/LittleArdiaWindow.tsx:117|docs/peek-motion.md:39| 4.12 |
|A-084|Little Ardia pinch settle|pinch ends / open / close|scale→1|spring|PINCH_SPRING (snappy + overshootClamping)|-|yes|apps/browser/src/components/littleArdia/LittleArdiaWindow.tsx:110|-| 4.12 |
|A-085|Little Ardia zoomed corner|`scale`|borderRadius = interp/scale|follows scale|-|-|yes|apps/browser/src/components/littleArdia/LittleArdiaWindow.tsx:104|-| 4.12 |
|A-086|Little Ardia swipe follow|swipe changed|translateX = distance; opacity 1→0 over 280|follows gesture|-|-|yes|apps/browser/src/components/littleArdia/LittleArdiaWindow.tsx:152,138,75|-| 4.12 |
|A-087|Little Ardia swipe close|swipe past 120 pt or 600 velocity|x→280|140 (leaveMs, apps/browser/src/components/littleArdia/tokens.ts:24)|default easing (none given; RN default inOut) [INFERENCE]|reduceMotion → close at once|yes|apps/browser/src/components/littleArdia/LittleArdiaWindow.tsx:143|-| 4.12 |
|A-088|Little Ardia swipe cancel|swipe ended short|x→0|spring|PINCH_SPRING|-|yes|apps/browser/src/components/littleArdia/LittleArdiaWindow.tsx:139|-| 4.12 |
|A-089|Peek stage (Genie / Seamless)|peek open / close / promote / pinch|card, page, scrim (native display link)|native (entrance 150 ms ease-out on non-native hosts)|native PeekMotion.swift|-|native|apps/browser/src/components/peek/PeekOverlay.tsx:197 (`PeekStage`)|docs/peek-motion.md:9 (§Design)| 4.14 |
|A-090|Peek card corner (zoom)|`zoom` prop|borderRadius interp/grown|-|-|-|-|apps/browser/src/components/peek/PeekOverlay.tsx:253-254,293|Not driven: the caller at :237-238 passes no `zoom` (dead path)| 4.14 |
|A-091|Find bar appear|mount|opacity 0→1, translateY −6→0|150 (FIND_APPEAR, apps/browser/src/components/find/tokens.ts:3)|curve.enter|none|yes|apps/browser/src/components/find/FindBar.tsx:38,81|-| 4.13 |
|A-092|Omnibox scope pulse in|scope set|glow 0→1→0.25 (opacity)|100 rise + 400 settle|enter then move|sequence; start waits waitForColor 150 (setTimeout :42); reduceMotion → set 0.25|yes|apps/browser/src/components/omnibox/ScopePulse.tsx:30-34|-| 4.9 |
|A-093|Omnibox scope pulse out|scope cleared|glow→0|150|curve.exit|reduceMotion → set 0|yes|apps/browser/src/components/omnibox/ScopePulse.tsx:48|-| 4.9 |
|A-094|Auto-hide toolbar glide|mode change (shown / peek / collapsed)|page, laid, bar, fade values|150 (TOOLBAR_MOTION show/hide)|curve.enter both ways|parallel, stopTogether false; reduceMotion via motionStep|yes|apps/browser/src/components/toolbar/AutoHideToolbar.tsx:101,103|-| 4.8 |
|A-095|Auto-hide initial settle|mount|same values, held at their initial levels|0 (settle)|-|-|yes|apps/browser/src/components/toolbar/AutoHideToolbar.tsx:90-91|-| 4.8 |
|A-096|Auto-hide band|`bar` value|translateY, scaleY of the band; divider translateY|150|curve.enter|-|yes|apps/browser/src/components/toolbar/AutoHideToolbar.tsx:139-141,148|-| 4.8 |
|A-097|Auto-hide controls / host label|`fade` / `bar`|opacity [1,.45,.15,0,0] / [0,0,1]; translateY + scale→SHRINK|150|curve.enter|peek delays via setTimeout|yes|apps/browser/src/components/toolbar/AutoHideToolbar.tsx:197-201,214,219|-| 4.8 |
|A-098|Toolbar band colour (AutoHide, Developer, Toolbar×2)|site colour changes|backgroundColor|150|curve.enter|-|no|apps/browser/src/components/toolbar/AutoHideToolbar.tsx:132; apps/browser/src/components/toolbar/DeveloperBar.tsx:32; apps/browser/src/components/toolbar/Toolbar.tsx:98,180|docs/peek-motion.md:34| 4.8 |
|A-099|Toolbar focus dim|pane focus changes (split)|opacity 0.5↔1|150 (TOOLBAR_MOTION.dim)|curve.enter|-|yes|apps/browser/src/components/toolbar/Toolbar.tsx:112|-| 4.8 |
|A-100|Field load fill (JS)|load show|opacity→1|200|curve.enter|-|yes|apps/browser/src/components/toolbar/FieldLoadFill.tsx:22|-| 4.8 |
|A-101|Field load fill progress|page progress|filled translateX|250|curve.enter|reduceMotion → setValue|yes|apps/browser/src/components/toolbar/FieldLoadFill.tsx:28|-| 4.8 |
|A-102|Field load fill finish|load done|filled→1, comet crossfade→1, then opacity→0|300 finish; 250 hide|enter; hide exit|sequence: sweep, delay 200 hold, fade|yes|apps/browser/src/components/toolbar/FieldLoadFill.tsx:35-40|-| 4.8 |
|A-103|Field glow (native)|same driver|native FieldGlowView with timings [200,250,300,200,250]|native|curves exported via fieldGlowMotion.mjs|-|native|apps/browser/src/components/toolbar/FieldGlowFill.tsx:12,18,47|-| 4.8 |
|A-104|Reload ↔ stop swap|loading starts|stateMotion 0→1: reload opacity/scale→0.72, stop in|150 (stateIn)|curve.enter|reduceMotion → setValue|yes|apps/browser/src/components/toolbar/NavButtons.tsx:70,96-102|docs/dia-spec.md:357| 4.8 |
|A-105|Stop → reload return|loading ends|stateMotion 1→0 + returnMotion rotate 28°→0|250 (stateOut)|curve.enter|parallel|yes|apps/browser/src/components/toolbar/NavButtons.tsx:75-78,99|docs/dia-spec.md:357| 4.8 |
|A-106|Stop glyph breathe|while loading, motion ready, not reduced|opacity 0.86↔1, scale 0.96↔1.06|1420 cycle (710 each leg)|curve.pulse|nativeLoop, infinite|yes|apps/browser/src/components/toolbar/NavButtons.tsx:83-87,104-105|-| 4.8 |
|A-107|Back/forward glyph handoff|history button handoff|out: opacity→0, x→±4; in: opacity 0→1 after 35, x ∓4→0|300 (slow+50)|curve.enter|incomingDelay 35 via inputRange|yes|apps/browser/src/components/toolbar/NavButtons.tsx:217,193-196|-| 4.8 |

Shared `Button` press and hover motion appears at many call sites (NavButtons:151,255; Toolbar; FindBar; SmallChrome; PeekOverlay:309; StripGroup:169; NewTabButton:52-53; etc.). The ui `Button` owns that motion, not this slice [INFERENCE]. I did not list each call site.

**Slice coverage and notes**

Command (from `apps/browser/src/components`), run as `rg -c -g '!*.test.*' -e '<assignment regex>' tabstrip window split littleArdia peek find omnibox toolbar` → 53 files:

- **apps/browser/src/components/tabstrip/FadeIn.tsx** (5): row :9.
- **apps/browser/src/components/tabstrip/GroupMember.tsx** (11): rows :68, :71-75.
- **apps/browser/src/components/tabstrip/IncomingTab.tsx** (6): row :48.
- **apps/browser/src/components/tabstrip/NewTabButton.tsx** (4): row :41/:51.
- **apps/browser/src/components/tabstrip/PinnedDock.tsx** (12): rows :29/:40-41; also hosts FadeIn :111 and drag.style :92/:142 (useStripDrag rows).
- **apps/browser/src/components/tabstrip/PinnedFace.tsx** (2): HoverFill row :66.
- **apps/browser/src/components/tabstrip/SelectedTab.tsx** (13): not animation by itself. It forwards an animated width (`plus`/`Animated.add`) and eased band fills from callers.
- **apps/browser/src/components/tabstrip/SplitChip.tsx** (9): uses drag.style (useStripDrag rows) and hover-card hooks. FadeLabel :106 is a static gradient.
- **apps/browser/src/components/tabstrip/StripGroup.tsx** (20): rows :84/:96, :105-167, :236. FadeLabel :214 is static.
- **apps/browser/src/components/tabstrip/StripHoverCard.tsx** (3): row :38/:45.
- **apps/browser/src/components/tabstrip/StripPage.tsx** (13): rows :218/:222/:131 and :189. It creates `grow` (:117), which the dockCross row animates.
- **apps/browser/src/components/tabstrip/TabChip.tsx** (18): rows :113, :137, :147, :238, :310. Also hosts FadeIn :117. FadeLabel :218 is static.
- **apps/browser/src/components/tabstrip/TopTabStrip.tsx** (4): row :164-165.
- **apps/browser/src/components/tabstrip/WindowDropGlow.tsx** (4): row :17.
- **apps/browser/src/components/tabstrip/dockCross.ts** (6): rows :12, :114.
- **apps/browser/src/components/tabstrip/model.ts** (2): not animation. It is a type only.
- **apps/browser/src/components/tabstrip/stripReorder.ts** (4): row :36.
- **apps/browser/src/components/tabstrip/tokens.ts** (5): token-only (:73-82).
- **apps/browser/src/components/tabstrip/useStripDrag.ts** (10): rows :34, :88, :118, :138.
- **apps/browser/src/components/window/BrowserWindowControls.tsx** (2): row :56-57.
- **apps/browser/src/components/window/ControlPill.tsx** (9): row :22.
- **apps/browser/src/components/window/LoadingPill.tsx** (19): rows :33-39, :41, :48-53.
- **apps/browser/src/components/window/NewTabNotice.tsx** (6): row :12. Pressed opacity :33 is a static swap.
- **apps/browser/src/components/window/PageOverlays.tsx** (4): row :64.
- **apps/browser/src/components/window/PaneOverlays.tsx** (5): row :15.
- **apps/browser/src/components/window/ProfileSwipe.tsx** (5): row :30/:72.
- **apps/browser/src/components/window/SidebarDock.tsx** (25): rows :90, :91, :239/:297/:323, :334.
- **apps/browser/src/components/window/SwipeOverlay.tsx** (42): rows :113, :129, :131, :149/:158, :171, :181.
- **apps/browser/src/components/window/TabPane.tsx** (4): row :151/:180.
- **apps/browser/src/components/window/WindowControls.tsx** (4): row (opacity prop :104).
- **apps/browser/src/components/window/dockMotion.ts** (1): not animation. It holds easing functions used by usePeek and test math (`springAt`, `slideAt`).
- **apps/browser/src/components/window/tokens.ts** (15): token-only.
- **apps/browser/src/components/window/useDockLights.ts** (3): row :17/:39.
- **apps/browser/src/components/window/usePeek.ts** (7): the shared hook for the two peek rows (:30-33).
- **apps/browser/src/components/split/SplitChrome.tsx** (15): rows :262-266, :270, :273, :334.
- **apps/browser/src/components/split/tokens.ts** (5): token-only.
- **apps/browser/src/components/littleArdia/LittleArdiaWindow.tsx** (17): rows :104, :110, :117, :138/:152, :139, :143.
- **apps/browser/src/components/littleArdia/SmallChrome.tsx** (2): not animation. FadeLabel :92 is static.
- **apps/browser/src/components/peek/PeekOverlay.tsx** (15): rows :197 and :253 (dead zoom path). `reduceMotion` is read only and passed to the native stage.
- **apps/browser/src/components/find/FindBar.tsx** (6): row :38.
- **apps/browser/src/components/find/tokens.ts** (2): token-only.
- **apps/browser/src/components/omnibox/ScopePulse.tsx** (12): rows :30, :48.
- **apps/browser/src/components/omnibox/tokens.ts** (4): token-only.
- **apps/browser/src/components/toolbar/AutoHideToolbar.tsx** (38): rows :90, :101, :139-148, :197-219, :132.
- **apps/browser/src/components/toolbar/DeveloperBar.tsx** (2): row :32 (eased band :46).
- **apps/browser/src/components/toolbar/FieldGlowFill.tsx** (1): row :12/:47 (native).
- **apps/browser/src/components/toolbar/FieldLoadFill.tsx** (17): rows :22, :28, :35-40.
- **apps/browser/src/components/toolbar/NavButtons.tsx** (40): rows :70, :75, :83, :217. The reduceMotion listeners are read only.
- **apps/browser/src/components/toolbar/Toolbar.tsx** (9): rows :98/:180, :112. FadeLabel :311 is static.
- **apps/browser/src/components/toolbar/WindowToolbar.tsx** (2): not animation. It holds the type of the bar/fade values.
- **apps/browser/src/components/toolbar/fieldGlowMotion.mjs** (5): not animation. It is a build/test script that exports curves to native.
- **apps/browser/src/components/toolbar/fieldLoadDrive.ts** (3): not animation. It is a driver that only passes the `still` flag from reduceMotion.
- **apps/browser/src/components/toolbar/tokens.ts** (9): token-only.

Supplementary rg commands I ran:
- `rg -n -g '!*.test.*' -e 'Animated\.|…|toValue|animating|pinchSettle|liftSpring|SLIDE|hoverFade|stiffness|damping|ActivityIndicator|Spinner'` (453 output lines).
- `rg -n -e '\.start\(|use…Hover|Easing\.|spring\(|scrollTo\(|setTimeout|cancelAnimationFrame|Animated\.event|…'`
- `rg -n 'useEasedColor|<Toast|useGated|<Spinner|ActivityIndicator|animating=|useHover\(|<Button'`

These searches found no spinners and no LayoutAnimation, cssEasing or cssSpring in the slice. They also found `apps/browser/src/components/window/sidebarDockCheck.ts`, `apps/browser/src/components/window/usePaneLayout.ts` and `apps/browser/src/components/tabstrip/stripToggle.ts`. Those files are outside the main regex, and they contain no animation call of their own.

Not checked: the internals of ui `Button`, `Toast`, `apps/browser/src/components/sidebar/hoverMotion` and `state/windows/profilePager`. I cite them only as call sites.

Mismatches with the spec docs:
- Sidebar dock slide: we use a 200 ms bezier; Dia uses a spring (r0.215 d0.79).
- Group expand/collapse: we use a 180 ms timing; Dia uses a spring (r0.30 d0.82).

## 3. Sidebar

Slice notes:

I inventoried every animation in the `apps/browser/src/components/sidebar/` slice; there are 38 rows below. I found no `LayoutAnimation`, `cssEasing`, `cssSpring`, `nativeLoop`, `usePresence` or `SlidingThumb` use here. No files were written.

Paths are relative to `apps/browser/src/components/`. The local tokens resolve as follows:
- **`SIDEBAR_MOTION`** (`apps/browser/src/components/sidebar/tokens.ts:155`): liveFold=400, fold=150, listSlide=140, listFade=100, hoverCardFade=100.
- **`EDGE_GLOW`** (`tokens.ts:169`): inMs=180, outMs=150.
- **`SPLIT_PREVIEW`** (`tokens.ts:161`) and **`GHOST_TILE_MORPH`** (`tokens.ts:164`): r0.1 d1, which gives k=3947.84, c=125.66, critically damped.
- **`SELECTED_ROW_PILL_MOTION`** (`tokens.ts:379-382`): enter 140 ms with `Easing.out(quad)`; exit 60 ms linear.
- **`ROOM_SPRING`** (`dragModel.ts:75`): `SLIDE` with restDisp 0.002 and restSpeed 0.02.
- **`ROWS_PER_FRAME`** (`pageLayout.ts:7`): 32.

|ID|Surface|Trigger|Properties animated|Duration ms|Curve / spring|Delay / loop|Native driver|Source|Spec| Slice |
|---|---|---|---|---|---|---|---|---|---|---|
|A-108|Fold (groups, live folders, pinned, completed footer)|`collapsed` changes, not on mount|`open` 0↔1|150|`curve.enter`|-|no|`apps/browser/src/components/sidebar/Fold.tsx:25`|-| 4.6 |
|A-109|Fold body|follows `open`|height 0→h, marginBottom −gap→0|(Fold)|(Fold)|-|no|`apps/browser/src/components/sidebar/Fold.tsx:82-83`|-| 4.6 |
|A-110|Fold body (alternate)|follows `open`|height 0→total, opacity|(Fold)|(Fold)|-|no|`apps/browser/src/components/sidebar/Fold.tsx:95`|-| 4.6 |
|A-111|Group chevron|follows the fold `open` value|rotate −90°→0°|150|`curve.enter`|-|no|`apps/browser/src/components/sidebar/GroupBlock.tsx:178`|-| 4.6 |
|A-112|Pinned "Clear" divider chevron|pinned fold `open` (`PageRows.tsx:74`)|rotate −90°→0°|150|`curve.enter`|-|no|`apps/browser/src/components/sidebar/ClearDivider.tsx:89`|`docs/dia-spec.md:327` (§Arc note: Clear above the unpinned tabs) [INFERENCE: the doc may not give motion]| 4.6 |
|A-113|Live folder chevron|fold `open`; static when reduce motion is on|rotate −90°→0°|150|`curve.enter`|-|no|`apps/browser/src/components/sidebar/LiveFolderRows.tsx:84`|-| 4.6 |
|A-114|Fold users|`useFold`|as Fold|150|`curve.enter`|-|no|`apps/browser/src/components/sidebar/LiveFolderBlock.tsx:61`, `apps/browser/src/components/sidebar/PageRows.tsx:74`, `apps/browser/src/components/sidebar/CompletedFooter.tsx:37`|-| 4.6 |
|A-115|List open/close slide (rows, splits, groups, + row, tail)|list diff: open/close/move|translateY from→0|140 × `devScale()`|`curve.cssEase` (0.25,0.1,0.25,1)|-|yes, except the tail when `!nativeTail`|`apps/browser/src/components/sidebar/ListMotion.tsx:260`|`docs/dia-spec.md:259` (§Opening and closing tabs)| 4.6 |
|A-116|Closed-row ghost fade|close|opacity 1→0|100 × `devScale()`|`curve.cssEaseOut` (0,0,0.58,1)|-|yes|`apps/browser/src/components/sidebar/ListMotion.tsx:282`|`docs/dia-spec.md:259` (§Opening and closing tabs)| 4.6 |
|A-117|Shift consumers|`useShift(shift)` reads the ListMotion slide|translateY|140|cssEase|-|yes|`apps/browser/src/components/sidebar/EntryMotion.tsx:35`; users: `apps/browser/src/components/sidebar/TabRow.tsx:86`, `apps/browser/src/components/sidebar/TabRow.tsx:298`, `apps/browser/src/components/sidebar/GroupBlock.tsx:94`, `apps/browser/src/components/sidebar/NewTabRow.tsx:41-42`, `apps/browser/src/components/sidebar/CleanUpUpsell.tsx:85`|`docs/dia-spec.md:259`| 4.6 |
|A-118|Edge glow|`held` toggles|opacity 0↔1|in 180 / out 150|`curve.fade`|-|yes|`apps/browser/src/components/sidebar/EdgeGlow.tsx:18`|-| 4.6 |
|A-119|Hover card enter|target set (fresh or returning)|opacity 0→1|100|`curve.fade`|-|yes|`apps/browser/src/components/sidebar/hoverMotion.ts:57`|`docs/dia-spec.md:230` (§Tab hover card motion)| 4.6 |
|A-120|Hover card exit|target cleared|opacity →0, then unmount|100|`curve.fade`|-|yes|`apps/browser/src/components/sidebar/hoverMotion.ts:66`|`docs/dia-spec.md:230` (§Tab hover card motion)| 4.6 |
|A-121|`useHoverFade`|`shown` toggles|opacity 0↔1|100|`curve.fade`|-|yes|`apps/browser/src/components/sidebar/hoverMotion.ts:84`|`docs/dia-spec.md:230`| 4.6 |
|A-122|Scroll edge line|scroll crosses the edge|opacity 0↔1|100 (`duration.fast`)|`curve.fade`|-|yes|`apps/browser/src/components/sidebar/ScrollEdge.tsx:43`|-| 4.6 |
|A-123|Sidebar scroll-linked edge|scroll (`Animated.event`)|`scrollY` → edge offset 0→GLOW_ROOM−edgeLift (clamped)|continuous|linear map|-|yes|`apps/browser/src/components/sidebar/SidebarPage.tsx:142`, `:155`|-| 4.6 |
|A-124|Selected-row pill enter|selected row scrolls out of view|progress 0→1: opacity 0/.85/1, translateY rise→0, scale 0.3→1 (rise 18)|140|`Easing.out(quad)`|-|yes; reduce motion uses `setValue`|`apps/browser/src/components/sidebar/SelectedRowPill.tsx:24` (call at `:158`; interpolations at `:184-187`)|-| 4.6 |
|A-125|Selected-row pill exit|row back in view|progress →0: opacity, translateY drop 4, scale 0.8, then unmount|60|linear|-|yes|`apps/browser/src/components/sidebar/SelectedRowPill.tsx:24` (call at `:162`; interpolations at `:195-198`)|-| 4.6 |
|A-126|Reveal tab scroll|switch/open via the pill|`ScrollView.scrollTo` animated (native scroll)|native|native|rAF-scheduled|n/a|`apps/browser/src/components/sidebar/useRevealTabs.ts:29` (rAF at `:46`)|-| 4.6 |
|A-127|Live item completion fold|item completes|height rowHeight→0, opacity 1→0|400|`curve.enter`|`Animated.delay(COMPLETION_MS−420)`, sequence|no|`apps/browser/src/components/sidebar/LiveItemRow.tsx:52-54` (styles at `:68`)|-| 4.6 |
|A-128|Live item completion glyph pop|completion|scale 0.3→1|spring|`liftSpring()` (bouncy or snappy)|-|yes|`apps/browser/src/components/sidebar/LiveItemRow.tsx:169` (interpolation at `:174`)|`docs/dia-spec.md:348` (§Press and hover feedback)| 4.6 |
|A-129|Refresh spinner|mounted/active|rotate 0→360°|900 (`period.spinner`)|linear|`Animated.loop`, infinite|yes|`apps/browser/src/components/sidebar/RefreshSpinner.tsx:10`|-| 4.6 |
|A-130|Tab loading spinner|`loading`|native `ActivitySpinner` animates itself [INFERENCE]|-|-|loop|native|`apps/browser/src/components/sidebar/TabRow.tsx:190`|-| 4.6 |
|A-131|Folder panel open|panel root measured|scale 0.92→1; opacity gated by `useGated(!away, shown, 0)`|spring|`SLIDE` k8100 c180|-|no|`apps/browser/src/components/sidebar/FolderPanel.tsx:50` (transform at `:78`, gate at `:40`)|-| 4.6 |
|A-132|Folder drop ring|drag over a tile (`on`)|opacity 0↔1, scale 0.92→1|spring|`SLIDE`|-|no|`apps/browser/src/components/sidebar/TileFace.tsx:260` (`:278`)|`docs/dia-spec.md:866` (§Dragging tabs and split targets)| 4.6 |
|A-133|Pinned tile slot slide|slot x/y change while `sliding`, width unchanged|position x,y|spring|`SLIDE`|-|no|`apps/browser/src/components/sidebar/TileFace.tsx:291`|`docs/dia-spec.md:866`| 4.6 |
|A-134|Pinned grid drop gap fade|drag gap value|opacity 0→1 over pinSlotPitch (clamped)|follows the gap spring|-|-|no|`apps/browser/src/components/sidebar/PinnedGrid.tsx:99`|`docs/dia-spec.md:866`| 4.6 |
|A-135|Split preview show/hide|`previews` toggles|`shown` 0↔1, plus `hidden` = 1−shown|spring|`spring(SPLIT_PREVIEW)` k3947.84 c125.66|-|yes|`apps/browser/src/components/sidebar/SplitPreview.tsx:30` (`:24`)|`docs/dia-spec.md:866`| 4.6 |
|A-136|Split preview side swap|side changes; `setValue` on first show|`toLeft` 0↔1|spring|`SPLIT_PREVIEW`|-|yes|`apps/browser/src/components/sidebar/SplitPreview.tsx:43`|`docs/dia-spec.md:866`| 4.6 |
|A-137|Drag ghost look morph|ghost look changes (row/tile/pane, slot)|w, h, corner, shiftX/Y, tileShown, paneShown, slot.x/y, pin|spring|`spring(GHOST_TILE_MORPH)`, rest 0.001/0.01|`Animated.parallel`, stopTogether false|no|`apps/browser/src/components/sidebar/DragGhost.tsx:136`, `:154`|`docs/dia-spec.md:866`| 4.6 |
|A-138|Drag ghost derived values|morph values|translateX/Y = follow·(pos+shift)+pin·slot; fill colour ghost→selected; lifted 1→0; rowShown|spring-driven|-|-|no|`apps/browser/src/components/sidebar/DragGhost.tsx:59-65`, `:176-179`|`docs/dia-spec.md:866`| 4.6 |
|A-139|Drag ghost pointer follow|PanResponder move|ghost position (`setValue`)|immediate|-|-|no|`apps/browser/src/components/sidebar/dragController.ts:235`|`docs/dia-spec.md:866`| 4.6 |
|A-140|Drag drop settle|drop|ghost position → landing spot, then clear|spring|`SLIDE`|-|no|`apps/browser/src/components/sidebar/dragController.ts:311`|`docs/dia-spec.md:866`| 4.6 |
|A-141|Neighbour room open/close|drag retarget|row gaps and source sizes, lerped through a progress listener|spring|`ROOM_SPRING`|-|no|`apps/browser/src/components/sidebar/dragTargeting.ts:91`|`docs/dia-spec.md:866`| 4.6 |
|A-142|Arriving or split row gap settle|item created after a drop / split settle|gap from→0|spring|`SLIDE` (`dragModel.ts:74`)|-|no|`apps/browser/src/components/sidebar/dragController.ts:31`, `:52`|`docs/dia-spec.md:866`| 4.6 |
|A-143|Drag auto-scroll|pointer near the viewport edge|`scrollTo` (not animated) every 16 ms, speed by distance/4|interval 16|-|`setInterval`|n/a|`apps/browser/src/components/sidebar/dragController.ts:375`|`docs/dia-spec.md:851` (§Many tabs, 48 pt edge)| 4.6 |
|A-144|Sidebar resize overshoot|release past the min/max width|width raw→clamped|spring|`springs.bouncy` k438.65 c25.13|-|no|`apps/browser/src/components/sidebar/resizeDrag.ts:74`|`docs/dia-spec.md:348` (§Press and hover feedback, "edge springing back")| 4.6 |
|A-145|Row/tile press squish (bounce mode)|press|scale 0.975 (row) / 0.94 (tile)|spring|press/release springs|90 ms min hold|yes|`apps/browser/src/components/sidebar/FolderPanel.tsx:105`, `apps/browser/src/components/sidebar/FolderTile.tsx:37`, `apps/browser/src/components/sidebar/GroupBlock.tsx:177`, `apps/browser/src/components/sidebar/LiveFolderBlock.tsx:138`, `apps/browser/src/components/sidebar/LiveItemRow.tsx:65`, `apps/browser/src/components/sidebar/NewTabRow.tsx:38` (all `apps/browser/src/components/sidebar/`)|`docs/dia-spec.md:348`| 4.6 |
|A-146|Icon press scale|press|scale 0.96 (bounce 0.9)|spring|snappy, or press/release|-|yes|`apps/browser/src/components/sidebar/NewTabRow.tsx:39` (only when no squish); `apps/browser/src/components/sidebar/TabRow.tsx:209`|`docs/dia-spec.md:348`| 4.6 |
|A-147|Hover fills|hover/press|opacity, show 50 / hide 200|50/200|`curve.fade`|-|yes|HoverFill: `apps/browser/src/components/sidebar/FolderPanel.tsx:122`, `apps/browser/src/components/sidebar/LiveItemRow.tsx:86`, `apps/browser/src/components/sidebar/NewTabRow.tsx:98`, `apps/browser/src/components/sidebar/TileFace.tsx:244`; HoverSwap: `apps/browser/src/components/sidebar/TabRow.tsx:367`, `apps/browser/src/components/sidebar/TileFace.tsx:53`, `:192`, `:215`|`docs/dia-spec.md:348`| 4.6 |

**Coverage.** I ran these commands from the `apps/browser/src/components/sidebar/` folder:
- The task's `rg -n -e '<pattern>'` with `-g '!*.test.*'` gave 343 matches in 45 files.
- The same pattern with `-c` gave the per-file counts.
- A focused search on Animated calls, `spring(`, `toValue`, `delay` and similar gave 122 lines.

Files with rows: Fold, GroupBlock, ClearDivider, LiveFolderRows, LiveFolderBlock, PageRows, ListMotion, EntryMotion, CleanUpUpsell, NewTabRow, TabRow, EdgeGlow, hoverMotion, HoverCard (`:57` renders `motion.style` from hoverMotion), ScrollEdge, SidebarPage, SelectedRowPill, useRevealTabs, LiveItemRow, RefreshSpinner, FolderPanel, FolderTile, TileFace, PinnedGrid, SplitPreview, DragGhost, dragController, dragTargeting, dragModel (`spring` helper `:74`, `ROOM_SPRING` `:75`), resizeDrag.

CompletedFooter.tsx had no hit, but it gets a row because it uses `useFold`.

Files that are not animation:
- **AddressBar:** `:145` is a rAF remeasure, and `:216` is a FadeLabel static gradient.
- **AnimatedFolder:** the name only; it renders a static `FolderIcon open=` with no motion in this file.
- **AwayTitle, GhostEntry, SearchTabs, TabCardSummary, SidebarRow, FolderHeaderTitle:** FadeLabel static gradient. The FolderHeaderTitle `Animated.View` only passes styles through. SearchTabs uses `scrollTo` with `animated:false`.
- **DragProvider:** `Animated.add` tail offset plus PanResponder wiring; the motion is in dragController.
- **GroupPanel:** `setInterval` 30 s clock tick, and `scrollTo` with `animated:false`.
- **LiveFolderIcon:** an import only.
- **RowHoverParts:** an instant hover background with no fade (`:11`).
- **slideSettle:** snaps an overrun slide to 0 with no motion.
- **dragGeometry:** a timer type only.
- **tokens:** tokens only.
- **TabSwitcher** (found by a later search): `scrollTo` with `animated` at `:76` is native scroll and only a timing guard (120/450 ms). I left it out of the table [INFERENCE: it may count as motion].

I did not check `ActivitySpinner` or `apps/browser/src/components/ui/FolderIcon` to see if they animate inside. Fold does not branch on reduce motion in the lines I read.

## 4. Content surfaces: media, live, ntp, profiles, settings, site, unread, bookmarks, pages, report

Slice notes:

All paths below are relative to `apps/browser/src/components/`. Spec citations point to `docs/`.

"Press scale" is `usePressScaleMotion`, with the scale in brackets. In "still" mode it uses `springs.snappy` (k 986.96, c 56.55) to that scale and back to 1. In "bounce" mode it uses `springs.press` to `bounceDip(scale)` (0.96→0.9, 0.985→0.97), then `springs.release` after a 90 ms hold. It always uses the native driver.

`useEased` and `useSlide` are local helpers in `apps/browser/src/components/bookmarks/barMotion.ts`. `useEased` (:14) is an opacity timing: in = `bookmarksBar.hoverIn` = 100 (`apps/browser/src/components/bookmarks/tokens.ts:22`), out = `hoverOut` = 150 (:23), `curve.enter`, JS driver. `useSlide` (:22) is `Animated.spring` with the tabstrip `SLIDE` (k 8100, c 180, JS driver).

|ID|Surface|Trigger|Properties animated|Duration ms|Curve / spring|Delay / loop|Native driver|Source|Spec| Slice |
|---|---|---|---|---|---|---|---|---|---|---|
|A-148|Bookmarks bar item|drag reorder changes room offset|translateX (:102)|spring|SLIDE|-|no|`apps/browser/src/components/bookmarks/BarItem.tsx:82`|-| 4.16 |
|A-149|Bookmarks bar chip plate|hover or press|opacity of hover/pressed fill|100 in / 150 out|curve.enter|-|no|`apps/browser/src/components/bookmarks/BarItem.tsx:169`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.16 |
|A-150|Bookmarks bar chip drop ring|drag hovers a folder|opacity of accent ring|100 / 150|curve.enter|-|no|`apps/browser/src/components/bookmarks/BarItem.tsx:170`|-| 4.16 |
|A-151|Bookmarks bar site colour band|site palette change, or Peek "Open as Tab" preview|backgroundColor (`useEasedColor`, `apps/browser/src/components/ui/toolbarColors.ts:89`)|150 (default `durations.base`) [INFERENCE: equals duration.base]|curve.enter|-|no|`apps/browser/src/components/bookmarks/BookmarksBar.tsx:30`|`docs/peek-motion.md:34 (§Design, toolbar fades to tab colour)`| 4.1 |
|A-152|Bookmarks bar drop line|drag slot appears or moves|opacity (useEased) + translateX (useSlide)|100/150 + spring|curve.enter + SLIDE|-|no|`apps/browser/src/components/bookmarks/BookmarksBar.tsx:260`|-| 4.16 |
|A-153|Folder menu panel|open or close (waits until measured)|opacity, scale 0.94→1, translateY −4→0, origin top left (`usePopoverMotion`/`Style`, `apps/browser/src/components/ui/Popover.tsx:92,105`)|open 150 / close 100|open curve.enter / close curve.exit|unmounts after close|yes|`apps/browser/src/components/bookmarks/FolderMenu.tsx:61`|-| 4.1 |
|A-154|Folder menu drop line|drag slot in menu|opacity + translateY|100/150 + spring|curve.enter + SLIDE|-|no|`apps/browser/src/components/bookmarks/FolderMenu.tsx:191`|-| 4.16 |
|A-155|Folder menu row fill and ring|hover, press, open child, drop into|opacity of fill and ring|100 / 150|curve.enter|-|no|`apps/browser/src/components/bookmarks/FolderMenu.tsx:213`|-| 4.16 |
|A-156|Folder menu row|drag reorder room offset|translateY (:306)|spring|SLIDE|-|no|`apps/browser/src/components/bookmarks/FolderMenu.tsx:276`|-| 4.16 |
|A-157|Bookmark drag ghost|drag starts or ends|opacity 0→0.92 (×ghostOpacity); left/top follow the pointer directly|100 / 150|curve.enter|-|no|`apps/browser/src/components/bookmarks/FolderMenus.tsx:80`|-| 4.16 |
|A-158|Folder tree chevron|press (only if row has children)|press scale (0.96)|spring|still/bounce|-|yes|`apps/browser/src/components/bookmarks/FolderTree.tsx:111`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.16 |
|A-159|Live CI progress bar, pending segment|pending checks > 0|opacity 1↔0.45|1100 per leg|curve.move|nativeLoop, 2 legs, iterations −1|yes|`apps/browser/src/components/live/LiveItemCard.tsx:167`|-| 4.22 |
|A-160|Meeting alert card|mount|opacity 0→1, translateY −10→0, scale 0.97→1|spring|springs.bouncy (`apps/browser/src/components/live/tokens.ts:41`)|no exit animation|yes|`apps/browser/src/components/live/MeetingAlert.tsx:49`|-| 4.22 |
|A-161|Meeting countdown wiggle (on group title, `apps/browser/src/components/sidebar/GroupBlock.tsx:214`)|minutes left crosses a WIGGLE_MARKS_MIN mark|rotate (−1..1 → −4°..4°)|6 × 100 = 600|curve.move|sequence of steps 1, −1, 0.8, −0.6, 0.3, 0 (`apps/browser/src/components/live/tokens.ts:43-50`)|yes|`apps/browser/src/components/live/MeetingCountdown.tsx:26`|-| 4.6 |
|A-162|Next meeting badge|badge text appears or clears|opacity, scale 0.5→1|spring|show bouncy / hide snappy (`apps/browser/src/components/live/tokens.ts:39-40`)|unmounts after hide|yes|`apps/browser/src/components/live/NextMeetingBadge.tsx:28`|`docs/dia-spec.md:357 (§Press and hover feedback, bouncy for live badge)`| 4.22 |
|A-163|Calendar preview refresh button|refresh running|rotate 0→360° (`apps/browser/src/components/sidebar/RefreshSpinner.tsx:10`)|900 (period.spinner)|linear|Animated.loop, infinite|yes|`apps/browser/src/components/live/CalendarPreview.tsx:81`|-| 4.6 |
|A-164|Cast sink icon tile|press (not active, not busy)|press scale (0.985)|spring|still/bounce|-|yes|`apps/browser/src/components/media/CastPicker.tsx:76`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.26 |
|A-165|Cast spinners|scanning; sink connecting or disconnecting|native ActivityIndicator, scaled 0.6|system|system|self-animating [INFERENCE: RN default animating=true]|native view|`apps/browser/src/components/media/CastPicker.tsx:55` and `:95`|-| 4.26 |
|A-166|Media marquee (title, artist)|text overflows its box|translateX 0→−overflow (:63); edge fade opacities (:78-79, :87, :93)|hold 1500, scroll overflow/30·1000, hold 1500, scroll back|linear on scroll legs; hold legs have no easing (helper default) [INFERENCE]|nativeLoop, 4 legs, iterations from caller|yes|`apps/browser/src/components/media/Marquee.tsx:22`|-| 4.26 |
|A-167|Mini player marquee|mounted with overflow|as above|as above|as above|iterations −1|yes|`apps/browser/src/components/media/MiniPlayer.tsx:31`|-| 4.26 |
|A-168|Mini player transport button|press|press scale (default 0.96)|spring|still/bounce|-|yes|`apps/browser/src/components/media/MiniPlayer.tsx:104`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.26 |
|A-169|Mini player album art|press|press scale (0.985)|spring|still/bounce|-|yes|`apps/browser/src/components/media/MiniPlayer.tsx:143`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.26 |
|A-170|Mini player unmute dot|press (only while muted)|press scale (0.96)|spring|still/bounce|-|yes|`apps/browser/src/components/media/MiniPlayer.tsx:144`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.26 |
|A-171|Mini player artwork|image loads (reset to 0 on new art, :149)|opacity 0→1|150|RN default easing (none given) [INFERENCE: Easing.inOut(ease)]|-|yes|`apps/browser/src/components/media/MiniPlayer.tsx:161`|-| 4.26 |
|A-172|Seek bar (mini and sidebar player)|pointer drag|fill width follows the pointer; no easing, snaps on release|-|none|-|n/a (setState)|`apps/browser/src/components/media/MiniPlayer.tsx:282`|-| 4.26 |
|A-173|Share bar|screen share starts or stops|height 0↔SHARE_BAR_HEIGHT|150 (mediaMotion.shareBarSlide)|curve.enter|-|no|`apps/browser/src/components/media/ShareBar.tsx:22`|-| 4.26 |
|A-174|Sidebar player expand/collapse|hover or seeking|`progress` 0↔1. It drives mask/unmask translateY, titleDrop, artist opacity, seek opacity, controls opacity, controlsLift, close opacity, trailing (:277-286) and the sidebar list clip (:77)|expand 220 / collapse 180; 0 under reduce motion|curve.fade|collapse starts after 160 ms (:107)|yes|`apps/browser/src/components/media/SidebarPlayer.tsx:118`|-| 4.26 |
|A-175|Sidebar player trailing control unpin|pointer leaves trailing slot (pin jumps to 1 at :147)|pin 1→0 (trailing opacity, unpinned opacity)|150|curve.fade|-|yes|`apps/browser/src/components/media/SidebarPlayer.tsx:155`|-| 4.26 |
|A-176|Sidebar player art|press|press scale (0.985)|spring|still/bounce|-|yes|`apps/browser/src/components/media/SidebarPlayer.tsx:237`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.26 |
|A-177|Sidebar player marquee|row hover|marquee as above|as above|as above|iterations −1 when hovered, else 1|yes|`apps/browser/src/components/media/SidebarPlayer.tsx:241`|-| 4.26 |
|A-178|New Tab mark intro|first show per tab; skipped under reduce motion|translateY sun 56→0, back 40.5→0, front 34→0; sun translateX drift 6→0|springs|springs.smooth (each)|delays sun 250, back 170, front 80, drift 250; parallel|yes|`apps/browser/src/components/ntp/NewTabPage.tsx:88`|`docs/dia-spec.md:660 (§1.50 New Tab intro: Dia has no logo entrance; this differs)`| 4.10 |
|A-179|New Tab power-up band|first intro of the tab|PowerUp shader (`@ardia/shaders`), speed 1, origin 0.5, halo = bar|shader clock|shader|once|native shader|`apps/browser/src/components/ntp/NewTabPage.tsx:108`|`docs/dia-spec.md:656 (§1.50 New Tab intro)`| 4.10 |
|A-180|New Tab card appear (CheckInBanner 300, TelemetryAsk 600)|mount|opacity 0→1|250|curve.enter|delay appearDelay|yes|`apps/browser/src/components/ntp/NtpCard.tsx:65`|-| 4.10 |
|A-181|New Tab card lift|hover or press (lift cards only)|scale 1.02 hover / 0.98 pressed / 1|spring|liftSpring(): snappy, or bouncy in bounce mode|-|yes|`apps/browser/src/components/ntp/NtpCard.tsx:69`|`docs/dia-spec.md:356 (§Press and hover feedback)`| 4.10 |
|A-182|New Tab card leave|dismiss|scale→0.98, opacity→0|250|curve.enter|parallel|yes|`apps/browser/src/components/ntp/NtpCard.tsx:73`|-| 4.10 |
|A-183|Personalize button glyph|press|press scale (0.96)|spring|still/bounce|-|yes|`apps/browser/src/components/ntp/Personalize.tsx:20`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.10 |
|A-184|Personalize panel|mount; no exit animation|opacity, translateY 8→0, scale 0.97→1|spring|springs.smooth (`apps/browser/src/components/ntp/tokens.ts:52`)|-|yes|`apps/browser/src/components/ntp/Personalize.tsx:62`|-| 4.10 |
|A-185|New Tab cards clear a Peek (used at NewTabExtras :26, :32)|peek span changes or leaves|translateX left and right layers|150|curve.enter|parallel|yes|`apps/browser/src/components/ntp/peekClearance.ts:33`|-| 4.10 |
|A-186|Release notes postcard drop|mount; reduce motion → setValue|translateY −280→0, opacity 0→1|spring + 250|springs.smooth + curve.enter|parallel|yes|`apps/browser/src/components/ntp/Postcard.tsx:47`|-| 4.10 |
|A-187|Postcard hover lift|hover|scale 1↔1.05|150|curve.enter|-|yes|`apps/browser/src/components/ntp/Postcard.tsx:58`|-| 4.10 |
|A-188|Postcard tooltip|hover|opacity|100 in / 100 out|curve.enter|-|yes|`apps/browser/src/components/ntp/Postcard.tsx:59`|-| 4.10 |
|A-189|Postcard dismiss|close button|opacity→0, scale→0.96|150|curve.exit|parallel|yes|`apps/browser/src/components/ntp/Postcard.tsx:64`|-| 4.10 |
|A-190|Release notes page open|mount|scrim opacity; card opacity (full at t 0.3), translateX/Y from postcard, scale card/page→1, rotate tilt→−1°|spring|springs.smooth (`apps/browser/src/components/ntp/tokens.ts:94`)|-|yes|`apps/browser/src/components/ntp/Postcard.tsx:152`|-| 4.10 |
|A-191|Release notes page close|scrim press|same values back to 0|250 (pageClose)|curve.exit|-|yes|`apps/browser/src/components/ntp/Postcard.tsx:157`|-| 4.10 |
|A-192|Profile dot|press|press scale (0.96)|spring|still/bounce|-|yes|`apps/browser/src/components/profiles/ProfileDots.tsx:86`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.19 |
|A-193|Profile dot opacity|pager position (swipe tracking and settle in `state/windows/profilePager.ts`)|opacity interpolated rest→selected→rest|follows pager|pager settle|-|follows pager [INFERENCE]|`apps/browser/src/components/profiles/ProfileDots.tsx:92`|`docs/dia-spec.md:606 (§Profile paging)`| 4.19 |
|A-194|Profile dot drag reorder (rendered as translateX at ProfileDots :97)|pan drag|dragged dot follows the pointer (setValue :50); others settle (:55); cancel settles all to 0 (:63)|150|curve.enter|-|yes|`apps/browser/src/components/profiles/useDotReorder.ts:11`|-| 4.19 |
|A-195|Profile indicator glyph|press (icon form only)|press scale (0.96)|spring|still/bounce|-|yes|`apps/browser/src/components/profiles/ProfileIndicator.tsx:47`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.19 |
|A-196|Settings search flash|search reveal of a setting|glow opacity 0→1, hold, →0|100 in, hold 900, 400 out|in curve.enter, out curve.move|Animated.sequence with delay|yes|`apps/browser/src/components/settings/Form.tsx:82`|-| 4.27 |
|A-197|Picture choice card|press|press scale (0.985)|spring|still/bounce|-|yes|`apps/browser/src/components/settings/PictureChoice.tsx:31`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.27 |
|A-198|Settings reveal scroll|search reveal of an out-of-view setting (after a rAF, :110)|ScrollView scrollTo animated (same pane only)|system|system|-|native scroll|`apps/browser/src/components/settings/SettingsWindow.tsx:119`|-| 4.27 |
|A-199|Theme editor FadeIn (used at :131)|mount|opacity 0→1, translateY 4→0|150|curve.enter|-|yes|`apps/browser/src/components/settings/ThemeEditor.tsx:194`|-| 4.27 |
|A-200|Theme editor live draft|colour drag|theme draft applied once per frame (requestAnimationFrame)|per frame|none|-|n/a|`apps/browser/src/components/settings/ThemeEditor.tsx:42`|-| 4.27 |
|A-201|Profile switcher sliding thumb|selected profile changes|x spring (native), width spring (JS); first place or reduce motion → setValue (`apps/browser/src/components/ui/SlidingThumb.tsx:25-26`); shift = dragged pill offset|spring|springs.snappy|-|x yes / width no|`apps/browser/src/components/settings/panes/ProfileSwitcher.tsx:36`|-| 4.1 |
|A-202|Profile switcher tray scroll|selection is out of view|scrollTo animated|system|system|-|native scroll|`apps/browser/src/components/settings/panes/ProfileSwitcher.tsx:60`|-| 4.27 |
|A-203|Profile switcher pill reorder (translateX at :178)|pan drag|dragged pill follows the pointer (:78); others settle (:90); cancel → 0 (:97)|150 (profileRowSettle)|curve.enter|-|yes|`apps/browser/src/components/settings/panes/ProfileSwitcher.tsx:30`|-| 4.27 |
|A-204|Selection popover|mount; no exit animation|opacity, scale 0.94→1, translateY ±3→0, origin at the anchor side|150 (POPOVER_MOTION.open)|curve.enter|-|yes|`apps/browser/src/components/site/SelectionPopover.tsx:37`|-| 4.18 |
|A-205|Site controls action tile|press|press scale (0.985)|spring|still/bounce|-|yes|`apps/browser/src/components/site/SiteControlsParts.tsx:98`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.18 |
|A-206|Site controls extension tile|press|press scale (0.985)|spring|still/bounce|-|yes|`apps/browser/src/components/site/SiteControlsParts.tsx:140`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.18 |
|A-207|Site controls header icon|press (if onPress)|press scale (0.96)|spring|still/bounce|-|yes|`apps/browser/src/components/site/SiteControlsParts.tsx:182`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.18 |
|A-208|Site controls footer chip|press (if onPress)|press scale (0.985)|spring|still/bounce|-|yes|`apps/browser/src/components/site/SiteControlsParts.tsx:232`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.18 |
|A-209|Site controls "more" capsule|press|press scale (0.985)|spring|still/bounce|-|yes|`apps/browser/src/components/site/SiteControlsParts.tsx:257`|`docs/dia-spec.md:348 (§Press and hover feedback)`| 4.18 |
|A-210|Unread count or dot (row and tile)|unread label changes|opacity, scale 0.5→1|spring|show bouncy / hide snappy (`apps/browser/src/components/unread/tokens.ts:28`)|unmounts after hide|yes|`apps/browser/src/components/unread/UnreadBadge.tsx:17`|`docs/dia-spec.md:357 (§Press and hover feedback, bouncy for unread)`| 4.23 |
|A-211|Bookmarks page row drag|pan drag|translateY = gesture dy; opacity 0.85 while dragging; snaps to 0 on release (:74), no animation|-|none|-|n/a (setValue)|`apps/browser/src/components/pages/BookmarkRow.tsx:70`|-| 4.17 |

**Slice coverage and notes**

**Commands I ran** (cwd `apps/browser/src/components`):
- `rg -c -g '!*.test.*' -e '<assignment regex>' media live ntp profiles settings site unread bookmarks pages report` gave 374 hits in 45 files. `apps/browser/src/components/report/` has 0 hits.
- The same `rg -n` with extra terms (`animating|toValue|useNativeDriver|liftSpring|pinchSettle|PinchArea|SLIDE`) gave 380 matches in 46 files.
- `rg -n` for the direct animation calls (`Animated.(timing|spring|…)`, `nativeLoop(`, `usePressScaleMotion(`, `useNativeDriver`, `ActivityIndicator`, `Spinner`, …) over the same folders.
- `rg -n` for `usePopoverMotion|useEasedColor|useEased(|useSlide(|RefreshSpinner|ActivityIndicator|useHover|Popover|Sheet|Toggle|Segmented` over the same folders.

**Files with hits (count) → rows or reason:**
- `apps/browser/src/components/media/CastPicker.tsx` (5) → :55, :76, :95
- `apps/browser/src/components/media/Marquee.tsx` (21) → :22
- `apps/browser/src/components/media/MiniPlayer.tsx` (23) → :31, :104, :143, :144, :161, :282. The other `duration` hits are media length, not motion.
- `apps/browser/src/components/media/ShareBar.tsx` (5) → :22
- `apps/browser/src/components/media/SharePicker.tsx` (2) → not animation: `setInterval` at :84 and :148 refreshes data and previews.
- `apps/browser/src/components/media/SidebarPlayer.tsx` (57) → :118, :155, :237, :241; `usePlayerMotion` :77 is the list clip, which the sidebar consumes.
- `apps/browser/src/components/media/tokens.ts` (2) → tokens only (`shareBarSlide` 150, `marqueePause` 1500, `marqueeSpeed` 30, at :53).
- `apps/browser/src/components/live/CalendarPreview.tsx` (3) → :81. The `FadeLabel` hits at :149 and :181 are a static gradient.
- `apps/browser/src/components/live/LiveItemCard.tsx` (10) → :167. `FadeLabel` :193 is static. `HoverCard` :28 animates inside `apps/browser/src/components/ui/` (other slice).
- `apps/browser/src/components/live/MeetingAlert.tsx` (6) → :49
- `apps/browser/src/components/live/MeetingCountdown.tsx` (4) → :26
- `apps/browser/src/components/live/NextMeetingBadge.tsx` (6) → :28
- `apps/browser/src/components/live/tokens.ts` (10) → tokens only (:38-51)
- `apps/browser/src/components/ntp/NewTabExtras.tsx` (5) → hosts the shifts from `peekClearance.ts:33`
- `apps/browser/src/components/ntp/NewTabPage.tsx` (15) → :88, :108. The rAF at :68 defers the rest of the page by one frame; it is not motion.
- `apps/browser/src/components/ntp/NtpCard.tsx` (11) → :65, :69, :73
- `apps/browser/src/components/ntp/Personalize.tsx` (11) → :20, :62
- `apps/browser/src/components/ntp/Postcard.tsx` (36) → :47, :58, :59, :64, :152, :157
- `apps/browser/src/components/ntp/peekClearance.ts` (6) → :33
- `apps/browser/src/components/ntp/tokens.ts` (6) → tokens only (:52, :94-97)
- `apps/browser/src/components/profiles/ProfileDots.tsx` (10) → :86, :92; translateX at :97
- `apps/browser/src/components/profiles/ProfileIndicator.tsx` (5) → :47
- `apps/browser/src/components/profiles/useDotReorder.ts` (6) → :11
- `apps/browser/src/components/settings/Form.tsx` (7) → :82
- `apps/browser/src/components/settings/PictureChoice.tsx` (5) → :31
- `apps/browser/src/components/settings/SettingsWindow.tsx` (1) → :119. The scrollTo at :96 is not animated.
- `apps/browser/src/components/settings/ThemeEditor.tsx` (6) → :42, :194
- `apps/browser/src/components/settings/panes/ProfileSwitcher.tsx` (11) → :30, :36, :60
- `apps/browser/src/components/settings/panes/SyncSection.tsx` (1) → not animation: a 30 s `setInterval` re-renders relative-time text.
- `apps/browser/src/components/settings/panes/Tabs.tsx` (3) → not animation: `durationParts`/`durationMinutes` are time-unit helpers.
- `apps/browser/src/components/settings/tokens.ts` (4) → tokens only (:59-64)
- `apps/browser/src/components/site/PromptParts.tsx` (2) → not animation: `duration.base` sets a prompt input-delay guard (:44).
- `apps/browser/src/components/site/SelectionPopover.tsx` (7) → :37
- `apps/browser/src/components/site/SiteControlsParts.tsx` (11) → :98, :140, :182, :232, :257. The `Tile` Animated.View at :51 only hosts the scale; its fill changes instantly.
- `apps/browser/src/components/unread/UnreadBadge.tsx` (8) → :17
- `apps/browser/src/components/unread/tokens.ts` (2) → tokens only (:28)
- `apps/browser/src/components/bookmarks/BarItem.tsx` (5) → :82, :169, :170. The pan responder (:29) only drives the drag store.
- `apps/browser/src/components/bookmarks/BookmarksBar.tsx` (3) → :30, :260
- `apps/browser/src/components/bookmarks/FolderMenu.tsx` (8) → :61, :191, :213, :276
- `apps/browser/src/components/bookmarks/FolderMenus.tsx` (6) → :80. The local `usePresence` (:61) has no animation of its own: it keeps closing panels mounted until the close animation at FolderMenu :61 finishes.
- `apps/browser/src/components/bookmarks/FolderTree.tsx` (5) → :111
- `apps/browser/src/components/bookmarks/barMotion.ts` (5) → helper definitions `useEased` :14 and `useSlide` :22; the call sites are in the rows.
- `apps/browser/src/components/bookmarks/tokens.ts` (3) → tokens only (:20-23). `springOpenMs` 550 is a spring-loaded folder open timer, not motion.
- `apps/browser/src/components/pages/BookmarkRow.tsx` (4) → :70
- `apps/browser/src/components/pages/tokens.ts` (2) → tokens only. `historyRow.scrollSettleMs` 150 is at :34; I did not trace where it is used.

**Outside the regex, inside the slice (shared `apps/browser/src/components/ui/` components, animated in their own files):**
- `ActivitySpinner`, a native view from `apps/browser/src/components/ui/native.ts:2`: `apps/browser/src/components/report/RecordSection.tsx:41`, `apps/browser/src/components/site/DeviceChooser.tsx:100`, `apps/browser/src/components/settings/panes/UpdatesSection.tsx:67`, `apps/browser/src/components/settings/panes/OfficialPlugins.tsx:104`.
- `Popover`: in `apps/browser/src/components/site/` (Prompts, ZoomControls, SiteControls, HiddenElements, DeviceChooser, EnginePrompt) and in `apps/browser/src/components/media/` (CastPicker :33, SharePicker :174).
- `Sheet`, `Toggle` and `Segmented`: many files under `apps/browser/src/components/settings/panes/`, and `apps/browser/src/components/profiles/CreateProfile.tsx`, `apps/browser/src/components/report/ReportSheet.tsx`, `apps/browser/src/components/bookmarks/EditBookmarkDialog.tsx`.

The motion of these components is in `apps/browser/src/components/ui/`, which is the ui slice, so I did not give them rows.

**Differences from the spec:**
- The New Tab mark intro (`NewTabPage.tsx:88`) does not agree with `docs/dia-spec.md:660`, which says Dia 1.50 has no logo entrance.
- `docs/dia-spec.md:356` says the New Tab card lift settles on `springs.snappy`. The code uses `liftSpring()` (`NtpCard.tsx:69`), which changes to bouncy in bounce mode.

I did not check the `nativeLoop` default easing on legs that have no easing (the marquee hold legs). I also did not check the system timing of `ActivityIndicator`, `ActivitySpinner` or animated `scrollTo`. These are marked [INFERENCE] or "system". The `Popover`, `Sheet`, `Toggle` and `Segmented` motion is left to the ui slice.

## 5. Onboarding, import, extensions, plugins, downloads, iconPicker (dev, favicon, glyph: no hits)

Slice notes:

I inventoried the animations in this slice by reading the source. I did not run the app, so I saw no motion at runtime. `apps/browser/src/components/dev/`, `apps/browser/src/components/favicon/` and `apps/browser/src/components/glyph/` have no hits.

Shared formulas used in the rows below:
- **Intro cues** (`apps/browser/src/components/onboarding/introCues.ts:4-15`): LETTER=520 ms, TAGLINE=600, EXIT=650. icon delay=250, letters at 1150+55·i (i=0..4), tagline at 2040, exit at 3740.
- **`useDemo`** (`apps/browser/src/components/onboarding/previews/timeline.ts:45`): one 0→1 `Animated.timing` per scene, linear, native driver, once on mount. Duration is `sceneMs` (`previews/tokens.ts:4-18`). `track` (`timeline.ts:11-26`) changes it into keyframe interpolations: 8 samples per segment, ease 1-(1-t)^3. `t.in`/`t.out` default to 240 ms. `crumbSwap` (`timeline.ts:55`) is leave 120 ms, arrive 200 ms, slide 8.
- **`mockMotion`** (`previews/tokens.ts:22-24`): move=400 ms enter, fade=400 ms enter, focus=250 ms enter.

|ID|Surface|Trigger|Properties animated|Duration ms|Curve / spring|Delay / loop|Native driver|Source|Spec| Slice |
|---|---|---|---|---|---|---|---|---|---|---|
|A-212|Onboarding intro icon|Intro mount|opacity 0→1 (reaches 1 at t=0.4), scale 0.6→1|spring|springs.smooth k=246.74 c=26.70|delay 250 − lag|yes|apps/browser/src/components/onboarding/Intro.tsx:86|-| 4.28 |
|A-213|Intro wordmark letters "Ardia" (5)|Intro mount|translateY 14→0 per letter. Opacity is also driven [INFERENCE]|520|curve.enter|delay 1150+55·i − lag; `Animated.parallel` (:85)|yes|apps/browser/src/components/onboarding/Intro.tsx:88|-| 4.28 |
|A-214|Intro tagline|Intro mount|opacity 0→1, translateY 6→0|600|curve.enter|delay 2040 − lag|yes|apps/browser/src/components/onboarding/Intro.tsx:90|-| 4.28 |
|A-215|Intro exit|Intro end|opacity 1→1→0 (holds to 0.6), scale 1→1.08|650|curve.move|delay 3740 − lag; then `onDone`|yes|apps/browser/src/components/onboarding/Intro.tsx:91|-| 4.28 |
|A-216|Intro skip|Skip press or page "skip" event|same exit value 0→1|250 (introSkip)|curve.enter|none|yes|apps/browser/src/components/onboarding/Intro.tsx:105|-| 4.28 |
|A-217|Intro web page (Meadow) motion|Live intro page|CSS: icon spring, letters, tagline, exit, skip, press. Same cues as the RN rows|icon=spring; letters 520; tagline 600; exit 650; skip 250|cssSpring(smooth), cssEasing(enter), cssEasing(move); press cssSpring(snappy) at scale 0.98/0.96|same cue delays; holds come from timeToReach(…, 0.6)|n/a (CSS)|apps/browser/src/components/onboarding/introPage.ts:22-55|-| 4.28 |
|A-218|Onboarding overlay backdrop|Overlay mount|presence opacity 0→1|250 (overlayIn)|curve.enter|none|yes|apps/browser/src/components/onboarding/OnboardingOverlay.tsx:47|-| 4.28 |
|A-219|Onboarding card|Step leaves "intro"|card 0→1: opacity 0→1, translateY 24→0, scale 0.97→1 (:84-87)|spring|springs.smooth|on intro: setValue(0)|yes|apps/browser/src/components/onboarding/OnboardingOverlay.tsx:52|-| 4.28 |
|A-220|Onboarding dismiss|`leaving` becomes true|presence →0. card 1→2: opacity →0, scale →1.03|400 (overlayOut)|curve.exit|parallel (:57)|yes|apps/browser/src/components/onboarding/OnboardingOverlay.tsx:58-59|-| 4.28 |
|A-221|Onboarding `Reveal` block|Mount|opacity 0→1, translateY 8→0|400 (reveal)|curve.enter|`delay` prop|yes|apps/browser/src/components/onboarding/Parts.tsx:30|-| 4.28 |
|A-222|StepLayout Reveal: header|Step mount|as Reveal|400|curve.enter|delay 60|yes|apps/browser/src/components/onboarding/StepLayout.tsx:96|-| 4.28 |
|A-223|StepLayout Reveal: body|Step mount|as Reveal|400|curve.enter|delay 140|yes|apps/browser/src/components/onboarding/StepLayout.tsx:104|-| 4.28 |
|A-224|StepLayout Reveal: footer buttons|Step mount|as Reveal|400|curve.enter|delay 240|yes|apps/browser/src/components/onboarding/StepLayout.tsx:127|-| 4.28 |
|A-225|StepLayout Reveal: preview pane|Step mount|as Reveal|400|curve.enter|delay 120|yes|apps/browser/src/components/onboarding/StepLayout.tsx:138|-| 4.28 |
|A-226|StepLayout Reveal: preview pane (2nd layout)|Step mount|as Reveal|400|curve.enter|delay 120|yes|apps/browser/src/components/onboarding/StepLayout.tsx:160|-| 4.28 |
|A-227|"More below" scroll hint|Scroll or size change crosses the overflow edge|opacity 0↔1|100 (duration.fast)|curve.fade|none|yes|apps/browser/src/components/onboarding/StepLayout.tsx:184|-| 4.28 |
|A-228|Onboarding buttons (primary, secondary, circle)|Press in/out|scale|press-scale helper|still: snappy; bounce: press/release|bounce: min hold 90|yes|apps/browser/src/components/onboarding/Buttons.tsx:11 (scales 0.98 at :38 and :65; 0.96 at :109; apps/browser/src/components/onboarding/tokens.ts:98)|docs/dia-spec.md:348 (§Press and hover feedback)| 4.28 |
|A-229|OptionCard|Press in/out|scale 0.98 with preview, else 0.99|press-scale helper|snappy, or press/release in bounce mode|bounce: min hold 90|yes|apps/browser/src/components/onboarding/OptionCard.tsx:39|docs/dia-spec.md:348 (§Press and hover feedback)| 4.28 |
|A-230|MockPanel overlay crossfade|`overlay` prop set or cleared|mock opacity 1→0. Overlay opacity 0→1, scale 0.97→1 (:41, :49)|250 show / 250 hide (mockOverlay)|curve.enter both ways|unmount after hide|yes (usePresence)|apps/browser/src/components/onboarding/Screen.tsx:38|-| 4.28 |
|A-231|Mock window layout (tabs top ↔ sidebar)|Setting change (not first render)|`lay` drives lerp positions (:170)|400|curve.enter (mockMotion.move)|none|no|apps/browser/src/components/onboarding/previews/MockWindow.tsx:90 (timing at :48)|-| 4.28 |
|A-232|Mock window address bar position|Setting change|`addr` drives lerp positions|400|curve.enter|none|no|apps/browser/src/components/onboarding/previews/MockWindow.tsx:91|-| 4.28 |
|A-233|Mock window site color|Setting change|`site` drives opacity fade (:171)|400|curve.enter (mockMotion.fade)|none|yes|apps/browser/src/components/onboarding/previews/MockWindow.tsx:92|-| 4.28 |
|A-234|Mock window light/dark crossfade|Appearance change|top layer opacity 0→1 above the old layer|400|curve.enter|old layer removed on finish|yes|apps/browser/src/components/onboarding/previews/MockWindow.tsx:93 (timing at :62)|-| 4.28 |
|A-235|Mock window focus demo|Option hover or replay (`focus`)|window opacity 1→0, demo opacity 0→1 (:108, :114)|250 (mockMotion.focus)|curve.enter|demo unmounts 240 ms after unfocus (:98)|yes|apps/browser/src/components/onboarding/previews/MockWindow.tsx:94|-| 4.28 |
|A-236|Focus demo scene clock|Demo mount|scene clock `p` 0→1|`DEMOS[id].duration` (sceneMs)|linear|once per mount|yes|apps/browser/src/components/onboarding/previews/MockWindow.tsx:125|-| 4.28 |
|A-237|Auto appearance badge|Mount and on dark change (keyed)|pill opacity in 0–220, translateY 6→0 over 0–300, glyph rotate −120°→0° over 60–700|clock 900 (sceneMs.autoBadge)|linear clock + easeOut track|once|yes|apps/browser/src/components/onboarding/previews/MockWindow.tsx:130-134|-| 4.28 |
|A-238|Mock accent backdrop crossfade|Accent change|new layer opacity 0→1|400|curve.enter|none|yes|apps/browser/src/components/onboarding/previews/MockWindow.tsx:145|-| 4.28 |
|A-239|Mock custom theme backdrop|Custom theme set or cleared|opacity 0↔1|400|curve.enter|none|yes|apps/browser/src/components/onboarding/previews/MockWindow.tsx:147|-| 4.28 |
|A-240|Replay of a choice preview|Replay action|focus cleared, then set after 280, cleared after 1980|timers|-|setTimeout 280 / 280+1700|n/a|apps/browser/src/components/onboarding/ChoiceList.tsx:35-36|-| 4.28 |
|A-241|Style preview sun|Mount|translateX SUN.from↔SUN.to (:79)|6500 per leg|curve.pulse|nativeLoop, infinite|yes|apps/browser/src/components/onboarding/previews/StylePreview.tsx:49|-| 4.28 |
|A-242|Scene: tabs in sidebar|Demo clock|opacity crossfade out(660)/in(700) and other tracks|1200|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/TabScenes.tsx:13|-| 4.28 |
|A-243|Scene: tabs on top|Demo clock|same pattern (:78, :81)|1200|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/TabScenes.tsx:68|-| 4.28 |
|A-244|Scene: address bar move|Demo clock|translate lerp; label crossfade with mid swap at 0.45–0.55 (:135)|1100|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/TabScenes.tsx:95|-| 4.28 |
|A-245|Scene: command bar as field|Demo clock|tracked opacity and transforms|1200|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/CommandBarScenes.tsx:16|docs/omnibox-ux.md [INFERENCE: not grepped per scene]| 4.28 |
|A-246|Scene: command bar centred|Demo clock|tracked|1200|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/CommandBarScenes.tsx:52|-| 4.28 |
|A-247|Scene: new tab opens page|Demo clock|tracked opacity (:88)|1300|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/CommandBarScenes.tsx:69|-| 4.28 |
|A-248|Scene: new tab opens bar|Demo clock|tracked|1300|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/CommandBarScenes.tsx:97|-| 4.28 |
|A-249|Scene: new tab position (top/bottom/next)|Demo clock|tracked (:132)|1100|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/CommandBarScenes.tsx:118|-| 4.28 |
|A-250|Scene: tidy (keep/auto)|Demo clock|tracked; transform at :35; `linear` track used|1500|easeOut / linear tracks|once|yes|apps/browser/src/components/onboarding/previews/scenes/LifecycleScenes.tsx:16|-| 4.28 |
|A-251|Scene: quit (restore/fresh)|Demo clock|tracked away/back layers (:60, :64)|1500|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/LifecycleScenes.tsx:51|-| 4.28 |
|A-252|Scene: windows (separate/shared)|Demo clock|tracked|1400|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/LifecycleScenes.tsx:81|-| 4.28 |
|A-253|Scene: links (little/tab)|Demo clock|tracked|1500|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/LinkScenes.tsx:31|-| 4.28 |
|A-254|Scene: appearance (auto/light/dark)|Demo clock|tracked|1200|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/LookScenes.tsx:17|-| 4.28 |
|A-255|Scene: site color (on/off)|Demo clock|tracked|1300|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/LookScenes.tsx:46|-| 4.28 |
|A-256|Scene: block (on/off)|Demo clock|tracked (:47, :70)|1300|easeOut track|once|yes|apps/browser/src/components/onboarding/previews/scenes/BlockingScenes.tsx:26|-| 4.28 |
|A-257|Tool tour fade|Tour becomes active or ends|opacity|show 250 / hide 150|show curve.enter, hide curve.exit|unmount after hide|yes (usePresence)|apps/browser/src/components/onboarding/tour/ToolTour.tsx:28|-| 4.28 |
|A-258|Tool tour spotlight hole and coach card|Stop target rect or card position changes|hole x/y/w/h/r and card x/y (layout props)|spring|springs.smooth|parallel; first placement uses setValue|no|apps/browser/src/components/onboarding/tour/ToolTour.tsx:82-83|-| 4.28 |
|A-259|Tool tour anchor polling|Interval|re-measures the target; this feeds the springs above|-|-|setInterval 400|n/a|apps/browser/src/components/onboarding/tour/ToolTour.tsx:61|-| 4.28 |
|A-260|Tool tour ring pulse|Ring mount|opacity 0.45↔1 (:153)|1100 per leg (period.pulse)|curve.pulse|nativeLoop, infinite|yes|apps/browser/src/components/onboarding/tour/ToolTour.tsx:131|-| 4.28 |
|A-261|Plugins step spinner|`busy`|native spinner [INFERENCE: ActivitySpinner is a native indicator]|system|system|continuous|n/a|apps/browser/src/components/onboarding/PluginsStep.tsx:94|-| 4.28 |
|A-262|Import progress row|Row mount|opacity 0→1, translateY 8→0|250 (rowAppear)|curve.enter|delay index·70 (stagger, apps/browser/src/components/import/tokens.ts:35)|yes|apps/browser/src/components/import/steps/ProgressStep.tsx:44|-| 4.29 |
|A-263|Import row check/warning mark|Status becomes done or failed|scale 0→1|spring|springs.bouncy k=438.65 c=25.13|none|yes|apps/browser/src/components/import/steps/ProgressStep.tsx:47|docs/dia-spec.md:357 (§Press and hover feedback)| 4.29 |
|A-264|Import active row spinner|status "active"|native ActivityIndicator|system|system|continuous|n/a|apps/browser/src/components/import/steps/ProgressStep.tsx:63|-| 4.29 |
|A-265|Import success mark|Done step mount|scale 0.4→1|spring|springs.bouncy|none|yes|apps/browser/src/components/import/steps/DoneStep.tsx:164|docs/dia-spec.md:357 (§Press and hover feedback)| 4.29 |
|A-266|Import window loading spinner|Loading|native ActivityIndicator|system|system|continuous|n/a|apps/browser/src/components/import/ImportWindow.tsx:26|-| 4.29 |
|A-267|Import Parts row spinner|`spinner` prop|native ActivityIndicator|system|system|continuous|n/a|apps/browser/src/components/import/Parts.tsx:17|-| 4.29 |
|A-268|Extension action popup|First page size measured|opacity 0→1, translateY −4→0 (:71-72)|150 (duration.base)|curve.enter|none; no exit animation|yes|apps/browser/src/components/extensions/ActionPopup.tsx:43|-| 4.20 |
|A-269|Extension side panel|Panel state change|width 0↔width+gap, inner opacity (:71-72)|150, or 0 when a tab switch makes it instant|curve.enter (also on close)|none|no|apps/browser/src/components/extensions/SidePanel.tsx:47|-| 4.20 |
|A-270|Toolbar extension slot slide|Reorder target change, drop, or release|`left`|spring|tabstrip SLIDE k=8100 c=180 m=1, JS driver|follows the drag with setValue while dragging (:170)|no|apps/browser/src/components/extensions/ToolbarExtensions.tsx:171|-| 4.20 |
|A-271|Toolbar extension drag follow|PanResponder move|left = index·pitch+dx (clamped), set directly|-|-|follows the pointer|no|apps/browser/src/components/extensions/ToolbarExtensions.tsx:194, 221-222|-| 4.20 |
|A-272|Toolbar extension button|Press|scale 0.96 (iconPressScale)|press-scale helper|snappy, or press/release in bounce mode|-|yes|apps/browser/src/components/extensions/ToolbarExtensions.tsx:216|docs/dia-spec.md:348 (§Press and hover feedback)| 4.20 |
|A-273|Plugin side panel|`!!panel` toggles|width 0↔width+gap (:65)|150|curve.enter both ways|none|no|apps/browser/src/components/plugins/PluginSidePanel.tsx:40|-| 4.21 |
|A-274|Plugin surface spinners|Loading flags|native spinner [INFERENCE]|system|system|continuous|n/a|apps/browser/src/components/plugins/PluginSurface.tsx:103; apps/browser/src/components/plugins/SurfaceBasics.tsx:83; apps/browser/src/components/plugins/SurfaceDetail.tsx:20; apps/browser/src/components/plugins/SurfaceForm.tsx:39; apps/browser/src/components/plugins/SurfaceList.tsx:101; apps/browser/src/components/plugins/SurfaceSidebar.tsx:18|-| 4.21 |
|A-275|Download "magnet" fly-in|New download event|arced translateX/Y (arc 90), opacity 0→1→1→0 (0/0.1/0.8/1), scale 0.6→1.1→0.3 (:308-322)|400 (MAGNET.flight)|curve.move|setValue(0) per download|yes|apps/browser/src/components/downloads/Downloads.tsx:289|docs/dia-spec.md:767 (§Sidebar rows (1.50.1), Downloads)| 4.15 |
|A-276|Downloads footer button|Press|scale 0.985 (controlPressScale)|press-scale helper|snappy, or press/release in bounce mode|-|yes|apps/browser/src/components/downloads/FooterButton.tsx:10|docs/dia-spec.md:348 (§Press and hover feedback)| 4.15 |
|A-277|Icon picker panel open|`place` set (ready)|opacity 0→1, scale 0.94→1, translateY ∓4→0 (apps/browser/src/components/ui/Popover.tsx:99-107)|150 (POPOVER_MOTION.open)|curve.enter|no close animation, because `closing` is always false|yes|apps/browser/src/components/iconPicker/IconPickerPanel.tsx:67|-| 4.1 |
|A-278|Icon grid cell|Press|scale 0.96 (iconPressScale)|press-scale helper|snappy, or press/release in bounce mode|-|yes|apps/browser/src/components/iconPicker/IconGrid.tsx:125|docs/dia-spec.md:348 (§Press and hover feedback)| 4.4 |

**Slice coverage and notes**

Commands I ran: I ran the assignment's `rg -n -c -g '!*.test.*' -e '<pattern>'` once per folder. I also ran a wider `rg -n` that added `Animated\.(timing|…|Value|event)`, `ActivityIndicator`, `ActivitySpinner`, `animating`, `toValue`, `delay`, `setTimeout` and `useNativeDriver`.

Hit counts from the assignment pattern:
- **onboarding:** Buttons 5, Intro 28, LayoutSteps 2, OnboardingOverlay 16, OptionCard 9, Parts 5, Screen 9, StepLayout 4, introPage 10, tokens 10, tour/ToolTour 40.
- **apps/browser/src/components/onboarding/previews:** Floating 11, Gallery 3, Kit 21, MockWindow 44, StylePreview 8, demos 29, scene 2, timeline 16, tokens 4.
- **apps/browser/src/components/onboarding/previews/scenes:** Blocking 7, CommandBar 5, Lifecycle 11, Link 8, Look 7, Tab 15.
- **import:** steps/DoneStep 5, steps/ProgressStep 10, tokens 4.
- **extensions:** ActionPopup 7, SidePanel 8, ToolbarExtensions 9.
- **plugins:** PluginPageBar 2, PluginSidePanel 8.
- **downloads:** Downloads 11, FooterButton 5, tokens 2.
- **iconPicker:** IconGrid 6, IconPickerPanel 3.
- **No hits:** apps/browser/src/components/dev/, apps/browser/src/components/favicon/, apps/browser/src/components/glyph/.

Files mapped to rows:
- **Rows, by Source line:**
  - **onboarding:** Intro.tsx (:86, 88, 90, 91, 105), introPage.ts (:22-55), OnboardingOverlay.tsx (:47, 52, 58-59), Parts.tsx (:30), StepLayout.tsx (:96, 104, 127, 138, 160, 184), Buttons.tsx (:11), OptionCard.tsx (:39), Screen.tsx (:38), tour/ToolTour.tsx (:28, 61, 82-83, 131).
  - **apps/browser/src/components/onboarding/previews:** MockWindow.tsx (:90-94, 125, 130-134, 145, 147), StylePreview.tsx (:49), timeline.ts (helpers used by the scene rows).
  - **apps/browser/src/components/onboarding/previews/scenes:** TabScenes (:13, 68, 95), CommandBarScenes (:16, 52, 69, 97, 118), LifecycleScenes (:16, 51, 81), LinkScenes (:31), LookScenes (:17, 46), BlockingScenes (:26).
  - **import:** ProgressStep.tsx (:44, 47, 63), DoneStep.tsx (:164).
  - **extensions:** ActionPopup.tsx (:43), SidePanel.tsx (:47), ToolbarExtensions.tsx (:171, 194, 216).
  - **plugins and downloads:** PluginSidePanel.tsx (:40), Downloads.tsx (:289), FooterButton.tsx (:10).
  - **iconPicker:** IconPickerPanel.tsx (:67), IconGrid.tsx (:125).
- **Extra rows from the wider pattern:** ChoiceList.tsx (:35-36), PluginsStep.tsx (:94), apps/browser/src/components/import/ImportWindow.tsx (:26), apps/browser/src/components/import/Parts.tsx (:17), and the six plugin Surface* spinner files.
- **Not animation:**
  - LayoutSteps.tsx: `durationTitle` is a text label.
  - apps/browser/src/components/onboarding/tokens.ts, previews/tokens.ts, apps/browser/src/components/import/tokens.ts, apps/browser/src/components/downloads/tokens.ts: tokens only. `checkMark` and `swatch` are defined, but nothing in this slice uses them.
  - previews/demos.ts: a table of scene durations only.
  - previews/scene.ts: a type only.
  - previews/Floating.tsx and Kit.tsx: they only wrap children in `Animated.View`, so that scenes can pass animated styles. They start no motion of their own.
  - previews/Gallery.tsx:60: a static `Animated.Value(at)` for fixed frames. It does not change.
  - apps/browser/src/components/plugins/PluginPageBar.tsx:47: an `Animated.View` with a static background color.
  - GlassBacking.tsx:42, SetupCodeOutro.tsx:22 and apps/browser/src/components/downloads/useDownloadsPlacement.ts:29: `setTimeout` calls for cache warm-up, the "copied" state and a placement deadline. They are not motion.

Not checked:
- Resolved later: the intro letters animate opacity and translateY from the same value (`apps/browser/src/components/onboarding/Intro.tsx:172-173`).
- Resolved later: `ActivitySpinner` is native. It rotates 360° in 1.88 s, linear, forever (`packages/shell/ios/ActivitySpinner.swift:62-67`). See A-030.
- I did not read each scene body. So the scene rows give the clock and duration per scene, not every tracked property.
- I did not search the docs for each scene, so the `omnibox-ux` Spec link on the command bar row is a guess.

## 6. Call sites of shared helpers

Each helper has one row above. This table lists every call site in scope (from `rg`, `*.test.*` excluded). Paths are under `apps/browser/src/components/`. For JSX components, the opening tag was parsed to find the `variant`, `look`, `animated` or `onPress` prop.

|Helper / component|Row|Count|Call sites (`apps/browser/src/components/` + file:lines)|
|---|---|---|---|---|
|`usePressScaleMotion(` (direct calls)|A-001|34|`bookmarks/FolderTree.tsx:111`; `downloads/FooterButton.tsx:10`; `extensions/ToolbarExtensions.tsx:216`; `iconPicker/IconGrid.tsx:125`; `media/CastPicker.tsx:76`; `media/MiniPlayer.tsx:104,143,144`; `media/SidebarPlayer.tsx:237`; `ntp/Personalize.tsx:20`; `onboarding/Buttons.tsx:11`; `onboarding/OptionCard.tsx:39`; `profiles/ProfileDots.tsx:86`; `profiles/ProfileIndicator.tsx:47`; `settings/PictureChoice.tsx:31`; `sidebar/NewTabRow.tsx:39`; `sidebar/TabRow.tsx:209`; `site/SiteControlsParts.tsx:98,140,182,232,257`; `tabstrip/TabChip.tsx:238`; `ui/Button.tsx:217`; `ui/ButtonKinds.tsx:30,61,128,181`; `ui/Field.tsx:53,217,306`; `ui/Segmented.tsx:33`; `ui/Squish.tsx:11`; `ui/motion.ts:151`|
|`useSquish(`|A-002|7|`sidebar/FolderPanel.tsx:105`; `sidebar/FolderTile.tsx:37`; `sidebar/GroupBlock.tsx:177`; `sidebar/LiveFolderBlock.tsx:138`; `sidebar/LiveItemRow.tsx:65`; `sidebar/NewTabRow.tsx:38`; `ui/Squish.tsx:9`|
|`<Squish>`|A-002|6|`sidebar/FolderPanel.tsx:120`; `sidebar/FolderTile.tsx:62`; `sidebar/GroupBlock.tsx:202`; `sidebar/LiveFolderBlock.tsx:158`; `sidebar/LiveItemRow.tsx:85`; `sidebar/NewTabRow.tsx:82`|
|`<HoverFill>`|A-003|11|`sidebar/FolderPanel.tsx:122`; `sidebar/LiveItemRow.tsx:86`; `sidebar/NewTabRow.tsx:98`; `sidebar/TileFace.tsx:244`; `tabstrip/PinnedFace.tsx:66`; `tabstrip/TabChip.tsx:310`; `ui/Button.tsx:163,243`; `ui/ButtonKinds.tsx:134`; `ui/ListRow.tsx:58`; `ui/Row.tsx:26`|
|`<HoverSwap>`|A-004|9|`sidebar/TabRow.tsx:367`; `sidebar/TileFace.tsx:53,192,215`; `ui/ButtonKinds.tsx:46,86,112,148,213`|
|`usePresence(` (bookmarks/FolderMenus.tsx uses a local `usePresence` with no motion)|A-026|7|`bookmarks/FolderMenus.tsx:29,61`; `onboarding/Screen.tsx:38`; `onboarding/tour/ToolTour.tsx:28`; `window/ControlPill.tsx:22`; `window/NewTabNotice.tsx:12`; `window/PaneOverlays.tsx:15`|
|`usePopoverMotion(`|A-021|5|`bookmarks/FolderMenu.tsx:61`; `downloads/Downloads.tsx:96`; `iconPicker/IconPickerPanel.tsx:67`; `ui/Popover.tsx:82,138`|
|`usePopoverMotionStyle(`|A-021|5|`bookmarks/FolderMenu.tsx:62`; `downloads/Downloads.tsx:101`; `iconPicker/IconPickerPanel.tsx:68`; `ui/Popover.tsx:99,139`|
|`<Popover>`|A-021|15|`media/CastPicker.tsx:33`; `media/SharePicker.tsx:174`; `plugins/PluginOverlays.tsx:76`; `site/DeviceChooser.tsx:121`; `site/EnginePrompt.tsx:18`; `site/HiddenElements.tsx:40`; `site/Prompts.tsx:74,101,167,233,266,293`; `site/SiteControls.tsx:210`; `site/ZoomControls.tsx:42`; `window/HistoryPopover.tsx:38`|
|`useSlidingThumb(` / `<SlidingThumb>`|A-017|3|`settings/panes/ProfileSwitcher.tsx:36`; `ui/Segmented.tsx:17`; `ui/SlidingThumb.tsx:8`|
|`<Segmented>`|A-017, A-018|7|`media/SharePicker.tsx:189`; `onboarding/Personalize.tsx:48`; `report/ReportSheet.tsx:63`; `settings/ThemeEditor.tsx:283`; `settings/panes/EngineModeRow.tsx:26`; `settings/panes/Tabs.tsx:112`; `settings/panes/UpdatesSection.tsx:54`|
|`useEasedColor(`|A-025|8|`bookmarks/BookmarksBar.tsx:30`; `plugins/PluginPageBar.tsx:28`; `tabstrip/TabChip.tsx:137`; `toolbar/AutoHideToolbar.tsx:132`; `toolbar/DeveloperBar.tsx:32`; `toolbar/Toolbar.tsx:98,180`; `ui/toolbarColors.ts:77`|
|`nativeLoop(`|A-027|6|`live/LiveItemCard.tsx:167`; `media/Marquee.tsx:22`; `onboarding/previews/StylePreview.tsx:49`; `onboarding/tour/ToolTour.tsx:131`; `toolbar/NavButtons.tsx:83`; `ui/motion.ts:73`|
|`liftSpring(`|A-028|2|`ntp/NtpCard.tsx:69`; `sidebar/LiveItemRow.tsx:169`|
|`pinchSettle(` / `<PinchArea>`|A-029|4|`littleArdia/LittleArdiaWindow.tsx:110,139,77`; `ui/PinchArea.tsx:9`|
|`<ActivitySpinner>`|A-030|12|`onboarding/PluginsStep.tsx:94`; `plugins/PluginSurface.tsx:103`; `plugins/SurfaceBasics.tsx:83`; `plugins/SurfaceDetail.tsx:20`; `plugins/SurfaceForm.tsx:39`; `plugins/SurfaceList.tsx:101`; `plugins/SurfaceSidebar.tsx:18`; `report/RecordSection.tsx:41`; `settings/panes/OfficialPlugins.tsx:104`; `settings/panes/UpdatesSection.tsx:67`; `sidebar/TabRow.tsx:190`; `site/DeviceChooser.tsx:100`|
|`<Toggle>` look="panel"|A-019|4|`site/SiteControls.tsx:307,310,316`; `site/TranslateControls.tsx:103`|
|`<Toggle>` form look|A-019|36|`plugins/SurfaceForm.tsx:112`; `profiles/CreateProfile.tsx:71`; `settings/Form.tsx:151`; `settings/panes/Advanced.tsx:187,264`; `settings/panes/Appearance.tsx:181,235,238`; `settings/panes/CalendarSection.tsx:79,85`; `settings/panes/General.tsx:96,184,187,214`; `settings/panes/LinkRoutingSection.tsx:82`; `settings/panes/LiveFoldersSection.tsx:80`; `settings/panes/Plugins.tsx:129,135,242`; `settings/panes/PreloadPagesRow.tsx:13`; `settings/panes/ProfileExtensions.tsx:87,131,158,161,168`; `settings/panes/ProfilePasswords.tsx:61,65,69`; `settings/panes/Profiles.tsx:119,215,219`; `settings/panes/ShareDiagnostics.tsx:24`; `settings/panes/SiteSettings.tsx:78,86`; `settings/panes/Tabs.tsx:169`; `settings/panes/UpdatesSection.tsx:49`|
|`<Swatch animated>`|A-020|1|`settings/ThemeEditor.tsx:181`|
|`<Dialog animated>`|A-022|7|`extensions/InstallDialog.tsx:66,93`; `plugins/PluginInputBox.tsx:14`; `plugins/PluginOverlays.tsx:40`; `plugins/PluginQuickPick.tsx:38`; `report/ReportSheet.tsx:32`; `report/SentView.tsx:12`|
|`<Toast>`|A-023|2|`pages/BookmarksPage.tsx:207`; `split/SplitChrome.tsx:334`|
|`<TooltipAbove>`|A-024|1|`profiles/ProfileDots.tsx:98`|
|`<SearchField>`|A-011|10|`iconPicker/IconPickerPanel.tsx:80`; `pages/PageLayout.tsx:42`; `plugins/PluginQuickPick.tsx:52`; `plugins/SurfaceList.tsx:100`; `settings/SettingsSearch.tsx:66`; `settings/panes/ProfilePasswords.tsx:167`; `settings/panes/Shortcuts.tsx:173`; `sidebar/GroupPanel.tsx:132`; `sidebar/SearchTabs.tsx:83`; `split/SplitEmptyState.tsx:59`|
|`<PopUp>`|A-012|48|`import/Parts.tsx:54`; `pages/ClearDataDialog.tsx:78`; `plugins/SurfaceForm.tsx:126`; `settings/Form.tsx:161`; `settings/ThemeEditor.tsx:266`; `settings/panes/Advanced.tsx:109,123`; `settings/panes/AirTrafficSheet.tsx:36,53,138,146,149`; `settings/panes/Appearance.tsx:135,138,178,241,277`; `settings/panes/BookmarksBarSection.tsx:21`; `settings/panes/CalendarSection.tsx:64,67,82`; `settings/panes/General.tsx:103`; `settings/panes/InterfaceSize.tsx:17`; `settings/panes/LinkRoutingSection.tsx:147,148`; `settings/panes/PasswordManagerRow.tsx:57`; `settings/panes/Plugins.tsx:139`; `settings/panes/ProfileAutofill.tsx:275,276`; `settings/panes/ProfileExtensions.tsx:150`; `settings/panes/ProfileSearch.tsx:46`; `settings/panes/Profiles.tsx:82`; `settings/panes/Shortcuts.tsx:176,234,240`; `settings/panes/SiteSettings.tsx:152`; `settings/panes/Tabs.tsx:172,207,211,260,263,267,288,324`; `settings/panes/WebEngineSection.tsx:38,82,108`; `settings/panes/WindowButtonsPicker.tsx:22`|
|`<Checkbox>` (ring look: also A-014)|A-013|27|`extensions/InstallDialog.tsx:116`; `import/Rows.tsx:57`; `import/steps/ChooseStep.tsx:41`; `import/steps/SafariStep.tsx:28`; `media/SharePicker.tsx:229`; `onboarding/HomeStep.tsx:147`; `onboarding/OptionCard.tsx:82`; `onboarding/Parts.tsx:257`; `onboarding/PluginsStep.tsx:97`; `onboarding/SetupLines.tsx:23`; `onboarding/SourceStep.tsx:102`; `pages/ClearDataDialog.tsx:106`; `pages/HistoryRow.tsx:120`; `report/AttachedRow.tsx:65`; `report/RecordSection.tsx:142,157`; `report/ReportSheet.tsx:116`; `settings/panes/CalendarSection.tsx:116`; `settings/panes/Checklist.tsx:26`; `settings/panes/FilterListsSheet.tsx:100,192`; `settings/panes/ProfileSearch.tsx:58,59`; `settings/panes/SyncSheets.tsx:363`; `settings/panes/Tabs.tsx:150,151`; `site/PromptParts.tsx:28`|
|`<Checkbox look="ring">`|A-014|6|`onboarding/HomeStep.tsx:147`; `onboarding/OptionCard.tsx:82`; `onboarding/Parts.tsx:257`; `onboarding/PluginsStep.tsx:97`; `onboarding/SetupLines.tsx:23`; `onboarding/SourceStep.tsx:102`|
|`<Row onPress>`|A-015|20|`settings/panes/Advanced.tsx:158`; `settings/panes/General.tsx:217,234`; `settings/panes/OfficialPlugins.tsx:82`; `settings/panes/Plugins.tsx:123`; `settings/panes/ProfileAutofill.tsx:45,76`; `settings/panes/ProfileExtensions.tsx:75`; `settings/panes/ProfilePasswords.tsx:63,67,71,174`; `settings/panes/ProfileSearch.tsx:55`; `settings/panes/Profiles.tsx:229`; `settings/panes/ShareDiagnostics.tsx:35`; `settings/panes/SiteSettings.tsx:104`; `settings/panes/WebEngineSection.tsx:57,149`; `split/SplitEmptyState.tsx:79,93`|
|`<ListRow>`|A-016|8|`plugins/PluginQuickPick.tsx:92`; `plugins/PluginSidebarViews.tsx:28`; `plugins/SurfaceList.tsx:150`; `plugins/SurfaceSidebar.tsx:63`; `settings/SettingsSearch.tsx:188,227`; `settings/SettingsWindow.tsx:206`; `sidebar/SearchTabs.tsx:128`|
|`<Button variant="icon">`|A-005|92|`downloads/Downloads.tsx:190`; `downloads/DownloadsButton.tsx:51`; `extensions/SidePanel.tsx:119,128`; `extensions/ToolbarExtensions.tsx:233,278,319`; `find/FindBar.tsx:128,129,131`; `iconPicker/IconGrid.tsx:105`; `iconPicker/IconPickerPanel.tsx:95`; `littleArdia/LittleArdiaWindow.tsx:224`; `littleArdia/SmallChrome.tsx:51`; `live/CalendarPreview.tsx:82`; `live/MeetingAlert.tsx:78`; `media/SidebarPlayer.tsx:173,210,212,214,218,338`; `omnibox/OmniboxFrame.tsx:34,105,109`; `omnibox/SuggestionRow.tsx:147`; `pages/BookmarkRow.tsx:120`; `pages/BookmarksPage.tsx:115`; `pages/DownloadsPage.tsx:110,112`; `pages/HistoryRow.tsx:150`; `peek/PeekOverlay.tsx:309`; `plugins/PageBarButtons.tsx:60`; `plugins/PluginSidePanel.tsx:96`; `plugins/PluginToolbarItems.tsx:62`; `plugins/SurfaceActions.tsx:121`; `settings/SettingsWindow.tsx:150,151`; `settings/panes/Advanced.tsx:119`; `settings/panes/AutomationRules.tsx:11`; `settings/panes/EngineModeRow.tsx:28`; `settings/panes/FilterListsSheet.tsx:195`; `settings/panes/FrameSlider.tsx:34`; `settings/panes/InterfaceSize.tsx:16`; `settings/panes/LinkRoutingSection.tsx:83,84,85`; `settings/panes/ProfileAutofill.tsx:252,266`; `settings/panes/ProfileExtensions.tsx:70`; `settings/panes/ProfilePasswords.tsx:263,275,276`; `settings/panes/ProfileSearch.tsx:113`; `settings/panes/ProfileSwitcher.tsx:135`; `settings/panes/Shortcuts.tsx:149,175`; `settings/panes/WebEngineSection.tsx:74,83,84`; `sidebar/AddressBar.tsx:74`; `sidebar/GroupBlock.tsx:223`; `sidebar/LiveFolderBlock.tsx:193`; `sidebar/LiveFolderRows.tsx:104`; `sidebar/LiveItemRow.tsx:116`; `sidebar/NewTabRow.tsx:115`; `sidebar/SidebarField.tsx:74`; `site/ReaderControls.tsx:20`; `site/TranslateControls.tsx:32`; `site/ZoomControls.tsx:18`; `tabstrip/NewTabButton.tsx:52,53`; `tabstrip/StripGroup.tsx:169`; `tabstrip/TabChip.tsx:196`; `toolbar/DeveloperBar.tsx:52,71,77`; `toolbar/DeveloperToolsButtons.tsx:25,27,29`; `toolbar/NavButtons.tsx:151,255`; `toolbar/Toolbar.tsx:143,164,165,263,271,318,336,343,377,398`|
|`<Button variant="text">`|A-006|25|`downloads/Downloads.tsx:125,243,244`; `find/FindBar.tsx:154,155`; `import/steps/ExtensionListing.tsx:27`; `live/CalendarPreview.tsx:106`; `live/LiveItemCard.tsx:102`; `onboarding/Parts.tsx:82`; `onboarding/SetupCodeStep.tsx:23`; `onboarding/tour/ToolTour.tsx:172`; `report/AttachedRow.tsx:74`; `report/RecordSection.tsx:80,81,82,170`; `report/SentView.tsx:25`; `settings/ThemeEditor.tsx:217`; `settings/panes/Plugins.tsx:64`; `settings/panes/UpdatesSection.tsx:91,92`; `sidebar/CleanUpUpsell.tsx:105`; `site/DeviceChooser.tsx:112,150`; `site/SiteExtensions.tsx:21`|
|`<Button variant="stepper">`|A-007|6|`settings/ThemeEditor.tsx:218,228`; `site/ReaderControls.tsx:60,61`; `site/ZoomControls.tsx:60,68`|
|`<Button variant="pill">`|A-008|49|`extensions/InstallDialog.tsx:76,79,80,107`; `iconPicker/IconPickerPanel.tsx:103`; `live/CalendarPreview.tsx:190`; `live/LiveItemCard.tsx:111`; `live/MeetingAlert.tsx:98,100,114`; `media/CastPicker.tsx:63,96`; `media/ShareBar.tsx:54,56,58`; `media/SharePicker.tsx:232,234`; `ntp/TelemetryAsk.tsx:101`; `onboarding/tour/ToolTour.tsx:174,175,176`; `sidebar/CleanUpUpsell.tsx:101,102`; `site/DeviceChooser.tsx:129,131`; `site/EnginePrompt.tsx:28,29`; `site/HiddenElements.tsx:55`; `site/Prompts.tsx:77,78,108,115,122,194,195,196,250,251,269,270,304,305`; `site/ReaderControls.tsx:62`; `site/ZoomControls.tsx:46`; `ui/Toast.tsx:56`; `window/ControlPill.tsx:53`; `window/PaneOverlays.tsx:53,69,77`|
|`<Button variant="close">`|A-009|7|`ntp/NtpCard.tsx:83`; `ntp/Postcard.tsx:122,214`; `plugins/PluginOverlays.tsx:56`; `report/ReportSheet.tsx:64`; `sidebar/CleanUpUpsell.tsx:95`; `ui/Popover.tsx:244`|
|`<Button>` push (no variant or `variant="push"`)|A-010|208|`bookmarks/BookmarkDialog.tsx:92,93,93,94`; `bookmarks/EditBookmarkDialog.tsx:62,63`; `import/Parts.tsx:73,74,119`; `import/steps/ChooseStep.tsx:65,68,69`; `import/steps/DoneStep.tsx:54,84,96,153,154`; `import/steps/ExtensionListing.tsx:56`; `import/steps/ProfilesStep.tsx:95,98`; `import/steps/ProgressStep.tsx:33`; `import/steps/SafariStep.tsx:31,57,60,83,85`; `import/steps/UnlockStep.tsx:39,41,42`; `onboarding/DevResetOnboarding.tsx:11`; `pages/BookmarksPage.tsx:112,149`; `pages/ClearDataDialog.tsx:92,93`; `pages/DownloadsPage.tsx:32,99,100,104,105,108`; `pages/HistoryPage.tsx:98,99,102`; `plugins/PluginConsentSheet.tsx:74,77`; `plugins/PluginInputBox.tsx:25,26`; `plugins/PluginPage.tsx:21`; `plugins/PluginQuickPick.tsx:80,81`; `plugins/SurfaceActions.tsx:102`; `plugins/SurfaceBasics.tsx:84,167`; `plugins/SurfaceForm.tsx:41`; `profiles/CreateProfile.tsx:94,95`; `report/RecordSection.tsx:46`; `report/ReportPreview.tsx:53`; `report/ReportSheet.tsx:162,163`; `report/SentView.tsx:49`; `settings/ThemeEditor.tsx:85`; `settings/panes/Advanced.tsx:84,112,167,262,263,299,300`; `settings/panes/AirTrafficSheet.tsx:162,164,165`; `settings/panes/AutomationRules.tsx:10`; `settings/panes/CalendarSection.tsx:55,58,59`; `settings/panes/ConnectAccountSheet.tsx:104,105,119,120,128,148,149,165,166,191,192,220,221`; `settings/panes/ExportBookmarksRow.tsx:26`; `settings/panes/FilterListsPage.tsx:30,42`; `settings/panes/FilterListsSheet.tsx:127,213`; `settings/panes/General.tsx:72,181,220,246,250`; `settings/panes/HiddenElementsSection.tsx:50`; `settings/panes/LinkRoutingSection.tsx:48,49,262`; `settings/panes/LiveFoldersSection.tsx:62,63,95,95`; `settings/panes/OfficialPlugins.tsx:112,114,115,132`; `settings/panes/PasskeyAccessRow.tsx:35,36`; `settings/panes/PasswordManagerPlugin.tsx:14`; `settings/panes/Plugins.tsx:58,212,213,246,247,252,288`; `settings/panes/ProfileAutofill.tsx:38,69,136,137,138`; `settings/panes/ProfileExtensions.tsx:91,173,178,183,191`; `settings/panes/ProfilePasswords.tsx:138,142,161,190,287,290,298,302,303`; `settings/panes/ProfileSearch.tsx:41,42,80,123,165,176,177`; `settings/panes/Profiles.tsx:201,203,226,251,257`; `settings/panes/SetupCodeRow.tsx:31`; `settings/panes/ShareDiagnostics.tsx:66,77,96,114,117,118,188`; `settings/panes/Shortcuts.tsx:177,197,216`; `settings/panes/SiteSettings.tsx:114,166,170,173`; `settings/panes/SwitchingTabs.tsx:50`; `settings/panes/SyncSection.tsx:139,141,143,157,158,229,231,233,236,256,278`; `settings/panes/SyncSheets.tsx:99,100,101,183,187,188,239,240,277,278,300,320,321,325,341,342,343,366,367`; `settings/panes/Tabs.tsx:340,341`; `settings/panes/UpdatesSection.tsx:68`; `settings/panes/WebEngineSection.tsx:41,42,118,129,130,153`; `settings/panes/WebKitLimitsSheet.tsx:69`|

Not animated, so not rows: `<Sheet>` (`apps/browser/src/components/ui/Dialog.tsx:107-113`) renders `Dialog` without `animated`. Its 24 call sites have no motion. `<Dialog>` without `animated` (5 sites: `bookmarks/BookmarkDialog.tsx:73`; `bookmarks/EditBookmarkDialog.tsx:46`; `pages/ClearDataDialog.tsx:74`; `ui/Dialog.tsx:109`; `window/PaneOverlays.tsx:66`) has no motion. `<ProgressTrack>` gets only numbers (see coverage).

### `useHover()` call sites (hover state only)

`useHover` (`apps/browser/src/components/ui/useHover.ts:10-17`) returns a boolean only. Where a file uses it with HoverFill/HoverSwap or a row above, that row holds the motion. Otherwise the hover change is an instant style swap with no motion. Examples: `apps/browser/src/components/ui/Menu.tsx:46-67` (menu rows keep an instant highlight, as `docs/dia-spec.md:367` says), and `apps/browser/src/components/ui/HoverSlot.tsx:41` (opacity 0/1 swap, no fade).

|Count|Call sites|
|---|---|---|
|72|`bookmarks/BarItem.tsx:131`; `bookmarks/FolderTree.tsx:110`; `downloads/Downloads.tsx:202`; `downloads/FooterButton.tsx:9`; `littleArdia/LittleArdiaWindow.tsx:231`; `live/CalendarPreview.tsx:113,164`; `live/LiveItemCard.tsx:187`; `media/CastPicker.tsx:73,115`; `media/MiniPlayer.tsx:103,142`; `media/SharePicker.tsx:251,276`; `media/SidebarPlayer.tsx:143,296`; `ntp/NtpCard.tsx:60`; `ntp/Personalize.tsx:19`; `ntp/Postcard.tsx:35`; `omnibox/AddChip.tsx:9`; `omnibox/GoButton.tsx:16`; `omnibox/OmniboxFrame.tsx:104`; `omnibox/SuggestionRow.tsx:114`; `omnibox/useBarItems.ts:44`; `onboarding/Buttons.tsx:10`; `onboarding/OptionCard.tsx:38`; `onboarding/Parts.tsx:169,230`; `pages/BookmarkRow.tsx:56`; `pages/DownloadsPage.tsx:60`; `profiles/ProfileDots.tsx:82`; `profiles/ProfileIndicator.tsx:45`; `report/AttachedRow.tsx:31`; `settings/PictureChoice.tsx:30`; `settings/SelectableCard.tsx:30`; `settings/panes/ProfileSwitcher.tsx:159`; `settings/panes/Shortcuts.tsx:263,278`; `sidebar/AddressBar.tsx:208`; `sidebar/ClearDivider.tsx:28,87,113`; `sidebar/FolderPanel.tsx:103`; `sidebar/LiveFolderBlock.tsx:133`; `sidebar/LiveFolderRows.tsx:28,60,82`; `sidebar/LiveItemRow.tsx:37`; `sidebar/NewTabRow.tsx:36`; `sidebar/TabRow.tsx:87`; `site/SiteControlsParts.tsx:97,139,181,231,256`; `split/SplitEmptyState.tsx:114`; `tabstrip/NewTabButton.tsx:30`; `toolbar/Toolbar.tsx:195`; `ui/Button.tsx:124,216`; `ui/ButtonKinds.tsx:29,60,127,180`; `ui/Field.tsx:216`; `ui/Menu.tsx:46`; `ui/Popover.tsx:227`; `ui/Row.tsx:13`; `ui/Swatch.tsx:40`; `ui/useHover.ts:10`; `window/NewTabNotice.tsx:13`; `window/WindowControls.tsx:35`|

### Other ui files checked, not animation

|File|Why no row|
|---|---|---|
|`apps/browser/src/components/ui/Dot.tsx`|`Animated.View` host only (:30); callers pass animated style.|
|`apps/browser/src/components/ui/useSettled.ts`|throttle timer `SETTLE_MS` 120 (:3, :32); not motion.|
|`apps/browser/src/components/ui/Menu.tsx`|instant hover/pressed fill; glass rows switch `GlassFill` state at once (:49-67).|
|`apps/browser/src/components/ui/HoverSlot.tsx`|instant opacity 0/1 swap (:41, :51).|
|`apps/browser/src/components/ui/Glass.tsx`|glass tint by state; no Animated call (native `GlassEffect`, :1).|
|`apps/browser/src/components/ui/native.ts`|re-exports native views (`ActivitySpinner` :2, `PeekStage` :10, `FieldGlowView` :6); their motion is native.|
|`apps/browser/src/components/ui/pinch.ts`|pure math used by A-029 (:1-47).|
|`apps/browser/src/components/ui/useHover.ts`|hover boolean only (:10-17).|
|`apps/browser/src/components/ui/HoverCard.tsx`|static `Panel` (:6-12).|
|`apps/browser/src/components/ui/OverPage.tsx`|static native `Surface` (:4-6).|
|`apps/browser/src/components/ui/Radio.tsx`|static sizes, no Animated (:8-12).|
|`apps/browser/src/components/ui/Badge.tsx` / `apps/browser/src/components/ui/KeyCap.tsx`|static font `scale` prop, no Animated (Badge :72, KeyCap :8).|
|`apps/browser/src/components/ui/FadeLabel.tsx`|static gradient mask (:26-33).|

## 7. How I searched

Scope for every command: `apps/browser/src/components apps/browser/src/lib/pressMotion.ts`, with `-g '!*.test.*'`. Run from the Ardia repo root. Counts are matching lines / files.

|Command (`rg -n -e '<pattern>' <scope> -g '!*.test.*'`)|Lines|Files|
|---|---|---|---|
|`Animated\.(timing\|spring\|decay\|loop\|sequence\|parallel\|stagger\|delay\|event)\(`|168|82|
|`nativeLoop\(`|6|6|
|`useAnimatedValue`|0|0|
|`new Animated\.Value`|148|84|
|`LayoutAnimation`|0|0|
|`cssEasing\|cssSpring\|timeToReach`|10|2|
|`usePressScaleMotion\(`|34|23|
|`useSquish\(\|<Squish\b`|13|7|
|`<HoverFill\b\|<HoverSwap\b\|useHoverFill\(`|23|13|
|`usePresence\(`|7|6|
|`useSlidingThumb\(\|<SlidingThumb\b`|5|3|
|`pinchSettle\(\|<PinchArea\b\|pinchStep\(`|6|3|
|`usePopoverMotion\(\|usePopoverMotionStyle\(\|<Popover\b`|25|14|
|`useEasedColor\(`|8|7|
|`useGated\(`|6|4|
|`liftSpring\(`|2|2|
|`ActivitySpinner\|ActivityIndicator`|35|18|
|`requestAnimationFrame`|12|12|
|`\.interpolate\(`|147|69|
|`(?i)transition`|0|0|
|`useHover\(`|72|53|
|`FadeLabel`|46|20|
|`springs\.\|springSpec\|spring\(`|83|39|
|`curve\.\|duration\.(fast\|base\|slow\|long)\|period\.`|153|71|
|`setInterval\(`|6|5|
|`scrollTo\(`|24|14|
|`reduceMotion\(`|26|16|
|`pressMotionNow\|usePressMotion\(`|8|4|
|broad pattern (below)|894|175|

Broad pattern: `Animated\.(timing\|spring\|decay\|loop\|sequence\|parallel\|stagger\|delay\|event)\|nativeLoop\|legsEasing\|toValue\|duration\|easing\|curve\.\|springs\.\|springSpec\|delay\|useNativeDriver\|usePresence\|useHoverFill\|usePressScaleMotion\|useSquish\|<Squish\|useSlidingThumb\|<SlidingThumb\|HoverFill\|HoverSwap\|pinchSettle\|PINCH_SPRING\|LayoutAnimation\|cssEasing\|cssSpring\|timeToReach\|transition\|period\.\|liftSpring\|interpolate\|hoverFade\|bouncePressMotion\|buttonMotion\|requestAnimationFrame\|setInterval\|reduceMotion\|FadeLabel\|Easing`.

Other checks: `rg -l '<Button\b'` and a JSX opening-tag parser to sort Button call sites by `variant`; the same parser for `<Toggle look="panel">`, `<Checkbox look="ring">`, `<Row onPress>`, `<Swatch animated>`, `<Dialog animated>`, `<SearchField>`, `<PopUp>`, `<ListRow>`. `LayoutAnimation`, `useAnimatedValue` and CSS `transition` have 0 hits. The sub-inventories also ran their own `rg` passes (listed in each slice's notes).

Completeness check: every file that has a hit for `Animated.(timing|spring|…)(`, `new Animated.Value`, `nativeLoop(` or a motion helper call (132 files) has a row or a reason below. Every file with a broad-pattern hit (175 files) is in the list below.

## 8. Per-file coverage

|File|Row IDs|Note|
|---|---|---|---|
|`apps/browser/src/components/bookmarks/BarItem.tsx`|A-148, A-149, A-150||
|`apps/browser/src/components/bookmarks/BookmarkDialog.tsx`|-|call site of A-010.|
|`apps/browser/src/components/bookmarks/BookmarksBar.tsx`|A-151, A-152|call site of A-025.|
|`apps/browser/src/components/bookmarks/EditBookmarkDialog.tsx`|-|call site of A-010.|
|`apps/browser/src/components/bookmarks/FolderMenu.tsx`|A-153, A-154, A-155, A-156|call site of A-021.|
|`apps/browser/src/components/bookmarks/FolderMenus.tsx`|A-157|call site of A-026.|
|`apps/browser/src/components/bookmarks/FolderTree.tsx`|A-158|call site of A-001.|
|`apps/browser/src/components/bookmarks/barMotion.ts`|-|helper definitions `useEased` (:14, opacity timing, 100 in / 150 out, curve.enter, JS) and `useSlide` (:22, spring `SLIDE`); call-site rows A-148..A-152 [per content slice].|
|`apps/browser/src/components/bookmarks/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/downloads/Downloads.tsx`|A-275|call site of A-005, A-006, A-021.|
|`apps/browser/src/components/downloads/DownloadsButton.tsx`|-|call site of A-005.|
|`apps/browser/src/components/downloads/FooterButton.tsx`|A-276|call site of A-001.|
|`apps/browser/src/components/downloads/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/extensions/ActionPopup.tsx`|A-268||
|`apps/browser/src/components/extensions/InstallDialog.tsx`|-|call site of A-008, A-013, A-022.|
|`apps/browser/src/components/extensions/SidePanel.tsx`|A-269|call site of A-005.|
|`apps/browser/src/components/extensions/ToolbarExtensions.tsx`|A-270, A-271, A-272|call site of A-001, A-005.|
|`apps/browser/src/components/find/FindBar.tsx`|A-091|call site of A-005, A-006.|
|`apps/browser/src/components/find/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/iconPicker/IconGrid.tsx`|A-278|call site of A-001, A-005.|
|`apps/browser/src/components/iconPicker/IconPickerPanel.tsx`|A-277|call site of A-005, A-008, A-011, A-021.|
|`apps/browser/src/components/import/ImportWindow.tsx`|A-266||
|`apps/browser/src/components/import/Parts.tsx`|A-267|call site of A-010, A-012.|
|`apps/browser/src/components/import/Rows.tsx`|-|call site of A-013.|
|`apps/browser/src/components/import/steps/ChooseStep.tsx`|-|call site of A-010, A-013.|
|`apps/browser/src/components/import/steps/DoneStep.tsx`|A-265|call site of A-010.|
|`apps/browser/src/components/import/steps/ExtensionListing.tsx`|-|call site of A-006, A-010.|
|`apps/browser/src/components/import/steps/ProfilesStep.tsx`|-|call site of A-010.|
|`apps/browser/src/components/import/steps/ProgressStep.tsx`|A-262, A-263, A-264|call site of A-010.|
|`apps/browser/src/components/import/steps/SafariStep.tsx`|-|call site of A-010, A-013.|
|`apps/browser/src/components/import/steps/UnlockStep.tsx`|-|call site of A-010.|
|`apps/browser/src/components/import/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/littleArdia/LittleArdiaWindow.tsx`|A-083, A-084, A-085, A-086, A-087, A-088|call site of A-005, A-029.|
|`apps/browser/src/components/littleArdia/SmallChrome.tsx`|-|call site of A-005.|
|`apps/browser/src/components/live/CalendarPreview.tsx`|A-163|call site of A-005, A-006, A-008.|
|`apps/browser/src/components/live/LiveItemCard.tsx`|A-159|call site of A-006, A-008, A-027.|
|`apps/browser/src/components/live/MeetingAlert.tsx`|A-160|call site of A-005, A-008.|
|`apps/browser/src/components/live/MeetingCountdown.tsx`|A-161||
|`apps/browser/src/components/live/NextMeetingBadge.tsx`|A-162||
|`apps/browser/src/components/live/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/media/CastPicker.tsx`|A-164, A-165|call site of A-001, A-008, A-021.|
|`apps/browser/src/components/media/Marquee.tsx`|A-166|call site of A-027.|
|`apps/browser/src/components/media/MiniPlayer.tsx`|A-167, A-168, A-169, A-170, A-171, A-172|call site of A-001.|
|`apps/browser/src/components/media/ShareBar.tsx`|A-173|call site of A-008.|
|`apps/browser/src/components/media/SharePicker.tsx`|-|call site of A-008, A-013, A-017, A-018, A-021.|
|`apps/browser/src/components/media/SidebarPlayer.tsx`|A-174, A-175, A-176, A-177|call site of A-001, A-005.|
|`apps/browser/src/components/media/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/ntp/NewTabPage.tsx`|A-178, A-179||
|`apps/browser/src/components/ntp/NtpCard.tsx`|A-180, A-181, A-182|call site of A-009, A-028.|
|`apps/browser/src/components/ntp/Personalize.tsx`|A-183, A-184|call site of A-001.|
|`apps/browser/src/components/ntp/Postcard.tsx`|A-186, A-187, A-188, A-189, A-190, A-191|call site of A-009.|
|`apps/browser/src/components/ntp/TelemetryAsk.tsx`|-|call site of A-008.|
|`apps/browser/src/components/ntp/peekClearance.ts`|A-185||
|`apps/browser/src/components/ntp/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/omnibox/OmniboxFrame.tsx`|-|call site of A-005.|
|`apps/browser/src/components/omnibox/ScopePulse.tsx`|A-092, A-093||
|`apps/browser/src/components/omnibox/SuggestionRow.tsx`|-|call site of A-005.|
|`apps/browser/src/components/omnibox/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/onboarding/Buttons.tsx`|A-228|call site of A-001.|
|`apps/browser/src/components/onboarding/ChoiceList.tsx`|A-240||
|`apps/browser/src/components/onboarding/DevResetOnboarding.tsx`|-|call site of A-010.|
|`apps/browser/src/components/onboarding/HomeStep.tsx`|-|call site of A-013, A-014.|
|`apps/browser/src/components/onboarding/Intro.tsx`|A-212, A-213, A-214, A-215, A-216||
|`apps/browser/src/components/onboarding/LayoutSteps.tsx`|-|not animation: `durationTitle` is a text label.|
|`apps/browser/src/components/onboarding/OnboardingOverlay.tsx`|A-218, A-219, A-220||
|`apps/browser/src/components/onboarding/OptionCard.tsx`|A-229|call site of A-001, A-013, A-014.|
|`apps/browser/src/components/onboarding/Parts.tsx`|A-221|call site of A-006, A-013, A-014.|
|`apps/browser/src/components/onboarding/Personalize.tsx`|-|call site of A-017, A-018.|
|`apps/browser/src/components/onboarding/PluginsStep.tsx`|A-261|call site of A-013, A-014, A-030.|
|`apps/browser/src/components/onboarding/Screen.tsx`|A-230|call site of A-026.|
|`apps/browser/src/components/onboarding/SetupCodeStep.tsx`|-|call site of A-006.|
|`apps/browser/src/components/onboarding/SetupLines.tsx`|-|call site of A-013, A-014.|
|`apps/browser/src/components/onboarding/SourceStep.tsx`|-|call site of A-013, A-014.|
|`apps/browser/src/components/onboarding/StepLayout.tsx`|A-222, A-223, A-224, A-225, A-226, A-227||
|`apps/browser/src/components/onboarding/introPage.ts`|A-217||
|`apps/browser/src/components/onboarding/previews/Gallery.tsx`|-|not animation: a static `Animated.Value(at)` for fixed frames (:60).|
|`apps/browser/src/components/onboarding/previews/MockWindow.tsx`|A-231, A-232, A-233, A-234, A-235, A-236, A-237, A-238, A-239||
|`apps/browser/src/components/onboarding/previews/StylePreview.tsx`|A-241|call site of A-027.|
|`apps/browser/src/components/onboarding/previews/demos.ts`|-|not animation: a table of scene durations.|
|`apps/browser/src/components/onboarding/previews/scenes/BlockingScenes.tsx`|A-256||
|`apps/browser/src/components/onboarding/previews/scenes/CommandBarScenes.tsx`|A-245, A-246, A-247, A-248, A-249||
|`apps/browser/src/components/onboarding/previews/scenes/LifecycleScenes.tsx`|A-250, A-251, A-252||
|`apps/browser/src/components/onboarding/previews/scenes/LinkScenes.tsx`|A-253||
|`apps/browser/src/components/onboarding/previews/scenes/LookScenes.tsx`|A-254, A-255||
|`apps/browser/src/components/onboarding/previews/scenes/TabScenes.tsx`|A-242, A-243, A-244||
|`apps/browser/src/components/onboarding/previews/timeline.ts`|-|helper `useDemo` (:45) and `track` (:11-26) used by scene rows A-242..A-256.|
|`apps/browser/src/components/onboarding/previews/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/onboarding/tokens.ts`|A-228||
|`apps/browser/src/components/onboarding/tour/ToolTour.tsx`|A-257, A-258, A-259, A-260|call site of A-006, A-008, A-026, A-027.|
|`apps/browser/src/components/pages/BookmarkRow.tsx`|A-211|call site of A-005.|
|`apps/browser/src/components/pages/BookmarksPage.tsx`|-|call site of A-005, A-010, A-023.|
|`apps/browser/src/components/pages/ClearDataDialog.tsx`|-|call site of A-010, A-012, A-013.|
|`apps/browser/src/components/pages/DownloadsPage.tsx`|-|call site of A-005, A-010.|
|`apps/browser/src/components/pages/HistoryPage.tsx`|-|call site of A-010.|
|`apps/browser/src/components/pages/HistoryRow.tsx`|-|call site of A-005, A-013.|
|`apps/browser/src/components/pages/PageLayout.tsx`|-|call site of A-011.|
|`apps/browser/src/components/pages/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/peek/PeekOverlay.tsx`|A-089, A-090|call site of A-005.|
|`apps/browser/src/components/plugins/PageBarButtons.tsx`|-|call site of A-005.|
|`apps/browser/src/components/plugins/PluginConsentSheet.tsx`|-|call site of A-010.|
|`apps/browser/src/components/plugins/PluginInputBox.tsx`|-|call site of A-010, A-022.|
|`apps/browser/src/components/plugins/PluginOverlays.tsx`|-|call site of A-009, A-021, A-022.|
|`apps/browser/src/components/plugins/PluginPage.tsx`|-|call site of A-010.|
|`apps/browser/src/components/plugins/PluginPageBar.tsx`|-|not animation: `Animated.View` with a static background (:47); `useEasedColor` call at :28 is A-025. call site of A-025.|
|`apps/browser/src/components/plugins/PluginQuickPick.tsx`|-|call site of A-010, A-011, A-016, A-022.|
|`apps/browser/src/components/plugins/PluginSidePanel.tsx`|A-273|call site of A-005.|
|`apps/browser/src/components/plugins/PluginSidebarViews.tsx`|-|call site of A-016.|
|`apps/browser/src/components/plugins/PluginSurface.tsx`|A-274|call site of A-030.|
|`apps/browser/src/components/plugins/PluginToolbarItems.tsx`|-|call site of A-005.|
|`apps/browser/src/components/plugins/SurfaceActions.tsx`|-|call site of A-005, A-010.|
|`apps/browser/src/components/plugins/SurfaceBasics.tsx`|A-274|call site of A-010, A-030.|
|`apps/browser/src/components/plugins/SurfaceDetail.tsx`|A-274|call site of A-030.|
|`apps/browser/src/components/plugins/SurfaceForm.tsx`|A-274|call site of A-010, A-012, A-019, A-030.|
|`apps/browser/src/components/plugins/SurfaceList.tsx`|A-274|call site of A-011, A-016, A-030.|
|`apps/browser/src/components/plugins/SurfaceSidebar.tsx`|A-274|call site of A-016, A-030.|
|`apps/browser/src/components/profiles/CreateProfile.tsx`|-|call site of A-010, A-019.|
|`apps/browser/src/components/profiles/ProfileDots.tsx`|A-192, A-193|call site of A-001, A-024.|
|`apps/browser/src/components/profiles/ProfileIndicator.tsx`|A-195|call site of A-001.|
|`apps/browser/src/components/profiles/useDotReorder.ts`|A-194||
|`apps/browser/src/components/report/AttachedRow.tsx`|-|call site of A-006, A-013.|
|`apps/browser/src/components/report/RecordSection.tsx`|-|`<ActivitySpinner>` call site (A-030). call site of A-006, A-010, A-013, A-030.|
|`apps/browser/src/components/report/ReportPreview.tsx`|-|call site of A-010.|
|`apps/browser/src/components/report/ReportSheet.tsx`|-|call site of A-009, A-010, A-013, A-017, A-018, A-022.|
|`apps/browser/src/components/report/SentView.tsx`|-|call site of A-006, A-010, A-022.|
|`apps/browser/src/components/settings/Form.tsx`|A-196|call site of A-012, A-019.|
|`apps/browser/src/components/settings/PictureChoice.tsx`|A-197|call site of A-001.|
|`apps/browser/src/components/settings/SettingsSearch.tsx`|-|call site of A-011, A-016.|
|`apps/browser/src/components/settings/SettingsWindow.tsx`|A-198|call site of A-005, A-016.|
|`apps/browser/src/components/settings/ThemeEditor.tsx`|A-199, A-200|call site of A-006, A-007, A-010, A-012, A-017, A-018, A-020.|
|`apps/browser/src/components/settings/panes/Advanced.tsx`|-|call site of A-005, A-010, A-012, A-015, A-019.|
|`apps/browser/src/components/settings/panes/AirTrafficSheet.tsx`|-|call site of A-010, A-012.|
|`apps/browser/src/components/settings/panes/Appearance.tsx`|-|call site of A-012, A-019.|
|`apps/browser/src/components/settings/panes/AutomationRules.tsx`|-|call site of A-005, A-010.|
|`apps/browser/src/components/settings/panes/BookmarksBarSection.tsx`|-|call site of A-012.|
|`apps/browser/src/components/settings/panes/CalendarSection.tsx`|-|call site of A-010, A-012, A-013, A-019.|
|`apps/browser/src/components/settings/panes/Checklist.tsx`|-|call site of A-013.|
|`apps/browser/src/components/settings/panes/ConnectAccountSheet.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/EngineModeRow.tsx`|-|call site of A-005, A-017, A-018.|
|`apps/browser/src/components/settings/panes/ExportBookmarksRow.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/FilterListsPage.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/FilterListsSheet.tsx`|-|call site of A-005, A-010, A-013.|
|`apps/browser/src/components/settings/panes/FrameSlider.tsx`|-|call site of A-005.|
|`apps/browser/src/components/settings/panes/General.tsx`|-|call site of A-010, A-012, A-015, A-019.|
|`apps/browser/src/components/settings/panes/HiddenElementsSection.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/InterfaceSize.tsx`|-|call site of A-005, A-012.|
|`apps/browser/src/components/settings/panes/LinkRoutingSection.tsx`|-|call site of A-005, A-010, A-012, A-019.|
|`apps/browser/src/components/settings/panes/LiveFoldersSection.tsx`|-|call site of A-010, A-019.|
|`apps/browser/src/components/settings/panes/OfficialPlugins.tsx`|-|`<ActivitySpinner>` call site (A-030). call site of A-010, A-015, A-030.|
|`apps/browser/src/components/settings/panes/PasskeyAccessRow.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/PasswordManagerPlugin.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/PasswordManagerRow.tsx`|-|call site of A-012.|
|`apps/browser/src/components/settings/panes/Plugins.tsx`|-|call site of A-006, A-010, A-012, A-015, A-019.|
|`apps/browser/src/components/settings/panes/PreloadPagesRow.tsx`|-|call site of A-019.|
|`apps/browser/src/components/settings/panes/ProfileAutofill.tsx`|-|call site of A-005, A-010, A-012, A-015.|
|`apps/browser/src/components/settings/panes/ProfileExtensions.tsx`|-|call site of A-005, A-010, A-012, A-015, A-019.|
|`apps/browser/src/components/settings/panes/ProfilePasswords.tsx`|-|call site of A-005, A-010, A-011, A-015, A-019.|
|`apps/browser/src/components/settings/panes/ProfileSearch.tsx`|-|call site of A-005, A-010, A-012, A-013, A-015.|
|`apps/browser/src/components/settings/panes/ProfileSwitcher.tsx`|A-201, A-202, A-203|call site of A-005, A-017.|
|`apps/browser/src/components/settings/panes/Profiles.tsx`|-|call site of A-010, A-012, A-015, A-019.|
|`apps/browser/src/components/settings/panes/SetupCodeRow.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/ShareDiagnostics.tsx`|-|call site of A-010, A-015, A-019.|
|`apps/browser/src/components/settings/panes/Shortcuts.tsx`|-|call site of A-005, A-010, A-011, A-012.|
|`apps/browser/src/components/settings/panes/SiteSettings.tsx`|-|call site of A-010, A-012, A-015, A-019.|
|`apps/browser/src/components/settings/panes/SwitchingTabs.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/SyncSection.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/SyncSheets.tsx`|-|call site of A-010, A-013.|
|`apps/browser/src/components/settings/panes/Tabs.tsx`|-|call site of A-010, A-012, A-013, A-017, A-018, A-019.|
|`apps/browser/src/components/settings/panes/UpdatesSection.tsx`|-|`<ActivitySpinner>` call site (A-030); `<Segmented>` A-017/A-018. call site of A-006, A-010, A-017, A-018, A-019, A-030.|
|`apps/browser/src/components/settings/panes/WebEngineSection.tsx`|-|call site of A-005, A-010, A-012, A-015.|
|`apps/browser/src/components/settings/panes/WebKitLimitsSheet.tsx`|-|call site of A-010.|
|`apps/browser/src/components/settings/panes/WindowButtonsPicker.tsx`|-|call site of A-012.|
|`apps/browser/src/components/settings/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/sidebar/AddressBar.tsx`|-|call site of A-005.|
|`apps/browser/src/components/sidebar/AwayTitle.tsx`|-|not animation: FadeLabel static gradient.|
|`apps/browser/src/components/sidebar/CleanUpUpsell.tsx`|A-117|call site of A-006, A-008, A-009.|
|`apps/browser/src/components/sidebar/ClearDivider.tsx`|A-112||
|`apps/browser/src/components/sidebar/CompletedFooter.tsx`|A-114||
|`apps/browser/src/components/sidebar/DragGhost.tsx`|A-137, A-138||
|`apps/browser/src/components/sidebar/EdgeGlow.tsx`|A-118||
|`apps/browser/src/components/sidebar/EntryMotion.tsx`|A-117||
|`apps/browser/src/components/sidebar/Fold.tsx`|A-108, A-109, A-110||
|`apps/browser/src/components/sidebar/FolderHeaderTitle.tsx`|-|not animation: FadeLabel static gradient; its `Animated.View` only passes styles.|
|`apps/browser/src/components/sidebar/FolderPanel.tsx`|A-131, A-145, A-147|call site of A-002, A-003.|
|`apps/browser/src/components/sidebar/FolderTile.tsx`|A-145|call site of A-002.|
|`apps/browser/src/components/sidebar/GhostEntry.tsx`|-|not animation: FadeLabel static gradient.|
|`apps/browser/src/components/sidebar/GroupBlock.tsx`|A-111, A-117, A-145|call site of A-002, A-005.|
|`apps/browser/src/components/sidebar/GroupPanel.tsx`|-|call site of A-011.|
|`apps/browser/src/components/sidebar/ListMotion.tsx`|A-115, A-116||
|`apps/browser/src/components/sidebar/LiveFolderBlock.tsx`|A-114, A-145|call site of A-002, A-005.|
|`apps/browser/src/components/sidebar/LiveFolderRows.tsx`|A-113|call site of A-005.|
|`apps/browser/src/components/sidebar/LiveItemRow.tsx`|A-127, A-128, A-145, A-147|call site of A-002, A-003, A-005, A-028.|
|`apps/browser/src/components/sidebar/NewTabRow.tsx`|A-117, A-145, A-146, A-147|call site of A-001, A-002, A-003, A-005.|
|`apps/browser/src/components/sidebar/PageRows.tsx`|A-114||
|`apps/browser/src/components/sidebar/PinnedGrid.tsx`|A-134||
|`apps/browser/src/components/sidebar/RefreshSpinner.tsx`|A-129||
|`apps/browser/src/components/sidebar/RowHoverParts.tsx`|-|not animation: instant hover background, no fade (:11).|
|`apps/browser/src/components/sidebar/ScrollEdge.tsx`|A-122||
|`apps/browser/src/components/sidebar/SearchTabs.tsx`|-|call site of A-011, A-016.|
|`apps/browser/src/components/sidebar/SelectedRowPill.tsx`|A-124, A-125||
|`apps/browser/src/components/sidebar/SidebarField.tsx`|-|call site of A-005.|
|`apps/browser/src/components/sidebar/SidebarPage.tsx`|A-123||
|`apps/browser/src/components/sidebar/SidebarRow.tsx`|-|not animation: FadeLabel static gradient.|
|`apps/browser/src/components/sidebar/SplitPreview.tsx`|A-135, A-136||
|`apps/browser/src/components/sidebar/TabCardSummary.tsx`|-|not animation: FadeLabel static gradient.|
|`apps/browser/src/components/sidebar/TabRow.tsx`|A-117, A-130, A-146, A-147|call site of A-001, A-004, A-030.|
|`apps/browser/src/components/sidebar/TileFace.tsx`|A-132, A-133, A-147|call site of A-003, A-004.|
|`apps/browser/src/components/sidebar/dragController.ts`|A-139, A-140, A-142, A-143||
|`apps/browser/src/components/sidebar/dragGeometry.ts`|-|not animation: timer type only.|
|`apps/browser/src/components/sidebar/dragModel.ts`|-|helper: `spring` (:74) and `ROOM_SPRING` (:75) used by A-141.|
|`apps/browser/src/components/sidebar/dragTargeting.ts`|A-141||
|`apps/browser/src/components/sidebar/hoverMotion.ts`|A-119, A-120, A-121||
|`apps/browser/src/components/sidebar/resizeDrag.ts`|A-144||
|`apps/browser/src/components/sidebar/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/sidebar/useRevealTabs.ts`|A-126||
|`apps/browser/src/components/site/DeviceChooser.tsx`|-|`<ActivitySpinner>` (A-030) and `<Popover>` (A-021) call sites. call site of A-006, A-008, A-021, A-030.|
|`apps/browser/src/components/site/EnginePrompt.tsx`|-|call site of A-008, A-021.|
|`apps/browser/src/components/site/HiddenElements.tsx`|-|call site of A-008, A-021.|
|`apps/browser/src/components/site/PromptParts.tsx`|-|call site of A-013.|
|`apps/browser/src/components/site/Prompts.tsx`|-|call site of A-008, A-021.|
|`apps/browser/src/components/site/ReaderControls.tsx`|-|call site of A-005, A-007, A-008.|
|`apps/browser/src/components/site/SelectionPopover.tsx`|A-204||
|`apps/browser/src/components/site/SiteControls.tsx`|-|call site of A-019, A-021.|
|`apps/browser/src/components/site/SiteControlsParts.tsx`|A-205, A-206, A-207, A-208, A-209|call site of A-001.|
|`apps/browser/src/components/site/SiteExtensions.tsx`|-|call site of A-006.|
|`apps/browser/src/components/site/TranslateControls.tsx`|-|call site of A-005, A-019.|
|`apps/browser/src/components/site/ZoomControls.tsx`|-|call site of A-005, A-007, A-008, A-021.|
|`apps/browser/src/components/split/SplitChrome.tsx`|A-079, A-080, A-081, A-082|call site of A-023.|
|`apps/browser/src/components/split/SplitEmptyState.tsx`|-|call site of A-011, A-015.|
|`apps/browser/src/components/split/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/tabstrip/FadeIn.tsx`|A-032||
|`apps/browser/src/components/tabstrip/GroupMember.tsx`|A-033, A-034||
|`apps/browser/src/components/tabstrip/IncomingTab.tsx`|A-035||
|`apps/browser/src/components/tabstrip/NewTabButton.tsx`|A-036|call site of A-005.|
|`apps/browser/src/components/tabstrip/PinnedDock.tsx`|A-037||
|`apps/browser/src/components/tabstrip/PinnedFace.tsx`|A-039|call site of A-003.|
|`apps/browser/src/components/tabstrip/SplitChip.tsx`|-|hosts `drag.style` from useStripDrag (A-041..A-044); FadeLabel :106 static.|
|`apps/browser/src/components/tabstrip/StripGroup.tsx`|A-047, A-048, A-049|call site of A-005.|
|`apps/browser/src/components/tabstrip/StripHoverCard.tsx`|A-057||
|`apps/browser/src/components/tabstrip/StripPage.tsx`|A-045, A-046||
|`apps/browser/src/components/tabstrip/TabChip.tsx`|A-050, A-051, A-052, A-053, A-054|call site of A-001, A-003, A-005, A-025.|
|`apps/browser/src/components/tabstrip/TopTabStrip.tsx`|A-055||
|`apps/browser/src/components/tabstrip/WindowDropGlow.tsx`|A-056||
|`apps/browser/src/components/tabstrip/dockCross.ts`|A-037, A-038||
|`apps/browser/src/components/tabstrip/stripReorder.ts`|A-036, A-040||
|`apps/browser/src/components/tabstrip/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/tabstrip/useStripDrag.ts`|A-041, A-042, A-043, A-044||
|`apps/browser/src/components/toolbar/AutoHideToolbar.tsx`|A-094, A-095, A-096, A-097, A-098|call site of A-025.|
|`apps/browser/src/components/toolbar/DeveloperBar.tsx`|A-098|call site of A-005, A-025.|
|`apps/browser/src/components/toolbar/DeveloperToolsButtons.tsx`|-|call site of A-005.|
|`apps/browser/src/components/toolbar/FieldGlowFill.tsx`|A-103||
|`apps/browser/src/components/toolbar/FieldLoadFill.tsx`|A-100, A-101, A-102||
|`apps/browser/src/components/toolbar/NavButtons.tsx`|A-104, A-105, A-106, A-107|call site of A-005, A-027.|
|`apps/browser/src/components/toolbar/Toolbar.tsx`|A-098, A-099|call site of A-005, A-025.|
|`apps/browser/src/components/toolbar/fieldGlowMotion.mjs`|-|not animation: build/test script that exports curves to native.|
|`apps/browser/src/components/toolbar/fieldLoadDrive.ts`|-|not animation: passes the `still` flag from reduceMotion to A-100..A-102.|
|`apps/browser/src/components/toolbar/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/ui/Button.tsx`|A-005, A-006|call site of A-001, A-003.|
|`apps/browser/src/components/ui/ButtonKinds.tsx`|A-007, A-008, A-009, A-010|call site of A-001, A-003, A-004.|
|`apps/browser/src/components/ui/Dialog.tsx`|A-022||
|`apps/browser/src/components/ui/Dot.tsx`|-|not animation; see "Other ui files checked".|
|`apps/browser/src/components/ui/FadeLabel.tsx`|-|not animation: static gradient mask (`FADE_WIDTH` 24, :26).|
|`apps/browser/src/components/ui/Field.tsx`|A-011, A-012, A-013, A-014|call site of A-001.|
|`apps/browser/src/components/ui/Glass.tsx`|-|not animation; see "Other ui files checked".|
|`apps/browser/src/components/ui/HoverFill.tsx`|A-003, A-004||
|`apps/browser/src/components/ui/HoverSlot.tsx`|-|not animation; see "Other ui files checked".|
|`apps/browser/src/components/ui/ListRow.tsx`|A-016|call site of A-003.|
|`apps/browser/src/components/ui/Menu.tsx`|-|not animation; see "Other ui files checked".|
|`apps/browser/src/components/ui/PinchArea.tsx`|A-029||
|`apps/browser/src/components/ui/Popover.tsx`|A-021|call site of A-009.|
|`apps/browser/src/components/ui/ProgressTrack.tsx`|-|can interpolate an `Animated.Value` width (:21), but every caller passes a number (DownloadsPage.tsx:86, ThemeControls.tsx:224,257, Downloads.tsx:235, SurfaceBasics.tsx:127, ReportSheet.tsx:152): no motion.|
|`apps/browser/src/components/ui/Row.tsx`|A-015|call site of A-003.|
|`apps/browser/src/components/ui/Segmented.tsx`|A-018|call site of A-001, A-017.|
|`apps/browser/src/components/ui/SlidingThumb.tsx`|A-017||
|`apps/browser/src/components/ui/Squish.tsx`|A-002|call site of A-001.|
|`apps/browser/src/components/ui/Swatch.tsx`|A-020||
|`apps/browser/src/components/ui/Switch.tsx`|A-019||
|`apps/browser/src/components/ui/Toast.tsx`|A-023|call site of A-008.|
|`apps/browser/src/components/ui/TooltipAbove.tsx`|A-024||
|`apps/browser/src/components/ui/cssMotion.ts`|A-031||
|`apps/browser/src/components/ui/hoverShift.ts`|-|not animation: dev check that hover causes no layout shift (`check` :165, waits `settleFrames` 3; 500 ms measure timeout :76).|
|`apps/browser/src/components/ui/index.ts`|-|re-exports only.|
|`apps/browser/src/components/ui/motion.ts`|A-001, A-002, A-003, A-026, A-027, A-028||
|`apps/browser/src/components/ui/native.ts`|A-030|not animation; see "Other ui files checked".|
|`apps/browser/src/components/ui/pinch.ts`|-|not animation; see "Other ui files checked".|
|`apps/browser/src/components/ui/reduceMotion.ts`|-|state source: `reduceMotion()` reads AccessibilityInfo (:5-11); rows branch on it.|
|`apps/browser/src/components/ui/tokens.ts`|A-020||
|`apps/browser/src/components/ui/toolbarColors.ts`|A-025, A-051||
|`apps/browser/src/components/ui/useHover.ts`|-|not animation; see "Other ui files checked".|
|`apps/browser/src/components/ui/useSettled.ts`|-|not animation; see "Other ui files checked".|
|`apps/browser/src/components/unread/UnreadBadge.tsx`|A-210||
|`apps/browser/src/components/unread/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/window/BrowserWindowControls.tsx`|A-077||
|`apps/browser/src/components/window/ControlPill.tsx`|A-063|call site of A-008, A-026.|
|`apps/browser/src/components/window/HistoryPopover.tsx`|-|call site of A-021.|
|`apps/browser/src/components/window/LoadingPill.tsx`|A-064, A-065, A-066||
|`apps/browser/src/components/window/NewTabNotice.tsx`|A-067|call site of A-026.|
|`apps/browser/src/components/window/PageOverlays.tsx`|A-069||
|`apps/browser/src/components/window/PaneOverlays.tsx`|A-068|call site of A-008, A-026.|
|`apps/browser/src/components/window/ProfileSwipe.tsx`|A-070||
|`apps/browser/src/components/window/SidebarDock.tsx`|A-058, A-059, A-060, A-061||
|`apps/browser/src/components/window/SwipeOverlay.tsx`|A-071, A-072, A-073, A-074, A-075, A-076||
|`apps/browser/src/components/window/TabPane.tsx`|A-069, A-078||
|`apps/browser/src/components/window/WindowControls.tsx`|A-077||
|`apps/browser/src/components/window/dockMotion.ts`|-|easing helpers for usePeek (A-055, A-060) and test math; no motion call.|
|`apps/browser/src/components/window/tokens.ts`|-|token-only file: duration/curve values that rows use; no motion call.|
|`apps/browser/src/components/window/useDockLights.ts`|A-062||
|`apps/browser/src/components/window/usePeek.ts`|A-055, A-060||
|`apps/browser/src/lib/pressMotion.ts`|-|mode store read by A-001, A-002, A-006, A-028 (`:1-19`).|
|`packages/shell/ios/ActivitySpinner.swift`|A-030||

---

# Animation inventory, part B: state, lib, platform, App/main, and TS packages

Scope: `apps/browser/src/{state,lib,platform}/**`, `apps/browser/src/App.tsx`, `apps/browser/src/main.tsx`, and the TS packages that the Mac app imports (`packages/{shell,core,shaders,ardiacore,import,sync}/src`). The app imports these packages (`apps/browser/package.json:17-22`). `*.test.*`, `*.windows.*`, `*.linux.*` and `*.web.*` files are not in scope.
All paths are relative to the Ardia repo root at commit 551d3c13c.

## Summary

- This scope has almost no React Native animation code. It has no `Animated.timing/spring/decay/loop/sequence/parallel/stagger`, no `nativeLoop`, no `LayoutAnimation`, no `Easing` and no `useNativeDriver` options in animation configs. The search counts are in "How I searched".
- The motion tokens in `apps/browser/src/components/ui/motion.ts` (`duration.base=150` and the others) are not imported anywhere in this scope (`rg 'ui/motion|from "../../components'` gives 0 hits). Thus no row needs a token value.
- This scope has three kinds of animation:
  1. Native-driven `Animated.Value` interpolations: the profile pager position (B-001, B-002).
  2. Native-module calls that start a native animation: the pager, window open and close, full screen and zoom, the drag preview, Peek, field glow, intro music, shader views, the spinner and the folder icon (B-003 to B-022).
  3. CSS transitions in scripts that are injected into web pages, and JS timers (B-023 to B-026).
- Most curves for native animations are in Swift (the AnimNative scope). When I did not read the curve in Swift, the cell says "native" and cites the spec value as the reference, or marks it [INFERENCE].

## Animation table

| ID | Surface | Trigger | Properties animated | Duration (ms) | Curve / spring | Delay / loop | Native driver? | Source (file:line) | Spec ref | Slice |
|---|---|---|---|---|---|---|---|---|---|---|
| B-001 | Profile pager: each profile page (sidebar page, strip page) | A trackpad swipe or a native pager settle. Native `onPagerPosition` events write `pager.pos` (`apps/browser/src/components/window/ProfileSwipe.tsx:30`, out of scope) | `transform.translateX = (pos − slot) × −width` | Native. The spec gives the settle as 250 ms, critically damped (400 ms without `sidebar-space-swipe-animations`) | Native. The spec gives the rubber band as `255·(1 − 1/(0.15·x/255 + 1))` (`docs/dia-spec.md:613`) | none | Yes. `new Animated.Value(index, { useNativeDriver: true })` | `apps/browser/src/state/windows/profilePager.ts:43`, `:230-234`; `packages/ardiacore/src/swipe.tsx:84,106` (`Animated.createAnimatedComponent` when `nativePager` is set) | dia-spec.md "Profile paging" (`docs/dia-spec.md:606`, settle `:618`) | 4.5 |
| B-002 | Shared Top Apps tiles across neighbouring profile pages | The same `pos` as B-001, only when a neighbour profile shares Top Apps (`sharedTileSides`) | `opacity` [slot−1, slot, slot+1] → [before?0:1, 1, after?0:1]; `translateX` → [before?−width:0, 0, after?width:0]. Both use `extrapolate: "clamp"` | Follows B-001 | Linear interpolation of `pos` | none | Yes (it is derived from the native `pos`) | `apps/browser/src/state/sidebar/sharedTiles.ts:16-22`, `:24-33` | - | 2.18 |
| B-003 | Profile pager: wheel-mouse page step | `ProfilePager.wheel()`: the sum of wheel deltas goes above 1 pt (`WHEEL_THRESHOLD`). Events count as one burst when they come less than 49 ms apart (`WHEEL_BURST_MS`). After a step, wheel events are ignored for 250 ms (`WHEEL_COOLDOWN_MS`) | Native pager position (this drives B-001 and B-002) | Native settle (see B-001) | Native | 250 ms cooldown between steps | Yes (native) | `apps/browser/src/state/windows/profilePager.ts:19-21`, `:129-141`, `:143-150` (`stepPager(windowId, delta, false, order)` at `:145`) | `docs/dia-spec.md:621` (Wheel mice) | 2.9 |
| B-004 | Profile pager: animated switch to a profile | `switchProfile(windowId, id, animated=true)` calls `pageToProfile` → `ProfilePager.switchTo`. It animates only when a pager surface is attached (`this.active`), else it switches at once | Native pager position | Native settle | Native | none | Yes (native) | `apps/browser/src/state/browser/actions.ts:231-234`; `apps/browser/src/state/windows/profilePager.ts:152-161` (`switchPager(...)` at `:157`), `:190-192` | `docs/dia-spec.md:606` | 2.5 |
| B-005 | Profile pager: cycle to the next or previous profile, with wrap | `cycleProfile(windowId, ±1)`. It falls back to `switchProfile(..., animated=true)` when the native step fails | Native pager position | Native settle | Native | none | Yes (native) | `apps/browser/src/state/browser/actions.ts:236-245` (`stepPager(windowId, delta, true, order)` at `:239`) | `docs/dia-spec.md:606` | 2.5 |
| B-006 | Profile pager: external profile change | The window's `profileId` changes from outside the pager (store subscription). `pos.setValue(index)` jumps first, then the native pager selects the page | Native pager position (JS sets the start value with no animation) | Native | Native | none | Yes | `apps/browser/src/state/windows/profilePager.ts:56`, `:70-81` (`setValue` `:72`, `selectPager` `:80`) | - | 2.9 |
| B-007 | Window close (fade or close motion of the whole window) | `closeWindowAnimated(windowId, { replacedBy })`. Callers: close-window action (`apps/browser/src/state/browser/actions.ts:77`), Little Ardia (`apps/browser/src/state/littleArdia/actions.ts:168`, `:204`), last-window close (`apps/browser/src/state/windows/windowClose.ts:78`). The store removes the window when the native promise settles | Native window motion (`WindowMotion.swift`, [INFERENCE] from the file name) | Native | Native | none. `finishAnimatedCloses()` ends all pending closes at once on quit and on last-window paths (`apps/browser/src/state/app/nativeSync.ts:103`, `apps/browser/src/state/windows/lastWindow.ts:14`) | Native (AppKit) | `apps/browser/src/state/windows/closeAnimated.ts:26-55` (call `:53`), `:57-59`; `apps/browser/src/platform/windows.ts:20`; type `packages/shell/src/windows.ts:54` | - | 2.5 |
| B-008 | Window open entrance | `windowSync` opens a window after the first sync (`prev` is set), when the kind is not `small` and there is no `chromeWindow`. It passes `animated: true` | Native window entrance [INFERENCE: the curve is in Swift] | Native | Native | none | Native | `apps/browser/src/state/windows/windowSync.ts:82-86` (`animated: true` at `:84`); option type `packages/shell/src/windows.ts:40` | - | 2.9 |
| B-009 | Full-screen restore at launch | `windowSync` calls `port.restoreFullScreen(ids, focused)` for windows that were full screen | macOS full-screen transition [INFERENCE: system animation] | System | System | none | Native (AppKit) | `apps/browser/src/state/windows/windowSync.ts:108`; `apps/browser/src/state/app/nativeSync.ts:56`, `:150`; `apps/browser/src/platform/windows.ts:21` | - | 2.9 |
| B-010 | Window zoom (automation) | Control-socket op that sets the zoom state → `setWindowZoomed(id, on)` → `window.zoom(nil)` (`packages/shell/ios/WindowGeometry.swift:40`) | Window frame, macOS zoom animation [INFERENCE: system animation] | System | System | none | Native (AppKit) | `apps/browser/src/state/automation/socketWindowGeometry.ts:46`; `apps/browser/src/platform/windows.ts:34` | - | 5.5 |
| B-011 | Window full screen (automation) | Control-socket op → `setWindowFullScreen(id, on)` → `window.toggleFullScreen(nil)` (`packages/shell/ios/WindowGeometry.swift:66`). JS refuses the op while a transition is still busy | macOS full-screen transition (system) | System. JS waits up to 4000 ms (`FULL_SCREEN_WAIT_MS`); the limit is 6 s (`FULL_SCREEN_LIMIT_S`) | System | none | Native (AppKit) | `apps/browser/src/state/automation/socketWindowGeometry.ts:11-12`, `:83-85`; `apps/browser/src/platform/windows.ts:35` | - | 5.5 |
| B-012 | Tab drag preview: appears as a pill, then morphs into a card | The tab drag gets its first picture → `dragPreview.begin(windowId, chipRect, grabPoint)` | Native panel frame and shape (pill → card) | Spec: pill for 30 ms, then 60 ms into the card | Native | none | Native | `apps/browser/src/state/drag/tabDrag.ts:149-169` (`begin` `:153`, page picture at 0.35× `:159`, `:162`, `:166`, placeholder `:167`); `apps/browser/src/platform/windows.ts:41-48` | dia-spec.md "Dragging tabs and split targets" (`docs/dia-spec.md:866`, Ardia `:898-902`) | 2.32 |
| B-013 | Tab drag preview: shape change while dragging | `syncPreview()` on pointer or target change → `dragPreview.update(shape, pointer)`, with shape `hidden`, `pill` or `card`. Native `place(..., animated: true)` (`packages/shell/ios/DragPreview.swift:122`, `:126`) | Native panel frame and shape | Spec: ~50 ms pill morph over another strip (Dia) | Native | none | Native | `apps/browser/src/state/drag/tabDrag.ts:130-139` (`update` `:138`), `:183-188` | `docs/dia-spec.md:893-897` | 5.7 |
| B-014 | Tab drag preview: drop into a new window | `endTabDrag()` with an outside drop → `dragPreview.end(newWindowId, frame)`. Other ends with no target (`:194`, `:206`, `:210`, `:214`) and cancels (`:70`, `:227`) take the preview down | The card grows into the new window's frame, then fades | Spec: 200 ms grow, then a 100 ms fade-in | Native | none | Native | `apps/browser/src/state/drag/tabDrag.ts:190-223` (`end` `:219`), `:225-228` | `docs/dia-spec.md:904-905` | 2.32 |
| B-015 | Peek card close | `dismissPeek(windowId)` → registered `motion.close()` → `PeekStage.close(generation)`. With no handle, it finishes at once (`finishPeek`) | Native Peek stage (card, scrim, page) | Native (`PeekMotion.swift`, display link) | Native | none | Native | `apps/browser/src/state/peek/actions.ts:23-36`, `:38-44` (call `:42`); `packages/shell/src/peekStage.tsx:77` | peek-motion.md "Design" (`docs/peek-motion.md:7-13`) | 2.19 |
| B-016 | Peek "Open as Tab" promote | `openPeekAsTab(windowId)` → `previewPeekToolbar`, then `motion.promote()` → `PeekStage.promote(generation)`. If the motion does not start, it ends the toolbar preview. With no handle it calls `promotePeekNow` (no motion) | Native Peek stage: the page scale springs to 1 and the origin moves to the tab position (spec) | Native | Native spring (spec) | none | Native | `apps/browser/src/state/peek/actions.ts:74-81` (call `:80`), `:57-72`; `packages/shell/src/peekStage.tsx:78` | `docs/peek-motion.md:7-13` (Fixed viewport, Open as Tab) | 2.19 |
| B-017 | Peek card entrance | `PeekStage.begin(generation, animated = true)`. Component call site is out of scope. Entrance kind is `genie`, `affine` or `fade`. Props `reduceMotion` and `meshAllowed` select it | Native Peek stage | Native | Native (Genie mesh, affine zoom, or fade) | none | Native | `packages/shell/src/peekStage.tsx:22`, `:36-50`, `:76`; flag `apps/browser/src/lib/platformFeatures.ts:56`; kill switch `peekGenie` `apps/browser/src/platform/killSwitches.ts:68-72` | `docs/peek-motion.md:7` | absorb-1 |
| B-018 | Address-field load glow (field glow) | Native module `ArdiaFieldGlow`: `show(id, timings)`, `progress(id, p, still)`, `finish(id, still)`, `reset(id)`. Callers and `FIELD_GLOW_TIMINGS` are in `apps/browser/src/components/toolbar/FieldGlowFill.tsx:12-21` (out of scope) | Glow fill and opacity (`CAKeyframeAnimation`, `packages/shell/ios/FieldGlowCore/FieldGlowLayers.swift:154`, `:254`) | From the `timings` argument (values are set in the components scope) | Native keyframes | Native | Native | `packages/shell/src/views.tsx:214-235`; `apps/browser/src/platform/fieldGlow.ts:4`; `apps/browser/src/state/tabs/fieldGlow.ts:1-4` | - | 4.8 |
| B-019 | Intro music fade-out (audio, not visual) | `finishOnboarding()` → `stopIntroMusic(0.6)` | Audio volume, to 0 | 600 (the wrapper default is 400) | Native | none | Native | `apps/browser/src/state/onboarding/onboarding.ts:356`; `apps/browser/src/platform/introMusic.ts:9`; type `packages/shell/src/app.ts:127` | - | 2.28 |
| B-020 | Power-up band shader (`PowerUp`) | Mount of the `PowerUp` view (call sites are in components) | Metal shader band, frame-counted clock | Spec: finishes at T = 3.75 (≈3750 ms at speed 1.25) | Shader; `speed` prop. Native default is 1.25 (`packages/shaders/ios/PowerUpView.swift:7`) at 60 fps (`:38`) | none. Paused with Reduce Motion (`PowerUpView.swift:91`) | Native (Metal) | `packages/shaders/src/index.tsx:39-60` | dia-spec.md "Intro: power-up band + edge light" (`docs/dia-spec.md:456-460`) | 4.30 |
| B-021 | Meadow intro scene (Mac) | Mount of `Meadow` (the native `ArdiaMeadow` view plays a WebGL scene page, `packages/shaders/ios/Meadow/index.html`) | Shader scene | Native or page | Page | none | Native | `packages/shaders/src/meadow.tsx:10-13`; types `packages/shaders/src/index.tsx:93-116` | dia-spec.md "1.50 New Tab intro" (`docs/dia-spec.md:642`) | 4.30 |
| B-022 | Native activity spinner | Mount of `ActivitySpinner` | `transform.rotation.z`, 0 → ∓2π | 1880 (`packages/shell/ios/ActivitySpinner.swift:65`) | linear | Loops forever | Native (Core Animation) | `packages/shell/src/views.tsx:56-58` | - | 5.7 |
| B-023 | Native folder icon open/close | `FolderIcon` prop `open` changes | Layer `contents` keyframes (frames of the folder art) | `FolderArt.duration × |to − from|` (`packages/shell/ios/FolderIconView.swift:58`) | Native keyframes | none | Native (Core Animation) | `packages/shell/src/views.tsx:60-68` | - | 5.7 |
| B-024 | Element picker highlight box (injected into the page) | The pointer moves, an arrow key changes the target, or the page scrolls or resizes. On scroll, `requestAnimationFrame(show)` runs | CSS `left`, `top`, `width`, `height` | 60 (`PICKER_STYLE.motionMs`) | `ease-out` | none | No (CSS in the web page) | `apps/browser/src/state/site/elementPicker.ts:6` (`LOOK` = `PICKER_STYLE`), `:127`, `:210`, `:219-220`; `apps/browser/src/state/site/tokens.ts:9` | - | 2.20 |
| B-025 | Reader mode control bar (injected shadow DOM) | Hover or focus-within on `.bar` | CSS `opacity` .55 ↔ 1 | 150 (`BAR_FADE = "0.15s"`) | `ease` = cubic-bezier(0.25, 0.1, 0.25, 1) | none | No (CSS in the web page) | `apps/browser/src/state/reader/tokens.ts:38`, `:78-81` | - | 2.38 |
| B-026 | Media playback position ticker (JS step, no interpolation) | `useTicker(active, ms = 500)` while media plays | It re-renders so that the elapsed time and progress move forward | 500 per step | Step | `setInterval` while `active` | No (JS) | `apps/browser/src/state/media/media.ts:118-125` | - | 2.30 |

## Motion-timing gates (not animations, but they set when an animation starts, or how long it is held)

The components scope draws these animations. This scope sets the timers or holds that start them.

| ID | Gate | Value (ms) | What it gates | Source | Slice |
|---|---|---|---|---|---|
| G-01 | Live-folder completion hold | 2600 (`COMPLETION_MS`) | Completed live items stay in the list (`completing`) for the row's completion animation. `LiveItemRow` uses `Animated.delay(COMPLETION_MS - 420)` (`apps/browser/src/components/sidebar/LiveItemRow.tsx:53`, out of scope) | `apps/browser/src/state/live/engine.ts:25`, `:185`; `apps/browser/src/state/live/store.ts:261-268`; `apps/browser/src/state/live/engine.ts:142-143`, `:167-168` |
| G-02 | Sidebar tab hover card delays | show 800, hide 160, aim 200 | The hover card fade (in the components scope) | `apps/browser/src/state/sidebar/hover.ts:14`, `:24`, `:33`; spec `docs/dia-spec.md:248-252` |
| G-03 | Hover card suppressed after a sidebar peek arrives | 400 (`PEEK_ARRIVAL_MS`) | Rows that a sliding sidebar brings under the pointer do not open a card | `apps/browser/src/state/sidebar/hover.ts:16-23`; spec `docs/dia-spec.md:251-252` |
| G-04 | Hover card after scroll | 150 (`SCROLL_SETTLE_MS`) | No card until the sidebar stops scrolling | `apps/browser/src/state/sidebar/hover.ts:232`, `:240`, `:258` |
| G-05 | Top-strip hover card delays | show 500, leave 160 | The strip hover card | `apps/browser/src/state/tabs/stripHover.ts:11`, `:52`, `:61` |
| G-06 | Ctrl-Tab switcher mount and show | mount 70, show 140; pointer poll 50 | The switcher panel appears | `apps/browser/src/state/sidebar/switcher.ts:17-20`, `:42-48`, `:122-130` |
| G-07 | Lasting-load overlay | 150 (`SHOW_AFTER_MS`) | The load overlay shows only for loads that last longer than 150 ms | `apps/browser/src/state/tabs/lastingLoad.ts:7`, `:20-23`, `:41-45` |
| G-08 | Bookmarks-bar folder spring-open | 550 (`SPRING_OPEN_MS`) | A folder opens while a drag hovers over it | `apps/browser/src/state/bookmarks/barDrag.ts:40`, `:69-74` |
| G-09 | Sidebar slide and peek flags | flags only | `moving`, `peeks` and `leaving` per window, used by the sidebar slide and peek | `apps/browser/src/state/windows/dockMotion.ts:40-84` |
| G-10 | Interface-scale relayout | 120 (`RELAYOUT_MS`) | Debounce before relayout | `apps/browser/src/state/settings/interfaceScale.ts:9`, `:34-37` |
| G-11 | Press-motion setting | `"still"` (default) or `"bounce"` | Selects the press spring in `components/ui/motion.ts` | `apps/browser/src/lib/pressMotion.ts:1-19`; `apps/browser/src/state/settings/settings.ts:156`; `apps/browser/src/state/settings/nativeSettings.ts:23` |
| G-12 | Animation kill switches | `sidebarSlide` (live), `sidebarSlideLean`, `peekGenie` (live) | Turn off the sidebar slide or the Genie Peek | `apps/browser/src/platform/killSwitches.ts:50-72` |
| G-13 | Selected-tab glow, fade unloaded tabs (static styles) | `"full" \| "soft" \| "off"`, `false` | Static looks, not animations | `apps/browser/src/state/settings/settings.ts:117`, `:127`; `apps/browser/src/state/sidebar/rowState.ts:18`, `:31` |

## Out of scope but seen (Windows only, or test doubles)

| File | Why it is excluded |
|---|---|---|
| `packages/shaders/src/meadowPage/shim.ts` | The Windows intro page script. It is bundled into the Windows engine page (`packages/shaders/scripts/meadow.mjs:12-17`) and used only by `packages/expo-windows/src/meadow/pageUrl.ts:4,49`. It has WAAPI animations: `target.animate(...)` `:121` (icon `:134`, letters `:136`, tagline `:139`, exit `:144`, skip `:153`), a canvas handoff transition `:61` (`MEADOW_HANDOFF` 400 ms `cubic-bezier(0.37,0,0.63,1)`, `packages/shaders/src/meadowPage/config.ts:8`), and a rAF frame counter `:29`, `:230`. The Mac Meadow ignores `intro` (`packages/shaders/src/meadow.tsx:10`). |
| `packages/shaders/src/meadow.windows.tsx`, `apps/browser/src/platform/motion.windows.ts`, `apps/browser/src/platform/animatedCleanup.windows.ts`, `apps/browser/src/lib/platformFeatures.{windows,linux}.ts` | Windows or Linux variants. |
| `apps/browser/src/test-native/*` (e.g. `apps/browser/src/test-native/shell.ts:113` `animateWindowClose: () => Promise.resolve()`) | Test doubles for the native layer. |

## How I searched

Scope roots `S` = `apps/browser/src/{state,lib,platform}`, `apps/browser/src/App.tsx`, `apps/browser/src/main.tsx`, `packages/{shell,core,shaders,ardiacore,import,sync}/src`. All searches use `rg --glob '!*.test.*' --glob '!*.json' <pattern> $S` (the counts are matching lines):

| Pattern | Hits | Files |
|---|---|---|---|
| `Animated\.` | 5 | 3 (profilePager, sharedTiles, ardiacore swipe) |
| `nativeLoop` | 0 | 0 |
| `LayoutAnimation` | 0 | 0 |
| `requestAnimationFrame` | 7 | 5 |
| `useNativeDriver` | 1 | 1 |
| `Easing` | 0 | 0 |
| `\.spring\(\|springs\.` | 0 | 0 |
| `\.animate\(` | 1 | 1 (meadowPage/shim, Windows) |
| `transition` | 7 | 5 |
| `@keyframes\|animation:` | 2 | 2 |
| `scrollIntoView\|scrollTo\(\|behavior:` | 4 | 3 (none smooth) |
| `setNativeProps` | 0 | 0 |
| `interpolate` | 2 | 1 |
| `setInterval\(` | 10 | 10 |
| `(?i)animat` | 94 | 30 |
| `(?i)motion` | 117 | 36 |
| `(?i)glow` | 17 | 8 |
| `(?i)fade` | 26 | 11 |
| `(?i)spring` | 18 | 3 |
| `animated: true\|animated\?\|animated =` | 5 | 4 |

Other searches:
- `rg -e 'fieldGlow[?]?\.' -e 'animateWindowClose' -e 'dragPreview…' -e 'restoreFullScreen\(' -e 'stopIntroMusic\(' -e 'hapticTick\('` over `apps/browser/src` and `packages`. These found the native call sites for B-007 to B-019.
- `rg '^(export )?const [A-Z_]*(MS|_S|SECONDS|DURATION|DELAY)\b\s*='` over state, lib and platform: about 230 constants. I read every motion-related one (G-01 to G-10). The others are network, telemetry or persistence timers.
- `rg 'ui/motion|from "../../components'` in scope: 0 hits. No motion tokens to resolve.
- Doc headings: `grep -n '^#'` in `docs/dia-spec.md`, `docs/peek-motion.md` and `docs/omnibox-ux.md`, then keyword greps (pill, glow, window close, complet). `docs/omnibox-ux.md` has no motion section that matches a row.
- App.tsx and main.tsx: `grep -c -i 'animat|motion|transition|requestAnimationFrame'` gives 0 and 0.

### Per-file coverage (every file with a hit in the union search, 72 files)

| File | Rows, or why it is not an animation |
|---|---|---|
| apps/browser/src/state/windows/profilePager.ts | B-001, B-003, B-004, B-006 |
| apps/browser/src/state/sidebar/sharedTiles.ts | B-002 |
| packages/ardiacore/src/swipe.tsx | B-001 (animated native component `:84`, `:106`); pager calls B-003 to B-006 (`:88-92`); `swipeHaptic` is a haptic |
| apps/browser/src/state/browser/actions.ts | B-004, B-005, B-007 (`:77`) |
| apps/browser/src/state/windows/closeAnimated.ts | B-007 |
| apps/browser/src/state/littleArdia/actions.ts | B-007 (callers `:168`, `:204`) |
| apps/browser/src/state/windows/windowClose.ts | B-007 (caller `:78`) |
| apps/browser/src/state/windows/lastWindow.ts | B-007 (`finishAnimatedCloses` `:14`) |
| apps/browser/src/state/app/nativeSync.ts | B-007 (`:103`), B-009 (`:56`, `:150`) |
| apps/browser/src/state/windows/windowSync.ts | B-008, B-009 |
| apps/browser/src/platform/windows.ts | B-007, B-009 to B-014 |
| packages/shell/src/windows.ts | B-007, B-008 (types); `transition` field `:44` is a geometry status |
| apps/browser/src/state/automation/socketWindowGeometry.ts | B-010, B-011 |
| apps/browser/src/state/peek/actions.ts | B-015, B-016; `hapticTick` `:70`, `:224` is a haptic |
| packages/shell/src/peekStage.tsx | B-015, B-016, B-017 |
| apps/browser/src/lib/platformFeatures.ts | B-017 (flag `:56`) |
| apps/browser/src/platform/killSwitches.ts | B-017, G-12 |
| packages/shell/src/views.tsx | B-018, B-022, B-023. `FadeLabel`, `EdgeFadeMask` and `Surface` fade props are static gradients, not animations |
| apps/browser/src/platform/fieldGlow.ts | B-018 |
| apps/browser/src/state/tabs/fieldGlow.ts | B-018 |
| apps/browser/src/platform/introMusic.ts | B-019 |
| packages/shell/src/app.ts | B-019 (type `:127`) |
| packages/shaders/src/index.tsx | B-020, B-021; `WindowBackdrop` and `ThemeSwatch` are static (`WindowBackdropView.swift:23` `isPaused = true`) |
| apps/browser/src/state/site/elementPicker.ts | B-024 |
| apps/browser/src/state/site/tokens.ts | B-024 |
| apps/browser/src/state/reader/tokens.ts | B-025 |
| apps/browser/src/state/live/engine.ts | G-01 |
| apps/browser/src/state/live/store.ts | G-01 |
| apps/browser/src/state/sidebar/hover.ts | G-02, G-03, G-04 |
| apps/browser/src/state/windows/dockMotion.ts | G-09 (state flags and traffic-light positions only) |
| apps/browser/src/state/bookmarks/barDrag.ts | G-08 |
| apps/browser/src/lib/pressMotion.ts | G-11 |
| apps/browser/src/state/settings/nativeSettings.ts | G-11 |
| apps/browser/src/state/settings/settings.ts | G-11, G-13 |
| apps/browser/src/state/sidebar/rowState.ts | G-13 (static `FADED` mark) |
| apps/browser/src/state/app/theme.ts | Setting reader for `selectedTabGlow` (G-13) |
| apps/browser/src/state/settings/configAllowlist.ts | Setting names only |
| apps/browser/src/state/app/transientUi.ts | Imports `peekShown` (flag read) |
| apps/browser/src/state/omnibox/barPlacement.ts | Imports `peekShown` (flag read) |
| apps/browser/src/state/sidebar/sidebarUi.ts | Imports `peekShown` (flag read) |
| apps/browser/src/state/windows/layout.ts | Imports traffic-light centres from dockMotion (static positions) |
| apps/browser/src/state/engines/engineSwap.ts | Double rAF waits for paint, and an instant `scrollTo` (`:50`, `:52`). Not animations |
| apps/browser/src/state/telemetry/profileSwap.ts | Double rAF to measure the swap time (`:62-63`). Not an animation |
| apps/browser/src/state/app/perfProbe.ts | Wraps timers, including rAF, for perf counts (`:99`) |
| apps/browser/src/state/app/perfNativeNodes.ts | Counts native Animated nodes (perf) |
| apps/browser/src/platform/animatedCleanup.ts | Patch to React Native's Animated cleanup order. Not an animation |
| apps/browser/src/platform/motion.ts | No-op `animationBursts` on Mac (`:3`) |
| packages/shell/src/motion.ts | Types for Windows animation-burst telemetry |
| packages/shell/src/index.tsx | Re-export `./motion` (`:24`) |
| apps/browser/src/state/telemetry/animationBursts.ts | Telemetry |
| apps/browser/src/state/telemetry/copy.ts | Telemetry text |
| apps/browser/src/state/telemetry/hourlyExtras.ts | Telemetry |
| apps/browser/src/state/telemetry/introCounts.ts | Telemetry (`"animated"` is a scene name) |
| apps/browser/src/state/telemetry/mediaCardCounts.ts | Telemetry (`motionStarted`) |
| apps/browser/src/state/telemetry/peekMotion.ts | Telemetry for B-017 entrance kinds |
| apps/browser/src/state/telemetry/platformCounts.ts | Telemetry |
| apps/browser/src/state/telemetry/usage.ts | Telemetry counters (`sidebar_motion_merges` and others) |
| apps/browser/src/state/plugins/lifecycle.ts | `PROMOTION_RETRY` (word match "motion") |
| apps/browser/src/state/site/permissions.ts | "motion sensors" label |
| apps/browser/src/state/site/selection.ts | Instant `scrollIntoView` (`:151`, `:167`) |
| apps/browser/src/state/reader/readerPage.ts | Instant `scrollIntoView` (`:176`) |
| apps/browser/src/state/reader/readabilitySource.ts | Vendored Readability ("dailymotion" regex) |
| apps/browser/src/lib/emojiData.ts | Emoji names |
| packages/core/src/bangList.ts | Search-bang names |
| packages/core/src/anonymousIdentity.ts | Word list ("Glowing") |
| apps/browser/src/lib/platformFeatures.linux.ts, apps/browser/src/lib/platformFeatures.windows.ts | Not Mac (see the excluded table) |
| apps/browser/src/platform/motion.windows.ts | Not Mac |
| packages/shaders/src/meadow.windows.tsx | Not Mac (`fadeDuration={0}`) |
| packages/shaders/src/meadowPage/shim.ts | Not Mac (see the excluded table) |

Other in-scope files that I read with no union hit, and that hold motion data: `apps/browser/src/state/drag/tabDrag.ts` (B-012 to B-014), `apps/browser/src/state/onboarding/onboarding.ts` (B-019), `packages/shaders/src/meadow.tsx` (B-021), `apps/browser/src/state/media/media.ts` (B-026), `apps/browser/src/state/tabs/stripHover.ts` (G-05), `apps/browser/src/state/sidebar/switcher.ts` (G-06), `apps/browser/src/state/tabs/lastingLoad.ts` (G-07), `apps/browser/src/state/settings/interfaceScale.ts` (G-10), `apps/browser/src/platform/pager.ts` (re-export of B-003 to B-006). `packages/import/src` and `packages/sync/src` have no hits.

---

# Anim C: native animations in the Mac app (Swift / ObjC / ObjC++ / Metal / engine)

Source: Ardia at commit 551d3c13c. All paths are relative to the Ardia repo root.

## What the Mac app builds

| Unit | Evidence | Included here |
|---|---|---|---|
| `ArdiaShell` pod: `packages/shell/ios/**/*.{h,m,mm,swift}`, macOS 14 | `packages/shell/ios/ArdiaShell.podspec:8,18` | Yes |
| `ArdiaShaders` pod: `packages/shaders/ios/**/*.{h,m,swift}` + `Meadow/*` resource bundle | `packages/shaders/ios/ArdiaShaders.podspec:8,14,15` | Yes. Metal source is inline Swift strings. `Meadow/index.html` is a bundled WebGL page built from `apps/site/src/scripts/meadow` (`packages/shaders/scripts/meadow.mjs:4-9`). |
| `ArdiaCore` pod: `packages/ardiacore/ios/*.{h,c,mm,swift}` | `packages/ardiacore/ios/ArdiaCore.podspec:8,13` | Yes |
| `ArdiaImport`, `ArdiaSync` pods | `packages/import/ios/ArdiaImport.podspec:15`, `packages/sync/ios/ArdiaSync.podspec:14` | Swept: no animation |
| App target `apps/browser/macos/Ardia-macOS` (`AppDelegate.swift`, `BrowserWindow.swift`, `main.swift`, `DisableFusebox.mm`) | Podfile paths at `apps/browser/macos/Podfile.lock:2414-2422` | Swept: no animation |
| Engine `engine/ardiacore/src` | 65 `.mm` files | Swept: no animation. `ACFindBar::Show/Hide(bool animate)` ignore the flag (`engine/ardiacore/src/ardia/core/ac_find_bar.mm:17-22`). |
| Engine Chromium patch `engine/chromium/src/chrome/browser/ardia` | `pip/ac_pip_mac.mm` is added only `if (is_mac)` (`engine/chromium/src/chrome/browser/ardia/BUILD.gn:68-73`) | Yes (PiP fades). `ac_pip_linux.cc` is Linux-only: skipped. |
| `packages/phone/ios` | `packages/phone/ios/ArdiaPhoneNative.podspec:13` (`:ios => '27.0'`) | Skipped (phone) |
| `packages/shell/docktile`, `packages/shell/plugin-host` | separate `build.sh` products | Swept: no animation |

Notation. $e_{out3}(t)=1-(1-t)^3$. $\text{smooth}(t)=t^2(3-2t)$. A "critically damped spring, response r" uses $\omega=2\pi/r$.

## Inventory

| ID | Surface | Trigger | Properties | Duration | Curve / spring (numbers) | Delay / loop | Source | Spec | Slice |
|---|---|---|---|---|---|---|---|---|---|
| C-001 | Tab loading spinner ring (`ArdiaActivitySpinner`) | View enters a window (`ActivitySpinner.swift:28-32`) | `transform.rotation.z` 0 → −2π (unflipped) or +2π (flipped) | 1.88 s | CABasicAnimation, linear | `repeatCount = .infinity`, not removed on completion | `packages/shell/ios/ActivitySpinner.swift:60-69` | `docs/dia-spec.md:588-590` (1.50 delta #8, section at `:541`) | 5.7 |
| C-002 | Browser window entrance (frame view layer) | Window reveal: `WindowManager.reveal` (`packages/shell/ios/WindowManager.swift:207-210`) | `opacity` 0→1; `transform` scale 0.98 (main) / 0.97 (little) → identity, scaled about bounds centre | 0.25 s main / 0.2 s little | CAAnimationGroup, cubic-bezier(0.23, 1, 0.32, 1); `fillMode = .backwards` | Shadow dropped, restored on completion or at 0.5 s fallback. Scale skipped when Reduce Motion is on or `scales == false`. | `packages/shell/ios/WindowMotion.swift:9-15,27,35-41,57-64,66-96` | - | 5.5 |
| C-003 | Browser window exit | Window close (`WindowManager.swift:397`) | `opacity` presentation → 0; `transform` presentation → scale 0.99 (main) / 0.985 (little) | 0.18 s main / 0.15 s little | Same group and curve as C-002 | `orderOut` in completion, or at 0.5 s fallback. Scale skipped with Reduce Motion. | `packages/shell/ios/WindowMotion.swift:9-10,98-135` | - | 5.5 |
| C-004 | Window replaced by another (Little Ardia handoff) | Close with a replacement id | None: old window stays, then `orderOut` | 0.2 s hold (`Style.little.entranceDuration`) | - | After replacement is revealed | `packages/shell/ios/WindowManager.swift:389-395` | - | 5.5 |
| C-005 | Tab drag preview panel: pill → card morph | Drag shape changes (`update(shape:at:)`) | Panel frame + alpha | 0.06 s (`morph`); pill held 0.03 s first (`pillMs`) | Hand-coded: `Timer` 1/120 s, $e_{out3}$; retargets to latest `desired` frame each tick | - | `packages/shell/ios/DragPreview.swift:8-9,99-130,204-225,255-270` | `docs/dia-spec.md:900-901` (section `:866`) | 5.7 |
| C-006 | Drag preview grows into new window | Drop away from a target (`end`) | Panel frame → new window frame, alpha → 1 | 0.2 s (`grow`) | Same timer, $e_{out3}$ | Polls for the window every 0.016 s, up to 20 tries, then grows to fallback rect | `packages/shell/ios/DragPreview.swift:10,146-161` | `docs/dia-spec.md:904-905` | 5.7 |
| C-007 | Drag preview fade-out | After grow and window reveal, or no window id | Panel alpha → 0 | 0.1 s (`fadeOut`) | Same timer, $e_{out3}$ | `orderOut` at end | `packages/shell/ios/DragPreview.swift:11,141-146` | `docs/dia-spec.md:905` (spec text says "fades in 0.1 s") | 5.7 |
| C-008 | Folder icon open/close (`FolderIconView`) | `wantsOpen` prop changes | `contents` (pre-rendered CGImage frames of the folder art) | 0.2 s × \|to − from\| | CAKeyframeAnimation, `calculationMode = .discrete`, 240 frames/s; shape mix uses smooth(progress) | Resumes mid-motion via `beginTime`; skipped with Reduce Motion or no window | `packages/shell/ios/FolderIconView.swift:49-71,97-106`; `packages/shell/ios/FolderArt.swift:17-18,22,63-64,77` | - | 5.7 |
| C-009 | Address-field load glow: show | `FieldGlow.show(id, timings)` from `apps/browser/src/components/toolbar/FieldGlowFill.tsx:12,18` | `slide.opacity` 0 → 1 (filled and comet reset to 0) | 200 ms (`show`) | CAKeyframeAnimation linear, values sampled 120/s from $e_{out3}$ (`.enter`) | Only when the view has a window (`packages/shell/ios/FieldGlow.swift:31`) | `packages/shell/ios/FieldGlowCore/FieldGlowModel.swift:10-16,62,81-85,133-136`; `FieldGlowLayers.swift:260-308` | - | 4.8 |
| C-010 | Load glow: progress fill | Progress event | `slide.position.x` → offset of filled width | 250 ms (`progress`); 0 when `still` | Keyframes sampled 120/s, `.enter` ($e_{out3}$); on resize the run is resampled from current value over the remaining time | - | `FieldGlowModel.swift:87-91`; `packages/shell/ios/FieldGlowCore/FieldGlowLayers.swift:137-159,245-259` | - | 5.7 |
| C-011 | Load glow: finish fill | Load finishes (not still) | `slide.position.x` → full | 300 ms (`finish`) | Keyframes 120/s, `.enter` | - | `FieldGlowModel.swift:100-102`; `FieldGlowLayers.swift:245-259` | - | 5.7 |
| C-012 | Load glow: comet cross-fade | Load finishes (not still) | `comet.opacity` 0→1 and `steady.opacity` 1→0 | 300 ms (`finish`) | Keyframes 120/s, `.enter` | - | `FieldGlowModel.swift:102`; `FieldGlowLayers.swift:309-321` | - | 5.7 |
| C-013 | Load glow: hide | Load finishes | `slide.opacity` → 0 | 250 ms (`hide`) | `.exit` = $t^2$, sampled 120/s; one keyframe track with `keyTimes` holds the value through the delay | Delay = finish + hold = 300 + 200 ms (hold 200 ms only when `still`). Completion delegate ends the phase. | `FieldGlowModel.swift:14,62,93-105`; `FieldGlowLayers.swift:266-299,325-329` | - | 5.7 |
| C-014 | Peek card: Genie entrance (mesh warp) | Peek begins and page is ready, or after 0.15 s ready timeout | Mesh grid of 37 rows × 5 columns (`rows = 36`, `columns = 4`) moving from link to card; scrim alpha | Per row until within 1 pt (`restWithin`) | Each row: $\text{remaining}(t,r)=(1+\omega t)e^{-\omega t}$, $\omega=2\pi/r$; $r=0.2\,(1-0.5g+g\,n)$, $n=\sqrt{1-k}$ (or $1-\sqrt{k}$ for $g<0$); stagger $g$ from link height, 0.75 below middle; neck width 0.12 × card; scrim $1-\text{remaining}(t,0.26)$ | Driven per frame (see C-024 note) | `packages/shell/ios/PeekMotion.swift:14,16-24,325-334,372-381,433-469`; `packages/shell/ios/PeekSpring.swift:41-45` | `docs/peek-motion.md:41-47` | 5.6 |
| C-015 | Peek card: affine entrance (no mesh, full screen, or fallback) | Same as C-014 when Genie is not possible | `presence` 0 → 1: scale lerp(0.12, 1), centre lerp source → card; card alpha = presence/0.35 (clamped) | Spring until settled (1e-3) | Critically damped spring, response 0.26; integrator `step`: $d=(d_0+(v_0+\omega d_0)dt)e^{-\omega dt}$ | - | `PeekMotion.swift:25,40,115-121,239-242,339-353,382-385`; `PeekSpring.swift:26-35` | `docs/peek-motion.md:3-4` | 5.6 |
| C-016 | Peek: Reduce Motion entrance | Entrance with Reduce Motion | Card alpha 0 → 1 | 0.15 s (`fadeSeconds`) | Linear | - | `PeekMotion.swift:15,243,375-377,494-497` | `docs/peek-motion.md:96` (P15) | 5.6 |
| C-017 | Peek: Seamless hand-over after an interrupted Genie | Pinch, close or promote during Genie | Mesh residual × `warp` 1 → 0; alpha/scrim residuals | Spring until 1e-3 | Critically damped, response 0.22; start velocity = Genie rate sampled 1/240 s back | - | `PeekMotion.swift:41,362-368,388-410` | `docs/peek-motion.md:55` | 5.6 |
| C-018 | Peek: pinch gesture | Trackpad magnify | `m` follows fingers, clamped to [−1, 1.2]; spread maps rest → page via rubber(); chrome alpha $1-1.6e$ | Live | Velocity filter $0.6v+0.4\Delta/dt$; rubber knee $2-1.2=0.8$, then $knee+(1-knee)(2u-u^2)$; spread-to-tab 0.4; pinch-in scale ≥ 0.5 | - | `PeekMotion.swift:11-13,26,186-212,336-353,513-518` | `docs/peek-motion.md:39-40` | 5.6 |
| C-019 | Peek: settle back after pinch | Pinch ends between thresholds (open ≥ 0.3, close ≤ −0.25) | `m` → 0 | Spring until settled | Critically damped, response 0.2 | - | `PeekMotion.swift:9-10,214-226,248-252` | `docs/peek-motion.md:39-40` | 5.6 |
| C-020 | Peek: Genie close | Close (×, ⌘W, Esc, pinch-in) when mesh allowed | Rows reverse into link; card alpha = closeAlpha·smooth((1−min row)/0.4); scrim × remaining(t, 0.26) | Rows until settled | Row spring base response 0.17 (`genieCloseResponse`) with stagger | - | `PeekMotion.swift:19,24,143-162,253-256,305-323,412-425` | `docs/peek-motion.md:47-48` | 5.6 |
| C-021 | Peek: affine close | Close without mesh | `presence` → 0, `m` → min(m, 0) | Until presence ≤ 0.02 | Critically damped, response 0.2 | - | `PeekMotion.swift:157-160,257-260` | - | 5.6 |
| C-022 | Peek: Reduce Motion close | Close with Reduce Motion | Card alpha current → 0 | 0.15 s | Linear | - | `PeekMotion.swift:153-154,260,494-497` | `docs/peek-motion.md:96` | 5.6 |
| C-023 | Peek: Open as Tab (promote) | Button, ⌘O, or spread past 0.3 | `m` → 0.48 (`spreadToTab 0.4 × fullAt 1.2`); `presence` → 1 | Spring until settled (4e-4 / 1e-3) | Critically damped, response 0.22. Reduce Motion: jump, scrim 1 → 0 linear over 0.15 s | - | `PeekMotion.swift:11-13,164-184,262-270,354-359` | `docs/peek-motion.md:16-18,34-37` | 5.6 |
| C-024 | Window drag pacer (custom window move) | Title-bar drag handled by `WindowMove` (`packages/shell/ios/WindowMove.swift:146`) | Window origin, placed once per display frame with the newest pointer | Live | No easing: coalesces moves; display link 60…max fps; falls back to direct placement after 0.05 s with no frame | - | `packages/shell/ios/WindowMovePacer.swift:9-40`; `packages/shell/ios/UpdaterCore/WindowDragPacing.swift:4,13-42` | - | 5.5 |
| C-025 | Intro music volume ramp (audio, not visual) | Mute toggle; `stopIntroMusic(fade)` | `mainMixerNode.outputVolume` | 0.25 s mute; stop 0.4 s default, 0.35 s from Intro | `Timer` 1/60 s, smooth(t) | - | `packages/shell/ios/IntroMusic.swift:57-67,100-115`; `apps/browser/src/platform/introMusic.ts:9`; `apps/browser/src/components/onboarding/Intro.tsx:104` | - | 5.8 |
| C-026 | Intro music baked tail fade (audio) | Buffer synthesis | Sample gain | Last 1.5 s | Raised cosine $0.5+0.5\cos(\pi j/n)$ | - | `packages/shell/ios/IntroMusic.swift:212,341-349` | - | 5.8 |
| C-027 | Power-up band (Metal, `powerUpFragment`) | `PowerUpView` enters a window or `replay()`; NTP intro and onboarding | Fragment colour/alpha over full view; layer has CAFilter `gaussianBlur` radius 24 | Clock runs to 3.75 s, then pauses | `time += 1/60` per draw at 60 fps. $t=0.75\cdot time\cdot speed+0.45-delay$. Fade-in `clamp(time·10)` (0.1 s). Fade-out after $t>fadeOutStart$ (1.0) over `fadeOutDuration` (2.0). Bands lag 0.025/0.05 by column. Trail edge uses `easeOutExpo`. Palette in OKLab → sRGB; alpha × 0.1. Uniform buffers: 0 direction, 1 count, 2 colors, 3 time, 4 speed (1.25 default), 5 delay (0), 6 fadeOutStart, 7 fadeOutDuration, 8 origin (0.5) | Paused when no window or Reduce Motion | `packages/shaders/ios/PowerUpView.swift:6-13,33-45,89-97,106-128,275-277,344-386,388-434` | `docs/dia-spec.md:456-461` | 4.30 |
| C-028 | Edge light halo (Metal, `haloFragment`) | `PowerUp` gets a `halo` frame (NTP bar) | Rim line + glow around rounded rect (inset 50, radius = PowerUp radius + 2) | Clock runs to 2 s | $t=0.8\cdot speed\cdot time-delay$, delay 0.18; sweep $=1.25(1-2^{-10t})$; glow shrink $(1-t)\cdot0.95$; noise `haloNoise(p·0.5, time)`; fade after 1.0 over 0.2. Buffers 0–10: resolution, direction, colors, isDark, time, speed, delay, fadeOutStart, fadeOutDuration, inset, cornerRadius | Paused with no window or Reduce Motion | `packages/shaders/ios/PowerUpView.swift:61-80,133-192,221-271` | `docs/dia-spec.md:462-466` | 4.30 |
| C-029 | Meadow web view hand-off (onboarding) | Page posts `ready` | `webView.alphaValue` 0 → 1 (poster hidden after) | 0.4 s | NSAnimationContext, cubic-bezier(0.37, 0, 0.63, 1). Page canvas uses the same CSS transition (`MEADOW_HANDOFF`) | - | `packages/shaders/ios/MeadowView.swift:24,96,164-176`; `packages/shaders/src/meadowPage/config.ts:8`; `packages/shaders/src/meadowPage/shim.ts:61` | - | 4.30 |
| C-030 | Meadow page: sunrise | Page start | Day-cycle position `day.c` | 300 ms hold + 2850 ms | easeInOutSine $(1-\cos\pi t)/2$ | Hold 300 ms | `apps/site/src/scripts/meadow/clock.ts:1-2,20-27`; `apps/site/src/scripts/meadow/scene.ts:881-884` | - | 4.30 |
| C-031 | Meadow page: travel / next stop | `travelTo` or advance | `day.c` | 2850 ms | easeInOutSine; Reduce Motion jumps | - | `apps/site/src/scripts/meadow/scene.ts:847-878,888-889` | - | 4.30 |
| C-032 | Meadow page: drag sun | Pointer drag | `day.c` follows target | Live | Exponential follow $1-e^{-18\,dt}$ | - | `apps/site/src/scripts/meadow/scene.ts:138,898` | - | 4.30 |
| C-033 | Meadow page: paint reveal | First paint / replay | `paintReveal` 0 → 1 | 900 ms | Linear | - | `apps/site/src/scripts/meadow/scene.ts:157,286-287,996` | - | 4.30 |
| C-034 | Meadow page: look parallax | Pointer hover | `look.x/y` | Live | $1-e^{-4\,dt}$ | Off with Reduce Motion | `apps/site/src/scripts/meadow/scene.ts:1003-1008` | - | 4.30 |
| C-035 | Meadow page: title mist | Tone change | `titleMist` | Live | $1-e^{-dt/1.2}$ | Jumps with Reduce Motion | `apps/site/src/scripts/meadow/scene.ts:148,1055` | - | 4.30 |
| C-036 | Meadow page GLSL time effects | Each frame; `uTime = windTime`, which accrues `dt` while wind is on | Sky band wobble `uTime·0.004`; cloud drift `(0.006, 0.0015)·uTime`; star twinkle `sin(3·uTime + hash)`; warp `0.01·uTime`; grass gust `0.85·uTime`; ripple `1.9·uTime`; flutter `sin((2.4+1.6h)·uTime)`; grain `fract(3.7·uTime)` | Continuous | Ambient frame cap 1000/30 ms | Loop | `apps/site/src/scripts/meadow/shaders.ts:45,482,491,501,523,601,603-604,1023`; `apps/site/src/scripts/meadow/scene.ts:134,983,1307` | - | 4.30 |
| C-037 | Meadow page: grass brush spring | Press / drag on grass | Per-cell bend | Live | Stiffness $520\cdot pressed+rate^2$, damping $46\sqrt{pressed}+2\,rate$; rate 7 (1 while held 0.85 s); glow decay $e^{-dt/3.4}$ | - | `apps/site/src/scripts/meadow/brush.ts:9-16,59,86-92` | - | 4.30 |
| C-038 | Meadow page: particles | Night / taps / drags | 34 fireflies (blink $\max(0,\sin(1.7\,speed\,t))^2$), ≤ 90 petals, ≤ 70 sparks (life 0.45–1.05 s) | Continuous | Hand-coded velocity integration | Loop | `apps/site/src/scripts/meadow/particles.ts:1-3,59-75,137-139,160-162,198-200` | - | 4.30 |
| C-039 | Meadow page: smear heal | After brushing | Smear field | Live | $e^{-dt/0.45}$ | - | `apps/site/src/scripts/meadow/smear.ts:1,24` | - | 4.30 |
| C-040 | Meadow page: creatures | Spawn / leave | `presence`, yaw | Live | Presence $1-e^{-dt/0.75}$; turn 2.4 rad/s; respawn delay 0.6 s | - | `apps/site/src/scripts/meadow/mobs.ts:25-30,603,610,1129` | - | 4.30 |
| C-041 | Meadow page: intro icon / letters / tagline / button | Intro config from app JS | WAAPI `element.animate(keyframes, {delay, duration, easing, fill:"both"})` | From `MeadowMotion` config (TS side) | From config | From config | `packages/shaders/src/meadowPage/shim.ts:120-124`; `packages/shaders/src/index.tsx:93,105` | - | 4.30 |
| C-042 | PiP window corner bend | `fitCorners:YES` on screen change, window move, slide end, throw end | `cornerRadius` of content and rim layers; free 6 pt, in a corner 9 pt | 0.2 s (`kCornerBendDuration`) | CABasicAnimation, easeInEaseOut | Shadow invalidated on completion | `packages/ardiacore/ios/ArdiaPiPController.mm:234-260,297,307,424,491`; `packages/ardiacore/ios/ArdiaPictureInPictureInternal.h:19`; `packages/ardiacore/ios/ArdiaPiPThrowMath.h:21-23` | `docs/dia-spec.md:1005,1101` | 3.10 |
| C-043 | PiP stash handle | Window tucked at / pulled from a screen edge | Handle `alphaValue` 0 ↔ 1 | 0.25 s | NSAnimationContext default timing | Hidden on completion when unstashed | `packages/ardiacore/ios/ArdiaPiPController.mm:389-405` | `docs/dia-spec.md:1082` | 3.10 |
| C-044 | PiP slide (`animateTo:`) | Rescue when lost, stash at edge, untuck with Reduce Motion or throw off | Window frame origin (rounded) | 0.25 s | `NSTimer` 1/120 s, $e_{out3}$ | Corner bend (C-042) at end | `packages/ardiacore/ios/ArdiaPiPController.mm:351-372,384,407-428` | - | 3.10 |
| C-045 | PiP throw / fling | Drag or swipe release above 350 pt/s; untuck | Window origin, per axis | Until resolved (test: 1.5–3.5 s at 60 Hz) | Analytic damped spring: response 0.9, ratio 0.9, $\omega_0=2\pi/0.9$, $\omega_d=\omega_0\sqrt{1-0.81}$, envelope $e^{-0.9\omega_0 t}$; resolved when \|offset\| and \|v\| < 0.2 on both axes; `NSTimer` 1/120 s | Trace capped at 1200 frames | `packages/ardiacore/ios/ArdiaPiPController.mm:437-495`; `packages/ardiacore/ios/ArdiaPiPController+Swipe.mm:220`; `packages/ardiacore/ios/ArdiaPiPThrowMath.h:24,28,291-308` | `docs/dia-spec.md:1054,1062-1065,1105` | 3.10 |
| C-046 | PiP wheel zoom ease | ⌘-scroll with a non-precise (wheel) device | Window frame width (aspect kept, pointer anchored) | Until \|target − width\| < 0.25 pt | `NSTimer` 1/120 s, step 25 % of remaining width per tick; scroll factor 0.004/pt, 0.08/line | Precise (trackpad) scroll steps at once | `packages/ardiacore/ios/ArdiaPiPZoom.mm:118-124,455-490,507-517`; `packages/ardiacore/ios/ArdiaPiPZoomMath.h:22` | `docs/dia-spec.md:1048` | 3.10 |
| C-047 | PiP pinch stand-in | Trackpad pinch on PiP | Stand-in panel card frame + mirror scale; real window alpha 0.01 | Live; hand-off linger 0.25 s, timeout 0.6 s | CADisplayLink; follow $1-e^{-dt/0.05}$, dt ≤ 0.05; snaps within 0.5 pt | - | `packages/ardiacore/ios/ArdiaPiPZoom.mm:242,315-377,396-407`; `packages/ardiacore/ios/ArdiaPiPZoomMath.h:25-27` | - | 3.10 |
| C-048 | Page cover uncover (full-screen transition) | Page drawn after full-screen change, + 0.034 s | Cover view `alphaValue` → 0, then removed | 0.1 s | NSAnimationContext default timing | 34 ms delay | `packages/ardiacore/ios/ArdiaPageCover.mm:112-123`; `packages/ardiacore/ios/ArdiaWindowFullScreen.mm:500-516` | - | 3.8 |
| C-049 | Held page reveal (first paint) | Held tab page painted (not detached) | Tab view layer `opacity` 0 → saved opacity | 0.16 s | CABasicAnimation, cubic-bezier(0.23, 1, 0.32, 1) | - | `packages/ardiacore/ios/ArdiaWebView+FirstPaint.mm:74-93` | - | 5.1 |
| C-050 | Sidebar space pager settle | Swipe ends, or selection change | `position` (page index, published to JS) | Until \|x\| < 0.003 and \|v\| < 0.1 | Critically damped, $\omega=2\pi/0.25$: $x=(x_0+bt)e^{-\omega t}$, $b=v_0+\omega x_0$. During drag: tracking scale 0.5, rubber band $(1-1/(0.15x/255+1))\cdot255$, flick 5. Display link 60…max fps plus 1/60 s timer when link stalls 0.05 s | - | `packages/ardiacore/ios/Pager.swift:4-16,293-356` | `docs/dia-spec.md:585-588` (1.50 delta #7) | 3.11 |
| C-051 | PiP window fade-out (engine) | Hide (back to tab) / close button | PiP window frame layer `opacity` 1 → 0; shadow off | 70 ms hide, 100 ms close | CABasicAnimation, easeOut | Widget hidden by timer at fade end | `engine/chromium/src/chrome/browser/ardia/pip/ac_pip_mac.mm:69-89`; `engine/chromium/src/chrome/browser/ardia/pip/ac_pip_window.cc:34-35,309-338` | `docs/dia-spec.md:1018,1020,1032` | 3.10 |
| C-052 | PiP controls fade (engine, views) | Pointer enters / leaves PiP | Controls, scrims, title opacity | 200 ms | `gfx::Tween::EASE_IN_OUT` | - | `engine/chromium/src/chrome/browser/ardia/pip/ac_pip_controls.cc:278-292`; `engine/chromium/src/chrome/browser/ardia/pip/ac_pip_window.cc:170-200` | `docs/dia-spec.md:1008,1031` (spec says Ardia ~190 ms) | 3.10 |

Peek driver (C-014…C-023): `PeekStage` runs a screen display link at 60…max fps and a 1/60 s backup timer that ticks when the link has been silent for more than 0.05 s (`packages/shell/ios/PeekStage.swift:191-205`). Each frame sets layer values with actions off (`PeekStage.swift:238-246`). `peekSlow` test value multiplies responses (`PeekStage.swift:46,52`).

## Tests that pin curve values (evidence only)

| Test | What it pins |
|---|---|---|
| `packages/shell/fieldglow-tests/FieldGlowModelTests.swift:87-95` | 0.25 s run gives 31 samples; `.enter` mid sample = $1-0.5^3$; `.exit` mid sample = 0.25 ($t^2$ inverse) |
| `packages/shell/fieldglow-tests/FieldGlowTimingTests.swift:86-89,140-143` | Native curves equal the JS easing; show = `.enter`, hide = `.exit`; fade length = hold + hide |
| `packages/shell/fieldglow-tests/FieldGlowFixture.json` (motion block) | show 200 enter, progress 250 enter, finish 300 enter, hold 200, hide 250 exit |
| `packages/ardiacore/tests/pip-throw.test.cc:609-631` | Spring start state, velocity is the derivative, settles between 1.5 s and 3.5 s at 60 Hz, overshoot between 0 and 2 pt |
| `packages/shell/tests/WindowDragPacingTests.swift:9-17` | Pacer emits once per frame; link-stall fallback |

## Not animations (hits explained)

| File:line | Hit | Why it is not an animation row |
|---|---|---|---|
| `packages/shell/ios/WindowMove.swift:41-49` | `_setFrame:…animate:` | Swizzle that blocks AppKit's own frame adjustment for the window being dragged. Relevant: AppKit's animated placement is suppressed. |
| `packages/shell/ios/WindowManager.swift:418`, `packages/shell/ios/DragPreview.swift:236`, `packages/ardiacore/ios/ArdiaPiPZoom.mm:271`, `packages/ardiacore/ios/WebKitAutofillDropdown.swift:95` | `animationBehavior = .none` | Turns off AppKit's window order-in/out animation. |
| `packages/shell/ios/FullScreenRestore.swift:55` | `toggleFullScreen` | System full-screen animation; no Ardia curve. |
| `packages/shell/ios/FadeLabel.swift:13,151`; `FadeLabelModule.swift:140`; `DockSelection.swift:90`; `Surface.swift:56`; `SymbolView.swift:33,111`; `PopoverBubble.swift:162`; `ReportLayerCache.swift:46`; `FieldGlow.swift:63`; `GlassEffect.swift:23` | `setDisableActions` / `actions = NSNull` | Implicit animations turned off. |
| `packages/shaders/ios/ArcGradientLayer.swift:20-22`; `GrainLayer.swift:14`; `ThemeSwatchView.swift:13`; `WindowTreatmentView.swift:49` | layer `actions` = null | Implicit animations turned off. |
| `packages/shaders/ios/WindowBackdropView.swift:23-24,32,79-90` | `isPaused = true`, `needsDisplay` | Metal draws on demand only; no time uniform. |
| `packages/shaders/ios/MetalSurface.swift:9,112-130` | `MTKView`, `draw(in:)` | Base render loop for C-027/C-028/backdrop. |
| `packages/shaders/ios/EarlyBackdrop.swift:32`; `MeadowPoster.swift`; `ThemeGradient.swift` | opacity values | Static values set without animation. |
| `packages/ardiacore/ios/ArdiaEarlyWindow+Pages.mm:131,151,174-231`; `ArdiaEarlyWindowCovers.mm:45,92,104,132-156,211`; `ArdiaEarlyWindow.mm:287-288,364`; `ArdiaLaunchPicture.mm:228`; `ArdiaTrafficLights.mm:370,404,506`; `ArdiaRuntime.h:22`; `ArdiaChromeWindowHost.mm:216-257`; `ArdiaWebView.mm:278`; `ArdiaWebView+Painting.mm:91,255`; `ArdiaWebView+RevealInput.mm:193,222`; `ArdiaWindowFullScreen.mm:88,91` | actions off, `setFrame:display:`, deadlines | Instant placement, fail-safe deadlines, no interpolation. |
| `packages/ardiacore/ios/ArdiaWebView+{ContextMenu,FieldProviders,Focus,InputMonitors,Navigation,PageMessages,Peek,StandIn,Tab,Transfer}.mm`, `WebKitTab+Site.swift:86`, `ArdiaPiPController+Swipe.mm:183,271` | `CACurrentMediaTime` | Time stamps for gesture/focus/input windows. |
| `packages/ardiacore/ios/ArdiaPiPZoom.mm:135-154` | `NSTimer` 1/hz | Dev pinch driver (`devPinch`), test only. |
| `packages/ardiacore/ios/ArdiaPiPController+SelfTest.mm:212`; `ArdiaChromeWindowHost+DevFullScreenProbe.mm:32,102-108`; `ArdiaFreezeCapture.mm:436` | timers | Self-test, probe, freeze capture. |
| `packages/shell/ios/DevModule.swift:425-456` | `CADisplayLink` | Dev snapshot burst (captures frames). |
| `packages/shell/ios/DevFreeze.swift:8-9`; `WindowMoveProbe.swift`; `WindowResizeProbe.swift:72`; `ReportRecorder.swift`; `ReportModule.swift:89-91`; `ReportProcessUsage.swift`; `FocusHandoff.swift`; `KeyEquivalents.swift:16`; `ShellScripting.swift:30`; `ServerWindowDrag.swift:61,134`; `LeavingOverPiP.swift:22`; `QuitFlow.swift:58`; `PluginProcess.swift:224`; `PluginCore/PluginProcessTree.swift:117`; `SystemSignalsModule.swift:58`; `plugin-host/HostGlobals.swift:47` | timers / time stamps | Probes, watches, timeouts. |
| `packages/shell/ios/OpenSurfaces.swift:145`; `ArdiaNowPlaying.mm:29`; `WebKitNotifications+Scripts.swift:55,118`; `WebKitDownloads.swift:9,121` | `actions` / `isPaused` words | Menu/notification/download data, not layers. |
| `packages/sync/ios/Core/Wordlist.swift:143` | "spring" | Word list entry. |
| `engine/ardiacore/src/ardia/core/ac_video_upscaler_kit.mm:60` | `smoothstep` | Spatial blend in an upscaler kernel; no time. |
| `engine/chromium/src/chrome/browser/ardia/pip/ac_pip_linux.cc:251` | `EASE_OUT` | Linux-only file. |

## How I searched

| # | Command (from repo root) | Hits |
|---|---|---|---|
| 1 | `rg -c "$P3" packages/shell/ios packages/shell/docktile packages/shaders/ios packages/ardiacore/ios apps/browser/macos/Ardia-macOS engine/ardiacore/src engine/chromium/src -g '*.{swift,m,mm,h,metal,cc,cpp,c}'` with `P3='CABasicAnimation\|CASpringAnimation\|CAKeyframeAnimation\|CATransition\|CAAnimationGroup\|NSAnimationContext\|\.animator\b\|animator\]\|UIView\.animate\|CADisplayLink\|CVDisplayLink\|displayLink\|animate:\|animation\(forKey\|addAnimation\|\.add\([a-zA-Z]+, forKey\|preferredFramesPerSecond\|CACurrentMediaTime\|timingFunction\|CAMediaTimingFunction\|setDisableActions\|MTKView'` | 60 files, 258 lines |
| 2 | Same `P3` over `packages/shell/docktile packages/shell/plugin-host packages/import/ios packages/sync/ios apps/browser/macos/Ardia-macOS` | 0 files |
| 3 | `rg -c "$P2"` over shell/ios, docktile, plugin-host, shaders/ios, ardiacore/ios, import/ios, sync/ios, Ardia-macOS, engine/ardiacore/src, engine/chromium/src with `P2='stiffness\|damping\|[Ss]pring\b\|smoothstep\|scheduledTimer\|Timer\(timeInterval\|timerWithTimeInterval\|NSTimer\|makeTimerSource\|dispatch_source_create\|animationBehavior\|allowsImplicitAnimation\|CAAction\|action\(forKey\|actions =\|exp\(-\|pow\(1 - \|easeOut\|easeIn\|Tween\|SetDuration\|isPaused\|uTime\|\btime \+='` | 44 files, 116 lines |
| 4 | `rg -l "CABasicAnimation\|CAKeyframeAnimation\|NSAnimationContext\|\.animator\b\|animator\]\|CADisplayLink\|CVDisplayLink" engine` | 1 file (`ac_pip_mac.mm`) |
| 5 | `rg -n "customWindowsTo\|startCustomAnimation\|animationResizeTime\|setFrame:[^;]*animate:YES\|NSViewAnimation\|withAnimation\|\.animation\("` over native dirs | 0 new animations (only `animation(forKey:)` checks already covered) |
| 6 | `rg -n "import SwiftUI\|withAnimation\|\.transition\("` over native dirs | SwiftUI views (`DownloadPrompt`, `PopoverBubble`, `RecordingPill`, `TranslateError`, `TranslateModule`, `TranslationSessions`) use no animation modifiers |
| 7 | `grep -n "uTime" apps/site/src/scripts/meadow/shaders.ts` and timing constants in `scene.ts`, `clock.ts`, `brush.ts`, `particles.ts`, `smear.ts`, `mobs.ts` | 9 `uTime` lines; constants cited in C-030…C-040 |

### Per-file coverage

| File | Rows |
|---|---|---|
| `packages/shell/ios/ActivitySpinner.swift` | C-001 |
| `packages/shell/ios/WindowMotion.swift` | C-002, C-003 |
| `packages/shell/ios/WindowManager.swift` | C-002, C-003, C-004 (triggers); `:418` explained |
| `packages/shell/ios/DragPreview.swift` | C-005, C-006, C-007 |
| `packages/shell/ios/FolderIconView.swift`, `FolderArt.swift` | C-008 |
| `packages/shell/ios/FieldGlow.swift`, `FieldGlowCore/FieldGlowLayers.swift`, `FieldGlowCore/FieldGlowModel.swift` | C-009 … C-013 |
| `packages/shell/ios/PeekStage.swift`, `PeekMotion.swift`, `PeekSpring.swift` | C-014 … C-023 |
| `packages/shell/ios/WindowMovePacer.swift`, `UpdaterCore/WindowDragPacing.swift`, `WindowMove.swift` | C-024; swizzle explained |
| `packages/shell/ios/IntroMusic.swift` | C-025, C-026 |
| `packages/shaders/ios/PowerUpView.swift`, `MetalSurface.swift` | C-027, C-028 |
| `packages/shaders/ios/MeadowView.swift` | C-029 |
| `packages/shaders/ios/Meadow/index.html` (built from `apps/site/src/scripts/meadow/*`, `packages/shaders/src/meadowPage/*`) | C-029 … C-041 |
| `packages/ardiacore/ios/ArdiaPiPController.mm`, `ArdiaPiPController+Swipe.mm`, `ArdiaPiPThrowMath.h`, `ArdiaPictureInPictureInternal.h` | C-042 … C-045 |
| `packages/ardiacore/ios/ArdiaPiPZoom.mm`, `ArdiaPiPZoomMath.h` | C-046, C-047 |
| `packages/ardiacore/ios/ArdiaPageCover.mm`, `ArdiaWindowFullScreen.mm` | C-048 |
| `packages/ardiacore/ios/ArdiaWebView+FirstPaint.mm` | C-049 |
| `packages/ardiacore/ios/Pager.swift` | C-050 |
| `engine/chromium/src/chrome/browser/ardia/pip/ac_pip_mac.mm`, `ac_pip_window.cc` | C-051 |
| `engine/chromium/src/chrome/browser/ardia/pip/ac_pip_controls.cc` | C-052 |
| All other files with hits | "Not animations" table above |

Limits. [INFERENCE] `NSWindow` blur state changes in `WindowTreatmentView` (`NSVisualEffectView`) may cross-fade by AppKit; Ardia sets no curve. The meadow page rows cite the site sources that `meadow.mjs` bundles; I did not map each row to minified lines in `Meadow/index.html`.

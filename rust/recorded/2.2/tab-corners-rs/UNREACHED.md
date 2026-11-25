# Lines that `tab-corners-rs` does not reach

`rust-anything gate` reports all lines of `tabCorners.ts` and of its harness as run. The lines below are in files of other units.

| File (`apps/browser/src/lib/`) | Lines | Why |
| --- | --- | --- |
| `themeLook.ts` | 150-165, 173-181, 184, 190, 194-203, 210-212, 288-299, 304-315, 317-335, 338-339, 342-356 | Covered by unit `theme-look-rs`. The harness builds each look with `parseLookBlocks` and `resolveLook` only; the lines are the other exports (`withSidebarText`, `lookUse`, `lookSchemas`, `lookDefaults`) and checks of blocks this unit does not need. |
| `windowLayout.ts` | 25-38, 41-46, 57-66 | Covered by unit `window-layout-rs`. `themeLook.ts` imports only `DEFAULT_SIDEBAR_WIDTH` from it. |

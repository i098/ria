# Lines that `plugin-markdown-rs` does not reach

`rust-anything gate` reports all lines of `pluginMarkdown.ts` as run. The lines below are in a file of another unit.

| File (`apps/browser/src/lib/`) | Lines | Why |
| --- | --- | --- |
| `urlParts.ts` | 51, 53-60, 118-125 | Ported earlier in `ria-state` `shared/url_parts.rs`; no unit of this slice owns it. `pluginWebUrl.ts` (which `pluginMarkdown.ts` calls) sends only ASCII hosts to `urlParts`, so the punycode branch of `asciiHost` and `webUrl` do not run. |

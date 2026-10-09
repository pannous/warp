# Distribution (2026-10-05)

Our own answer to a "growth" cold email: README is repositioned (README.md), these are the channels and drafts.
README examples are verified by probes/readme/*.warp (run each with `warp <file>`).

## Positioning
One line: **a data format that is also a programming language, compiled straight to WebAssembly GC; try it in the
browser.** The hooks, strongest first:
1. Live playground, zero install (warp.pannous.com). Every post leads with the link.
2. Wasm GC structs, not linear memory: rare in practice, and wasm people care about it.
3. "No silent footguns": the compiler warns with the reading it took plus the explicit form. A design story PL
   people like to argue about (upto inclusive?, `[x]*n`, `a[-1]`).
4. Data = code, Lisp-like but with braces.

## Channels (in order)
| # | channel | why | rules |
|---|---|---|---|
| 1 | r/ProgrammingLanguages | home audience for new languages; self-posts welcome | lead with a design question, not "check out my lang" |
| 2 | awesome-wasm-langs (github.com/appcypher/awesome-wasm-langs) | permanent listing, steady trickle | PR adding a Warp entry |
| 3 | Show HN | playground = instant try | weekday 8–10 am US Eastern, title without hype, author stays to answer for hours |
| 4 | r/rust + This Week in Rust | wasm-encoder + wasmtime GC reading in Rust is the angle | TWiR: PR to this-week-in-rust repo, "Project/Tooling Updates" |
| 5 | Bytecode Alliance Zulip #wasm / r/WebAssembly | Wasm GC producers are few, people are curious | technical: name sections, reflection getters for JS reading GC structs |

Not worth it: Twitter/X blasts, Product Hunt, paid "promotion". Lobste.rs needs an invite (tags: plt, wasm).

Momentum after the spike: one post per real milestone (e.g. LSP quick fixes for the warnings, packages), a short
blog post per design decision from notes/welcoming.md, answer every issue fast. Write down "surprised me" reports.

## Drafts (nothing posted yet; each post needs the user's go)

### r/ProgrammingLanguages
Title: Warp: when the code is ambiguous, the compiler warns with the explicit form instead of guessing silently

Body: Warp is a data notation that is also a language (JSON-like, no quoted keys) and compiles to Wasm GC structs.
One design rule I'd like feedback on: if code has two plausible readings, the compiler takes a default but warns and
names the explicit form (`1 upto 4`: exclusive, fix `..<` or `to`); if a wrong guess would corrupt data
(`[x]*n`: repeat or multiply?) it refuses. The warning goes away after "got it", and the meaning never depends on
your answer. Playground: https://warp.pannous.com/ (the compiler runs as wasm in your browser), source:
https://github.com/pannous/warp. Which ambiguities would you add or remove?

### Show HN
Title: Show HN: Warp – a data format that is also a language, compiled to Wasm GC in the browser

Text: The notation is JSON-ish (`contact{name: James age: 33}`). The same syntax writes programs, which compile to
WebAssembly GC structs with full name sections. The Rust compiler itself is compiled to wasm, so the playground runs
everything locally. Rust rewrite of my older C++ project Wasp. Rough edges expected; I'd love "this surprised me"
reports.

### awesome-wasm-langs entry
`- [Warp](https://github.com/pannous/warp) - data format and language compiling to Wasm GC structs; the compiler runs in the browser ([playground](https://warp.pannous.com/)).`

### r/rust
Title: Reading Wasm GC structs back into Rust types: wasm_struct!/is! in a wasm-first language compiler
Focus: gc_traits.rs, `GcObject`, wasm-encoder usage, playground built as a plain C-ABI cdylib without wasm-bindgen.

## .warp files in Finder (macOS, card g_gHmE, 2026-10-09)
`warp register` builds ~/Applications/Warp Lang.app (an osacompile applet, src/file_type.rs): it exports the type
com.pannous.warp.source (.warp, .wasp; conforms to public.source-code and public.plain-text, so editors are offered under
Open With) and runs an opened file with `warp <file>` in a Terminal window. The Info.plist edit needs an ad hoc
re-sign (codesign --sign -), otherwise Launch Services keeps the file's dynamic type. Verified on this Mac:
mdls gives com.pannous.warp.source and NSWorkspace names Warp Lang.app as the default app; the double-click itself (a
Terminal window) was not run headless. Idea: the Homebrew formula's post_install could run `warp register`.


⏺ warp binary: 12.3 MiB (7.1 MiB .text)
  ┌────────┬─────────┬─────────────────────────────┐
  │ % Text │  Size   │            Crate            │
  ├────────┼─────────┼─────────────────────────────┤
  │ 35.2%  │ 2.5 MiB │ cranelift_codegen           │
  ├────────┼─────────┼─────────────────────────────┤
  │ 10.7%  │ 777 KiB │ wast                        │
  ├────────┼─────────┼─────────────────────────────┤
  │ 10.4%  │ 756 KiB │ wasmparser                  │
  ├────────┼─────────┼─────────────────────────────┤
  │ 10.4%  │ 755 KiB │ wasmtime_internal_cranelift │
  ├────────┼─────────┼─────────────────────────────┤
  │ 10.0%  │ 723 KiB │ std                         │
  ├────────┼─────────┼─────────────────────────────┤
  │ 8.3%   │ 599 KiB │ wasmtime                    │
  ├────────┼─────────┼─────────────────────────────┤
  │ 2.2%   │ 159 KiB │ wasmtime_environ            │
  ├────────┼─────────┼─────────────────────────────┤
  │ 2.2%   │ 158 KiB │ rustyline                   │
  ├────────┼─────────┼─────────────────────────────┤
  │ 1.8%   │ 128 KiB │ warp (your code)            │
  ├────────┼─────────┼─────────────────────────────┤
  │ 1.7%   │ 125 KiB │ wasm_encoder                │
  └────────┴─────────┴─────────────────────────────┘
  ~77% of the binary is wasmtime/cranelift JIT compiler infrastructure. Your code is only 128 KiB (1.8%). rustyline (new REPL dependency) adds 158 KiB.
## Night 2026-10-04 (raw finds)
- `warp lower <code>` prints the program after all lowering passes: the fastest way to see what the emitter gets.
- `tests/queue.sh -- filter` builds the default features: tests/ffi/* need `tests/queue.sh --all-features -- filter`
  (test.sh uses --all-features). A colored compile error also slips past `grep ^error`: strip ANSI first.
- The parser gives `c()` and `(c)` the same node; closures read a one-item round list of a function variable as a call.
- ~/.gitignore's `*_LOCAL*` matches case-insensitively on macOS: tests/control/test_branch_assigned_locals.rs was silently ignored (git add refused, mod.rs committed without it); renamed to …_variables.rs. Avoid "_locals" in tracked file names. (night 2026-10-04)
- Night 2026-10-04 bug hunt by probing idioms (`warp eval` one-liners, probes/night/*.warp): most finds were silent wrong values, not crashes — the raw-struct module shortcut returning an instance for `f(pt(…))`, dropped arguments (`log(100,10)`, `sin(0,5)`, `f(1,2)` of f(x)), str() of a call printing its source, `#m#1` as `(#m)#1`, `w#1.upper()` as `w#(1.upper())`. Probing small realistic programs (word frequency, bank account, matrix transpose) found more than reading code. Probe one-liners first, then fix the shared rule (arity check, position fallback, node_add concat), not the one case.
- A debug warpgate build serves its UI through rust-embed from warpgate-web/dist at run time: a CI job that only downloads the binary must also get dist (test_gzip "Manifest not found").

## Night 2026-10-04 (threads and more), raw finds
- wasmtime 49: GC and the threads proposal (shared memory + atomics) compile in one module (needs the cargo feature
  `threads` and Config::shared_memory), but an imported memory takes index 0 ahead of the module's own: every memory
  instruction of the emitter assumes 0, so shared arrays went through host words instead.
- Browser: a Worker created while its creator's thread is blocked (a running wasm program) never starts; tasks need a
  pool made while idle (emscripten's PTHREAD_POOL_SIZE for the same reason). Atomics.wait works in workers only.
- V8 names wasm frames `module.function (wasm://…)` in error stacks.
- Exact ratios and big integers travel inside an instance as i64 handles into its own memory: never pass them raw to
  another instance (null reference trap).
- A run-time function that calls another must be emitted after it (indices are given at emission): push before set,
  text_concat before node arithmetic.
- A Pratt-parser hook like try_parse_return is the cheap way to give a keyword unary-minus binding (`await`).
- Wiki sweep 2026-10-07: `WARP=<private copy> probes/wiki_blocks.py` (fenced blocks) and probes/wiki_examples.py (`a` → `b`)
  run from the main checkout (worktrees have no wiki/). Most → diffs are fragments needing the sentence's context.
  A parameter's kind is guessed from body usage before calls are seen (user_functions::with_usage_kinds): counting
  guessed List, so an uncalled text helper failed; joining it to a text literal now says Text.

- 2026-10-09 wasmtime 49 GC heap: its growth goes through the store's ResourceLimiter::memory_growing (store/gc.rs grow_gc_heap → memory.grow(delta, limiter)); a refusal is swallowed (GcHeapGrowthFailed) and the allocation traps "GC heap out of memory". The GC heap is capped at 4 GiB anyway (1 << 32), and gc-copying uses half of it: one allocation over ~2 GB fails even unlimited. warp caps every memory at WARP_MEMORY_CAP_MB (default 2048, crates/warp-runtime/src/engine.rs cap_memory), so 1 GiB of live GC objects per run.
- 2026-10-11 the readline console called `eval(input)` for every line, so it had no state between prompts. Replaying only successful declaration and mutation source makes ordinary variables persistent without replaying earlier print/query lines; effectful assignment right sides still run again until the runtime supports a persistent instance.
2026-10-12 wasm-split: `/usr/local/bin/wasm-split` 111 shadowed Homebrew 133; 111 rejects `--multi-split`, while 133 supports the route manifest interface.
- 2026-10-12 Codex cron lifecycle: thread 01a130c0-c3c4-7c00-bd8e-35942d3548e1 was persisted as interrupted while codex_worker.sh, codex_app_run.py and their app-server proxy stayed alive; its event stream ended at an in-progress command and had no turn/completed or .last.md. A real three-second timeout probe proved turn/interrupt produces the persisted interrupted terminal state and can be reaped without a proxy leak. Explicit app-server projectId uses an internal project id and did not place the thread in the desktop sidebar project; leave it unset and let the desktop group the main cwd (card codex-app). During a real active sleep probe, app-server emitted active but the desktop list still reported notLoaded (card codex-desktop); the wrapper's start/end notices are therefore the accurate lifecycle signal.
- 2026-10-16 git-push: 'fatal: failed to store: -61' after a successful push came from Homebrew's system gitconfig
  helper osxkeychain (errSecWrPerm: the keychain refuses writes from headless sessions) running after gh's helper.
  ~/.config/git/config now resets the list (`helper =`) before `helper = !gh auth git-credential` for github.com and
  gist.github.com, the form `gh auth setup-git` writes; git credential then asks gh only.

- 2026-10-17 (warp-parser): object_entries must not take a square list of key: value items as one object: a one-entry map in a list is held as a bare key ([{first: 2}] is [first: 2]), so [a: 1, b: 2] is also a list of one-entry maps; generalising it broke 10 tests (list fields, possessives). Local readings (a held lambda's holder) are fine.

2026-10-17 interviewer: src/extensions.rs texts! Separator::Colon edit (unknown origin, user "fix it") reverted to Comma; backup data/patches/texts-macro-colon-2026-10-17.patch

- 2026-10-17 parenthesized-first: parsing and lowering preserved `((try e else 0), 2)` as a round comma tuple, but inference treated it as the last element's type and list emission treated its control-flow element as a statement sequence. Round comma tuples must be classified and emitted as value collections at both boundaries.

- 2026-10-19 warp-cache: with WARP_SELF_HOSTED=1, right after lib/ or the binary changed, two runs printed nothing at all (exit 0) while a third warmed the stage cache; warm and fresh-cache reruns were right. Not reproduced: maybe a run reading a kept program another run was still writing (scratch/refusals/cache shared by parallel runs).
- 2026-10-19 warp-cache: tests/queue.sh killed a run of 7 WARP_SELF_HOSTED=1 test files at the 4 GB memory cap (each test process compiles the stage programs on a cold cache); `--test-threads=2` stayed under it.
- 2026-10-19 warp-cache: Rust's list semantics: `+=` and push change the list every alias holds; `xs = xs + [x]` makes a new list (emitter.warp appends in place only while xs is unshared).

- card time-zones-warp (sprint, blind): lib/time.warp has the zone rules in warp (Zone class, Dst enum, time_zones table, zone_offset, zone_instant over whole seconds); src/wasm_emitter/time/time_zones.rs stays until the emitter calls these words; time_in_zone waits for card time-primitives (form/position of a time value)

- exact-pow-warp (2026-10-19): runtime.warp exact_pow/int_pow; emitter.warp int ^ → exact_pow (node result). i64 only: a power beyond the int range fails in int_overflow where Rust gives a big int (probes/ratio_text.warp 2^70, samples with 2^100 may now enter the core and differ instead of being refused). A ratio or float base (node kind) is still refused (floats have no ^).
- 2026-10-19 warp-parsing: a new probe or sample shifts the twins' 1-in-4 day share (by position in the sorted source list), so a twin failure that's already on main can show up only on your branch. `TWIN_ONLY=<path> tests/queue.sh -- <test>` on main settles it (probes/unknown_tag.warp, card emitter-prints).
- 2026-10-19 warp-parsing: the scans before parsing (declaration_scan.rs code_only) skipped no `#` comments, so an apostrophe in one hid code up to the next apostrophe. That also masked a type-scan bug: `= type` at a line end declared the next line's first word a type. Compare `warp parse` with two binaries and bisect by dropping lines (card comment-apostrophe).

- 2026-10-18 like-tea: a prelude function gets the same natural call labels as user functions by declaring the parameter after its preposition (`replace(t, old, with new)`); positional calls keep working and `.replace "tea" with: "coffee"` uses the shared preposition-call lowering.
- 2026-10-18 like-tea CI: prelude reflection reads signatures as text, so labelled parameters need the same label stripping as AST parameters; keep `replace.signature` labelled while `replace.params` returns the variable name `new`.
- 2026-10-19 traps-wasm-trap: `text_argument` propagated Error and converted Codepoint but never required Text, so `byte_at(3, 0)` reached `emit_text_bytes` and trapped on the payload cast. Validate the shared argument after conversion, before any text storage access.

- 2026-10-18 julia-floor-divide: Unicode math operator aliases are declared in lib/math.warp and read into the Rust operator table, while lib/compiler/parser.warp keeps a direct SYMBOL_OPERATORS twin; compound glyph aliases such as ÷= must be declared in both places and ordered before their shorter glyph.

- emitter.warp: a runtime word compiled as an arity variant is named `word·arity` (returned_error·0); error_throw uses the word before the `·`. A one-statement curly block assigned as a value (`x = {f(1)}`) is data natively, so lowerings must splice a block's statement, not hold the block (tries.warp, 2026-10-20).
2026-10-20 build-failing-space: cloud-test.sh already deleted target/ and retried ENOSPC once, but a second ENOSPC flowed through the ordinary summary as red and collect() returned that full VM to the pool. Persistent ENOSPC needs its own environmental result and job-to-session retirement at collection time, where the pool identity is known.

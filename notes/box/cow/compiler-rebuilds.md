# compiler.wasm / compiler-self.wasm: what is baked in, what really needs a rebuild, and a plan

Investigation on 2026-10-23 (Europe/Berlin). Base: `4e3808b76` (the tree `/workspace/cow/trees/rebuild-policy`, read-only, nothing committed).
Cross-checked against `origin/main` `7b437156b` (09:51 CEST), which already fixed part of this in `ab93badf7` (07:49 CEST).
Note: `origin/warp-host-wasm-load` has moved to `5bc01c62f`, and `4e3808b76` is no longer its ancestor. The findings are the same on both.
Measured means I ran it or read it. Inferred means I worked it out from the code.

## TL;DR

* **The root cause of last night's refusals was not a baked host-word list.** The words `std_io`/`std_pure` verbs (`run_wasm`,
  `wasm.load`, …) are **not** known to the compiler at all: lib/os.warp / lib/wasm.warp declare them and the host
  dispatches them at run time. The compiler instead **baked the absolute path of the lib/ it was built in** (`module_directory`)
  and read `use os`, `use wasm`, sugar rules, aliases and entities **from that build folder**, not from WARP_LIB.
  A compiler-self built in a tree whose lib/os.warp had only `run_wasm(bytes)` and no lib/wasm.warp gave exactly
  "a call of run_wasm with 2 arguments" and refused `use wasm`, wherever it was copied to.
  * **Measured on the box** with the shipped tarball's compiler-self.wasm, which has `/workspace/cow/trees/warp-host/lib/compiler` baked in. With
    `WARP_LIB=/tmp/rbp-test/lib`, a word I added to that lib/os.warp was refused ("outside the core: the unknown name
    probe_word"). After I deleted `cwd` from that lib/os.warp, `use os; print(cwd())` still compiled and ran. The words come
    from the baked folder, not from WARP_LIB.
  * The baked path can be read inside the module: `compiler-self.wasm` in trees/pure-error-positions/data contains
    `/workspace/cow/scratch/pep/work/lib/compiler` (it was built there and copied). That scratch lib/ has **no wasm.warp**.
* main's `ab93badf7` adds `parser.warp run_library() = env("WARP_LIB") ?? folder_of(module_directory)` and uses it for
  std_module_path, entities, aliases and diagnostics. **That only helps when WARP_LIB is set in the environment.** Neither warp-host
  (`crates/warp-runtime/src/warp_host/main.rs` compile) nor warp-rust's pure lane sets it. When lib/ is found by walking up from the
  source, or as an installed lib/, the compiler still falls back to the baked build folder. That also covers **released tarballs**: the tarball
  bakes `/workspace/cow/trees/warp-host/…` (inferred: `use os` breaks on any machine without that folder).
* What is really baked into the module, besides compiler logic: **the text of lib/prelude, math, list and regex** (`module_source(...)` is
  inlined at build time), the **host import table** (host_words.warp), the **std module name list** and the **sugar module lists**.
* Rebuild volume (measured, lower bound): on the **box**, 52 distinct compiler.wasm and 32 distinct compiler-self.wasm builds on
  2026-10-23 between 00:00 and 09:56 CEST, and 19+14 on 2026-10-22. On the **Mac**, 21+13 still on disk for the 23rd and 10+5 for the 22nd. One
  self rebuild takes 5:42 on the Mac (pure: 1:48) and 661 s on the box. The box kernel log shows 3 OOM "Killed process" entries.

---

## 1. What is baked in at build time, and what is read at run time

Line numbers are for `4e3808b76`, with main's in brackets where they differ.

### Baked in (a change needs a rebuild today)

| What | Where | Effect |
|---|---|---|
| **`module_directory` = absolute build path of lib/compiler** | substituted at compile time: `lib/compiler/uses.warp:28` `with_module_directory` (pure lane), `src/modules/loader.rs:143` (Rust lane, compiler.wasm) | every runtime lookup below that uses it reads the **build folder** |
| std module lookup for `use os`, `use wasm`, … (definitions, arities) | `uses.warp:141` `std_module_path = folder_of(module_directory)/name.warp` → used by `prelude_words.warp:204,243` (with_used_modules, type modules), `rewrites.warp:106,136` (module and sugar rules) | **the cause of "run_wasm with 2 arguments" / `use wasm`**. main: `run_library()` (WARP_LIB, else baked) |
| parser's `use` scan for operators | `parser.warp:1597` (main 1612) | same, main uses run_library() |
| entities.tsv, aliases.warp | `parser.warp:35`, `parser.warp:227` (main 38, 230) | same; fails loudly when the build folder is gone ("the alias table … is missing", fixed in ab93badf7 only under WARP_LIB) |
| diagnostics messages default | `diagnostics.warp:22` (overridden by `compiler.warp:15 messages_from(library_folder)`) | OK at run time |
| **text of lib/prelude.warp** (typed builtin lines, bang words) | `builtin_signatures.warp:15` `prelude_text = module_source("prelude")`, `parser.warp:125` (main 128) | inlined as a text literal: measured, 263/263 long prelude lines are inside compiler-self.wasm |
| **text of lib/math.warp** (libm foreign lines, operator aliases, circumfixes) | `builtin_signatures.warp:159` (main 159, 215), `library_calls.warp:15`, `parser.warp:177,200,1575` (main 180,203,1590) | inlined (82/82 lines found) |
| **text of lib/list.warp, lib/regex.warp** (glyph operators) | `parser.warp:1575` (main 1590) | inlined (56/56, 7/7) |
| …in **compiler.wasm** these texts come from the **warp-rust binary** (`include_str!` in `src/modules/std_modules.rs:7-…`, inlined by `src/lowering/words/library_words.rs:964`), not from lib/ | | compiler.wasm's typed lines are as old as the binary that built it |
| **host import table** (name → wasm param/result kinds; arity check) | `lib/compiler/host_words.warp:4-85`; used in `emitter/host.warp:5,17-19` (refusal "a call of X with N arguments"), `builtin_signatures.warp:189,204`, `compiler.warp:47` | a **new raw host import** (like `gpu_compute`, not a std_io verb) needs a rebuild. 2 commits on main since Oct 21 |
| std module names (which modules' **rewrite rules** apply) | `rewrites.warp:72` (main 89), 36 names, **no `wasm`** | a new lib module's rules are ignored until a rebuild |
| sugar module lists and their order | `rewrites.warp:68-70` (main 84-87) | a new lib/sugar/*.warp file needs a rebuild; edits to existing sugar files are read at run time (from the baked path) |
| type modules auto-used | `prelude_words.warp:181` `["list","map","text","time"]` | language semantics, fine to keep baked |
| WASI-served std words | `emitter.warp:1030-1036` (main 1051) `std_word`: `std_<module>_<verb>` if the runtime defines it | runtime/ is read at run time, so this is not baked |
| time host words | `emitter/times.warp:21` | compiler logic |
| the compiler's own `use os`, `use file` and prelude | `compiler.warp:9-10` | the compiler's own I/O. Changes to these words only matter if their semantics change |

### Read at run time (no rebuild needed, provided the right lib/ is used)

* `lib/compiler/runtime.warp` and `runtime/**`: `compiler.warp:26` (`library_folder`), lowered and kept (`kept/keep`). The cache key covers
  the compiler bytes and the lib fingerprint: warp-host `cache.rs:23`, warp-rust `std_adapters.rs:271` + `self_hosted.rs:665`. **Runtime word changes need no rebuild.**
* `lib/prelude.warp` definitions a program reaches: `compiler.warp:21 prelude_source`. Typed lines are the exception (baked, above).
* `errors.data / warnings.data / hints.data`: `compiler.warp:15 messages_from(library_folder)`.
* C imports, `lib/native/*.warp`: `c_imports.warp:76-184` via `library_folder`.
* **std_io / std_pure verbs and their arities**: declared in lib/*.warp (`os.warp:19-22`, `wasm.warp:7-13`). The compiler only knows
  `std_io: [["node","node","node"],["node"]]` (`host_words.warp:19`). The host dispatches the verb (`warp_host/words.rs:121…`).
  **There is no baked list of these words.** A new verb or overload needs no compiler rebuild once the std module comes from the run's lib/.

## 2. What really needs a rebuild, and what forces one today but shouldn't

**Needs a rebuild (legitimate):**
* edits to `lib/compiler/**/*.warp`, except `runtime/**` and `*.data` (630 main commits touched lib/compiler since Oct 21, so most rebuilds are this case)
* a new **raw host import** in host_words.warp (rare: 2 commits since Oct 21). It also needs a host build.
* a new warp-rust build (WARP_SOURCE). The stages that build compiler-self are Rust-compiled, and so is compiler.wasm.
* edits to lib/sugar, lib/aliases.warp or lib/prelude.warp **that change how the compiler's own source parses or lowers**. These are inputs to the build,
  but not baked into its behaviour on other programs.

**Forces a rebuild today, but shouldn't:**
1. **Any lib/*.warp std module change (new host verbs, new overloads, new modules like wasm.warp).** The compiler reads them from its *build* folder.
   Even on main this happens when WARP_LIB is unset. Copying a compiler-self between trees silently pins it to the builder's lib/, or to nothing if that tree is gone.
2. **Typed-line changes in prelude.warp and math.warp, operator changes in list.warp and regex.warp:** inlined by `module_source`.
3. **A new std module's rewrite rules:** the `std_module_names` list.
4. **A new sugar module file:** the sugar lists.
5. **Over-broad staleness keys:** `standalone.rs:208 judged_build` = lib *path* + fingerprint of **all of lib/** + WARP_SOURCE, and
   `self_hosted.rs:673 stage_key` also hashes all of lib/. Any lib edit (a doc comment in sound.warp) marks the pair stale and invalidates the stage cache,
   so the next self build recompiles all stages, the slow and memory-hungry part. The lib *path* in the judged parts means a correct pair copied
   into another tree is always reported as "built from another lib/".
6. `scripts/self-check.sh:26-31` always runs `warp rebuild pure` and `warp rebuild self`, even for a branch that never touched compiler inputs.

## 3. Plan per unnecessary trigger

Ordered by value. "Owner conflict" is the risk against the Claude workers editing lib/compiler.

**P1. The run's lib/ wins over the baked folder: hosts export WARP_LIB to the compiler run.** *(fixes trigger 1 completely, together with main's ab93badf7)*
* `crates/warp-runtime/src/warp_host/main.rs` `compile()` (≈ line 191 at base, 275 on main): before `run_wasm(...)`, put `WARP_LIB=<lib>` into the
  environment the compiler module sees. Simplest is `std::env::set_var("WARP_LIB", &lib)` (the process uses one lib/). Otherwise the os.env adapter answers
  `WARP_LIB` from `keeper.library`. ~3-6 lines.
* `src/compiler/standalone.rs` (`compiled_code` / `warm` / `pure` paths) and `packaging/bin/warp` `compile_module` (`export WARP_LIB=$lib`, 1 line):
  the same for warp-rust's `warp pure/self`. ~3 lines Rust, as an urgent pure-pipeline fix since Rust is frozen.
* Optional belt and braces in lib/compiler: `parser.warp run_library()` could prefer a `global library_root` that compiler.warp sets from `args#4`
  (~6 lines in parser.warp + 1 in compiler.warp). Skip it if the host fix lands.
* Size: ~10 lines, no lib/compiler edits. **Owner conflict: none.** It is warp-host's crate, the branch this task is based on.
* Test: the `/tmp/rbp-test` experiment above (add a word to the run's lib/os.warp, compile `use os` with it). It should pass without WARP_LIB.

**P2. `module_source(name)` is read from the run's lib/, with the baked copy kept as a fallback.** *(trigger 2)*
* Add to `lib/compiler/parser.warp` (next to `run_library`):
  `library_source(name: text): text := (try read(run_library() + "/" + name + ".warp") else "") or module_source(name)`.
  The baked copy stays for the browser playground, which has no lib/ files.
* Replace the 9 `module_source(` call sites (main): `builtin_signatures.warp:15,159,215`, `library_calls.warp` (math_text), `parser.warp:128,180,203,1590`.
  The top-level ones run at module init. That is fine once P1 has put WARP_LIB in the environment.
* Size: ~12 lines across 3 files. **Owner conflict: medium.** parser.warp (+86 lines on main today) and builtin_signatures.warp (+28) are being edited.
  Make it one small mechanical commit, rebased just before merging. The integrator should take it between batches.

**P3. std module names come from the lib/ folder.** *(trigger 3)*
* `rewrites.warp:89` (main): `std_module_names = std_modules_in(run_library()) ?? [baked list]`, where `std_modules_in` (put it in `uses.warp`, which has
  low churn) lists `lib/*.warp` stems with `list_files`. Also add `wasm` to the fallback list.
* Size: ~6 lines. **Owner conflict: high on rewrites.warp** (+446 lines on main today, sugar-engine work). Change only that one line, and land it after the sugar engine cards.

**P4. Sugar modules listed by a data file in lib/sugar.** *(trigger 4, optional)*
* `lib/sugar/order.data` lists the phases (first / main / iteration / late) and the files. `rewrites.warp:84-87` reads it at run time, with the baked lists as fallback.
* Size: ~15 lines. **Owner conflict: high** (the same hot lines). Only do it once the sugar lists stop changing daily.

**P5. Host import table read at run time.** *(raw host imports, optional)*
* `host_words.warp`: `host_words = runtime_host_words() ?? { …baked… }`. It parses `run_library()/compiler/host_words.warp` as data
  (parser.warp `written_tree`, as aliases are read). Validation stays in `emitter/host.warp:15-19`.
* Size: ~10 lines. **Owner conflict: low** (host_words.warp: 2 commits in 3 days). Value: low, because a new raw import needs a host rebuild anyway.
  **Recommendation: skip.** New host features should go through `std_io` verbs (P1 makes those rebuild-free), not new raw imports.

**P6. Narrow staleness keys (Rust, after P1/P2).** *(trigger 5)*
* `src/compiler/standalone.rs:208 judged_build`: drop the lib *path* (it is meaningless after P1) and hash only the compiler inputs (§4 list), not all of lib/.
* `src/compiler/self_hosted.rs:673 stage_key`: hash the compiler inputs instead of `hash_library` (all of lib/). Then the stage cache survives std-module and test edits.
  This is the biggest saving on memory and time for `warp rebuild self` (inferred).
* Size: ~20 lines of Rust. **Owner conflict: none with lib/compiler**, but it is Rust and needs the user's or Supervisor's OK under the "Rust frozen" rule.

## 4. Rebuild policy

**Inputs hash.** Draft script: `/workspace/cow/scratch/rebuild-policy/compiler-inputs.sh <lib> [<rust-fingerprint>] [-v]`. It is not in the repo yet;
proposed as `scripts/compiler-inputs.sh`. It takes sha256 over the relative path and contents of:
`lib/compiler/**/{*.warp,*.data,*.tsv}`, `lib/sugar/**`, `lib/{aliases,prelude,os,file,list,map,text,time,math,regex}.warp`, plus the
Rust fingerprint (WARP_SOURCE). After P1/P2 the std modules other than the ones the compiler itself uses (`os`, `file`, prelude and type modules)
and `runtime/**`, `*.data` can be dropped from the list. Measured: 0.1 s, and the same hash for two trees at the same commit in different paths.
* WARP_SOURCE of the current binary: read the 3rd line of the `warp.build` custom section of the delivered `compiler.wasm` beside the binary,
  or add `warp version --source` (1 line of Rust).

**Stored beside the wasm:** `data/compiler.wasm.inputs` and `data/compiler-self.wasm.inputs` (one line each: hash, the warp-rust WARP_SOURCE, commit, date).
* `scripts/rebuild.sh pure|self` (new, ~30 lines). If `data/X.wasm` exists and `X.wasm.inputs` matches the current hash, print "up to date" and exit 0.
  Otherwise copy from the shared store if that hash is there, else `warp rebuild X`, write `.inputs`, and publish to the shared store.
  `--force` rebuilds anyway.
* `scripts/self-check.sh:28` calls `scripts/rebuild.sh $1` instead of `$warp rebuild $1`. `scripts/slot.sh` is unchanged. The cache check runs **before**
  taking a slot, so a cache hit never queues for a heavy slot (the wrapper calls slot.sh only on a miss).
* Later, the same check can go into `warp rebuild` itself (standalone.rs `rebuild`: skip when the stored inputs hash equals the current one), as part of P6.

**Shared known-good pairs**, content-addressed by inputs hash and reflink-cheap:
* **Box:** `/workspace/cow/compilers/<hash>/{compiler.wasm,compiler-self.wasm,inputs.txt}` (on the cow mount, so `cp --reflink`), with
  `/workspace/cow/compilers/main -> <hash of origin/main's latest good pair>`. `boxtree` can reflink `main` into the new tree's `data/` when the tree's hash matches.
* **Mac:** `/Volumes/Build/compilers/<hash>/…` (off the Data volume, like the slots), with `main ->` symlink. `~/dev/bin/main-builds/<commit>/` already keeps
  `compiler.wasm` + `warp` per main commit (10 dirs) but **not compiler-self.wasm and no inputs hash**. The main-binary refresh job should also drop
  both wasm files plus `inputs.txt` into the hash store.
* Write-once directories (`<hash>.new-$$` then rename), the same pattern as `standalone.rs write_new`. Never overwrite a published pair. Prune entries older than 3 days that are not `main`.
* **Rule for agents (AGENTS.md wording, for Supervisor):** "Never `warp rebuild` directly. `scripts/rebuild.sh` copies the shared pair for your inputs hash.
  A branch that doesn't touch lib/compiler, lib/sugar, aliases or prelude never builds." This needs P1 first. Otherwise a copied pair still reads the builder's lib/.

## 5. How often rebuilds happened (measured, lower bounds: an overwritten file counts once)

* **Box** (`/workspace`, `/tmp`, distinct sha1 of compiler*.wasm by mtime): 2026-10-22: 19 compiler.wasm + 14 compiler-self.wasm.
  2026-10-23 00:00 to 09:56 CEST: **52 + 32**. Distinct builds per hour on the 23rd: 00h 2, 01h 7, 02h 8, 03h 12, 04h 10, 05h 6, 07h 7, 08h 15, 09h 17.
* **Mac** (`/Volumes/Build`, `/Volumes/Worktrees`, `~/dev/angles`, `~/dev/bin`, `/tmp`, mtime < 8 days): 2026-10-22: 10 + 5. 2026-10-23 so far: 21 + 13
  (58 files). Main binary builds in `~/dev/bin/main-builds`: 10 since 2026-10-22 21:58 CEST.
* Slot locks: `/Volumes/Build/self-check/slot.{0,1,2}.lock` hold only a pid and are deleted on exit, so **there is no history**. At 09:51 all 3 were busy
  (gate self-check of grok-cm, a sweep, an r2l run). Suggestion: slot.sh appends `start/end/slot/command` to `/Volumes/Build/self-check/slots.log` (2 lines).
* Duration: Mac `warp rebuild pure` 1:48 and `rebuild self` 5:42 (`/Volumes/Build/warp-host-demo-rebuild.log`, 09:37 CEST). Box self rebuild 661 s
  (`trees/integrate-self.log`). Box: 15 GB RAM, 8 cores, 3 OOM kills in dmesg.
* How many could have been skipped: of the 21 box trees with a surviving compiler-self.wasm since the 22nd, **9 had no lib/compiler changes of their
  own** (`*-base`, `rmain`, `sdef-main`, `warp-host`, …): copies of main at some commit, which a shared pair keyed by inputs hash serves. The other 12
  carried branch edits to lib/compiler (legitimate). 6 different WARP_SOURCE values appear among those 21 builds, so Rust binaries also differed (inferred:
  different main binaries over the day). Rough box cost today: ~84 builds × 2-11 min ≈ 6-10 CPU-hours (inferred).

## Suggested order

1. P1 (warp-host + warp-rust export WARP_LIB): no lib/compiler conflict. It fixes the "built from older lib/" class of failures and makes copied pairs universal.
2. scripts/compiler-inputs.sh + scripts/rebuild.sh + shared stores (box and Mac), and self-check.sh calling rebuild.sh.
3. P2 (`library_source`): one mechanical commit, timed with the integrator.
4. P6 (narrow keys in Rust), with approval.
5. P3, then P4 once the sugar lists settle. P5 skipped.

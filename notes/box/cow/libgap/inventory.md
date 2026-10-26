# Warp Lib Gap inventory (grok-warp-lib-gap, 2026-10-23 Europe/Berlin)

## Sources
- Baseline (measured by Supervisor on the Mac, main 6b7abc92de): `scripts/lib_bench.sh` → 6b7abc92de.txt (copy here;
  Mac: ~/dev/angles/warp/data/lib_bench/6b7abc92de.txt). parsed 119/119, pure-compiled 82/119, Rust 96/119
  (core/runtime/ counted apart).
- Pure suite at 6b7abc92de: suite-6b7abc92de.out (copy here; Mac: /Volumes/Build/measure-pure/suite-6b7abc92de.out),
  5623 of 10720 failed (last line of the file) → 5097 pass.
- Box repro per file (origin/main 32af43655, branch libgap-lines-clash): `pure1.sh <tree> <file>`, the exact
  pure_verdict of scripts/lib_bench.sh:
  `cd <tree>/<dir>; WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_CRASH_CARDS=0 WARP_LIB=<tree>/lib WARP_SELF_HOSTED=pure-only
   WARP_EMIT_REFUSALS=<tmp> timeout 120 warp <file>`
  warp = /workspace/cow/libgap/warp-main-rust, a release build whose src/ equals origin/main's pure-lane code
  (built from the wasm-commands tree; its only Rust diff to main is the `warp pure <command>`/repl CLI).
  First run after a lib edit compiles the stage programs (2-4 min at box load 30+), later runs 1-8 s.
- Sweep after the fix: sweep.sh → sweep-5d9fa2685.txt (pure only, one file at a time).

## The 14 files Rust compiles but pure doesn't (6b7abc92de), re-measured on the box at 32af43655
| file | pure verdict | root cause | where the fix belongs | owner |
|---|---|---|---|---|
| lib/shell.warp | passes.warp stage: `lines(any) is declared in both file.warp and text.warp` (the bench showed it as a wasm `unreachable` trap) | two std modules declare `lines` with one signature; uses.warp claim_signatures (twin of loader.rs) refuses | lib/text.warp | **FIXED** card pure-lines-clash, branch libgap-lines-clash |
| lib/math.warp | the unknown name x | a bodiless foreign declaration `sqrt(x: number): number from warp` is not read as a declaration: its parameter leaks as a name | foreign forms / definition heads (core/foreign_forms.warp, definitions.warp) | grok-warp-code-3 (sugar-foreign-forms), grok-warp-ffi (pure-c-imports), grok-warp-code (sugar-definitions) |
| lib/channels.warp | a call of sleep with 2 arguments | `5 ms` (and `5ms`): unit literals unknown in the pure lane ("the unknown name ms") | units/time literal fold (core, parser or time_arithmetic) | Claude (core) |
| lib/draw.warp | the unknown name view_width | host global view_width/view_height not known to emitter.warp | core/emitter (host words) | emitter (Claude) |
| lib/wagi.warp | the unknown name wasi_environment | prelude `wasi_environment(): text from warp` not implemented by emitter.warp | emitter / WASI | Claude |
| lib/extra/netbase.warp | the call fetch(…) | prelude `fetch(url, options...): text from emitter` not expanded by emitter.warp | emitter | Claude |
| lib/markup.warp | a test of the type pair | `value is pair` not in emitter runtime_words core_type_words (core/emitter/runtime_words.warp:159) | emitter | Claude |
| lib/router.warp | the call answer(…) | `route.answer(request)`: calling a function value held in a record field | emitter closures / lambdas.warp | Claude (emitter / folds) |
| lib/units.warp | the call shown(…) | method of a class (`q.shown()`) | classes | Warp Classes (pure-class-methods) — skipped |
| lib/case_table.warp, lib/sound.warp, lib/uncertain.warp | an instance of X, a class with methods | classes with methods | classes | Warp Classes — skipped |
| core/compiler.warp, core/passes.warp | timed out after 120 s | pure lane too slow on the compiler itself | core | Claude — not touched |
| core/emitter.warp | `module is a main-level variable: declare it global module` | emitter.warp assigns the main-level `module` inside a function | core/emitter.warp | emitter (Claude) |

## Files neither lane compiles
- 10× `unknown type Body of parameter body` + control_flow/encoding/literals/module_layout: all core/emitter/*.warp,
  emitter.warp's parts (emitter.warp:19-30 `use "emitter/…"`); Body/Function are declared in emitter.warp, so a part
  compiled alone cannot know them. Not a pure-lane gap: a bench artefact. Options (owned, not done): count
  core/emitter/ apart like core/runtime/ in scripts/lib_bench.sh, or move the shared classes into a
  part each part uses.
- foreign_forms.warp (`unknown type Parser`, a parser.warp part), rewrites.warp (word_lines from library_calls.warp),
  runtime.warp, time_arithmetic.warp, aliases.warp, extrema.warp, fast_math.warp, prelude.warp, wasmtime.warp: missing
  uses / library-word clashes; core ones owned.

## Overlap with the pure suite errors (suite-6b7abc92de.out, counts of ✗ lines matching)
- `the unknown name x` 106 — same family as math.warp: definition heads with a return/type clause (`int g(x) = …`,
  `def f(x) as int or error := …`, `f(x: T): T from warp`) leave their parameters as unknown names.
- unit names (`ms`, `km`, `s`, …) unknown: 37 (ms 13, km 16) — same as channels.warp.
- `view_width` 14 — same as draw.warp.
- `class with methods` 221 — Warp Classes.
- `a test of the type …` 40 (pair 1).
- fetch / wasi_environment / `is declared in both`: 0 in the suite.
- The non-refusal errors (cast failure 68, not a list 25, no field json 13) do not show up in the lib files.

## 2026-10-23 ~02:00 Europe/Berlin: x-family investigation (no edits yet: no SendToAgent to announce files)
Pure lane probes (box, warm cache, WARP_SELF_HOSTED=pure-only, lib at 35887ba4c):
- works: `g(x: int): int := x+1`
- unknown name x: `int g(x) = x+1`, `int g(x) := x+1`, `g(x): int = x+1`, `f(x) as int := x`, `def f(x) as int := x`,
  `def f(x) as int or error := x`, `sqrt(x: number): number from warp`; `mysq(x: float): float from warp` → "the call mysq(…)".
parser.warp shapes (probe /tmp/lg/v/shapes.warp):
- `g(x): int = e` → Assign(Colon(head, int), e)  — definitions.warp only turns Assign into Define when is_head (names only)
- `g(x: int) = e` → Assign(head with Colon(x, int), e) — is_head refuses typed parameters
- `f(x) as int := e` → Define(As(head, int), e) — result_held only matches Colon
- `def f(x) as int or error := e` → [def, Define(Or(As(head, int), error), e)]
- `int g(x) = e` / `:= e` → List Space [int, Assign|Define(head, e)] — C-style result type in front, not handled
- `sqrt(x: number): number from warp` → [Colon(Sqrt(ø, (x: number)), number), from, warp] — sqrt parsed as the √
  operator; is_runtime_declaration drops only `from runtime`, not `from warp` / `from emitter`
Fix site: core/definitions.warp (definition, is_head, result_held, is_runtime_declaration); keyword forms
(`def …`) now move to lib/sugar/functions.warp on branch sugar-definitions (Warp Code), which also edits
definitions.warp → collision risk. foreign_forms.warp (Warp Code 3) handles `from 'c'` lines, not `from warp`.
Suite (6b7abc92de): of 106 "unknown name x" failures ~30 are definition heads; the rest are other forms
(`x : 100 int`, `x be 3`, `real x`, `x : a+b` quotes, `{x*x}(x=5)`, `! Pure` effect annotations, `where`, `fast f(x)`).

## 2026-10-23 ~02:30: card pure-def-heads (branch libgap-def-heads, fix 408d9b96f, tip a6ec951d2)
core/definitions.warp: keyword-less typed heads (`f(x): T = e`, `f(x: T) = e`, `f(x) as T := e`/`= e`,
`T f(x) = e`/`:= e`) become `f(x): T := e` (declared_result_form, result_held unchanged); `… from warp|emitter` lines
go like `from runtime`. Measured with `warp test --each` (runtime-split's release binary as host, compiler.wasm built by
`warp rebuild pure` from each lib; base = origin/main dcba843fd lib):
- the 54 test files with an "unknown name x" failure: 462 → 479 of 994 pass (+17, 0 regressions)
- tests/functions: 618 → 631 of 1430 pass (+13, 0 regressions)
- lib/math.warp: unknown name x → next refusal `the call fmin(…)` (from 'm' C imports: Warp FFI's pure-c-imports)
Not covered (other root causes): `def`-keyword variants (Warp Code), `fast f(x)` (a variable declared fast),
`f(x) as int or text` unions, `f(x, y=10) = …` defaults.

## pure-system-values (Ready, branch libgap-view-size, tip 3b70334bd, 2026-10-23)
- Root cause: emitter.warp local_value refused unbound system values (view_width/view_height/gpu_drawing in draw.warp, online, key_down, mouse_*).
- Fix: emitter.warp only (local_value + system_value_read, reached_words imports system_value). Host.warp untouched (pure-c-imports edits host_argument).
- Measured: 15 affected test files (/tmp/lg/svt), warp test with compiler.wasm rebuilt per lib (/workspace/cow/libgap/sv): 67 -> 52 of 114 failed, +15, 0 regressions.
- draw.warp now: "the unknown name canvas" (property accessors `canvas.width := …`, and canvas() beside canvas(w, h)) -> next root cause.
- Left: phrases (dark mode, clipboard count), listeners (whenever online), set_system_value (text to i32 host param, host.warp).

## pure-wasi-environment (Ready, branch libgap-wasi-env, tip a861e6786)
- runtime/wasi.warp wasi_environment() (added after wasi_list, away from wasm-commands' unmerged hunk after std_os_env); library_words maps it.
- Measured: probe `FOO=bar` -> split(wasi_environment(), chr(0)) finds FOO=bar. wagi.warp now stops at `is pair` (markup root cause). No .warp suite case.
- Next: markup/wagi `is pair` (runtime_words core_type_words), draw property accessors (canvas.width :=), router record-field function call.

## pure-is-pair (Ready, libgap-is-pair 8a86f9983)
- core_type_words pair: "key". Measured (pure-only probes): markup.warp moves to "unknown name option" (element blocks); test_type_word_scopes:4 moves to Error("no field 0") (for item in a map held as any: separate bug). Suite gain 0 alone.

## pure-property-accessors (Ready, libgap-canvas-props 3bf55c469)
- New core/property_accessors.warp, hooked in passes.warp with_used before with_type_module_uses.
- Pitfalls met: a global fn `written` shadowed rewrites.warp's local `written: text`; `found#1 + "." + …` parses as found#(1 + …) ("not an int"); lambda `entry => entry#2` -> loop_item error; scope.warp counts `canvas.width = …` as binding canvas.
- Measured: 17 files, 89 -> 77 of 160 failed (+12, 0 regressions), base main a7479a2c2 + system-values.
## pure-field-calls (Ready, libgap-field-calls 64968d5ee)
- lambdas.warp: a field the program gives a function value (a lambda, a `function`-typed parameter, a named function of
  arity > 0) is called through closure_call_N(x.f, args) (dotted_lowered, callable_fields_of).
- Measured (m3: sol-warp-9178bf3a4 copy + compiler.wasm per lib, base = origin/main 44f47d779, 189 .pairs.warp files of
  tests/control + field-function files, 1073 cases): 632 -> 627 failed, +5 (test_field_function_values :4 :6 :8 :18 :21), 0 regressions.
- lib/router.warp (pure-only): refusal `the call answer(…)` -> runs (prints []).
- Not covered: `s = {f: twice}; s.f(4)` (a named function stored as a value: still `the call f(…)`), `o = {f := it*2}; o.f(3)`
  (object functions, test_object_functions).

## pure-property-accessors re-Ready (libgap-canvas-props 8a37ffb70, 2026-10-23 08:07 CEST)
- Bounced: merged on main 27fdb8402, `warp rebuild self` refused `the call Text(…)` (property_written built a Text node).
- Fix: merged origin/main (0427a028a), the read-only raise is parsed: plain_node(parsed("raise \"… is read-only …\"")) (8a37ffb70).
- Measured (box-slot, sol-warp-9178bf3a4 copy, WARP_LIB=the tree's lib): `warp rebuild self` exit 0 (built compiler-self.wasm);
  `warp self hello.warp` (print "hello") exit 0, prints hello.
- Re-measure of the 17 files (base main 27fdb8402 vs 8a37ffb70) still running: /workspace/cow/libgap/cp2/{base,after}.out.
- 17-file re-measure (base main 27fdb8402 vs 8a37ffb70): 89 -> 77 of 160 failed, +12, 0 regressions.

## pure-exit-miscompile (Ready, libgap-exit-never 89807a321) — issue #3098
- emitter.warp group_expression: a host call of `exit` is [never, ended_code(host_call(…))], its code ending in
  unreachable, so try/if take the other branch's kind. Pitfall: `host_call(…)#2 + […]` written in place is read by the
  Rust-compiled stage as a text's character (cast failure); through a `value: any` parameter (as converted) it works.
- New test tests/control/test_pure_exit_branch.warp.
- Measured (m3, base origin/main 44f47d779): 1073 .pairs cases 632 -> 632 (0 gains, 0 regressions; no pairs case uses exit);
  m3/w test_*.warp folder 18 -> 15 of 28 failed (+3: try/catch exit in a function and at top level, if/else exit), 0 regressions.
- box-slot `warp rebuild self` exit 0, `warp self hello.warp` prints hello.

## pure-nil-in-any-map (Ready, libgap-nil-in-map 579d524ed) — issue #3119
- runtime/maps.warp collection_contains / collection_position: ø names no entry (name_text(ø) trapped on the null node).
- New test tests/control/test_pure_nil_in_map.warp (incl. the issue's map-of-instances shape).
- Measured: 1073 .pairs cases 632 -> 632 (0/0); m3/w 18 -> 15 of 28 failed (+3), 0 regressions; probes nil1/nil2 trap on base, right after.
- box-slot `warp rebuild self` exit 0, `warp self hello.warp` prints hello.

## pure-unit-literals (in progress, branch libgap-units-2 off main 38458226b; pcm2 is in main)
- Deviation from the plan, on purpose: the rules are lib/sugar/units.warp (pure-only sugar, in first_sugar_module_names),
  not lib/units.warp: Rust's rewrites.rs applies lib/*.warp std-module rules before its static units pass, and Rust
  parses `3 km` as 3*km too, so rules in lib/units.warp would rewrite Rust's quantities to bare numbers (lost units).
  Also sugar rules (unlike std-module rules) don't match phrase runs, so the parser reads a spaced unit word as a
  product like Rust (parser.warp at_spaced_unit, twin of literals/units.rs) and every rule is `$n * unit`.
- quantity_value(amount, "dimension") holds the SI amount (emitter compiles it as the amount); analyzer.warp
  quantity_refusal (small extension) refuses showing a quantity (value/print/str/text join) and mixed dimensions;
  emitter typed_wasm_module says that refusal.
- Probes: first run had glued rules firing but quantity_value unreached (sugar runs after the prelude reach) -> now an
  emitter word; analyzer map literal {print:…, puts:…} = duplicate key print (aliases) -> list of texts.

## pure-unit-literals: overload/type probe (2026-10-23 ~10:20 CEST), lib origin/main 1ce7b95c5
Pure (measured): instance op dispatch o1/o2/o4 trap "cast failure"; `use units` refuses "unknown name for"; class-as-constructor ms(5) OK; static `ms.of` refused; `5 as ms` refused; text() display of instance OK.
Rust (measured): o1 a*b/+/</== dispatch OK; o4 `operator *` OK; o7 ms*500 OK (0.5); o2/o6 number-left `2*v` FAILS "not an int" (no reflected dispatch); o9 static OK; oa `5 as ms` refused; ob text "5.01km".
Rust timing (measured, wall incl compile): 1M Quantity adds 1.47s vs 1M float adds 0.10s.
Rust lowering: class_methods/operators.rs operator_calls = compile-time static rewrite to method call by known instance class (OPERATOR_METHODS); no Eq method in table (== handled elsewhere).

## PARKED: pure-unit-literals (2026-10-23 10:35 CEST) until Warp's conversion mechanism (`5 as ms`, as/convert) exists
- State: branch libgap-units-2, tip 50addcb2a (pushed), off main 38458226b; NOT measured, NOT Ready.
- Accepted design: one pure sugar rule in lib/sugar/units.warp, `rewrite $n * $u => quantity_value($n, $u) where $n is literal and $u is unit`;
  the new rewrites.warp condition `is unit` reads lib/units.warp's unit_definitions at compile time (core/unit_table.warp:
  exact numerator/denominator per unit, quantity, long name → short unit; simple `quantity`/`unit X := N U`/`U/N`/`name` lines only, no SI prefix generation).
  No template computation needed: the unit word travels in quantity_value; the emitter scales it (emitter/quantities.warp quantity_amount).
  The analyzer keeps the written unit (quantity_facts → kinds.quantities {units, refusal}); showing via print/str/text/joins prints amount/scale + unit
  (emitter quantity_text, sums in the finer unit like Rust's 105cm); mixed kinds → loud "DimensionError: … incompatible units"; text + quantity → DimensionError
  (as Rust); program value of a quantity still refused (pure would show a quoted text). Parser: spaced unit word after a number is a product unless
  `:` or a lone `=` follows (at_spaced_unit / defines_after), unit words from unit_table.
- Panic status (strings.rs:72 first_char unwrap on None, Rust lane while building the pure stages): bisected to e40f04a50, whose only change is
  analyzer.warp `showing_words = {print: 1, …}` → `showing_words = ["print", "str", "text", "text_form", "serialize"]` (a global list of texts,
  used as `head_name(n) in showing_words`). 669d65d84 (parser) fine; 9dcee55fb fails earlier with the map's duplicate key. Still present at 50addcb2a.
  Not yet narrowed inside that line (suspects: a global text list in the analyzer stage, or one of those words, e.g. "text"/"str", read as a type/char).
  No fix found; no Rust change made. Half-run variant bisect in /tmp/lg/bv (vA..vO, stubbed analyzer functions) was killed unfinished.
- Probes: /tmp/lg/u2/p01–p26.warp + run.sh <lib> <out> (500ms, 5 * ms, ms = 2, `{w: 2 ms: 3}`/`{w = 2 ms = 3}`, sleep(20ms)/sleep(5 ms), 1 km == 1000 m,
  str(3m + 2m), print 5 cm + 1 m, "${x}", DimensionErrors, kilograms/grams, 6 km / 2000 m, m², f(3 km), 1.5 h, p26 channel "probe-u2" send/receive);
  Rust-lane expected values noted in this session (e.g. str(3m+2m) "5m", 5 cm + 1 m 105cm, "is ${x}" "is 5km", p26 "hi"; main pure: p26 refuses "sleep with 2 arguments").
  Older probes /tmp/lg/u/p, overload/type probes /tmp/lg/ov (o1–ob, t1/t2 timing), bisect /tmp/lg/bis (run.sh, out).
- Unit table checked in the Rust lane: km 1000/1, ms 1/1000, hours 3600, inch 127/5000, mi 201168/125, lb 45359237/1e8, meters 1, min 60.
## pure-as-conversions (2026-10-23 ~11:10 CEST, branch libgap-as-conversions off d9490fc59) — PAUSED for Supervisor plan OK
- Pushed 22c2ff825: core/conversions.warp (new file, ~70 lines) + definitions.warp hook + errors.data own_type_conversion. Violates the
  later soft pass ban (new core file): uncommitted fold of it into definitions.warp sits in the tree (D conversions.warp, M definitions.warp).
- Local-only tree libgap-as-units (not pushed): merge of libgap-units-2 + quantity_as (`3 km as m`, `5 as ms`) commit aaec4aa36; still has unit_table.warp (new file, must fold).
- Survey probes queued (box slots full 30+ min): /tmp/lg/ac (a1-a12 built-in/units, c1-c8 program conversions; base.out vs after.out, PID 2249671),
  /tmp/lg/acu (u1-u6 units with RUST_BACKTRACE=1, units.out, PID 2261879). Test set for counts: /tmp/lg/act (114 .warp files with `as`), measure.sh in libgap/ac.
- Static survey: built-in `as` (int/float/text/bool/char/list/codepoint/number) already in emitter.warp:911-927 + analyzer converted_kind:219;
  243 `as <typeword>` test lines in 87 files; `text(p: P)` → emitter.warp:856 "a function value in as"; interpolation makes text_form(x) (interpolation.warp:11);
  no `x as list of T` test exists (8 `list of int/text` type lines); 6 unit `as` test lines; general-conversions has Rust tests only (8 is!), no .warp twin.
- 12:00 CEST: pushed tip 5af404bea (fold 6a5ed5258, text(p) 87f9f3a3a, tests a1524debc, as number/int widths 8d708137c, shown clash fix 55536f832, list-of 5af404bea). Probe survey base (main d9490): built-ins OK except `as number` ("as on text/char and node"); class conversions "unknown name it"; to_text "a function value in as"; list of → "operator . on int". measure.sh (PID 2295579) waiting for a box slot: base then after1 (=5af404bea lib snapshot /tmp/lg/acafter1).
- 12:20 CEST: base (origin/main d9490 lib) on /tmp/lg/act (116 files incl. the new test_pure_conversions + test_text_plus_printable): 522 of 1058 failed (536 pass); after1 rebuild running.
- 12:51 CEST: after1 compiler.wasm built OK from the branch lib (rebuild pure passes); after1 test run waiting for a slot (PID 3631894); followup (probes + selfcheck-branch) PID 2727435 queued after it. New test file checked: values only (rule b).
- 13:06 CEST: after1 (5af404bea lib): 503 of 1058 failed (555 pass): +19, 0 regressions (as number, hex/binary, fraction text, fixed widths).
- 13:15 CEST: probes on 5af404bea: c1-c3 'no field name', c4 'field name of a int', c5 trap, c7 'as on text and node', c9 'unknown name p', c10/c11 list-of still '. on int' (the assignment wraps the phrase), c6 'assignment to … as …' (pre-existing). Fix 5b710a86b: typed function per source + any-typed dispatcher; probes2+selfcheck queued (followup2.sh).
- 13:30 CEST: probes on 5b710a86b: c2-c4 'no field name', c5 trap, c7 'a list where a text is needed', c9 prints default Person{…} (text(p) not caught). Fix 72-char: param converted·value + renamed body (8411245e3); selfcheck of 5b710a86b running; probes3 queued.
- 13:41 CEST: selfcheck of 5b710a86b OK: rebuild pure 145s, pure hello, rebuild self 440s, self hello all pass. probes3 (8411245e3 lib) queued, PID 140775.
- 13:55 CEST: probes3 (8411245e3): every conversion probe 'call stack exhausted' in passes.warp: my renamed() clashed with lambdas.warp renamed. Fix round 1: aefe652be uses lambdas' renamed; probes4 queued.
- 14:30 parked (user: cut tokens). tip aefe652be pushed; probes c1-c5,c7,c9,c13,c14 stack-exhausted since 8411245e3 (converted·value param + renamed); x1/x2 bisect job killed unrun. Not Ready.

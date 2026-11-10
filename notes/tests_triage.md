# Tests triage 2026-11-09, updated 2026-11-10

## Update 2026-11-10: run 16:53 at 99d8f7f7a1 (1422 of 15481 failed, 2438 files)

The 05:00 run on 11-10 was still running at 10:00, so this update counts the 11-09 16:53 results: 1024 errors, 398
wrong values. The clusters below come from the error text, grouped (script: the error without position and `(no owner)`). Cards
landed since that run: unknown-name-test, package-tests-registry (test_packages:4 green), library-typed-outcome.

Biggest clusters by cause (new or grown against the table below in **bold**):

| cases | cluster (error) | main files | card |
|--|--|--|--|
| 398 | wrong value, no common error: regex_malformed 9, code_type 8, one_dispatch 8, quasi_quote 6, big_int_bits 6, hyperreals 6, meta_attributes 6 | judge per file | (per file) |
| 40 | wasm trap: cast failure | test_nearest 12, sound_spectrum 4, try_list_index 3, nearest_default 3 | cast-failure-traps |
| 21 | `text_with_at` (text element write) | test_string 12, footguns 6, test_wasm 3 | triage-text-element-write |
| 20 | a function value in `.` (charged fields) | charged_fields 5, object_functions 4, reflection_mode 4, on_error_of_function 3 | charged-field-values |
| 20 | `x!` / `interpret(…)`: running a node as code (14 + 6 unknown interpret) | run_time_blocks, run_time_block_cache, object_blocks | fails-outside |
| **18** | **not a list: `map reverse [[1 2] [3 4]]`, a library word mapped** | map_library_word_first, parameter_scope 3, path_after_function 3, property_definitions 3 | sorted-map-library |
| **12** | **not a function: `twice(body: block) := { body!; body! }`, a block parameter run** | block_parameter_calls, block_parameters, function_objects, count_shadowed | block-values-forced |
| **12** | **unknown name a: a lambda's own parameter or a sibling field, `(a: int, b = 1): int => a + b`, `{a: 3, f: x => x * a}`** | field_function_values, parameter_patterns, block_data_paths, arrow_defaults_and_types | lambda-own (new) |
| 12 | operator . on int (`it` in nested blocks) + 11 unknown name it | nested_it 4, returned_blocks 4, versions 3, glued_method_block 3 | nested-it-blocks |
| **12** | **`xs.insert(4, at: 0)`: insert·list·int·any called with 3 arguments** (was 5) | welcoming_sugar 5, position_meets_brackets 2, list_insert | library-labelled-args |
| 11 | a variable declared fast | fast_scope 7, fast_int 3 | fast-declaration |
| 10 | call stack exhausted | try_stack_overflow 3, numeric 2 | add-stack-overflow |
| 10 | invalid number (`format 2.2 as int` pick) | argument_cast 4, exponent_texts 3 | argument-cast-overload |
| 13 | `session[…]` / `use storage` words | test_storage, store_values | session-storage-words |
| 10 | `•/test/fixtures/wasm/*.wasm`: no folder above . holds .git | wasm_modules, test_wasm, component_short_use | wasm-fixture-root |
| **6** | **`sorted(xs, >)`, `.sort(by: -it)`: sorted_node called with 2 arguments** | relational_arms, it_arguments, named_function_argument, operator_arguments | sorted-comparator-syntax (new) |
| **6** | **no case for value = 0: `match v { Circle => …; Square => … }` over classes** | class_destructuring 3, match_type_arms, structural_patterns | tree-patterns? |

Unchanged since the table below: ∞ 7, divide by zero 7, `x as text?` 6, Color cases 6, calendar/clock durations 6,
`time` defined twice 6, callable parameter 6, tuple returns 5, mixins 5, Array.from 5, display 5.

Basis: test/results.txt (1475 native failures of 15401, run 07:12) and test/tests-web.txt (1398 BOTH, 51 PAGE, 34 EXP).
**results.txt is partly stale**: the 6 "no module named sorted/ring/meshlets/forest/disjoint/aquarium" clusters
(~75 cases) are gone, the libs landed after the run. So the 121 files with 4 or more failures were rerun on
2026-11-09 (787 old cases, 712 still red). The long tail (files with 1 to 3 failures, about 690 cases) was not rerun:
treat those as unverified until the next full run. Card notes carry the exact `file:line` lists ("tests (rerun …)").

Cheap = a lib or one rule, hard = needs core/ (an Interviewer OK first) or a user decision (D).
Order: cases per effort, best first.

| # | cases | cluster | card | guess |
|--|--|--|--|--|
| 1 | 21 | `k#1 = 97` text element write: `outside the core: the call text_with_at` (test_string, test_footguns, test_wasm) | triage-text-element-write | cheap-ish, core (one assignment lowering) |
| 2 | 20 | `x!` / `interpret(…)` run a node as code (test_run_time_blocks, _block_cache, object_blocks) | fails-outside | hard, core, D (run-time eval) |
| 3 | 18 | `it` in method/nested blocks: `[1,2].map{ it*2 }` → "operator . on int", "unknown name it" | nested-it-blocks (new) | medium, core (implicit parameter) |
| 4 | 16 | charged fields / getters `o = {s := 2+3}; o.s` → "a function value in ." | charged-field-values (new) | medium, core |
| 5 | 15 | reflection words (`no field type`, `shapes`, `fib.dump`, operation type) | reflection-words | medium, partly lib |
| 6 | 12 | `nearest(sqrt(2), rational)` and 9 more: wasm trap cast failure (nearest 12, sound_spectrum 4, …) | cast-failure-traps (new) | probably 1 root cause, cheap once found, core? |
| 7 | 12 | `use wasm fixture` `•/test/fixtures/wasm/fourty_two.wasm`: "no folder above . holds .git" | wasm-fixture-root (new) | cheap (test env: root marker) |
| 8 | 11 | `session[…]`, `use storage` words unknown (test_storage, test_store_values) | session-storage-words (new) | cheap, lib/storage |
| 9 | 10 | mixins `class Duck with Walker`, trait constraints | mixins-traits (new) | hard, core + D |
| 10 | 10 | wasm component calls (`shout`, `fib`) | lib-wasm-modules (Done!) | reopen or new card, medium |
| 11 | 9 | `use forest; Glance(…)`: "a int where a text is needed" | forest-glance-text (new) | cheap, lib bug |
| 12 | 8 | `x, y = f()` assignment to a call result (tuple returns, unpacking) | call-destructuring (new) | medium, core |
| 13 | 7 | uniscript versions: `uniscript(…)`, `version` | uniscript-ownership (Done!) | reopen, medium |
| 14 | 7 | `fast g(n) := …`: "a variable declared fast" | fast-declaration (new) | medium, core |
| 15 | 7 | `format 2.2 as int` overload pick: "invalid number" | argument-cast-overload (new) | medium |
| 16 | 7 | `x as text?` unknown name | optional-cast-text (new) | cheap-ish, core (type word) |
| 17 | 6 | `run(f: callable)` | callable-parameter (new) | medium |
| 18 | 5 | `∞` unknown name (test_infinity) | float-overflow-inf | cheap in lib/reals, but ∞() := can't be defined |
| 19 | 5 | `xs.insert(4, at: 0)`: labelled argument on a library word | library-labelled-args (new) | cheap, lib signature |
| 20 | 5 | calendar vs clock duration compare | duration-comparison | medium, D (kind rules) |
| 21 | 5 | `x is Shape` for a type with cases, Color payload tests | shape-rect | medium, core |
| 22 | 5 | `Array.from(xs, f)` js callback | js-array-from-callback (new) | cheap, lib/js (also PAGE) |
| 23 | 4 | `@primitive sq(x: i32)`: fitted_int not primitive | primitive-use-fails | medium, core |
| 24 | 4 | `display.dpi` (test_pixel_unit) | display-dpi-word (new) | cheap, lib |
| 25 | 6 | `type(2024-06-01T12:00Z)` = instant; `time := now` clash "time is defined 2 times" | time-type-names (new) | medium |
| 26 | 4 | `from_json(…)` of a class | class-from-json (new) | medium |
| 27 | 4 | `protocol` default methods | protocol-default-methods (new) | medium, D |
| 28 | 4 | regex error texts differ (also red in the page) | regex-error-messages (new) | cheap, align the messages (D which wording) |
| 29 | 4 | `codepoint` return type: "a node given to a variable declared char" | codepoint-return-type (new) | medium |

Rest of the rerun: "wrong value" cases (got `no`, `0`, `3`: 33 + 8 + 5, spread over test_quasi_quote 4,
test_library_signatures, test_for_implicit_it 3, test_big_int_bits 3, test_suffix_words, …) have no common cause: judge
per file. Existing cards that already cover failing tests but whose count is not in the table: core-try-divide-by-zero
and core-text-times-negative (7 "divide by zero"), collections-binding + collections-set-literal (`int of node`, 21 cases
in test_std_collections, test_collections_structures, test_set_add_after_init), listeners-of-removal (test_reactive_ports),
add-stack-overflow (10 "call stack exhausted"), core-left-number-operator-cast. The umbrella cards lib-text-fails,
lib-functions-fails, lib-std-modules-fails, samples-fails-a-m / n-z are the right home for lib-side leftovers.

## Page-only (PAGE, 51) and native-only by decision (EXP, 34)

PAGE: test/ffi (c_int_arguments 4, c_bytes_results 4, ffi_gaps 4, libm_header_functions 2, js_callbacks 2, host_word_types 2,
c_imports_run 2), test/web (fetch_signals 3, fetch_later 3), regex edges 4, shader_error_place 2, host_word_failures 2.
Mostly C/FFI and file access the page cannot do: decide per file whether the test belongs to EXP.
EXP: modules/test_shell_edges 6, test_exec_sh_runs 4, test_std_exec 3, ffi_float_results 3, webview 2.

## Duplicates and unsure pairs

Closed: parameter-named-like and parameter-named → parameters-must (parameter must win over any global, library or
other file's word).
Unsure, left open, same family "a name shadows another" (probably one root fix, not identical):
steps-inside / function-local / core-global-fixit-let / core-function-sets-global (assignment in a function with a
main-level same name); function-parameter / param-named-fn / parameter-shadows-function / param-named-of;
global-used / function-global; variable-named-like / variable-named-like-2 / level-variable;
headless-ocean-run / ocean-sample-slow (same sample, different symptom).
Cards that reflect fixed things stay: titles are truncated, judge by `todo show`.

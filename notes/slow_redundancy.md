# Slow-lane redundancy (decision slow-redundant, 2026-10-22)

Every test of scripts/slow-tests.txt as of 2026-10-22, the capability it checks, and whether it stays. A test whose
capability another test (slow or fast) already checks, or checks closely enough not to be worth its wait (user:
"not just exact redundancy, also things too similar and not worth the 1 minute wait"), is ignored with
`#[ignore = "redundant: covered by <test>"]` and leaves the slow list. Seconds: data/test_times/over_1s_batch7_2026-10-17.tsv
(a loaded Mac run); "—" = not timed there (the self-hosting tests of the original over-60 s list).

Caveat on savings: most slow tests pay a shared one-time cost (the self-hosted stage compile in
`self_hosted_stage_cache`, `warp rebuild self` behind a OnceLock, a cargo crate's component build, `use sound`'s
module). Ignoring one of them moves that cost to its sibling, so the wall-clock saving is smaller than its seconds.

## Ignored (7)

| test | s | capability | covered by |
|---|---|---|---|
| parser::test_warp_pure::warp_self_runs_hello_world | — | `warp self` compiles and runs a program printing a text | parser::test_warp_pure::warp_self_runs_loops_and_texts (prints a text plus loops and concatenation); scripts/self-check.sh runs hello under `warp self` too |
| parser::test_self_hosted_analysis::a_program_the_analyses_agree_on_runs_self_analyzed | — | analyzer.warp runs beside the front lane and the program still runs | parser::test_self_hosted_analysis::programs_emitted_with_the_analyzed_kinds_give_their_values (analyzer.warp runs and its kinds feed the emitter) |
| parser::test_self_hosted_cache::the_front_end_alone_leaves_emitting_to_rust | — | WARP_SELF_HOSTED=front gives the value; its phase report shows the Rust emitter (an intermediate) | parser::test_self_pure::number_keys_run_in_the_front_lanes (values in the front lane) |
| ffi::test_crate_errors::an_uncaught_crate_error_says_only_what_the_crate_said | 37.6 | an uncaught crate Err is a loud error with the crate's message (and no .wasm path) | ffi::test_cargo_crates::test_cargo_crate_errors_are_loud |
| numbers::test_law::test_law_proved_by_lean | 20.4 | Lean proves a law of a non-recursive function | numbers::test_law::test_law_overflow_promotion_proved_by_lean (6 s) |
| web::test_bound_field_types::a_bound_float_takes_any_number | 11.3 | a field bound to a float takes a number | web::test_bound_field_types::a_bound_int_takes_whole_numbers_only (same form, the stricter type); float's `step="any"` in web::test_form_bindings::a_bound_field_is_validated_by_the_declared_type |
| programs::test_sound_tempo::notes_last_beats | 12.3 | play and melody take `beat(…)` durations | programs::test_sound_tempo::the_tempo_sets_the_beat, a_beat_is_a_share_of_a_minute; melody with a beat in lists::test_empty_list_concat |

Timed seconds of the ignored tests: 81.5 s plus the three untimed self-hosting runs.

## Kept

| test | s | capability (why it stays) |
|---|---|---|
| parser::test_self_analyzer::matching_sources_keep_matching | — | analyzer.warp runs on 1384 lowered sources without failing (the only wide analyzer check; asks "ran", close to an intermediate: question to warp-interviewer) |
| parser::test_self_emitter_times::time_constants_run_as_rust_runs_them | — | emitter.warp's time nodes stay inside the core and give Rust's values |
| parser::test_self_emitter::every_source_inside_the_core_runs_as_rust_runs_it | — | the emitter twin over a daily share of samples and probes |
| parser::test_self_exact_pow::int_powers_run_as_rust_runs_them | — | int `^` stays inside the warp emitter (the wide twin checks only a rotating share and a floor count) |
| parser::test_self_sort::sort_runs_as_rust_sorts | — | runtime.warp's stable sort and sorted_by (same reason) |
| parser::test_self_floor_division::self_hosted_floor_division_prints_as_rust_prints_it | — | parser.warp reads `÷`, `÷=`, `/!`, `div` (no other self-hosted test) |
| parser::test_self_hosted_analysis::programs_emitted_with_the_analyzed_kinds_give_their_values | — | emitter reading analyzer.warp's kinds |
| parser::test_self_hosted_cache::an_identical_library_elsewhere_reuses_the_kept_stages | — | stage cache keyed by lib/'s content |
| parser::test_self_hosted_cache::the_stage_program_is_compiled_once | — | stage cache reused |
| parser::test_self_hosted_cache::the_stage_programs_ignore_the_working_folder | — | stages compile from lib/ alone |
| parser::test_self_pure::a_call_with_comma_arguments_parses_pure | — | enum case bound per function (pure parser) |
| parser::test_self_pure::a_failing_stage_names_its_file | — | a broken stage names its file |
| parser::test_self_pure::a_lexer_over_short_words_takes_linear_time_pure | — | pure runtime's text cache (speed) |
| parser::test_self_pure::a_map_member_beside_its_entry_runs_pure | — | map member beside an entry of its name |
| parser::test_warp_pure::warp_pure_and_self_run_inline_code | — | `warp pure/self <code>` on the command line |
| parser::test_warp_pure::warp_pure_runs_a_library_word | — | a library word through `warp pure` |
| parser::test_warp_pure::warp_self_refuses_a_function_declared_in_two_used_files | — | module name clash under `warp self` |
| parser::test_warp_pure::warp_self_runs_loops_and_texts | — | loops and text concatenation under `warp self` (now also the hello-world cover) |
| ffi::test_run_wasm_library::run_wasm_runs_a_module_given_as_bytes | — | libwarp_runtime runs a module |
| ffi::test_run_wasm_library::run_wasm_of_no_module_fails_loudly | — | its loud failure |
| programs::test_playground_samples::every_playground_sample_runs_without_an_error (+ slices 2–4) | 69, 66, 69, 60 | disjoint quarters of the playground menu |
| types::test_type_model::test_warp_computes_what_the_type_model_computes | 61.1 | values agree with the Lean model |
| types::test_type_model::test_warp_rejects_what_the_type_model_rejects | 29.7 | type rejections agree with the model |
| ffi::test_cargo_crates::test_cargo_crate_struct_is_a_resource | 40.1 | crate struct as a resource (regex) |
| ffi::test_cargo_crates::test_cargo_crate_free_functions | 40.1 | free functions, `use crate`, `as` (strsim) |
| ffi::test_cargo_crates::test_cargo_crate_texts_pass_as_bytes | 40.0 | text passed as &[u8] (hex, crc32fast) |
| ffi::test_cargo_crates::test_cargo_crate_type_functions_fields_and_display | 39.9 | associated functions, fields, Display, resource arguments (semver) |
| ffi::test_cargo_crates::test_cargo_crate_errors_are_loud | 39.0 | crate Err and a missing crate are loud |
| ffi::test_cargo_crates::test_cargo_crate_without_callable_functions_is_refused | 38.5 | a crate warp can't call is refused |
| ffi::test_crate_errors::try_catches_a_crate_error | 37.7 | `try` catches a crate Err |
| numbers::test_recursive_law::recursive_laws_are_proved_by_induction | 27.6 | Lean induction over recursive functions |
| numbers::test_recursive_law::a_proved_law_is_a_lemma_of_the_next | 27.2 | proved laws as lemmas |
| numbers::test_recursive_law::a_precondition_skips_the_instances_it_excludes | 15.4 | property testing honours `=>` (the path without Lean) |
| programs::test_code_golf::every_golf_solution_prints_its_holes_output | 27.2 | golf solutions' outputs |
| web::test_guide::every_guide_snippet_shows_its_value | 25.0 | the playground guide's values |
| web::test_routes::route_patterns_accept_the_url_pattern_syntax | 20.8 | URL pattern syntax |
| web::test_routes::nested_routes_show_inside_their_layout | 13.5 | nested routes |
| web::test_routes::a_typed_route_parameter_computes_with_its_type | 10.3 | `:x:text`, `:x:float` parameters (`:int` alone is in nested routes) |
| web::test_bound_field_types::a_bound_int_takes_whole_numbers_only | 18.4 | typed binding refuses 5.1 |
| programs::test_note_values::a_note_value_prints_as_its_name | 17.4 | note(…) from a frequency, cents, .key, .octave(), .name(), note_value alias |
| web::test_page_tests::passing_page_tests_are_counted | 15.6 | page tests' passing count (the fast siblings test failures) |
| control::test_member_completion::every_offered_builtin_is_a_method_of_its_type | 15.5 | completion offers only real methods |
| parser::test_big_literals::a_100_kb_literal_compiles_in_linear_time | 15.3 | compile speed |
| lists::test_map_variables::two_million_keys_are_set_and_found | 14.7 | map speed |
| web::test_served_forms::the_todo_sample_creates_toggles_and_lists | 13.7 | served sample: POST, toggle, API |
| web::test_served_forms::a_page_starts_from_rows_of_a_server_table | 11.8 | page rendered from stored rows |
| programs::test_sound_patterns::a_cycle_lasts_its_seconds_whatever_its_steps | 13.3 | cycle length under subdivision, repeat, euclid, chords |
| programs::test_sound_patterns::chance_drops_some_cycles_the_same_each_run | 10.3 | `?` seeded chance |
| web::test_components::a_components_plural_names_its_instances | 13.2 | `counters`, `Counter.instances` |
| web::test_components::a_handler_outside_a_component_reads_its_state | 11.6 | `Counter.count#1` (the only test of that form) |
| web::test_form_bindings::a_c_style_declared_variable_is_bound | 11.8 | `int age = 30` bound (bug fix of card bind-ignores; question to warp-interviewer) |
| functions::test_preposition_calls::play_and_melody_take_their_durations_after_for_and_each | 11.6 | `for:`/`each:` durations honoured (the fast sibling checks only that they parse) |
| web::test_field_events::focus_and_blur_reach_their_handlers | 11.3 | focus and blur events |
| programs::test_sound_notes::sharps_and_flats_are_notes | 11.1 | ♯/♭ glyphs (♭ is tested nowhere else) |
| text::test_text_cursor::a_cursor_reads_a_text_by_character_classes | 10.8 | text cursor |
| web::test_dev_server::a_dev_build_keeps_component_state | 10.6 | dev reload keeps state |
| programs::test_sound_clock::sounds_queue_on_the_audio_clock | 10.3 | queued seconds add up |

## Second pass: the 24 added later (18 by the batch-8 merge, 6 by warp-cloud-lane in b645ddb221), all kept

Seconds from the newest data/cloud/*/timings.tsv. The `test_self_pure` tests take only 1–14 s each; they are on the list because
whichever runs first pays the stage compile. Ignoring them would save nothing.

| test | s | capability (why it stays) |
|---|---|---|
| parser::test_warp_pure::warp_self_takes_repeated_parameter_names | 1145 (timed out) | `middle(_, value, _)` under `warp self` (bug pin of the self-compiled compiler) |
| parser::test_warp_pure::warp_self_writes_the_largest_int | 1144 (timed out) | the largest int literal under `warp self` (bug pin) |
| functions::test_bodiless_runtime_words::each_runtime_part_runs_alone | 262 | each core/runtime/ part compiles alone (nothing else checks this; the most expensive test on the list) |
| parser::test_self_float_as_exact::float_as_rational_runs_as_rust_runs_it | 61.5 | `as rational` of a float inside the warp emitter (narrow twin, as for int powers) |
| parser::test_self_float_text::float_texts_of_emitter_warp_are_rusts | 44.1 | shortest float digits in runtime.warp (narrow twin) |
| text::test_text_builders::texts_built_in_turn_grow_linearly | 35.2 | text appends in linear time |
| ffi::test_cargo_crates::test_cargo_crate_trap_names_rust_functions | 33.2 | crate name section in backtraces |
| web::test_fix_buttons_apply::the_samples_fix_buttons_find_their_text | 18.9 | the samples' fix buttons apply |
| functions::test_ported_calls::every_ported_call_form_keeps_its_value | 17.0 | ported call forms |
| parser::test_self_pure::unicode_glyphs_read_self_hosted | 13.9 | glyph operators and symbols in the front and pure lanes |
| parser::test_self_pure::op_cases_run_pure | 13.6 | Op/Bracket enum cases |
| parser::test_self_hosted_cache::the_warp_emitter_emits_inside_its_core | 12.5 | emitter.warp emits closures, classes, try; `go` falls back to Rust |
| parser::test_self_pure::lib_forms_read_self_hosted | 11.2 | keyword keys, operator aliases, `&f`, ∛, ° |
| parser::test_self_hosted_cache::the_warp_emitter_fails_with_rusts_runtime_errors | 8.9 | Rust's runtime error texts from emitter.warp modules |
| parser::test_self_pure::lambdas_run_pure | 9.0 | lambdas, closures, function values |
| parser::test_self_pure::is_tests_types_and_equality_compares_self_hosted | 8.5 | `is` against types and values |
| parser::test_self_pure::type_values_run_pure | 7.4 | types as values |
| parser::test_self_pure::number_keys_run_in_the_front_lanes | 6.5 | number keys in the front lanes (also covers the ignored front-end test) |
| web::test_style_rules::a_selector_with_a_value_is_its_color | 5.7 | style selector shorthand |
| parser::test_self_pure::a_member_named_like_a_prefix_word_reads_pure | 4.3 | `math.sqrt` / `math.abs` members |
| parser::test_self_hosted_cache::the_warp_emitter_imports_host_words | 3.4 | host word imports |
| parser::test_self_pure::parser_words_run_pure | 58.9 | 38 parser-rest probes in the pure lane |
| parser::test_self_pure::an_empty_list_literal_runs_pure | 1.7 | `[]` is a list in the pure lane |
| parser::test_self_pure::a_match_arm_of_a_constant_runs_pure | 1.0 | a `True:` arm (match_arm_forms covers only case patterns) |

# tests/ layout

User 2026-10-03: "probes can be turned into real tests by condensing what really matters … sorted out and just
grouped into folders".

## Rules
- ONE test crate: `tests/main.rs` declares `mod common;` and one `mod <folder>;` per topic folder; each
  `tests/<folder>/mod.rs` declares that folder's files. A new test file goes into the folder of its topic and gets a
  `mod` line in that folder's mod.rs (a file left flat in tests/ with a `mod` line in main.rs still works and gets
  moved by the next tidy batch).
- File names keep their `test_` stem, so filters like `tests/queue.sh -- test_lists::` keep working (cargo filters
  are substrings). Full test names gain the folder: `lists::test_lists::test_index`.
- No `probe_*.rs` in tests/: a probe's cases that matter become tests in the topic file, the rest is dropped.
- Only probes condense (user 2026-10-03, via the Interviewer: "Only probes condense"): a probe line already asserted
  by a test is dropped, the rest moves into the topic file. Assertions in regular test files stay where they are, even
  when duplicated. The test count drops only by probe tests that were duplicates; each one is named in its commit.
- `#[cfg(feature = "native")]` stays on the `mod` line of a native-only file (now in the folder's mod.rs).
- Network (card flaky-tests, 2026-10-12): no third-party hosts like httpbin.org, and no mocks. A test of a request
  goes to a server we run: `common::serve(status, body)` (a fixed answer) or `common::echo_url()` (warp's own server
  answering POST /post with {data, headers}); in the browser the test page's /__stub__ (test_in_browser.py) answers
  both. Where a real public host is the point, it is pannous.com, and the test uses `is_online!` / `online_eval`:
  retried once, logged as `RETRY`, only when no answer came (connection, DNS, timeout), never on a wrong answer.
- Data files stay where they are: tests/fixtures/, tests/warp/ (paths in the tests are relative to the crate root).

## Folders
User 2026-10-03: "OK as listed".

Moved one folder per branch (`tests-tidy-<folder>`), small and quick to merge, because other workers add test files
all the time.

- `parser/`: test_parser test_parser_sugar test_warp_format test_warp_position test_comments test_inline_comments test_meta test_meta_attributes test_attributes test_surface_syntax test_newline_precedence test_juxtaposition test_statement_sequence test_statement_terminators test_statements_after_type test_one_line_statements test_semicolon_square test_symbol_hyphen test_glyph_aliases test_number_glyphs test_superscript_signs test_spaced_required_fields test_dollar_names test_json test_xml
- `node/`: node_values_test test_node test_node_operators test_node_todo test_normalization
- `numbers/`: test_number test_math test_float_assignment test_float_bit_operations test_float_exact_context test_float_parameters test_float_promotion test_float_to_int_range test_declared_float_exact_reals test_rounding_in_functions test_sum_of_decimals test_shift_operators test_fixed_width_ints test_unbounded_int test_rational_type test_angle test_units_arithmetic test_units_compare test_units_followup test_counting_units test_zero_fill test_law test_float_text test_float_zero_and_compound
- `operators/`: test_operators test_operator_parsing test_operator_declarations test_less_than_compare test_equality_never_chains test_structural_equality test_logic_grouped_operands test_mutating_bang test_negated_call test_add_to test_in_position test_truthiness_of_objects test_like test_logical_calls (+ probe_operators, probe_precedence, probe_increment condensed)
- `types/`: test_types test_type_of test_type_name_matching test_type_of_real_variable test_type_test_is_only test_type_tests test_type_upgrading test_type_words test_typed_arrays test_generic_types test_array_types test_cast_bugs test_records_classes test_traits test_struct_field_of_constructor test_struct_types test_person_struct test_constructor_vs_data test_data test_types_scope test_float_fields test_records test_spaced_construction test_type_word_user_function (+ probe_type condensed)
- `lists/`: test_lists test_index_assignment test_list_arithmetic test_list_number_comparison test_list_parameters test_list_plus_number test_list_truthiness test_min_max test_min_max_lists test_empty_list_argument test_empty_list_count test_decimal_list_elements test_signed_operand_list test_size_count test_size_property_word test_bare_list_assignment test_key_lookup test_key_subscript_hint test_typed_lists test_tuples test_unpacking (+ probe_index_loop condensed)
- `text/`: test_string test_text_bytes test_text_casts test_text_concat test_text_functions test_text_getters test_text_runtime test_text_variable_assignment test_interpolation test_quote_output test_cast_to_string test_constant_expression_text test_declared_text_one_character test_library_unicode test_runtime_text_as_int test_print_arguments test_print_type_error test_text_repeat test_text_as_float
- `control/`: test_if_call_condition test_condition_block test_not_condition_block test_switch_match test_switch_no_case_value test_switch_value test_for_loop test_loop_forms test_loops_in_functions test_while_paren_condition test_while_value test_iteration_words test_control_words test_do_block test_try_deep test_try_else test_times_count_once test_trap_messages test_blocks test_empty_block test_empty_block_binding test_top_level_block test_block_assigns_outer test_if_value_kind test_try_exits_and_naming test_try_else_value
- `functions/`: test_functions test_def_forms test_tuple_returns test_function_keyword test_function_values test_lambdas test_closures test_block_function test_bare_function_name test_argument_kinds test_parameter_call_kinds test_parameter_codepoint_calls test_undefined_calls test_to_definition test_method_words test_suffix_words test_property_access test_property_assign test_property_words test_optional_words test_effects test_multi_value test_object_arguments test_return_type_dispatch test_typed_returns test_user_method_form (+ probe_def_syntax, tests/probes/probe_function_def condensed)
- `scope/`: test_globals test_global_constant_words test_global_declaration test_global_modifiers test_data_scope test_use_scopes test_undefined_variable test_prefixed_declarations test_export_declaration test_colon_body_assignment test_colon_body_extent test_eval_state
- `modules/`: test_include test_use_modules test_module_cache test_packages test_package_pin test_package_tools test_versions test_header_search test_folder_scope
- `ffi/`: test_ffi test_ffi_import_group test_ffi_warning_once test_host test_wasi test_download test_host_words test_libm_linking
- `wasm/`: test_wasm test_wasm_emitter test_emitter test_wasm_reader test_wast test_gc_name_registry test_gc_struct test_name_subsection_order test_wasm_names_order wasm_optimizer_test test_optimizer_exceptions test_optimizer_extended_const test_read_bytes_plain_result test_compile_only test_wit test_wit_types
- `web/`: test_web test_web_playground test_uniscript
- `welcoming/` (one folder, user OK 2026-10-03): test_footguns test_welcoming_ask test_welcoming_break test_welcoming_count_argument test_welcoming_elements test_welcoming_empty_push test_welcoming_globals test_welcoming_indent test_welcoming_listexpr test_welcoming_listparams test_welcoming_maps test_welcoming_parse test_welcoming_print test_welcoming_rangeblock test_welcoming_slices test_welcoming_sugar test_got_it_warnings test_warning_mode test_style_dont_care test_it_shadow_warning test_c_style (+ probe_footguns condensed)
- `programs/`: test_algo_dijkstra test_algo_levenshtein test_algo_life test_algo_queens test_algo_sieve test_algo_sorting test_all_samples test_samples test_kitchensink
- `sweeps/`: test_todo test_panic_sweep test_sweep_fixes (regression sweeps; test_todo's passing cases move to their topic file)

Not tests: tests/notes/ → notes/OLD/, tests/probes/probe_fib_parsing.rs condensed into parser/.

## Progress
- web/ merged 2026-10-03 (209b058d)
- operators/ (tests-tidy-operators): probe_operators, probe_precedence, probe_increment condensed into test_operator_parsing.rs
- types/ (tests-tidy-types): probe_type condensed into test_type_of.rs
- lists/ (tests-tidy-lists): probe_index_loop condensed into test_index_assignment.rs
- functions/ (tests-tidy-functions): probe_def_syntax condensed into test_def_forms.rs; tests/probes/ (never compiled) removed
- parser/ (tests-tidy-parser): pure move
- node/ (tests-tidy-node): pure move
- numbers/ (tests-tidy-numbers): pure move
- text/ (tests-tidy-text): pure move
- control/ (tests-tidy-control): pure move
- scope/ (tests-tidy-scope): pure move
- modules/ (tests-tidy-modules): pure move
- ffi/ (tests-tidy-ffi): pure move
- wasm/ (tests-tidy-wasm): pure move
- programs/ (tests-tidy-programs): pure move
- sweeps/ (tests-tidy-sweeps): pure move
- welcoming/ (tests-tidy-welcoming): probe_footguns condensed into test_footguns.rs (the Solved catalog of notes/footguns.md)
- leftovers (tests-tidy-leftovers): 15 flat files added meanwhile moved; probe_destructuring promoted to lists/test_unpacking.rs

## Child-process tests
A test running the warp binary (common::warp_command) reports a failed run whole: how it ended (`run.status`: an exit
code, or the signal that killed it under load), its stdout and stderr, as tests/programs/test_sound_spectrum.rs
run_report does. An assert printing only stderr hid why the visualizer run failed in a loaded gate (cards
visualizer-test-exits, visualizer-test-flakes). An environment skip (no WebGPU adapter) stays separate from a failure.

## Consolidation by judgment (card probes-check, user 2026-10-16)
A probe becomes test assertions only when it checks decided behaviour with a stable value; exploratory, visual,
sound, timing and undecided ones stay. A probe listed in tests/parser/self_parser_sources.txt keeps its parse parity:
its text moves into tests/fixtures/parser_corpus/<folder>.warp (snippets concatenated, names as comments).
- welcoming (consolidate-welcoming): probes/{listparams,maps,slices,listexpr,elements,break,print,globals}/ deleted.
- functions (consolidate-functions): english_setter, function_ref_compare, runtime_function_equality,
  returned_list_alias, union_result, quantity_default, methods_extension, thrice_block, calling_closure,
  function_values_tour, function_objects/doc_strings, function_values/ deleted. Kept: print_closure (`print &f, g`
  undecided), effects_last, fail_effects (card print-effects), functions/deep_recursion (stack size),
  functions/fade_unit_argument (sound), function_objects/print_parse (exploratory).
- algo/ (549 bisection snippets, no expected values; samples/<algo>.warp are the tests): kept.
- unreferenced probes (consolidate-unref, list from warp-repo-cleanup): 43 one-off probes deleted, every case already
  asserted by its card's test but copy of an instance (test_named_arguments). Kept by class: markup with warnings
  (accessibility, attribute_values, markup_mistakes, field_events, style_rules, page_test_counter), shaders and
  painting, sound, stored tables (orm_*, units_stress, unit_value, stored), timing (json_parse, global_typed_array,
  orm_identity, await_any_gated, timers_listen), server/CLI (program_server, use_script, warning_levels, expect,
  types_first_run), open questions (average_undefined, print_quantity, natural_chain_abs, component_imports),
  print_closure and fail_effects (card print-effects), takes_three (uniscript module), duration_as_unit,
  duration_in_unit, print_units (executable: user-touched, never deleted); the .py/.sh/.mjs/.md/.lean
  files are tools and research.
- matching (consolidate-matching): 25 of probes/matching/ deleted, their cases asserted in tests/control/test_match_*,
  test_tree_find, test_parameter_patterns, test_plain_enum. Kept: definition_cases (its diagnostic is cut,
  cleanup-cuts-2), gap_feeding (phrase calls parked), tree_patterns (design examples), inventory.sh (spec inventory),
  enum_int_fix, parameter_patterns, variant_equality (executable: user-touched, never deleted).
- semantics (consolidate-semantics): probes/{semantics,json5,class_forms_ruby}/ deleted, and of self_gaps/, key_names/,
  values_keys/ all but text_costs (fuel), esc_exits (window), class_map (10000 instances), character_text and
  list_text_quotes (shebang scripts: user-touched, never deleted). The 12 in the self-parser
  corpus move into tests/fixtures/parser_corpus/semantics.warp. generators/ kept (cut: cleanup-drop-generators),
  laws/ kept (Lean).

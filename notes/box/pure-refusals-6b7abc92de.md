# Pure-lane refusals, suite 6b7abc92de

5623 failing tests (5107 errors, 516 wrong values) of 10720; reasons normalized (names, positions, paths stripped).

| # | reason | tests | examples |
|---|---|---|---|
| 1 | unknown name (top: match 186, on 159, go 111, x 106, a 79, samples 47, it 45, now 45, square 40, switch 36, shared 33, stored 30) | 2041 | tests/control/test_after.pairs.warp:4, tests/control/test_after.warp:4 |
| 2 | wrong value (no error) | 516 | tests/control/test_code_holes.warp:44, tests/control/test_control_words.warp:6 |
| 3 | an instance of C, a class with methods | 221 | tests/control/test_function_objects.warp:58, tests/control/test_function_objects.warp:62 |
| 4 | the group (… | 142 | tests/control/test_blocks.pairs.warp:7, tests/control/test_match_tuples.warp:4 |
| 5 | a data | 104 | tests/parser/test_time_unit_keys.pairs.warp:8, tests/types/test_date_arithmetic_at_run_time.pairs.warp:4 |
| 6 | a number | 100 | tests/modules/test_std_math_text.pairs.warp:52, tests/modules/test_std_math_text.pairs.warp:53 |
| 7 | a function value in . | 68 | tests/control/test_data_string.warp:4, tests/control/test_doc_strings.warp:6 |
| 8 | wasm trap: cast failure | 68 | tests/control/test_try_division.pairs.warp:5, tests/control/test_try_division.pairs.warp:6 |
| 9 | a list where a text is needed | 64 | tests/control/test_block_parameters.pairs.warp:16, tests/control/test_run_time_blocks.pairs.warp:38 |
| 10 | no field F | 62 | tests/control/test_charged_fields.pairs.warp:7, tests/control/test_function_objects.warp:25 |
| 11 | a value after an assignment | 62 | tests/control/test_event_footguns.pairs.warp:38, tests/control/test_event_footguns.warp:23 |
| 12 | an assignment to `… : …` | 57 | tests/control/test_database_filters.warp:4, tests/control/test_error_not_in_data.pairs.warp:15 |
| 13 | call f | 55 | tests/control/test_error_branch_kind.pairs.warp:20, tests/control/test_object_functions.pairs.warp:4 |
| 14 | an exact power needs an integer exponent | 48 | tests/control/test_library_signatures.pairs.warp:26, tests/modules/test_node_json.pairs.warp:24 |
| 15 | an int beyond N bits | 37 | tests/functions/test_multi_value.pairs.warp:4, tests/functions/test_multi_value.pairs.warp:5 |
| 16 | the type of a list | 34 | tests/lists/test_empty_results.pairs.warp:5, tests/lists/test_global_typed_lists.pairs.warp:17 |
| 17 | a none as a value | 31 | tests/functions/test_definition_bodies.warp:13, tests/parser/test_rust_items.pairs.warp:4 |
| 18 | call si_amount | 30 | tests/lists/test_element_wise_vectors.warp:4, tests/lists/test_element_wise_vectors.warp:7 |
| 19 | wasm trap: wasm `unreachable` instruction executed | 29 | tests/functions/test_argument_cast.pairs.warp:4, tests/functions/test_bare_function_name.pairs.warp:10 |
| 20 | call dir | 28 | tests/control/test_library_signatures.pairs.warp:6, tests/control/test_reflection_words.pairs.warp:22 |
| 21 | call cos | 28 | tests/ffi/test_ffi.warp:77, tests/ffi/test_ffi.warp:82 |
| 22 | a node for a typed field of Sample | 27 | tests/functions/test_arity_and_type_overloads.warp:16, tests/programs/test_music_values.warp:10 |
| 23 | operator as on text and node | 27 | tests/numbers/test_quantity_cast.pairs.warp:7, tests/types/test_as_number_variable.pairs.warp:4 |
| 24 | not a list | 25 | tests/control/test_code_holes.warp:57, tests/control/test_rust_option.pairs.warp:11 |
| 25 | call sin | 23 | tests/ffi/test_builtin_lines.warp:4, tests/ffi/test_ffi.warp:68 |
| 26 | an assignment to an element of a text | 23 | tests/text/test_string.pairs.warp:38, tests/text/test_string.pairs.warp:39 |
| 27 | call count | 21 | tests/control/test_field_of_elements.warp:7, tests/control/test_named_patterns.warp:23 |
| 28 | the operator ≈ on int | 21 | tests/operators/test_alike_in_warp.pairs.warp:7, tests/operators/test_approximately.pairs.warp:4 |
| 29 | running a node as code (`x!`) | 20 | tests/control/test_block_parameters.pairs.warp:4, tests/control/test_block_parameters.pairs.warp:8 |
| 30 | `once …` (library word pending) | 18 | tests/control/test_count_word_blocks.pairs.warp:7, tests/control/test_event_footguns.pairs.warp:23 |
| 31 | call interpret | 16 | tests/control/test_host_word_failures.pairs.warp:4, tests/control/test_run_time_blocks.pairs.warp:4 |
| 32 | call fabs | 15 | tests/ffi/test_ffi.pairs.warp:16, tests/ffi/test_ffi.warp:72 |
| 33 | call play | 15 | tests/functions/test_preposition_calls.warp:82, tests/programs/test_music_values.warp:4 |
| 34 | call kind_mark | 15 | tests/types/test_kind_marks.pairs.warp:4, tests/types/test_kind_marks.pairs.warp:5 |
| 35 | cannot access length code of bool | 14 | tests/functions/test_function_words_consume.pairs.warp:16, tests/functions/test_optional_words.pairs.warp:26 |
| 36 | invalid number | 14 | tests/numbers/test_exponent_texts.pairs.warp:4, tests/numbers/test_exponent_texts.pairs.warp:5 |
| 37 | the declaration global `… = …` | 14 | tests/scope/test_export_declaration.pairs.warp:8, tests/scope/test_export_declaration.pairs.warp:9 |
| 38 | the operator ∛ with nothing before it | 13 | tests/ffi/test_ffi_one_parser.pairs.warp:5, tests/ffi/test_libm_header_functions.pairs.warp:6 |
| 39 | call insert | 13 | tests/lists/test_inserted_word.pairs.warp:10, tests/lists/test_inserted_word.pairs.warp:11 |
| 40 | an error value | 13 | tests/modules/test_std_exec.pairs.warp:4, tests/modules/test_std_exec.pairs.warp:5 |
| 41 | an assignment to a type | 13 | tests/types/test_sum_types.pairs.warp:8, tests/types/test_type_alias_fields.pairs.warp:4 |
| 42 | call parse | 12 | tests/control/test_field_of_passed_list.warp:12, tests/ffi/test_cargo_crates.pairs.warp:19 |
| 43 | call twice | 12 | tests/control/test_reflection_words.pairs.warp:33, tests/control/test_reflection_words.pairs.warp:34 |
| 44 | call ceil | 12 | tests/ffi/test_ffi.pairs.warp:4, tests/ffi/test_ffi.pairs.warp:13 |
| 45 | index out of range: N not in N…N | 12 | tests/functions/test_closures.pairs.warp:46, tests/functions/test_closures.pairs.warp:47 |
| 46 | cannot access length code of int | 12 | tests/functions/test_optional_words.pairs.warp:25, tests/functions/test_random_dispatch.pairs.warp:18 |
| 47 | an assignment to `… as …` | 12 | tests/functions/test_render_as.warp:4, tests/functions/test_render_as.warp:5 |
| 48 | operator xor on bool and bool | 12 | tests/wasm/test_wasm.pairs.warp:261, tests/wasm/test_wasm.pairs.warp:262 |
| 49 | call puti | 11 | tests/control/test_braceless_output_calls.pairs.warp:4, tests/control/test_braceless_output_calls.pairs.warp:5 |
| 50 | `n times` (library word pending) | 11 | tests/control/test_control_words.pairs.warp:14, tests/control/test_control_words.pairs.warp:15 |
| 51 | call map | 11 | tests/control/test_for_it.pairs.warp:6, tests/control/test_for_it.warp:5 |
| 52 | a call of f with N arguments | 11 | tests/control/test_switch_match.pairs.warp:32, tests/functions/test_default_parameters.pairs.warp:33 |
| 53 | call log | 11 | tests/ffi/test_foreign_js.pairs.warp:18, tests/functions/test_shadowed_log.pairs.warp:7 |
| 54 | call in | 11 | tests/numbers/test_dynamic_units.pairs.warp:4, tests/numbers/test_dynamic_units.pairs.warp:5 |
| 55 | the operator ~ on int | 11 | tests/operators/test_rough_similarity.pairs.warp:4, tests/operators/test_rough_similarity.pairs.warp:6 |
| 56 | an instance of C, a class not declared here | 11 | tests/parser/test_glued_method_block.pairs.warp:9, tests/parser/test_references.pairs.warp:4 |
| 57 | the field name of a list | 10 | tests/control/test_call_result_reflection.warp:4, tests/control/test_call_result_reflection.warp:5 |
| 58 | the operator do on int | 10 | tests/control/test_do_block.pairs.warp:12, tests/control/test_loops_in_functions.pairs.warp:5 |
| 59 | call methods | 10 | tests/control/test_function_objects.warp:4, tests/control/test_function_objects.warp:67 |
| 60 | call stringify | 10 | tests/ffi/test_foreign_js.pairs.warp:7, tests/ffi/test_js_callbacks.warp:4 |
| 61 | a call of random with N arguments | 10 | tests/ffi/test_host_words.pairs.warp:5, tests/ffi/test_host_words.warp:4 |
| 62 | call x | 10 | tests/functions/test_arrow_result_type.pairs.warp:12, tests/functions/test_functions.pairs.warp:9 |
| 63 | call exp | 10 | tests/functions/test_float_values_of_tuples.warp:4, tests/functions/test_float_values_of_tuples.warp:5 |
| 64 | operator === on list and list | 10 | tests/lists/test_list_identity.pairs.warp:4, tests/lists/test_list_identity.pairs.warp:5 |
| 65 | call area | 10 | tests/modules/test_import_compiled.warp:4, tests/modules/test_import_compiled.warp:22 |
| 66 | call uniscript | 10 | tests/modules/test_package_registry.pairs.warp:4, tests/modules/test_packages.pairs.warp:4 |
| 67 | the operator as on int | 10 | tests/types/test_class_json.pairs.warp:7, tests/types/test_class_json.warp:8 |
| 68 | call fib | 9 | tests/control/test_run_time_blocks.warp:4, tests/ffi/test_component_short_use.warp:4 |
| 69 | call floor | 9 | tests/ffi/test_browser_dollar_names.pairs.warp:4, tests/ffi/test_foreign_python.pairs.warp:5 |
| 70 | call atoi | 9 | tests/ffi/test_ffi.pairs.warp:32, tests/ffi/test_ffi.pairs.warp:33 |
| 71 | call strlen | 9 | tests/ffi/test_ffi.pairs.warp:34, tests/ffi/test_ffi.warp:12 |
| 72 | call strcmp | 9 | tests/ffi/test_ffi.warp:42, tests/ffi/test_ffi.warp:44 |
| 73 | an assignment to the call f(…) | 9 | tests/functions/test_tuple_returns.pairs.warp:4, tests/functions/test_tuple_returns.pairs.warp:10 |
| 74 | call dot | 9 | tests/lists/test_element_wise_lists.pairs.warp:12, tests/lists/test_element_wise_lists.pairs.warp:21 |
| 75 | call quantity | 9 | tests/numbers/test_dynamic_units.pairs.warp:8, tests/numbers/test_dynamic_units.pairs.warp:9 |
| 76 | operator as on char and node | 9 | tests/types/test_as_number_variable.warp:4, tests/types/test_as_number_variable.warp:7 |
| 77 | operator + on bool and int | 9 | tests/types/test_bool_assign.pairs.warp:17, tests/types/test_bool_assign.pairs.warp:18 |
| 78 | a parameter of kind block | 8 | tests/control/test_block_parameter_calls.pairs.warp:4, tests/control/test_block_parameter_calls.pairs.warp:5 |
| 79 | call help | 8 | tests/control/test_doc_strings.warp:21, tests/control/test_doc_strings.warp:27 |
| 80 | a parameter of kind [text] | 8 | tests/control/test_text_patterns.pairs.warp:25, tests/control/test_text_patterns.pairs.warp:26 |
| 81 | call g | 8 | tests/functions/test_field_function_values.pairs.warp:6, tests/functions/test_named_arguments.pairs.warp:9 |
| 82 | a call of node_copy with N arguments | 8 | tests/functions/test_named_arguments.pairs.warp:18, tests/functions/test_named_arguments.warp:18 |
| 83 | quoted code whose position or comment is read | 8 | tests/node/test_parse_word.pairs.warp:34, tests/node/test_parse_word.pairs.warp:35 |
| 84 | call byte_slice | 8 | tests/programs/test_calculator_fixes.pairs.warp:6, tests/programs/test_calculator_fixes.warp:20 |
| 85 | operator * on text and int | 8 | tests/text/test_ascii_index_costs.warp:4, tests/text/test_print_type_error.pairs.warp:4 |
| 86 | a call of sleep with N arguments | 7 | tests/control/test_channels.pairs.warp:8, tests/control/test_channels.warp:10 |
| 87 | a call of sorted_node with N arguments | 7 | tests/control/test_relational_arms.pairs.warp:4, tests/functions/test_it_arguments.pairs.warp:7 |
| 88 | divide by zero | 7 | tests/control/test_try_index_and_float.pairs.warp:16, tests/numbers/test_nan_observed.pairs.warp:4 |
| 89 | operator * on list and list | 7 | tests/functions/test_concatenating_parameter.pairs.warp:6, tests/functions/test_map_phrase.pairs.warp:14 |
| 90 | call pop | 7 | tests/lists/test_aliased_changes.pairs.warp:8, tests/lists/test_aliased_changes.pairs.warp:9 |
| 91 | a test of the type time | 7 | tests/numbers/test_dimension_types.pairs.warp:13, tests/numbers/test_dimension_types.pairs.warp:14 |
| 92 | the operator if with nothing before it | 7 | tests/numbers/test_time_of_day.pairs.warp:5, tests/numbers/test_unit_literals.pairs.warp:11 |
| 93 | a function value in / | 7 | tests/parser/test_home_paths.warp:19, tests/parser/test_parameter_scope.warp:4 |
| 94 | the operator :: on int | 7 | tests/parser/test_rust_paths.pairs.warp:16, tests/types/test_enum_cases.pairs.warp:5 |
| 95 | the field names of a list | 6 | tests/control/test_call_result_reflection.warp:6, tests/control/test_field_of_passed_list.warp:8 |
| 96 | call cell_new | 6 | tests/control/test_cell_truth.pairs.warp:4, tests/control/test_cell_truth.pairs.warp:5 |
| 97 | `twice …` (library word pending) | 6 | tests/control/test_count_word_blocks.pairs.warp:6, tests/control/test_count_word_blocks.pairs.warp:11 |
| 98 | call for | 6 | tests/control/test_for_forms.pairs.warp:4, tests/control/test_for_forms.pairs.warp:5 |
| 99 | call meta | 6 | tests/control/test_meta_values.warp:4, tests/control/test_meta_values.warp:6 |
| 100 | a call of node_text with N arguments | 6 | tests/control/test_plural_field_of_elements.pairs.warp:4, tests/control/test_plural_field_of_elements.pairs.warp:5 |
| 101 | call sqlite3_open | 6 | tests/ffi/test_ffi_handles.warp:4, tests/ffi/test_ffi_handles.warp:5 |
| 102 | call date | 6 | tests/ffi/test_foreign_handles.pairs.warp:7, tests/ffi/test_foreign_handles.warp:4 |
| 103 | call shout | 6 | tests/modules/test_wasm_modules.pairs.warp:20, tests/modules/test_wasm_modules.pairs.warp:22 |
| 104 | a variable declared real | 6 | tests/numbers/test_float_exact_context.pairs.warp:17, tests/numbers/test_float_exact_context.pairs.warp:18 |
| 105 | operator ≈ on list and list | 6 | tests/operators/test_alike_in_warp.pairs.warp:6, tests/operators/test_alike_in_warp.pairs.warp:11 |
| 106 | the declaration global `… := …` | 6 | tests/scope/test_export_declaration.pairs.warp:15, tests/scope/test_global_modifiers.pairs.warp:13 |
| 107 | the operator === on int | 6 | tests/types/test_same_identity.pairs.warp:4, tests/types/test_same_identity.pairs.warp:5 |
| 108 | the operator # on int | 5 | tests/control/test_blocks.pairs.warp:8, tests/control/test_try_index_and_float.pairs.warp:5 |
| 109 | `thrice …` (library word pending) | 5 | tests/control/test_count_word_blocks.pairs.warp:4, tests/control/test_count_word_blocks.pairs.warp:5 |
| 110 | the import fd_write, not declared | 5 | tests/control/test_error_words.pairs.warp:8, tests/control/test_error_words.pairs.warp:9 |
| 111 | wasm trap: null reference | 5 | tests/control/test_library_signatures.pairs.warp:5, tests/operators/test_postfix_words.pairs.warp:4 |
| 112 | not a text | 5 | tests/control/test_named_patterns.warp:51, tests/modules/test_include.pairs.warp:4 |
| 113 | call fmin | 5 | tests/ffi/test_ffi.pairs.warp:6, tests/ffi/test_ffi.warp:26 |
| 114 | call fmax | 5 | tests/ffi/test_ffi.pairs.warp:7, tests/ffi/test_ffi.pairs.warp:17 |
| 115 | call randomUUID | 5 | tests/ffi/test_foreign_js.pairs.warp:11, tests/ffi/test_web_idl.pairs.warp:4 |
| 116 | call sqrt | 5 | tests/ffi/test_foreign_type_tests.pairs.warp:6, tests/ffi/test_foreign_type_tests.pairs.warp:7 |
| 117 | a parameter of kind callable | 5 | tests/functions/test_callable_parameter.warp:4, tests/functions/test_callable_parameter.warp:7 |
| 118 | the field map of a list | 5 | tests/functions/test_nested_it.pairs.warp:8, tests/functions/test_nested_it.pairs.warp:9 |
| 119 | a call of total with N arguments | 5 | tests/functions/test_rust_references.pairs.warp:26, tests/functions/test_variadic.pairs.warp:4 |
| 120 | index must be an integer; fix: compute it with /! (floor division) or `… as int` | 5 | tests/lists/test_hash_range_slices.pairs.warp:4, tests/lists/test_hash_range_slices.pairs.warp:5 |
| 121 | the operator . on int | 5 | tests/modules/test_versions.pairs.warp:4, tests/modules/test_versions.pairs.warp:10 |
| 122 | could not run the program: failed to compile: wasm[N]::function[N]::exp: WebAssembly trans | 5 | tests/numbers/test_fast_math.pairs.warp:7, tests/numbers/test_fast_math.pairs.warp:8 |
| 123 | call ln | 5 | tests/numbers/test_log_base.pairs.warp:6, tests/programs/test_calculator_fixes.pairs.warp:12 |
| 124 | call operator_⌞ | 5 | tests/numbers/test_log_glyphs.pairs.warp:4, tests/numbers/test_log_glyphs.pairs.warp:5 |
| 125 | call operator_⌟ | 5 | tests/numbers/test_log_glyphs.pairs.warp:12, tests/wasm/test_wasm.pairs.warp:455 |
| 126 | call hertz | 5 | tests/programs/test_note_values.warp:37, tests/programs/test_sound.warp:8 |
| 127 | the field words of a text | 5 | tests/text/test_selector_prepositions.pairs.warp:8, tests/text/test_selector_prepositions.pairs.warp:9 |
| 128 | a variable declared char | 5 | tests/types/test_codepoint_digit.pairs.warp:12, tests/types/test_codepoint_digit.pairs.warp:13 |
| 129 | a test of the type unit | 5 | tests/types/test_empty_type_aliases.pairs.warp:4, tests/types/test_empty_type_aliases.pairs.warp:5 |
| 130 | call log10 | 5 | tests/wasm/test_wasm.pairs.warp:450, tests/wasm/test_wasm.pairs.warp:453 |
| 131 | call round_half_up | 5 | tests/welcoming/test_footguns.pairs.warp:366, tests/welcoming/test_footguns.pairs.warp:367 |
| 132 | not a function | 4 | tests/control/test_block_parameter_calls.warp:7, tests/functions/test_block_function_argument.warp:4 |
| 133 | `]` closes no group | 4 | tests/control/test_blocks.pairs.warp:11, tests/control/test_blocks.pairs.warp:12 |
| 134 | call reduce | 4 | tests/control/test_iteration_words.pairs.warp:5, tests/control/test_iteration_words.pairs.warp:6 |
| 135 | call Regex | 4 | tests/ffi/test_cargo_crates.pairs.warp:10, tests/ffi/test_cargo_crates.pairs.warp:11 |
| 136 | call compressBound | 4 | tests/ffi/test_ffi_gaps.pairs.warp:4, tests/ffi/test_ffi_gaps.pairs.warp:5 |
| 137 | call getenv | 4 | tests/ffi/test_ffi_gaps.pairs.warp:19, tests/ffi/test_ffi_gaps.pairs.warp:20 |
| 138 | call atan2 | 4 | tests/ffi/test_foreign_lines.pairs.warp:4, tests/modules/test_std_shadowed_names.pairs.warp:5 |
| 139 | call hypot | 4 | tests/ffi/test_foreign_lines.pairs.warp:5, tests/ffi/test_libm_linking.warp:4 |
| 140 | call puts | 4 | tests/ffi/test_wasi.pairs.warp:4, tests/lists/test_lists.pairs.warp:34 |
| 141 | call add | 4 | tests/functions/test_argument_labels.pairs.warp:8, tests/functions/test_functions.pairs.warp:42 |
| 142 | call greet | 4 | tests/functions/test_argument_labels.warp:4, tests/modules/test_import_compiled.warp:6 |
| 143 | call apply | 4 | tests/functions/test_argument_labels.warp:8, tests/functions/test_argument_labels.warp:9 |
| 144 | a call of play with N arguments | 4 | tests/functions/test_arity_and_type_overloads.warp:20, tests/functions/test_arity_and_type_overloads.warp:24 |
| 145 | call filter | 4 | tests/functions/test_foreign_spellings.pairs.warp:28, tests/functions/test_it_arguments.pairs.warp:5 |
| 146 | a parameter of kind string | 4 | tests/functions/test_parameter_overloads.warp:9, tests/types/test_types.warp:4 |
| 147 | call a | 4 | tests/lists/test_lists.pairs.warp:71, tests/lists/test_lists.pairs.warp:78 |
| 148 | call map_text | 4 | tests/modules/test_case_table_module.pairs.warp:4, tests/modules/test_case_table_module.pairs.warp:5 |
| 149 | call crc32 | 4 | tests/modules/test_wasm_modules.pairs.warp:39, tests/modules/test_wasm_modules.pairs.warp:40 |
| 150 | operator * on char and int | 4 | tests/numbers/test_units_type_holes.pairs.warp:14, tests/text/test_repeat_typed.pairs.warp:4 |
| 151 | the operator ≈ on float | 4 | tests/operators/test_approximately.pairs.warp:25, tests/operators/test_approximately.pairs.warp:26 |
| 152 | the operator % with nothing after it | 4 | tests/operators/test_postfix_words.pairs.warp:12, tests/operators/test_postfix_words.pairs.warp:13 |
| 153 | operator + on bool and bool | 4 | tests/types/test_bool_type.pairs.warp:23, tests/wasm/test_wasm.pairs.warp:34 |
| 154 | call speak | 4 | tests/types/test_class_cases_ported.warp:16, tests/types/test_class_cases_ported.warp:17 |
| 155 | call from_json | 4 | tests/types/test_class_json.warp:9, tests/types/test_class_json.warp:10 |
| 156 | call double | 4 | tests/types/test_class_methods.pairs.warp:14, tests/types/test_class_methods.pairs.warp:15 |
| 157 | call hex | 4 | tests/types/test_one_dispatch.pairs.warp:32, tests/types/test_text_of_text.pairs.warp:10 |
| 158 | a function value in as | 4 | tests/types/test_printable.warp:4, tests/types/test_printable.warp:7 |
| 159 | call byte | 4 | tests/types/test_type_calls.pairs.warp:6, tests/types/test_type_calls.pairs.warp:12 |
| 160 | a test of the type Color, which has cases or subclasses | 4 | tests/types/test_variant_payloads.pairs.warp:15, tests/types/test_variant_payloads.pairs.warp:16 |
| 161 | call zoned | 4 | tests/welcoming/test_footguns.warp:14, tests/welcoming/test_footguns.warp:15 |
| 162 | n is a main-level variable: declare it `global n` to change it from a function, or use a n | 3 | tests/control/test_event_footguns.warp:16, tests/control/test_event_footguns.warp:18 |
| 163 | a test of the type function | 3 | tests/control/test_function_objects.warp:72, tests/programs/test_sound_tempo.warp:20 |
| 164 | the operator to with nothing after it | 3 | tests/control/test_match_item_patterns.warp:10, tests/functions/test_variadic.pairs.warp:20 |
| 165 | k is a main-level variable: declare it `global k` to change it from a function, or use a n | 3 | tests/control/test_nested_block_calls.warp:4, tests/control/test_nested_block_calls.warp:8 |
| 166 | a int where a list is needed | 3 | tests/control/test_object_blocks.pairs.warp:4, tests/control/test_object_blocks.pairs.warp:5 |
| 167 | call data | 3 | tests/control/test_quasi_quote.pairs.warp:6, tests/control/test_quasi_quote.pairs.warp:12 |
| 168 | call write | 3 | tests/control/test_try_host_failure.pairs.warp:7, tests/scope/test_parameter_shadows_function.warp:4 |
| 169 | call count_up | 3 | tests/ffi/test_component_short_use.pairs.warp:4, tests/modules/test_wasm_modules.pairs.warp:4 |
| 170 | call atol | 3 | tests/ffi/test_ffi.warp:123, tests/ffi/test_ffi.warp:125 |
| 171 | call toupper | 3 | tests/ffi/test_ffi_gaps.pairs.warp:10, tests/ffi/test_libc_results.warp:30 |
| 172 | call strtol | 3 | tests/ffi/test_ffi_text_out_pointers.warp:6, tests/modules/test_wasm_modules.pairs.warp:34 |
| 173 | call Fraction | 3 | tests/ffi/test_foreign_handles.pairs.warp:12, tests/ffi/test_foreign_operators.pairs.warp:4 |
| 174 | call deque | 3 | tests/ffi/test_foreign_operators.pairs.warp:7, tests/ffi/test_foreign_operators.pairs.warp:8 |
| 175 | call from | 3 | tests/ffi/test_foreign_operators.pairs.warp:11, tests/ffi/test_foreign_operators.pairs.warp:12 |
| 176 | operator * on bool and bool | 3 | tests/functions/test_braceless_call_in_logic.pairs.warp:4, tests/functions/test_braceless_call_in_logic.pairs.warp:5 |
| 177 | a test of the type callable | 3 | tests/functions/test_callable_parameter.pairs.warp:4, tests/functions/test_callable_parameter.pairs.warp:5 |
| 178 | a variable declared hi | 3 | tests/functions/test_definition_bodies.pairs.warp:15, tests/functions/test_definition_bodies.warp:24 |
| 179 | call answer | 3 | tests/functions/test_field_closure_call.warp:4, tests/functions/test_field_closure_call.warp:7 |
| 180 | a call of c with N arguments | 3 | tests/functions/test_foreign_spellings.pairs.warp:24, tests/functions/test_foreign_spellings.pairs.warp:25 |
| 181 | the operator + with nothing after it | 3 | tests/functions/test_function_values.pairs.warp:22, tests/functions/test_function_values.pairs.warp:24 |
| 182 | call any | 3 | tests/functions/test_generator_arguments.pairs.warp:6, tests/functions/test_generator_arguments.pairs.warp:10 |
| 183 | call d | 3 | tests/functions/test_lambda_held_by_field.warp:6, tests/functions/test_lambda_held_by_field.warp:19 |
| 184 | a parameter of kind [int] | 3 | tests/functions/test_parameter_overloads.pairs.warp:14, tests/welcoming/test_welcoming_listparams.pairs.warp:31 |
| 185 | the field pop of a list | 3 | tests/lists/test_pop.pairs.warp:12, tests/lists/test_pop.pairs.warp:13 |
| 186 | an assignment to the call a(…) | 3 | tests/lists/test_unpacking.pairs.warp:28, tests/types/test_class_destructuring.pairs.warp:9 |
| 187 | call width | 3 | tests/modules/test_import_compiled.warp:10, tests/programs/test_property_definitions.warp:21 |
| 188 | call json | 3 | tests/modules/test_json5.pairs.warp:4, tests/modules/test_json5.pairs.warp:5 |
| 189 | call to_hex | 3 | tests/modules/test_package_registry.pairs.warp:7, tests/modules/test_package_registry.pairs.warp:10 |
| 190 | the field split of a text | 3 | tests/modules/test_runtime_word_lines.pairs.warp:4, tests/text/test_split_default.pairs.warp:4 |
| 191 | a call of mix with N arguments | 3 | tests/modules/test_std_shadowed_names.warp:24, tests/modules/test_std_shadowed_names.warp:30 |
| 192 | a call of unique with N arguments | 3 | tests/modules/test_use_modules.warp:4, tests/numbers/test_litre_hectare.warp:8 |
| 193 | a parameter of kind real | 3 | tests/numbers/test_exact_parameters.pairs.warp:4, tests/numbers/test_float_exact_context.pairs.warp:20 |
| 194 | a variable declared i64 | 3 | tests/numbers/test_fast_int.pairs.warp:8, tests/numbers/test_fast_int.pairs.warp:12 |
| 195 | call type | 3 | tests/numbers/test_unit_definitions.pairs.warp:18, tests/numbers/test_unit_definitions.pairs.warp:19 |
| 196 | operator == on int and bool | 3 | tests/operators/test_equality_never_chains.pairs.warp:7, tests/types/test_bool_type.pairs.warp:11 |
| 197 | operator < on bool and int | 3 | tests/operators/test_equality_never_chains.pairs.warp:10, tests/operators/test_equality_never_chains.pairs.warp:12 |
| 198 | operator > on bool and int | 3 | tests/operators/test_equality_never_chains.pairs.warp:11, tests/types/test_generic_classes.pairs.warp:10 |
| 199 | a function value in \| | 3 | tests/operators/test_pipe_stages.pairs.warp:16, tests/operators/test_pipe_stages.pairs.warp:19 |
| 200 | operator ~ on text and text | 3 | tests/operators/test_rough_similarity.pairs.warp:13, tests/operators/test_rough_similarity.pairs.warp:15 |
| 201 | operator then on bool and int | 3 | tests/operators/test_then_pipe.pairs.warp:9, tests/wasm/test_wasm.pairs.warp:33 |
| 202 | the operator ± on int | 3 | tests/parser/test_if_condition_words.pairs.warp:4, tests/parser/test_if_condition_words.pairs.warp:5 |
| 203 | call format! | 3 | tests/parser/test_rust_macros.pairs.warp:4, tests/parser/test_rust_macros.pairs.warp:5 |
| 204 | cannot access length code of float | 3 | tests/programs/test_music_values.warp:24, tests/programs/test_music_values.warp:26 |
| 205 | the field string of a list | 3 | tests/text/test_chars_codepoints.pairs.warp:10, tests/text/test_chars_codepoints.pairs.warp:11 |
| 206 | call String | 3 | tests/text/test_text_type_words.pairs.warp:8, tests/text/test_text_type_words.warp:5 |
| 207 | call number | 3 | tests/types/test_any_field_text.pairs.warp:4, tests/types/test_any_field_text.pairs.warp:5 |
| 208 | a int given to a variable declared bool | 3 | tests/types/test_any_into_annotated_types.pairs.warp:8, tests/types/test_bool_assign.pairs.warp:11 |
| 209 | a char where a bool is needed | 3 | tests/types/test_any_place.pairs.warp:10, tests/types/test_types.pairs.warp:23 |
| 210 | a list for a typed field of Holder | 3 | tests/types/test_class_cases_ported.warp:35, tests/types/test_class_cases_ported.warp:36 |
| 211 | call __init__ | 3 | tests/types/test_class_forms_ported.warp:50, tests/types/test_class_operators.warp:10 |
| 212 | a test of the type operation | 3 | tests/types/test_code_type.pairs.warp:15, tests/types/test_operation_type.pairs.warp:14 |
| 213 | operator * on bool and int | 3 | tests/types/test_nested_type_calls.pairs.warp:18, tests/welcoming/test_footguns.pairs.warp:61 |
| 214 | an assignment to a number | 3 | tests/wasm/test_wasm.pairs.warp:461, tests/wasm/test_wasm.pairs.warp:462 |
| 215 | the field product of a list | 3 | tests/welcoming/test_dot_without_parens.pairs.warp:4, tests/welcoming/test_dot_word_same_path.pairs.warp:6 |
| 216 | call range | 3 | tests/welcoming/test_welcoming_rangeblock.pairs.warp:18, tests/welcoming/test_welcoming_rangeblock.pairs.warp:19 |
| 217 | the field s of a int | 2 | tests/control/test_charged_fields.pairs.warp:6, tests/functions/test_charged_getters.pairs.warp:10 |
| 218 | a parameter of kind [int or text] | 2 | tests/control/test_error_not_in_data.pairs.warp:21, tests/control/test_error_not_in_data.pairs.warp:22 |
| 219 | a parameter of kind [int or error] | 2 | tests/control/test_error_not_in_data.warp:10, tests/control/test_error_not_in_data.warp:13 |
| 220 | the operator >= with nothing before it | 2 | tests/control/test_match_deep_conditions.warp:13, tests/control/test_match_deep_conditions.warp:14 |
| 221 | operator else on text and bool | 2 | tests/control/test_match_else_line.warp:4, tests/control/test_match_else_line.warp:9 |
| 222 | call fields | 2 | tests/control/test_reflection_synonyms.pairs.warp:7, tests/types/test_empty_reflection.pairs.warp:4 |
| 223 | call switch | 2 | tests/control/test_switch_match.pairs.warp:31, tests/control/test_switch_value.pairs.warp:7 |
| 224 | call big | 2 | tests/control/test_table_method_reads.warp:4, tests/control/test_table_method_reads.warp:7 |
| 225 | a break out of a try | 2 | tests/control/test_try_exits_and_naming.pairs.warp:7, tests/control/test_try_exits_and_naming.pairs.warp:13 |
| 226 | the field methods of a list | 2 | tests/control/test_value_methods_are_functions.pairs.warp:6, tests/modules/test_dir_builtin_members.warp:4 |
| 227 | call to_string | 2 | tests/ffi/test_cargo_crates.pairs.warp:20, tests/parser/test_rust_paths.pairs.warp:23 |
| 228 | call atof | 2 | tests/ffi/test_ffi.warp:20, tests/ffi/test_ffi.warp:22 |
| 229 | call strncmp | 2 | tests/ffi/test_ffi.warp:52, tests/ffi/test_ffi.warp:54 |
| 230 | call fmod | 2 | tests/ffi/test_ffi.warp:107, tests/ffi/test_ffi.warp:109 |
| 231 | call adler32 | 2 | tests/ffi/test_ffi_gaps.pairs.warp:7, tests/modules/test_wasm_modules.pairs.warp:41 |
| 232 | operator >= on int and char | 2 | tests/ffi/test_ffi_gaps.pairs.warp:12, tests/ffi/test_libc_results.warp:32 |
| 233 | call sqlite3_db_handle | 2 | tests/ffi/test_ffi_gaps.pairs.warp:21, tests/ffi/test_ffi_handles.pairs.warp:4 |
| 234 | call pow | 2 | tests/ffi/test_ffi_signatures.warp:21, tests/functions/test_call_arity.pairs.warp:6 |
| 235 | a call of joined with N arguments | 2 | tests/ffi/test_foreign_js.pairs.warp:10, tests/ffi/test_foreign_python.pairs.warp:7 |
| 236 | call isclose | 2 | tests/ffi/test_foreign_type_tests.pairs.warp:8, tests/ffi/test_foreign_type_tests.pairs.warp:12 |
| 237 | call resolve | 2 | tests/ffi/test_js_callbacks.pairs.warp:4, tests/ffi/test_js_callbacks.pairs.warp:7 |
| 238 | call strstr | 2 | tests/ffi/test_libc_results.warp:26, tests/modules/test_wasm_modules.pairs.warp:27 |
| 239 | call strrchr | 2 | tests/ffi/test_libc_results.warp:28, tests/modules/test_wasm_modules.pairs.warp:28 |
| 240 | call strchr | 2 | tests/ffi/test_libc_results.warp:38, tests/modules/test_wasm_modules.pairs.warp:31 |
| 241 | call exp2 | 2 | tests/ffi/test_libm_header_functions.pairs.warp:4, tests/ffi/test_libm_header_functions.pairs.warp:5 |
| 242 | call ldexp | 2 | tests/ffi/test_libm_header_functions.pairs.warp:9, tests/ffi/test_libm_header_functions.pairs.warp:10 |
| 243 | call atob | 2 | tests/ffi/test_web_idl.pairs.warp:23, tests/ffi/test_web_idl.pairs.warp:27 |
| 244 | call arcsin | 2 | tests/functions/test_alias_definition.pairs.warp:4, tests/functions/test_library_word_aliases.pairs.warp:12 |
| 245 | call h | 2 | tests/functions/test_alias_definition.pairs.warp:16, tests/types/test_class_cases_ported.warp:13 |
| 246 | call arctan | 2 | tests/functions/test_alias_definition.warp:4, tests/functions/test_library_word_aliases.pairs.warp:11 |
| 247 | call move | 2 | tests/functions/test_argument_labels.pairs.warp:4, tests/functions/test_argument_labels.warp:5 |
| 248 | wrong number of arguments | 2 | tests/functions/test_arrow_defaults_and_types.pairs.warp:4, tests/functions/test_foreign_spellings.pairs.warp:27 |
| 249 | a default of f reading a | 2 | tests/functions/test_call_arity.pairs.warp:9, tests/functions/test_default_parameters.pairs.warp:9 |
| 250 | call Add | 2 | tests/functions/test_csharp_functions.pairs.warp:8, tests/functions/test_csharp_functions.pairs.warp:9 |
| 251 | operator > on list and int | 2 | tests/functions/test_function_words_consume.pairs.warp:15, tests/functions/test_function_words_consume.pairs.warp:17 |
| 252 | call square | 2 | tests/functions/test_functions.pairs.warp:43, tests/modules/test_use_modules.warp:6 |
| 253 | operator - on int and list | 2 | tests/functions/test_kind_tested_parameter.warp:4, tests/functions/test_kind_tested_parameter.warp:5 |
| 254 | call fold | 2 | tests/functions/test_lambda_spellings.pairs.warp:9, tests/functions/test_swift_closures.pairs.warp:9 |
| 255 | an assignment to the call inner(…) | 2 | tests/functions/test_late_binding.pairs.warp:22, tests/functions/test_nonlocal_passed_around.pairs.warp:4 |
| 256 | a text where a list is needed | 2 | tests/functions/test_method_words.pairs.warp:19, tests/functions/test_method_words.pairs.warp:20 |
| 257 | a call of square with N arguments | 2 | tests/functions/test_nested_prefix_calls.pairs.warp:5, tests/types/test_one_dispatch.warp:29 |
| 258 | call abs | 2 | tests/functions/test_operator_word_methods.pairs.warp:6, tests/numbers/test_number_methods.pairs.warp:11 |
| 259 | could not run the program: failed to compile: wasm[N]::function[N]::total: WebAssembly tra | 2 | tests/functions/test_parameter_any.warp:7, tests/functions/test_parameter_any.warp:8 |
| 260 | a parameter of kind [float] | 2 | tests/functions/test_parameter_overloads.warp:25, tests/types/test_typed_list_elements.warp:6 |
| 261 | a list where a int is needed | 2 | tests/functions/test_parameter_retype.pairs.warp:6, tests/types/test_checked_call_kinds.pairs.warp:9 |
| 262 | a parameter of kind {name, title?} | 2 | tests/functions/test_parameter_shapes.pairs.warp:4, tests/functions/test_parameter_shapes.warp:4 |
| 263 | a parameter of kind {name?, phone} | 2 | tests/functions/test_parameter_shapes.pairs.warp:8, tests/functions/test_parameter_shapes.warp:16 |
| 264 | a parameter of kind {phone-number:text} | 2 | tests/functions/test_parameter_shapes.pairs.warp:14, tests/functions/test_parameter_shapes.warp:25 |
| 265 | operator ? on node and char | 2 | tests/functions/test_property_assign.pairs.warp:23, tests/functions/test_property_assign.pairs.warp:24 |
| 266 | call half | 2 | tests/functions/test_return_type_forms.pairs.warp:8, tests/types/test_static_result.pairs.warp:4 |
| 267 | an assignment to the call inc(…) | 2 | tests/functions/test_returned_function_names.pairs.warp:4, tests/functions/test_returned_function_names.pairs.warp:5 |
| 268 | operator . on list and int | 2 | tests/functions/test_rust_references.pairs.warp:16, tests/lists/test_numbered_fields.pairs.warp:6 |
| 269 | a call of add with N arguments | 2 | tests/functions/test_user_method_form.pairs.warp:8, tests/types/test_dispatch.warp:11 |
| 270 | a call of count with N arguments | 2 | tests/functions/test_variadic.pairs.warp:7, tests/text/test_count_method.pairs.warp:12 |
| 271 | a call of apply with N arguments | 2 | tests/functions/test_variadic.pairs.warp:33, tests/functions/test_variadic.warp:4 |
| 272 | a list for a typed field of Stack | 2 | tests/lists/test_comprehension_beside_own_push.warp:4, tests/lists/test_comprehension_beside_own_push.warp:19 |
| 273 | floor of a list | 2 | tests/lists/test_element_wise_vectors.pairs.warp:19, tests/lists/test_element_wise_vectors.pairs.warp:38 |
| 274 | operator !== on list and list | 2 | tests/lists/test_list_identity.pairs.warp:6, tests/types/test_passed_list_identity.pairs.warp:8 |
| 275 | call sort_by | 2 | tests/lists/test_sort_long.warp:4, tests/lists/test_sort_long.warp:7 |
| 276 | the field x of a list | 2 | tests/lists/test_swizzles.pairs.warp:4, tests/lists/test_swizzles.pairs.warp:7 |
| 277 | the field y of a list | 2 | tests/lists/test_swizzles.pairs.warp:5, tests/lists/test_swizzles.pairs.warp:8 |
| 278 | an assignment to the group [… | 2 | tests/lists/test_unpacking.pairs.warp:25, tests/welcoming/test_let_const_changes.pairs.warp:9 |
| 279 | call P | 2 | tests/modules/test_import_compiled.warp:8, tests/modules/test_import_compiled.warp:16 |
| 280 | the field json of a list | 2 | tests/modules/test_node_json.pairs.warp:6, tests/modules/test_node_json.pairs.warp:25 |
| 281 | call words | 2 | tests/modules/test_std_math_text.pairs.warp:76, tests/modules/test_std_math_text.pairs.warp:77 |
| 282 | call acos | 2 | tests/modules/test_std_words.pairs.warp:4, tests/numbers/test_libm_functions.pairs.warp:7 |
| 283 | call cosh | 2 | tests/modules/test_std_words.pairs.warp:6, tests/numbers/test_libm_functions.pairs.warp:8 |
| 284 | call XXH32 | 2 | tests/modules/test_wasm_modules.pairs.warp:49, tests/modules/test_wasm_modules.pairs.warp:50 |
| 285 | the field @comment of a int | 2 | tests/node/test_comment_meta.warp:16, tests/node/test_comment_meta.warp:36 |
| 286 | operator else on bool and int | 2 | tests/numbers/test_angle.pairs.warp:34, tests/numbers/test_angle.pairs.warp:158 |
| 287 | a test of the type duration | 2 | tests/numbers/test_dimension_types.pairs.warp:12, tests/types/test_time_kinds_are_types.pairs.warp:7 |
| 288 | a variable declared int64 | 2 | tests/numbers/test_fast_int.pairs.warp:11, tests/numbers/test_fixed_width_ints.pairs.warp:21 |
| 289 | operator ≈ on text and text | 2 | tests/operators/test_approximately.pairs.warp:15, tests/operators/test_rough_similarity.pairs.warp:14 |
| 290 | operator ≈ on bool and int | 2 | tests/operators/test_approximately.pairs.warp:19, tests/operators/test_approximately.pairs.warp:20 |
| 291 | a function value in or | 2 | tests/operators/test_bar_pipe.pairs.warp:5, tests/operators/test_bar_pipe.pairs.warp:10 |
| 292 | operator == on bool and int | 2 | tests/operators/test_equality_never_chains.pairs.warp:4, tests/operators/test_equality_never_chains.pairs.warp:5 |
| 293 | cannot access length code of char | 2 | tests/operators/test_library_infix_words.pairs.warp:19, tests/text/test_unicode_escape.pairs.warp:6 |
| 294 | a function value in then | 2 | tests/operators/test_then_pipe.pairs.warp:6, tests/operators/test_then_pipe.pairs.warp:18 |
| 295 | call collect | 2 | tests/parser/test_rust_paths.pairs.warp:5, tests/parser/test_rust_paths.pairs.warp:6 |
| 296 | call canvas | 2 | tests/programs/test_paint_shader.warp:4, tests/programs/test_window_size.warp:23 |
| 297 | operator * on int and text | 2 | tests/text/test_repeat_typed.pairs.warp:5, tests/text/test_text_repeat.pairs.warp:5 |
| 298 | a call of split_text with N arguments | 2 | tests/text/test_split_default.pairs.warp:5, tests/text/test_split_default.pairs.warp:7 |
| 299 | a bool given to a variable declared int | 2 | tests/types/test_any_into_annotated.pairs.warp:8, tests/types/test_bool_assign.pairs.warp:8 |
| 300 | an assignment to `{x …` | 2 | tests/types/test_class_destructuring.pairs.warp:4, tests/types/test_class_destructuring.pairs.warp:6 |
| 301 | an assignment to the group {… | 2 | tests/types/test_class_destructuring.warp:4, tests/types/test_class_destructuring.warp:16 |
| 302 | call scaled | 2 | tests/types/test_class_methods.warp:16, tests/types/test_class_methods.warp:28 |
| 303 | a parameter of kind ø | 2 | tests/types/test_empty_type_aliases.pairs.warp:12, tests/types/test_empty_type_aliases.pairs.warp:13 |
| 304 | a test of the type Shape, which has cases or subclasses | 2 | tests/types/test_enum_cases.pairs.warp:6, tests/types/test_enum_cases.pairs.warp:7 |
| 305 | a list for a typed field of G | 2 | tests/types/test_field_named_like_builtin.warp:19, tests/types/test_field_named_like_builtin.warp:23 |
| 306 | a parameter of kind int or text | 2 | tests/types/test_inline_unions.pairs.warp:14, tests/types/test_inline_unions.pairs.warp:15 |
| 307 | a test of the type instant | 2 | tests/types/test_time_kinds_are_types.pairs.warp:5, tests/types/test_time_kinds_are_types.pairs.warp:10 |
| 308 | a test of the type date | 2 | tests/types/test_time_kinds_are_types.pairs.warp:6, tests/types/test_time_kinds_are_types.pairs.warp:9 |
| 309 | a call of smallest with N arguments | 2 | tests/types/test_trait_constraints.warp:4, tests/types/test_trait_constraints.warp:5 |
| 310 | call char | 2 | tests/types/test_types.pairs.warp:36, tests/welcoming/test_footguns.pairs.warp:187 |
| 311 | an assignment to `… or …` | 2 | tests/types/test_union_returns.pairs.warp:11, tests/types/test_union_returns.pairs.warp:14 |
| 312 | an instance of Person not giving each field | 2 | tests/types/test_value_constructor.pairs.warp:4, tests/types/test_value_constructor.pairs.warp:5 |
| 313 | the operator ^= on int | 2 | tests/wasm/test_wasm.pairs.warp:234, tests/wasm/test_wasm.pairs.warp:360 |
| 314 | the field capitalize of a text | 2 | tests/welcoming/test_dot_without_parens.pairs.warp:6, tests/welcoming/test_dot_word_same_path.pairs.warp:4 |
| 315 | a break outside a loop | 2 | tests/welcoming/test_welcoming_indent.pairs.warp:4, tests/welcoming/test_welcoming_indent.warp:42 |
| 316 | call get | 2 | tests/welcoming/test_welcoming_listparams.pairs.warp:9, tests/welcoming/test_welcoming_listparams.pairs.warp:33 |
| 317 | call is! | 1 | tests/control/test_bang_words.pairs.warp:11 |
| 318 | a node for a typed field of pt | 1 | tests/control/test_error_not_in_data.warp:6 |
| 319 | divide by zero: a field holds no error; handle it with try … else, or declare `or error` | 1 | tests/control/test_error_not_in_data.warp:9 |
| 320 | a number += a text | 1 | tests/control/test_for_unit_words.pairs.warp:11 |
| 321 | the field methods of a text | 1 | tests/control/test_function_objects.warp:78 |
| 322 | the field returns of a float | 1 | tests/control/test_function_value_words.pairs.warp:5 |
| 323 | the field signature of a float | 1 | tests/control/test_library_signatures.pairs.warp:4 |
| 324 | call properties | 1 | tests/control/test_reflection_synonyms.pairs.warp:4 |
| 325 | the field doc of a int | 1 | tests/control/test_reflection_words.warp:29 |
| 326 | the field listeners of a int | 1 | tests/control/test_signal_reflection.pairs.warp:4 |
| 327 | a call of exit with N arguments | 1 | tests/control/test_system_signals.pairs.warp:4 |
| 328 | call try·else | 1 | tests/control/test_try_catch_except.warp:8 |
| 329 | a return out of a try | 1 | tests/control/test_try_exits_and_naming.pairs.warp:4 |
| 330 | a continue out of a try | 1 | tests/control/test_try_exits_and_naming.pairs.warp:10 |
| 331 | the field a of a int | 1 | tests/control/test_try_named_traps.pairs.warp:7 |
| 332 | call even | 1 | tests/control/test_type_word_filter_loops.pairs.warp:11 |
| 333 | call odd | 1 | tests/control/test_type_word_filter_loops.warp:8 |
| 334 | call test | 1 | tests/control/test_variable_signals.pairs.warp:18 |
| 335 | call save | 1 | tests/control/test_variable_signals.warp:14 |
| 336 | call levenshtein | 1 | tests/ffi/test_cargo_crates.pairs.warp:4 |
| 337 | call hamming | 1 | tests/ffi/test_cargo_crates.pairs.warp:5 |
| 338 | call jaro_winkler | 1 | tests/ffi/test_cargo_crates.pairs.warp:6 |
| 339 | call escape | 1 | tests/ffi/test_cargo_crates.pairs.warp:9 |
| 340 | call encode | 1 | tests/ffi/test_cargo_crates.pairs.warp:15 |
| 341 | call hash | 1 | tests/ffi/test_cargo_crates.pairs.warp:16 |
| 342 | call counter | 1 | tests/ffi/test_component_short_use.warp:6 |
| 343 | call tan | 1 | tests/ffi/test_ffi.warp:86 |
| 344 | call tolower | 1 | tests/ffi/test_ffi_gaps.pairs.warp:11 |
| 345 | call fopen | 1 | tests/ffi/test_ffi_handles.pairs.warp:5 |
| 346 | call srand | 1 | tests/ffi/test_ffi_signatures.warp:7 |
| 347 | call sqrtf | 1 | tests/ffi/test_ffi_signatures.warp:15 |
| 348 | call fabsf | 1 | tests/ffi/test_ffi_signatures.warp:17 |
| 349 | call ldexpf | 1 | tests/ffi/test_ffi_signatures.warp:23 |
| 350 | call SetTraceLogLevel | 1 | tests/ffi/test_ffi_signatures.warp:35 |
| 351 | call IsKeyPressed | 1 | tests/ffi/test_ffi_signatures.warp:38 |
| 352 | call SetMouseOffset | 1 | tests/ffi/test_ffi_signatures.warp:40 |
| 353 | call GetRandomValue | 1 | tests/ffi/test_ffi_signatures.warp:43 |
| 354 | call TextToInteger | 1 | tests/ffi/test_ffi_signatures.warp:45 |
| 355 | call strtod | 1 | tests/ffi/test_ffi_text_out_pointers.warp:4 |
| 356 | call sqlite3_errstr | 1 | tests/ffi/test_ffi_text_results.pairs.warp:7 |
| 357 | call NumberFormat | 1 | tests/ffi/test_foreign_handles.pairs.warp:4 |
| 358 | call Path | 1 | tests/ffi/test_foreign_handles.pairs.warp:8 |
| 359 | call isoformat | 1 | tests/ffi/test_foreign_handles.pairs.warp:9 |
| 360 | call Decimal | 1 | tests/ffi/test_foreign_handles.pairs.warp:13 |
| 361 | call expm1 | 1 | tests/ffi/test_foreign_lines.pairs.warp:8 |
| 362 | call erf | 1 | tests/ffi/test_foreign_lines.warp:4 |
| 363 | call SimpleNamespace | 1 | tests/ffi/test_foreign_member_assignment.warp:7 |
| 364 | a call of mean with N arguments | 1 | tests/ffi/test_foreign_python.pairs.warp:6 |
| 365 | call factorial | 1 | tests/ffi/test_foreign_python.pairs.warp:10 |
| 366 | call strcspn | 1 | tests/ffi/test_libc_results.warp:36 |
| 367 | call putl | 1 | tests/ffi/test_wasi.pairs.warp:8 |
| 368 | call putf | 1 | tests/ffi/test_wasi.pairs.warp:11 |
| 369 | call assert | 1 | tests/ffi/test_web_idl.pairs.warp:5 |
| 370 | call now | 1 | tests/ffi/test_web_idl.pairs.warp:6 |
| 371 | call btoa | 1 | tests/ffi/test_web_idl.pairs.warp:24 |
| 372 | call URL | 1 | tests/ffi/test_web_idl.pairs.warp:30 |
| 373 | call URLSearchParams | 1 | tests/ffi/test_web_idl.pairs.warp:31 |
| 374 | call Blob | 1 | tests/ffi/test_web_idl.pairs.warp:32 |
| 375 | call Date | 1 | tests/ffi/test_web_idl.pairs.warp:33 |
| 376 | call OffscreenCanvas | 1 | tests/ffi/test_web_idl.warp:4 |
| 377 | call arccos | 1 | tests/functions/test_alias_definition.pairs.warp:5 |
| 378 | `]` closes the group opened which needs `)` | 1 | tests/functions/test_anonymous_application.pairs.warp:8 |
| 379 | the field f of a int | 1 | tests/functions/test_arity_overloads.pairs.warp:19 |
| 380 | a default of g reading x | 1 | tests/functions/test_call_arity.pairs.warp:11 |
| 381 | a parameter of kind invocable | 1 | tests/functions/test_callable_parameter.warp:23 |
| 382 | a parameter of kind runnable | 1 | tests/functions/test_callable_parameter.warp:25 |
| 383 | operator * on int and char | 1 | tests/functions/test_closure_value_arguments.pairs.warp:9 |
| 384 | the field string of a int | 1 | tests/functions/test_conversion_methods.pairs.warp:4 |
| 385 | the field int of a text | 1 | tests/functions/test_conversion_methods.pairs.warp:5 |
| 386 | the field float of a int | 1 | tests/functions/test_conversion_methods.pairs.warp:6 |
| 387 | call Half | 1 | tests/functions/test_csharp_functions.pairs.warp:10 |
| 388 | a parameter of kind data | 1 | tests/functions/test_data_parameter.warp:27 |
| 389 | a default of area reading w | 1 | tests/functions/test_default_parameters.pairs.warp:18 |
| 390 | a parameter of kind N | 1 | tests/functions/test_default_parameters.pairs.warp:34 |
| 391 | a call of greet with N arguments | 1 | tests/functions/test_default_parameters.warp:10 |
| 392 | the field forEach of a list | 1 | tests/functions/test_each_spellings.pairs.warp:4 |
| 393 | the field for_each of a list | 1 | tests/functions/test_each_spellings.pairs.warp:6 |
| 394 | the field foreach of a list | 1 | tests/functions/test_each_spellings.pairs.warp:7 |
| 395 | call Where | 1 | tests/functions/test_foreign_spellings.pairs.warp:4 |
| 396 | the field to_i of a text | 1 | tests/functions/test_foreign_spellings.pairs.warp:5 |
| 397 | the field sumOf of a list | 1 | tests/functions/test_foreign_spellings.pairs.warp:7 |
| 398 | call Count | 1 | tests/functions/test_foreign_spellings.pairs.warp:11 |
| 399 | a call of sum with N arguments | 1 | tests/functions/test_foreign_spellings.pairs.warp:16 |
| 400 | the operator * with nothing after it | 1 | tests/functions/test_function_values.pairs.warp:23 |
| 401 | call all | 1 | tests/functions/test_generator_arguments.pairs.warp:11 |
| 402 | call println | 1 | tests/functions/test_library_word_aliases.pairs.warp:4 |
| 403 | the field insert of a list | 1 | tests/functions/test_member_phrases.pairs.warp:4 |
| 404 | the field contains of a list | 1 | tests/functions/test_member_phrases.pairs.warp:5 |
| 405 | the field replace of a text | 1 | tests/functions/test_member_phrases.pairs.warp:6 |
| 406 | call norm | 1 | tests/functions/test_operator_word_methods.pairs.warp:4 |
| 407 | call cbrt | 1 | tests/functions/test_operator_word_methods.pairs.warp:7 |
| 408 | block is a main-level variable: declare it `global block` to change it from a function, or | 1 | tests/functions/test_overloaded_function_values.warp:19 |
| 409 | a char where a int is needed | 1 | tests/functions/test_parameter_retype.pairs.warp:4 |
| 410 | a text where a int is needed | 1 | tests/functions/test_parameter_retype.pairs.warp:5 |
| 411 | a parameter of kind {name} | 1 | tests/functions/test_parameter_shapes.pairs.warp:5 |
| 412 | a parameter of kind {name:text} | 1 | tests/functions/test_parameter_shapes.pairs.warp:11 |
| 413 | a parameter of kind {name?, phone text, email?, mutable status} | 1 | tests/functions/test_parameter_shapes.warp:10 |
| 414 | a parameter of kind {name?, phone number} | 1 | tests/functions/test_parameter_shapes.warp:21 |
| 415 | a test of the type \"…" '' \"…" | 1 | tests/functions/test_pattern_clauses.pairs.warp:11 |
| 416 | a call of g with N arguments | 1 | tests/functions/test_preposition_calls.warp:55 |
| 417 | operator < on list and int | 1 | tests/functions/test_random_dispatch.pairs.warp:23 |
| 418 | a parameter of kind Int | 1 | tests/functions/test_return_type_forms.pairs.warp:12 |
| 419 | an assignment to the call named(…) | 1 | tests/functions/test_tuple_returns.pairs.warp:7 |
| 420 | a call of rest with N arguments | 1 | tests/functions/test_variadic.pairs.warp:8 |
| 421 | call first | 1 | tests/functions/test_variadic.pairs.warp:9 |
| 422 | call xs | 1 | tests/functions/test_words_in_phrases.pairs.warp:27 |
| 423 | operator + on float and list | 1 | tests/lists/test_element_wise_lists.pairs.warp:15 |
| 424 | a list where a float is needed | 1 | tests/lists/test_element_wise_vectors.pairs.warp:31 |
| 425 | a variable declared map of int, int | 1 | tests/lists/test_number_keys.pairs.warp:12 |
| 426 | operator : on text and node | 1 | tests/lists/test_pair_values.warp:4 |
| 427 | the field clear of a list | 1 | tests/lists/test_pop.pairs.warp:14 |
| 428 | call list | 1 | tests/lists/test_python_collections.pairs.warp:8 |
| 429 | a call of fromkeys with N arguments | 1 | tests/lists/test_python_collections.pairs.warp:20 |
| 430 | a function value in # | 1 | tests/lists/test_size_count.pairs.warp:15 |
| 431 | the field w of a list | 1 | tests/lists/test_swizzles.pairs.warp:6 |
| 432 | the field zy of a list | 1 | tests/lists/test_swizzles.pairs.warp:11 |
| 433 | the field xy of a list | 1 | tests/lists/test_swizzles.pairs.warp:12 |
| 434 | the field wzyx of a list | 1 | tests/lists/test_swizzles.pairs.warp:13 |
| 435 | the field xyz of a list | 1 | tests/lists/test_swizzles.pairs.warp:14 |
| 436 | the field yx of a list | 1 | tests/lists/test_swizzles.pairs.warp:15 |
| 437 | the field zyx of a list | 1 | tests/lists/test_swizzles.pairs.warp:16 |
| 438 | the field xyy of a list | 1 | tests/lists/test_swizzles.pairs.warp:17 |
| 439 | operator : on char and int | 1 | tests/modules/test_json5.pairs.warp:12 |
| 440 | call netbase_path | 1 | tests/modules/test_netbase_package.warp:4 |
| 441 | the field json of a text | 1 | tests/modules/test_node_json.pairs.warp:7 |
| 442 | the field json of a int | 1 | tests/modules/test_node_json.pairs.warp:8 |
| 443 | operator : on int and char | 1 | tests/modules/test_node_json.pairs.warp:20 |
| 444 | call instance_copy | 1 | tests/modules/test_runtime_word_lines.pairs.warp:9 |
| 445 | call rainbow | 1 | tests/modules/test_selective_package.pairs.warp:7 |
| 446 | call loads | 1 | tests/modules/test_std_aliases.pairs.warp:5 |
| 447 | call dumps | 1 | tests/modules/test_std_aliases.pairs.warp:7 |
| 448 | call sub | 1 | tests/modules/test_std_aliases.pairs.warp:8 |
| 449 | call exists | 1 | tests/modules/test_std_file.pairs.warp:7 |
| 450 | call zeros | 1 | tests/modules/test_std_matrix.pairs.warp:15 |
| 451 | call eye | 1 | tests/modules/test_std_matrix.pairs.warp:16 |
| 452 | call matmul | 1 | tests/modules/test_std_matrix.pairs.warp:17 |
| 453 | a call of zip with N arguments | 1 | tests/modules/test_std_qualified.pairs.warp:4 |
| 454 | a call of gcd with N arguments | 1 | tests/modules/test_std_qualified.pairs.warp:5 |
| 455 | a call of repeat with N arguments | 1 | tests/modules/test_std_qualified.pairs.warp:6 |
| 456 | the operator / with nothing after it | 1 | tests/modules/test_std_regex.pairs.warp:10 |
| 457 | operator # on char and int | 1 | tests/modules/test_std_regex.pairs.warp:13 |
| 458 | a call of clamp with N arguments | 1 | tests/modules/test_std_shadowed_names.warp:27 |
| 459 | call sinh | 1 | tests/modules/test_std_words.pairs.warp:5 |
| 460 | call tanh | 1 | tests/modules/test_std_words.pairs.warp:7 |
| 461 | call count_twice | 1 | tests/modules/test_wasm_modules.pairs.warp:17 |
| 462 | call XXH64 | 1 | tests/modules/test_wasm_modules.pairs.warp:51 |
| 463 | call uncompress | 1 | tests/modules/test_wasm_modules.warp:7 |
| 464 | a parameter of kind exact | 1 | tests/numbers/test_exact_parameters.pairs.warp:5 |
| 465 | a parameter of kind rational | 1 | tests/numbers/test_exact_parameters.pairs.warp:7 |
| 466 | a variable declared f64 | 1 | tests/numbers/test_fast_scope.pairs.warp:21 |
| 467 | a variable declared int32 | 1 | tests/numbers/test_fixed_width_ints.pairs.warp:9 |
| 468 | a variable declared uint32 | 1 | tests/numbers/test_fixed_width_ints.pairs.warp:20 |
| 469 | a variable declared uint16 | 1 | tests/numbers/test_fixed_width_ints.pairs.warp:23 |
| 470 | a parameter of kind uint32 | 1 | tests/numbers/test_fixed_width_ints.pairs.warp:24 |
| 471 | call rational | 1 | tests/numbers/test_float_as_rational.pairs.warp:5 |
| 472 | int64 overflow: the value does not fit int64 (-N…N); fix: declare it int, which is unbound | 1 | tests/numbers/test_float_as_rational.pairs.warp:9 |
| 473 | a variable declared exact | 1 | tests/numbers/test_float_exact_context.pairs.warp:27 |
| 474 | call asin | 1 | tests/numbers/test_libm_functions.pairs.warp:6 |
| 475 | call log2 | 1 | tests/numbers/test_libm_functions.pairs.warp:9 |
| 476 | call trunc | 1 | tests/numbers/test_libm_functions.pairs.warp:10 |
| 477 | the field sqrt of a int | 1 | tests/numbers/test_number_methods.pairs.warp:10 |
| 478 | the field abs of a int | 1 | tests/numbers/test_number_methods.pairs.warp:12 |
| 479 | the field cbrt of a int | 1 | tests/numbers/test_number_methods.pairs.warp:13 |
| 480 | call to | 1 | tests/numbers/test_pixel_unit.warp:14 |
| 481 | a variable declared rational | 1 | tests/numbers/test_rational_type.pairs.warp:29 |
| 482 | call second | 1 | tests/numbers/test_unit_definitions.warp:28 |
| 483 | operator ≈ on char and char | 1 | tests/operators/test_approximately.pairs.warp:16 |
| 484 | operator ≈ on bool and text | 1 | tests/operators/test_approximately.pairs.warp:21 |
| 485 | operator != on bool and int | 1 | tests/operators/test_equality_never_chains.pairs.warp:6 |
| 486 | a function value in - | 1 | tests/operators/test_negated_call_operand.pairs.warp:7 |
| 487 | operator or on int and float | 1 | tests/operators/test_pipe_stages.pairs.warp:9 |
| 488 | operator * on text and text | 1 | tests/operators/test_pipeline.pairs.warp:10 |
| 489 | operator % on int and char | 1 | tests/operators/test_postfix_words.pairs.warp:16 |
| 490 | call root | 1 | tests/operators/test_root_word.pairs.warp:5 |
| 491 | operator ~ on char and char | 1 | tests/operators/test_rough_similarity.pairs.warp:16 |
| 492 | operator ~ on list and list | 1 | tests/operators/test_rough_similarity.pairs.warp:20 |
| 493 | operator == on char and int | 1 | tests/operators/test_structural_equality.pairs.warp:46 |
| 494 | the operator then on float | 1 | tests/operators/test_then_pipe.pairs.warp:15 |
| 495 | the operator then on int | 1 | tests/operators/test_then_pipe.pairs.warp:17 |
| 496 | a call of operator_matches with N arguments | 1 | tests/operators/test_used_operator_call_form.pairs.warp:5 |
| 497 | an assignment to a text | 1 | tests/parser/test_emoji_atoms.pairs.warp:14 |
| 498 | a function value in : | 1 | tests/parser/test_named_argument_lambda.pairs.warp:7 |
| 499 | call assert! | 1 | tests/parser/test_rust_macros.pairs.warp:18 |
| 500 | call assert_eq! | 1 | tests/parser/test_rust_macros.pairs.warp:19 |
| 501 | a function value in not | 1 | tests/parser/test_rust_macros.pairs.warp:22 |
| 502 | call iter | 1 | tests/parser/test_rust_paths.pairs.warp:4 |
| 503 | call dpi | 1 | tests/programs/test_property_definitions.warp:23 |
| 504 | call beat | 1 | tests/programs/test_sound_effects.warp:39 |
| 505 | call dB | 1 | tests/programs/test_sound_synth.warp:47 |
| 506 | the operator ^= on float | 1 | tests/sweeps/test_panic_sweep.pairs.warp:8 |
| 507 | an assignment to `{a …` | 1 | tests/text/test_backtick_templates.pairs.warp:5 |
| 508 | a variable declared string | 1 | tests/text/test_declared_text_one_character.pairs.warp:5 |
| 509 | the field letters of a text | 1 | tests/text/test_selector_prepositions.pairs.warp:23 |
| 510 | an instance of Rope not giving each field | 1 | tests/text/test_str_computed.warp:4 |
| 511 | index out of range | 1 | tests/text/test_string.pairs.warp:62 |
| 512 | a call of from_utf8 with N arguments | 1 | tests/text/test_utf8_bytes.pairs.warp:6 |
| 513 | a int given to a variable declared int | 1 | tests/types/test_bare_class.pairs.warp:8 |
| 514 | call Create | 1 | tests/types/test_class_aliases.pairs.warp:8 |
| 515 | a test of the type animal, which has cases or subclasses | 1 | tests/types/test_class_cases_ported.pairs.warp:14 |
| 516 | call after | 1 | tests/types/test_class_cases_ported.warp:5 |
| 517 | call moved | 1 | tests/types/test_class_cases_ported.warp:27 |
| 518 | call doubled | 1 | tests/types/test_class_cases_ported.warp:46 |
| 519 | an assignment to `{y …` | 1 | tests/types/test_class_destructuring.pairs.warp:5 |
| 520 | call describe | 1 | tests/types/test_class_extends.warp:4 |
| 521 | the operator  on int | 1 | tests/types/test_class_forms_ported.warp:45 |
| 522 | call asdict | 1 | tests/types/test_class_json.pairs.warp:10 |
| 523 | call encodeToString | 1 | tests/types/test_class_json.warp:17 |
| 524 | call older | 1 | tests/types/test_class_methods.warp:5 |
| 525 | call perimeter | 1 | tests/types/test_class_methods.warp:11 |
| 526 | call walk | 1 | tests/types/test_class_mixins.warp:5 |
| 527 | a variable declared codepoint | 1 | tests/types/test_codepoint_digit.pairs.warp:14 |
| 528 | a parameter of kind codepoint | 1 | tests/types/test_codepoint_digit.pairs.warp:15 |
| 529 | call max | 1 | tests/types/test_dispatch.pairs.warp:18 |
| 530 | a test of the type empty | 1 | tests/types/test_empty_type_aliases.pairs.warp:8 |
| 531 | a test of the type nil | 1 | tests/types/test_empty_type_aliases.pairs.warp:10 |
| 532 | a parameter of kind unit | 1 | tests/types/test_empty_type_aliases.pairs.warp:11 |
| 533 | call bar | 1 | tests/types/test_file_declarations.pairs.warp:15 |
| 534 | call with | 1 | tests/types/test_generic_classes.pairs.warp:7 |
| 535 | a int for a typed field of dog | 1 | tests/types/test_instance_arguments.pairs.warp:5 |
| 536 | call inc | 1 | tests/types/test_mutating_methods.pairs.warp:4 |
| 537 | call text | 1 | tests/types/test_nested_type_calls.pairs.warp:5 |
| 538 | operator * on list and int | 1 | tests/types/test_one_dispatch.pairs.warp:51 |
| 539 | a call of parse with N arguments | 1 | tests/types/test_one_dispatch.pairs.warp:76 |
| 540 | a test of the type expression | 1 | tests/types/test_operation_type.pairs.warp:15 |
| 541 | the operator !== on int | 1 | tests/types/test_same_identity.pairs.warp:6 |
| 542 | operator === on int and bool | 1 | tests/types/test_same_identity.pairs.warp:23 |
| 543 | operator === on text and text | 1 | tests/types/test_same_identity.pairs.warp:25 |
| 544 | a list for a typed field of Tree | 1 | tests/types/test_self_typed_field.pairs.warp:5 |
| 545 | a char written to a typed field phone-number | 1 | tests/types/test_struct_instances.warp:25 |
| 546 | a test of the type Result, which has cases or subclasses | 1 | tests/types/test_sum_types.pairs.warp:4 |
| 547 | a char for a typed field of P | 1 | tests/types/test_text_defaults.pairs.warp:6 |
| 548 | a call of least with N arguments | 1 | tests/types/test_trait_constraints.warp:9 |
| 549 | call y | 1 | tests/types/test_tuple_values.pairs.warp:10 |
| 550 | call decimal | 1 | tests/types/test_type_calls.pairs.warp:7 |
| 551 | the field type of a int | 1 | tests/types/test_type_of.pairs.warp:4 |
| 552 | a parameter of kind Type | 1 | tests/types/test_type_parameters.pairs.warp:6 |
| 553 | a call of first_of with N arguments | 1 | tests/types/test_type_parameters.pairs.warp:14 |
| 554 | a parameter of kind [T] | 1 | tests/types/test_type_parameters.pairs.warp:15 |
| 555 | a variable declared Type | 1 | tests/types/test_type_parameters.pairs.warp:21 |
| 556 | a test of the type pair | 1 | tests/types/test_type_word_scopes.warp:4 |
| 557 | a parameter of kind double | 1 | tests/types/test_type_words.pairs.warp:10 |
| 558 | a parameter of kind int or float | 1 | tests/types/test_union_spellings.pairs.warp:7 |
| 559 | a char for a typed field of B | 1 | tests/types/test_union_spellings.pairs.warp:10 |
| 560 | a parameter of kind Circle or Square | 1 | tests/types/test_union_spellings.warp:4 |
| 561 | an instance of Counter not giving each field | 1 | tests/types/test_value_constructor.pairs.warp:6 |
| 562 | not an int | 1 | tests/types/test_variant_payloads.pairs.warp:24 |
| 563 | `)` is missing: the group opened runs to the end of the input | 1 | tests/wasm/test_wasm.pairs.warp:424 |
| 564 | hum_block is a main-level variable: declare it `global hum_block` to change it from a func | 1 | tests/web/test_signal_sound.warp:15 |
| 565 | left is a main-level variable: declare it `global left` to change it from a function, or u | 1 | tests/web/test_signal_sound.warp:25 |
| 566 | a call of gpu_render with N arguments | 1 | tests/web/test_webgpu.pairs.warp:4 |
| 567 | operator == on int and char | 1 | tests/welcoming/test_footguns.pairs.warp:7 |
| 568 | operator - on char and int | 1 | tests/welcoming/test_footguns.pairs.warp:94 |
| 569 | operator == on int and text | 1 | tests/welcoming/test_footguns.pairs.warp:229 |
| 570 | call round_half_even | 1 | tests/welcoming/test_footguns.pairs.warp:365 |
| 571 | call w | 1 | tests/welcoming/test_list_phrases.pairs.warp:4 |
| 572 | a text where a char is needed | 1 | tests/welcoming/test_welcoming_globals.pairs.warp:15 |

## "unknown name" split by name

| name | tests | examples |
|---|---|---|
| match | 186 | tests/control/test_code_hole_calls.warp:4, tests/control/test_code_hole_calls.warp:10 |
| on | 159 | tests/control/test_clipboard.pairs.warp:7, tests/control/test_colon_handlers.warp:4 |
| go | 111 | tests/control/test_after.pairs.warp:4, tests/control/test_async_ports.pairs.warp:10 |
| x | 106 | tests/control/test_block_assigns_outer.pairs.warp:5, tests/control/test_block_assigns_outer.pairs.warp:7 |
| a | 79 | tests/control/test_charged_fields.pairs.warp:4, tests/control/test_charged_fields.pairs.warp:5 |
| samples | 47 | tests/control/test_branch_assigned_variables.pairs.warp:6, tests/control/test_try_host_failure.pairs.warp:4 |
| it | 45 | tests/control/test_each_colon.pairs.warp:9, tests/control/test_each_colon.pairs.warp:10 |
| now | 45 | tests/functions/test_charged_getters.pairs.warp:14, tests/functions/test_charged_getters.pairs.warp:15 |
| square | 40 | tests/control/test_parallel_map.pairs.warp:20, tests/functions/test_broadcasting.pairs.warp:9 |
| switch | 36 | tests/control/test_error_branch_kind.pairs.warp:10, tests/control/test_match_alternatives.pairs.warp:7 |
| shared | 33 | tests/control/test_after.warp:4, tests/control/test_after.warp:5 |
| stored | 30 | tests/control/test_database_optional.warp:4, tests/control/test_database_optional.warp:7 |
| whenever | 23 | tests/control/test_colon_handlers.warp:15, tests/control/test_colon_handlers.warp:20 |
| s | 23 | tests/control/test_go_blocks.pairs.warp:12, tests/functions/test_function_typed_parameters.warp:18 |
| f | 21 | tests/control/test_control_words.pairs.warp:33, tests/functions/test_arity_overloads.pairs.warp:12 |
| nonlocal | 21 | tests/functions/test_late_binding.warp:15, tests/functions/test_late_binding.warp:18 |
| n | 20 | tests/control/test_one_method_form.pairs.warp:7, tests/control/test_one_method_form.pairs.warp:8 |
| m | 19 | tests/control/test_quasi_quote.warp:20, tests/numbers/test_dimension_types.pairs.warp:7 |
| await | 17 | tests/control/test_async_ports.pairs.warp:4, tests/control/test_async_ports.pairs.warp:7 |
| def | 17 | tests/functions/test_any_result.pairs.warp:4, tests/functions/test_any_result.pairs.warp:5 |
| linear | 17 | tests/lists/test_linear_arrays.pairs.warp:4, tests/lists/test_linear_arrays.pairs.warp:7 |
| trait | 17 | tests/parser/test_rust_items.warp:13, tests/types/test_class_interfaces.warp:4 |
| km | 16 | tests/numbers/test_decimal_units.pairs.warp:4, tests/numbers/test_dimension_types.pairs.warp:6 |
| b | 15 | tests/functions/test_function_words_consume.pairs.warp:20, tests/functions/test_function_words_consume.pairs.warp:21 |
| view_width | 14 | tests/control/test_host_word_failures.warp:4, tests/lists/test_global_typed_lists.pairs.warp:26 |
| tests | 14 | tests/ffi/test_ffi.pairs.warp:10, tests/ffi/test_ffi.pairs.warp:20 |
| do | 13 | tests/control/test_block_assigns_outer.pairs.warp:4, tests/control/test_block_assigns_outer.pairs.warp:6 |
| ms | 13 | tests/control/test_parallel_map.warp:4, tests/ffi/test_host_words.pairs.warp:9 |
| h | 12 | tests/control/test_quasi_quote.warp:23, tests/functions/test_python_ruby_definitions.warp:28 |
| add | 11 | tests/control/test_reflection_words.warp:57, tests/control/test_reflection_words.warp:59 |
| undo | 11 | tests/control/test_undo_history.pairs.warp:4, tests/control/test_undo_history.pairs.warp:5 |
| Hz | 10 | tests/numbers/test_dimension_types.pairs.warp:23, tests/numbers/test_units_after_numbers.warp:30 |
| where | 9 | tests/control/test_data_quoting.pairs.warp:25, tests/lists/test_for_where.warp:4 |
| _ | 9 | tests/control/test_for_underscore.pairs.warp:6, tests/control/test_is_pattern.pairs.warp:4 |
| a-b | 9 | tests/control/test_iteration_words.pairs.warp:13, tests/numbers/test_math.pairs.warp:17 |
| let | 9 | tests/control/test_rust_inclusive_range.pairs.warp:4, tests/control/test_rust_option.pairs.warp:7 |
| hex | 9 | tests/parser/test_parser_sugar.pairs.warp:25, tests/parser/test_parser_sugar.pairs.warp:26 |
| result | 9 | tests/scope/test_result_word.pairs.warp:4, tests/scope/test_result_word.pairs.warp:5 |
| each | 8 | tests/control/test_each_colon.pairs.warp:4, tests/control/test_each_colon.pairs.warp:6 |
| reduce | 8 | tests/control/test_iteration_words.pairs.warp:4, tests/control/test_iteration_words.pairs.warp:7 |
| session | 8 | tests/control/test_storage.pairs.warp:26, tests/control/test_storage.pairs.warp:27 |
| foo | 8 | tests/functions/test_parameter_overloads.pairs.warp:10, tests/functions/test_parameter_overloads.warp:16 |
| from | 8 | tests/functions/test_preposition_calls.warp:24, tests/functions/test_preposition_calls.warp:28 |
| xs | 8 | tests/functions/test_variadic.pairs.warp:29, tests/lists/test_where_filter.pairs.warp:13 |
| plus | 8 | tests/numbers/test_number_words.pairs.warp:4, tests/numbers/test_number_words.pairs.warp:10 |
| decimal | 8 | tests/numbers/test_repeating_decimal.pairs.warp:17, tests/numbers/test_repeating_decimal.pairs.warp:18 |
| interface | 8 | tests/parser/test_time_unit_keys.pairs.warp:11, tests/types/test_class_interfaces.warp:20 |
| emit | 7 | tests/control/test_event_footguns.pairs.warp:4, tests/control/test_event_footguns.pairs.warp:26 |
| this | 7 | tests/control/test_reflect_inferred.warp:4, tests/control/test_reflect_inferred.warp:5 |
| storage | 7 | tests/control/test_storage.pairs.warp:4, tests/control/test_storage.pairs.warp:5 |
| Math | 7 | tests/ffi/test_foreign_js.pairs.warp:4, tests/ffi/test_foreign_js.pairs.warp:5 |
| format | 7 | tests/functions/test_argument_cast.warp:4, tests/functions/test_argument_cast.warp:5 |
| fn | 7 | tests/functions/test_rust_references.pairs.warp:19, tests/functions/test_rust_references.pairs.warp:22 |
| at | 6 | tests/control/test_colon_handlers.warp:10, tests/control/test_time_of_day.pairs.warp:4 |
| math | 6 | tests/control/test_function_objects.warp:7, tests/control/test_function_objects.warp:9 |
| floor | 6 | tests/control/test_function_value_words.pairs.warp:4, tests/functions/test_map_library_word_first.pairs.warp:4 |
| interpret | 6 | tests/control/test_run_time_blocks.pairs.warp:5, tests/control/test_run_time_blocks.pairs.warp:15 |
| when | 6 | tests/control/test_when_signals.pairs.warp:4, tests/control/test_when_signals.warp:4 |
| upper | 6 | tests/functions/test_braceless_closure_call.pairs.warp:4, tests/functions/test_braceless_closure_call.pairs.warp:9 |
| $0 | 6 | tests/functions/test_dollar_body.pairs.warp:4, tests/functions/test_nested_it.pairs.warp:16 |

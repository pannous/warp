# Restructuring: layout by concept

Decision restructuring (user, 2026-10-14, notes/decisions.md): every function lives in the file of its concept. A
general file (parser core, expressions.rs, mod.rs) holds no syntax of a particular keyword or construct. Moves are pure
moves in their own commits (visibility widened to `pub(crate)` where a function leaves its module is the only edit);
merged duplicates are cuts and belong to warp-duplicates-exact / warp-duplicates-similar. Inventory:
data/restructuring/functions.tsv (name, file, line). One branch per area, parser first.

## Parser (`src/warp_parser/`)

Core stays flat and general: `mod.rs` (WarpParser, ParserOptions, parse entry points, Atom), `scanning.rs` (cursor,
whitespace, identifiers), `lookahead.rs` (generic peeks), `expressions.rs` (the Pratt loop, operator diagnostics),
`atoms.rs` (atom dispatch, calls), `suffixes.rs` (suffix dispatch), `declaration_scan.rs` (pre-pass over declared
types). Everything keyword- or construct-specific goes to a concept file:

```
warp_parser/
  mod.rs scanning.rs lookahead.rs expressions.rs atoms.rs suffixes.rs declaration_scan.rs   core
  blocks.rs          {…}, do…end, colon/indented bodies, trailing blocks
  comments.rs        // # /* */ comments, the // floor-division ambiguity
  commands.rs        print and braceless commands `sleep 1s`
  pipes.rs           |>, f.(xs) broadcast
  math_notation.rs   superscript powers, |x| norm bars
  annotations.rs     @name, @name(value), Ruby @x
  control/           conditions.rs (conditions, unless, statement modifiers, `x empty`), if_else.rs (if/else/elvis),
                     loops.rs (for/while/until/times), switch.rs (match arms), guards.rs (try/catch/finally/fallback),
                     jumps.rs (return, await, emit)
  definitions/       functions.rs (definitions, parameters, function references), lambdas.rs, variables.rs
                     (global/nonlocal/constants/<-), foreign.rs, operators.rs (user operators, operator methods)
  types/             declarations.rs (class/struct/enum/sum types), expressions.rs (type applications, list<T>, T[])
  collections/       lists.rs (brackets, separators), subscripts.rs (index, slices), ranges.rs (down to, ..n-1)
  literals/          mod.rs (parse_value), numbers.rs, units.rs, text.rs, shaders.rs, constants.rs, paths.rs (paths,
                     URLs, versions), data.rs (`data a and b`)
  markup/            xml.rs, css.rs
```

Removed by the moves: statements.rs, lists.rs, literals.rs, subscripts.rs, user_operators.rs, xml.rs, atoms/ (its two
files split into blocks.rs, definitions/functions.rs, definitions/lambdas.rs, types/declarations.rs).

Blocked until their branches land (ask the owner): atoms.rs (cleanup-park-phrases, synonyms-table), lookahead.rs and
mod.rs (synonyms-table). Moves out of other files go first.

### Function → target (parser, 320 moves; 93 functions stay in core)

| target | function ← current file |
|---|---|
| annotations.rs | lookahead.rs: `parse_attribute`, `reads_instance_variable`, `attribute_value`, `try_parse_trailing_attribute` |
| atoms.rs | lists.rs: `parse_symbol` |
| blocks.rs | atoms/blocks_and_lambdas.rs: `closing_end_follows_from`, `names_end`, `without_end_lines`, `is_endless_definition`, `code_words`; mod.rs: `curly_block`, `split_trailing_block`; lists.rs: `with_trailing_block`, `is_block`; expressions.rs: `continue_colon_body`, `at_body_start`; statements.rs: `colon_body`, `indented_lines_below`, `statement_block`, `as_block`; lookahead.rs: `skip_python_colon`; scanning.rs: `bare_operator_in_block` |
| collections/lists.rs | lists.rs: `at_group_end`, `parse_bracketed`, `parse_list_with_separators`, `parse_separator`, `group_by_separators`; mod.rs: `with_arrow_bodies`; scanning.rs: `signed_number_starts_a_list`, `sign_glued_to_operand` |
| collections/ranges.rs | expressions.rs: `combined`; lookahead.rs: `down_to_length`; mod.rs: `is_minus_one`; user_operators.rs: `range_reading` |
| collections/subscripts.rs | subscripts.rs: `subscript`, `counts_from_the_end`, `slice_bounds`, `hash_range_warning`, `hash_slice_bounds`, `unless_open`, `subscript_key`; suffixes.rs: `try_parse_subscript`, `try_attributed_body`, `parse_slice_from_start`, `attribute_items`; expressions.rs: `names_a_key`; mod.rs: `is_unindexable_keyword` |
| commands.rs | mod.rs: `is_print_word`, `starts_print`, `one_expression`, `print_call`, `print_arguments`, `print_arguments_of`, `grouped_list`, `is_command`; statements.rs: `rest_of_statement`, `expression_of_words`, `expression_binding`, `words_of_expression`, `words_binding`, `print_call_from`, `take_braceless_print`, `braceless_argument_follows` |
| comments.rs | scanning.rs: `at_hash_comment`, `at_line_comment`, `at_block_comment_start`, `consume_block_comment`, `take_trailing_comment`, `skip_spaces_and_inline_comments`, `skip_whitespace_and_comments`, `consume_to_line_end`, `consume_rest_of_line`, `comment_reads_like_divisor`, `at_floor_division`, `educate_on_spaced_floor_division`; mod.rs: `comment_hiding_a_closer`, `glued_floor_division`, `missing_closer` |
| control/conditions.rs | expressions.rs: `parse_condition`, `with_equals_comparing`, `guarded_statement`, `ends_with_clause_word`; mod.rs: `negate_condition`, `guarded_command`; lookahead.rs: `peek_negated_control_word`; suffixes.rs: `try_parse_test_word` |
| control/guards.rs | lookahead.rs: `peek_guard_word`, `guard_word_ahead`, `parse_guard`, `with_finally`, `finally_ahead`, `fallback_word_ahead`, `parse_caught_name`, `parse_guarded_phrase` |
| control/if_else.rs | expressions.rs: `finish_if_prefix`, `branch_assignment`; statements.rs: `parse_branch`, `parse_optional_else`, `else_on_a_later_line`, `at_else_if_word`, `parse_else_if`; suffixes.rs: `condition_has_branch`, `try_parse_elvis` |
| control/jumps.rs | expressions.rs: `try_parse_return`, `try_parse_await`, `try_parse_emit`; lookahead.rs: `try_parse_after_return`, `return_ahead` |
| control/loops.rs | statements.rs: `for_iterable_and_body_word`, `loop_variable_in_follows`, `parse_loop_variable`, `at_c_style_for_head`, `try_parse_for_in`, `parse_iterable`, `at_loop_body`, `try_parse_condition_loop`, `names_adjective_and_type`, `c_style_for`, `finish_while_prefix`; mod.rs: `for_in_loop`, `while_do`, `iterates_keys`; suffixes.rs: `try_parse_times`, `times_fills_list`, `value_after_times`, `parse_times_loop` |
| control/switch.rs | scanning.rs: `else_arm_key`, `in_arm_key` |
| definitions/foreign.rs | atoms.rs: `foreign_form`; statements.rs: `skip_foreign_modifiers` |
| definitions/functions.rs | expressions.rs: `register_defined_function`; atoms.rs: `try_parse_to_definition`, `at_to_header_end`, `phrase_result_type`, `phrase_redefinition`; mod.rs: `defined_function_name`, `is_function_block`, `typed_parameter`, `typed_parameters`; lookahead.rs: `defines_named`, `after_function_keyword_word`, `result_type_follows`, `function_reference_after`, `starts_function_reference`, `starts_capture`; atoms/blocks_and_lambdas.rs: `elixir_function_head`; lists.rs: `with_definition_block`, `is_parameter_word`, `is_typed_method`, `assigned_method_call` |
| definitions/lambdas.rs | atoms/blocks_and_lambdas.rs: `parse_matlab_lambda`, `backslash_lambda_ahead`, `parse_backslash_lambda`, `generic_parameters_length`, `parse_stabby_lambda`; lists.rs: `field_holding_lambda` |
| definitions/operators.rs | expressions.rs: `try_parse_operator_method_head`, `parenthesis_length`, `try_parse_operator_value`; mod.rs: `operator_call`; user_operators.rs: `user_operator_at`, `try_parse_user_suffix`, `try_parse_user_infix`, `try_parse_user_prefix`, `try_parse_operator_declaration`, `hint_operator`; declaration_scan.rs: `is_operator_glyph`, `declared_glyph`, `declared_operator`, `is_operator_word`, `scan_user_operators`, `declared_operators`, `precedence_declaration`, `built_in_level`, `default_level` |
| definitions/variables.rs | statements.rs: `try_parse_global_declaration`, `name_after_blanks`, `parse_nonlocal_declaration`, `name_after_comma`, `skip_declaration_modifiers`, `typed_global`; lists.rs: `declarations_of_several_globals`; expressions.rs: `note_variable`; atoms.rs: `names_field`, `type_annotation_follows`, `refuse_constant_assignment`, `assignment_follows`; scanning.rs: `left_arrow_assignment`, `follows_operand_on_its_line`, `is_set_apart` |
| expressions.rs | mod.rs: `chained_equality`, `ungrouped_equality`, `logic_mixed_with_comparison`, `joins_operands` |
| literals/constants.rs | mod.rs: `check_constants`, `truth_glyph`; atoms.rs: `truth_glyph_at` |
| literals/data.rs | expressions.rs: `try_parse_data` |
| literals/mod.rs | literals.rs: `parse_value` |
| literals/numbers.rs | literals.rs: `parse_number`, `scientific_float`, `keep_source_literal`, `parse_number_value`, `read_number_value`, `repeating_decimal_literal`, `vulgar_fraction_literal`, `integer_node`, `push_digits`, `parse_exponent`, `overlined_fraction`, `float_node`, `number_cast_follows`, `literal_type_suffix`, `at_ordinal_suffix`; mod.rs: `number_in_text`, `real`, `vulgar_fraction`, `is_number_glyph` |
| literals/paths.rs | atoms.rs: `parse_path_literal`, `url_follows`, `parse_url`, `version_operand`; lookahead.rs: `ends_path`, `starts_path_literal`, `is_url_terminator` |
| literals/shaders.rs | literals.rs: `parse_shader_block`, `shader_blocks` |
| literals/text.rs | literals.rs: `parse_string`, `read_text`, `quoted_text`, `warn_unknown_entity`, `parse_f_string`, `hex_byte_escape`, `unicode_escape`, `parse_dollar_hole`, `parse_swift_hole`, `text_until_closing`, `closing_brace`, `without_common_indentation`, `dedented_lines`; atoms.rs: `parse_emoji` |
| literals/units.rs | literals.rs: `with_juxtaposed_factor`, `at_spaced_unit`, `hint_glued_unit`, `spaced_word`, `with_time_unit`, `time_unit_at`, `following_duration` |
| lookahead.rs | literals.rs: `blanks_then`, `block_after_blanks`, `closes_after_blanks`, `ends_optional_type`, `next_field_follows`; lists.rs: `starts_code` |
| markup/css.rs | lookahead.rs: `css_color_length`, `at_pseudo_class_selector` |
| markup/xml.rs | xml.rs: `parse_xml_text_content`, `skip_processing_instruction`, `skip_xml_comment`, `skip_doctype`, `parse_cdata`, `is_at_line_end`, `only_blanks_before_newline`, `parse_xml_tag`, `parse_attribute_value`, `parse_closing_tag`, `xml_element`, `curly_or_empty` |
| math_notation.rs | suffixes.rs: `superscript_exponent`, `superscript_variable_exponent`, `try_parse_superscript_power`; mod.rs: `superscript_sign`, `superscript_letter`, `superscript_digit`; expressions.rs: `parse_norm_bars` |
| node/ (duplicates check) | mod.rs: `mentions` |
| pipes.rs | expressions.rs: `at_pipeline`, `pipe_parameters`; mod.rs: `piped`, `dot_call`; scanning.rs: `pipe_operand` |
| scanning.rs | statements.rs: `at_identifier_start`; atoms/type_declarations.rs: `blanks_ahead`; mod.rs: `is_identifier_char`, `is_music_accidental`, `read_atoms` |
| types/declarations.rs | atoms/type_declarations.rs: `type_declaration_follows`, `parse_type_declaration_body`, `parse_comma_names`, `skip_struct_words`, `skip_conformances`, `declared_type_after_blanks`, `class_keyword_after_blanks`, `identifier_after_blanks`, `parse_primary_constructor`, `variants_ahead`, `parse_sum_type`, `enum_with_values_ahead`, `parse_enum_with_values`, `variant_class`, `parse_type_parameters`, `any_for_type_parameters`, `untyped_parameters`, `without_case_keyword`, `statements`; lists.rs: `class_body`, `transform_fields_to_types`, `symbol_to_type`; atoms.rs: `type_parameter_names` |
| types/expressions.rs | lists.rs: `joined_type_name`, `words_ahead_length`, `type_application_length`, `parse_type_application`, `with_type_phrases`, `without_ref_words`; mod.rs: `names_a_type`, `type_application_name`, `list_type_head`, `noted_list_type_head`; suffixes.rs: `try_parse_array_type_suffix` |

## Root (`src/*.rs`, 56 flat files): proposed folders

The user (2026-10-14): the root needs the most restructuring. File level first: `git mv` into a concept folder, with
`pub use <folder>::<file>;` in lib.rs so `crate::<file>::…` paths stay valid (no use-site edits in the move commit);
then a per-function pass over the big mixed files (main.rs, pipeline.rs, normalize.rs, diagnostic.rs, tasks.rs,
modules.rs, wasm_modules.rs, cargo_crates.rs, effects.rs), whose table is added here when it starts.

- stays at the root: lib.rs, main.rs, pipeline.rs, util.rs, extensions.rs + extensions/, words.rs, operators.rs,
  type_kinds.rs, node/, warp_parser/, lowering/, analyzer/, wasm_emitter/
- compiler/: compile_context, compile_time, analysis_memo, context, local, function, meta, dead_functions,
  function_equality
- analyzer/: effects (effect inference is analysis)
- diagnostics/: diagnostic, fixits, normalize (hints toward canonical syntax), crash_card
- numbers/: real, uncertain, fixed_width, units.rs + units/
- time/: time.rs becomes time/mod.rs
- text/: interpolation, uniscript_entities (+ .tsv), injection
- formats/: wisp_parser, web_idl, ffi_parser, meta_section
- modules/: modules.rs + modules/, wasm_modules, components, component_builder, cargo_crates, package_tools,
  std_adapters, std_docs, foreign, binaryen
- host/: host.rs + host/, host_parts, sandbox, channels, tasks, shared, database
- run/: wasm_reader, file_type (`warp register`)
- web/: markup, accessibility
- law.rs + law/, graphics/, sound/, gc_traits/, ffi/: stay

## Lowering (`src/lowering/`, 112 files): proposed folders

File-level first (pure `git mv` + mod paths), then per function inside each folder when the area starts (its table is
added here then).

- core: mod.rs, nodes.rs, folding.rs, pipes.rs
- control/: blocks, switch, match_exhaustive, for_loop/, go_blocks, parallel, run_time_blocks, test_blocks,
  scoped_handlers, result_word
- definitions/: declarations/, declaration_forms, definition_cases, file_declarations, overloads, lambdas, closures,
  function_values, named_arguments, parameter_patterns, parameter_shapes, variadic, inlining, memoization,
  late_binding, getters, nonlocal_cells, mutation, references, undo_history
- types/: class_methods/, traits/, sum_variants, plain_enums, optional_casts, object_groups, type_aliases,
  type_constructor, type_name_matching, type_tests, tuples, named_patterns
- collections/: comprehensions, list_element_checks, list_phrases, lazy_ranges, nested_index, index_positions,
  one_entry_maps, min_max, broadcasting, shared_arrays, number_keys, key_names, word_slices, tree_walks, of_lookups
- words/: library_words/, phrase_calls, phrase_words, parse_word, number_words, times_phrases, music_words,
  channel_words, negated_calls, picked_calls, word_operators
- welcoming/: welcome_forms, ambiguous_forms, soft_keywords, std_aliases, kebab_names
- web/: element_events, field_elements, glued_attributes, markup_tags, page_html, style_rules, select_options, routes,
  serve, listeners, transitions, component_state, component_worlds
- signals/: event_signals, fetch_signals, signal_values, system_signals, system_values, variable_signals
- data/: database_tables/, stored_values, meta_entries, introspection, reflection, printable, versions
- foreign/: foreign_modules, library_signatures, gpu_maps

Owned by open branches (warp-operator-shall: the big lowering files; warp-cleanup: type-name-params, orm): checked
with the owner before each folder move.

Status (branches layout-lowering-signals … -control, 2026-10-14): all folders exist. Each folder mod.rs does
`use super::*`, so moved files kept every `super::x` path unchanged. Still flat until their open branches land:
declarations and type_name_matching (warp-cleanup's park-phrases), welcome_forms (foreign-spellings),
and the park-phrases files phrase_calls, list_phrases, word_slices, music_words, of_lookups.

## Analyzer (`src/analyzer/`, 10 files)

Small and already by concept (booleans, counting, imports, inference, list_views, upcast_fields, user_functions/,
variables, checks/). Per-function pass only: keyword-specific checks leave mod.rs/inference.rs for their concept file.

## WASM emitter (`src/wasm_emitter/`, 60 files): proposed folders

- core: mod.rs, module_finish, type_manager, import_manager, globals, layout, config, declared_values, constructors,
  values, diagnostics, function_builder, string_table
- numbers/: arithmetic, big_int/, exact, float_text, radix, casts, uncertain
- text/: text_builtins, text_rooms, text_unicode, similarity
- lists/: list_emitter, list_ops/, list_dispatch/, list_abi, list_sharing, int_lists, linear_arrays, tuple_emitter
- maps/: map_backend, map_index, key_emitter, struct_backend, raw_structs
- time/: times, time_arithmetic, time_zones
- control/: control_flow, loop_control, try_guard, traps
- functions/: closures, user_function_calls, node_functions, cells
- host/: wasi_emitter, ffi_emitter, component_adapters
- reflection/: reflection, node_api, equality, witness, library_ops/

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

Status (2026-10-14): chunks 1, 2a, 2b are on main. Chunk 3 (branch layout-parser-3, afdb0dd5f, sent to the
Integrator): atoms.rs keeps only symbols, calls, keyword forms and glued suffixes; atoms/type_declarations.rs is
types/declarations.rs; lists.rs is dissolved into collections/lists.rs, literals/, definitions/ and types/.
Tool: scripts/move_fn.py (cuts fns with docs and attributes, widens what leaves mod.rs or goes deeper); intermediate
folder mod.rs files start with `use super::*;` and glob-import their children so `use super::*` chains see them.

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

The user (2026-10-14): the root needs the most restructuring. File level first: `git mv` into a concept folder with
scripts/move_module.py, which rewrites every `crate::`/`warp::` use path (no `pub use` aliases); then a per-function pass over the big mixed files (main.rs, pipeline.rs, normalize.rs, diagnostic.rs, tasks.rs,
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

Status (2026-10-14): branch layout-root (4749cbf5b) did compiler/, analyzer/effects, numbers/, text/, formats/, modules/,
host/, run/, web/. Deferred: diagnostics/ (diagnostic and normalize are used by nearly every open branch) and time/.

### Moves with many users (pattern, warp-supervisor 2026-10-14)

A module whose path many files use (and open branches keep adding uses of) moves in two steps:
1. Move the file with `scripts/move_module.py <module> <folder> --keep-paths`. Uses stay as they are, and lib.rs keeps
   the old path alive with `pub use <folder>::<module>;`. No open branch breaks, and branches merged later just keep
   writing the old path.
2. Once those branches have landed, run one fastmod pass in a quiet window right after an Integrator batch (agree the
   window with warp-integrator): `crate::<module>` → `crate::<folder>::<module>` (also `warp::` in tests), then drop
   the re-export.

Applied: diagnostics/ (diagnostic 173 using files, normalize 115, fixits 12, crash_card 2) on branch
layout-diagnostics; step 2 is pending. time.rs → time/mod.rs needed no path changes (branch layout-time).

### Per-function pass over the root files (card layout-root)

One chain of branches, all pure moves: layout-main → layout-pipeline → layout-modules (5e83c8849) → layout-root-6 →
layout-root-7, and layout-root-8 on main (sent 2026-10-14). Each root file or mod.rs re-exports its new children, so `crate::…` paths stay.
Users were updated only where few files use them (run_time_blocks, errors/run_time).

The rule (user 2026-10-14): split a file by concept, never for its length; ~500 lines is fine. type_kinds.rs was split
and then reverted (5e83c8849): its entries are related enough for one file.

| from | to |
|---|---|
| main.rs (1291 → 463) | src/cli/: flags, outputs, executables, display, serve, console |
| pipeline.rs (1042 → 623) | compiler/compile_mode.rs (CompileMode, `for_*`; pipeline re-exports), compiler/run_time_blocks.rs, errors/run_time.rs |
| modules.rs (1214 → 180) | modules/: use_statements, implicit_modules, needed_definitions, declarations, program_file; std/package/loader items to their files |
| normalize.rs | normalize/: positions, style, hints, check_style, mod.rs (core hint functions) |
| operators.rs | operators/: spellings, binding_power, codes, mod.rs (Op and its kinds) |
| law.rs (504 → 383) | law/asserted.rs, law/property_tests.rs |
| host.rs (365 → 281) | host/fetch.rs, host/aliases.rs; host.rs keeps the host word registry |
| modules/wasm_modules.rs | wasm_modules/: exports, c_calls, program_uses, linking (native) |
| analyzer/effects.rs | effects/: termination (Div), constraints, events, queries, resolver; mod.rs keeps the effect sets and the report |
| errors/diagnostic.rs | diagnostic/: positions (where, excerpts), asks (ambiguities, got it); mod.rs keeps reporting and pragmas |
| host/tasks.rs | tasks/: values, channels, pool; mod.rs keeps the task table |
| modules/cargo_crates.rs | cargo_crates/: rustdoc (reading the API), wrapper (WIT and Rust generation), compile (cargo build); mod.rs keeps download and cache (layout-root-8, on main after cleanup-roots) |

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
`use super::*`, so moved files kept every `super::x` path unchanged. welcome_forms joined welcoming/ last (branch
layout-welcoming, after rs-private); park-phrases deleted phrase_calls, list_phrases and word_slices. Only folding,
nodes and pipes stay flat in src/lowering: they serve every folder.

### Lowering per function (branch layout-lowering-functions)

Moved with scripts/move_fn.py. Callers name the new module, and the crate root re-exports it (lib.rs).

| new file | from | functions and constants |
|---|---|---|
| collections/list_mutations.rs | definitions/declaration_forms.rs | `appends`, `is_list_mutating_method`, `is_removal_method`, `is_place`, `appended_element`, `called_on_place`, `popped_list`, `removed_key`, `cleared_collection`, `became`, `prelude_call`, `inserted_element`, `insert_arguments`, `insert_position`, `lowered_insert`, `updates_list`, `Inserted`; the method and pseudo-call constants (`LIST_BECOME`, `INSERT_AT_CALL`, `REMOVED_VALUE_CALL` …) |
| collections/filled_lists.rs | definitions/declaration_forms.rs | `zero_element`, `zero_list`, `lower_list_times`, `list_times`, `is_list_literal`, `list_plus`, `filled_list`, `zero_filled_subscript`, `typed_array_value`, `typed_array_declaration`, `counted_array_declaration`, `ZERO_FILL_CALL`, the list-times/list-plus topics |
| types/inline_unions.rs | definitions/declaration_forms.rs | `lower_inline_unions`, `lower_unions_of` |
| words/library_words/function_methods.rs | words/library_words.rs | `lower_function_methods`, `in_function_scope`, `list_constructor_calls`, `function_methods_as_calls` |
| words/library_words/guards.rs | words/library_words.rs | `Lowering::{is_known_list, lower_try, lower_try_binding, lower_assert}`, the try/assert placeholders |
| words/library_words/similarity.rs | words/library_words.rs | `is_similarity_call`, `Lowering::lower_similar`, `VALUES_SIMILAR`, `VALUES_ROUGH`, `SIMILARITY_LEVELS` |
| words/library_words/templates.rs | words/library_words.rs | `Lowering::instantiate_template`, `temporary_name`, `named_apart`, `substitute`, `TRY_TEMPORARY`, `TRY_BASE` |
| types/class_methods/method_aliases.rs | types/class_methods.rs | `method_aliases`, `aliased_method`, `with_method_aliases`, `METHOD_ALIASES` |
| types/class_methods/json.rs | types/class_methods.rs | `lower_json_classes`, `with_json_classes`, `TO_JSON_WORD`, `TO_JSON_OF_CLASSES_WORD`, `SERIALIZER_ALIASES` |
| types/class_methods/statics.rs | types/class_methods.rs | `instance_member`, `is_static`, `static_name`, `static_members`, `static_reads`, `static_definitions` |
| foreign/gpu_maps/kernels.rs | foreign/gpu_maps.rs | the WGSL translation: `Kernel`, `Reduction`, `Translation`, `ProgramFunction`, the painted fragment shaders, `float_literal` |

Branches: layout-lowering-library-words, -class-methods, -gpu-maps (2026-10-14).

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

Status (2026-10-14): the folders exist (branch layout-emitter, on main).

### Emitter and analyzer per function (branch split-emitter)

Moved with scripts/move_fn.py, pure moves. The emitter's folder modules are glob re-exported (`pub use
control::traps::*`), the analyzer's files too (`pub use inference::*`), so outside paths stay as they are.

| target | function ← current file |
|---|---|
| wasm_emitter/control/traps.rs | globals.rs: `emit_raised_error`, `emit_returned_value`, `emit_trap_detail`, `emit_trap_position`, `trap_position_global`, `trap_detail_global`; mod.rs: `EXACT_TRAP_MESSAGES`, `TRAP_DETAIL`, `TRAP_DETAIL_PREFIX` |
| wasm_emitter/control/control_flow.rs | mod.rs: `if_then_parts`, `LOGICAL_SCRATCH` (and/or) |
| wasm_emitter/host/fetch.rs (new) | globals.rs: `emit_fetch_call`, `extract_url_string` |
| wasm_emitter/host/wasi_emitter.rs | mod.rs: `emit_wagi_start`, `OUTPUT_WORDS`, `OUTPUT_CALLS` |
| wasm_emitter/numbers/arithmetic.rs | mod.rs: `is_numeric`, `emit_float_comparison`, `LIBM_POW`, `LIBM_CBRT` |
| wasm_emitter/numbers/casts.rs | mod.rs: `emit_truncating_cast`, `emit_truncating_cast_via`, `emit_float_in_exact_context`, `I64_RANGE_LIMIT`, `ROUNDING_FUNCTIONS`; values.rs: `is_int_type_word` |
| wasm_emitter/lists/list_emitter.rs | values.rs: `emit_range`, `emit_list_structure`, `emit_list_structure_with`, `items_info` |
| wasm_emitter/lists/list_sharing.rs | mod.rs: `reads_collection`, `COLLECTION_READING_WORDS` |
| wasm_emitter/text/text_builtins.rs | mod.rs: `join_call`, `joined_text` |
| wasm_emitter/maps/key_emitter.rs | mod.rs: `emit_new_key` |
| wasm_emitter/functions/closures.rs | mod.rs: `CAPTURE_EXPORT_PREFIX` |
| analyzer/checks/kind_changes.rs | variables.rs: `check_kind_changes`, `global_variables`, `first_kind_change`, `evident_kind`, `evident_results`, `last_expression` |
| analyzer/inference.rs | user_functions.rs: `literal_kind`, `computed_literal_kind`, `exact_literal_value`, `operand_kind`, `argument_literal_kind`, `is_decimal_literal` |
| analyzer/annotations.rs (new: the kinds type words and annotations give) | user_functions.rs: `type_word_kind`, `annotated_kind`, `is_map_type`, `names_empty_type`, `names_list_type`, `field_kind_key`, `declared_field_kinds`, `program_field_kinds` |
| analyzer/main_globals.rs (new: local-or-global, a main variable a function assigns) | variables.rs: `resolve_main_variable_assignments` … `assignment_target_root` (17 functions); after branch diagnostics-paths lands, which edits them |

Stay: mod.rs keeps the module driver (emit, emit_for_node, emit_node_main, emit_node_value, memory, kind globals,
string allocation, the Node list encoding `bracket_info` … `bracket_of_info`); `kind_with_article`/`with_article` and
the declaration keyword tests of variables.rs are used everywhere and wait for a words/ home.

### Same-named functions after duplicates-exact (split-emitter, 2026-10-14)

All 494 names used in several files of data/restructuring/functions.tsv have a verdict in duplicates_exact.md. A fresh
inventory of main (9fdebbb1b) has 7 names that are new since:

| name | verdict |
|---|---|
| `uses_it` (analyzer/user_functions/definitions.rs, lowering/type_name_matching.rs) | true duplicate: one `analyzer::uses_it`, Node::mentions of IMPLICIT_PARAMETER |
| `is_arithmetic` (operators.rs Op method, analyzer/checks/lints.rs) | lints' repeated the operator list: `is_ungrouped_arithmetic` asks Op::is_arithmetic |
| `aliased_method`, `method_aliases` (class_methods.rs, welcome_forms.rs) | distinct: a class's own method for a word vs. another language's method name; both read METHOD_ALIASES |
| `keyword_definition` (declarations.rs, library_words.rs) | distinct: the whole definition vs. name and parameters of the head |
| `map_key` (law/type_model/expressions.rs, wasm_emitter fields_and_keys.rs) | distinct: Lean model vs. emitter |
| `start_of` (warp_parser/atoms.rs, diagnostics/normalize.rs) | distinct: parser cursor vs. a node's position |

`is_type_word` ×4 (card name-their): four different sets, renamed by set: `is_builtin_type_word` (BUILTIN_TYPES),
`is_kinded_type_word` (analyzer::type_word_kind), `is_kinded_type_or_any` (function heads),
`is_kinded_or_testable_type` (units). The parser's local closure in typed_parameter is the kinded set again.
Seven constants spell "it" (card duplicate-constants).

### Duplicate constants (card duplicate-constants, split-emitter, 2026-10-14)

Finder: `probes/cleanup/duplicate_constants.py` (constants holding the same text or word list); merger:
`scripts/merge_constant.py [--home <file>] NAME file:LOCAL…` (spec of the first round: data/split-emitter/merges.txt).
- Merged into src/words.rs (branch words-table): `IMPLICIT_PARAMETER` (seven "it"s) and 36 language words: FOR IN EACH
  COUNT RETURN OF MAP GLOBAL ANY ADD ALL AT CHR DELETE DIR DOT ERROR FLOAT GO IS IS_ERROR LET LIST NUMBER ON PAIR POP
  PRINT RAISE REAL SELF SEND SORT STR SYMBOL TEXT (`<NAME>_WORD`, ANY_TYPE).
- Distinct despite equal text, kept with their names: a word's role differs from the spelling (`VALUE_FIELD` vs.
  `VALUE_WORD`, `·` as UNIT_PRODUCT vs. NAME_SEPARATOR, `"wasm"` as runtime vs. extension, `MODULE_EXTENSIONS` of
  modules.rs (warp, wasp) vs. wasm_modules.rs (wasm, wat)); `EXTREMA` of min_max.rs pairs words with operators.
- Second round (data/split-emitter/merges2.txt, after errors-paths): ADD DELETE EACH ERROR FLOAT FOR IN IS IS_ERROR LET
  LIST MAP OF PRINT RETURN SELF (RECEIVER, RECEIVER_WORD) COUNT STR GLOBAL, and new homes DO ORD SPLIT LOG COPY.
- Distinct, kept: Swift's closure `in` (SWIFT_IN), KERNEL_COUNT (gpu_maps), AWAIT_ALL_WORD, MINIMUM_KEYWORD (`from`
  of versions), STATE_KEYWORD (`global` in effects), LET_TOPIC, the field and parameter names that spell a word
  (NUMBER_FIELD, ERROR_FIELD, LIST_PARAMETER, MAP_PARAMETER, LIST_ANSWER, TEXT_HOLE, ANY_STEP), page_tests' TEXT_WORD.
- Left: the non-word duplicates, each into one home with `--home`: UNION_JOINER (law/type_model + declared_types),
  MODULE_FILE ×3 (web/route_split, deploy, site), JSON_TYPE/TEXT_TYPE (web_server + dev_server), STACK_EXHAUSTED
  (law + host), MISSING_TARGET_SIGN (package_tools + cargo_crates), WARP_CALL, FIRED_PREFIX ("once_fired_" ×2;
  signal_values' "signal·fired·" differs), STYLE ×6, the same word lists EXTREMA/EXTREMUM_WORDS, run/file_type
  EXTENSIONS = modules MODULE_EXTENSIONS, CHANGE_WORDS ×2.

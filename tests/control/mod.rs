mod test_aborting_handlers;
mod test_after;
mod test_all_with_condition;
mod test_async_ports;
mod test_atomic_arrays;
mod test_await_any_unwarned; // card await-warns
mod test_await_any_values;
mod test_await_any_variables;
mod test_await_go;
mod test_bare_try; // card failed-raised
mod test_block_assigns_outer;
mod test_block_parameter_calls; // card repeats-loop
mod test_block_parameters;
mod test_block_statements;
mod test_blocks;
mod test_braceless_call_in_branch;
mod test_braceless_output_calls;
mod test_branch_assigned_variables;
mod test_broadcast;
mod test_call_condition_block; // card if-exists
mod test_call_result_reflection; // card name-function
mod test_catch_binding;
mod test_cell_truth;
mod test_channels;
mod test_charged_fields;
mod test_check_assert;
mod test_check_holds;
mod test_clipboard;
mod test_code_hole_calls; // card code-hole-arm-state
mod test_code_holes; // card code-holes
mod test_colon_handlers;
mod test_condition_block;
mod test_constant_listener;
mod test_control_words;
mod test_count_word_blocks; // card thrice-block
mod test_ctrl_c_rule;
mod test_daily_timer;
mod test_data_quoting; // card data-quoting
mod test_data_string; // card data-string
mod test_database_bool_column; // card todo-app
mod test_database_busy; // card orm-sample-lock
mod test_database_dangling_key; // card orm-dangling
mod test_database_field_named; // card orm-size-field
mod test_database_filters; // card orm-filters
mod test_database_members_add; // cards orm-nested, orm-list-add
#[cfg(feature = "native")] // its one test counts native database queries: not in the browser build
mod test_database_members_query; // card orm, one-to-many as a query
mod test_database_nested_add; // card orm-nested
mod test_database_optional; // card orm-optional
mod test_database_optional_units; // card unit-field
mod test_database_relations; // card orm, step 4
mod test_database_short_names; // card orm-one-letter-column
mod test_database_tables; // card orm
mod test_database_units_everywhere; // card browser-unit
mod test_do_block;
mod test_doc_strings; // card doc-strings
mod test_each_colon;
mod test_emit_operands;
mod test_empty_block;
mod test_empty_block_binding;
mod test_empty_bodies;
mod test_empty_reset_in_if;
mod test_error_branch_kind;
mod test_error_fields; // card error-field
mod test_error_not_in_data;
mod test_error_value_return;
mod test_error_values;
mod test_error_words; // card raise-boom
mod test_event_footguns;
mod test_event_handler_lists;
mod test_event_signals;
mod test_exit_in_handler; // card key-app
mod test_exit_signal;
mod test_expect; // card expect-make
mod test_field_of_elements; // name of people with age > 20 (user 2026-10-10)
mod test_field_of_passed_list; // card field-list-passed
mod test_filter_loop_over_variable;
mod test_filter_loops;
mod test_for_forms; // cards print-comma, forms-space, chain-block
mod test_for_implicit_it;
mod test_for_it;
mod test_for_loop;
mod test_for_over_a_number;
mod test_for_semicolon_end; // card print-end
mod test_for_underscore;
mod test_for_unit_words;
mod test_frame_event; // card frame-event
mod test_fresh_names; // card cleanup-fresh-names
mod test_function_body; // card g_X_3s
mod test_function_dir; // card dir-function
mod test_function_objects; // card module-methods
mod test_glued_block_words; // card glued-loop
mod test_go_blocks;
mod test_go_heavy_map; // card go-heavy-map
mod test_go_statement; // card go-play
mod test_grouped_receivers; // card dump-text
mod test_handler_globals;
mod test_handler_unwinding; // card on-error-abort
mod test_held_key; // card held-key
mod test_help_word;
mod test_host_word_failures; // card host-word
mod test_if_call_condition;
mod test_if_empty_branch;
mod test_if_instance_branches; // card if-instance-branch
mod test_if_then_assign;
mod test_if_value_kind;
mod test_implicit_await;
mod test_is_pattern;
mod test_iteration_words;
mod test_job_lists;
mod test_key_down; // card keyboard-state
mod test_key_name_case; // key names in any case
mod test_key_names; // card key-if
mod test_library_signatures; // card reflect-library-signatures
mod test_lifted_loop_literals; // card loop-literal-list
mod test_list_without; // people without team (card people-where)
mod test_listener_removal;
mod test_listener_removal_in_functions;
mod test_loop_forms;
mod test_loop_if_value;
mod test_loop_value;
mod test_loop_value_at_run_time;
mod test_loops_in_functions;
mod test_lowercase_key_names; // card if-key
mod test_many_tasks;
mod test_map_loop_words; // card key-print
mod test_match_alternatives;
mod test_match_alternatives_hint;
mod test_match_binding_arm;
mod test_match_deep_conditions; // card deep-conditions
mod test_match_else_line;
mod test_match_guards_and_finally;
mod test_match_item_patterns; // card list-pattern
mod test_match_maps; // card map-patterns
mod test_match_negative_subject; // card match-negative-subject
mod test_match_other;
mod test_match_pattern_bindings; // card pattern-names
mod test_match_ranges;
mod test_match_tuples;
mod test_match_type_arms;
mod test_match_whole_value;
mod test_member_completion;
mod test_method_let_shadows_field; // card receiver-field-assign
mod test_mixed_range_bounds;
mod test_multiline_errors;
mod test_named_broadcast;
mod test_named_patterns; // card patterns-values
mod test_named_reflection;
mod test_not_condition_block;
mod test_object_blocks;
mod test_object_functions;
mod test_on_error_of_function;
mod test_on_set_value; // card on-set-value
mod test_one_method_form; // card one-method-form
mod test_or_error_type;
mod test_page_events;
mod test_parallel_let; // card lower-parallel
mod test_parallel_map;
mod test_parsed_code_patterns; // card tree-patterns step 7
mod test_pattern_unit_name; // card pattern-unit-name
mod test_plural_field_of_elements; // card plural-of
mod test_print_empty; // card print-error
mod test_print_empty_map;
mod test_quasi_quote; // cards data-pattern-sealed, unit-hole-name, data-splice
mod test_quoted_data_untouched; // card data-quote-lowered
mod test_raise;
mod test_reactive_ports;
mod test_reflect_inferred; // card reflect-inferred
mod test_reflection_mode; // card use-reflection
mod test_reflection_synonyms; // card reflection-synonyms
mod test_reflection_words; // card reflection
mod test_relational_arms;
mod test_rest_patterns;
mod test_run_tests; // card run-tests
mod test_run_time_block_cache;
mod test_run_time_blocks;
mod test_rust_inclusive_range;
mod test_rust_option; // card rust-option
mod test_schema_cast; // card loaded-config
mod test_schemas; // card schemas
mod test_scoped_handlers;
mod test_shared_arrays;
mod test_shared_signals;
mod test_shared_values;
mod test_signal_fields;
mod test_signal_in_loop;
mod test_signal_lists;
mod test_signal_reflection;
mod test_signal_values;
#[cfg(feature = "native")] // a page runs its timers after main, never inside main's loop (host-timers.js)
mod test_slow_handler; // card timer-handler
mod test_soft_tests;
mod test_split_kernel; // card go-split-kernel
mod test_storage;
mod test_store_values;
mod test_stored_default; // card stored-visits
mod test_stored_signals;
mod test_structural_patterns;
mod test_switch_match;
mod test_switch_no_case_value;
mod test_switch_value;
mod test_system_signals;
mod test_system_values;
#[cfg(feature = "native")] // its one test counts the table rows read natively: not in the browser build
mod test_table_batched_keys;
#[cfg(feature = "native")] // counts the rows SQLite read
mod test_table_comprehension; // orm: a filtered comprehension over a table is its query
#[cfg(feature = "native")] // counts the rows SQLite read
mod test_table_comprehension_where; // card people-where-cond
mod test_table_element_writes; // orm: people#1.age = 5 writes through
mod test_table_key_filter; // card table-key-filter
mod test_table_linear_lookups; // card orm-linear
mod test_table_method_reads;
mod test_table_paging;
mod test_table_query_loading; // orm: a filter reads only its rows
mod test_table_remove; // card table-remove
mod test_table_transactions; // card orm step 5
mod test_task_bools;
mod test_task_char; // card task-char
mod test_task_closures;
mod test_task_dir;
mod test_task_error_positions; // card runtime-errors
mod test_task_exact;
mod test_task_globals; // card error-undefined
mod test_task_list_literal;
mod test_task_name_scope;
mod test_task_parameter_shadows;
mod test_task_pool;
mod test_task_signals;
mod test_task_values;
mod test_tasks;
mod test_text_loop_value;
mod test_text_patterns; // card text-patterns
mod test_threads;
mod test_time_of_day;
mod test_times_count_once;
mod test_top_level_block;
mod test_trap_messages;
mod test_tree_find; // card tree-find
mod test_tree_replace; // card tree-replace
mod test_try_braceless_call;
mod test_try_catch_except;
mod test_try_deep;
mod test_try_division;
mod test_try_else;
mod test_try_else_print; // card try-print
mod test_try_else_value;
mod test_try_exits_and_naming;
mod test_try_finally;
mod test_try_host_failure; // card try-catch
mod test_try_index_and_float;
mod test_try_list_index;
mod test_try_named_traps;
mod test_try_stack_overflow;
mod test_type_field_fallback; // card type-non
mod test_type_word_filter_loops;
mod test_uncharged_blocks;
mod test_undo_history;
mod test_unhandled_emit_operand;
mod test_unhandled_emits;
mod test_unit_tasks; // card units-tasks
mod test_unwrap;
mod test_unwrap_message;
mod test_value_methods_are_functions; // card methods-last-n
mod test_variable_signals;
#[cfg(feature = "native")] // a WebSocket server on a local port
mod test_web_sockets;
mod test_welcome_forms;
mod test_when_signals;
mod test_while_paren_condition;
mod test_while_value;
#[cfg(feature = "native")] // the paint window's input
mod test_window_clicks; // card native-window
#[cfg(feature = "native")] // the warp binary's window opener
mod test_window_opens; // card window-loop
mod test_word_named_variables; // card codepoint-loop-variable
mod test_word_statements; // card error-beep // card rest-patterns // card member-completion // card rust-inclusive-range

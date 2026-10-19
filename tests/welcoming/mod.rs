mod test_adopted_acknowledgements;
mod test_advice_fixes; // card advise-fix
mod test_aligned_comment; // card hint-comment-floordiv
mod test_and_or_got_it; // card done-done
mod test_braceless_call_argument;
mod test_braceless_word_argument;
mod test_c_style;
mod test_cli_error_exit; // card cli-error-exit
#[cfg(all(feature = "native", unix))] // runs the warp binary
mod test_closed_pipe;
mod test_const_list_methods;
mod test_constant_shadowing;
mod test_count_shadowed; // card count-shadowed
mod test_cross_module_overload; // card cross-module-overload
mod test_discarded_error_warning; // card error-if
mod test_discarded_pure_warning;
mod test_dot_without_parens; // cards type-module, abc-chars
mod test_dot_word_same_path; // card type-module follow-up
mod test_end_and_def_blocks; // cards std-module-swallowed, end-indented, def-indented
mod test_english_operator_words;
mod test_error_excerpt;
mod test_error_positions;
#[cfg(feature = "native")] // runs the warp binary
mod test_fi_block_end; // card allow-stupid
mod test_fixits;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_footguns;
mod test_format_cli; // card beautify-cli
mod test_format_files_only; // cards fix-rewrote, fix-laid
#[cfg(feature = "native")]
// wasmtime's fuel: the browser runs without it, stopped by a timer (playground.js RUN_TIMEOUT_MS)
mod test_fuel_default; // card fuel-default
mod test_function_bindings; // card serve-var
mod test_golf_words; // cards golf-chr, golf-chars, golf-print, golf-inline
mod test_got_it_scope;
mod test_got_it_warnings;
mod test_hint_positions;
mod test_indented_body_without_colon; // cards golf-indented, golf-bare, golf-indented-while, golf-indented-swap
mod test_index_hint_numbers; // card g_YiSA
mod test_index_hint_simple;
mod test_it_shadow_warning;
mod test_item_list_cast_hint;
mod test_keys_of_a_map_parameter;
mod test_left_arrow;
mod test_let_const_changes;
mod test_list_phrases;
mod test_lone_words;
mod test_lowered_error_text;
mod test_map_broadcast_hint; // card hint-map
mod test_near_miss_words; // card hard-coded
mod test_open_range_slices; // card golf-open
mod test_own_definition_hint; // card hint-own-definition
mod test_parser_runtime_positions; // card runtime-parser
mod test_piped_answers; // card yes-fix
mod test_plain_hint_got_it; // card hints-dismissed
mod test_quiet_hints;
mod test_relaxed_pragma;
mod test_script_pragma;
mod test_shell_words; // card unix-words
mod test_slash_comment_after_value;
mod test_slash_comment_needs_space;
mod test_slash_comment_prose_operators; // card comment-after
mod test_sleep_fractions; // card playground-tour
mod test_sleep_seconds;
mod test_sleep_unit_shadow;
mod test_sleep_unit_warning;
mod test_strict_warning_statement; // card under-use
mod test_style_dont_care;
mod test_template_positions; // card template-positions
mod test_ternary_hint_got_it; // card hint-dismiss
mod test_time_limit; // card fuel-metering
mod test_undefined_in_text_sum; // card compile-path
mod test_unknown_prefix_word;
mod test_unknown_word_error;
mod test_variable_applied; // card error-local
mod test_warning_levels; // card warning-levels
mod test_warning_mode;
mod test_welcoming_ask;
mod test_welcoming_break;
mod test_welcoming_count_argument;
mod test_welcoming_elements;
mod test_welcoming_empty_push;
mod test_welcoming_globals;
mod test_welcoming_indent;
mod test_welcoming_listexpr;
mod test_welcoming_listparams;
mod test_welcoming_maps;
mod test_welcoming_parse;
mod test_welcoming_print;
mod test_welcoming_rangeblock;
mod test_welcoming_slices;
mod test_welcoming_sugar; // cards crash-show, show-crashes, crashes-broken

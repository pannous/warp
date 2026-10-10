#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_footguns;
mod test_got_it_warnings;
mod test_it_shadow_warning;
mod test_style_dont_care;
mod test_unknown_prefix_word;
mod test_lone_words;
mod test_unknown_word_error;
mod test_warning_mode;
mod test_welcoming_ask;
mod test_welcoming_break;
mod test_welcoming_count_argument;
mod test_welcoming_elements;
mod test_welcoming_empty_push;
mod test_welcoming_globals;
mod test_welcoming_indent;
mod test_indented_body_without_colon; // cards golf-indented, golf-bare, golf-indented-while, golf-indented-swap
mod test_end_and_def_blocks; // cards std-module-swallowed, end-indented, def-indented
mod test_golf_words; // cards golf-chr, golf-chars, golf-print, golf-inline
mod test_welcoming_listexpr;
mod test_welcoming_listparams;
mod test_welcoming_maps;
mod test_welcoming_parse;
mod test_welcoming_print;
mod test_welcoming_rangeblock;
mod test_welcoming_slices;
mod test_open_range_slices; // card golf-open
mod test_welcoming_sugar;
mod test_let_const_changes;
mod test_function_bindings; // card serve-var
mod test_const_list_methods;
mod test_c_style;
mod test_item_list_cast_hint;
mod test_adopted_acknowledgements;
mod test_error_positions;
mod test_error_excerpt;
mod test_fixits;
mod test_braceless_call_argument;
mod test_got_it_scope;
mod test_lowered_error_text;
mod test_quiet_hints;
mod test_sleep_unit_warning;
mod test_index_hint_simple;
mod test_index_hint_numbers; // card g_YiSA
mod test_keys_of_a_map_parameter;
mod test_english_operator_words;
mod test_list_phrases;
mod test_constant_shadowing;
mod test_left_arrow;
mod test_hint_positions;
mod test_slash_comment_after_value;
mod test_slash_comment_prose_operators; // card comment-after
mod test_sleep_unit_shadow;
mod test_undefined_in_text_sum; // card compile-path
mod test_count_shadowed; // card count-shadowed
mod test_discarded_pure_warning;
mod test_slash_comment_needs_space;
mod test_ternary_hint_got_it; // card hint-dismiss
mod test_and_or_got_it; // card done-done
mod test_plain_hint_got_it; // card hints-dismissed
#[cfg(feature = "native")] // wasmtime's fuel: the browser runs without it, stopped by a timer (playground.js RUN_TIMEOUT_MS)
mod test_fuel_default; // card fuel-default
mod test_cli_error_exit; // card cli-error-exit
mod test_advice_fixes; // card advise-fix

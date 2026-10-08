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
mod test_welcoming_listexpr;
mod test_welcoming_listparams;
mod test_welcoming_maps;
mod test_welcoming_parse;
mod test_welcoming_print;
mod test_welcoming_rangeblock;
mod test_welcoming_slices;
mod test_welcoming_sugar;
mod test_let_const_changes;
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
mod test_sleep_unit_shadow;
mod test_undefined_in_text_sum; // card compile-path
mod test_count_shadowed; // card count-shadowed
mod test_discarded_pure_warning;
mod test_fuel_default; // card fuel-default

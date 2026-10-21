mod test_add_to_text;
mod test_ascii_index_costs;
mod test_backtick_templates;
mod test_brace_hole; // card brace-hole
mod test_byte_at_type_error; // card traps-wasm-trap
mod test_byte_loop_inline; // card loop-calls
mod test_case_table;
mod test_cast_to_string;
mod test_char_parameter_type; // chars-codepoints follow-up
mod test_character_arithmetic;
mod test_character_comparison;
mod test_chars_codepoints; // card chars-codepoints
mod test_chr;
mod test_codepoint_bytes;
mod test_codepoint_counts; // card codepoints-traps
mod test_constant_expression_text;
mod test_count_in;
mod test_count_method;
mod test_count_of_split; // card count-split
mod test_counts_in_warp; // card count-rs
mod test_declared_text_one_character;
mod test_error_as_text;
mod test_for_each_char;
mod test_guillemet_strings;
mod test_interpolated_source_forms;
mod test_interpolation;
mod test_large_text_literals; // card self-serializer
mod test_library_unicode;
mod test_map_text;
mod test_nested_list_text;
mod test_one_character_argument;
mod test_print_all_characters;
mod test_print_arguments;
mod test_print_gives_nothing;
mod test_print_juxtaposed;
mod test_print_list_shapes; // card print-shows
mod test_print_lists;
mod test_print_runtime_number;
mod test_print_spaced_variable; // card print-prints
mod test_print_type_error;
mod test_print_walk;
mod test_put_runtime_values;
mod test_quote_output;
mod test_quoted_computed_container_text;
mod test_quoted_container_texts;
mod test_repeat_typed; // card repeat-int
mod test_reverse_characters; // card reverse-characters
mod test_rope_keys; // card suite-json-sample
mod test_runtime_kind_texts;
mod test_runtime_text_as_int;
mod test_scan_by_index; // card self-text-index
mod test_selector_prepositions; // card of-in-chain
mod test_shalom_units;
mod test_sort_of_a_call; // card count-sorted
mod test_split_default; // card split-without
mod test_split_element_kind;
mod test_str_computed; // card str-inline
mod test_string;
mod test_terminal_escapes;
mod test_text_as_float;
mod test_text_builders; // card self-byte-buffer
mod test_text_building;
mod test_text_bytes;
mod test_text_cache_lru; // card text-cache-lru
mod test_text_casts;
mod test_text_concat;
mod test_text_cursor;
mod test_text_functions;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_text_getters;
#[cfg(feature = "native")] // reads a file
mod test_text_parameter_copy; // card self-lexer
mod test_text_plus_anything; // card print-oldest
mod test_text_plus_float_call;
mod test_text_plus_printable; // card print-oldest
mod test_text_repeat;
mod test_text_runtime;
mod test_text_search;
mod test_text_slices; // cards text-slice, long-text
mod test_text_type_words; // card cleanup-closed-lists
mod test_text_unit_lists; // decision text-unit-plurals
mod test_text_variable_assignment;
mod test_trim;
mod test_unicode_escape;
mod test_utf8_bytes; // card consists-characters

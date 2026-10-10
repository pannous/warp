mod test_cast_to_string;
mod test_character_comparison;
mod test_constant_expression_text;
mod test_declared_text_one_character;
mod test_one_character_argument;
mod test_interpolation;
mod test_interpolated_source_forms;
mod test_brace_hole; // card brace-hole
mod test_backtick_templates;
mod test_library_unicode;
mod test_print_arguments;
mod test_print_type_error;
mod test_quote_output;
mod test_runtime_text_as_int;
mod test_split_element_kind;
mod test_string;
mod test_text_bytes;
mod test_text_casts;
mod test_text_concat;
mod test_text_functions;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_text_getters;
mod test_text_repeat;
mod test_text_runtime;
mod test_text_variable_assignment;
mod test_text_as_float;
mod test_text_plus_float_call;
mod test_trim;
mod test_text_search;
mod test_nested_list_text;
mod test_chr;
mod test_unicode_escape;
mod test_terminal_escapes;
mod test_put_runtime_values;
mod test_print_juxtaposed;
mod test_print_lists;
mod test_runtime_kind_texts;
mod test_text_building;
mod test_map_text;
mod test_print_walk;
mod test_count_in;
mod test_character_arithmetic;
mod test_print_all_characters;
mod test_add_to_text;
mod test_print_gives_nothing;
mod test_guillemet_strings;
mod test_quoted_container_texts;
mod test_print_runtime_number;
mod test_utf8_bytes;
mod test_case_table;
mod test_error_as_text;
mod test_count_method;
mod test_codepoint_bytes;
mod test_repeat_typed; // card repeat-int
mod test_split_default; // card split-without
mod test_str_computed; // card str-inline
mod test_text_plus_anything; // card print-oldest
mod test_text_plus_printable; // card print-oldest
mod test_text_type_words; // card cleanup-closed-lists

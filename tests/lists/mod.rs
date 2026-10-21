mod test_aliased_changes; // card aliased-list
mod test_append_words;
mod test_auto_linear_floats; // card auto-linear-floats
mod test_bare_list_assignment;
mod test_byte_size;
mod test_calls_in_blocks;
mod test_captured_call_lists;
mod test_comprehension_beside_own_push; // card prefix-count-field
mod test_concat_declared;
mod test_concat_keeps_arrays; // card runtime-speed
mod test_count_after_function; // card print-people
mod test_counting_word_operands; // card count-xs
mod test_counting_word_variables;
mod test_counting_words;
mod test_decimal_list_elements;
mod test_deep_index_assignment;
mod test_delete_list; // card delete-list
mod test_down_and_letter_ranges;
mod test_dynamic_range;
mod test_element_wise_lists;
mod test_element_wise_scalar;
mod test_element_wise_vectors;
mod test_empty_items;
mod test_empty_list_argument;
mod test_empty_list_concat; // card null-typed
mod test_empty_list_count;
mod test_empty_start; // card start: `t = []` appends, ø + text fails
mod test_empty_start_plus; // card local-fails: `t = []; t + "a"`, the bracket hint of `xs += [3]`
mod test_field_count;
mod test_filter_element_scope;
mod test_find_any_all;
mod test_first_items; // card first-first
mod test_float_array_writes;
mod test_for_where; // card for-where
mod test_fractional_count;
mod test_function_maps; // card map-locals
mod test_generated_name_hints;
mod test_global_comprehension;
mod test_global_typed_lists;
mod test_hash_range_slices;
mod test_hash_range_warning;
mod test_held_element_compare;
mod test_index_assignment;
mod test_index_out_of_range;
mod test_inserted_word; // card insert-at-warp
pub(crate) mod test_int_map; // card int-map
mod test_join_unknown_parts; // card warp-profile: `out += part` of an `any` part grows in place
mod test_key_lookup;
mod test_key_subscript_hint;
mod test_keys_of_a_value; // card keys-value
mod test_lazy_range;
mod test_library_word_operands; // card derive-hard
mod test_linear_append;
mod test_linear_arrays;
mod test_linear_hint; // card linear-hint
mod test_list_arithmetic;
mod test_list_compound_add;
mod test_list_held_last; // card profile-stage-programs
mod test_list_identity; // card w0-list-writes
mod test_list_item_conversions; // card room-conversions
mod test_list_literal_sign; // decision list-sign
mod test_list_methods; // card list-methods
mod test_list_number_comparison;
mod test_list_of_objects;
mod test_list_parameters;
mod test_list_passed_to_function_value; // card fixnow-builtin-function-add
mod test_list_plus_number;
mod test_list_plus_parts; // card def-ps: `out = out + p` of parts of unknown kind
mod test_list_retype;
mod test_list_times_list;
mod test_list_truthiness;
mod test_lists;
mod test_lists_read_by_turns; // several long lists read by turns: a read cursor each
mod test_long_list_copies;
mod test_long_list_result;
mod test_map_field_named_like_word; // card map-field
mod test_map_get_kind;
mod test_map_index;
mod test_map_inside_loop; // card map-inside
mod test_map_literal_number_keys; // card map-literal
mod test_map_parameter_keys;
mod test_map_starts;
mod test_map_typed;
mod test_map_variables;
mod test_min_max;
mod test_min_max_lists;
mod test_nested_comprehension; // card double-for-comprehension
mod test_nested_index_assignment;
mod test_nested_maps;
mod test_node_lists;
mod test_none_word;
mod test_number_keys;
mod test_number_list_elements; // card winamp-like
mod test_numbered_fields;
mod test_object_list_walk; // orm identity map: index walks of object lists
mod test_one_entry_map_parts; // card age-name
mod test_one_entry_map_text;
mod test_pair_values;
mod test_phrase_in_call; // card phrase-in-call
mod test_picked_linear; // card compiler-picks-dot
mod test_pop;
mod test_python_collections; // card python-stdlib
mod test_range_arguments;
mod test_range_in_structure;
mod test_range_returns;
mod test_range_values;
mod test_remove_and_index_of;
mod test_self_sum_in_place; // card out-list: `out = out + x` of an unshared list grows it in place
mod test_set_past_the_end;
mod test_signed_operand_list;
mod test_size_count;
mod test_size_property_word;
mod test_sort_long;
mod test_sorted_by_phrase; // card sorted-by-phrase
mod test_swizzles; // card compile-shaders, decision S1
mod test_tuple_items_of_lists;
mod test_tuples;
mod test_type_word_items;
mod test_typed_list_insert; // card w0-list-writes
mod test_typed_lists;
mod test_unit_named_variable_in; // card chars-chars
mod test_unit_word_text_in; // card in-text-word
mod test_unpacking;
mod test_unpacking_declared;
mod test_unpacking_into_existing;
mod test_where_filter;
mod test_word_slices;
mod test_worklist_count; // card list-count-grows

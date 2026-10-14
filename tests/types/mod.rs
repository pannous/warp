mod test_array_types;
mod test_inline_unions;
// card union-types
mod test_class_unions;
// card class-field
mod test_any_field_text;
mod test_any_into_annotated;
mod test_any_into_annotated_types;
mod test_any_place; // card yes-stores
mod test_as_number_variable; // card number-variable
mod test_be_definitions;
mod test_binary_int; // card binary-int
mod test_block_data_paths;
mod test_bool_assign;
mod test_bool_conversion;
mod test_bool_literal_value;
mod test_bool_return; // card bool-return
mod test_bool_type;
mod test_cast_bugs;
mod test_chained_assignments;
mod test_checked_call_kinds;
mod test_checked_results;
mod test_class_aliases;
mod test_class_cases_ported;
mod test_class_destructuring;
mod test_class_extends;
mod test_class_field_names;
mod test_class_field_type_phrases;
mod test_class_fields_checked;
mod test_class_forms_ported;
mod test_class_interfaces;
mod test_class_json;
mod test_class_method_changes;
mod test_class_methods;
mod test_class_mixins;
mod test_class_operators;
mod test_class_properties;
mod test_class_witnesses;
mod test_codepoint_digit; // card codepoint-digit
mod test_collection_declarations;
mod test_conformance_claims;
mod test_construction_checks;
mod test_construction_defaults;
mod test_constructor_vs_data;
mod test_copies;
mod test_cycle_mark_depth; // card cycle-mark-depth
mod test_cyclic_objects; // card orm: objects pointing to each other
mod test_data;
mod test_data_run_now; // card data-run-now
mod test_data_stays_data; // card data-bare
mod test_date_arithmetic_at_run_time; // card run-time-dates, stage B2
mod test_dates_at_run_time; // card runtime-dates
mod test_declaration_without_value;
mod test_declaration_zero_value;
mod test_declared_int_of_float_expression;
mod test_declared_list_marks;
mod test_dispatch;
mod test_element_field_writes;
mod test_empty_literal_kind;
mod test_empty_named_key; // card object-empty-field
mod test_empty_type_aliases;
mod test_enum_cases;
mod test_enum_parameter_type;
mod test_enums_ported;
mod test_extension_methods;
mod test_field_named_like_builtin; // cards field-named-size, orm-members-word
mod test_field_named_like_library_word; // card field-before-library-word
mod test_file_declarations;
mod test_filled_map_types; // card number-keyed
mod test_flags;
mod test_float_fields;
mod test_floats_add_exact; // card floats-add-exact
mod test_fraction_text;
mod test_generic_classes;
mod test_generic_types;
mod test_hex_int; // card hex-printing
mod test_hex_word; // hex x is hex(x), literals are 0x…
mod test_index_position; // card index-position
mod test_instance_arguments;
mod test_instance_final_text; // card instance-final
mod test_instance_size; // card size-instance
mod test_int_accepts; // card int-accepts
mod test_int_declaration_stays_whole;
mod test_interface_signatures;
mod test_is_declaration;
mod test_is_teaches_be;
mod test_iterable;
mod test_kept_wiki_forms;
mod test_list_annotations;
mod test_list_element_types;
mod test_list_test_of_a_map;
mod test_loop_item_into_annotated;
mod test_loop_text_arithmetic;
mod test_map_runtime_kind; // card map-runtime
mod test_map_type_tests; // card map-answers
mod test_match_exhaustive; // card match-exhaustive
mod test_match_exhaustive_assigned; // card exhaustive-assigned
mod test_match_static_variant;
mod test_maybe_declaration; // card maybe-int
mod test_method_bodies; // card method-bodies
mod test_method_result_text; // card instance-result
mod test_multi_word_class; // card multi-word-class
mod test_mutating_methods;
mod test_nested_type_calls; // text hex 255 is text(hex(255))
mod test_number_converts; // card number-converts
mod test_number_field; // card number-field
mod test_number_keeps_exact; // card number-keeps-exact
mod test_operation_type; // card parse-type
mod test_optional_casts;
mod test_optional_list_items;
mod test_optional_number_fields;
mod test_param_named_like_global; // card param-named-like-global
mod test_parameter_text; // card param-text
mod test_passed_list_identity; // card xs-xs
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_person_struct;
mod test_plain_enum; // card plain-enum
mod test_printable;
mod test_real_references;
mod test_real_text;
mod test_records;
mod test_records_classes;
mod test_recursive_fields;
mod test_repeated_type_parameters;
mod test_return_as_hint; // return-as: the -> hint at the definition
mod test_returned_instances; // card units-dynamic
mod test_returned_list_alias; // card identity-copies-list
mod test_runtime_text_ratio;
mod test_same_identity;
mod test_self_typed_field; // card empty-field
mod test_shared_instances;
mod test_shared_lists; // card shared-lists
mod test_sized_array_values;
mod test_spaced_construction;
mod test_static_result; // card static-int
mod test_struct_field_of_constructor;
mod test_struct_instances;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_struct_types;
mod test_sum_types;
mod test_symbolism_object; // card symbolism-object
mod test_tagged_objects;
mod test_tagged_unions; // card tagged-unions
mod test_text_as_bool; // card yes-print
mod test_text_as_number; // card units-dynamic
mod test_text_of_text; // text(text(5)) is "5"
mod test_time_kinds_are_types; // card anchor-time
mod test_time_programs_run; // card run-time-dates, stage B4
mod test_time_type_names;
mod test_time_values_at_run_time; // card run-time-dates
mod test_time_zones_at_run_time; // card run-time-dates, stage B3
mod test_trait_constraints;
mod test_trait_default_methods;
mod test_trait_equals_membership;
mod test_trait_runtime_dispatch;
mod test_traits;
mod test_tuple_values;
mod test_type_alias_fields;
mod test_type_calls; // card types-constructor
mod test_type_colon_block;
mod test_type_compared_with_text; // card type-text-compare
#[cfg(feature = "native")] // runs lean and lake
mod test_type_model;
mod test_type_name_definition_forms;
mod test_type_name_matching;
mod test_type_of;
mod test_type_of_consistency; // card type-of-consistency
mod test_type_of_node_values;
mod test_type_of_real_variable;
mod test_type_static;
mod test_type_test_is_only;
mod test_type_tests;
mod test_type_tests_anywhere;
mod test_type_upgrading;
mod test_type_word_scopes;
mod test_type_word_user_function;
mod test_type_words;
mod test_typed_arrays;
mod test_typed_list_elements; // card typed-list-elements
mod test_types;
mod test_types_scope;
mod test_union_returns;
mod test_union_spellings;
mod test_unknown_conversion; // card unknown-word
mod test_unknown_type; // card unknown-type
mod test_upcast_fields;
mod test_user_counting_word; // card user-defined-size
mod test_value_constructor;
mod test_variant_equality; // card variant-left
mod test_variant_payloads;
mod test_widened_list_views; // card p215-user // generic functions: methods by the first argument's class // card lossy-return: a fraction reaching an int result is an error

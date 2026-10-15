mod test_as_prefixed_name;
mod test_attributes;
mod test_bare_operator_block;
mod test_big_literals; // card big-literals
mod test_comment_hides_bracket;
mod test_comments;
mod test_dollar_names;
mod test_element_wise_phrase; // card cleanup-parser-element-wise
mod test_emoji_atoms;
mod test_empty_argument;
mod test_end_word;
mod test_entity_table;
mod test_escaped_quote_slashes; // card self-serializer
mod test_format_layout; // card warp-format
mod test_glued_method_block;
mod test_glued_pair_value;
mod test_glyph_aliases;
mod test_guard_comment; // card else-comment-swallows
mod test_if_condition_words; // card parser-if-stops
mod test_inline_comments;
mod test_json;
mod test_juxtaposition;
mod test_meta;
mod test_meta_attributes;
mod test_newline_precedence;
mod test_number_glyphs;
mod test_one_line_statements;
mod test_operator_word_keys; // card abs-fails
mod test_parse_long_sources;
mod test_parser;
mod test_parser_sugar;
mod test_path_literal; // card play-mozart
mod test_references;
mod test_scheme_relative_url;
#[cfg(feature = "native")]
mod test_self_differential; // card self-differential
#[cfg(feature = "native")]
mod test_self_lexer; // card self-lexer
#[cfg(feature = "native")]
mod test_self_parser; // card self-parser
#[cfg(feature = "native")]
mod test_self_serializer; // card self-serializer
mod test_semicolon_square;
mod test_spaced_children;
mod test_spaced_optional_type;
mod test_spaced_required_fields;
mod test_statement_phrases;
mod test_statement_sequence;
mod test_statement_terminators;
mod test_statements_after_type;
mod test_superscript_signs;
mod test_surface_syntax;
mod test_symbol_hyphen;
mod test_time_unit_keys;
mod test_times_phrase; // card self-parser
mod test_trailing_annotation;
mod test_unary_plus;
mod test_uniscript_entities;
mod test_uniscript_hyphen_names; // card uniscript-literal-hyphen
mod test_url_literal; // card add-http
mod test_warp_format;
mod test_warp_position;
mod test_wisp_def_params;
mod test_wisp_malformed;
mod test_wisp_roundtrip;
mod test_xml; // card pannous-com

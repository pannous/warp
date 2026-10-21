mod test_and_after_comparison; // card print-sq
mod test_as_prefixed_name;
mod test_attributes;
mod test_bare_operator_block;
mod test_beautify_synonyms; // card warp-format-strict
pub(crate) mod test_big_literals; // card big-literals
mod test_braceless_range_argument; // card braceless-argument
mod test_comment_hides_bracket;
mod test_comments;
mod test_dollar_names;
mod test_element_wise_phrase; // card cleanup-parser-element-wise
mod test_emoji_atoms;
mod test_empty_argument;
mod test_end_word;
mod test_entity_table;
mod test_escaped_quote_slashes; // card self-serializer
mod test_explicit_symbols;
mod test_format_layout; // card warp-format
mod test_format_multiline_text; // card lint-text-braces
mod test_format_semicolons; // card linter-should
mod test_glued_method_block;
mod test_glued_pair_value;
mod test_glyph_aliases;
mod test_guard_comment; // card else-comment-swallows
mod test_home_paths; // card unix-words
mod test_if_condition_words; // card parser-if-stops
mod test_inline_comments;
mod test_juxtaposition;
mod test_meta;
mod test_meta_attributes;
mod test_named_argument_lambda; // card holder-says
mod test_newline_precedence;
mod test_number_glyphs;
mod test_one_line_statements;
mod test_operator_aliases;
mod test_operator_word_keys; // card abs-fails
mod test_parameter_scope; // card path-parameter-scope
mod test_parse_long_definitions;
mod test_parse_long_sources;
mod test_parser;
mod test_parser_sugar;
mod test_path_after_function; // card name-after
mod test_path_literal; // card play-mozart
mod test_references;
mod test_rust_items; // card rust-items
mod test_rust_macros;
mod test_rust_paths;
mod test_scheme_relative_url;
#[cfg(feature = "native")]
mod test_self_analyzer; // card inferred-kinds
#[cfg(feature = "native")]
mod test_self_emitter; // card emits-wasm
#[cfg(feature = "native")]
mod test_self_emitter_times; // card time-primitives
#[cfg(feature = "native")]
mod test_self_exact_pow; // card exact-pow-warp
#[cfg(feature = "native")]
mod test_self_exact_rounding; // card exact-rounding-warp
#[cfg(feature = "native")]
mod test_self_float_as_exact; // card float-as-exact-warp
#[cfg(feature = "native")]
mod test_self_float_text; // card float-text-warp
#[cfg(feature = "native")]
mod test_self_hosted_analysis; // card End-end
#[cfg(feature = "native")]
mod test_self_hosted_cache; // card End-end
#[cfg(feature = "native")]
mod test_self_hosted_library; // card shared-warp-build
#[cfg(feature = "native")]
mod test_self_pure; // card pure-runs
#[cfg(feature = "native")]
mod test_self_sort; // card sort-warp
#[cfg(feature = "native")] // a lib/compiler file on disk
mod test_self_time_arithmetic; // card time-arithmetic-warp
#[cfg(feature = "native")]
mod test_self_time_text; // card time-text-warp
mod test_semicolon_square;
mod test_spaced_children;
mod test_spaced_optional_type;
mod test_spaced_required_fields;
mod test_statement_phrases;
mod test_statements_after_type;
mod test_superscript_signs;
mod test_surface_syntax;
mod test_symbol_hyphen;
mod test_time_unit_keys;
mod test_times_phrase; // card self-parser
mod test_unary_plus;
mod test_uniscript_entities;
mod test_uniscript_hyphen_names; // card uniscript-literal-hyphen
mod test_url_literal; // card add-http
mod test_warp_format;
mod test_wisp_def_params;
mod test_wisp_malformed;
mod test_wisp_roundtrip;
mod test_xml; // card pannous-com // decision operator-alias, card standalone-math-units

# Printing `of` values

`property of value` is a lookup that produces one value. Printing it must use the ordinary value printer: for example,
`print name of person` prints the whole name, and `print effects of f` prints the whole effect name.

Item-by-item printing is reserved for the explicit selector spellings with `in` or `from`, such as
`print words in text`. The shared `is_selector_preposition` check keeps those walks aligned with the selector lookup
rule without treating every `of` lookup as an iterable.

Regression coverage is in `tests/functions/test_print_effects_walked.rs`.

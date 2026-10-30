# typed-parse plan

Add an outcome test for typed JSON parsing, then derive validation from the declared class using the existing library rewrite mechanism. Reject mismatched types and missing or unknown fields without coercion; preserve optional and defaulted fields. Verify the test and smoke before landing.

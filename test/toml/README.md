# toml-test conformance for lib/toml (not part of smoke)

`toml-test/` is the TOML 1.0 case set of https://github.com/toml-lang/toml-test at commit
96fd088448ec0f4d9f061c730ed7854399c46faf (2026-10-27): the files listed in `files-toml-1.0.0` (208 valid with their
.json, 509 invalid), 927 files, 135 KB, with its LICENSE (MIT).

    WARP_LIB=$PWD/lib test/toml/toml_test.py [--fails valid|invalid|valid/string|all]

Valid cases: parse_toml's value typed as toml-test's JSON must equal the .json (datetimes compared as times, so .6 is
.600); invalid cases must raise. 9 invalid cases are not UTF-8 and can't be a warp text: counted as skipped.

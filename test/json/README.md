# JSONTestSuite

`JSONTestSuite/`: test_parsing/ (318 files) and test_transform/ (22) of https://github.com/nst/JSONTestSuite, commit
1ef36fa01286573e846ac449e8683f8833c5b26a (2024-11-22), MIT (`JSONTestSuite/LICENSE`).

`json_test_suite.py` runs every test_parsing file through lib/json's parse_json: y_ must parse to python's json.loads
value, n_ must be refused, i_ is listed (`--i`). Every y_ file and every test_transform file also round trips,
parse_json(to_json(parse_json(x))) == parse_json(x). Files that aren't UTF-8 are skipped, counted by kind. Cases run
in chunks; a chunk that dies or hangs is run case by case, with a 5 s limit each. `--depth` finds the deepest [
nesting parsed; `--fails y|n|i|rt` lists failures. Outside smoke; run from the repo root.

lib/json reads JSON5 on purpose (its header comment). An n_ case accepted for one of these extensions counts as by
design, by extension: comments, single quotes, trailing commas, bare keys (numeric ones too), unknown escapes read as
the character (\x is x), raw control characters in strings, JSON5 numbers (+1, .5, 1.). Lone surrogates (\uD800)
are refused on purpose, as warp texts are UTF-8, so those test_transform cases don't round trip.

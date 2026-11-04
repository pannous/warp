# html conformance data

- `entities.json`: WHATWG's named character references, https://html.spec.whatwg.org/entities.json
  (2231 entries, last-modified 2025-11-12, fetched 2026-11-04). `gen_html_entities.py` turns it into
  `lib/html_entities.warp` (run from the repo root; not a baked file). `entities_test.py` runs every name through
  `html_unescape`: with its `;`, and the 106 legacy names without it (at the end, and before `#`). lib has a single
  text-context decoding, with no attribute context, so there is no attribute group.
- `html5lib-tokenizer/`: tokenizer tests from https://github.com/html5lib/html5lib-tests (commit
  c777c408b61078ea2eb4acefc2535f54dbc8b28a, 2026-10-01, MIT: `LICENSE`). Left out: namedEntities.test (1.1 MB;
  entities.json covers it), test3.test (352 KB, generated, tags), and unicodeChars*.test (no character references).
- `tokenizer_test.py`: lib has no tokenizer, only character-reference decoding (`html_unescape`). Selection rule:
  a case is run when its state is Data or RCDATA, its output is only Character tokens, its input has no CR or NUL
  (input stream preprocessing), no `</` and, in Data state, no `<`, and it has no lone surrogates. The other cases
  are counted as n/a, with the reason. Parse-error lists are ignored.

Both runners print counts by group, take `--fails GROUP`, and are not part of smoke.

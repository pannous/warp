# csv conformance for lib/csv (not part of smoke)

- `csv-spectrum/`: csvs/ and json/ of https://github.com/max-mapper/csv-spectrum at commit
  d30e80f8b99d2eecb3778f1d7b9ed1cb425502ec (2024-07-04), with its readme.md and package.json (license BSD-2-Clause;
  the repository has no LICENSE file), 4017 bytes. location_coordinates is skipped: its json is no list and its phone
  number is not the csv's.
- `rfc4180/`: RFC 4180 edge cases written for this card, 557 bytes: quotes, doubled quotes, embedded LF and CRLF, CRLF,
  LF and CR line endings, empty and empty quoted fields, a trailing newline or none, a blank line, spaces, UTF-8, a
  UTF-8 BOM. Expected rows are python's csv.reader's, except the BOM (dropped, as Excel writes it) and a blank line
  (a record of one empty field, as RFC 4180's grammar reads it; python gives []).

    WARP_LIB=$PWD/lib test/csv/csv_test.py [--fails spectrum|rfc4180|roundtrip|all]

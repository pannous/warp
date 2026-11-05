# RFC 3339 date-times

`gen_rfc3339_cases.py` writes `rfc3339_cases.tsv`: each input and the milliseconds since 1970 python's datetime
gives it (or ERR), under the strict RFC 3339 grammar: YYYY-MM-DD, then T, t or a space, hh:mm:ss, an optional
fraction of any length (cut to milliseconds), and Z, z or ±hh:mm (-00:00 is the same instant as Z). 24:00, 23:60
and the leap second 23:59:60 are refused, as datetime refuses them and lib keeps no leap seconds. Week dates
(2026-W45-3), ordinal dates (2026-309) and other ISO 8601 forms aren't supported, so they're refused.
`rfc3339_test.py` runs each case through lib/time's parse_date and round trips the accepted ones through
Instant(ms).text(), counts by group, `--fails GROUP`. Outside smoke; run from the repo root.

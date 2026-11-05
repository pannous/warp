# mustache spec

`mustache-spec/`: the official specs from https://github.com/mustache/spec, commit
e8ec001db7f594521e773c34866aca2b5d6b0037 (2026-04-27), the JSON form of each file, MIT (`mustache-spec/LICENSE`).
`mustache_spec.py` runs every case through lib/template's `mustache(template, data, partials)` and prints counts by
file: the six required ones (comments, delimiters, interpolation, inverted, partials, sections) and the optional
~dynamic-names, ~inheritance, ~lambdas. A ~lambdas case's lambda is code for other languages (no warp), so those
cases are n/a. `--fails FILE` lists a file's failures. Outside smoke; run from the repo root.

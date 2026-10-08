#!/bin/bash
# fix-parse: rebuild an export of origin/main plus this fix's hunks only, then run the probe snippets
cd "$(dirname "$0")/../.." || exit 1
export RUST_TEST_THREADS=2 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/opt/cargo/warp-parse
mkdir -p data/parse scratch
git diff -U2 src/warp_parser.rs src/analyzer.rs > data/parse/all.diff
python3 probes/parse/select_hunks.py data/parse/all.diff data/parse/mine.diff
rm -rf scratch/parse-export && mkdir scratch/parse-export
git archive origin/main | tar -x -C scratch/parse-export
cp tests/welcoming/test_welcoming_parse.rs scratch/parse-export/tests/
cd scratch/parse-export || exit 1
GIT_CEILING_DIRECTORIES="$(cd .. && pwd)" git apply --recount ../../data/parse/mine.diff || exit 1
cargo --offline build --all-features 2>&1 | grep -E "^(error|warning)" -A6 | head -30
for f in ../../probes/parse/*.warp; do printf '%s\t' "$f"; timeout 200 "$CARGO_TARGET_DIR/debug/warp" "$f" 2>&1 | tail -1; done

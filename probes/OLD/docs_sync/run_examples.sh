#!/bin/bash
# Runs each doc example of a file with main's warp: one per line, `code ⇒ value` or `code ⇒ !error part`.
# A value must be the whole result (» value, texts with or without quotes); an expected error part must appear in an Error.
# usage: probes/docs_sync/run_examples.sh <file>   (WARP=<binary> to try another build)
WARP=${WARP:-/Users/me/dev/bin/warp}
while IFS= read -r line; do
  [[ -z "$line" || "$line" == \#* ]] && continue
  code="${line% ⇒ *}"; expected="${line##* ⇒ }"
  output=$(WARP_NO_WINDOW=1 timeout 20 "$WARP" eval "$code" </dev/null 2>&1)
  result=$(grep '^» ' <<<"$output" | tail -1)
  if [[ "$expected" == !* ]]; then
    [[ "$output" == *Error* && "$output" == *"${expected#!}"* ]] && pass=1 || pass=0
  else
    [[ "$result" == "» $expected" || "$result" == "» \"$expected\"" ]] && pass=1 || pass=0
  fi
  if ((pass)); then echo "PASS  $code"; else echo "FAIL  $code   expected: $expected   got: $(head -2 <<<"$output" | tr '\n' ' ')"; fi
done < "$1"

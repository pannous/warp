#!/usr/bin/env bash
# A native call passes a 24-byte struct by value (notes/native_host.md). The library is compiled by this machine's
# cc, so the same source checks x86-64 (the struct on the stack) and ARM64 (a pointer to a copy).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
cc -shared -fPIC -o "$stage/libbyvalue.so" "$checkout/test/ffi/byvalue.c" || exit 1
cat > "$stage/call.warp" << EOF
import (calloc, set_field_i64) from "c"
sum24(n: i64, block: s24): i64 from "$stage/libbyvalue.so"
pair24(left: handle, right: handle, block: s24): i64 from "$stage/libbyvalue.so"
p = calloc(1, 24)
set_field_i64(p, 0, 10)
set_field_i64(p, 8, 32)
set_field_i64(p, 16, 1)
[sum24(1, p), pair24(p, 0, p)]
EOF
got=$("$warp" "$stage/call.warp" 2>/dev/null)
if [ "$got" = "[44, 44]" ]; then echo "✓ struct by value"; exit 0; fi
echo "✗ struct by value: got $got, expected [44, 44]"
exit 1

#!/usr/bin/env bash
# a runtime error raised in a host module (lib/host/native_host) reaches the user as its sentence, never as a bare
# wasm trap (card native-host). Run it with bash test/native/test_host_errors.sh; the warp under test is WARP
source "$(dirname "$0")/checks.sh"
said=$("$warp" --mute 'use gpu_passes
fields = gpu_fields(0, 4, ["dye"])
1' 2>&1)
check "a host module's raise says its sentence" "$(grep -o 'gpu_fields needs a positive width and height' <<<"$said")" "gpu_fields needs a positive width and height"
check "and no bare wasm trap" "$(grep -c 'wasm trap' <<<"$said")" "0"
exit $((failures > 0))

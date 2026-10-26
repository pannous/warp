// xxHash 0.8.2 (the header-only copy zstd ships) as one core WebAssembly module, built by `sh xxhash.sh`; xxhash.h beside
// the module declares what warp may call (tests/modules/test_wasm_modules.rs)
#define XXH_NAMESPACE
#define XXH_STATIC_LINKING_ONLY
#define XXH_IMPLEMENTATION
#include "xxhash.h"

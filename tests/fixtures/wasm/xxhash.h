// the C interface of xxhash.wasm (built by xxhash.sh), xxhash.h's own prototypes with its typedefs spelled out
// (XXH32_hash_t is unsigned int, XXH64_hash_t unsigned long long): the input crosses as a text with its length, the
// state is the module's pointer, a number
unsigned int XXH_versionNumber(void);
unsigned int XXH32(const void *input, size_t length, unsigned int seed);
unsigned long long XXH64(const void *input, size_t length, unsigned long long seed);
struct XXH32_state_s *XXH32_createState(void);
int XXH32_freeState(struct XXH32_state_s *statePtr);
int XXH32_reset(struct XXH32_state_s *statePtr, unsigned int seed);
int XXH32_update(struct XXH32_state_s *statePtr, const void *input, size_t length);
unsigned int XXH32_digest(const struct XXH32_state_s *statePtr);

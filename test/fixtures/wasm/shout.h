// the C interface of shout.wasm: its texts cross as char *, copied into its memory through its exported malloc
// (notes/wasm_modules.md)
char *shout(const char *text);
int letters(const char *text);

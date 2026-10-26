// the C interface of libc_text.wasm, as libc's own headers declare it: size_t and long are 32 bits there, the module's
// own types win; char * crosses as text
size_t strlen(const char *s);
char *strstr(const char *haystack, const char *needle);
char *strchr(const char *s, int c);
char *strrchr(const char *s, int c);
int toupper(int c);
int tolower(int c);
int atoi(const char *s);
long atol(const char *s);
int abs(int n);
long strtol(const char *nptr, char **endptr, int base);

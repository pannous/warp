// the C interface of zlib.wasm (built by zlib.sh), zlib.h's own prototypes with its typedefs spelled out (uLong is
// unsigned long, Bytef unsigned char): const pointers carry the program's text, unsigned results stay unsigned
const char *zlibVersion(void);
unsigned long crc32(unsigned long crc, const unsigned char *buf, unsigned int len);
unsigned long adler32(unsigned long adler, const unsigned char *buf, unsigned int len);
unsigned long compressBound(unsigned long sourceLen);

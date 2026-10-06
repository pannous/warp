/* glibc's ctype.h explains its tables in prose that reads like a call:
   rather than `unsigned char's because tolower (EOF) must be EOF, which
   doesn't fit into an `unsigned char'. */
extern int tolower (int __c) __THROW;

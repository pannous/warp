// The system a native build calls (lib/native/wasi.c, foreign.c, and the C functions a program imports by name): POSIX,
// on Windows answered by the platform layer lib/native/windows.c, so no other C file and no warp code asks which
// system it runs on (card windows-build)
#ifndef WARP_SYSTEM_H
#define WARP_SYSTEM_H
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#ifdef _WIN32
#include "windows.c"
#else
#include <dlfcn.h>
#include <sys/ioctl.h>
#endif
#endif

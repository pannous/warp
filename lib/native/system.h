// The system a native build calls (lib/native/wasi.c, foreign.c, and the C functions a program imports by name): POSIX,
// on Windows answered by the platform layer lib/native/windows.c, so no other C file and no warp code asks which
// system it runs on (card windows-build)
#ifndef WARP_SYSTEM_H
#define WARP_SYSTEM_H
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <signal.h>
#include <stdbool.h>
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
#include <spawn.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
// the process's environment, which a child posix_spawnp starts gets (glibc declares it only under _GNU_SOURCE)
extern char** environ;
#endif
#endif

// The POSIX calls of a native build on Windows, answered over Win32 and the C runtime (included by lib/native/system.h):
// lib/native/wasi.c, foreign.c and the programs' imports read the same as on macOS and Linux. A POSIX call Windows
// has no answer for is missing here on purpose: a program calling it stops loudly ("no C function fork …")
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <direct.h>
#include <io.h>
#include <psapi.h>

// ## the calls lib/native/*.c make itself

// a folder opened as a file: Windows's _open never opens one, so asking for one fails as POSIX's O_DIRECTORY does on a
// file
#define O_DIRECTORY 0
// the trap signal Windows has no number for: signal() refuses it, the crash lines go without it
#define SIGTRAP 5

// strndup by a name of its own: some mingw headers declare it, some don't
#define strndup posix_strndup
static char* posix_strndup(const char* text, size_t most) {
	size_t length = strnlen(text, most);
	char* copy = malloc(length + 1);
	if (!copy) return NULL;
	memcpy(copy, text, length);
	copy[length] = 0;
	return copy;
}

// the terminal's size (foreign.c terminal_size): the console window's rows and columns
struct winsize { unsigned short ws_row, ws_col, ws_xpixel, ws_ypixel; };
enum { TIOCGWINSZ = 0x5413 };
static int ioctl(int fd, unsigned long request, struct winsize* size) {
	CONSOLE_SCREEN_BUFFER_INFO info;
	if (request != TIOCGWINSZ || !GetConsoleScreenBufferInfo((HANDLE)_get_osfhandle(fd), &info)) {
		errno = ENOTTY;
		return -1;
	}
	size->ws_row = (unsigned short)(info.srWindow.Bottom - info.srWindow.Top + 1);
	size->ws_col = (unsigned short)(info.srWindow.Right - info.srWindow.Left + 1);
	return 0;
}

// sigaction as signal(): a handler and nothing else (Ctrl+C in the console, foreign.c interrupts_engine)
typedef int signal_set;
struct sigaction { void (*sa_handler)(int); int sa_flags; signal_set sa_mask; };
enum { SA_RESTART = 0x10000000 };
static int sigemptyset(signal_set* set) {
	*set = 0;
	return 0;
}
static int sigaction(int number, const struct sigaction* action, struct sigaction* old) {
	(void)old;
	return signal(number, action->sa_handler) == SIG_ERR ? -1 : 0;
}

// files and the standard streams in bytes, as POSIX has them: the C runtime's text mode would write "\r\n" for "\n"
__attribute__((constructor)) static void files_in_bytes(void) {
	_set_fmode(_O_BINARY);
	for (int fd = 0; fd <= 2; fd++) _setmode(fd, _O_BINARY);
}

// ## the calls programs and the native warp import by name (`readlink(…): i32 from "c"`)
// answered here before any module's: Windows's C runtime lacks them, or has them by another layout (its timespec's
// nanoseconds are 32 bits, its POSIX names carry an underscore)

// a path as POSIX writes it: forward slashes, which Windows takes as well as backslashes and warp's path words split
// on, and on the current drive no drive letter, "/a/b" as Windows reads it too: the folder the native warp preopens as
// "/" for its modules (core/native/driver.warp) is that drive's root, so the module's WASI reaches the path (another
// drive's "C:/x" it does not yet: card windows-drives)
static void posix_path(char* path) {
	for (char* c = path; *c; c++) if (*c == '\\') *c = '/';
	if (path[0] && path[1] == ':' && path[2] == '/' && (path[0] | 0x20) - 'a' + 1 == _getdrive()) memmove(path, path + 2, strlen(path + 2) + 1);
}
// the one link a Windows program can read: Linux's /proc/self/exe, this executable
static const char SELF_LINK[] = "/proc/self/exe";
static ssize_t posix_readlink(const char* path, char* buffer, size_t size) {
	if (strcmp(path, SELF_LINK)) {
		errno = EINVAL;
		return -1;
	}
	DWORD length = GetModuleFileNameA(NULL, buffer, (DWORD)size);
	if (length == 0 || length >= size) {
		errno = ENAMETOOLONG;
		return -1;
	}
	buffer[length] = 0;
	posix_path(buffer);
	return (ssize_t)strlen(buffer);
}
// a path made absolute, NULL for one that names nothing; resolved NULL: a copy the caller frees
static char* posix_realpath(const char* path, char* resolved) {
	if (GetFileAttributesA(path) == INVALID_FILE_ATTRIBUTES) {
		errno = ENOENT;
		return NULL;
	}
	char* full = _fullpath(resolved, path, MAX_PATH);
	if (full) posix_path(full);
	return full;
}
static char* posix_getcwd(char* buffer, size_t size) {
	char* folder = _getcwd(buffer, (int)size);
	if (folder) posix_path(folder);
	return folder;
}
// the system's name, the first field: "Windows"
enum { NAME_BYTES = 65 };
struct utsname { char sysname[NAME_BYTES], nodename[NAME_BYTES], release[NAME_BYTES], version[NAME_BYTES], machine[NAME_BYTES]; };
static const char SYSTEM_NAME[] = "Windows";
static int posix_uname(struct utsname* names) {
	memset(names, 0, sizeof *names);
	snprintf(names->sysname, NAME_BYTES, "%s", SYSTEM_NAME);
	DWORD size = NAME_BYTES;
	GetComputerNameA(names->nodename, &size);
	snprintf(names->machine, NAME_BYTES, "%s", sizeof(void*) == 8 ? "x86_64" : "x86");
	return 0;
}
static int posix_setenv(const char* name, const char* value, int overwrite) {
	if (!overwrite && getenv(name)) return 0;
	return _putenv_s(name, value) ? -1 : 0;
}
static int posix_unsetenv(const char* name) { return _putenv_s(name, "") ? -1 : 0; }
// POSIX's 48-bit generator, its numbers the same as on macOS and Linux for a seed
static uint64_t random48 = 0x1234ABCD330EULL;
enum { RANDOM48_BITS = 48 };
static void posix_srand48(long seed) { random48 = ((uint64_t)(uint32_t)seed << 16) | 0x330E; }
static double posix_drand48(void) {
	random48 = (0x5DEECE66DULL * random48 + 0xB) & ((1ULL << RANDOM48_BITS) - 1);
	return (double)random48 / (double)(1ULL << RANDOM48_BITS);
}
// a line of a stream into *line, grown as needed, its newline kept; -1 at the end
static ssize_t posix_getline(char** line, size_t* capacity, FILE* stream) {
	size_t length = 0;
	int c;
	while ((c = fgetc(stream)) != EOF) {
		if (length + 2 > *capacity) {
			size_t grown = *capacity ? 2 * *capacity : 128;
			char* bigger = realloc(*line, grown);
			if (!bigger) return -1;
			*line = bigger;
			*capacity = grown;
		}
		(*line)[length++] = (char)c;
		if (c == '\n') break;
	}
	if (length == 0) return -1;
	(*line)[length] = 0;
	return (ssize_t)length;
}
// a time as 64-bit seconds and 64-bit nanoseconds, the timespec of 64-bit macOS and Linux
struct posix_time { int64_t seconds, nanoseconds; };
enum { NANOSECONDS = 1000000000, FILE_TIME_TICK_NANOSECONDS = 100 };
// the seconds from 1601, where a FILETIME counts from, to 1970
static const int64_t UNIX_EPOCH_SECONDS = 11644473600LL;
// the wall clock, whichever clock is asked: warp asks only CLOCK_REALTIME (core/native/driver.warp milliseconds_now)
static int posix_clock_gettime(int clock, struct posix_time* time) {
	(void)clock;
	FILETIME now;
	GetSystemTimePreciseAsFileTime(&now);
	int64_t ticks = (int64_t)(((uint64_t)now.dwHighDateTime << 32) | now.dwLowDateTime);
	int64_t ticks_a_second = NANOSECONDS / FILE_TIME_TICK_NANOSECONDS;
	time->seconds = ticks / ticks_a_second - UNIX_EPOCH_SECONDS;
	time->nanoseconds = ticks % ticks_a_second * FILE_TIME_TICK_NANOSECONDS;
	return 0;
}
static int posix_nanosleep(const struct posix_time* wanted, struct posix_time* left) {
	(void)left;
	Sleep((DWORD)(wanted->seconds * 1000 + wanted->nanoseconds / 1000000));
	return 0;
}
static int posix_usleep(unsigned microseconds) {
	Sleep(microseconds / 1000);
	return 0;
}
static struct tm* posix_localtime_r(const time_t* seconds, struct tm* fields) { return localtime_s(fields, seconds) ? NULL : fields; }
// the load average Windows keeps none of: -1, as POSIX says for a system that cannot tell
static int posix_getloadavg(double* loads, int count) {
	(void)loads, (void)count;
	return -1;
}
// flock's operations as LockFileEx's: shared or exclusive, waiting or not, the whole file
enum { LOCK_SH = 1, LOCK_EX = 2, LOCK_NB = 4, LOCK_UN = 8 };
static int posix_flock(int fd, int operation) {
	HANDLE file = (HANDLE)_get_osfhandle(fd);
	OVERLAPPED whole = {0};
	if (operation & LOCK_UN) return UnlockFileEx(file, 0, MAXDWORD, MAXDWORD, &whole) ? 0 : -1;
	DWORD flags = ((operation & LOCK_EX) ? LOCKFILE_EXCLUSIVE_LOCK : 0) | ((operation & LOCK_NB) ? LOCKFILE_FAIL_IMMEDIATELY : 0);
	if (LockFileEx(file, flags, 0, MAXDWORD, MAXDWORD, &whole)) return 0;
	errno = EWOULDBLOCK;
	return -1;
}
#define POSIX_CALL(name) {#name, (void*)posix_##name}
static const struct { const char* name; void* function; } POSIX_CALLS[] = {
	POSIX_CALL(readlink), POSIX_CALL(realpath), POSIX_CALL(getcwd), POSIX_CALL(uname), POSIX_CALL(setenv),
	POSIX_CALL(unsetenv), POSIX_CALL(srand48), POSIX_CALL(drand48), POSIX_CALL(getline), POSIX_CALL(clock_gettime),
	POSIX_CALL(nanosleep), POSIX_CALL(usleep), POSIX_CALL(localtime_r), POSIX_CALL(getloadavg), POSIX_CALL(flock),
};

// ## dlopen and dlsym, which find them

// Windows names the POSIX calls of its C runtime with a leading underscore (_open, _getpid, _unlink), the plain names
// being the linker's aliases, which a lookup by name never sees
static const char CRT_PREFIX[] = "_";
// dlopen's flags: LoadLibrary takes none, a library's symbols are always its own
enum { RTLD_LAZY = 1, RTLD_NOW = 2, RTLD_GLOBAL = 0x100 };
// the process, as dlopen(NULL) names it: dlsym asks the calls above, then each module loaded, the executable first
static char process_handle;
static void* dlopen(const char* path, int flags) {
	(void)flags;
	return path ? (void*)LoadLibraryA(path) : (void*)&process_handle;
}
static void* module_symbol(HMODULE module, const char* name) {
	char runtime_name[256];
	void* found = (void*)GetProcAddress(module, name);
	if (found || strlen(name) + sizeof CRT_PREFIX > sizeof runtime_name) return found;
	snprintf(runtime_name, sizeof runtime_name, "%s%s", CRT_PREFIX, name);
	return (void*)GetProcAddress(module, runtime_name);
}
static void* dlsym(void* handle, const char* name) {
	if (handle != &process_handle) return module_symbol((HMODULE)handle, name);
	for (size_t i = 0; i < sizeof POSIX_CALLS / sizeof *POSIX_CALLS; i++)
		if (!strcmp(name, POSIX_CALLS[i].name)) return POSIX_CALLS[i].function;
	HMODULE modules[1024];
	DWORD bytes;
	if (!K32EnumProcessModules(GetCurrentProcess(), modules, sizeof modules, &bytes)) return NULL;
	for (DWORD i = 0; i < bytes / sizeof *modules && i < sizeof modules / sizeof *modules; i++) {
		void* found = module_symbol(modules[i], name);
		if (found) return found;
	}
	return NULL;
}

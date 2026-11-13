// The POSIX calls of a native build on Windows, answered over Win32 and the C runtime (included by lib/native/system.h):
// lib/native/wasi.c, foreign.c and the programs' imports read the same as on macOS and Linux. A POSIX call Windows
// has no answer for is missing here on purpose: a program calling it stops loudly ("no C function fork …")
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <io.h>
#include <psapi.h>

// Windows names the POSIX calls of its C runtime with a leading underscore (_open, _getpid, _unlink), the plain names
// being the linker's aliases, which a lookup by name never sees
static const char CRT_PREFIX[] = "_";
// dlopen's flags: LoadLibrary takes none, a library's symbols are always its own
enum { RTLD_LAZY = 1, RTLD_NOW = 2, RTLD_GLOBAL = 0x100 };
// the process, as dlopen(NULL) names it: dlsym asks each module loaded, the executable first
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
	HMODULE modules[1024];
	DWORD bytes;
	if (!K32EnumProcessModules(GetCurrentProcess(), modules, sizeof modules, &bytes)) return NULL;
	for (DWORD i = 0; i < bytes / sizeof *modules && i < sizeof modules / sizeof *modules; i++) {
		void* found = module_symbol(modules[i], name);
		if (found) return found;
	}
	return NULL;
}

// a folder opened as a file: Windows's _open never opens one, so asking for one fails as POSIX's O_DIRECTORY does on a
// file
#define O_DIRECTORY 0
// the trap signal Windows has no number for: signal() refuses it, the crash lines go without it
#define SIGTRAP 5

char* strndup(const char* text, size_t most) {
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

// The POSIX calls programs and the native warp import by name (`readlink(…): i32 from "c"`), which the executable
// exports (-Wl,--export-all-symbols, extensions/commands/build.warp): paths come back with forward slashes, which
// Windows takes as well as backslashes and warp's path words split on
static void forward_slashes(char* path) {
	for (char* c = path; *c; c++) if (*c == '\\') *c = '/';
}
// the one link a Windows program can read: Linux's /proc/self/exe, this executable
static const char SELF_LINK[] = "/proc/self/exe";
ssize_t readlink(const char* path, char* buffer, size_t size) {
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
	forward_slashes(buffer);
	return (ssize_t)length;
}
// a path made absolute, NULL for one that names nothing; resolved NULL: a copy the caller frees
char* realpath(const char* path, char* resolved) {
	if (GetFileAttributesA(path) == INVALID_FILE_ATTRIBUTES) {
		errno = ENOENT;
		return NULL;
	}
	char* full = _fullpath(resolved, path, MAX_PATH);
	if (full) forward_slashes(full);
	return full;
}
// the system's name, the first field: "Windows"
enum { NAME_BYTES = 65 };
struct utsname { char sysname[NAME_BYTES], nodename[NAME_BYTES], release[NAME_BYTES], version[NAME_BYTES], machine[NAME_BYTES]; };
static const char SYSTEM_NAME[] = "Windows";
int uname(struct utsname* names) {
	memset(names, 0, sizeof *names);
	snprintf(names->sysname, NAME_BYTES, "%s", SYSTEM_NAME);
	DWORD size = NAME_BYTES;
	GetComputerNameA(names->nodename, &size);
	snprintf(names->machine, NAME_BYTES, "%s", sizeof(void*) == 8 ? "x86_64" : "x86");
	return 0;
}
int setenv(const char* name, const char* value, int overwrite) {
	if (!overwrite && getenv(name)) return 0;
	return _putenv_s(name, value) ? -1 : 0;
}
int unsetenv(const char* name) { return _putenv_s(name, "") ? -1 : 0; }
// POSIX's 48-bit generator, its numbers the same as on macOS and Linux for a seed
static uint64_t random48 = 0x1234ABCD330EULL;
enum { RANDOM48_BITS = 48 };
void srand48(long seed) { random48 = ((uint64_t)(uint32_t)seed << 16) | 0x330E; }
double drand48(void) {
	random48 = (0x5DEECE66DULL * random48 + 0xB) & ((1ULL << RANDOM48_BITS) - 1);
	return (double)random48 / (double)(1ULL << RANDOM48_BITS);
}
// a line of a stream into *line, grown as needed, its newline kept; -1 at the end
ssize_t getline(char** line, size_t* capacity, FILE* stream) {
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

// The POSIX calls of a native build on Windows, answered over Win32 and the C runtime (included by lib/native/system.h):
// lib/native/wasi.c, foreign.c and the programs' imports read the same as on macOS and Linux. A POSIX call Windows
// has no answer for is missing here on purpose: a program calling it stops loudly ("no C function fork …")
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <ctype.h>
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

// a path as POSIX writes it, Git Bash's (MSYS's) way: forward slashes, a drive as a folder of the root ("D:\a\b" is
// "/d/a/b"), so a path warp shows works in the sh lines it runs (popen) and in WASI, where the native warp's root
// preopen is each drive's under that name (posix_wasi_config_preopen_dir); and back for the C runtime
static void posix_path(char* path) {
	for (char* c = path; *c; c++) if (*c == '\\') *c = '/';
	if (isalpha((unsigned char)path[0]) && path[1] == ':') {
		path[1] = (char)tolower((unsigned char)path[0]);
		path[0] = '/';
	}
}
static int is_drive_path(const char* path) { return path[0] == '/' && isalpha((unsigned char)path[1]) && (path[2] == '/' || !path[2]); }
// the path as Windows takes it, in the buffer when it changes: "/d/a/b" is "D:/a/b", "/d" "D:/"
static const char* windows_path(const char* path, char buffer[MAX_PATH]) {
	if (!path || !is_drive_path(path)) return path;
	snprintf(buffer, MAX_PATH, "%c:%s", toupper((unsigned char)path[1]), path[2] ? path + 2 : "/");
	return buffer;
}
// the C runtime's calls of one path, the path given as Windows takes it
#define ON_WINDOWS_PATH(result, name, call, ...) static result posix_##name(const char* path, ##__VA_ARGS__) { char buffer[MAX_PATH]; path = windows_path(path, buffer); return call; }
ON_WINDOWS_PATH(int, access, _access(path, mode), int mode)
ON_WINDOWS_PATH(int, chdir, _chdir(path))
ON_WINDOWS_PATH(int, creat, _creat(path, mode), int mode)
ON_WINDOWS_PATH(FILE*, fopen, fopen(path, mode), const char* mode)
ON_WINDOWS_PATH(int, mkdir, _mkdir(path), int mode)
ON_WINDOWS_PATH(int, rmdir, _rmdir(path))
ON_WINDOWS_PATH(int, unlink, _unlink(path))
ON_WINDOWS_PATH(int, open_path, _open(path, flags, mode), int flags, int mode)
static int posix_rename(const char* from, const char* to) {
	char from_buffer[MAX_PATH], to_buffer[MAX_PATH];
	return rename(windows_path(from, from_buffer), windows_path(to, to_buffer));
}
// lib/native/wasi.c's own open
#define open posix_open_path
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
	char buffer[MAX_PATH];
	path = windows_path(path, buffer);
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
// ### processes: pipes, posix_spawnp, waitpid, popen; Windows starts a process whole (CreateProcess), no fork

enum { PIPE_BYTES = 65536 };
static int posix_pipe(int ends[2]) { return _pipe(ends, PIPE_BYTES, _O_BINARY); }
// a spawn's file actions, as many as warp gives (core/native/driver.warp words_run: two dup2s and a close), in the
// bytes warp allocates for posix_spawn_file_actions_t (SPAWN_ACTIONS_BYTES)
enum { MOST_ACTIONS = 8, ACTION_DUP2 = 1, ACTION_CLOSE = 2, STANDARD_STREAMS = 3 };
struct spawn_actions { int count; struct { int kind, from, to; } actions[MOST_ACTIONS]; };
static int action_added(struct spawn_actions* actions, int kind, int from, int to) {
	if (actions->count == MOST_ACTIONS) return ENOMEM;
	actions->actions[actions->count].kind = kind;
	actions->actions[actions->count].from = from;
	actions->actions[actions->count].to = to;
	actions->count++;
	return 0;
}
static int posix_spawn_file_actions_init(struct spawn_actions* actions) {
	actions->count = 0;
	return 0;
}
static int posix_spawn_file_actions_destroy(struct spawn_actions* actions) { return posix_spawn_file_actions_init(actions); }
static int posix_spawn_file_actions_adddup2(struct spawn_actions* actions, int from, int to) { return action_added(actions, ACTION_DUP2, from, to); }
// (a handle the child should not hold: Windows hands a child only the standard handles it is given)
static int posix_spawn_file_actions_addclose(struct spawn_actions* actions, int fd) { return action_added(actions, ACTION_CLOSE, fd, fd); }

// the processes started and not yet waited for, by id
enum { MOST_CHILDREN = 64 };
static struct { DWORD id; HANDLE process; } children[MOST_CHILDREN];
static void child_kept(DWORD id, HANDLE process) {
	for (int i = 0; i < MOST_CHILDREN; i++)
		if (!children[i].process) {
			children[i].id = id;
			children[i].process = process;
			return;
		}
	CloseHandle(process); // (more children at once than this: never waited for, its status lost)
}
// an argument on Windows's one command line, as the C runtime splits it back: quoted, its quotes and the backslashes
// before them escaped
static size_t argument_quoted(char* line, const char* word) {
	size_t at = 0;
	line[at++] = '"';
	for (const char* c = word;; c++) {
		size_t backslashes = 0;
		while (*c == '\\') backslashes++, c++;
		if (!*c || *c == '"') {
			size_t doubled = backslashes * 2 + (*c == '"');
			while (doubled--) line[at++] = '\\';
			if (!*c) break;
			line[at++] = '"';
		} else {
			while (backslashes--) line[at++] = '\\';
			line[at++] = *c;
		}
	}
	line[at++] = '"';
	return at;
}
static char* command_line(char* const words[]) {
	size_t room = 1;
	for (int i = 0; words[i]; i++) room += 2 * strlen(words[i]) + 3;
	char* line = malloc(room);
	size_t at = 0;
	char program[MAX_PATH];
	for (int i = 0; words[i]; i++) {
		if (i) line[at++] = ' ';
		at += argument_quoted(line + at, i ? words[i] : windows_path(words[i], program));
	}
	line[at] = 0;
	return line;
}
// the child's standard handles, the parent's unless an action gives one, each inheritable
static HANDLE standard_handle(const struct spawn_actions* actions, int stream) {
	static const DWORD STANDARD[STANDARD_STREAMS] = {STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE};
	HANDLE handle = GetStdHandle(STANDARD[stream]);
	for (int i = 0; actions && i < actions->count; i++)
		if (actions->actions[i].kind == ACTION_DUP2 && actions->actions[i].to == stream) handle = (HANDLE)_get_osfhandle(actions->actions[i].from);
	SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT);
	return handle;
}
// the file found on PATH as CreateProcess finds it (".exe" added); the environment is this process's
static int posix_spawnp(int* child, const char* file, const struct spawn_actions* actions, const void* attributes, char* const words[], char* const environment[]) {
	(void)file, (void)attributes, (void)environment;
	STARTUPINFOA startup = {.cb = sizeof startup, .dwFlags = STARTF_USESTDHANDLES};
	startup.hStdInput = standard_handle(actions, 0);
	startup.hStdOutput = standard_handle(actions, 1);
	startup.hStdError = standard_handle(actions, 2);
	PROCESS_INFORMATION started;
	char* line = command_line(words);
	BOOL ran = CreateProcessA(NULL, line, NULL, NULL, TRUE, 0, NULL, NULL, &startup, &started);
	free(line);
	if (!ran) return ENOENT;
	CloseHandle(started.hThread);
	child_kept(started.dwProcessId, started.hProcess);
	*child = (int)started.dwProcessId;
	return 0;
}
// a child's end: its exit code as POSIX's status (code << 8); -1 waits for any child, WNOHANG for none that runs on
enum { WNOHANG = 1, EXIT_CODE_SHIFT = 8 };
static int posix_waitpid(int id, int* status, int options) {
	HANDLE waited[MOST_CHILDREN];
	int places[MOST_CHILDREN], count = 0;
	for (int i = 0; i < MOST_CHILDREN; i++)
		if (children[i].process && (id == -1 || children[i].id == (DWORD)id)) places[count] = i, waited[count++] = children[i].process;
	if (count == 0) {
		errno = ECHILD;
		return -1;
	}
	DWORD ended = WaitForMultipleObjects((DWORD)count, waited, FALSE, (options & WNOHANG) ? 0 : INFINITE);
	if (ended == WAIT_TIMEOUT) return 0;
	if (ended >= WAIT_OBJECT_0 + (DWORD)count) return -1;
	int place = places[ended - WAIT_OBJECT_0];
	DWORD code = 0;
	GetExitCodeProcess(children[place].process, &code);
	if (status) *status = (int)((code & 0xFF) << EXIT_CODE_SHIFT);
	CloseHandle(children[place].process);
	children[place].process = NULL;
	return (int)children[place].id;
}
// a sh line run with its output (mode "r") or input ("w") a stream: by the POSIX sh on PATH (Git for Windows'), as
// macOS and Linux run /bin/sh; none there, NULL (warp says "cannot run sh for …")
enum { MOST_STREAMS = 16 };
static struct { FILE* stream; int child; } streams[MOST_STREAMS];
static FILE* posix_popen(const char* line, const char* mode) {
	char sh[MAX_PATH];
	if (!SearchPathA(NULL, "sh.exe", NULL, MAX_PATH, sh, NULL)) {
		errno = ENOENT;
		return NULL;
	}
	int ends[2];
	if (posix_pipe(ends)) return NULL;
	int reading = mode[0] == 'r';
	struct spawn_actions actions = {0};
	action_added(&actions, ACTION_DUP2, ends[reading ? 1 : 0], reading ? 1 : 0);
	char* words[] = {sh, "-c", (char*)line, NULL};
	int child;
	int failed = posix_spawnp(&child, sh, &actions, NULL, words, NULL);
	close(ends[reading ? 1 : 0]);
	if (failed) {
		close(ends[reading ? 0 : 1]);
		errno = failed;
		return NULL;
	}
	FILE* stream = _fdopen(ends[reading ? 0 : 1], reading ? "rb" : "wb");
	for (int i = 0; i < MOST_STREAMS; i++)
		if (!streams[i].stream) {
			streams[i].stream = stream;
			streams[i].child = child;
			break;
		}
	return stream;
}
static int posix_pclose(FILE* stream) {
	int child = -1, status = -1;
	for (int i = 0; i < MOST_STREAMS; i++)
		if (streams[i].stream == stream) child = streams[i].child, streams[i].stream = NULL;
	fclose(stream);
	if (child < 0 || posix_waitpid(child, &status, 0) < 0) return -1;
	return status;
}

// the root a module's WASI reaches (core/native/driver.warp preopens "/" as "/"): each drive under its POSIX name,
// "C:/" as "/c", wasmtime's own preopen called for each
static void* modules_symbol(const char* name);
typedef bool (*preopen_t)(void* config, const char* host, const char* guest, bool mutable);
static bool wasi_config_preopen_dir(void* config, const char* host, const char* guest, bool mutable) {
	static preopen_t preopened;
	if (!preopened) preopened = (preopen_t)modules_symbol("wasi_config_preopen_dir");
	if (strcmp(host, "/") || strcmp(guest, "/")) return preopened(config, host, guest, mutable);
	bool any = false;
	DWORD drives = GetLogicalDrives();
	for (int letter = 0; letter < 26; letter++) {
		if (!(drives & (1u << letter))) continue;
		char drive[] = {(char)('A' + letter), ':', '/', 0}, name[] = {'/', (char)('a' + letter), 0};
		any |= preopened(config, drive, name, mutable);
	}
	return any;
}

#define POSIX_CALL(name) {#name, (void*)posix_##name}
// (POSIX's own posix_ calls, under their names)
#define POSIX_NAMED(name) {#name, (void*)name}
static const struct { const char* name; void* function; } POSIX_CALLS[] = {
	POSIX_CALL(readlink), POSIX_CALL(realpath), POSIX_CALL(getcwd), POSIX_CALL(uname), POSIX_CALL(setenv),
	POSIX_CALL(unsetenv), POSIX_CALL(srand48), POSIX_CALL(drand48), POSIX_CALL(getline), POSIX_CALL(clock_gettime),
	POSIX_CALL(nanosleep), POSIX_CALL(usleep), POSIX_CALL(localtime_r), POSIX_CALL(getloadavg), POSIX_CALL(flock),
	POSIX_CALL(pipe), POSIX_NAMED(posix_spawn_file_actions_init), POSIX_NAMED(posix_spawn_file_actions_destroy),
	POSIX_NAMED(posix_spawn_file_actions_adddup2), POSIX_NAMED(posix_spawn_file_actions_addclose), POSIX_NAMED(posix_spawnp),
	POSIX_CALL(waitpid), POSIX_CALL(popen), POSIX_CALL(pclose), POSIX_CALL(access), POSIX_CALL(chdir), POSIX_CALL(creat),
	POSIX_CALL(fopen), POSIX_CALL(mkdir), POSIX_CALL(rmdir), POSIX_CALL(unlink), POSIX_CALL(rename),
	POSIX_NAMED(wasi_config_preopen_dir),
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
	char buffer[MAX_PATH];
	return path ? (void*)LoadLibraryA(windows_path(path, buffer)) : (void*)&process_handle;
}
static void* module_symbol(HMODULE module, const char* name) {
	char runtime_name[256];
	void* found = (void*)GetProcAddress(module, name);
	if (found || strlen(name) + sizeof CRT_PREFIX > sizeof runtime_name) return found;
	snprintf(runtime_name, sizeof runtime_name, "%s%s", CRT_PREFIX, name);
	return (void*)GetProcAddress(module, runtime_name);
}
// a symbol of a module loaded, the executable first
static void* modules_symbol(const char* name) {
	HMODULE modules[1024];
	DWORD bytes;
	if (!K32EnumProcessModules(GetCurrentProcess(), modules, sizeof modules, &bytes)) return NULL;
	for (DWORD i = 0; i < bytes / sizeof *modules && i < sizeof modules / sizeof *modules; i++) {
		void* found = module_symbol(modules[i], name);
		if (found) return found;
	}
	return NULL;
}
static void* dlsym(void* handle, const char* name) {
	if (handle != &process_handle) return module_symbol((HMODULE)handle, name);
	for (size_t i = 0; i < sizeof POSIX_CALLS / sizeof *POSIX_CALLS; i++)
		if (!strcmp(name, POSIX_CALLS[i].name)) return POSIX_CALLS[i].function;
	return modules_symbol(name);
}

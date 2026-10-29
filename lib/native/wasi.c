// The WASI preview 1 calls of a primitive program answered by libc in a native build (core/commands/commands.warp build_native,
// notes/primitive.md "Native build"), and the executable's main. Included behind the header wasm2c wrote for the module
// "program" and the definitions of instantiate, which hands wasm2c the instance of each module the program imports, and
// run_main, which calls the program's main and prints what it gives (core/extensions/primitive.warp native_main).
#include <fcntl.h>
#include <signal.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "wasm-rt-impl.h"

// WASI's numbers: open flags, rights, seek origins, the error of a failed call
enum { OPEN_CREATE = 1, OPEN_DIRECTORY = 2, OPEN_EXCLUSIVE = 4, OPEN_TRUNCATE = 8 };
enum { RIGHT_READ = 1 << 1, RIGHT_WRITE = 1 << 6 };
enum { SEEK_FROM_START = 0, SEEK_FROM_HERE = 1 };
enum { WASI_FAILED = 29 }; // EIO

struct w2c_wasi__snapshot__preview1 {
	w2c_program* program;
	wasm_rt_memory_t* memory;
	int argc;
	char** argv;
};

// the WASI of the program running, for a callback a C library makes into it (foreign.c warp_host_callback)
static struct w2c_wasi__snapshot__preview1* running_wasi;

void instantiate(w2c_program* program, struct w2c_wasi__snapshot__preview1* wasi);
void run_main(w2c_program* program);

static uint8_t* at(struct w2c_wasi__snapshot__preview1* wasi, uint32_t address) { return wasi->memory->data + address; }
static uint32_t load32(struct w2c_wasi__snapshot__preview1* wasi, uint32_t address) {
	uint32_t value;
	memcpy(&value, at(wasi, address), sizeof value);
	return value;
}
static void store32(struct w2c_wasi__snapshot__preview1* wasi, uint32_t address, uint32_t value) { memcpy(at(wasi, address), &value, sizeof value); }
static void store64(struct w2c_wasi__snapshot__preview1* wasi, uint32_t address, uint64_t value) { memcpy(at(wasi, address), &value, sizeof value); }

// fd_write and fd_read: each (address, length) piece in turn, the bytes moved stored at `moved`
static uint32_t moved_pieces(struct w2c_wasi__snapshot__preview1* wasi, uint32_t fd, uint32_t pieces, uint32_t count, uint32_t moved, int writing) {
	uint32_t total = 0;
	for (uint32_t i = 0; i < count; i++) {
		uint8_t* bytes = at(wasi, load32(wasi, pieces + 8 * i));
		uint32_t length = load32(wasi, pieces + 8 * i + 4);
		ssize_t done = writing ? write((int)fd, bytes, length) : read((int)fd, bytes, length);
		if (done < 0) return WASI_FAILED;
		total += (uint32_t)done;
	}
	store32(wasi, moved, total);
	return 0;
}
uint32_t w2c_wasi__snapshot__preview1_fd_write(struct w2c_wasi__snapshot__preview1* wasi, uint32_t fd, uint32_t pieces, uint32_t count, uint32_t written) {
	return moved_pieces(wasi, fd, pieces, count, written, 1);
}
uint32_t w2c_wasi__snapshot__preview1_fd_read(struct w2c_wasi__snapshot__preview1* wasi, uint32_t fd, uint32_t pieces, uint32_t count, uint32_t read) {
	return moved_pieces(wasi, fd, pieces, count, read, 0);
}
uint32_t w2c_wasi__snapshot__preview1_fd_seek(struct w2c_wasi__snapshot__preview1* wasi, uint32_t fd, uint64_t offset, uint32_t whence, uint32_t position) {
	int origin = whence == SEEK_FROM_START ? SEEK_SET : whence == SEEK_FROM_HERE ? SEEK_CUR : SEEK_END;
	off_t reached = lseek((int)fd, (off_t)offset, origin);
	if (reached < 0) return WASI_FAILED;
	store64(wasi, position, (uint64_t)reached);
	return 0;
}
uint32_t w2c_wasi__snapshot__preview1_fd_close(struct w2c_wasi__snapshot__preview1* wasi, uint32_t fd) {
	(void)wasi;
	return close((int)fd) == 0 ? 0 : WASI_FAILED;
}
// the path is the program's folder's, as the first folder a WASI host opens for it
uint32_t w2c_wasi__snapshot__preview1_path_open(struct w2c_wasi__snapshot__preview1* wasi, uint32_t folder, uint32_t lookup, uint32_t path, uint32_t length,
	uint32_t open_flags, uint64_t rights, uint64_t inherited, uint32_t flags, uint32_t opened) {
	(void)folder, (void)lookup, (void)inherited, (void)flags;
	char* name = strndup((char*)at(wasi, path), length);
	int access = (rights & RIGHT_WRITE) ? ((rights & RIGHT_READ) ? O_RDWR : O_WRONLY) : O_RDONLY;
	int how = access | ((open_flags & OPEN_CREATE) ? O_CREAT : 0) | ((open_flags & OPEN_TRUNCATE) ? O_TRUNC : 0) |
		((open_flags & OPEN_EXCLUSIVE) ? O_EXCL : 0) | ((open_flags & OPEN_DIRECTORY) ? O_DIRECTORY : 0);
	int fd = open(name, how, 0644);
	free(name);
	if (fd < 0) return WASI_FAILED;
	store32(wasi, opened, (uint32_t)fd);
	return 0;
}
uint32_t w2c_wasi__snapshot__preview1_args_sizes_get(struct w2c_wasi__snapshot__preview1* wasi, uint32_t count, uint32_t size) {
	uint32_t bytes = 0;
	for (int i = 0; i < wasi->argc; i++) bytes += (uint32_t)strlen(wasi->argv[i]) + 1;
	store32(wasi, count, (uint32_t)wasi->argc);
	store32(wasi, size, bytes);
	return 0;
}
uint32_t w2c_wasi__snapshot__preview1_args_get(struct w2c_wasi__snapshot__preview1* wasi, uint32_t arguments, uint32_t buffer) {
	for (int i = 0; i < wasi->argc; i++) {
		uint32_t length = (uint32_t)strlen(wasi->argv[i]) + 1;
		store32(wasi, arguments + 4 * i, buffer);
		memcpy(at(wasi, buffer), wasi->argv[i], length);
		buffer += length;
	}
	return 0;
}
void w2c_wasi__snapshot__preview1_proc_exit(struct w2c_wasi__snapshot__preview1* wasi, uint32_t code) {
	(void)wasi;
	exit((int)code);
}

// A run that crashes says so in one line, never a bare "abort": what crashed it (a trap of the program, which wasm2c's
// handler also makes of a fault in C code under it: a library's memory unmapped beneath the run), what the executable
// was running (its own arguments, or those of the module the native warp's driver runs), then the process ends with
// that signal's status
static const int CRASH_SIGNALS[] = {SIGABRT, SIGTRAP, SIGILL};
static char activity[1024];
static void running_with(char** arguments, int count) {
	size_t used = (size_t)snprintf(activity, sizeof activity, "running");
	for (int i = 0; i < count && used < sizeof activity; i++) used += (size_t)snprintf(activity + used, sizeof activity - used, " %s", arguments[i]);
}
// a module's run beginning (core/native/driver.warp run), named by its WASI arguments for the line a crash prints: what
// ran before it, which crash_context_restored gives back once a nested run (a host word's: parse_code) is over
char* crash_context_running(char** arguments, int32_t count) {
	char* outer = strdup(activity);
	running_with(arguments, count);
	return outer;
}
void crash_context_restored(char* outer) {
	snprintf(activity, sizeof activity, "%s", outer);
	free(outer);
}
static void crash_said(const char* what) {
	const char* parts[] = {"warp: ", what, " while ", activity, "\n"};
	for (size_t i = 0; i < sizeof parts / sizeof *parts; i++) (void)!write(STDERR_FILENO, parts[i], strlen(parts[i]));
}
static void crashed_by(int signal_number) {
	crash_said(strsignal(signal_number));
	signal(signal_number, SIG_DFL);
	raise(signal_number);
}
static void crash_reported(void) {
	for (size_t i = 0; i < sizeof CRASH_SIGNALS / sizeof *CRASH_SIGNALS; i++) signal(CRASH_SIGNALS[i], crashed_by);
}

int main(int argc, char** argv) {
	wasm_rt_init();
	crash_reported();
	running_with(argv, argc);
	w2c_program program;
	struct w2c_wasi__snapshot__preview1 wasi = {&program, 0, argc, argv};
	instantiate(&program, &wasi);
	wasi.memory = w2c_program_memory(&program);
	running_wasi = &wasi;
	wasm_rt_trap_t trapped = (wasm_rt_trap_t)WASM_RT_SETJMP(g_wasm_rt_jmp_buf);
	if (trapped != WASM_RT_TRAP_NONE) {
		crash_said(wasm_rt_strerror(trapped));
		signal(SIGABRT, SIG_DFL);
		abort();
	}
	run_main(&program);
	wasm2c_program_free(&program);
	wasm_rt_free();
	return 0;
}

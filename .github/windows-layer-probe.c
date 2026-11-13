// probe of lib/native/windows.c's calls on windows-latest (card windows-build-check); goes with the probe workflow
#include "native/system.h"
int main(void) {
	const char* home = getenv("HOME");
	printf("HOME=%s\n", home ? home : "(none)");
	char path[512];
	snprintf(path, sizeof path, "%s/.cache/warp/test-slots/0", home ? home : ".");
	char buffer[MAX_PATH];
	printf("windows path: %s\n", windows_path(path, buffer));
	int fd = posix_creat(path, 0644);
	printf("creat: %d errno %d\n", fd, errno);
	if (fd >= 0) {
		int locked = posix_flock(fd, LOCK_EX | LOCK_NB);
		printf("flock: %d errno %d last error %lu\n", locked, errno, GetLastError());
	}
	int fd2 = posix_creat("slot-here", 0644);
	printf("creat here: %d; flock: %d last error %lu\n", fd2, fd2 >= 0 ? posix_flock(fd2, LOCK_EX | LOCK_NB) : -9, GetLastError());
	return 0;
}

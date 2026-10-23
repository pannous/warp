#!/usr/bin/env bash
# libc in pure warp over plain C calls (lib/c.warp, card ffi-libc): the functions warp-rust hand-links
# (c_signatures.rs HAND_LINKED_LIBC: abs strlen atoi atol atof strcmp strncmp rand) as plain foreign lines, which
# warp-host links by dlsym without its Rust shims, and more of libc over the generic primitives: memory (malloc,
# c_write, c_bytes), bytes (memcmp), buffers a call fills (getcwd, gethostname), errno (c_errno), files and time.
# Run it through the queue: tests/queue.sh tests/host/test_ffi_libc.sh
LIBRARY=libc
PRELUDE=()
source "$(dirname "$0")/ffi_library_test.sh"
here=$stage
hostname=$(hostname)

# the hand-linked eight, as plain C calls
program strlen 'import strlen from "c"' 'print strlen("hello")' 'print strlen("héllo")'
program atoi 'import (atoi, atol, atof) from "c"' 'print atoi("42") + 1' 'print atol("9000000000")' 'print atof("2.5") * 2'
program strcmp 'import (strcmp, strncmp) from "c"' 'print(if strcmp("apple", "banana") < 0 then "before" else "after")' 'print strncmp("abcx", "abcy", 3)'
program abs_rand 'import (abs, rand) from "c"' 'print abs(-7)' 'print(if rand() >= 0 then "random" else "negative")'
# C memory: malloc, write, read back, free
program memory 'use c_memory' 'import (malloc, free) from "c"' 'p = malloc(4)' 'c_write(p, [1, 0, 255, 4])' 'print c_bytes(p, 4)' 'free(p)'
program memcmp 'import memcmp from "c"' 'print(if memcmp([1, 0, 2], [1, 0, 3], 3) < 0 then "less" else "not less")'
# a buffer the call fills, its text read back
program getcwd 'use c_memory' 'import (calloc, getcwd, free) from "c"' 'b = calloc(1, 4096)' 'getcwd(b, 4096)' 'print c_text(b)' 'free(b)'
program hostname 'use c_memory' 'import (calloc, gethostname, free) from "c"' 'b = calloc(1, 256)' 'print gethostname(b, 255)' 'print c_text(b)' 'free(b)'
# errno after a failing call, named by strerror
program errno 'use c_memory' 'import (unlink, strerror) from "c"' 'print unlink("/no/such/folder/file")' 'print strerror(c_errno())'
# files: a folder made, found, removed
program folder 'import (mkdir, access, rmdir) from "c"' 'print mkdir("made", 448)' 'print access("made", 0)' 'print rmdir("made")' 'print access("made", 0)'
program time 'import (time, getpid, strtoul) from "c"' 'print(if time(0) > 1700000000 then "now" else "past")' 'print(if getpid() > 0 then "process" else "none")' 'print strtoul("ff", 16)'

check strlen "strlen: bytes, as C counts them" 0 "5" "6" ! "Error"
check atoi "atoi, atol past 32 bits, atof" 0 "43" "9000000000" "5" ! "Error"
check strcmp "strcmp and strncmp" 0 "before" "0" ! "Error"
check abs_rand "abs and rand" 0 "7" "random" ! "Error"
check memory "malloc, c_write and c_bytes, NUL and all" 0 "[1 0 255 4]" ! "Error"
check memcmp "memcmp of byte lists with NULs" 0 "less" ! "Error"
check getcwd "getcwd fills a buffer: the working directory" 0 "$here" ! "Error"
check hostname "gethostname fills a buffer" 0 "0" "$hostname" ! "Error"
check errno "errno after unlink fails, strerror's text" 0 "-1" "No such file or directory" ! "Error"
check folder "mkdir, access, rmdir" 0 "0" "-1" ! "Error"
check time "time, getpid, strtoul with an out end" 0 "now" "process" "255" ! "Error"
finish

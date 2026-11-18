#!/usr/bin/env bash
# wap reads files (wap/preload/prelude.wap read; wiki/standard-library.md): read(path) is the file's text, a path it
# cannot open stops the program naming it. Run it with bash test/wap/test_files.sh
source "$(dirname "$0")/compile.sh"

printf 'line one\nline two' >"$stage/sample.txt"
check "a file's text" "$(ran read_file "print read(\"$stage/sample.txt\")")" "line one
line two"
check "a file read is a text like any" "$(ran read_compared "if read(\"$stage/sample.txt\") == \"line one\" + \"
line two\" then print \"same\" else print \"other\"")" "same"
check "a file that cannot be read" "$(ran read_missing "print read(\"$stage/missing.txt\")")" "cannot read $stage/missing.txt"
exit $((failures > 0))

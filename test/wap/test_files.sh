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
check "the command line, the program first" "$(ran command_line 'for each in arguments() { print each }' >/dev/null; $warp "$stage/command_line.wasm" a "b c")" "$stage/command_line.wasm
a
b c"
check "bytes written to a file, made or emptied first" "$(ran bytes_written "write_bytes(\"$stage/written.bin\", [104, 105, 33])
print read(\"$stage/written.bin\")
write_bytes(\"$stage/written.bin\", [111, 107])
print read(\"$stage/written.bin\")")" "hi!
ok"
check "args are the command line after the program's file, program its file and folder" "$(printf '%s\n' 'print text(#args)' 'print args#2' 'print program.folder' > "$stage/program_words.wap" && $warp "$compiler" "$stage/program_words.wap" "$stage/program_words.wasm" 2>&1 && $warp "$stage/program_words.wasm" a "b c" 2>&1)" "2
b c
$stage"
check "write_bytes writes a file's bytes, read gives them back" "$(ran bytes_file "write_bytes(\"$stage/out.bin\", [104, 105])
print read(\"$stage/out.bin\")")" "hi"
check "wall_clock_ms is the wall clock's milliseconds since 1970" "$(ran clock 'start = wall_clock_ms()
print text(start > 1700000000000 and wall_clock_ms() - start < 60000)')" "yes"
exit $((failures > 0))

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
check "the command line, the program first" "$(ran command_line 'arguments(): list<text> := prim arguments()
for each in arguments() { print each }' >/dev/null; $warp "$stage/command_line.wasm" a "b c")" "$stage/command_line.wasm
a
b c"
check "args are the command line after the program's file, program its file and folder" "$(printf '%s\n' 'print text(#args)' 'print args#2' 'print program.folder' > "$stage/program_words.wap" && $warp "$compiler" "$stage/program_words.wap" "$stage/program_words.wasm" 2>&1 && $warp "$stage/program_words.wasm" a "b c" 2>&1)" "2
b c
$stage"
exit $((failures > 0))

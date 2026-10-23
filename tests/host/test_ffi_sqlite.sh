#!/usr/bin/env bash
# SQLite in pure warp over plain C calls (lib/native/sqlite3.warp, topic ffi-libraries-roadmap): programs compiled by
# compiler-self.wasm `use sqlite3`, and warp-host calls libsqlite3, nothing of SQLite in Rust. Also the sqlite checks of
# tests/ffi/ (test_ffi_handles, test_ffi_gaps, test_ffi_text_results) on this lane. Run it through the queue:
# tests/queue.sh tests/host/test_ffi_sqlite.sh. The host and compiler are found as tests/host/test_ffi.sh finds them.
# Skips everything without libsqlite3.
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/ffi-sqlite-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED WARP_CRATES_CACHE

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
compiler_path=${WARP_FFI_COMPILER_FILE:-$(dirname "$rust")/compiler-self.wasm}
[ -f "$compiler_path" ] || { echo "✗ no compiler-self.wasm beside $rust: $rust rebuild self builds it"; exit 1; }
has_sqlite() {
	local file
	for file in /usr/lib/*/libsqlite3.so* /lib/*/libsqlite3.so* /usr/lib64/libsqlite3.so* /opt/homebrew/lib/libsqlite3.dylib; do
		[ -e "$file" ] && return 0
	done
	[ "$(uname)" = Darwin ] # in the dyld cache
}
has_sqlite || { echo "- sqlite: no libsqlite3 here, skipped"; exit 0; }
rm -rf "$stage" && mkdir -p "$stage/bin" || exit 1
ln -f "$host" "$stage/bin/warp-host" 2>/dev/null || cp "$host" "$stage/bin/warp-host" || exit 1
cp "$compiler_path" "$stage/bin/compiler-self.wasm" || exit 1
host=$stage/bin/warp-host
cd "$stage" || exit 1
program() { # program <name> <source lines…>: <name>.warp in the stage, after `use sqlite3` and an in-memory db
	local name=$1
	shift
	printf '%s\n' 'use sqlite3' 'db = sqlite3_open(":memory:")' "$@" >"$stage/$name.warp"
}
statement='statement = sqlite3_prepare_v2(db, "select 1+2, '"'"'warp'"'"'", -1)'

# tests/ffi/test_ffi_handles: a session on handles
program step "$statement" 'sqlite3_step(statement)'
program column_int "$statement" 'sqlite3_step(statement)' 'sqlite3_column_int(statement, 0)'
program column_text "$statement" 'sqlite3_step(statement)' 'print sqlite3_column_text(statement, 1)'
program done "$statement" 'sqlite3_step(statement)' 'sqlite3_step(statement)'
program finalize "$statement" 'sqlite3_finalize(statement)' 'sqlite3_close(db)'
program db_handle "$statement" 'print(if sqlite3_db_handle(statement) == db then "same" else "other")'
program null_handle 'sqlite3_db_handle(0)'
# tests/ffi/test_ffi_gaps, test_ffi_text_results
program version 'print(if sqlite3_libversion_number() > 3000000 then "new enough" else "old")' 'print sqlite3_libversion()'
program errstr 'print sqlite3_errstr(1)'
# every column type through binds and back
program types 'run_query(db, "create table t(i integer, big integer, d real, s text, x blob, n)")' \
	'run_query(db, "insert into t values (7, 1, 2.5, 1, 1, 1)")' \
	'query(db, "insert into t values (?, ?, ?, ?, ?, ?)", [8, 9007199254740993, 0.25, "hi there", [0, 1, 255], ø])' \
	'for row in query(db, "select * from t where i = ?", [8]) { for value in row { print value } }' \
	'print query(db, "select typeof(i), typeof(big), typeof(d), typeof(s), typeof(x), typeof(n) from t where i = 8", [])'
program maps 'run_script(db, "create table people(name text, age int); insert into people values ('"'"'ada'"'"', 36), ('"'"'alan'"'"', 41)")' \
	'found = query_maps(db, "select name, age from people where age > ? order by age", [30])' \
	'print #found' 'print found#1["name"]' 'print found#2["age"]'
program script 'print run_script(db, "create table t(x); insert into t values (1); insert into t values (2); update t set x = x * 10")' \
	'print rows(db, "select sum(x) from t")'
program changes 'run_query(db, "create table t(x)")' 'print run_query(db, "insert into t values (5), (6), (7)")' \
	'print sqlite3_last_insert_rowid(db)' 'print sqlite3_changes(db)' 'print sqlite3_total_changes(db)'
program error_thrown 'try run_script(db, "insert into missing values (1)") catch e { print "caught " + e }'
program step_error_thrown 'run_query(db, "create table t(x unique)")' 'run_query(db, "insert into t values (1)")' \
	'try run_query(db, "insert into t values (1)") catch e { print "caught " + e }' 'print sqlite3_errcode(db)'
program metadata 'run_query(db, "create table t(name text, n int)")' \
	's = sqlite3_prepare_v2(db, "select name, n from t where n > :least", -1)' \
	'print sqlite3_column_count(s)' 'print sqlite3_column_name(s, 1)' 'print sqlite3_column_decltype(s, 0)' \
	'print sqlite3_bind_parameter_count(s)' 'print sqlite3_bind_parameter_name(s, 1)' \
	'print sqlite3_bind_parameter_index(s, ":least")' 'print sqlite3_sql(s)' 'print sqlite3_stmt_readonly(s)' \
	'print sqlite3_finalize(s)'
program reset 'run_query(db, "create table t(x)")' 's = sqlite3_prepare_v2(db, "insert into t values (?)", -1)' \
	'for x in [1, 2, 3] { sqlite3_bind_int(s, 1, x); sqlite3_step(s); sqlite3_reset(s) }' 'sqlite3_finalize(s)' \
	'print rows(db, "select count(*), sum(x) from t")'
program file_db 'file = "'"$stage"'/test.db"' 'one = sqlite3_open(file)' 'run_query(one, "create table t(x)")' \
	'run_query(one, "insert into t values ('"'"'kept'"'"')")' 'sqlite3_close(one)' \
	'two = sqlite3_open_v2(file, SQLITE_OPEN_READONLY, 0)' 'print rows(two, "select x from t")' \
	'try run_query(two, "insert into t values (1)") catch e { print "caught " + e }' 'print sqlite3_close_v2(two)'
program bad_query 'rows(db, "selec nothing")'
program complete 'print sqlite3_complete("select 1;")' 'print sqlite3_complete("select 1")' 'print sqlite3_get_autocommit(db)'

run=0 failed=0
# expect <name> <exit status> <text the output holds>… [! <text it lacks>…]: stdout and stderr together
expect() {
	local name=$1 status=$2 wanted=() unwanted=() said got text problems=""
	shift 2
	while [ $# -gt 0 ]; do
		if [ "$1" = '!' ]; then unwanted+=("$2"); shift; else wanted+=("$1"); fi
		shift
	done
	said=$("$host" --compiler self run "$program.warp" 2>&1)
	got=$?
	run=$((run + 1))
	for text in "${wanted[@]}"; do [[ $said == *"$text"* ]] || problems+=" missing [$text]"; done
	for text in "${unwanted[@]}"; do [[ $said == *"$text"* ]] && problems+=" unwanted [$text]"; done
	if [ "$got" = "$status" ] && [ -z "$problems" ]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: exit $got (expected $status),$problems"
		echo "$said" | sed 's/^/    /'
	fi
}
check() { program=$1; shift; expect "$@"; }

check step "a step with a row ready is SQLITE_ROW" 100 "100"
check column_int "sqlite3_column_int" 3 "3"
check column_text "sqlite3_column_text is a text" 0 "warp" ! "Error"
check done "the step after the last row is SQLITE_DONE" 101 "101"
check finalize "finalize and close are SQLITE_OK" 0 ! "Error"
check db_handle "a pointer result is the handle of the same pointer" 0 "same"
check null_handle "a NULL handle crosses as 0" 0 ! "Error"
check version "sqlite3_libversion_number and sqlite3_libversion" 0 "new enough" "3." ! "Error"
check errstr "sqlite3_errstr: a C text result" 0 "SQL logic error"
check types "every column type: integer, int64, double, text, blob, null" 0 "8" "9007199254740993" "0.25" "hi there" \
	"[0 1 255]" "ø" '"integer" "integer" "real" "text" "blob" "null"' ! "Error"
check maps "query_maps: rows as maps by column name, a bound parameter" 0 "2" "ada" "41" ! "Error"
check script "run_script runs several statements (sqlite3_exec)" 0 "4" "30" ! "Error"
check changes "changes, total_changes and last_insert_rowid" 0 "3" ! "Error"
check error_thrown "an exec's error is thrown with sqlite3_errmsg's text" 0 "caught" "no such table: missing"
check step_error_thrown "a step's error is thrown: a constraint" 0 "caught" "UNIQUE constraint failed" "19"
check metadata "column names and declared types, parameter names and indexes, the statement's text" 0 \
	"2" "n" "text" "1" ":least" "select name, n from t where n > :least" ! "Error"
check reset "bind, step and reset a statement again" 0 "3 6" ! "Error"
check file_db "a file database: kept after close, read-only by open_v2" 0 "kept" "caught" "readonly database" ! "Error:"
check bad_query "a query that doesn't prepare fails the run, naming its C status" 1 "sqlite3_prepare_v2 gave no sqlite3_stmt"
check complete "sqlite3_complete and sqlite3_get_autocommit" 0 ! "Error"

echo "sqlite: $((run - failed)) passed / $run"
[ "$failed" = 0 ]

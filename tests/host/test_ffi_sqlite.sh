#!/usr/bin/env bash
# SQLite in pure warp over plain C calls (lib/native/sqlite3.warp, topic ffi-libraries-roadmap): programs compiled by
# compiler-self.wasm `use sqlite3`, and warp-host calls libsqlite3, nothing of SQLite in Rust. Also the sqlite checks of
# tests/ffi/ (test_ffi_handles, test_ffi_gaps, test_ffi_text_results) on this lane. Run it through the queue:
# tests/queue.sh tests/host/test_ffi_sqlite.sh. The host and compiler are found as tests/host/test_ffi.sh finds them.
# Skips everything without libsqlite3.
LIBRARY=sqlite
PRELUDE=('use sqlite3' 'db = sqlite3_open(":memory:")')
source "$(dirname "$0")/ffi_library_test.sh"
has_library sqlite3 || { echo "- sqlite: no libsqlite3 here, skipped"; exit 0; }
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
program error_thrown 'v = try run_script(db, "insert into missing values (1)") catch e { "recovered" }' 'print v' \
	'run_script(db, "create table missing(x); insert into missing values (42)")' 'print rows(db, "select sum(x) from missing")'
program step_error_thrown 'run_query(db, "create table t(x unique)")' 'run_query(db, "insert into t values (17)")' \
	'v = try run_query(db, "insert into t values (17)") catch e { "recovered" }' 'print v' \
	'print rows(db, "select count(*), sum(x) from t")'
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
	'v = try run_query(two, "insert into t values (1)") catch e { "recovered" }' 'print v' \
	'print rows(two, "select count(*) from t")' 'print sqlite3_close_v2(two)'
program literal_query 'print rows(db, "select '"'"'fine'"'"', 6 * 7")'
program complete 'print sqlite3_complete("select 1;")' 'print sqlite3_complete("select 1")' 'print sqlite3_get_autocommit(db)'

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
check error_thrown "a try around a failing exec recovers, the database stays usable" 0 "recovered" "42" ! "Error"
check step_error_thrown "a try around a step that breaks a constraint recovers, the row kept once" 0 "recovered" "1 17" ! "Error"
check metadata "column names and declared types, parameter names and indexes, the statement's text" 0 \
	"2" "n" "TEXT" "1" ":least" "select name, n from t where n > :least" ! "Error"
check reset "bind, step and reset a statement again" 0 "3 6" ! "Error"
check file_db "a file database: kept after close, read by open_v2, a try around a write recovers" 0 "kept" "recovered" ! "Error:"
check literal_query "a query of literals: a text and a computed number" 0 "fine" "42" ! "Error"
check complete "sqlite3_complete and sqlite3_get_autocommit" 0 ! "Error"

finish

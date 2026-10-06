// C pointers cross as handle ids (card ffi-handles, notes/ffi_handles.md): a pointer result (sqlite3 *, FILE *) is an id
// into the run's handle table, 0 is NULL; a struct-pointer parameter takes such an id; an out-pointer (sqlite3 **ppDb)
// is left out of the wasp call and becomes its result, NULL there is a loud error with the C status
#![cfg(feature = "native")]
use warp::is;

const OPEN_AND_PREPARE: &str = "use sqlite3; db = sqlite3_open(\":memory:\"); stmt = sqlite3_prepare_v2(db, \"select 1+2, 'wasp'\", -1)";

#[test]
fn a_sqlite_session_runs_on_handles() {
	is!(&format!("{OPEN_AND_PREPARE}; sqlite3_step(stmt)"), 100); // SQLITE_ROW
	is!(&format!("{OPEN_AND_PREPARE}; sqlite3_step(stmt); sqlite3_column_int(stmt, 0)"), 3);
	is!(&format!("{OPEN_AND_PREPARE}; sqlite3_step(stmt); sqlite3_column_text(stmt, 1)"), "wasp");
	is!(&format!("{OPEN_AND_PREPARE}; sqlite3_step(stmt); sqlite3_step(stmt)"), 101); // SQLITE_DONE
	is!(&format!("{OPEN_AND_PREPARE}; sqlite3_finalize(stmt); sqlite3_close(db)"), 0);
}

#[test]
fn a_pointer_result_is_a_handle() {
	is!("use sqlite3; sqlite3_db_handle(0)", 0); // NULL in, NULL out
	is!(&format!("{OPEN_AND_PREPARE}; sqlite3_db_handle(stmt) == db"), true);
	is!("use c; f = fopen(\"/dev/null\", \"r\"); fclose(f)", 0);
}

#[test]
fn a_null_out_pointer_is_a_loud_error() {
	crate::common::fails_with("use sqlite3; db = sqlite3_open(\":memory:\"); sqlite3_prepare_v2(db, \"no sql here\", -1)", "sqlite3_prepare_v2 gave no sqlite3_stmt (C status 1)");
}

#[test]
fn an_unknown_handle_is_a_loud_error() {
	crate::common::fails_with("use sqlite3; sqlite3_step(42)", "sqlite3_step: 42 is no C handle of this run");
}

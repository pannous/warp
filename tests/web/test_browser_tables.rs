//! The playground's stored tables (web/playground/host-files.js `table`, card orm-browser-stores) under node: each row
//! is its own IndexedDB key and the rows stay in id order, so inserting, paging, updating and deleting cost the same on
//! a big table as on a small one (an insert spread every id: 20 000 rows took 1.5 s, quadratic, and 200 000 overflowed)
//! Timing probe: probes/orm_browser_stores.mjs
use std::process::Command;

const HOST_FILES: &str = include_str!("../../web/playground/host-files.js");
const ROWS: usize = 100_000;
/// a hundred thousand inserts, a page walk and as many updates take ~0.2 s; quadratic they took minutes
const BUDGET_MS: usize = 5_000;
const STUB: &str = r#"
const self = { location: { href: "http://localhost/web/playground/" } };
let part;
const addHostPart = given => { part = given; };
"#;
const BIG_TABLE: &str = r#"
const table = part.adapters.table;
const [file, schema] = ["people.database.json", [["name", "text", null], ["age", "int", 0]]];
const started = Date.now();
table.migrate.call({ warn: console.error }, "people", schema, file);
for (let i = 1; i <= ROWS; i++) table.insert("people", ["name", "age"], [`p${i}`, i % 90], file);
for (let start = 1; start <= ROWS; start += 100) table.page("people", schema, file, start, 100);
for (let id = 1; id <= ROWS; id++) table.update("people", id, "age", id % 7, file);
table.delete("people", 2, file);
const added = table.insert("people", ["name", "age"], ["new", 1], file);
const elapsed = Date.now() - started;
console.log(JSON.stringify({
	first: table.page("people", schema, file, 1, 3),
	last: table.page("people", schema, file, ROWS - 1, 5),
	count: table.count("people", schema, file),
	added,
	fast: elapsed < BUDGET_MS,
}));
databaseChannel?.close(); // an open BroadcastChannel keeps node running
"#;

#[test]
fn a_big_browser_table_costs_the_same_per_row() {
	let script = format!("{STUB}{HOST_FILES}\nconst [ROWS, BUDGET_MS] = [{ROWS}, {BUDGET_MS}];\n{BIG_TABLE}");
	let output = Command::new("node").arg("-e").arg(script).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	let expected = format!(
		r#"{{"first":[[1,"p1",1],[3,"p3",3],[4,"p4",4]],"last":[[{ROWS},"p{ROWS}",{}],[{},"new",1]],"count":{ROWS},"added":{},"fast":true}}"#,
		ROWS % 7,
		ROWS + 1,
		ROWS + 1
	);
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), expected);
}

// card todo-app: a bool field is an INTEGER 0/1 column, read back as the bool it was (it read back as the Int 0)
use crate::is;
use warp::wasm_emitter::eval;

const TODO: &str = "class Todo{title: text; done: bool}\ntodos: [Todo] = database.todos_bool";

fn program(rest: &str) -> String {
	format!("{TODO}\n{rest}")
}

#[test]
fn a_bool_column_reads_back_as_a_bool() {
	eval(&program("todos.add(Todo(\"milk\", false))\ntodos.add(Todo(\"tea\", true))"));
	is!(&program("todos#1.done"), false);
	is!(&program("todos#2.done"), true);
	is!(&program("count(todos where not it.done)"), 1);
	is!(&program("count(todos where not done)"), 1);
	let rows = eval(&program("todos")).serialize();
	assert!(rows.contains("done:no") && rows.contains("done:yes"), "{rows}");
}

// a one-character text ("x" parses as a character) in a text column is that text, not '' (found writing the todo app)
#[test]
fn a_one_character_text_is_stored_as_its_text() {
	let program = |rest: &str| format!("class Note{{title: text}}\nnotes: [Note] = database.notes_char\n{rest}");
	eval(&program("notes.add(Note(\"x\"))"));
	is!(&program("notes#1.title"), "x");
}

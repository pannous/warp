// `ages["alice"]` looks a key up, it counts no position: no hint to shift it into `#`; a position index still gets one
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

fn index_hints(code: &str) -> Vec<String> {
	let (_, hints) = capture_hints(|| eval(code));
	hints.into_iter().filter(|hint| hint.reason == "use # for indexing").map(|hint| hint.canonical).collect()
}

#[test]
fn a_text_key_subscript_gets_no_index_hint() {
	assert_eq!(index_hints("ages={alice:3}; ages[\"alice\"]"), Vec::<String>::new());
	assert_eq!(index_hints("ages={alice:3}; ages['alice']"), Vec::<String>::new());
	assert_eq!(index_hints("xs=[1,2]; xs[0]"), vec!["xs#1".to_string()]); // card g_YiSA: a number only
}

// card text-key: a variable holding a text, or a parameter typed text, is a key too
#[test]
fn a_text_variable_subscript_gets_no_index_hint() {
	assert_eq!(index_hints("l=\"en\"; m={en:\"x\"}; m[l]"), Vec::<String>::new());
	assert_eq!(index_hints("get(m, k:text) := m[k]; get({en:\"x\"}, \"en\")"), Vec::<String>::new());
	assert_eq!(index_hints("m={en:{hi:\"x\"}}; l=\"en\"; k=\"hi\"; m[l][k]"), Vec::<String>::new());
	assert_eq!(index_hints("xs=[1,2]; xs[0]"), vec!["xs#1".to_string()]); // card g_YiSA: a number only
}

// card index-hint-parens: in a chain each index is shifted, `g#1#2`; card g_YiSA: a name index gets no hint, so the
// chain is of numbers (it was `g[i][j]` → `g#(i+1)#(j+1)`, its parentheses kept)
#[test]
fn a_chained_index_hint_keeps_its_parentheses() {
	let hints = index_hints("g=[[1,2],[3,4]]; g[0][1]");
	assert_eq!(hints, vec!["g#1".to_string(), "g#1#2".to_string()]);
}

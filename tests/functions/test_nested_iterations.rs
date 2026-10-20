//! An iteration word inside another's function keeps its own result: `xs.any(x => ys.any(…))` (card rust-lambdas)
use crate::is;

#[test]
fn nested_iteration_words_keep_their_own_results() {
	is!("[6, 5].any(s => [1].any(w => s == 6))", true);
	is!("[5, 6].all(s => [1].all(w => s > 4))", true);
	is!("[6, 5].all(s => [1].all(w => s == 5))", false);
	is!("[6, 5].find(s => [1, 2].find(w => w == s - 4) != ø)", 6);
}

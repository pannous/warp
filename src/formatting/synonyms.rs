//! `warp beautify`'s rule "synonyms canonical" (decision warp-format, card warp-format-strict): a word the compiler
//! notes as another language's word for a warp word (diagnostic::note_alias: `auto` → var, `__add__` → plus) becomes
//! the warp word by the note's "I meant" fix. A fix is kept only when the program still lowers to the same code
use super::Change;
use crate::errors::diagnostic::{every_hint_of, Diagnostic, ALIAS_TOPIC_PREFIX};
use crate::errors::fixits;

const RULE: &str = "synonyms canonical";

/// The program as the emitter gets it, to compare; None when it does not lower
fn lowered(code: &str) -> Option<String> {
	crate::pipeline::lower(code).ok().map(|program| program.serialize())
}

fn is_alias_note(hint: &Diagnostic) -> bool {
	hint.topic.as_deref().is_some_and(|topic| topic.starts_with(ALIAS_TOPIC_PREFIX)) && !hint.fixes.is_empty()
}

/// The code with every alias note's fix that keeps the lowered program, the last in the source first so the earlier
/// positions stay valid
pub(super) fn canonical_synonyms(code: &str) -> (String, Vec<Change>) {
	let (program, hints) = every_hint_of(|| lowered(code));
	let Some(program) = program else {
		return (code.to_string(), Vec::new());
	};
	let mut notes: Vec<Diagnostic> = hints.into_iter().filter(is_alias_note).collect();
	notes.sort_by_key(|note| std::cmp::Reverse((note.line, note.column)));
	let mut text = code.to_string();
	let mut changes = Vec::new();
	for note in notes {
		let candidate = fixits::fixed(&text, note.line, note.column, &note.fixes[0]);
		let Some(candidate) = candidate.filter(|candidate| *candidate != text) else { continue };
		if crate::errors::diagnostic::quietly(|| lowered(&candidate)).as_ref() == Some(&program) {
			text = candidate;
			changes.push(Change { line: note.line, rule: RULE });
		}
	}
	changes.reverse();
	(text, changes)
}

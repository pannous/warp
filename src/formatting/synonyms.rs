//! `warp beautify`'s rule "synonyms canonical" (decision warp-format, card warp-format-strict): a word the compiler
//! notes as another language's word for a warp word (diagnostic::note_alias: `auto` → var, `__add__` → plus) becomes
//! the warp word by the note's "I meant" fix. A fix is kept only when the program still lowers to the same code
use super::{lowered, Change};
use crate::errors::diagnostic::{every_hint_of, take_warnings, Diagnostic, ALIAS_TOPIC_PREFIX};
use crate::errors::fixits;
use crate::warp_parser::FLOOR_DIVISION_TOPIC;

const RULE: &str = "synonyms canonical";
/// The warnings whose fix the user decided on, each with its rule: a `//` comment meant as floor division becomes `/!`
/// (decision floor-division), a result that can be ø is declared `T?` (decision optional-result)
const MEANT_REWRITES: [(&str, &str); 2] = [
	(FLOOR_DIVISION_TOPIC, "floor division /!"),
	(crate::lowering::type_name_matching::OPTIONAL_RESULT_TOPIC, "optional result T?"),
];

fn is_alias_note(hint: &Diagnostic) -> bool {
	hint.topic.as_deref().is_some_and(|topic| topic.starts_with(ALIAS_TOPIC_PREFIX)) && !hint.fixes.is_empty()
}

/// The code with every alias note's fix that keeps the lowered program, after the rewrites the user decided on
/// (meant_rewrites), the last in the source first so the earlier positions stay valid
pub(super) fn canonical_synonyms(code: &str) -> (String, Vec<Change>) {
	let (code, mut changes) = meant_rewrites(code);
	let (program, hints) = every_hint_of(|| lowered(&code));
	let Some(program) = program else {
		return (code, changes);
	};
	let notes: Vec<Diagnostic> = hints.into_iter().filter(is_alias_note).collect();
	let (text, aliases) = applied(&code, notes, RULE, |candidate| {
		crate::errors::diagnostic::quietly(|| lowered(candidate)).as_ref() == Some(&program)
	});
	changes.extend(aliases);
	(text, changes)
}

/// The fixes of the warnings whose rewrite the user decided on (MEANT_REWRITES), where the program then still lowers
fn meant_rewrites(code: &str) -> (String, Vec<Change>) {
	take_warnings(); // only this code's warnings
	every_hint_of(|| lowered(code));
	let warnings = take_warnings();
	let mut text = code.to_string();
	let mut changes = Vec::new();
	for (topic, rule) in MEANT_REWRITES {
		let meant: Vec<Diagnostic> = warnings
			.iter()
			.filter(|warning| warning.topic.as_deref() == Some(topic) && !warning.fixes.is_empty())
			.cloned()
			.collect();
		let (rewritten, rule_changes) = applied(&text, meant, rule, |candidate| {
			crate::errors::diagnostic::quietly(|| lowered(candidate)).is_some()
		});
		text = rewritten;
		changes.extend(rule_changes);
	}
	(text, changes)
}

/// `text` with the first fix of each diagnostic that `accepts`, the last in the source first
fn applied(
	text: &str,
	mut diagnostics: Vec<Diagnostic>,
	rule: &'static str,
	accepts: impl Fn(&str) -> bool,
) -> (String, Vec<Change>) {
	diagnostics.sort_by_key(|diagnostic| std::cmp::Reverse((diagnostic.line, diagnostic.column)));
	diagnostics.dedup_by_key(|diagnostic| (diagnostic.line, diagnostic.column));
	let mut text = text.to_string();
	let mut changes = Vec::new();
	for diagnostic in diagnostics {
		let candidate = fixits::fixed(&text, diagnostic.line, diagnostic.column, &diagnostic.fixes[0]);
		let Some(candidate) = candidate.filter(|candidate| *candidate != text) else { continue };
		if accepts(&candidate) {
			text = candidate;
			changes.push(Change { line: diagnostic.line, rule });
		}
	}
	changes.reverse();
	(text, changes)
}

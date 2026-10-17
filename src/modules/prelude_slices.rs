//! lib/prelude.warp parsed by the definitions a program reaches (card prelude-slices): parsing all of it took 8 ms of
//! a program calling one prelude word. The prelude is cut into slices, each a definition with the comment lines above
//! it, kept on disk with the names each declares and calls (std_words_cache.rs); a program parses those its calls
//! reach, the closure of those names
use super::*;

/// The extension of the kept slices in the module cache
const SLICES_EXTENSION: &str = "prelude-slices";
const FIELD_SEPARATOR: char = '\t';
const NAME_SEPARATOR: char = ' ';
const COMMENT_START: &str = "//";

/// Lines `first..end` of the prelude (counted from 1), the names their statements declare and those they call
struct PreludeSlice {
	first: usize,
	end: usize,
	declared: Vec<String>,
	called: Vec<String>,
}

/// The prelude's slices: kept from an earlier run, else cut from its parse and kept; None when its statements carry no
/// lines to cut by
fn prelude_slices() -> Option<&'static [PreludeSlice]> {
	static SLICES: std::sync::OnceLock<Option<Vec<PreludeSlice>>> = std::sync::OnceLock::new();
	SLICES
		.get_or_init(|| kept_text(SLICES_EXTENSION).and_then(|text| read_slices(&text)).or_else(cut_slices))
		.as_deref()
}

fn cut_slices() -> Option<Vec<PreludeSlice>> {
	let source = std_module(PRELUDE_MODULE)?;
	let lines: Vec<&str> = source.lines().collect();
	let statements = std_module_statements(PRELUDE_MODULE);
	// a statement starts at its line, or at the comment lines right above it
	let with_comment = |line: usize| {
		(1..line)
			.rev()
			.take_while(|above| lines[above - 1].trim_start().starts_with(COMMENT_START))
			.last()
			.unwrap_or(line)
	};
	let starts: Vec<usize> =
		statements.iter().map(|statement| Some(with_comment(first_line(statement)?))).collect::<Option<_>>()?;
	let mut firsts: Vec<usize> = starts.clone();
	firsts.dedup();
	let slices: Vec<PreludeSlice> = firsts
		.iter()
		.enumerate()
		.map(|(index, &first)| {
			let end = firsts.get(index + 1).copied().unwrap_or(lines.len() + 1);
			let own: Vec<Node> = statements
				.iter()
				.zip(&starts)
				.filter(|(_, start)| **start == first)
				.map(|(statement, _)| statement.clone())
				.collect();
			let declared =
				own.iter().filter(|statement| is_declaration(statement)).filter_map(name_declared_by).collect();
			let called = called_names(&own).into_iter().filter(|name| !name.contains(char::is_whitespace)).collect();
			PreludeSlice { first, end, declared, called }
		})
		.collect();
	keep_text(SLICES_EXTENSION, &written_slices(&slices));
	Some(slices)
}

/// The line a statement starts at: its own position, else its first part's (a definition's head)
fn first_line(statement: &Node) -> Option<usize> {
	match statement.written_at() {
		Some((line, _, _)) if line > 0 => Some(line as usize),
		_ => match statement.drop_meta() {
			Node::Key(left, _, _) => first_line(left),
			Node::List(items, _, _) => first_line(items.first()?),
			_ => None,
		},
	}
}

fn written_slices(slices: &[PreludeSlice]) -> String {
	let names = |names: &[String]| names.join(&NAME_SEPARATOR.to_string());
	let line = |slice: &PreludeSlice| {
		let PreludeSlice { first, end, declared, called } = slice;
		format!("{first}{FIELD_SEPARATOR}{end}{FIELD_SEPARATOR}{}{FIELD_SEPARATOR}{}", names(declared), names(called))
	};
	slices.iter().map(line).collect::<Vec<_>>().join("\n")
}

fn read_slices(text: &str) -> Option<Vec<PreludeSlice>> {
	let names = |field: &str| field.split(NAME_SEPARATOR).filter(|name| !name.is_empty()).map(String::from).collect();
	let slice = |line: &str| {
		let mut fields = line.split(FIELD_SEPARATOR);
		let (first, end) = (fields.next()?.parse().ok()?, fields.next()?.parse().ok()?);
		Some(PreludeSlice { first, end, declared: names(fields.next()?), called: names(fields.next()?) })
	};
	text.lines().map(slice).collect()
}

/// The statements of slice `index`, parsed once per thread, at their lines in lib/prelude.warp
fn parsed_slice(index: usize, slice: &PreludeSlice) -> Vec<Node> {
	thread_local!(static PARSED: std::cell::RefCell<HashMap<usize, Vec<Node>>> = std::cell::RefCell::default());
	if let Some(parsed) = PARSED.with_borrow(|parsed| parsed.get(&index).cloned()) {
		return parsed;
	}
	let source = std_module(PRELUDE_MODULE).unwrap_or_default();
	let text: String =
		source.lines().skip(slice.first - 1).take(slice.end - slice.first).collect::<Vec<_>>().join("\n");
	// the lines above as empty ones: positions stay those of the file
	let placed = "\n".repeat(slice.first - 1) + &text;
	let shown = shown_path(&std_path(PRELUDE_MODULE));
	let parsed = statements(crate::normalize::without_hints(|| {
		crate::compiler::meta::parsing_file(&shown.to_string_lossy(), || WarpParser::parse(&placed))
	}));
	PARSED.with_borrow_mut(|cache| cache.insert(index, parsed.clone()));
	parsed
}

/// The prelude's declarations a statement mentioning `mentioned` may reach: those named by them (is_needed), their
/// own calls in turn, and the implicit operations (traits IMPLICIT_OPERATIONS), which a type alone calls. A superset
/// of what needed_definitions picks from them
pub(super) fn prelude_declarations_reached(mentioned: &HashSet<String>) -> Vec<Node> {
	let Some(slices) = prelude_slices() else {
		return std_module_statements(PRELUDE_MODULE).into_iter().filter(is_declaration).collect();
	};
	let mut mentioned = mentioned.clone();
	let mut taken = vec![false; slices.len()];
	loop {
		let reached: Vec<usize> = (0..slices.len())
			.filter(|&index| !taken[index])
			.filter(|&index| {
				slices[index].declared.iter().any(|name| {
					is_needed(name, &mentioned) || crate::lowering::traits::IMPLICIT_OPERATIONS.contains(&name.as_str())
				})
			})
			.collect();
		if reached.is_empty() {
			break;
		}
		for index in reached {
			taken[index] = true;
			mentioned.extend(slices[index].called.iter().cloned());
		}
	}
	let indices = (0..slices.len()).filter(|&index| taken[index]);
	indices.flat_map(|index| parsed_slice(index, &slices[index])).filter(is_declaration).collect()
}

/// The prelude's declarations of these words, for a program defining them too (warn_replaced) and prelude_definition
pub(crate) fn prelude_declarations_of(words: &[&str]) -> Vec<Node> {
	let declared = |statement: &Node| name_declared_by(statement).is_some_and(|name| words.contains(&name.as_str()));
	let Some(slices) = prelude_slices() else {
		return std_module_statements(PRELUDE_MODULE).into_iter().filter(is_declaration).filter(declared).collect();
	};
	let declares = |slice: &PreludeSlice| slice.declared.iter().any(|name| words.contains(&name.as_str()));
	let indices = (0..slices.len()).filter(|&index| declares(&slices[index]));
	indices.flat_map(|index| parsed_slice(index, &slices[index])).filter(is_declaration).filter(declared).collect()
}

//! The soft hints of `warp format`: reported, never rewritten (decision warp-format)

use super::lines::{scan, LineScan};

/// Abbreviations warp avoids, with the word it writes (user, 2026-10-15: "fmt, str, cnt"), every word, built-ins too
const ABBREVIATIONS: [(&str, &str); 35] = [
	("arr", "list"),
	("avg", "average"),
	("btn", "button"),
	("buf", "buffer"),
	("calc", "calculate"),
	("cfg", "configuration"),
	("cnt", "count"),
	("ctx", "context"),
	("del", "delete"),
	("dict", "map"),
	("dst", "destination"),
	("elem", "element"),
	("err", "error"),
	("fmt", "format"),
	("fn", "function"),
	("func", "function"),
	("idx", "index"),
	("img", "image"),
	("init", "initialize"),
	("len", "length"),
	("lst", "list"),
	("msg", "message"),
	("num", "number"),
	("nums", "numbers"),
	("obj", "object"),
	("pos", "position"),
	("prev", "previous"),
	("ptr", "pointer"),
	("req", "request"),
	("res", "result"),
	("src", "source"),
	("str", "text"),
	("tmp", "temporary"),
	("val", "value"),
	("vals", "values"),
];

pub struct Hint {
	pub line: usize,
	pub message: String,
}

/// The words of a name: `max_cnt` and `maxCnt` hold `max` and `cnt`
fn parts(name: &str) -> Vec<String> {
	let mut parts = vec![String::new()];
	for c in name.chars() {
		let starts_part = c == '_' || (c.is_uppercase() && parts.last().is_some_and(|part| !part.is_empty()));
		if starts_part {
			parts.push(String::new());
		}
		if c != '_' {
			parts.last_mut().unwrap().extend(c.to_lowercase());
		}
	}
	parts.into_iter().filter(|part| !part.is_empty()).collect()
}

fn words_of_code(line: &str, scan: &LineScan) -> Vec<String> {
	let code: String = scan.code.iter().map(|&(start, end)| format!("{} ", &line[start..end])).collect();
	code.split(|c: char| !c.is_alphanumeric() && c != '_').filter(|word| !word.is_empty()).map(str::to_string).collect()
}

/// Each abbreviation once, at its first line; then one hint for the `//` comments
pub fn soft_hints(code: &str) -> Vec<Hint> {
	let lines: Vec<String> = code.lines().map(str::to_string).collect();
	let scans = scan(&lines);
	let mut hints = Vec::new();
	let mut seen = Vec::new();
	for (index, (line, scan)) in lines.iter().zip(&scans).enumerate() {
		for word in words_of_code(line, scan) {
			for part in parts(&word) {
				let Some((_, full)) = ABBREVIATIONS.iter().find(|(short, _)| *short == part) else { continue };
				if !seen.contains(&part) {
					hints.push(Hint {
						line: index + 1,
						message: format!("warp avoids abbreviations: {part} → {full} (in {word})"),
					});
					seen.push(part);
				}
			}
		}
	}
	let slash_comments: Vec<usize> = (0..lines.len())
		.filter(|&index| scans[index].comment.is_some_and(|at| lines[index][at..].starts_with("//")))
		.collect();
	if let Some(&first) = slash_comments.first() {
		hints.push(Hint {
			line: first + 1,
			message: format!(
				"prefer # comments: `//` also means floor division ({} // comments; warp format hash-comments rewrites them)",
				slash_comments.len()
			),
		});
	}
	hints
}

//! A WGSL error of a `shader { … }` block as a warp error (card wgsl-error): the GPU names the line and column in the
//! shader's text (`paint: 4:21: expected "{", found ":"`), the error names them in the program, whose line is shown
//! underlined. The WGSL itself stays as written; only its `$name` holes became `values.name`.
use crate::graphics::shader_holes::written;
use crate::node::Node;
use crate::warp_parser::shader_blocks;

/// `paint: 4:21: why`: the failing word, the line and column in the shader's text, and why
fn shader_failure(message: &str) -> Option<(&str, usize, usize, &str)> {
	let (word, rest) = message.split_once(": ")?;
	let mut parts = rest.splitn(3, ':');
	let line = parts.next()?.parse().ok()?;
	let column = parts.next()?.parse().ok()?;
	Some((word, line, column, parts.next()?.strip_prefix(' ')?))
}

/// The program's error with the shader's place as the program's own (`paint: why at 6:22`); unchanged when no single
/// block of `code` has that place
pub fn relocated(result: Node, code: &str) -> Node {
	let Node::Error(reason) = result.drop_meta() else {
		return result;
	};
	let Node::Text(message) = reason.drop_meta() else {
		return result;
	};
	match shader_failure(message).and_then(|(word, line, column, why)| {
		place_in_program(code, line, column).map(|(line, column)| format!("{word}: {why} at {line}:{column}"))
	}) {
		Some(message) => crate::node::error(&message),
		None => result,
	}
}

/// Where the line and column of a shader's text are in `code`: in its one block with such a place
pub fn place_in_program(code: &str, line: usize, column: usize) -> Option<(usize, usize)> {
	let places: Vec<_> = shader_blocks(code)
		.into_iter()
		.filter_map(|((start_line, start_column), lines)| {
			let shader_line = lines.get(line.checked_sub(1)?)?;
			let text_column = written_column(&shader_line.text, column)?;
			let block_column = if shader_line.line == 0 { start_column } else { 1 };
			Some((
				start_line + shader_line.line,
				block_column + shader_line.dropped + text_column - 1,
			))
		})
		.collect();
	match places.as_slice() {
		[place] => Some(*place),
		_ => None,
	}
}

/// The column in a block's line of a column in the shader's line, where each `$name` is `values.name`; none past its end
fn written_column(line: &str, column: usize) -> Option<usize> {
	let characters: Vec<char> = line.chars().collect();
	let shader_length = |length: usize| {
		written(&characters[..length].iter().collect::<String>())
			.0
			.chars()
			.count()
	};
	(0..=characters.len())
		.find(|&length| shader_length(length) + 1 >= column)
		.map(|length| length + 1)
}

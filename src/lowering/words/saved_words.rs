//! `save` is one word for every value a file can hold, overloaded by its type (decision save-sound): a word naming
//! what a module makes, as save's argument, is the value of that module's class whose save overload writes it.
//! `save sound as "song.wav"`, `save(sound, "song.wav")` (card save-sound): the sounds made since the last save, lib/sound.warp's
//! Recording, a WAV file; `save canvas as "painting.png"` (card save-painting): the canvas of lib/draw.warp as drawn so
//! far, its Picture, a PNG. Unless the program names something so itself.

use super::nodes::{call, key};
use crate::modules::{says_use, uses_sound, SOUND_MODULE};
use crate::node::Node;
use crate::operators::Op;

const SAVE_WORD: &str = "save";
/// The word, the class it names as save's argument, the module that makes it
const SAVED_WORDS: [(&str, &str, &str); 2] = [("sound", "Recording", SOUND_MODULE), ("canvas", "Picture", "draw")];

pub fn lower(program: Node) -> Node {
	let used: Vec<(&str, &str)> = SAVED_WORDS
		.into_iter()
		.filter(
			|(_, _, module)| if *module == SOUND_MODULE { uses_sound(&program) } else { says_use(&program, module) },
		)
		.map(|(word, class, _)| (word, class))
		.collect();
	if used.is_empty() {
		return program;
	}
	let defined = super::music_words::defined_names(&program);
	let used: Vec<(&str, &str)> = used.into_iter().filter(|(word, _)| !defined.contains(*word)).collect();
	saved(program, &used)
}

fn saved(node: Node, words: &[(&str, &str)]) -> Node {
	match node.map_children(|child| saved(child, words)) {
		Node::List(items, bracket, separator)
			if !separator.separates_statements() && items.first().is_some_and(|word| word.is_symbol(SAVE_WORD)) =>
		{
			Node::List(items.into_iter().map(|item| made(item, words)).collect(), bracket, separator)
		}
		other => other,
	}
}

/// `sound` and `sound as "x.wav"`: Recording() and `Recording() as "x.wav"`
fn made(item: Node, words: &[(&str, &str)]) -> Node {
	let class = |word: &Node| words.iter().find(|(name, _)| word.is_symbol(name)).map(|(_, class)| call(class, vec![]));
	match item.drop_meta() {
		Node::Key(made, Op::As, file) => match class(made) {
			Some(value) => key(value, Op::As, file.as_ref().clone()),
			None => item,
		},
		word => class(word).map(|value| value.with_meta_of(&item)).unwrap_or(item),
	}
}

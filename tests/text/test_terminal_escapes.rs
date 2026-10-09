// `\e` is the escape character (bash, Ruby) and `\x1b` a byte in hex (Python, JS, Rust): terminal codes such as
// "\e[H" (cursor home) and "\x1b[2J" (clear) of samples/game_of_life.warp (card g_n-58); before, both lost the backslash
use crate::is;

#[test]
fn escape_and_hex_escapes_are_their_characters() {
	is!("\"\\e[H\" == \"\\u{1b}[H\"", 1);
	is!("\"\\x1b[2J\" == \"\\u{1b}[2J\"", 1);
	is!("\"\\x41\\x42\"", "AB");
	is!("#\"\\e[H\"", 3);
}

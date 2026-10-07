// card web-i18n (lib/i18n.wasp, notes/i18n.md): translations as data, a message per key and language, plural forms
// chosen by the CLDR category of `count`
use crate::common::fails_with;
use crate::eq;
use warp::wasm_emitter::eval;

const MESSAGES: &str = "use i18n\nmessages = { en: { cart: { one: \"{count} item\", other: \"{count} items\" }, hello: \"Hello {name}\" }, ru: { cart: { one: \"{count} товар\", few: \"{count} товара\", many: \"{count} товаров\" } } }\n";

fn translated(call: &str) -> warp::Node {
	eval(&format!("{MESSAGES}{call}"))
}

#[test]
fn a_message_takes_its_values_and_plural_form() {
	eq!(translated("translate(messages, \"en\", \"hello\", { name: \"Ana\" })"), "Hello Ana");
	eq!(translated("translate(messages, \"en\", \"cart\", { count: 1 })"), "1 item");
	eq!(translated("translate(messages, \"en\", \"cart\", { count: 3 })"), "3 items");
	eq!(translated("translate(messages, \"ru\", \"cart\", { count: 21 })"), "21 товар");
	eq!(translated("translate(messages, \"ru\", \"cart\", { count: 3 })"), "3 товара");
	eq!(translated("translate(messages, \"ru\", \"cart\", { count: 11 })"), "11 товаров");
}

#[test]
fn a_missing_translation_comes_from_the_first_language() {
	eq!(translated("translate(messages, \"ru-RU\", \"hello\", { name: \"Ana\" })"), "Hello Ana");
	eq!(translated("translate(messages, \"de\", \"cart\", { count: 2 })"), "2 items");
	fails_with(&format!("{MESSAGES}translate(messages, \"en\", \"bye\", {{}})"), "no translation of bye");
}

#[test]
fn plural_categories_follow_cldr() {
	let category = |language: &str, n: i64| eval(&format!("use i18n\nplural_category(\"{language}\", {n})"));
	eq!(category("en", 1), "one");
	eq!(category("en", 0), "other");
	eq!(category("fr", 0), "one");
	eq!(category("ja", 1), "other");
	eq!(category("pl", 22), "few");
	eq!(category("pl", 25), "many");
	eq!(category("cs", 3), "few");
	eq!(category("ar", 0), "zero");
	eq!(category("ar", 105), "few");
	eq!(category("ar", 111), "many");
}

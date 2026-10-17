// Signals in object fields (notes/signals.md phase 5, cards signal-param-field and signal-field-escape): a function
// listening to a field of its parameter, `watch(p) := on change p.age {…}`, subscribes to that field of the object it
// is given, so later writes of the field reach it; a variable kept in such a field stays a signal there
#[cfg(feature = "native")]
fn printed(code: &str) -> String {
	crate::common::printed(code).lines().filter(|line| !line.starts_with('»')).map(|line| format!("{line}\n")).collect()
}

#[test]
#[cfg(feature = "native")]
fn a_function_subscribes_to_a_field_of_its_parameter() {
	assert_eq!(printed("watch(p) := on change p.age {print \"age \" + value}\nperson = {age: 1}\nwatch(person)\nperson.age = 2\nperson.age = 2\nperson.age = 3"), "age 2\nage 3\n");
}

#[test]
#[cfg(feature = "native")]
fn a_signal_kept_in_a_field_stays_a_signal() {
	assert_eq!(printed("watch(p) := on change p.age {print \"age \" + value}\na = 1\nperson = {age: a}\nwatch(person)\na = 5\nperson.age = 6"), "age 5\nage 6\n");
}

#[test]
fn a_watched_field_keeps_its_value() {
	crate::is!("watch(p) := on set p.age {print value}\nperson = {age: 1}\nwatch(person)\nperson.age = person.age + 10\nperson.age * 2", 22);
}

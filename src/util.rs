use serde_json::json;

pub fn fetch(p0: &str) -> String {
	ureq::get(p0)
		.call()
		.unwrap()
		.body_mut()
		.read_to_string()
		.unwrap()
}
pub fn load_module(_p0: &str) {
	todo!()
}


pub fn show_type_name<T>(_: &T) {
	use std::any::{type_name, type_name_of_val};
	// println!("{}", type_name_of_val(*json!({"name": "Alice"})));
	println!("{}", type_name::<T>());
}

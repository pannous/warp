//! Generating the wrapper: the API's WIT and the Rust calling the crate
use super::*;

impl Api {
	pub(super) fn wit(&self, crate_name: &str) -> String {
		let mut wit = format!("package warp:{WRAPPER_PREFIX}{};\n\ninterface api {{\n", crate_name.replace('_', "-"));
		for resource in &self.resources {
			let kebab = kebab(&resource.name).unwrap_or_default();
			wit += &format!("  resource %{kebab} {{\n");
			for function in &resource.functions {
				wit += &format!("    {}\n", function_wit(function, true));
			}
			wit += "  }\n";
		}
		for function in &self.functions {
			wit += &format!("  {}\n", function_wit(function, false));
		}
		wit + "}\n\nworld wrapper {\n  export api;\n}\n"
	}

	/// The wrapper's Rust and which item each range of its lines belongs to
	pub(super) fn rust(&self, crate_name: &str) -> (String, Vec<(std::ops::Range<usize>, String)>) {
		let module = format!("exports::warp::{}::api", rust_name(&format!("{WRAPPER_PREFIX}{crate_name}")));
		let mut code = Code::default();
		code.line("#![allow(warnings, clippy::all)]", None);
		code.line("wit_bindgen::generate!({ world: \"wrapper\", path: \"wit\" });", None);
		code.line(&format!("use {module}::*;"), None);
		code.line("struct Wrapper;", None);
		code.line("export!(Wrapper);", None);
		for resource in &self.resources {
			let name = &resource.name;
			let key = Some(name.as_str());
			code.line(&format!("pub struct {name}Resource(std::cell::RefCell<::{}::{name}>);", self.crate_name), key);
			code.line(&format!("impl Guest{} for {name}Resource {{", camel(&kebab(name).unwrap_or_default())), key);
			for function in &resource.functions {
				let body = self.call(function, Some(name));
				code.line(&body, Some(&format!("{name}.{}", function.name)));
			}
			code.line("}", key);
		}
		code.line("impl Guest for Wrapper {", None);
		for resource in &self.resources {
			let name = &resource.name;
			let camel = camel(&kebab(name).unwrap_or_default());
			code.line(&format!("type {camel} = {name}Resource;"), Some(name));
		}
		for function in &self.functions {
			code.line(&self.call(function, None), Some(&function.name));
		}
		code.line("}", None);
		(code.text, code.ranges)
	}

	/// One wrapper function: its signature as wit-bindgen's trait has it and the call of the crate's function
	fn call(&self, function: &Function, resource: Option<&str>) -> String {
		let snake = kebab(&function.name).unwrap_or_default().replace('-', "_");
		let mut parameters: Vec<String> = vec![];
		if function.receiver.is_some() {
			parameters.push("&self".into());
		}
		let mut arguments = vec![];
		for (position, (_, conversion)) in function.parameters.iter().enumerate() {
			parameters.push(format!("a{position}: {}", conversion.rust));
			arguments.push(conversion.of(&format!("a{position}")));
		}
		let callee = match (function.receiver, resource) {
			(Some(Receiver::Mutable), _) => format!("self.0.borrow_mut().{}", function.name),
			(Some(Receiver::Shared), _) => format!("self.0.borrow().{}", function.name),
			(Some(Receiver::Owned), _) => format!("self.0.borrow().clone().{}", function.name),
			(Some(Receiver::Field), _) => format!("self.0.borrow().{}.clone()", function.name),
			(None, Some(resource)) => format!("::{}::{resource}::{}", self.crate_name, function.name),
			(None, None) => format!("::{}::{}", self.crate_name, function.name),
		};
		let call = match function.receiver {
			Some(Receiver::Field) => callee,
			_ => format!("{callee}({})", arguments.join(", ")),
		};
		let (returns, body) = match &function.result {
			Some(result) => (format!(" -> {}", result.rust), result.of(&call)),
			None => (String::new(), format!("{call};")),
		};
		format!("fn {snake}({}){returns} {{ {body} }}", parameters.join(", "))
	}
}

fn function_wit(function: &Function, in_resource: bool) -> String {
	let parameters: Vec<String> =
		function.parameters.iter().map(|(name, conversion)| format!("%{name}: {}", conversion.wit)).collect();
	let returns = match &function.result {
		Some(result) => format!(" -> {}", result.wit),
		None => String::new(),
	};
	let kind = match in_resource && function.receiver.is_none() {
		true => "static func",
		false => "func",
	};
	format!("%{}: {kind}({}){returns};", kebab(&function.name).unwrap_or_default(), parameters.join(", "))
}

/// Generated source, remembering which item each line belongs to
#[derive(Default)]
struct Code {
	text: String,
	ranges: Vec<(std::ops::Range<usize>, String)>,
	lines: usize,
}

impl Code {
	fn line(&mut self, line: &str, item: Option<&str>) {
		self.lines += 1;
		if let Some(item) = item {
			self.ranges.push((self.lines..self.lines + 1, item.to_string()));
		}
		self.text += line;
		self.text += "\n";
	}
}

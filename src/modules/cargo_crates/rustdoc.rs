//! Reading the crate's API from rustdoc's JSON: its functions and the structs it constructs
use super::*;

/// A crate's value as WIT and the Rust converting it: `expression` has `{}` where the value goes
#[derive(Clone)]
pub(super) struct Conversion {
	pub(super) wit: String,
	pub(super) rust: String,
	pub(super) expression: String,
}

impl Conversion {
	fn new(wit: &str, rust: &str, expression: &str) -> Conversion {
		Conversion { wit: wit.into(), rust: rust.into(), expression: expression.into() }
	}
	fn same(wit: &str, rust: &str) -> Conversion {
		Conversion::new(wit, rust, "{}")
	}
	/// `list<T>` / `Vec<T>` of `element`'s types
	fn list_of(element: &Conversion, expression: &str) -> Conversion {
		Conversion::new(&format!("list<{}>", element.wit), &format!("Vec<{}>", element.rust), expression)
	}
	/// `option<T>` / `Option<T>` of `inner`'s types
	fn option_of(inner: &Conversion, expression: &str) -> Conversion {
		Conversion::new(&format!("option<{}>", inner.wit), &format!("Option<{}>", inner.rust), expression)
	}
	/// `borrow<regex-set>` / `RegexSetBorrow<'_>` of the resource wrapping the Rust type `resource`
	fn borrowed(resource: &str, expression: &str) -> Option<Conversion> {
		let kebab = kebab(resource)?;
		Some(Conversion::new(&format!("borrow<{kebab}>"), &format!("{}Borrow<'_>", camel(&kebab)), expression))
	}
	pub(super) fn of(&self, value: &str) -> String {
		self.expression.replace("{}", value)
	}
	fn is_identity(&self) -> bool {
		self.expression == "{}"
	}
}

/// The receiver of a method: `&self`, `&mut self`, `self`
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Receiver {
	Shared,
	Mutable,
	Owned,
	/// a public field read: `self.major`
	Field,
}

#[derive(Clone)]
pub(super) struct Function {
	/// rust name as written in the crate
	pub(super) name: String,
	pub(super) parameters: Vec<(String, Conversion)>,
	pub(super) result: Option<Conversion>,
	pub(super) receiver: Option<Receiver>,
}

pub(super) struct Resource {
	/// the name at the crate's root: `Regex`
	pub(super) name: String,
	pub(super) functions: Vec<Function>,
}

/// What the wrapper exports: the crate's free functions and its structs as resources
pub(super) struct Api {
	pub(super) crate_name: String,
	pub(super) functions: Vec<Function>,
	pub(super) resources: Vec<Resource>,
}

/// Rust's primitive types and their WIT names
pub(super) const PRIMITIVES: [(&str, &str); 12] = [
	("bool", "bool"),
	("char", "char"),
	("i8", "s8"),
	("i16", "s16"),
	("i32", "s32"),
	("i64", "s64"),
	("u8", "u8"),
	("u16", "u16"),
	("u32", "u32"),
	("u64", "u64"),
	("f32", "f32"),
	("f64", "f64"),
];
/// a struct implementing Display gets the method to_string
pub(super) const DISPLAY_TRAIT: &str = "Display";
pub(super) const TO_STRING: &str = "to_string";
pub(super) const STRING_TRAITS: [&str; 5] = ["AsRef", "Borrow", "Into", "ToString", "Display"];

pub(super) fn last_segment(path: &str) -> &str {
	path.rsplit("::").next().unwrap_or(path)
}

pub(super) fn type_arguments(path: &Value) -> Vec<&Value> {
	path["args"]["angle_bracketed"]["args"].as_array().into_iter().flatten().filter_map(|arg| arg.get("type")).collect()
}

/// The WIT name of a Rust name, if it is a valid one (each segment starts with a lowercase letter)
pub(super) fn kebab(rust: &str) -> Option<String> {
	let kebab = crate::modules::components::kebab_case(rust);
	let valid = kebab.split('-').all(|segment| {
		segment.starts_with(|c: char| c.is_ascii_lowercase()) && segment.chars().all(|c| c.is_ascii_alphanumeric())
	});
	valid.then_some(kebab)
}

/// `regex-set` → `RegexSet`, the name wit-bindgen gives a resource
pub(super) fn camel(kebab: &str) -> String {
	kebab
		.split('-')
		.map(|segment| {
			let mut chars = segment.chars();
			chars.next().map(|first| first.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
		})
		.collect()
}

/// Reads rustdoc JSON: the items at the crate's root (re-exports followed) and how their types convert
struct Reader<'a> {
	index: &'a Value,
	crate_name: String,
	/// the structs a function constructs, by rustdoc id: their name at the root
	resources: HashMap<String, String>,
}

impl<'a> Reader<'a> {
	fn item(&self, id: &Value) -> Option<&'a Value> {
		self.index.get(id.to_string().trim_matches('"'))
	}

	/// The root's public items by the name they have there: `use` re-exports followed, glob re-exports expanded
	fn root_items(&self, module: &'a Value, items: &mut Vec<(String, &'a Value)>, seen: &mut HashSet<String>) {
		for item in self.items(&module["inner"]["module"]["items"]) {
			if let Some(reexport) = item["inner"].get("use") {
				let Some(target) = reexport.get("id").and_then(|id| self.item(id)) else {
					continue;
				};
				let key = reexport["id"].to_string();
				if reexport["is_glob"] == true {
					if seen.insert(key) && target["inner"].get("module").is_some() {
						self.root_items(target, items, seen);
					}
				} else if let Some(name) = reexport["name"].as_str() {
					items.push((name.to_string(), target));
				}
			} else if item["visibility"] == "public" {
				if let Some(name) = item["name"].as_str() {
					items.push((name.to_string(), item));
				}
			}
		}
	}

	/// The trait bounds of a generic parameter, from the function's generics
	fn bounds(generics: &'a Value, name: &str) -> Vec<&'a Value> {
		let declared = generics["params"]
			.as_array()
			.into_iter()
			.flatten()
			.filter(|param| param["name"] == name)
			.flat_map(|param| param["kind"]["type"]["bounds"].as_array().into_iter().flatten());
		let predicates = generics["where_predicates"]
			.as_array()
			.into_iter()
			.flatten()
			.filter_map(|predicate| predicate.get("bound_predicate"))
			.filter(|predicate| predicate["type"]["generic"] == name)
			.flat_map(|predicate| predicate["bounds"].as_array().into_iter().flatten());
		declared.chain(predicates).collect()
	}

	/// A parameter given as a generic with these bounds: a text for `AsRef<str>`, `Into<String>`…, bytes for
	/// `AsRef<[u8]>`; any other trait is tried as a text too and dropped if the compiler refuses
	fn bounded(&self, bounds: &[&Value]) -> Option<Conversion> {
		let traits: Vec<&Value> =
			bounds.iter().filter_map(|bound| bound.get("trait_bound")).map(|bound| &bound["trait"]).collect();
		let bytes = traits
			.iter()
			.any(|t| type_arguments(t).first().is_some_and(|argument| argument["slice"]["primitive"] == "u8"));
		match traits.as_slice() {
			[] => None,
			_ if bytes => Some(Conversion::same("list<u8>", "Vec<u8>")),
			_ if traits.iter().all(|t| STRING_TRAITS.contains(&last_segment(t["path"].as_str().unwrap_or("")))) => {
				Some(Conversion::same("string", "String"))
			}
			_ => Some(Conversion::new("string", "String", "{}.as_str()")),
		}
	}

	/// The type a plain type alias names: `HammingResult` is `Result<usize, StrSimError>`
	fn aliased(&self, ty: &Value) -> Option<&'a Value> {
		let alias = self.item(&ty["resolved_path"]["id"])?["inner"].get("type_alias")?;
		alias["generics"]["params"].as_array().is_some_and(|params| params.is_empty()).then(|| &alias["type"])
	}

	fn resource_of(&self, ty: &Value, self_type: Option<&str>) -> Option<String> {
		if ty["generic"] == "Self" {
			return self_type.map(String::from);
		}
		let path = ty.get("resolved_path")?;
		if !type_arguments(path).is_empty() {
			return None;
		}
		self.resources.get(&path["id"].to_string()).cloned()
	}

	fn parameter(&self, ty: &Value, generics: &Value, self_type: Option<&str>) -> Option<Conversion> {
		if let Some(aliased) = self.aliased(ty) {
			return self.parameter(aliased, generics, self_type);
		}
		if let Some(primitive) = ty["primitive"].as_str() {
			return primitive_conversion(primitive, true);
		}
		if let Some(name) = ty["generic"].as_str().filter(|name| *name != "Self") {
			return self.bounded(&Reader::bounds(generics, name));
		}
		if let Some(bounds) = ty["impl_trait"].as_array() {
			return self.bounded(&bounds.iter().collect::<Vec<_>>());
		}
		if let Some(reference) = ty.get("borrowed_ref") {
			let inner = &reference["type"];
			let mutable = reference["is_mutable"] == true;
			if let Some(resource) = self.resource_of(inner, self_type) {
				let borrow = if mutable { "borrow_mut" } else { "borrow" };
				return Conversion::borrowed(
					&resource,
					&format!("&{}*{{}}.get::<{resource}Resource>().0.{borrow}()", if mutable { "mut " } else { "" }),
				);
			}
			if mutable {
				return None;
			}
			if inner["primitive"] == "str" {
				return Some(Conversion::new("string", "String", "&{}"));
			}
			if let Some(element) = inner.get("slice") {
				let element = self.element(element)?;
				return Some(match element.is_identity() {
					true => Conversion::list_of(&element, "&{}"),
					false => Conversion::list_of(
						&element,
						&format!("&{{}}.iter().map(|x| {}).collect::<Vec<_>>()", element.of("x")),
					),
				});
			}
			if let Some(name) = inner["generic"].as_str().filter(|name| *name != "Self") {
				return self
					.bounded(&Reader::bounds(generics, name))
					.map(|text| Conversion { expression: "&{}".into(), ..text });
			}
			return None;
		}
		if let Some(resource) = self.resource_of(ty, self_type) {
			return Conversion::borrowed(&resource, &format!("{{}}.get::<{resource}Resource>().0.borrow().clone()"));
		}
		let path = ty.get("resolved_path")?;
		let arguments = type_arguments(path);
		match (last_segment(path["path"].as_str()?), arguments.as_slice()) {
			("String", []) => Some(Conversion::same("string", "String")),
			("PathBuf", []) => Some(Conversion::new("string", "String", "{}.into()")),
			("Vec", [element]) => {
				let element = self.parameter(element, generics, self_type).filter(Conversion::is_identity)?;
				Some(Conversion::list_of(&element, "{}"))
			}
			("Option", [inner]) => {
				let inner = self.parameter(inner, generics, self_type)?;
				let expression = match inner.expression.as_str() {
					"{}" => "{}",
					"&{}" => "{}.as_deref()",
					_ => return None,
				};
				Some(Conversion::option_of(&inner, expression))
			}
			_ => None,
		}
	}

	/// An element of a slice parameter, borrowed from the wrapper's Vec: `&[&str]` from `Vec<String>`
	fn element(&self, ty: &Value) -> Option<Conversion> {
		if let Some(primitive) = ty["primitive"].as_str() {
			return primitive_conversion(primitive, true).filter(Conversion::is_identity);
		}
		if ty["borrowed_ref"]["type"]["primitive"] == "str" {
			return Some(Conversion::new("string", "String", "{}.as_str()"));
		}
		let path = ty.get("resolved_path")?;
		(last_segment(path["path"].as_str()?) == "String").then(|| Conversion::new("string", "String", "{}.clone()"))
	}

	fn result(&self, ty: &Value, self_type: Option<&str>) -> Option<Conversion> {
		if let Some(aliased) = self.aliased(ty) {
			return self.result(aliased, self_type);
		}
		if let Some(primitive) = ty["primitive"].as_str() {
			return primitive_conversion(primitive, false);
		}
		if let Some(resource) = self.resource_of(ty, self_type) {
			let kebab = kebab(&resource)?;
			let camel = camel(&kebab);
			return Some(Conversion::new(
				&kebab,
				&camel,
				&format!("{camel}::new({resource}Resource(std::cell::RefCell::new({{}})))"),
			));
		}
		if let Some(reference) = ty.get("borrowed_ref") {
			let inner = &reference["type"];
			if inner["primitive"] == "str" {
				return Some(Conversion::new("string", "String", "{}.to_string()"));
			}
			let element = self.result(inner.get("slice")?, self_type).filter(Conversion::is_identity)?;
			return Some(Conversion::list_of(&element, "{}.to_vec()"));
		}
		if let Some(items) = ty["tuple"].as_array() {
			let items: Vec<Conversion> = items
				.iter()
				.map(|item| self.result(item, self_type).filter(Conversion::is_identity))
				.collect::<Option<_>>()?;
			let wit: Vec<&str> = items.iter().map(|item| item.wit.as_str()).collect();
			let rust: Vec<&str> = items.iter().map(|item| item.rust.as_str()).collect();
			return Some(Conversion::same(&format!("tuple<{}>", wit.join(", ")), &format!("({})", rust.join(", "))));
		}
		let path = ty.get("resolved_path")?;
		let arguments = type_arguments(path);
		match (last_segment(path["path"].as_str()?), arguments.as_slice()) {
			("String", []) => Some(Conversion::same("string", "String")),
			("PathBuf", []) => Some(Conversion::new("string", "String", "{}.to_string_lossy().into_owned()")),
			("Cow", [inner]) if inner["primitive"] == "str" => {
				Some(Conversion::new("string", "String", "{}.into_owned()"))
			}
			("Cow", [inner]) => {
				let element = self.result(inner.get("slice")?, self_type).filter(Conversion::is_identity)?;
				Some(Conversion::list_of(&element, "{}.into_owned()"))
			}
			("Vec", [element]) => {
				let element = self.result(element, self_type)?;
				Some(match element.is_identity() {
					true => Conversion::list_of(&element, "{}"),
					false => Conversion::list_of(
						&element,
						&format!("{{}}.into_iter().map(|x| {}).collect()", element.of("x")),
					),
				})
			}
			("Option", [inner]) => {
				let inner = self.result(inner, self_type)?;
				Some(match inner.is_identity() {
					true => Conversion::option_of(&inner, "{}"),
					false => Conversion::option_of(&inner, &format!("{{}}.map(|x| {})", inner.of("x"))),
				})
			}
			("Result", [ok, ..]) => {
				let unit = ok["tuple"].as_array().is_some_and(|items| items.is_empty());
				let (wit, rust, mapped) = match unit {
					true => ("_".to_string(), "()".to_string(), "{}".to_string()),
					false => {
						let ok = self.result(ok, self_type)?;
						let mapped = match ok.is_identity() {
							true => "{}".to_string(),
							false => format!("{{}}.map(|x| {})", ok.of("x")),
						};
						(ok.wit, ok.rust, mapped)
					}
				};
				Some(Conversion::new(
					&format!("result<{wit}, string>"),
					&format!("Result<{rust}, String>"),
					&format!("{mapped}.map_err(|e| e.to_string())"),
				))
			}
			_ => None,
		}
	}

	fn function(&self, name: &str, item: &Value, self_type: Option<&str>) -> Option<Function> {
		let function = item["inner"].get("function")?;
		let header = &function["header"];
		if header["is_unsafe"] == true || header["is_async"] == true {
			return None;
		}
		kebab(name)?;
		let generics = &function["generics"];
		let inputs = function["sig"]["inputs"].as_array()?;
		let mut receiver = None;
		let mut parameters = vec![];
		for (position, input) in inputs.iter().enumerate() {
			let (parameter, ty) = (input[0].as_str().unwrap_or(""), &input[1]);
			if position == 0 && parameter == "self" {
				receiver = Some(match ty.get("borrowed_ref") {
					Some(reference) if reference["is_mutable"] == true => Receiver::Mutable,
					Some(_) => Receiver::Shared,
					None => Receiver::Owned,
				});
				continue;
			}
			let parameter = kebab(parameter).unwrap_or_else(|| format!("arg{position}"));
			parameters.push((parameter, self.parameter(ty, generics, self_type)?));
		}
		let output = &function["sig"]["output"];
		let result = match output.is_null() {
			true => None,
			false => Some(self.result(output, self_type)?),
		};
		Some(Function { name: name.to_string(), parameters, result, receiver })
	}

	/// The public functions of a struct's inherent impls
	fn methods(&self, name: &str, item: &Value) -> Vec<Function> {
		let mut methods: Vec<Function> = self
			.inherent_items(item)
			.filter(|method| method["visibility"] == "public")
			.filter_map(|method| self.function(method["name"].as_str()?, method, Some(name)))
			.collect();
		for (field, result) in self.fields(item, name) {
			if !methods.iter().any(|method| method.name == field) {
				methods.push(Function {
					name: field,
					parameters: vec![],
					result: Some(result),
					receiver: Some(Receiver::Field),
				});
			}
		}
		if self.implements(item, DISPLAY_TRAIT) && !methods.iter().any(|method| method.name == TO_STRING) {
			methods.push(Function {
				name: TO_STRING.into(),
				parameters: vec![],
				result: Some(Conversion::same("string", "String")),
				receiver: Some(Receiver::Shared),
			});
		}
		methods
	}

	/// The items a list of ids names
	fn items(&self, ids: &'a Value) -> impl Iterator<Item = &'a Value> + '_ {
		ids.as_array().into_iter().flatten().filter_map(|id| self.item(id))
	}

	/// The impl blocks of a struct
	fn impls(&self, item: &'a Value) -> impl Iterator<Item = &'a Value> + '_ {
		self.items(&item["inner"]["struct"]["impls"])
	}

	/// The items of a struct's inherent impls (those of no trait)
	fn inherent_items(&self, item: &'a Value) -> impl Iterator<Item = &'a Value> + '_ {
		let inherent = self.impls(item).filter(|implementation| implementation["inner"]["impl"]["trait"].is_null());
		inherent.flat_map(|implementation| self.items(&implementation["inner"]["impl"]["items"]))
	}

	/// The public fields of a struct whose values cross: each read by a method of its name (`v.major()`)
	fn fields(&self, item: &'a Value, name: &str) -> Vec<(String, Conversion)> {
		self.items(&item["inner"]["struct"]["kind"]["plain"]["fields"])
			.filter(|field| field["visibility"] == "public")
			.filter_map(|field| {
				let result = self.result(field["inner"].get("struct_field")?, Some(name))?;
				Some((kebab(field["name"].as_str()?).map(|_| field["name"].as_str())??.to_string(), result))
			})
			.collect()
	}

	fn implements(&self, item: &Value, trait_name: &str) -> bool {
		self.impls(item).any(|implementation| {
			implementation["inner"]["impl"]["trait"]["path"]
				.as_str()
				.is_some_and(|path| last_segment(path) == trait_name)
		})
	}

	/// Does a function of the struct make one (`new` returning `Self`, `Result<Self, E>`, `Option<Self>`)?
	fn constructs(&self, item: &Value) -> bool {
		self.inherent_items(item).filter_map(|method| method["inner"].get("function")).any(|function| {
			let output = &function["sig"]["output"];
			let is_self = |ty: &Value| ty["generic"] == "Self" || ty["resolved_path"]["id"] == item["id"];
			is_self(output) || type_arguments(&output["resolved_path"]).first().is_some_and(|ty| is_self(ty))
		})
	}
}

/// A primitive as a parameter (`usize` from a u64) or a result (`usize` as a u64)
pub(super) fn primitive_conversion(primitive: &str, parameter: bool) -> Option<Conversion> {
	let sized = |wit: &str, rust: &str, native: &str| match parameter {
		true => Conversion::new(wit, rust, &format!("{{}} as {native}")),
		false => Conversion::new(wit, rust, &format!("{{}} as {rust}")),
	};
	match primitive {
		"usize" => Some(sized("u64", "u64", "usize")),
		"isize" => Some(sized("s64", "i64", "isize")),
		_ => PRIMITIVES.iter().find(|(rust, _)| *rust == primitive).map(|(rust, wit)| Conversion::same(wit, rust)),
	}
}

impl Api {
	pub(super) fn of(doc: &Value, crate_name: &str) -> Api {
		let index = &doc["index"];
		let mut reader = Reader { index, crate_name: crate_name.to_string(), resources: HashMap::new() };
		let root = reader.item(&doc["root"]).cloned().unwrap_or(Value::Null);
		let mut items = vec![];
		reader.root_items(&root, &mut items, &mut HashSet::new());
		let structs: Vec<(String, &Value)> = items
			.iter()
			.filter(|(name, item)| {
				let plain =
					item["inner"]["struct"]["generics"]["params"].as_array().is_some_and(|params| params.is_empty());
				plain && reader.constructs(item) && kebab(name).is_some()
			})
			.map(|(name, item)| (name.clone(), *item))
			.collect();
		for (name, item) in &structs {
			let id = item["id"].to_string();
			reader.resources.insert(id, name.clone());
		}
		let mut names = HashSet::new();
		let resources: Vec<Resource> = structs
			.iter()
			.filter(|(name, _)| names.insert(kebab(name).unwrap_or_default()))
			.map(|(name, item)| Resource { name: name.clone(), functions: reader.methods(name, item) })
			.collect();
		let functions = items
			.iter()
			.filter_map(|(name, item)| reader.function(name, item, None))
			.filter(|function| names.insert(kebab(&function.name).unwrap_or_default()))
			.collect();
		Api { crate_name: reader.crate_name, functions, resources }
	}

	/// Item keys: `escape`, `Regex`, `Regex.is_match`
	pub(super) fn drop_items(&mut self, refused: &HashSet<String>) {
		self.functions.retain(|function| !refused.contains(&function.name));
		self.resources.retain(|resource| !refused.contains(&resource.name));
		for resource in &mut self.resources {
			let name = resource.name.clone();
			resource.functions.retain(|function| !refused.contains(&format!("{name}.{}", function.name)));
		}
	}
}

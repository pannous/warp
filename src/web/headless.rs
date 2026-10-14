//! Headless pages (card web-testing, notes/web_framework.md step 14): a program whose value is markup runs as the
//! playground runs it, without a page. `Page::render(code)` runs main and keeps the run; `click("Add")` calls the
//! handler of the element showing "Add" (`on·click·N·node`, element_events.rs) and reads the page anew (`page·value`,
//! event_signals.rs), as playground.js and worker.js do; `text()` and `html()` are what the page shows.
//! Natively the run is a wasmtime instance; in the browser tests it is the host's last listening run (host.js
//! pageEventOutcome through warp_host.page_event).
//!
//! ```ignore
//! let mut page = Page::render("count = 0\ndiv{ button{ on click { count += 1 } \"Add\" } p{ count } }")?;
//! page.click("Add")?;
//! assert_eq!(page.text(), "Add1");
//! ```

use crate::element_events::HANDLER_ATTRIBUTE_PREFIX;
use crate::node::Node;
use crate::web::markup::to_html;
use serde_json::{json, Map, Value};

/// the attribute naming the component instance an element belongs to (component_state.rs), passed as event.instance
const INSTANCE_ATTRIBUTE: &str = "data-warp-instance";
const INSTANCE: &str = "instance";
const CLICK: &str = "click";
const INPUT: &str = "input";
const SUBMIT: &str = "submit";
const CHANGE: &str = "change";
const FOCUS: &str = "focus";
const BLUR: &str = "blur";
/// the submitted form's named fields, as markup.js submitDetail hands them over
const FIELDS: &str = "fields";
/// Fields whose typed value reaches the handler as a number, as the page's valueAsNumber gives it (markup.js)
const NUMBER_FIELDS: [&str; 2] = ["number", "range"];
/// the labels a form field is found by besides its text
const FIELD_LABELS: [&str; 4] = ["placeholder", "aria-label", "name", "id"];
/// elements without content or closing tag
const VOID_ELEMENTS: [&str; 13] =
	["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];
/// elements whose content the page does not show as text
const HIDDEN_CONTENT: [&str; 2] = ["style", "script"];
const ENTITIES: [(&str, &str); 5] = [("&lt;", "<"), ("&gt;", ">"), ("&quot;", "\""), ("&#39;", "'"), ("&amp;", "&")];

pub struct Page {
	markup: Node,
	#[cfg(feature = "native")]
	run: native::Run,
}

impl Page {
	/// Runs the program; Err is its value when it fails or needs no module
	pub fn render(code: &str) -> Result<Page, Node> {
		// compiled for a page: its page events do come; it runs where the server's code is, so it calls it directly
		let module = crate::pipeline::for_a_page(|| crate::pipeline::prerendering(|| crate::pipeline::compile(code)))?;
		#[cfg(feature = "native")]
		let page = native::Run::start(module).map(|(markup, run)| Page { markup, run });
		#[cfg(not(feature = "native"))]
		let page = match crate::pipeline::run_module(module) {
			failure @ Node::Error(_) => Err(failure),
			markup => Ok(Page { markup }),
		};
		page
	}

	/// What the page shows: the program's markup, read anew after each event
	pub fn markup(&self) -> &Node {
		&self.markup
	}

	pub fn html(&self) -> String {
		to_html(&self.markup)
	}

	/// The text the page shows, without style sheets and scripts
	pub fn text(&self) -> String {
		shown(&self.html()).1
	}

	/// The events the program dispatched to the page so far (card page-custom-event): an emit no handler received,
	/// its name and detail (as warp text), as the browser's page gets them as CustomEvents
	#[cfg(feature = "native")]
	pub fn dispatched(&self) -> &[(String, String)] {
		self.run.dispatched()
	}

	/// A click on the element showing `label` (its text) with a click handler
	pub fn click(&mut self, label: &str) -> Result<(), String> {
		self.fire(CLICK, label, |_| Map::new())
	}

	/// `text` typed into the form field found by its placeholder, name or id (`input{ bind: name }`)
	pub fn type_into(&mut self, label: &str, text: &str) -> Result<(), String> {
		self.fire(INPUT, label, |field| field_detail(field, text))
	}

	/// `text` committed in the form field found by its placeholder, name or id (`on change {…}`)
	pub fn change(&mut self, label: &str, text: &str) -> Result<(), String> {
		self.fire(CHANGE, label, |field| field_detail(field, text))
	}

	/// The form field found by its placeholder, name or id gets the focus (`on focus {…}`)
	pub fn focus(&mut self, label: &str) -> Result<(), String> {
		self.fire(FOCUS, label, |_| Map::new())
	}

	/// The form field found by its placeholder, name or id loses the focus (`on blur {…}`)
	pub fn blur(&mut self, label: &str) -> Result<(), String> {
		self.fire(BLUR, label, |_| Map::new())
	}

	/// The form showing `label` (its text, name or id) sent with `fields`, its named fields (`on submit { event.fields }`)
	pub fn submit(&mut self, label: &str, fields: Value) -> Result<(), String> {
		self.fire(SUBMIT, label, |_| Map::from_iter([(FIELDS.to_string(), fields)]))
	}

	/// The event on the element showing `label`, with `detail` as its `event` (and the element's component instance)
	fn fire(
		&mut self,
		event: &str,
		label: &str,
		detail_of: impl FnOnce(&Element) -> Map<String, Value>,
	) -> Result<(), String> {
		let handler_attribute = format!("{HANDLER_ATTRIBUTE_PREFIX}{event}");
		let shows = |element: &Element| {
			element.text.trim() == label || FIELD_LABELS.iter().any(|name| element.attribute(name) == Some(label))
		};
		let element = shown(&self.html())
			.0
			.into_iter()
			.find(|element| element.attribute(&handler_attribute).is_some() && shows(element))
			.ok_or_else(|| format!("no element showing {label:?} handles {event}; the page: {}", self.html()))?;
		let mut detail = detail_of(&element);
		if let Some(instance) = element.attribute(INSTANCE_ATTRIBUTE).and_then(|instance| instance.parse::<i64>().ok())
		{
			detail.insert(INSTANCE.to_string(), json!(instance));
		}
		let page_event = format!("{event}·{}", element.attribute(&handler_attribute).expect("found by it"));
		self.markup = self.handled(&page_event, &Value::Object(detail))?;
		Ok(())
	}

	#[cfg(feature = "native")]
	fn handled(&mut self, page_event: &str, detail: &Value) -> Result<Node, String> {
		self.run.handled(page_event, detail)
	}

	#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
	fn handled(&mut self, page_event: &str, detail: &Value) -> Result<Node, String> {
		match crate::web::page_event_in_host(page_event, detail) {
			Node::Error(problem) => Err(problem.serialize()),
			markup => Ok(markup),
		}
	}

	#[cfg(not(any(target_arch = "wasm32", feature = "native")))]
	fn handled(&mut self, _page_event: &str, _detail: &Value) -> Result<Node, String> {
		Err("this build of warp has no runner".to_string())
	}
}

/// An element as the page shows it: its attributes and its text
struct Element {
	attributes: Vec<(String, String)>,
	text: String,
}

impl Element {
	fn attribute(&self, name: &str) -> Option<&str> {
		self.attributes.iter().find(|(attribute, _)| attribute == name).map(|(_, value)| value.as_str())
	}
}

/// The elements of the page's HTML in document order, and the text the whole page shows
fn shown(html: &str) -> (Vec<Element>, String) {
	let (mut found, mut open, mut text) = (Vec::<Element>::new(), Vec::<usize>::new(), String::new());
	let mut rest = html;
	while !rest.is_empty() {
		let Some(tag_start) = rest.find('<') else {
			add_text(&mut found, &open, &mut text, rest);
			break;
		};
		add_text(&mut found, &open, &mut text, &rest[..tag_start]);
		let tag_end = rest[tag_start..].find('>').map_or(rest.len(), |end| tag_start + end);
		let tag = &rest[tag_start + 1..tag_end];
		rest = rest.get(tag_end + 1..).unwrap_or_default();
		if tag.starts_with('/') {
			open.pop();
			continue;
		}
		let name = tag.split_whitespace().next().unwrap_or_default().to_string();
		if HIDDEN_CONTENT.contains(&name.as_str()) {
			let closing = format!("</{name}>");
			rest = rest.find(&closing).map_or("", |end| &rest[end + closing.len()..]);
			continue;
		}
		found.push(Element { attributes: attributes(&tag[name.len()..]), text: String::new() });
		if !VOID_ELEMENTS.contains(&name.as_str()) {
			open.push(found.len() - 1);
		}
	}
	(found, text)
}

fn add_text(found: &mut [Element], open: &[usize], text: &mut String, html: &str) {
	let plain = ENTITIES.iter().fold(html.to_string(), |plain, (entity, character)| plain.replace(entity, character));
	open.iter().for_each(|&element| found[element].text.push_str(&plain));
	text.push_str(&plain);
}

/// ` data-warp-click="1" checked`: the attributes of a start tag, a bare one with an empty value
fn attributes(mut rest: &str) -> Vec<(String, String)> {
	let mut found = vec![];
	loop {
		rest = rest.trim_start();
		let name_end = rest.find(|character: char| character == '=' || character.is_whitespace()).unwrap_or(rest.len());
		if name_end == 0 {
			return found;
		}
		let name = rest[..name_end].to_string();
		rest = &rest[name_end..];
		let value = match rest.strip_prefix("=\"") {
			Some(quoted) => {
				let end = quoted.find('"').unwrap_or(quoted.len());
				rest = quoted.get(end + 1..).unwrap_or_default();
				quoted[..end].to_string()
			}
			None => String::new(),
		};
		found.push((name, value));
	}
}

#[cfg(feature = "native")]
mod native {
	use crate::event_signals::{HANDLER_PREFIX, PAGE_VALUE};
	use crate::host::HostState;
	use crate::node::{Bracket, Node, Separator};
	use crate::pipeline::CompiledModule;
	use crate::run::wasm_reader::{run_main_kept, val_to_node, with_trap_detail, Imports};
	use crate::wasm_emitter::failed_run;
	use serde_json::Value;
	use wasmtime::{AsContextMut, Instance, Store, Val};

	const NODE_SUFFIX: &str = "·node";

	/// A program's instance, kept after main for its handlers
	pub(super) struct Run {
		store: Store<HostState>,
		instance: Instance,
	}

	impl Run {
		pub(super) fn start(module: CompiledModule) -> Result<(Node, Run), Node> {
			let imports = Imports { host: module.needs_host, wasi: module.needs_wasi, ffi: module.needs_ffi };
			let (value, mut store, instance) = run_main_kept(&module.bytes, imports).map_err(failed_run)?;
			let markup = val_to_node(&value, &mut store, &instance).unwrap_or_else(failed_run);
			Ok((markup, Run { store, instance }))
		}

		pub(super) fn dispatched(&self) -> &[(String, String)] {
			&self.store.data().dispatched
		}

		/// Runs the handler `on·click·1·node` with `[detail]`, then reads page·value
		pub(super) fn handled(&mut self, page_event: &str, detail: &Value) -> Result<Node, String> {
			let arguments =
				Node::List(vec![crate::modules::foreign::node_of(detail)], Bracket::Square, Separator::None);
			self.call(&format!("{HANDLER_PREFIX}{page_event}{NODE_SUFFIX}"), Some(arguments))?;
			self.call(PAGE_VALUE, None)
		}

		/// Calls an export of the program, with one node argument or none
		fn call(&mut self, name: &str, argument: Option<Node>) -> Result<Node, String> {
			let function = self
				.instance
				.get_func(&mut self.store, name)
				.ok_or_else(|| format!("the program exports no {name}"))?;
			let arguments = match argument {
				Some(argument) => vec![self.built(&argument)?],
				None => vec![],
			};
			let mut results: Vec<Val> = function
				.ty(&self.store)
				.results()
				.map(|result| Val::default_for_ty(&result).unwrap_or(Val::AnyRef(None)))
				.collect();
			let outcome = function.call(&mut self.store, &arguments, &mut results);
			with_trap_detail(outcome, &mut self.store, &self.instance)
				.map_err(|failure| failed_run(failure).serialize())?;
			let result = results.first().copied().unwrap_or(Val::AnyRef(None));
			val_to_node(&result, &mut self.store, &self.instance).map_err(|failure| failure.to_string())
		}

		/// A node built inside the program's instance, as a host word's answer is
		fn built(&mut self, node: &Node) -> Result<Val, String> {
			use crate::host::tasks::{Builders, TaskValue};
			let value = TaskValue::of(node).map_err(|problem| problem.to_string())?;
			let (instance, store) = (self.instance, &mut self.store);
			let builders = Builders::of(&mut |export| instance.get_export(&mut *store, export))
				.map_err(|problem| problem.to_string())?;
			builders.build(&value, &mut self.store.as_context_mut()).map_err(|problem| problem.to_string())
		}
	}
}

/// What a number field gives for `text`: a whole number as an int, as JSON.stringify writes valueAsNumber, no number null
fn number_typed(text: &str) -> Value {
	match text.trim().parse::<f64>() {
		Ok(number) if number.fract() == 0.0 && number.abs() < i64::MAX as f64 => json!(number as i64),
		Ok(number) => json!(number),
		Err(_) => Value::Null,
	}
}

/// What a form field holds after `text`, as markup.js inputDetail hands it over: a number from a number or range field
fn field_detail(field: &Element, text: &str) -> Map<String, Value> {
	let value = match field.attribute("type") {
		Some(kind) if NUMBER_FIELDS.contains(&kind) => number_typed(text),
		_ => json!(text),
	};
	Map::from_iter([("value".to_string(), value), ("checked".to_string(), json!(false))])
}

//! `try X else Y`, `try X catch e { Y }` and `assert C else X` as templates
use super::*;
use crate::lowering::error_classes::ROOT_ERROR_CLASS;
use crate::wasm_emitter::text::text_builtins::{ERROR_CLASS, RAISE_ON, TEXT_FORM};
use crate::words::INSTANCE_OF;

/// The placeholders of the `try`/`assert` templates
const TRY_VALUE_PLACEHOLDER: &str = "try_placeholder_value";
const TRY_FALLBACK_PLACEHOLDER: &str = "try_placeholder_fallback";
const LIST_PLACEHOLDER: &str = "try_placeholder_list";
const INDEX_PLACEHOLDER: &str = "try_placeholder_index";
const DIVIDEND_PLACEHOLDER: &str = "try_placeholder_dividend";
const DIVISOR_PLACEHOLDER: &str = "try_placeholder_divisor";
const ASSERT_CONDITION_PLACEHOLDER: &str = "assert_placeholder_condition";

impl Lowering {
	/// A list or text written in place (`[1 2]#3`, `"ab"#5`): `count` of it cannot fail
	fn is_known_list(&self, list: &Node) -> bool {
		matches!(list.drop_meta(), Node::List(_, Bracket::Square, _) | Node::Text(_))
	}

	/// `try X else Y`: X, or Y when X fails. Errors are values: an Error result is replaced. An index out of range and a
	/// division or modulo by zero directly under `try` are checked before they happen; a runtime error deeper inside X
	/// (an index in a sum, a called function) is caught as a wasm exception (`ran_without_error`, wasm_emitter/control/try_guard.rs).
	pub(super) fn lower_try(&self, guarded: Node, fallback: Node) -> Node {
		let (value, fallback_placeholder) = (TRY_VALUE_PLACEHOLDER, TRY_FALLBACK_PLACEHOLDER);
		match guarded.drop_meta() {
			Node::Key(target, Op::Assign, assigned) if matches!(target.drop_meta(), Node::Symbol(_)) => {
				Node::Key(target.clone(), Op::Assign, Box::new(self.lower_try(assigned.as_ref().clone(), fallback)))
			}
			// a list written in place: the index is checked before it happens; any other value
			// may be no list at all (`n#1`, `xs#2#1`), which the exception path catches as not_a_list
			Node::Key(list, Op::Hash, index) if !matches!(list.drop_meta(), Node::Empty) && self.is_known_list(list) => self.instantiate_template(
				&format!("(try_tmp_list={LIST_PLACEHOLDER}; try_tmp_index={INDEX_PLACEHOLDER}; if try_tmp_index >= 1 and try_tmp_index <= count(try_tmp_list) {{try_tmp_list#try_tmp_index}} else {{{fallback_placeholder}}})"),
				&[(LIST_PLACEHOLDER, list), (INDEX_PLACEHOLDER, index), (fallback_placeholder, &fallback)],
			),
			// the divisor checked first; any other error of the division (a text divided) is caught as below
			Node::Key(dividend, op @ (Op::Div | Op::Mod | Op::Rem), divisor) => self.instantiate_template(
				&format!("(try_tmp_divisor={DIVISOR_PLACEHOLDER}; if try_tmp_divisor==0 {{{fallback_placeholder}}} else {{\
					try_tmp_finished={RAN_WITHOUT_ERROR}({{try_tmp_value={DIVIDEND_PLACEHOLDER} {op} try_tmp_divisor}}); \
					if try_tmp_finished and not is_error(try_tmp_value) {{try_tmp_value}} else {{{fallback_placeholder}}}}})"),
				&[(DIVISOR_PLACEHOLDER, divisor), (DIVIDEND_PLACEHOLDER, dividend), (fallback_placeholder, &fallback)],
			),
			_ => self.instantiate_template(
				// the guarded assignment comes first, so the kind of try_tmp_value is known where it is read
				&format!("(try_tmp_finished={RAN_WITHOUT_ERROR}({{try_tmp_value={value}}}); if try_tmp_finished and not is_error(try_tmp_value) {{try_tmp_value}} else {{{fallback_placeholder}}})"),
				&[(value, &guarded), (fallback_placeholder, &fallback)],
			),
		}
	}

	/// `try X catch e { Y }`: Y runs with e the caught Error: the Error X gave back, else the runtime error it raised
	/// (caught_error, try_guard.rs)
	/// `except C as e { Y }` binds `e: C`: Y runs for an Error of the class C, any other Error is raised on with its class
	/// (decision error-classes); `except Error` catches every one
	pub(super) fn lower_try_binding(&self, guarded: Node, fallback: Node, binding: &Node) -> Node {
		let (value, fallback_placeholder) = (TRY_VALUE_PLACEHOLDER, TRY_FALLBACK_PLACEHOLDER);
		let (name, class) = match binding.drop_meta() {
			Node::Key(name, Op::Colon, class) => (name.as_ref().clone(), Some(class.drop_meta().name())),
			_ => (binding.clone(), None),
		};
		let handled = match class.filter(|class| class != ROOT_ERROR_CLASS) {
			Some(class) if !self.error_classes.is_error_class(&class) => {
				return crate::node::error(&format!(
					"except {class}: {class} is no error class: declare it with `class {class}: Error`"
				));
			}
			Some(class) => {
				let caught = self.error_classes.caught_by(&class);
				let tests: Vec<String> = caught
					.iter()
					.map(|class| format!("{INSTANCE_OF}({CAUGHT_BINDING_PLACEHOLDER}, \"{class}\")"))
					.collect();
				format!(
					"if {} {{{fallback_placeholder}}} else \
					{{{RAISE_ON}({TEXT_FORM}({CAUGHT_BINDING_PLACEHOLDER}), {ERROR_CLASS}({CAUGHT_BINDING_PLACEHOLDER}))}}",
					tests.join(" or ")
				)
			}
			None => fallback_placeholder.to_string(),
		};
		self.instantiate_template(
			&format!("(try_tmp_finished={RAN_WITHOUT_ERROR}({{try_tmp_value={value}}}); if try_tmp_finished and not is_error(try_tmp_value) {{try_tmp_value}} else {{\
				{CAUGHT_BINDING_PLACEHOLDER} = {CAUGHT_ERROR}(try_tmp_finished, try_tmp_value); {handled}}})"),
			&[(value, &guarded), (fallback_placeholder, &fallback), (CAUGHT_BINDING_PLACEHOLDER, &name)],
		)
	}

	/// `assert C else X` is 1 when C holds and throws X otherwise, also mid-program (`check C`, card check-assert); without
	/// a message the error says the assertion failed
	pub(super) fn lower_assert(&self, condition: Node, message: Node) -> Node {
		let message = match message.drop_meta() {
			Node::Empty => Node::Text(format!("{}: {}", crate::warp_parser::ASSERTION_FAILED, condition.serialize())),
			_ => message,
		};
		let condition_placeholder = ASSERT_CONDITION_PLACEHOLDER;
		self.instantiate_template(
			&format!("(if {condition_placeholder} {{1}} else {{{}({TRY_FALLBACK_PLACEHOLDER})}})", RAISE_WORD),
			&[(condition_placeholder, &condition), (TRY_FALLBACK_PLACEHOLDER, &message)],
		)
	}
}

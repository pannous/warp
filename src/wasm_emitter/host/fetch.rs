//! `fetch url`: the host's fetch import, a Text of the body or an Error of the reason
use super::super::*;

impl WasmGcEmitter {
	/// Emit a fetch call using the host.fetch import (host.fetch_within for an explicit timeout)
	/// Returns a Text node with the body, or an Error node with the reason: the host marks a failure by a negative length
	pub(crate) fn emit_fetch_call(
		&mut self,
		func: &mut Function,
		url_node: &Node,
		timeout: Option<std::time::Duration>,
	) {
		let url = self.extract_url_string(url_node);
		// a URL known only at run time (a text variable, a parameter, a concatenation, a value of run-time kind such as
		// `server + "/json"` of a parameter given ø somewhere), else the URL as written
		let run_time_value = crate::analyzer::is_run_time_kind(&self.get_type(url_node)) && !url_node.is_empty_node();
		if self.is_runtime_text(url_node) || run_time_value {
			self.emit_run_time_ptr_len(func, url_node);
		} else {
			let (url_ptr, url_len) = self.memory_string(&url);
			Self::emit_list(func, &[I32Const(url_ptr as i32), I32Const(url_len as i32)]);
		}
		let import = match timeout {
			Some(timeout) => {
				func.instruction(&I::I64Const(timeout.as_millis().min(i64::MAX as u128) as i64));
				"host_fetch_within"
			}
			None => "host_fetch",
		};
		if let Some(f) = self.ctx.func_registry.get(import) {
			func.instruction(&I::Call(f.call_index as u32));
		} else {
			if timeout.is_some() {
				func.instruction(&I::Drop);
			}
			let reason = format!("fetch {url} failed: host imports are not available");
			let (ptr, len) = self.memory_string(&reason);
			Self::emit_list(func, &[I32Const(ptr as i32), I32Const(-(len as i32))]);
		}
		let (len, ptr) = (self.scratch(0), self.scratch(1));
		Self::emit_list(func, &[I::I64ExtendI32S, I::LocalSet(len), I::I64ExtendI32U, I::LocalSet(ptr)]);
		self.emit_host_text_result(func, len, ptr);
	}

	/// Extract URL string from parsed node tree
	/// Handles patterns like: https://... which parses as Key(Symbol("https"), Colon, ...)
	pub(crate) fn extract_url_string(&self, node: &Node) -> String {
		fn node_to_string(n: &Node) -> String {
			match n.drop_meta() {
				Node::Symbol(s) => s.clone(),
				Node::Text(s) => s.clone(),
				Node::Key(left, op, right) => {
					let left_str = node_to_string(left);
					let op_str = match op {
						Op::Colon => ":",
						Op::Div => "/",
						Op::Dot => ".",
						_ => "",
					};
					let right_str = node_to_string(right);
					format!("{}{}{}", left_str, op_str, right_str)
				}
				Node::List(items, _, _) => items.iter().map(node_to_string).collect::<Vec<_>>().join(""),
				Node::Error(inner) => {
					// Handle parse errors - check if it's an "Unexpected character '/'" error
					if let Node::Text(msg) = inner.drop_meta() {
						if msg.contains("Unexpected character '/'") {
							return "/".to_string();
						}
					}
					// Otherwise recurse into the inner node
					node_to_string(inner)
				}
				_ => format!("{:?}", n),
			}
		}
		node_to_string(node)
	}
}

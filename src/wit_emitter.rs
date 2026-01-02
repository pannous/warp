use crate::extensions::numbers::Number;
use crate::node::{Bracket, DataType, Node};

pub struct WitEmitter {
    indent_level: usize,
    output: String,
}

impl WitEmitter {
    pub fn new() -> Self {
        WitEmitter {
            indent_level: 0,
            output: String::new(),
        }
    }

    fn indent(&self) -> String {
        "  ".repeat(self.indent_level)
    }

    fn emit_line(&mut self, line: &str) {
        self.output.push_str(&self.indent());
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn emit_blank(&mut self) {
        self.output.push('\n');
    }

    pub fn emit_node_type_definitions(&mut self) {
        self.emit_line("// WebAssembly Interface Types for Node AST");
        self.emit_blank();

        // Emit Number variant
        self.emit_line("variant number {");
        self.indent_level += 1;
        self.emit_line("int(s64),");
        self.emit_line("float(f64),");
        self.emit_line("quotient(tuple<s64, s64>),");
        self.emit_line("complex(tuple<f64, f64>),");
        self.indent_level -= 1;
        self.emit_line("}");
        self.emit_blank();

        // Emit Bracket enum
        self.emit_line("enum bracket {");
        self.indent_level += 1;
        self.emit_line("curly,");
        self.emit_line("square,");
        self.emit_line("round,");
        self.indent_level -= 1;
        self.emit_line("}");
        self.emit_blank();

        // Emit Kind enum
        self.emit_line("enum kind {");
        self.indent_level += 1;
        self.emit_line("object,");
        self.emit_line("group,");
        self.emit_line("pattern,");
        self.indent_level -= 1;
        self.emit_line("}");
        self.emit_blank();

        // Emit DataType enum
        self.emit_line("enum data-type {");
        self.indent_level += 1;
        self.emit_line("vec,");
        self.emit_line("tuple,");
        self.emit_line("struct,");
        self.emit_line("primitive,");
        self.emit_line("string,");
        self.emit_line("other,");
        self.indent_level -= 1;
        self.emit_line("}");
        self.emit_blank();

        // Emit Meta record
        self.emit_line("record meta {");
        self.indent_level += 1;
        self.emit_line("comment: option<string>,");
        self.emit_line("line: option<u32>,");
        self.emit_line("column: option<u32>,");
        self.indent_level -= 1;
        self.emit_line("}");
        self.emit_blank();

        // Emit Data record
        self.emit_line("record data {");
        self.indent_level += 1;
        self.emit_line("type-name: string,");
        self.emit_line("data-type: data-type,");
        self.indent_level -= 1;
        self.emit_line("}");
        self.emit_blank();

        // Emit main Node variant (recursive)
        self.emit_line("variant node {");
        self.indent_level += 1;
        self.emit_line("empty,");
        self.emit_line("number(number),");
        self.emit_line("text(string),");
        self.emit_line("symbol(string),");
        self.emit_line("key-value(tuple<string, node>),");
        self.emit_line("pair(tuple<node, node>),");
        self.emit_line("tag(tuple<string, node, node>),");
        self.emit_line("block(tuple<list<node>, kind, bracket>),");
        self.emit_line("list(list<node>),");
        self.emit_line("data(data),");
        self.emit_line("with-meta(tuple<node, meta>),");
        self.indent_level -= 1;
        self.emit_line("}");
        self.emit_blank();
    }

    pub fn emit_interface(&mut self, package_name: &str, interface_name: &str) {
        self.emit_line(&format!("package {}:{};", package_name, interface_name));
        self.emit_blank();
        self.emit_line(&format!("interface {} {{", interface_name));
        self.indent_level += 1;
        self.emit_blank();

        self.emit_node_type_definitions();

        // Emit core functions
        self.emit_line("// Parse WASP format to Node");
        self.emit_line("parse: func(input: string) -> Node;");
        self.emit_blank();

        self.emit_line("// Serialize Node to JSON");
        self.emit_line("to-json: func(node: node) -> result<string, string>;");
        self.emit_blank();

        self.emit_line("// Serialize Node to WASP format");
        self.emit_line("to-wasp: func(node: node) -> string;");
        self.emit_blank();

        self.indent_level -= 1;
        self.emit_line("}");
        self.emit_blank();

        // Emit world
        self.emit_line(&format!("world {} {{", interface_name));
        self.indent_level += 1;
        self.emit_line(&format!("export {};", interface_name));
        self.indent_level -= 1;
        self.emit_line("}");
    }

    pub fn get_output(&self) -> &str {
        &self.output
    }

    pub fn emit_to_file(&self, path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;
        file.write_all(self.output.as_bytes())?;
        Ok(())
    }
}

/// Convert a Node to its WIT representation as a string
pub fn node_to_wit_value(node: &Node) -> String {
    match node {
        Node::Empty => "empty".to_string(),
        Node::Number(n) => match n {
            Number::Int(i) => format!("i64.const({})", i),
            Number::Float(f) => format!("f64.const({})", f),
            _ => todo!("Number variant not implemented in WIT conversion"),
        },
        Node::Text(s) => format!("text(\"{}\")", escape_string(s)),
        Node::Char(c) => format!("codepoint('{}')", c),
        Node::Symbol(s) => format!("symbol(\"{}\")", escape_string(s)),
        Node::Key(k, v) => {
            format!(
                "key-value((\"{}\", {}))",
                escape_string(k),
                node_to_wit_value(v)
            )
        }
        Node::Pair(a, b) => {
            format!("pair(({}, {}))", node_to_wit_value(a), node_to_wit_value(b))
        }
        Node::Tag {
            title,
            params,
            body,
        } => {
            format!(
                "tag((\"{}\", {}, {}))",
                escape_string(title),
                node_to_wit_value(params),
                node_to_wit_value(body)
            )
        }
        Node::List(items, bracket) => {
            let items_str = items
                .iter()
                .map(node_to_wit_value)
                .collect::<Vec<_>>()
                .join(", ");
            let bracket_str = match bracket {
                Bracket::Curly => "curly",
                Bracket::Square => "square",
                Bracket::Round => "round",
                Bracket::Other(_, _) => "curly", // fallback
            };
            // Curly brackets -> block, others -> list
            match bracket {
                Bracket::Curly => format!("block(([{}], {}))", items_str, bracket_str),
                _ => format!("list(([{}], {}))", items_str, bracket_str),
            }
        }
        Node::Data(_dada) => {
            // format!("list([{}])", dada) // Dada doesn't implement fmt::Display
            "[data?]".to_string() // data LOSS!
        }
        Node::Meta { node, data: meta } => {
            let comment = if let Some(c) = &meta.comment {
                format!("some(\"{}\")", escape_string(c))
            } else {
                "none".to_string()
            };
            let line = if let Some(l) = meta.line {
                format!("some({})", l)
            } else {
                "none".to_string()
            };
            let column = if let Some(c) = meta.column {
                format!("some({})", c)
            } else {
                "none".to_string()
            };
            format!(
                "with-meta(({}, {{ comment: {}, line: {}, column: {} }}))",
                node_to_wit_value(node),
                comment,
                line,
                column
            )
        }
        Node::Error(e) => format!("error(\"{}\")", escape_string(e)),
        Node::False => "false".to_string(),
        Node::True => "true".to_string(),
        _ => todo!("no wit"),
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eq;
    use crate::meta::MetaData;

    #[test]
    fn test_emit_wit_interface() {
        let mut emitter = WitEmitter::new();
        emitter.emit_interface("wasp", "ast");

        let output = emitter.get_output();
        println!("{}", output);

        assert!(output.contains("package wasp:ast"));
        assert!(output.contains("variant node"));
        assert!(output.contains("variant number"));
        assert!(output.contains("record meta"));
        assert!(output.contains("parse: func"));
        assert!(output.contains("to-json: func"));
    }

    #[test]
    #[ignore]
    fn test_node_to_wit_value() {
        // todo this has currently nothing to do with Wit lol
        let node = Node::int(42);
        let wit = node_to_wit_value(&node);
        eq!(wit, "number(int(42))");

        let node = Node::text("hello");
        let wit = node_to_wit_value(&node);
        eq!(wit, "text(\"hello\")");

        let node = Node::keys("name", "Alice");
        let wit = node_to_wit_value(&node);
        assert!(wit.contains("key-value"));
        assert!(wit.contains("name"));
        assert!(wit.contains("Alice"));
    }

    #[test]
    fn test_node_with_meta_to_wit() {
        let node = Node::int(42).with_meta(MetaData::with_position(10, 5));
        let wit = node_to_wit_value(&node);

        println!("{}", wit);
        assert!(wit.contains("with-meta"));
        assert!(wit.contains("line: some(10)"));
        assert!(wit.contains("column: some(5)"));
    }

    #[test]
    #[ignore]
    fn test_complex_node_to_wit() {
        let node = Node::list(vec![Node::int(1), Node::int(2), Node::text("hello")]);

        let wit = node_to_wit_value(&node);
        println!("{}", wit);

        assert!(wit.contains("list(["));
        assert!(wit.contains("number(int(1))"));
        assert!(wit.contains("number(int(2))"));
        assert!(wit.contains("text(\"hello\")"));
    }
}

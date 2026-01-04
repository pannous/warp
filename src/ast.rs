use crate::node::Node;
use std::ops::Deref;

macro_rules! ast_type {
	($name:ident, $tag:literal) => {
		#[derive(Clone, Copy, Debug)]
		struct $name<'a>(&'a Node);

		impl<'a> $name<'a> {
			#[inline]
			fn try_from(n: &'a Node) -> Option<Self> {
				match n {
					Node::Key(key, ..) => match key.as_ref() {
						Node::Symbol(title) | Node::Text(title) if title == $tag => Some(Self(n)),
						_ => None,
					},
					_ => None,
				}
			}
		}

		impl<'a> std::ops::Deref for $name<'a> {
			type Target = Node;
			#[inline]
			fn deref(&self) -> &Node {
				self.0
			}
		}
	};
}

// - zero-sized wrapper
// - checked once
// - statically enforced afterwards
// struct ClassDeclaration<'a>(&'a Node);
// impl<'a> ClassDeclaration<'a> {
//     fn try_from(n: &'a Node) -> Option<Self> {
//         match n {
//             Node::Tag { title, .. } if title == "class" => Some(Self(n)),
//             _ => None,
//         }
//     }
//     fn as_node(self) -> &'a Node {
//         self.0
//     }
// }
//
// // fn f(n: &Node) {} now accepts &ClassDeclaration !!!
// impl<'a> Deref for ClassDeclaration<'a> {
//     type Target = Node;
//     fn deref(&self) -> &Node {
//         self.0
//     }
// }
// impl<'a> AsRef<Node> for ClassDeclaration<'a> {
//     fn as_ref(&self) -> &Node {
//         self.0
//     }
// }

ast_type!(ClassDeclaration, "class");
ast_type!(TypeExpr, "type");
ast_type!(Record, "record");
ast_type!(FunctionDeclaration, "function");
ast_type!(IfExpression, "if");
ast_type!(WhileExpression, "while");
ast_type!(ForExpression, "for");

fn walk<'a>(n: &'a Node, f: &mut impl FnMut(&'a Node)) {
	f(n);
	match n {
		Node::Key(k, v) => {
			walk(k, f);
			walk(v, f);
		}
		_ => {}
	}
}

// fn upgrade(n: Node) -> Node {
//     match n {
//         Node::Tag { title, params, body } if title == "class" => {
//             Node::Meta( Box::new(Node::Tag {
//                     title,
//                     params: Box::new(upgrade(*params)),
//                     body: Box::new(upgrade(*body)),
//                 }),Symbol("ClassDeclaration".to_string()) )
//             }
//         }
//         Node::Tag { title, params, body } => Node::Tag {
//             title,
//             params: Box::new(upgrade(*params)),
//             body: Box::new(upgrade(*body)),
//         },
//         other => other,
//     }
// }
//
// fn analyze_ast(n: &Node) -> Node {
//     walk(n, &mut |node| {
//         if let Some(class_decl) = ClassDeclaration::try_from(node) {
//             println!("Found class declaration: {:?}", class_decl);
//         }
//     });
//     n.clone()
// }

//! card empty-text-default: a field whose default is a text (`name = ""`, `name: "x"`) is a field with that default,
//! not one of the type the text names ("unknown type:  of field name")
use crate::is;

#[test]
fn a_text_default_is_no_type_name() {
	is!(r#"class P{name=""}; P("a").name"#, "a");
	is!(r#"class P{name="hello"}; P("a").name"#, "a");
	is!(r#"class P{name:"hello"}; P("a").name"#, "a");
	is!(r#"class P{name="hello"}; P().name"#, "hello");
	is!(r#"class P{name=""}; P().name"#, "");
	is!(r#"class P{name:text="x"}; P().name"#, "x");
}

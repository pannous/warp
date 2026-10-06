use warp::wasm_emitter::eval;
use crate::is;

#[test]
fn block_comment_with_hash_is_ignored() {
	is!("/# c #/ 3", 3);
	is!("3 /# c #/ + 4", 7);
	is!("/# multi\nline #/\n5", 5);
	is!("3", 3);
}

#[test]
fn doc_comments_do_not_change_behaviour() {
	is!("## doc\n3", 3);
	is!("/** doc */ 3", 3);
	is!("/** doc\n more */\nx=4; x*2", 8);
	is!("x=4; x*2", 8);
}

#[test]
fn backslash_at_line_end_continues_the_statement() {
	is!("1 + \\\n2", 3);
	is!("1 \\\n+ 2", 3);
	is!("x = 1 + \\\n  2 + \\\n  3; x", 6);
	is!("1 + 2", 3);
}

#[test]
fn elvis_takes_the_left_unless_it_is_falsy() {
	is!("ø?:1", 1);
	is!("2?:1", 2);
	is!("0?:7", 7);
	is!("x=ø; x?:5", 5);
	is!("x=3; x?:5", 3);
	is!("ø?:ø?:9", 9);
	is!("i=0; (i++ ?: 5); i", 1);
}

#[test]
fn hex_word_reads_a_hexadecimal_number() {
	is!("hex 1010", 4112);
	is!("hex 1010 + 1", 4113);
	is!("0x1010", 4112);
}

#[test]
fn final_constant_and_val_bind_once_like_const() {
	for keyword in ["const", "final", "constant", "val"] {
		let reassigned = eval(&format!("{keyword} x=\"hi\"; x=\"bye\"; x"));
		assert!(format!("{reassigned:?}").contains("Error"), "{keyword}: {reassigned:?}");
		is!(&format!("{keyword} x=3; x+1"), 4);
	}
}

//! `nand` (wiki/operator.md: `nand ¬&`) in every spelling, Unicode's ⊼ too (like ⊻ for xor): `not (a and b)`
use warp::is;

#[test]
fn test_nand_spellings() {
	for spelling in ["nand", "¬&", "⊼"] {
		is!(&format!("1 {spelling} 1"), 0);
		is!(&format!("1 {spelling} 0"), 1);
		is!(&format!("true {spelling} false"), 1);
	}
}

#[test]
fn test_nand_binds_like_and() {
	is!("1 ⊼ 1 or 1", 1); // (1 ⊼ 1) or 1
	is!("0 or 1 ⊼ 1", 0); // 0 or (1 ⊼ 1)
}

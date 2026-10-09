// card orm-one-letter-column: a one-letter name is a character once parsed ("n" is 'n'); a table or column so named works
use crate::is;

#[test]
fn a_one_letter_column_takes_rows() {
	is!("class Cup{label: text; n: int}\ncups: [Cup] = database.cups_lettered\ncups.add(Cup(\"big\", 2))\ncups#1.n", 2);
	is!("class Cup{label: text; n: int}\ncups: [Cup] = database.cups_lettered\nc = cups#1\nc.n += 1\ncups#1.n", 3);
}

#[test]
fn a_one_letter_table_takes_rows() {
	is!("class Pot{label: text}\npots: [Pot] = database.p\npots.add(Pot(\"tea\"))\npots#1.label", "tea");
}

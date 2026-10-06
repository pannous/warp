//! `import (sin, floor, fabs) from 'm'` imports every name of the group

use crate::is;

#[test]
fn a_group_of_three_names_is_imported() {
	is!("import (sin,floor,fabs) from 'm'; fabs(-1.5)", 1.5);
	is!("import (sin,floor,fabs) from 'm'; fabs(floor(sin(3.14159)))", 0.0);
}

#[test]
fn a_group_with_spaces_is_imported() {
	is!("import (sin, floor) from 'm'; floor(3.7)", 3);
}

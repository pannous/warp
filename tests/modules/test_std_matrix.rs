//! The standard library module matrix: a matrix is a list of rows, m[i][j] reads and writes an element
use crate::is;

fn shown(program: &str) -> String {
	format!("use matrix; string({program})")
}

#[test]
fn use_matrix_makes_matrices() {
	is!(&shown("zeros(2, 3)"), "[[0 0 0] [0 0 0]]");
	is!(&shown("ones(2, 2)"), "[[1 1] [1 1]]");
	is!(&shown("identity(3)"), "[[1 0 0] [0 1 0] [0 0 1]]");
	is!(&shown("matrix(2, 2, () => 3)"), "[[3 3] [3 3]]");
	is!("use matrix; m = random_matrix(2, 3); string(shape(m))", "[2 3]");
	is!("use matrix; m = random_matrix(3, 2); [rows(m), cols(m)] == [3, 2]", true);
}

#[test]
fn use_matrix_computes() {
	is!(&shown("transpose([[1, 2, 3], [4, 5, 6]])"), "[[1 4] [2 5] [3 6]]");
	is!(&shown("matrix_add([[1, 2], [3, 4]], [[10, 20], [30, 40]])"), "[[11 22] [33 44]]");
	is!(&shown("scale([[1, 2], [3, 4]], 0.5)"), "[[0.5 1] [1.5 2]]");
	is!(&shown("map_matrix([[1, 2], [3, 4]], x => x * x)"), "[[1 4] [9 16]]");
	is!("use matrix; dot([1, 2, 3], [4, 5, 6])", 32);
	is!(&shown("matmul([[1, 2], [3, 4]], [[5, 6], [7, 8]])"), "[[19 22] [43 50]]");
}

/// an element is written in place (m[i][j] = v), a float into a matrix of exact zeros as well; the module's own m and
/// row stay apart from the program's
#[test]
fn a_matrix_element_is_written() {
	is!("use matrix; m = zeros(2, 2); m[1][0] = 2.5; string(m)", "[[0 0] [2.5 0]]");
	is!("use matrix; m = random_matrix(2, 2); m[0][1] = m[0][1] + 5; m[0][1] - 5 + 1 > 0", true);
	is!("use matrix; row = 7; m = ones(1, 2); string([m, row])", "[[[1 1]] 7]");
}

/// numpy's names are the matrix words (Julia's zeros, ones and transpose already are): np.zeros((2, 3)) takes its shape
#[test]
fn numpy_names_are_matrix_words() {
	is!("string(np.zeros((2, 3)))", "[[0 0 0] [0 0 0]]");
	is!("string(np.eye(2))", "[[1 0] [0 1]]");
	is!("string(np.matmul([[1, 2], [3, 4]], [[5, 6], [7, 8]]))", "[[19 22] [43 50]]");
	is!("np.dot([1, 2], [3, 4])", 11);
	is!("np = 3; np + 1", 4);
}

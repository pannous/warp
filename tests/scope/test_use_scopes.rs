//! D15 revised (user, 2026-10-03): no folder scope by default; `use folder`, `use package` and `use project` make every
//! definition of the folder, the package (the folder holding <folder name>.warp, and below) or the project (the folder
//! holding .git, and below) visible, looked up lazily by name
use crate::is;
use crate::common::fails_with;

#[test]
fn siblings_are_invisible_without_a_scope() {
	fails_with("tests/warp/folder_scope/plain.warp", "area");
}

#[test]
fn use_package_sees_every_file_below_the_package_folder() {
	is!("tests/warp/shapes_package/app/main.warp", 13);
	fails_with("tests/warp/shapes_package/app/folder_only.warp", "circle_area");
}

#[test]
fn use_project_sees_every_file_below_the_project_root() {
	is!("tests/warp/project_scope/main.warp", 42);
}

#[test]
fn use_folder_in_inline_code_is_the_working_directory() {
	fails_with("use folder\nno_such_function_anywhere(1)", "no_such_function_anywhere");
	fails_with("use package\n1", "use package: no package folder");
}

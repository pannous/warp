//! D15 revised (user, 2026-10-03): no folder scope by default; `use folder`, `use package` and `use project` make every
//! definition of the folder, the package (the folder holding <folder name>.wasp, and below) or the project (the folder
//! holding .git, and below) visible, looked up lazily by name
use crate::common::fails_with;
use warp::*;

#[test]
fn siblings_are_invisible_without_a_scope() {
	fails_with("tests/wasp/folder_scope/plain.wasp", "area");
}

#[test]
fn use_package_sees_every_file_below_the_package_folder() {
	is!("tests/wasp/shapes_package/app/main.wasp", 13);
	fails_with("tests/wasp/shapes_package/app/folder_only.wasp", "circle_area");
}

#[test]
fn use_project_sees_every_file_below_the_project_root() {
	is!("tests/wasp/project_scope/main.wasp", 42);
}

#[test]
fn use_folder_in_inline_code_is_the_working_directory() {
	fails_with("use folder\nno_such_function_anywhere(1)", "no_such_function_anywhere");
	fails_with("use package\n1", "use package: no package folder");
}

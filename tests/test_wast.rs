use warp::eq;
use warp::wasp_parser::parse;

#[test]
#[ignore]
fn test_parse() {
	//	Mark::markmode();
	//	const let node : Node = Mark::parseFile("/Users/me/dev/wasm/test.wat");

	let wat = r#"(module
  (table (;0;) 1 1 funcref);
  (memory (;0;) 2);
  (export "memory" (memory 0));
  (export "add1" (func 0));
  (export "wasp_main" (func $main));

  (type $ii_i (func (param i32 i32) (result i32)));
  (func $add (type $ii_i) (param i32 i32) (result i32);
    local.get 0
    local.get 1
    i32.add
    );

  (func $main (type 0) (param i32 i32) (result i32);
	  local.get 0
	  local.get 1
	  (call $add);
	  drop
	  i32.const 21
	  i32.const 21
	  call $add
	  ;;(i32.const 42);
  );
)"#;

	let module = parse(wat);
	eq!(module, "module");
	// printf!("%s", module.toString());
	eq!(module.length(), 8);
	//	eq!(node.length(), 12);
	//	puts(node);
	eq!(module[0], "table");
	eq!(module[1], "memory");
	eq!(module[2], "export");
	eq!(module["func"].length(), 2);
	eq!(module["func"]["$main"]["param"].length(), 2);
}

#[test]
#[ignore]
fn test_wast() {
	// use_polish_notation = true;
	test_parse();
	// use_polish_notation = false;
}

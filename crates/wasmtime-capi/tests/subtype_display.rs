//! ref.test and ref.cast against open struct types answer as the type tree says (patches/subtype-display.patch): a
//! chain deeper than the display's columns, a sibling, each test asked twice (the second time from a filled row)
use wasmtime::{AnyRef, Config, Engine, Instance, Module, Rooted, Store, Val};

/// types $t0 <: … <: $t{CHAIN-1}, deeper than the display's 7 columns, and a sibling $s of $t1
const CHAIN: usize = 10;

fn module_text() -> String {
    let mut types = String::from("(type $t0 (sub (struct)))\n(type $s (sub $t0 (struct (field i64))))\n");
    let mut funcs = String::from("(func (export \"new_s\") (result anyref) (struct.new $s (i64.const 0)))\n");
    for k in 1..CHAIN {
        types += &format!("(type $t{k} (sub $t{} (struct)))\n", k - 1);
    }
    for k in 0..CHAIN {
        funcs += &format!(
            "(func (export \"new_{k}\") (result anyref) (struct.new $t{k}))
             (func (export \"is_{k}\") (param anyref) (result i32) (ref.test (ref $t{k}) (local.get 0)))
             (func (export \"cast_{k}\") (param anyref) (ref.cast (ref $t{k}) (local.get 0)) drop)\n"
        );
    }
    format!("(module {types} {funcs})")
}

#[test]
fn subtype_tests_follow_the_type_tree() {
    let mut config = Config::new();
    config.wasm_gc(true).wasm_function_references(true);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, module_text()).unwrap();
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let make = |store: &mut Store<()>, name: &str| -> Rooted<AnyRef> {
        let f = instance.get_typed_func::<(), Option<Rooted<AnyRef>>>(&mut *store, name).unwrap();
        f.call(store, ()).unwrap().unwrap()
    };
    // each object with the depths of the chain it is a subtype of
    let mut objects: Vec<(String, Vec<usize>)> = (0..CHAIN).map(|k| (format!("new_{k}"), (0..=k).collect())).collect();
    objects.push(("new_s".into(), vec![0]));
    for (maker, supertypes) in &objects {
        let object = make(&mut store, maker);
        for target in 0..CHAIN {
            let expected = supertypes.contains(&target);
            let is = instance.get_typed_func::<Option<Rooted<AnyRef>>, i32>(&mut store, &format!("is_{target}")).unwrap();
            for _ in 0..2 {
                assert_eq!(is.call(&mut store, Some(object)).unwrap() == 1, expected, "{maker} is $t{target}");
            }
            let cast = instance.get_func(&mut store, &format!("cast_{target}")).unwrap();
            let cast = cast.call(&mut store, &[Val::AnyRef(Some(object))], &mut []);
            assert_eq!(cast.is_ok(), expected, "{maker} cast to $t{target}");
        }
    }
}

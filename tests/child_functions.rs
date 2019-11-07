mod utils;

use ewasm::{Execute, RootRuntime};
use utils::escape;
use wabt::wat2wasm;

fn compile_wat(child_code: &str) -> Vec<u8> {
    let child_asm = wat2wasm(child_code).unwrap();

    wat2wasm(format!(
        r#"
            (module
                    (import "env" "eth2_exec" (func $exec (param i32) (param i32)))
                    (import "env" "eth2_expose" (func $expose (param i32) (param i32)))
                    (import "env" "eth2_return" (func $return (param i32) (param i32) (result i32)))
                    (import "env" "eth2_argument" (func $argument (param i32) (param i32) (result i32)))

                    (memory (export "memory") 1)
                    (data (i32.const 0) "some_func")
                    (data (i32.const 10) "{}")
                    (func $some_func
                        (export "some_func")
                        (param i32)
                        (param i32)
                        (result i32)

                        (; Read the argument, and check the result ;)
                        (drop (call $argument (i32.const 89) (i32.const 4)))
                        (if (i32.ne (i32.load (i32.const 89)) (i32.const 9999))
                            (then (unreachable)))

                        (; Return a value to the caller ;)
                        (i32.store (i32.const 99) (i32.const 8888))
                        (drop (call $return (i32.const 99) (i32.const 4)))

                        (i32.const 6654))

                    (func $main (export "main")
                        (call $expose (i32.const 0) (i32.const 9))
                        (call $exec (i32.const 10) (i32.const {})))
            )"#,
        escape(&child_asm),
        child_asm.len(),
    ))
    .unwrap()
}

#[test]
fn call() {
    let child_code = r#"
    (module
        (import
            "env"
            "eth2_call"
            (func
                $eth2_call
                (param i32)
                (param i32)
                (param i32)
                (param i32)
                (param i32)
                (param i32)
                (result i32)))
        (memory (export "memory") 1)
        (data (i32.const 0) "some_func")
        (func $main (export "main") (local $x i32)
            (i32.store (i32.const 10) (i32.const 9999))
            (set_local $x
                (call
                    $eth2_call
                    (i32.const 0)
                    (i32.const 9)
                    (i32.const 10)
                    (i32.const 4)
                    (i32.const 15)
                    (i32.const 4)))
            (if
                (i32.ne (get_local $x) (i32.const 6654))
                (then (unreachable)))
            (if
                (i32.ne (i32.load (i32.const 15)) (i32.const 8888))
                (then (unreachable)))))
    "#;

    let code = compile_wat(child_code);

    let mut runtime = RootRuntime::new(&code, &[], [0u8; 32]);
    runtime.execute();
}

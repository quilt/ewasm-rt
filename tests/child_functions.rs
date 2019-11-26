mod utils;

use ewasm::{Execute, RootRuntime};
use utils::escape;
use wabt::wat2wasm;

fn compile_wat(child_code: &str) -> Vec<u8> {
    let child_asm = wat2wasm(child_code).unwrap();

    wat2wasm(format!(
        r#"
        (module
            (import "env" "eth2_loadModule" (func $load (param i32) (param i32) (param i32)))
            (import "env" "eth2_expose" (func $expose (param i32) (param i32)))
            (import "env" "eth2_return" (func $return (param i32) (param i32) (result i32)))
            (import "env" "eth2_argument" (func $argument (param i32) (param i32) (result i32)))
            (import
                "env"
                "eth2_callModule"
                (func
                    $call
                    (param i32)
                    (param i32)
                    (param i32)
                    (param i32)
                    (param i32)
                    (param i32)
                    (param i32)
                    (result i32)))

            (memory (export "memory") 1)
            (data (i32.const 0) "some_func")
            (data (i32.const 10) "main")
            (data (i32.const 22) "{}")
            (func $some_func
                (export "some_func")
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
                (call $load (i32.const 0) (i32.const 22) (i32.const {}))
                (i32.store (i32.const 14) (i32.const 1234))
                (drop
                    (call
                        $call
                        (i32.const 0)   (; Slot ;)
                        (i32.const 10)  (; Name Offset ;)
                        (i32.const 4)   (; Name Length ;)
                        (i32.const 14)  (; Argument Offset ;)
                        (i32.const 4)   (; Argument Length ;)
                        (i32.const 18)  (; Return Offset ;)
                        (i32.const 4)   (; Return Length ;)
                    )
                )

                (; Check the returned buffer from the child runtime ;)
                (if
                    (i32.ne (i32.load (i32.const 18)) (i32.const 4321))
                    (then (unreachable)))
            )
        )
        "#,
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
            "eth2_return"
            (func
                $eth2_return
                (param i32)
                (param i32)
                (result i32)))
        (import
            "env"
            "eth2_argument"
            (func
                $eth2_argument
                (param i32)
                (param i32)
                (result i32)))
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
        (func $main (export "main") (result i32) (local $x i32)
            (; Check that the argument provided by the caller is 1234 ;)
            (drop (call $eth2_argument (i32.const 10) (i32.const 4)))
            (if
                (i32.ne (i32.load (i32.const 10)) (i32.const 1234))
                (then (unreachable)))

            (; Return a value to the caller ;)
            (i32.store (i32.const 10) (i32.const 4321))
            (drop (call $eth2_return (i32.const 10) (i32.const 4)))

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
                (then (unreachable))
            )

            (i32.const 6301)
        )
    )
    "#;

    let code = compile_wat(child_code);

    let mut runtime = RootRuntime::new(&code, &[], [0u8; 32]);
    runtime.execute();
}

#[cfg(all(test, feature = "debug"))]
#[cfg_attr(feature = "debug", test)]
fn print() {
    let child_code = r#"
    (module
        (import
            "env"
            "eth2_return"
            (func
                $eth2_return
                (param i32)
                (param i32)
                (result i32)))
        (import "env" "print" (func $print (param i32) (param i32)))
        (memory (export "memory") 1)
        (data (i32.const 0) "hello world")
        (func $main (export "main") (result i32) (local $x i32)
            (; print data ;)
            (call $print (i32.const 0) (i32.const 11))

            (; Return a value to the caller ;)
            (i32.store (i32.const 10) (i32.const 4321))
            (call $eth2_return (i32.const 10) (i32.const 4))

        )
    )
    "#;

    let code = compile_wat(child_code);
    let mut runtime = RootRuntime::new(&code, &[], [0u8; 32]);

    runtime.set_logger(Box::new(|b| {
        assert_eq!(b, "hello world");
    }));

    let _ = runtime.execute();
}

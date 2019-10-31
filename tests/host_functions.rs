use ewasm::{Execute, Runtime};
use wabt::wat2wasm;

fn compile_wat(code: &str) -> Vec<u8> {
    wabt::wat2wasm(
        [
            r#"
            (module
                    (import "env" "eth2_savePostStateRoot" (func $save_post_root (param i32)))
                    (memory (export "memory") 1)
                    (func $main (export "main")
            "#,
            code,
            r#"))"#,
        ]
        .concat(),
    )
    .unwrap()
}

fn build_root(n: u8) -> [u8; 32] {
    let mut ret = [0u8; 32];
    ret[0] = n;
    ret
}

#[test]
fn save_post_root() {
    let code = compile_wat(
        r#"
            (i32.const 0)
            (i32.const 42)
            (i32.store)
            (call $save_post_root (i32.const 0))
        "#,
    );

    let mut runtime = Runtime::new(&code, &[], [0u8; 32]);
    let post_root = runtime.execute();
    assert_eq!(post_root, build_root(42));
}

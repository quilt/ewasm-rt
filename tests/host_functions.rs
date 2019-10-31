use ewasm::{Execute, Runtime};
use wabt::wat2wasm;

fn compile_wat(code: &str) -> Vec<u8> {
    wat2wasm(
        [
            r#"
            (module
                    (import "env" "eth2_savePostStateRoot" (func $save_post_root (param i32)))
                    (import "env" "eth2_loadPreStateRoot" (func $load_pre_root (param i32)))
                    (import "env" "eth2_blockDataSize" (func $block_data_size (result i32)))
                    (import "env" "eth2_blockDataCopy" (func $block_data_copy (param i32) (param i32) (param i32)))
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
            (i32.store (i32.const 0) (i32.const 42))
            (call $save_post_root (i32.const 0))
        "#,
    );

    let mut runtime = Runtime::new(&code, &[], [0u8; 32]);
    let post_root = runtime.execute();
    assert_eq!(post_root, build_root(42));
}

#[test]
fn load_pre_root() {
    let code = compile_wat(
        r#"
            (call $load_pre_root (i32.const 0))
            (call $save_post_root (i32.const 0))
        "#,
    );

    let mut runtime = Runtime::new(&code, &[], build_root(42));
    let post_root = runtime.execute();
    assert_eq!(post_root, build_root(42));
}

#[test]
fn block_data_size() {
    let code = compile_wat(
        r#"
            ;; This stores the result of $block_data_size into memory address 0.
            (i32.store (i32.const 0) (call $block_data_size))
            (call $save_post_root (i32.const 0))
        "#,
    );

    let mut runtime = Runtime::new(&code, &[0u8; 42], build_root(42));
    let post_root = runtime.execute();
    assert_eq!(post_root, build_root(42));
}

#[test]
fn block_data_copy() {
    let code = compile_wat(
        r#"
            (call $block_data_copy (i32.const 0) (i32.const 0) (i32.const 32))
            (call $save_post_root (i32.const 0))
        "#,
    );

    let block_data = build_root(42);
    let mut runtime = Runtime::new(&code, &block_data, [0u8; 32]);
    let post_root = runtime.execute();
    assert_eq!(post_root, build_root(42));
}

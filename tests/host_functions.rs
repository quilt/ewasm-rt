mod utils;

use ewasm::{Execute, RootRuntime};
use utils::escape;
use wabt::wat2wasm;

fn nop() -> Vec<u8> {
    wat2wasm(r#"(module (func $main (export "main") (nop)))"#).unwrap()
}

fn compile_wat(code: &str) -> Vec<u8> {
    wat2wasm(
        [
            r#"
            (module
                    (import "env" "eth2_savePostStateRoot" (func $save_post_root (param i32)))
                    (import "env" "eth2_loadPreStateRoot" (func $load_pre_root (param i32)))
                    (import "env" "eth2_blockDataSize" (func $block_data_size (result i32)))
                    (import "env" "eth2_blockDataCopy" (func $block_data_copy (param i32) (param i32) (param i32)))
                    (import "env" "eth2_bufferGet" (func $buffer_get (param i32) (param i32) (param i32) (result i32)))
                    (import "env" "eth2_bufferSet" (func $buffer_set (param i32) (param i32) (param i32)))
                    (import "env" "eth2_bufferMerge" (func $buffer_merge (param i32) (param i32)))
                    (import "env" "eth2_bufferClear" (func $buffer_clear (param i32)))
                    (import "env" "eth2_exec" (func $exec (param i32) (param i32)))
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
fn exec() {
    let child_code = nop();

    let code = wat2wasm(format!(
        r#"
            (module
                    (import "env" "eth2_exec" (func $exec (param i32) (param i32)))
                    (memory (export "memory") 1)
                    (data (i32.const 0) "{}")
                    (func $main (export "main")
                        (call $exec (i32.const 0) (i32.const {}))
            ))"#,
        escape(&child_code),
        child_code.len(),
    ))
    .unwrap();

    let mut runtime = RootRuntime::new(&code, &[], [0u8; 32]);
    runtime.execute();
}

#[test]
fn save_post_root() {
    let code = compile_wat(
        r#"
            (i32.store (i32.const 0) (i32.const 42))
            (call $save_post_root (i32.const 0))
        "#,
    );

    let mut runtime = RootRuntime::new(&code, &[], [0u8; 32]);
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

    let mut runtime = RootRuntime::new(&code, &[], build_root(42));
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

    let mut runtime = RootRuntime::new(&code, &[0u8; 42], build_root(42));
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
    let mut runtime = RootRuntime::new(&code, &block_data, [0u8; 32]);
    let post_root = runtime.execute();
    assert_eq!(post_root, build_root(42));
}

#[test]
fn buffer_get_and_set() {
    let code = compile_wat(
        r#"
            (i32.store (i32.const 32) (i32.const 42))
            (call $buffer_set (i32.const 0) (i32.const 0) (i32.const 32))
            (call $buffer_get (i32.const 0) (i32.const 0) (i32.const 64))
            (drop)
            (call $save_post_root (i32.const 64))
        "#,
    );

    let mut runtime = RootRuntime::new(&code, &[], [0u8; 32]);
    let post_root = runtime.execute();
    assert_eq!(post_root, build_root(42));
}

#[test]
fn buffer_merge() {
    let code = compile_wat(
        r#"
            (i32.store (i32.const 0) (i32.const 1))
            (i32.store (i32.const 32) (i32.const 1))

            (i32.store (i32.const 64) (i32.const 2))
            (i32.store (i32.const 96) (i32.const 2))

            (i32.store (i32.const 128) (i32.const 2))
            (i32.store (i32.const 160) (i32.const 3))

            (i32.store (i32.const 192) (i32.const 4))
            (i32.store (i32.const 224) (i32.const 4))

            (call $buffer_set (i32.const 0) (i32.const 0) (i32.const 32))
            (call $buffer_set (i32.const 0) (i32.const 64) (i32.const 96))
            (call $buffer_set (i32.const 1) (i32.const 128) (i32.const 160))
            (call $buffer_set (i32.const 1) (i32.const 192) (i32.const 224))

            (call $buffer_merge (i32.const 0) (i32.const 1))

            (call $buffer_get (i32.const 0) (i32.const 0) (i32.const 256))
            (call $buffer_get (i32.const 0) (i32.const 64) (i32.const 288))
            (call $buffer_get (i32.const 0) (i32.const 192) (i32.const 310))

            (drop)
            (drop)
            (drop)

            ;; Store the result of buffer[0,1] + buffer[0,2] + buffer[0,4] at mem[342]
            (i32.store 
                (i32.const 342)
                (i32.add
                    (i32.add (i32.load (i32.const 256)) (i32.load (i32.const 288)))
                    (i32.add (i32.load (i32.const 310)) (i32.const 0))
                )
            )

            (call $save_post_root (i32.const 342))
        "#,
    );

    let mut runtime = RootRuntime::new(&code, &[], [0u8; 32]);
    let post_root = runtime.execute();

    // The post root should be 1 + 3 + 4 = 8
    assert_eq!(post_root, build_root(8));
}

#[test]
fn buffer_clear() {
    let code = compile_wat(
        r#"
            (i32.store (i32.const 0) (i32.const 2))
            (i32.store (i32.const 32) (i32.const 2))

            (i32.store (i32.const 64) (i32.const 1))
            (i32.store (i32.const 96) (i32.const 1))

            (call $buffer_set (i32.const 0) (i32.const 0) (i32.const 32))
            (call $buffer_set (i32.const 1) (i32.const 64) (i32.const 96))

            (call $buffer_clear (i32.const 1))

            (call $buffer_get (i32.const 0) (i32.const 0) (i32.const 128))
            (call $buffer_get (i32.const 1) (i32.const 64) (i32.const 160))

            (drop)
            (drop)

            ;; Store the result of buffer[0,1] + buffer[1,2] at mem[192]
            (i32.store 
                (i32.const 192)
                (i32.sub (i32.load (i32.const 128)) (i32.load (i32.const 160)))
            )

            (call $save_post_root (i32.const 192))
        "#,
    );

    let mut runtime = RootRuntime::new(&code, &[], [0u8; 32]);
    let post_root = runtime.execute();

    // The post root should be 2 - 0 = 2
    assert_eq!(post_root, build_root(2));
}

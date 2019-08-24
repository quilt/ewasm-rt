# Rune

A modular WebAssembly runtime for Ethereum 2.0.

## Interface

```rust
struct Runtime {
    code: &[u8],
    shard_pre_state: &[u8; 32],
    beacon_pre_state: &[u8; 32],
    data: &[u8],
}

trait Executor {
    pub fn execute() -> Result<RuntimeResult, Error>;
}
```

## Goals

The eventual goal is to build a runtime that can be plugged into stateless and stateful nodes.
Also, it would be great to make it easy to swap in alternative WebAssembly interpreters.

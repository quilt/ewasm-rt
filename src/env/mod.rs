pub mod child;
pub mod root;

use typed_builder::TypedBuilder;

use wasmi::{MemoryRef, RuntimeValue, Trap};

pub type ExtResult = Result<Option<RuntimeValue>, Trap>;

#[derive(Debug, Clone, TypedBuilder)]
struct StackFrame {
    memory: MemoryRef,

    argument_offset: u32,
    argument_length: u32,

    return_offset: u32,
    return_length: u32,
}

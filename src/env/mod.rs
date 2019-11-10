pub mod child;
pub mod root;

use typed_builder::TypedBuilder;

use wasmi::{MemoryInstance, MemoryRef, RuntimeValue, Trap};

pub type ExtResult = Result<Option<RuntimeValue>, Trap>;

#[derive(Debug, Clone, TypedBuilder)]
struct StackFrame {
    memory: MemoryRef,

    argument_offset: u32,
    argument_length: u32,

    return_offset: u32,
    return_length: u32,
}

impl StackFrame {
    pub fn transfer_argument(
        &self,
        dest: &MemoryRef,
        dest_ptr: u32,
        dest_len: u32,
    ) -> Result<u32, wasmi::Error> {
        let len = std::cmp::min(dest_len, self.argument_length);

        MemoryInstance::transfer(
            &self.memory,
            self.argument_offset as usize,
            dest,
            dest_ptr as usize,
            len as usize,
        )
        .map(|_| self.argument_length)
    }

    pub fn transfer_return(
        &self,
        src: &MemoryRef,
        src_ptr: u32,
        src_len: u32,
    ) -> Result<u32, wasmi::Error> {
        let len = std::cmp::min(src_len, self.return_length);

        MemoryInstance::transfer(
            src,
            src_ptr as usize,
            &self.memory,
            self.return_offset as usize,
            len as usize,
        )
        .map(|_| self.return_length)
    }
}

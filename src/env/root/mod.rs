mod resolver;

use arrayref::array_ref;

use crate::buffer::Buffer;
use crate::env::child::ChildRuntime;
use crate::execute::Execute;

use log::debug;

use self::resolver::{
    RuntimeModuleImportResolver, ARGUMENT_FUNC_INDEX, BLOCKDATACOPY_FUNC_INDEX,
    BLOCKDATASIZE_FUNC_INDEX, BUFFERCLEAR_FUNC_INDEX, BUFFERGET_FUNC_INDEX, BUFFERMERGE_FUNC_INDEX,
    BUFFERSET_FUNC_INDEX, CALLMODULE_FUNC_INDEX, EXPOSE_FUNC_INDEX, LOADMODULE_FUNC_INDEX,
    LOADPRESTATEROOT_FUNC_INDEX, PRINT_FUNC_INDEX, RETURN_FUNC_INDEX, SAVEPOSTSTATEROOT_FUNC_INDEX,
};

use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use super::{ExtResult, StackFrame};

use wasmi::{
    Externals, FuncInstance, ImportsBuilder, MemoryRef, Module, ModuleInstance, ModuleRef,
    RuntimeArgs, RuntimeValue, Trap,
};

#[derive(Clone)]
pub(crate) struct RootRuntimeWeak<'a>(Weak<Inner<'a>>);

impl<'a> RootRuntimeWeak<'a> {
    pub fn upgrade(&self) -> Option<RootRuntime<'a>> {
        self.0.upgrade().map(RootRuntime)
    }
}

#[derive(Clone)]
pub struct RootRuntime<'a>(Rc<Inner<'a>>);

impl<'a> RootRuntime<'a> {
    pub fn new<'b>(code: &'b [u8], data: &'a [u8], pre_root: [u8; 32]) -> RootRuntime<'a> {
        let module = Module::from_buffer(code).expect("Module loading to succeed");

        let mut imports = ImportsBuilder::new();
        imports.push_resolver("env", &RuntimeModuleImportResolver);

        let instance = ModuleInstance::new(&module, &imports)
            .expect("Module instantation expected to succeed")
            .assert_no_start();

        RootRuntime(Rc::new(Inner {
            instance,
            data,
            pre_root,
            children: Default::default(),
            post_root: Default::default(),
            call_targets: Default::default(),
            call_stack: Default::default(),
            buffer: Default::default(),
            logger: Default::default(),
        }))
    }

    pub fn set_logger<F: Fn(&str) + 'a>(&mut self, f: F) {
        let mut logger = self.0.logger.borrow_mut();
        *logger = Some(Box::new(f));
    }

    pub(crate) fn print(&self, bytes: &[u8]) {
        match self.0.logger.borrow().as_ref() {
            Some(log) => log(&String::from_utf8_lossy(bytes)),
            None => (),
        }
    }

    pub(super) fn call(&self, name: &str, frame: StackFrame) -> i32 {
        if !self.0.call_targets.borrow().contains(name) {
            panic!("function `{}` is not a safe call target", name);
        }

        let export = self
            .0
            .instance
            .export_by_name(name)
            .expect("Exposed name doesn't exist");

        let func = export.as_func().expect("Exposed name isn't a function");

        self.0.call_stack.borrow_mut().push(frame);

        let mut externals = RootExternals(self);

        let result = FuncInstance::invoke(&func, &[], &mut externals)
            .expect("function provided by root runtime failed")
            .expect("function provided by root runtime did not return a value")
            .try_into()
            .expect("funtion provided by rooot runtime return a non-i32 value");

        self.0.call_stack.borrow_mut().pop().unwrap();

        result
    }

    fn memory(&self) -> MemoryRef {
        self.0
            .instance
            .export_by_name("memory")
            .expect("Module expected to have 'memory' export")
            .as_memory()
            .cloned()
            .expect("'memory' export should be a memory")
    }

    pub(crate) fn downgrade(&self) -> RootRuntimeWeak<'a> {
        RootRuntimeWeak(Rc::downgrade(&self.0))
    }

    /// Copies data from the given offset and length into the buffer allocated
    /// by the caller. Returns the total size of the caller's buffer.
    ///
    /// # Signature
    ///
    /// ```text
    /// eth2_return(offset: u32, length: u32) -> u32
    /// ```
    fn ext_return(&self, args: RuntimeArgs) -> ExtResult {
        let memory = self.memory();

        let src_ptr: u32 = args.nth(0);
        let src_len: u32 = args.nth(1);

        let call_stack = self.0.call_stack.borrow();
        let top = call_stack
            .last()
            .expect("eth2_return requires a call stack");

        let len = top.transfer_return(&memory, src_ptr, src_len).unwrap();

        Ok(Some(len.into()))
    }

    /// Copies the argument data from the most recent call into memory at the
    /// given offtet and length. Returns the actual length of the argument data.
    ///
    /// # Signature
    ///
    /// ```text
    /// eth2_argument(dest_offset: u32, dest_length: u32) -> u32
    /// ```
    fn ext_argument(&self, args: RuntimeArgs) -> ExtResult {
        let memory = self.memory();

        let dest_ptr: u32 = args.nth(0);
        let dest_len: u32 = args.nth(1);

        let call_stack = self.0.call_stack.borrow();
        let top = call_stack
            .last()
            .expect("eth2_argument requires a call stack");

        let len = top.transfer_argument(&memory, dest_ptr, dest_len).unwrap();

        Ok(Some(len.into()))
    }

    fn ext_expose(&self, args: RuntimeArgs) -> ExtResult {
        let memory = self.memory();

        let name_ptr: u32 = args.nth(0);
        let name_len: u32 = args.nth(1);
        let name_bytes = memory.get(name_ptr, name_len as usize).unwrap();
        let name = String::from_utf8(name_bytes).unwrap();

        self.0.call_targets.borrow_mut().insert(name);

        Ok(None)
    }

    fn ext_load_pre_state_root(&self, args: RuntimeArgs) -> ExtResult {
        let ptr: u32 = args.nth(0);

        debug!("loadprestateroot to {}", ptr);

        // TODO: add checks for out of bounds access
        let memory = self.memory();
        memory
            .set(ptr, &self.0.pre_root[..])
            .expect("expects writing to memory to succeed");

        Ok(None)
    }

    fn ext_save_post_state_root(&self, args: RuntimeArgs) -> ExtResult {
        let ptr: u32 = args.nth(0);
        debug!("savepoststateroot from {}", ptr);

        // TODO: add checks for out of bounds access
        let mut post_root = self.0.post_root.borrow_mut();
        let memory = self.memory();
        memory
            .get_into(ptr, &mut post_root[..])
            .expect("expects reading from memory to succeed");

        Ok(None)
    }

    fn ext_block_data_size(&self, _: RuntimeArgs) -> ExtResult {
        let ret: i32 = self.0.data.len() as i32;
        debug!("blockdatasize {}", ret);
        Ok(Some(ret.into()))
    }

    fn ext_block_data_copy(&self, args: RuntimeArgs) -> ExtResult {
        let ptr: u32 = args.nth(0);
        let offset: u32 = args.nth(1);
        let length: u32 = args.nth(2);
        debug!(
            "blockdatacopy to {} from {} for {} bytes",
            ptr, offset, length
        );

        // TODO: add overflow check
        let offset = offset as usize;
        let length = length as usize;

        // TODO: add checks for out of bounds access
        let memory = self.memory();
        memory
            .set(ptr, &self.0.data[offset..length])
            .expect("expects writing to memory to succeed");

        Ok(None)
    }

    fn ext_buffer_get(&self, args: RuntimeArgs) -> ExtResult {
        let frame: u32 = args.nth(0);
        let key_ptr: u32 = args.nth(1);
        let value_ptr: u32 = args.nth(2);

        debug!(
            "bufferget for frame {} with key at {}, and returning the value to {}",
            frame, key_ptr, value_ptr
        );

        // TODO: add overflow check
        let frame = frame as u8;

        // TODO: add checks for out of bounds access
        let memory = self.memory();

        let key = memory.get(key_ptr, 32).expect("read to suceed");
        let key = *array_ref![key, 0, 32];

        if let Some(value) = self.0.buffer.borrow().get(frame, key) {
            memory
                .set(value_ptr, value)
                .expect("writing to memory to succeed");

            Ok(Some(0.into()))
        } else {
            Ok(Some(1.into()))
        }
    }

    fn ext_buffer_set(&self, args: RuntimeArgs) -> ExtResult {
        let frame: u32 = args.nth(0);
        let key_ptr: u32 = args.nth(1);
        let value_ptr: u32 = args.nth(2);

        debug!(
            "bufferset for frame {} with key at {} and value at {}",
            frame, key_ptr, value_ptr
        );

        // TODO: add overflow check
        let frame = frame as u8;

        // TODO: add checks for out of bounds access
        let memory = self.memory();

        let key = memory.get(key_ptr, 32).expect("read to suceed");
        let key = *array_ref![key, 0, 32];

        let value = memory.get(value_ptr, 32).expect("read to suceed");
        let value = *array_ref![value, 0, 32];

        self.0.buffer.borrow_mut().insert(frame, key, value);

        Ok(None)
    }

    fn ext_buffer_merge(&self, args: RuntimeArgs) -> ExtResult {
        let frame_a: u32 = args.nth(0);
        let frame_b: u32 = args.nth(1);

        debug!("buffermerge frame {} into frame {}", frame_b, frame_a);

        // TODO: add overflow check
        let frame_a = frame_a as u8;
        let frame_b = frame_b as u8;

        self.0.buffer.borrow_mut().merge(frame_a, frame_b);

        Ok(None)
    }

    fn ext_buffer_clear(&self, args: RuntimeArgs) -> ExtResult {
        let frame: u32 = args.nth(0);

        // TODO: add overflow check
        let frame = frame as u8;

        debug!("bufferclear on frame {}", frame);

        self.0.buffer.borrow_mut().clear(frame);

        Ok(None)
    }

    /// Loads a compiled Wasm module from memory into the slot specified.
    ///
    /// # Signature
    ///
    /// ```text
    /// eth2_loadModule(slot: u32, code_offset: u32, code_length: u32) -> ()
    /// ```
    fn ext_load_module(&self, args: RuntimeArgs) -> ExtResult {
        let slot: u32 = args.nth(0);
        let code_ptr: u32 = args.nth(1);
        let code_len: u32 = args.nth(2);

        debug!(
            "load module 0x{:x} ({} bytes) into {}",
            code_ptr, code_len, slot
        );

        let mut children = self.0.children.borrow_mut();

        let entry = match children.entry(slot) {
            Entry::Occupied(_) => panic!("reusing module slot identifiers not supported"),
            Entry::Vacant(x) => x,
        };

        let memory = self.memory();
        let code = memory.get(code_ptr, code_len as usize).unwrap();

        let child = ChildRuntime::new(self.downgrade(), &code);
        entry.insert(child);

        Ok(None)
    }

    /// Calls the function `name` from the module in `slot`.
    ///
    /// # Signature
    ///
    /// ```text
    /// eth2_callModule(
    ///     slot: u32,
    ///     name_offset: u32,
    ///     name_length: u32
    ///     argument_offset: u32,
    ///     argument_length: u32,
    ///     return_offset: u32,
    ///     return_length: u32,
    /// ) -> u32
    /// ```
    fn ext_call_module(&self, args: RuntimeArgs) -> ExtResult {
        let memory = self.memory();

        let slot: u32 = args.nth(0);

        let name_ptr: u32 = args.nth(1);
        let name_len: u32 = args.nth(2);
        let name_bytes = memory.get(name_ptr, name_len as usize).unwrap();
        let name = String::from_utf8(name_bytes).unwrap();

        let arg_ptr: u32 = args.nth(3);
        let arg_len: u32 = args.nth(4);

        let ret_ptr: u32 = args.nth(5);
        let ret_len: u32 = args.nth(6);

        let frame = StackFrame::builder()
            .argument_offset(arg_ptr)
            .argument_length(arg_len)
            .return_offset(ret_ptr)
            .return_length(ret_len)
            .memory(memory)
            .build();

        // TODO: There's probably a bug here. It might be impossible to load a
        // new module depending on the callstack.

        let children = self.0.children.borrow();
        let retcode = children[&slot].call(&name, frame);

        Ok(Some(retcode.into()))
    }

    fn ext_print(&self, args: RuntimeArgs) -> ExtResult {
        let memory = self.memory();

        let ptr: u32 = args.nth(0);
        let len: u32 = args.nth(1);

        let bytes = memory.get(ptr, len as usize).unwrap();

        self.print(&bytes);

        Ok(None)
    }
}

struct Inner<'a> {
    data: &'a [u8],
    pre_root: [u8; 32],
    post_root: RefCell<[u8; 32]>,
    instance: ModuleRef,
    buffer: RefCell<Buffer>,

    children: RefCell<HashMap<u32, ChildRuntime<'a>>>,

    call_targets: RefCell<HashSet<String>>,
    call_stack: RefCell<Vec<StackFrame>>,

    logger: RefCell<Option<Box<dyn Fn(&str) + 'a>>>,
}

impl<'a> Execute for RootRuntime<'a> {
    fn execute(&mut self) -> [u8; 32] {
        let mut externals = RootExternals(self);

        #[cfg(feature = "extra-pages")]
        externals
            .0
            .memory()
            .grow(wasmi::memory_units::Pages(100))
            .expect("page count to increase by 100");

        self.0
            .instance
            .invoke_export("main", &[], &mut externals)
            .expect("Executed 'main'");

        *self.0.post_root.borrow()
    }
}

struct RootExternals<'a, 'b>(&'a RootRuntime<'b>);

impl<'a, 'b> Externals for RootExternals<'a, 'b> {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            LOADPRESTATEROOT_FUNC_INDEX => self.0.ext_load_pre_state_root(args),
            SAVEPOSTSTATEROOT_FUNC_INDEX => self.0.ext_save_post_state_root(args),
            BLOCKDATASIZE_FUNC_INDEX => self.0.ext_block_data_size(args),
            BLOCKDATACOPY_FUNC_INDEX => self.0.ext_block_data_copy(args),
            BUFFERGET_FUNC_INDEX => self.0.ext_buffer_get(args),
            BUFFERSET_FUNC_INDEX => self.0.ext_buffer_set(args),
            BUFFERMERGE_FUNC_INDEX => self.0.ext_buffer_merge(args),
            BUFFERCLEAR_FUNC_INDEX => self.0.ext_buffer_clear(args),
            LOADMODULE_FUNC_INDEX => self.0.ext_load_module(args),
            CALLMODULE_FUNC_INDEX => self.0.ext_call_module(args),
            EXPOSE_FUNC_INDEX => self.0.ext_expose(args),
            ARGUMENT_FUNC_INDEX => self.0.ext_argument(args),
            RETURN_FUNC_INDEX => self.0.ext_return(args),
            PRINT_FUNC_INDEX => self.0.ext_print(args),
            _ => panic!("unknown function index"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::buffer::Buffer;
    use lazy_static::lazy_static;
    use wabt::wat2wasm;
    use wasmi::memory_units::Pages;
    use wasmi::MemoryInstance;

    lazy_static! {
        static ref NOP: Vec<u8> = wat2wasm(
            r#"
            (module
                (memory (export "memory") 1)
                (func $main (export "main") (nop)))
        "#
        )
        .unwrap();
    }

    fn build_root(n: u8) -> [u8; 32] {
        let mut ret = [0u8; 32];
        ret[0] = n;
        ret
    }

    fn build_runtime<'a>(data: &'a [u8], pre_root: [u8; 32], buffer: Buffer) -> RootRuntime<'a> {
        let mut rt = RootRuntime::new(&NOP, data, pre_root);
        Rc::get_mut(&mut rt.0).unwrap().buffer = buffer.into();
        rt
    }

    #[test]
    fn return_long_value_does_not_overwrite() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();

        let runtime = build_runtime(&[], build_root(42), Buffer::default());
        runtime.memory().set(0, &[45, 99, 7]).unwrap();

        let frame = StackFrame::builder()
            .argument_offset(0u32)
            .argument_length(0u32)
            .return_offset(0u32)
            .return_length(2u32)
            .memory(memory.clone())
            .build();

        runtime.0.call_stack.borrow_mut().push(frame);

        let mut externals = RootExternals(&runtime);
        let result: u32 = Externals::invoke_index(
            &mut externals,
            RETURN_FUNC_INDEX,
            [0.into(), 3.into()][..].into(),
        )
        .expect("trap while calling return")
        .expect("return did not return a result")
        .try_into()
        .expect("return did not return an integer");

        assert_eq!(result, 2);
        assert_eq!(memory.get(0, 3).unwrap(), [45, 99, 0]);
    }

    #[test]
    fn return_copies_value_into_parent_frame() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();

        let runtime = build_runtime(&[], build_root(42), Buffer::default());
        runtime.memory().set(0, &[45]).unwrap();

        let frame = StackFrame::builder()
            .argument_offset(0u32)
            .argument_length(0u32)
            .return_offset(0u32)
            .return_length(2u32)
            .memory(memory.clone())
            .build();

        runtime.0.call_stack.borrow_mut().push(frame);

        let mut externals = RootExternals(&runtime);
        let result: u32 = Externals::invoke_index(
            &mut externals,
            RETURN_FUNC_INDEX,
            [0.into(), 1.into()][..].into(),
        )
        .expect("trap while calling return")
        .expect("return did not return a result")
        .try_into()
        .expect("return did not return an integer");

        assert_eq!(result, 2);
        assert_eq!(memory.get(0, 1).unwrap(), [45]);
    }

    #[test]
    fn return_provides_buffer_size() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        let runtime = build_runtime(&[], build_root(42), Buffer::default());

        let frame = StackFrame::builder()
            .argument_offset(0u32)
            .argument_length(0u32)
            .return_offset(0u32)
            .return_length(2u32)
            .memory(memory.clone())
            .build();

        runtime.0.call_stack.borrow_mut().push(frame);

        let mut externals = RootExternals(&runtime);
        let result: u32 = Externals::invoke_index(
            &mut externals,
            RETURN_FUNC_INDEX,
            [0.into(), 0.into()][..].into(),
        )
        .expect("trap while calling return")
        .expect("return did not return a result")
        .try_into()
        .expect("return did not return an integer");

        assert_eq!(result, 2);
    }

    #[test]
    fn argument_provides_buffer_size() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        let runtime = build_runtime(&[], build_root(42), Buffer::default());

        let frame = StackFrame::builder()
            .return_offset(0u32)
            .return_length(0u32)
            .argument_offset(0u32)
            .argument_length(2u32)
            .memory(memory.clone())
            .build();

        runtime.0.call_stack.borrow_mut().push(frame);

        let mut externals = RootExternals(&runtime);
        let result: u32 = Externals::invoke_index(
            &mut externals,
            ARGUMENT_FUNC_INDEX,
            [0.into(), 0.into()][..].into(),
        )
        .expect("trap while calling argument")
        .expect("argument did not return a result")
        .try_into()
        .expect("argument did not return an integer");

        assert_eq!(result, 2);
    }

    #[test]
    fn argument_copies_value_from_parent_frame() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        memory.set(0, &[32, 123]).unwrap();

        let runtime = build_runtime(&[], build_root(42), Buffer::default());
        runtime.memory().set(0, &[32, 45]).unwrap();

        let frame = StackFrame::builder()
            .argument_offset(0u32)
            .argument_length(2u32)
            .return_offset(0u32)
            .return_length(0u32)
            .memory(memory.clone())
            .build();

        runtime.0.call_stack.borrow_mut().push(frame);

        let mut externals = RootExternals(&runtime);
        let result: u32 = Externals::invoke_index(
            &mut externals,
            ARGUMENT_FUNC_INDEX,
            [0.into(), 1.into()][..].into(),
        )
        .expect("trap while calling return")
        .expect("return did not return a result")
        .try_into()
        .expect("return did not return an integer");

        assert_eq!(result, 2);
        assert_eq!(runtime.memory().get(0, 2).unwrap(), [32, 45]);
    }

    #[test]
    fn argument_long_value_does_not_leak() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        memory.set(0, &[32, 123, 234]).unwrap();

        let runtime = build_runtime(&[], build_root(42), Buffer::default());
        runtime.memory().set(0, &[45, 45, 45]).unwrap();

        let frame = StackFrame::builder()
            .argument_offset(0u32)
            .argument_length(2u32)
            .return_offset(0u32)
            .return_length(0u32)
            .memory(memory.clone())
            .build();

        runtime.0.call_stack.borrow_mut().push(frame);

        let mut externals = RootExternals(&runtime);
        let result: u32 = Externals::invoke_index(
            &mut externals,
            ARGUMENT_FUNC_INDEX,
            [0.into(), 3.into()][..].into(),
        )
        .expect("trap while calling return")
        .expect("return did not return a result")
        .try_into()
        .expect("return did not return an integer");

        assert_eq!(result, 2);
        assert_eq!(runtime.memory().get(0, 3).unwrap(), [32, 123, 45]);
    }

    #[test]
    fn load_pre_state_root() {
        let runtime = build_runtime(&[], build_root(42), Buffer::default());

        let mut externals = RootExternals(&runtime);
        Externals::invoke_index(
            &mut externals,
            LOADPRESTATEROOT_FUNC_INDEX,
            [0.into()][..].into(),
        )
        .unwrap();

        assert_eq!(runtime.memory().get(0, 32).unwrap(), build_root(42));
    }

    #[test]
    fn save_post_state_root() {
        let runtime = build_runtime(&[], build_root(0), Buffer::default());

        let memory = runtime.memory();
        memory.set(100, &build_root(42)).expect("sets memory");

        let mut externals = RootExternals(&runtime);
        Externals::invoke_index(
            &mut externals,
            SAVEPOSTSTATEROOT_FUNC_INDEX,
            [100.into()][..].into(),
        )
        .unwrap();

        assert_eq!(runtime.memory().get(100, 32).unwrap(), build_root(42));
    }

    #[test]
    fn block_data_size() {
        let runtime = build_runtime(&[1; 42], build_root(0), Buffer::default());

        let mut externals = RootExternals(&runtime);
        assert_eq!(
            Externals::invoke_index(&mut externals, BLOCKDATASIZE_FUNC_INDEX, [][..].into())
                .unwrap()
                .unwrap(),
            42.into()
        );
    }

    #[test]
    fn block_data_copy() {
        let data: Vec<u8> = (1..21).collect();
        let runtime = build_runtime(&data, build_root(0), Buffer::default());

        let mut externals = RootExternals(&runtime);
        Externals::invoke_index(
            &mut externals,
            BLOCKDATACOPY_FUNC_INDEX,
            [1.into(), 0.into(), 20.into()][..].into(),
        )
        .unwrap();

        let mut externals = RootExternals(&runtime);
        Externals::invoke_index(
            &mut externals,
            BLOCKDATACOPY_FUNC_INDEX,
            [23.into(), 10.into(), 20.into()][..].into(),
        )
        .unwrap();

        // This checks that the entire data blob was loaded into memory.
        assert_eq!(runtime.clone().memory().get(1, 20).unwrap(), data);

        // This checks that the data after the offset was loaded into memory.
        assert_eq!(runtime.memory().get(23, 10).unwrap()[..], data[10..]);
    }

    #[test]
    fn buffer_get() {
        let mut buffer = Buffer::default();

        // Insert a value into the buffer that corresponds to the above key.
        buffer.insert(0, [1u8; 32], build_root(42));

        let runtime = build_runtime(&[], build_root(0), buffer);

        let memory = runtime.memory();

        // Save the 32 byte key at position 0 in memory
        memory.set(0, &[1u8; 32]).unwrap();

        let mut externals = RootExternals(&runtime);
        Externals::invoke_index(
            &mut externals,
            BUFFERGET_FUNC_INDEX,
            [0.into(), 0.into(), 32.into()][..].into(),
        )
        .unwrap();

        assert_eq!(
            runtime.clone().memory().get(32, 32).unwrap(),
            build_root(42)
        );
    }

    #[test]
    fn buffer_set() {
        let runtime = build_runtime(&[], build_root(0), Buffer::default());

        let memory = runtime.memory();
        memory.set(0, &[1u8; 32]).unwrap();
        memory.set(32, &[2u8; 32]).unwrap();

        let mut externals = RootExternals(&runtime);
        Externals::invoke_index(
            &mut externals,
            BUFFERSET_FUNC_INDEX,
            [0.into(), 0.into(), 32.into()][..].into(),
        )
        .unwrap();

        let buffer = runtime.0.buffer.borrow();
        assert_eq!(buffer.get(0, [1u8; 32]), Some(&[2u8; 32]));
    }

    #[test]
    fn buffer_merge() {
        let mut buffer = Buffer::default();

        buffer.insert(1, [0u8; 32], [0u8; 32]);
        buffer.insert(1, [1u8; 32], [1u8; 32]);
        buffer.insert(2, [2u8; 32], [2u8; 32]);
        buffer.insert(2, [0u8; 32], [3u8; 32]);

        let runtime = build_runtime(&[], build_root(0), buffer);

        let mut externals = RootExternals(&runtime);
        Externals::invoke_index(
            &mut externals,
            BUFFERMERGE_FUNC_INDEX,
            [1.into(), 2.into()][..].into(),
        )
        .unwrap();

        let buffer = runtime.0.buffer.borrow();
        assert_eq!(buffer.get(1, [0u8; 32]), Some(&[3u8; 32]));
        assert_eq!(buffer.get(1, [1u8; 32]), Some(&[1u8; 32]));
        assert_eq!(buffer.get(1, [2u8; 32]), Some(&[2u8; 32]));
        assert_eq!(buffer.get(2, [0u8; 32]), Some(&[3u8; 32]));
        assert_eq!(buffer.get(2, [2u8; 32]), Some(&[2u8; 32]));
    }

    #[test]
    fn buffer_clear() {
        let mut buffer = Buffer::default();

        buffer.insert(1, [0u8; 32], [0u8; 32]);
        buffer.insert(2, [0u8; 32], [0u8; 32]);

        let runtime = build_runtime(&[], build_root(0), buffer);

        let mut externals = RootExternals(&runtime);
        Externals::invoke_index(
            &mut externals,
            BUFFERCLEAR_FUNC_INDEX,
            [2.into()][..].into(),
        )
        .unwrap();

        let buffer = runtime.0.buffer.borrow();
        assert_eq!(buffer.get(1, [0u8; 32]), Some(&[0u8; 32]));
        assert_eq!(buffer.get(2, [0u8; 32]), None);
    }
}

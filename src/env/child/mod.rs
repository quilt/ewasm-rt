mod resolver;

use crate::env::root::{RootRuntime, RootRuntimeWeak};

use self::resolver::{externals, ChildModuleImportResolver};

use std::cell::RefCell;

use super::{ExtResult, StackFrame};

use wasmi::{
    Externals, FuncInstance, ImportsBuilder, MemoryRef, Module, ModuleInstance, ModuleRef,
    RuntimeArgs, RuntimeValue, Trap,
};

#[derive(Debug)]
pub struct ChildRuntime<'a> {
    instance: ModuleRef,
    root: RootRuntimeWeak<'a>,

    call_stack: RefCell<Vec<StackFrame>>,
}

impl<'a> ChildRuntime<'a> {
    pub(crate) fn new(root: RootRuntimeWeak<'a>, code: &[u8]) -> Self {
        let module = Module::from_buffer(code).expect("Module loading to succeed");

        let mut imports = ImportsBuilder::new();
        imports.push_resolver("env", &ChildModuleImportResolver);

        let instance = ModuleInstance::new(&module, &imports)
            .expect("Module instantation expected to succeed")
            .assert_no_start();

        Self {
            instance,
            root,
            call_stack: Default::default(),
        }
    }

    pub(super) fn call(&self, name: &str, frame: StackFrame) -> i32 {
        let export = self
            .instance
            .export_by_name(name)
            .expect("name doesn't exist in child");

        let func = export.as_func().expect("name isn't a function");

        self.call_stack.borrow_mut().push(frame);

        let mut externals = ChildExternals(self);
        let result = FuncInstance::invoke(&func, &[], &mut externals)
            .expect("function provided by child runtime failed")
            .expect("function provided by child runtime did not return a value")
            .try_into()
            .expect("funtion provided by child runtime return a non-i32 value");

        self.call_stack.borrow_mut().pop().unwrap();

        result
    }

    fn memory(&self) -> MemoryRef {
        self.instance
            .export_by_name("memory")
            .expect("Module expected to have 'memory' export")
            .as_memory()
            .cloned()
            .expect("'memory' export should be a memory")
    }

    fn root(&self) -> RootRuntime<'a> {
        self.root
            .upgrade()
            .expect("root runtime dropped before child")
    }

    fn ext_call(&self, args: RuntimeArgs) -> ExtResult {
        let memory = self.memory();

        let name_ptr: u32 = args.nth(0);
        let name_len: u32 = args.nth(1);
        let name_bytes = memory.get(name_ptr, name_len as usize).unwrap();
        let name = String::from_utf8(name_bytes).unwrap();

        let arg_ptr: u32 = args.nth(2);
        let arg_len: u32 = args.nth(3);

        let ret_ptr: u32 = args.nth(4);
        let ret_len: u32 = args.nth(5);

        let frame = StackFrame::builder()
            .argument_offset(arg_ptr)
            .argument_length(arg_len)
            .return_offset(ret_ptr)
            .return_length(ret_len)
            .memory(memory)
            .build();

        let retcode = self.root().call(&name, frame);

        Ok(Some(retcode.into()))
    }
}

#[derive(Debug)]
struct ChildExternals<'a, 'b>(&'a ChildRuntime<'b>);

impl<'a, 'b> Externals for ChildExternals<'a, 'b> {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            externals::CALL => self.0.ext_call(args),
            _ => panic!("unknown function index"),
        }
    }
}

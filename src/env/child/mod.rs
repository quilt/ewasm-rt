mod resolver;

use crate::env::root::{RootRuntime, RootRuntimeWeak, StackFrame};

use self::resolver::{externals, ChildModuleImportResolver};

use super::ExtResult;

use wasmi::{
    Externals, ImportsBuilder, MemoryRef, Module, ModuleInstance, ModuleRef, RuntimeArgs,
    RuntimeValue, Trap,
};

#[derive(Debug)]
pub struct ChildRuntime<'a> {
    instance: ModuleRef,
    root: RootRuntimeWeak<'a>,
}

impl<'a> ChildRuntime<'a> {
    pub(crate) fn new(root: RootRuntimeWeak<'a>, code: &[u8]) -> Self {
        let module = Module::from_buffer(code).expect("Module loading to succeed");

        let mut imports = ImportsBuilder::new();
        imports.push_resolver("env", &ChildModuleImportResolver);

        let instance = ModuleInstance::new(&module, &imports)
            .expect("Module instantation expected to succeed")
            .assert_no_start();

        Self { instance, root }
    }

    pub fn execute(&mut self) {
        self.instance
            .clone()
            .invoke_export("main", &[], self)
            .expect("Executed 'main'");
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

impl<'a> Externals for ChildRuntime<'a> {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            externals::CALL => self.ext_call(args),
            _ => panic!("unknown function index"),
        }
    }
}

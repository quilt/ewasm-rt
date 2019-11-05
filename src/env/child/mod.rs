mod resolver;

use crate::execute::Execute;

use self::resolver::ChildModuleImportResolver;

use wasmi::{
    Externals, ImportsBuilder, MemoryRef, Module, ModuleInstance, RuntimeArgs, RuntimeValue, Trap,
};

#[derive(Debug)]
pub struct ChildRuntime<'a> {
    code: &'a [u8],
}

impl<'a> ChildRuntime<'a> {
    pub fn new(code: &'a [u8]) -> Self {
        Self { code }
    }

    pub fn execute(&mut self) {
        let module = Module::from_buffer(self.code).expect("Module loading to succeed");
        let mut imports = ImportsBuilder::new();
        imports.push_resolver("env", &ChildModuleImportResolver);

        let instance = ModuleInstance::new(&module, &imports)
            .expect("Module instantation expected to succeed")
            .assert_no_start();

        instance
            .invoke_export("main", &[], self)
            .expect("Executed 'main'");
    }
}

impl<'a> Externals for ChildRuntime<'a> {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            _ => panic!("unknown function index"),
        }
    }
}

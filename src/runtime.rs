use crate::execute::Execute;
use crate::externals;
use crate::resolver::RuntimeModuleImportResolver;
use wasmi::{
    Error as InterpreterError, FuncInstance, FuncRef, ImportsBuilder, MemoryRef, Module,
    ModuleImportResolver, ModuleInstance, Signature, ValueType,
};

pub struct Runtime<'a> {
    pub code: &'a [u8],
    pub data: &'a [u8],

    pub pre_root: [u8; 32],
    pub post_root: [u8; 32],
    pub beacon_pre_state: [u8; 32],

    // ???
    pub ticks_left: u32,
    pub memory: Option<MemoryRef>,
}

impl<'a> Execute<'a> for Runtime<'a> {
    fn execute(&'a mut self) {
        let module = Module::from_buffer(self.code).expect("Module loading to succeed");
        let mut imports = ImportsBuilder::new();
        imports.push_resolver("env", &RuntimeModuleImportResolver);

        let instance = ModuleInstance::new(&module, &imports)
            .expect("Module instantation expected to succeed")
            .assert_no_start();

        let result = instance
            .invoke_export("main", &[], self)
            .expect("Executed 'main'");
    }
}

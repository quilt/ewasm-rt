pub mod externals {
    pub const CALL: usize = 1;
}

use wasmi::{
    Error as InterpreterError, FuncInstance, FuncRef, ModuleImportResolver, Signature, ValueType,
};

pub struct ChildModuleImportResolver;

impl<'a> ModuleImportResolver for ChildModuleImportResolver {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, InterpreterError> {
        let func_ref = match field_name {
            "eth2_call" => FuncInstance::alloc_host(
                // eth2_call(name, name_len, arg, arg_len, ret, ret_len)
                Signature::new(&[ValueType::I32; 6][..], Some(ValueType::I32)),
                externals::CALL,
            ),
            _ => {
                return Err(InterpreterError::Function(format!(
                    "host module doesn't export function with name {}",
                    field_name
                )))
            }
        };
        Ok(func_ref)
    }
}

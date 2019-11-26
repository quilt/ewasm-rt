pub mod externals {
    pub const CALL: usize = 1;
    pub const ARGUMENT: usize = 2;
    pub const RETURN: usize = 3;

    #[cfg(feature = "debug")]
    pub const PRINT: usize = 99;
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
            "eth2_return" => FuncInstance::alloc_host(
                // eth2_return(offset: u32, length: u32) -> u32
                Signature::new(&[ValueType::I32; 2][..], Some(ValueType::I32)),
                externals::RETURN,
            ),
            "eth2_argument" => FuncInstance::alloc_host(
                // eth2_argument(offset: u32, length: u32) -> u32
                Signature::new(&[ValueType::I32; 2][..], Some(ValueType::I32)),
                externals::ARGUMENT,
            ),
            "eth2_call" => FuncInstance::alloc_host(
                // eth2_call(name, name_len, arg, arg_len, ret, ret_len)
                Signature::new(&[ValueType::I32; 6][..], Some(ValueType::I32)),
                externals::CALL,
            ),
            #[cfg(feature = "debug")]
            "print" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32; 2][..], None),
                externals::PRINT,
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

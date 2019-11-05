use wasmi::{
    Error as InterpreterError, FuncInstance, FuncRef, ModuleImportResolver, Signature, ValueType,
};

//pub const EXEC_FUNC_INDEX: usize = 8;

pub struct ChildModuleImportResolver;

impl<'a> ModuleImportResolver for ChildModuleImportResolver {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, InterpreterError> {
        let func_ref = match field_name {
            /*
            "eth2_loadPreStateRoot" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                LOADPRESTATEROOT_FUNC_INDEX,
            ),
            */
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

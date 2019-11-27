use wasmi::{
    Error as InterpreterError, FuncInstance, FuncRef, ModuleImportResolver, Signature, ValueType,
};

pub const LOADPRESTATEROOT_FUNC_INDEX: usize = 0;
pub const BLOCKDATASIZE_FUNC_INDEX: usize = 1;
pub const BLOCKDATACOPY_FUNC_INDEX: usize = 2;
pub const SAVEPOSTSTATEROOT_FUNC_INDEX: usize = 3;
pub const BUFFERGET_FUNC_INDEX: usize = 4;
pub const BUFFERSET_FUNC_INDEX: usize = 5;
pub const BUFFERMERGE_FUNC_INDEX: usize = 6;
pub const BUFFERCLEAR_FUNC_INDEX: usize = 7;
pub const LOADMODULE_FUNC_INDEX: usize = 8;
pub const EXPOSE_FUNC_INDEX: usize = 9;
pub const ARGUMENT_FUNC_INDEX: usize = 10;
pub const RETURN_FUNC_INDEX: usize = 11;
pub const CALLMODULE_FUNC_INDEX: usize = 12;
pub const PRINT_FUNC_INDEX: usize = 99;

pub struct RuntimeModuleImportResolver;

impl<'a> ModuleImportResolver for RuntimeModuleImportResolver {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, InterpreterError> {
        let func_ref = match field_name {
            "eth2_loadPreStateRoot" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                LOADPRESTATEROOT_FUNC_INDEX,
            ),
            "eth2_savePostStateRoot" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                SAVEPOSTSTATEROOT_FUNC_INDEX,
            ),
            "eth2_blockDataSize" => FuncInstance::alloc_host(
                Signature::new(&[][..], Some(ValueType::I32)),
                BLOCKDATASIZE_FUNC_INDEX,
            ),
            "eth2_blockDataCopy" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                BLOCKDATACOPY_FUNC_INDEX,
            ),
            "eth2_bufferGet" => FuncInstance::alloc_host(
                Signature::new(
                    &[ValueType::I32, ValueType::I32, ValueType::I32][..],
                    Some(ValueType::I32),
                ),
                BUFFERGET_FUNC_INDEX,
            ),
            "eth2_bufferSet" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                BUFFERSET_FUNC_INDEX,
            ),
            "eth2_bufferMerge" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                BUFFERMERGE_FUNC_INDEX,
            ),
            "eth2_bufferClear" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                BUFFERCLEAR_FUNC_INDEX,
            ),
            "eth2_loadModule" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32; 3][..], None),
                LOADMODULE_FUNC_INDEX,
            ),
            "eth2_callModule" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32; 7][..], Some(ValueType::I32)),
                CALLMODULE_FUNC_INDEX,
            ),
            "eth2_expose" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                EXPOSE_FUNC_INDEX,
            ),
            "eth2_argument" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], Some(ValueType::I32)),
                ARGUMENT_FUNC_INDEX,
            ),
            "eth2_return" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], Some(ValueType::I32)),
                RETURN_FUNC_INDEX,
            ),
            "print" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32; 2][..], None),
                PRINT_FUNC_INDEX,
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

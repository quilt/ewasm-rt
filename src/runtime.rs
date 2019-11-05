use arrayref::array_ref;

use crate::buffer::Buffer;
use crate::execute::Execute;
use crate::resolver::{
    RuntimeModuleImportResolver, BLOCKDATACOPY_FUNC_INDEX, BLOCKDATASIZE_FUNC_INDEX,
    BUFFERCLEAR_FUNC_INDEX, BUFFERGET_FUNC_INDEX, BUFFERMERGE_FUNC_INDEX, BUFFERSET_FUNC_INDEX,
    EXEC_FUNC_INDEX, LOADPRESTATEROOT_FUNC_INDEX, SAVEPOSTSTATEROOT_FUNC_INDEX,
};

use log::debug;

use wasmi::{
    Externals, ImportsBuilder, MemoryRef, Module, ModuleInstance, RuntimeArgs, RuntimeValue, Trap,
};

pub type ExtResult = Result<Option<RuntimeValue>, Trap>;

#[derive(Clone)]
pub struct Runtime<'a> {
    pub(crate) code: &'a [u8],
    pub(crate) data: &'a [u8],
    pub(crate) pre_root: [u8; 32],
    pub(crate) post_root: [u8; 32],
    pub(crate) memory: Option<MemoryRef>,
    pub(crate) buffer: Buffer,
}

impl<'a> Runtime<'a> {
    pub fn new(code: &'a [u8], data: &'a [u8], pre_root: [u8; 32]) -> Runtime<'a> {
        Runtime {
            code,
            data,
            pre_root,
            post_root: [0u8; 32],
            memory: None,
            buffer: Buffer::default(),
        }
    }

    fn ext_load_pre_state_root(&mut self, args: RuntimeArgs) -> ExtResult {
        let ptr: u32 = args.nth(0);

        debug!("loadprestateroot to {}", ptr);

        // TODO: add checks for out of bounds access
        let memory = self.memory.as_ref().expect("expects memory object");
        memory
            .set(ptr, &self.pre_root[..])
            .expect("expects writing to memory to succeed");

        Ok(None)
    }

    fn ext_save_post_state_root(&mut self, args: RuntimeArgs) -> ExtResult {
        let ptr: u32 = args.nth(0);
        debug!("savepoststateroot from {}", ptr);

        // TODO: add checks for out of bounds access
        let memory = self.memory.as_ref().expect("expects memory object");
        memory
            .get_into(ptr, &mut self.post_root[..])
            .expect("expects reading from memory to succeed");

        Ok(None)
    }

    fn ext_block_data_size(&mut self, _: RuntimeArgs) -> ExtResult {
        let ret: i32 = self.data.len() as i32;
        debug!("blockdatasize {}", ret);
        Ok(Some(ret.into()))
    }

    fn ext_block_data_copy(&mut self, args: RuntimeArgs) -> ExtResult {
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
        let memory = self.memory.as_ref().expect("expects memory object");
        memory
            .set(ptr, &self.data[offset..length])
            .expect("expects writing to memory to succeed");

        Ok(None)
    }

    fn ext_buffer_get(&mut self, args: RuntimeArgs) -> ExtResult {
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
        let memory = self.memory.as_ref().expect("expects memory object");

        let key = memory.get(key_ptr, 32).expect("read to suceed");
        let key = *array_ref![key, 0, 32];

        if let Some(value) = self.buffer.get(frame, key) {
            memory
                .set(value_ptr, value)
                .expect("writing to memory to succeed");

            Ok(Some(0.into()))
        } else {
            Ok(Some(1.into()))
        }
    }

    fn ext_buffer_set(&mut self, args: RuntimeArgs) -> ExtResult {
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
        let memory = self.memory.as_ref().expect("expects memory object");

        let key = memory.get(key_ptr, 32).expect("read to suceed");
        let key = *array_ref![key, 0, 32];

        let value = memory.get(value_ptr, 32).expect("read to suceed");
        let value = *array_ref![value, 0, 32];

        self.buffer.insert(frame, key, value);

        Ok(None)
    }

    fn ext_buffer_merge(&mut self, args: RuntimeArgs) -> ExtResult {
        let frame_a: u32 = args.nth(0);
        let frame_b: u32 = args.nth(1);

        debug!("buffermerge frame {} into frame {}", frame_b, frame_a);

        // TODO: add overflow check
        let frame_a = frame_a as u8;
        let frame_b = frame_b as u8;

        self.buffer.merge(frame_a, frame_b);

        Ok(None)
    }

    fn ext_buffer_clear(&mut self, args: RuntimeArgs) -> ExtResult {
        let frame: u32 = args.nth(0);

        // TODO: add overflow check
        let frame = frame as u8;

        debug!("bufferclear on frame {}", frame);

        self.buffer.clear(frame);

        Ok(None)
    }

    fn ext_exec(&mut self, args: RuntimeArgs) -> ExtResult {
        let code_ptr: u32 = args.nth(0);
        let code_len: u32 = args.nth(1);

        debug!("exec 0x{:x} ({} bytes)", code_ptr, code_len);

        unimplemented!()
    }
}

impl<'a> Execute<'a> for Runtime<'a> {
    fn execute(&'a mut self) -> [u8; 32] {
        let module = Module::from_buffer(self.code).expect("Module loading to succeed");
        let mut imports = ImportsBuilder::new();
        imports.push_resolver("env", &RuntimeModuleImportResolver);

        let instance = ModuleInstance::new(&module, &imports)
            .expect("Module instantation expected to succeed")
            .assert_no_start();

        self.memory = Some(
            instance
                .export_by_name("memory")
                .expect("Module expected to have 'memory' export")
                .as_memory()
                .cloned()
                .expect("'memory' export should be a memory"),
        );

        instance
            .invoke_export("main", &[], self)
            .expect("Executed 'main'");

        self.post_root
    }
}

impl<'a> Externals for Runtime<'a> {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            LOADPRESTATEROOT_FUNC_INDEX => self.ext_load_pre_state_root(args),
            SAVEPOSTSTATEROOT_FUNC_INDEX => self.ext_save_post_state_root(args),
            BLOCKDATASIZE_FUNC_INDEX => self.ext_block_data_size(args),
            BLOCKDATACOPY_FUNC_INDEX => self.ext_block_data_copy(args),
            BUFFERGET_FUNC_INDEX => self.ext_buffer_get(args),
            BUFFERSET_FUNC_INDEX => self.ext_buffer_set(args),
            BUFFERMERGE_FUNC_INDEX => self.ext_buffer_merge(args),
            BUFFERCLEAR_FUNC_INDEX => self.ext_buffer_clear(args),
            EXEC_FUNC_INDEX => self.ext_exec(args),
            _ => panic!("unknown function index"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::buffer::Buffer;
    use wasmi::memory_units::Pages;
    use wasmi::{MemoryInstance, MemoryRef};

    fn build_root(n: u8) -> [u8; 32] {
        let mut ret = [0u8; 32];
        ret[0] = n;
        ret
    }

    fn build_runtime<'a>(
        data: &'a [u8],
        pre_root: [u8; 32],
        memory: MemoryRef,
        buffer: Buffer,
    ) -> Runtime<'a> {
        Runtime {
            code: &[],
            data: data,
            pre_root,
            post_root: [0; 32],
            memory: Some(memory),
            buffer,
        }
    }

    #[test]
    fn load_pre_state_root() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        let mut runtime = build_runtime(&[], build_root(42), memory, Buffer::default());

        assert!(Externals::invoke_index(
            &mut runtime,
            LOADPRESTATEROOT_FUNC_INDEX,
            [0.into()][..].into()
        )
        .is_ok());

        assert_eq!(runtime.memory.unwrap().get(0, 32).unwrap(), build_root(42));
    }

    #[test]
    fn save_post_state_root() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        memory.set(100, &build_root(42)).expect("sets memory");

        let mut runtime = build_runtime(&[], build_root(0), memory, Buffer::default());

        assert!(Externals::invoke_index(
            &mut runtime,
            SAVEPOSTSTATEROOT_FUNC_INDEX,
            [100.into()][..].into()
        )
        .is_ok());

        assert_eq!(
            runtime.memory.unwrap().get(100, 32).unwrap(),
            build_root(42)
        );
    }

    #[test]
    fn block_data_size() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        let mut runtime = build_runtime(&[1; 42], build_root(0), memory, Buffer::default());

        assert_eq!(
            Externals::invoke_index(&mut runtime, BLOCKDATASIZE_FUNC_INDEX, [][..].into())
                .unwrap()
                .unwrap(),
            42.into()
        );
    }

    #[test]
    fn block_data_copy() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        let data: Vec<u8> = (1..21).collect();
        let mut runtime = build_runtime(&data, build_root(0), memory, Buffer::default());

        assert!(Externals::invoke_index(
            &mut runtime,
            BLOCKDATACOPY_FUNC_INDEX,
            [1.into(), 0.into(), 20.into()][..].into()
        )
        .is_ok());

        assert!(Externals::invoke_index(
            &mut runtime,
            BLOCKDATACOPY_FUNC_INDEX,
            [23.into(), 10.into(), 20.into()][..].into()
        )
        .is_ok());

        // This checks that the entire data blob was loaded into memory.
        assert_eq!(runtime.clone().memory.unwrap().get(1, 20).unwrap(), data);

        // This checks that the data after the offset was loaded into memory.
        assert_eq!(runtime.memory.unwrap().get(23, 10).unwrap()[..], data[10..]);
    }

    #[test]
    fn buffer_get() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        let mut buffer = Buffer::default();

        // Save the 32 byte key at position 0 in memory
        memory.set(0, &[1u8; 32]).unwrap();

        // Insert a value into the buffer that corresponds to the above key.
        buffer.insert(0, [1u8; 32], build_root(42));

        let mut runtime = build_runtime(&[], build_root(0), memory, buffer);

        assert!(Externals::invoke_index(
            &mut runtime,
            BUFFERGET_FUNC_INDEX,
            [0.into(), 0.into(), 32.into()][..].into()
        )
        .is_ok());

        assert_eq!(
            runtime.clone().memory.unwrap().get(32, 32).unwrap(),
            build_root(42)
        );
    }

    #[test]
    fn buffer_set() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        memory.set(0, &[1u8; 32]).unwrap();
        memory.set(32, &[2u8; 32]).unwrap();

        let mut runtime = build_runtime(&[], build_root(0), memory, Buffer::default());

        assert!(Externals::invoke_index(
            &mut runtime,
            BUFFERSET_FUNC_INDEX,
            [0.into(), 0.into(), 32.into()][..].into()
        )
        .is_ok());

        assert_eq!(runtime.buffer.get(0, [1u8; 32]), Some(&[2u8; 32]));
    }

    #[test]
    fn buffer_merge() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        let mut buffer = Buffer::default();

        buffer.insert(1, [0u8; 32], [0u8; 32]);
        buffer.insert(1, [1u8; 32], [1u8; 32]);
        buffer.insert(2, [2u8; 32], [2u8; 32]);
        buffer.insert(2, [0u8; 32], [3u8; 32]);

        let mut runtime = build_runtime(&[], build_root(0), memory, buffer);

        assert!(Externals::invoke_index(
            &mut runtime,
            BUFFERMERGE_FUNC_INDEX,
            [1.into(), 2.into()][..].into()
        )
        .is_ok());

        assert_eq!(runtime.buffer.get(1, [0u8; 32]), Some(&[3u8; 32]));
        assert_eq!(runtime.buffer.get(1, [1u8; 32]), Some(&[1u8; 32]));
        assert_eq!(runtime.buffer.get(1, [2u8; 32]), Some(&[2u8; 32]));
        assert_eq!(runtime.buffer.get(2, [0u8; 32]), Some(&[3u8; 32]));
        assert_eq!(runtime.buffer.get(2, [2u8; 32]), Some(&[2u8; 32]));
    }

    #[test]
    fn buffer_clear() {
        let memory = MemoryInstance::alloc(Pages(1), None).unwrap();
        let mut buffer = Buffer::default();

        buffer.insert(1, [0u8; 32], [0u8; 32]);
        buffer.insert(2, [0u8; 32], [0u8; 32]);

        let mut runtime = build_runtime(&[], build_root(0), memory, buffer);

        assert!(Externals::invoke_index(
            &mut runtime,
            BUFFERCLEAR_FUNC_INDEX,
            [2.into()][..].into()
        )
        .is_ok());

        assert_eq!(runtime.buffer.get(1, [0u8; 32]), Some(&[0u8; 32]));
        assert_eq!(runtime.buffer.get(2, [0u8; 32]), None);
    }
}

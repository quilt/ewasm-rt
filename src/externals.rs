use crate::resolver::{
    BLOCKDATACOPY_FUNC_INDEX, BLOCKDATASIZE_FUNC_INDEX, LOADPRESTATEROOT_FUNC_INDEX,
    PUSHNEWDEPOSIT_FUNC_INDEX, SAVEPOSTSTATEROOT_FUNC_INDEX, USETICKS_FUNC_INDEX,
};
use crate::runtime::Runtime;
use wasmi::{Externals, RuntimeArgs, RuntimeValue, Trap, TrapKind};

impl<'a> Externals for Runtime<'a> {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            USETICKS_FUNC_INDEX => {
                let ticks: u32 = args.nth(0);
                if self.ticks_left < ticks {
                    // FIXME: use TrapKind::Host
                    return Err(Trap::new(TrapKind::Unreachable));
                }
                self.ticks_left -= ticks;
                Ok(None)
            }
            LOADPRESTATEROOT_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                println!("loadprestateroot to {}", ptr);

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory object");
                memory
                    .set(ptr, &self.pre_root[..])
                    .expect("expects writing to memory to succeed");

                Ok(None)
            }
            SAVEPOSTSTATEROOT_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                println!("savepoststateroot from {}", ptr);

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory object");
                memory
                    .get_into(ptr, &mut self.post_root[..])
                    .expect("expects reading from memory to succeed");

                Ok(None)
            }
            BLOCKDATASIZE_FUNC_INDEX => {
                let ret: i32 = self.data.len() as i32;
                println!("blockdatasize {}", ret);
                Ok(Some(ret.into()))
            }
            BLOCKDATACOPY_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                let offset: u32 = args.nth(1);
                let length: u32 = args.nth(2);
                println!(
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
            PUSHNEWDEPOSIT_FUNC_INDEX => unimplemented!(),
            _ => panic!("unknown function index"),
        }
    }
}

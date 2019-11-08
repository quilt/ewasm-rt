pub trait Execute {
    fn execute(&mut self) -> [u8; 32];
}

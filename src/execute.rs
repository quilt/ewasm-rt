pub trait Execute<'a> {
    fn execute(&'a mut self);
}

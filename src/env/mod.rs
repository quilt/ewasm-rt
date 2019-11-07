pub mod child;
pub mod root;

use wasmi::{RuntimeValue, Trap};

pub type ExtResult = Result<Option<RuntimeValue>, Trap>;

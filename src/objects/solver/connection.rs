use crate::types::Cost;

#[derive(Clone)]
pub struct Connection {
    pub index : u32,
    pub cost : Cost
}

unsafe impl Send for Connection {}

unsafe impl Sync for Connection {}
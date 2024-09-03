use crate::types::{Cost, Index};

#[derive(Clone)]
pub struct Connection {
    pub index : Index,
    pub cost : Cost
}

unsafe impl Send for Connection {}

unsafe impl Sync for Connection {}
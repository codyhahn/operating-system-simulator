use std::rc::Rc;

pub struct Program {
    pub id: u32,
    pub priority: u32,
    pub instruction_buffer_size: usize,
    pub in_buffer_size: usize,
    pub out_buffer_size: usize,
    pub temp_buffer_size: usize,
    pub data: Rc<[u32]>
}
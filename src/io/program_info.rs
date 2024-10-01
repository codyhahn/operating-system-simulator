pub struct ProgramInfo {
    pub id: u32,
    pub priority: u32,
    pub instruction_buffer_size: usize,
    pub in_buffer_size: usize,
    pub out_buffer_size: usize,
    pub temp_buffer_size: usize,
    pub data_start_idx: usize
}
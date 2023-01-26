use crate::timer::get_time;


pub enum HostClint {
    SBI,
    Direct
}

impl HostClint {
    pub fn get_mtime(&self) -> usize {
        match self {
            Self::SBI => { get_time() },
            _ => { unimplemented!() }
        }
    }
}


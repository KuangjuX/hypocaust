use crate::timer::get_time;


#[allow(unused)]
pub enum HostClint {
    SBI,
    Direct
}

impl HostClint {
    #[allow(unused)]
    pub fn get_mtime(&self) -> usize {
        match self {
            Self::SBI => { get_time() },
            _ => { unimplemented!() }
        }
    }
}


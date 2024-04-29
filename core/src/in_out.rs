#[derive(Debug, Clone, Copy)]
pub enum OutPort {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
}

#[derive(Debug, Clone, Copy)]
pub enum InPort {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
}

pub trait InOut {
   fn write(&self, port: OutPort, value: u8);
   fn read(&self, port: InPort) -> u8;
}

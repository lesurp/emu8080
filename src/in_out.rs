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
    fn write(&self, port: u8, value: u8);
    fn read(&self, port: u8) -> u8;
}

pub struct DummyInOut;
impl InOut for DummyInOut {
    fn write(&self, _: u8, _: u8) {}

    fn read(&self, _: u8) -> u8 {
        panic!("This is a dummy implementation, this should not actually be called!");
    }
}

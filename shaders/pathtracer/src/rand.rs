#[derive(Default)]
pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn from_seed(seed: u32) -> Self {
        Self { state: seed }
    }

    pub fn next_float(&mut self) -> f32 {
        f32::from_bits((self.next() >> 9) | 0x3f80_0000) - 1.
    }

    pub fn next(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }
}

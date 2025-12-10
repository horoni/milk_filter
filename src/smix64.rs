#![allow(dead_code)]

pub trait Rng {
    fn new() -> Self;
    fn from_seed(seed: u64) -> Self;
    fn gen_u64(&mut self) -> u64;
    fn gen_u32(&mut self) -> u32;
    fn gen_f64(&mut self) -> f64;
    fn gen_f32(&mut self) -> f32;
}

pub struct SplitMix64 {
    state: u64,
}

impl Rng for SplitMix64 {
    /// Construxts Rng with `state` = 0
    fn new() -> Self {
        Self { state: 0 }
    }

    /// Init `state` with provided `seed`
    fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline]
    fn gen_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    #[inline]
    fn gen_u32(&mut self) -> u32 {
        (self.gen_u64() >> 32) as u32
    }

    #[inline]
    fn gen_f64(&mut self) -> f64 {
        self.gen_u64() as f64 / (1_u128).wrapping_shl(64) as f64
    }

    #[inline]
    fn gen_f32(&mut self) -> f32 {
        self.gen_u32() as f32 / (1_u64).wrapping_shl(32) as f32
    }
}

pub fn random() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    use std::time::{SystemTime, UNIX_EPOCH};
    #[cfg(target_arch = "wasm32")]
    use web_time::{SystemTime, UNIX_EPOCH};

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    SplitMix64::from_seed(seed).gen_u64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_u64() {
        let mut rng = SplitMix64::from_seed(1234567);
        assert_eq!(rng.gen_u64(), 6457827717110365317);
        assert_eq!(rng.gen_u64(), 3203168211198807973);
        assert_eq!(rng.gen_u64(), 9817491932198370423);
        assert_eq!(rng.gen_u64(), 4593380528125082431);
        assert_eq!(rng.gen_u64(), 16408922859458223821);
    }
}

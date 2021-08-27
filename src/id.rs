use rand::{prelude::Distribution, Rng};
use serde::{Deserialize, Serialize};

pub fn generate_random_id() -> String {
    let rng = &mut rand::thread_rng();
    let result: String = rng
        .sample_iter(Omeglenumeric)
        .take(8)
        .map(char::from)
        .collect();

    return result;
}

// implementation was straight up copied from Alphanumeric implementation,
// but edited to the omegle's charset.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Omeglenumeric;
impl Distribution<u8> for Omeglenumeric {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
        const RANGE: u32 = 34;
        const GEN_ASCII_STR_CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ\
                0123456789";

        loop {
            let var = rng.next_u32() >> (32 - 6);

            if var < RANGE {
                return GEN_ASCII_STR_CHARSET[var as usize];
            }
        }
    }
}

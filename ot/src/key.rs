use rand::thread_rng;
use num::bigint::{BigInt, RandBigInt};
use openssl::{bn::BigNumRef, rsa::Rsa};
use crate::PublicKey;

// Generate a new RSA key pair.
pub fn gen_key(size: u32) -> (BigInt, PublicKey) {
    let rsa = Rsa::generate(size).unwrap();
    let d = rsa.d();
    let e = rsa.e();
    let n = rsa.n();
    (convert_big(d), (convert_big(n), convert_big(e)))
}

// Generate a random BigInt.
pub fn gen_rand(size: u32) -> BigInt {
    let mut rng = thread_rng();
    rng.gen_bigint(size as u64)
}

// Convert from openssl's bignum to num's.
fn convert_big(bn: &BigNumRef) -> BigInt {
    let string = bn.to_string();
    BigInt::parse_bytes(string.as_bytes(), 10).unwrap()
}

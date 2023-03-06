use std::io::{self, prelude::*, BufRead};
use num::bigint::{BigInt, RandBigInt};
use rand::thread_rng;
use openssl::{bn::BigNumRef, rsa::Rsa};
use session_types::*;

// Value for OT sender's RSA key.
const KEY_SIZE: u32 = 2048;

// The type of an RSA based 1-of-2 Oblivious Transfer protocol.
//
// Protocol and comments taken from [1] to illustrate the clean mapping of
// protocol code to descriptions.
//
// [1]: https://en.wikipedia.org/wiki/Oblivious_transfer
type PublicKey = (BigInt, BigInt);
type RandOptions = (BigInt, BigInt);
type OT = Send<(PublicKey, RandOptions),
                Recv<BigInt,
                     Send<RandOptions,
                          Eps>>>;

// Helper Cryptography Functions
// -----------------------------

// Generate a new RSA key pair.
fn gen_key(size: u32) -> (BigInt, PublicKey) {
    let rsa = Rsa::generate(size).unwrap();
    let d = rsa.d();
    let e = rsa.e();
    let n = rsa.n();
    (convert_big(d), (convert_big(n), convert_big(e)))
}

// Generate a random BigInt.
fn gen_rand(size: u32) -> BigInt {
    let mut rng = thread_rng();
    rng.gen_bigint(size as u64)
}

// Convert from openssl's bignum to num's.
fn convert_big(bn: &BigNumRef) -> BigInt {
    let string = bn.to_string();
    BigInt::parse_bytes(string.as_bytes(), 10).unwrap()
}

// Read a u32 from STDIN.
fn read_u32(prompt: &str) -> u32 {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    print!("{}: ", prompt);
    stdout.flush().unwrap();

    let mut line = String::new();
    stdin.lock().read_line(&mut line).unwrap();
    line.trim().parse().unwrap_or_else(|e| {
        panic!("{}", e);
    })
}

// Select two messages for 1-of-2 OT.
fn read_choices() -> (BigInt, BigInt) {
    let m0 = read_u32("Alice offer 0");
    let m1 = read_u32("Alice offer 1");
    (m0.into(), m1.into())
}

// Select bit for 1-of-2 OT.
fn read_choice() -> bool {
    let choice = read_u32("Bob choose 0 or 1");
    if choice > 1 {
        panic!("choice must be 0 or 1");
    }
    choice != 0
}

// The OT Protocol Implementation
// ------------------------------

// OT protocol sender (Alice).
pub fn sender(c: Chan<(), OT>) {
    // Alice has two messages, m0, m1 and wants to send exactly one of them to
    // Bob. Bob does not want Alice to know which one he receives.
    let (m0, m1) = read_choices();
    // Alice generates an RSA key pair, comprising the modulus N, the public
    // exponent e and the private exponent d.
    let (d, (n, e)) = gen_key(KEY_SIZE);
    // She also generates two random values, x0, x1,
    let (x0, x1) = (gen_rand(32), gen_rand(32));
    // and sends them to Bob along with her public modulus and exponent.
    let (c, v) = c.send(((n.clone(), e), (x0.clone(), x1.clone()))).recv();
    // Alice doesn't know (and hopefully cannot determine) which of x0 and x1
    // Bob chose. She applies both of her random values and comes up with two
    // possible values for k. One of these will be equal to k and can be
    // correctly decrypted by Bob (but not Alice), while the other will produce
    // a meaningless random value that does not reveal any information about k.
    let (k0, k1) = ((v.clone()-x0 % &n).modpow(&d, &n),
                    (v.clone()-x1 % &n).modpow(&d, &n));
    // She combines the two secret messages with each of the possible keys, m0
    // and m1 and sends them both to Bob.
    c.send((m0+k0, m1+k1)).close();
}

// OT's complement protocol for the receiver (Bob).
pub fn receiver(c: Chan<(), <OT as Dual>::Dual>) {
    // Bob waits for the OT sender...
    let (c, ((n, e), (x0, x1))) = c.recv();
    // then generates a random value k,
    let k = gen_rand(KEY_SIZE-1);
    // and picks b to be either 0 or 1,
    let b = read_choice();
    // and selects either the first or second xb,
    let xb = if b { x1 } else { x0 };
    // and blinds xb by computing v,
    let v = (xb + k.clone().modpow(&e, &n)) % &n;
    // which he sends to Alice.
    let (c, (m0,m1)) = c.send(v).recv();
    // Bob knows which of the two messages can be un-blinded with k, so he
    // is able to compute exactly one of the messages mb.
    let mb = if b { (m1 - k) % &n } else { (m0 - k) % &n };
    c.close();

    // We're done!
    println!("Bob got: {}", mb);
}

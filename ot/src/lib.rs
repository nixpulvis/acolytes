use num::bigint::BigInt;
use session_types::*;

mod key;
use key::{gen_rand, gen_key};

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

/// The receiver can choose only one of two values of the sender's known tuple.
#[derive(Copy, Clone)]
pub enum Choice {
    Left,
    Right,
}
use Choice::{Left, Right};

// The OT Protocol Implementation
// ------------------------------
/// OT protocol sender (Alice).
/// Alice has two messages, m0, m1 and wants to send exactly one of them to
/// Bob. Bob does not want Alice to know which one he receives.
pub fn sender(m: (BigInt, BigInt), c: Chan<(), OT>) {
    let (m0, m1) = m;
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
pub fn receiver<C>(chooser: C, c: Chan<(), <OT as Dual>::Dual>)
where C: Fn((&BigInt, &BigInt), (&BigInt, &BigInt)) -> Choice
{
    // Bob waits for the OT sender...
    let (c, ((n, e), (x0, x1))) = c.recv();
    // then generates a random value k,
    let k = gen_rand(KEY_SIZE-1);
    // and picks b to be either 0 or 1,
    let b = chooser((&n, &e), (&x0, &x1));
    // and selects either the first or second xb,
    let xb = match b { Left => x0, Right => x1 };
    // and blinds xb by computing v,
    let v = (xb + k.clone().modpow(&e, &n)) % &n;
    // which he sends to Alice.
    let (c, (m0,m1)) = c.send(v).recv();
    // Bob knows which of the two messages can be un-blinded with k, so he
    // is able to compute exactly one of the messages mb.
    let mb = match b { Left => (m0 - k) % &n, Right => (m1 - k) % &n };
    c.close();

    // We're done!
    println!("Bob got: {}", mb);
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;
    use std::thread;
    use std::time::Duration;
    use channels::Channel;
    use super::*;

    // TODO: Pull out the `read_choice(s)` logic to main.
    #[ignore]
    #[test]
    fn oblivious_transfer() {
        let addr = "127.0.0.1:2200";


        thread::spawn(move || {
            let c = Channel::accept_from_socket_addr(addr).unwrap();
            let ch = Chan(c, PhantomData);
            receiver(|_,_| { Choice::Left }, ch);
        });
        thread::sleep(Duration::from_millis(10));
        thread::spawn(move || {
            let identity = "nixpulvis".to_string();
            let c = Channel::connect_to_socket_addr(identity, addr).unwrap();
            let ch = Chan(c, PhantomData);
            sender((BigInt::from(1), BigInt::from(2)), ch);
        }).join().unwrap();
    }
}

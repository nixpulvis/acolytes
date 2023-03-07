use std::env;
use std::io::{self, prelude::*, BufRead};
use std::marker::PhantomData;
use num::bigint::BigInt;
use docopt::Docopt;
use channels::Channel;
use session_types::Chan;
use ot::{Choice, sender, receiver};

// const USAGE: &'static str = "
// Usage: ot --receiver [<choice>]
//        ot --sender [<offers>]
// ";
const USAGE: &'static str = "
Usage: ot --receiver
       ot --sender
";

fn main() {
    // Startup our logger, to watch the protocol in action.
    env_logger::init();

    let addr = "127.0.0.1:1337";
    let args = Docopt::new(USAGE)
        .and_then(|d| d.argv(env::args().into_iter()).parse())
        .unwrap_or_else(|e| e.exit());

    if args.get_bool("--sender") {
        let identity = "nixpulvis".to_string();
        let c = Channel::connect_to_socket_addr(identity, addr).unwrap();
        let ch = Chan(c, PhantomData);
        sender(read_choices(), ch);
    } else if args.get_bool("--receiver") {
        let c = Channel::accept_from_socket_addr(addr).unwrap();
        let ch = Chan(c, PhantomData);
        receiver(|_,_| read_choice(), ch);
    }
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
    let m0 = read_u32("Alice offer left");
    let m1 = read_u32("Alice offer right");
    (m0.into(), m1.into())
}

// Select bit for 1-of-2 OT.
fn read_choice() -> Choice {
    let choice = read_u32("Bob choose left (0) or right (1)");
    if choice > 1 {
        panic!("choice must be 0 or 1");
    }
    if choice == 0 {
        Choice::Left
    } else {
        Choice::Right
    }
}

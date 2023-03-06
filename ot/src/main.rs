use std::env;
use std::marker::PhantomData;
use docopt::Docopt;
use channels::Channel;
use session_types::Chan;
use ot::{sender, receiver};

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
        sender(ch);
    } else if args.get_bool("--receiver") {
        let c = Channel::accept_from_socket_addr(addr).unwrap();
        let ch = Chan(c, PhantomData);
        receiver(ch);
    }
}

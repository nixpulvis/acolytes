use session_types::*;
use std::io::{self, Write};

// Type of an admittance protocol.
//
// In this protocol a client asks an admittor for addmitance until some expire
// time. The expire time is just a number, and could be something like hours for
// and amusment park, or days allowed to ski. This protocol could be part of a
// larger protocol for time-bounded computation, for example.

type Id = u64;
type Expire = u64;
type Accept = Send<Expire, Eps>;
type Reject = Eps;
type Admittance = Recv<Id, Choose<Accept, Reject>>;
type Client = <Admittance as Dual>::Dual;

fn admittor(c: Chan<(), Admittance>) {
    let (c, id) = c.recv();
    // Leet tickets get in for a long time.
    if id == 1337 {
        c.sel0().send(1337).close();
    // Let in anyone mod 3 (~1/3 of requests).
    } else if id % 3 == 0 {
        c.sel0().send(100).close();
    // Everyone else we just ignore at this point.
    } else {
        c.sel1().close();
    }
}

fn client(c: Chan<(), Client>) {
    // Read an id number from STDIN.
    let n = read_input("Enter a ticket number: ");

    match c.send(n).offer() {
        Branch::Left(c) => {
            let (c, expire) = c.recv();
            // Client expects to be admitted for at least 60 minutes.
            assert!(expire > 60);
            println!(
                r###"
                    ---------------------------
                    ACCEPTED for {} minute(s)
                    ---------------------------
                    "###,
                expire
            );
            // We're done.
            c.close();
        }
        Branch::Right(_) => println!("\nDAMN IT!\n"),
    };
}

fn read_input(m: &str) -> u64 {
    print!("{}", m);
    io::stdout().flush().expect("failed to flush stdout");
    let mut input_text = String::new();
    io::stdin()
        .read_line(&mut input_text)
        .expect("failed to read from stdin");
    match input_text.trim().parse::<u64>() {
        Ok(i) => return i,
        Err(_) => panic!("not given valid u64"),
    };
}

fn main() {
    connect(admittor, client);
}


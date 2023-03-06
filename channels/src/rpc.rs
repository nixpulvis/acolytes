use std::fmt::Debug;
use std::io::Error;
use serde::{Serialize, Deserialize};
use super::Channel;

/// Cannel RPC interface.
///
/// `call` -> `accept_call` functions for evaluating a function `D -> C`. Call
/// provides the argument, and `accept_call` receives the input, passes it
/// through to it's underlying function, and then finally sends it back to the
/// caller.
impl Channel {
    pub fn call<D, C>(&mut self, domain: &D) -> Result<C, Error>
        where D: Serialize + Debug,
              C: for<'de> Deserialize<'de> + Debug,
    {
        self.send(&domain)?;
        let codomain = self.recv()?;
        Ok(codomain)
    }

    pub fn accept_call<D, C>(&mut self, func: &dyn Fn(&D) -> C) -> Result<D, Error>
        where D: for<'de> Deserialize<'de> + Debug,
              C: Serialize + Debug,
    {
        let domain = self.recv()?;
        // NOTE: We could make this check the `Result` of the call by changing
        // the type of this function to `&Fn(&D) -> Result<C, ...>`.
        let codomain = (func)(&domain);
        self.send(&codomain)?;
        Ok(domain)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use super::*;

    #[test]
    fn accept_and_connect() {
        thread::spawn(move || {
            let mut accept_channel = Channel::accept_from_socket_addr("127.0.0.1:1337").unwrap();
            let input = accept_channel.accept_call(&|b: &bool| !b).unwrap();
            assert!(input);
        });
        thread::sleep(Duration::from_millis(10));
        thread::spawn(move || {
            let mut connect_channel = Channel::connect_to_socket_addr("nixpulvis".into(), "127.0.0.1:1337").unwrap();
            assert_eq!(false, connect_channel.call(&true).unwrap());
        }).join().unwrap();
    }

    // #[test]
    // fn remote_call() {
    //     thread::spawn(move || {
    //         accept_channel()
    //     });
    //     thread::sleep(Duration::from_millis(10));
    //     assert_eq!(false, connect_channel().call(&10).unwrap());
    // }

    // #[test]
    // fn remote_calls() {
    //     thread::spawn(move || {
    //         let mut channel = accept_channel();
    //         channel.accept_call(&|b: bool| !b).unwrap();
    //         channel.accept_call(&|s: bool| format!("{}", s)).unwrap();
    //     });
    //     thread::sleep(Duration::from_millis(10));
    //     let mut channel = connect_channel();
    //     assert_eq!(false, channel.call(&true).unwrap());
    //     assert_eq!("false", channel.call::<_, String>(&false).unwrap());
    // }
}

//! Bidirectional channel (accept, recv) and (connect, send).
use std::fmt::{self, Debug};
use std::io::{Error, ErrorKind};
use std::net::{ToSocketAddrs, TcpStream, TcpListener};
use serde::{Serialize, Deserialize};
use log::{info, error};

/// Sending and receiving *whole* wire messages.
///
// TODO: Do we always want TCP?
// TODO: Option<Id> should be `enum Info`
// ```rust
// enum Info {
//     Unauthenticated,
//     Authenticated(Id),
//     ...?
// }
// ```
pub struct Channel(Option<String>, TcpStream);

/// Channel information.
impl Channel {
    pub fn info(&self) -> &str {
        match &self.0 {
            Some(s) => s,
            None => "",
        }
    }
}

mod rpc;

// /// either type and Deserialize impl.
// pub mod either;
// use self::either::Either;

/// Channel establishment.
///
/// This provides a simple authentication scheme.
impl Channel {
    pub fn accept_from_socket_addr<A: ToSocketAddrs + Debug>(socket_addr: A) -> Result<Channel, Error> {
        let listener = TcpListener::bind(&socket_addr)?;
        info!("accept on: {:?}", &socket_addr);
        let (stream, _addr) = listener.accept()?;
        info!("accepting client: {:?}, {:?}", stream, _addr);
        Channel::accept_from_tcp_stream(stream)

        // TODO: We need a proper event loop, mio, or romio?
        // listener.set_nonblocking(true)?;
        // loop {
        //     match listener.accept() {
        //         Ok((mut stream, _addr)) => {
        //             println!("accepting client: {:?}, {:?}", stream, _addr);
        //             return Channel::accept_from_tcp_stream(stream);
        //         },
        //         Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
        //             println!("waiting..");
        //             continue;
        //         },
        //         Err(e) => {
        //             println!("accept error: {:?}", e);
        //             return Err(e)
        //         }
        //     }
        // }
    }

    /// Accept from a tcp stream, we must get some "info" and return "ok".
    /// TODO: <question> Boolean to tell channel it was not good?
    pub fn accept_from_tcp_stream(stream: TcpStream) -> Result<Channel, Error> {
        let mut channel = Channel(None, stream);
        let id = channel.accept_call(&|id: &String| {
            // NOTE: This is obviously not the final dynamic check. But it shows
            // how we can do some logic before we truly establish the `Channel`.
            if id == "nixpulvis" {
                "ok"
            } else {
                "err"
            }
        })?;
        channel.0 = id.into();
        info!("accepted {:?}", channel);
        Ok(channel)
    }

    /// Create a stream, and connect to it.
    pub fn connect_to_socket_addr<A: ToSocketAddrs>(info: String, socket_addr: A) -> Result<Channel, Error> {
        let stream = TcpStream::connect(&socket_addr)?;
        Self::connect_to_tcp_stream(info, stream)

        // let first_addr = socket_addr.to_socket_addrs().unwrap().next().unwrap();
        // let stream = TcpStream::connect_timeout(&first_addr, Duration::from_secs(2))?;
        // TODO: We need a proper event loop, mio, or romio?
        // stream.set_nonblocking(true)?;
    }

    /// Connect to a tcp stream, we'll send the "info" for this channel, we
    /// must get back the response "ok".
    pub fn connect_to_tcp_stream(info: String, stream: TcpStream) -> Result<Channel, Error> {
        let mut channel = Channel(Some(info.clone()), stream);
        let ack = channel.call::<String, String>(&info)?;
        if ack == "ok" {
            info!("authenticated: {:?}", info);
            Ok(channel)
        } else {
            let error = format!("invalid channel ack: {}", ack);
            Err(Error::new(ErrorKind::Other, error))
        }
    }
}

/// Message passing send, and receive functions.
impl Channel {
    pub fn send<T: Serialize + Debug>(&mut self, message: &T) -> Result<(), Error> {
        // self.1.set_write_timeout(Some(Duration::from_secs(2)))?;
        bincode::serialize_into(&mut self.1, message).map_err(|e| {
            error!("error sending: {}", e);
            Error::new(ErrorKind::Other, e)
        })?;
        info!("send({:?}) {:?}", message, self.0);
        Ok(())
    }

    pub fn recv<T>(&mut self) -> Result<T, Error>
    where for<'de> T: Deserialize<'de> + Debug
    {
        // self.1.set_read_timeout(Some(Duration::from_secs(2)))?;
        let message = bincode::deserialize_from(&mut self.1).map_err(|e| {
            error!("error receiving: {}", e);
            Error::new(ErrorKind::Other, e)
        })?;
        info!("recv({:?}) {:?}", message, self);
        Ok(message)
    }

    // /// Receive an `Either<T, U>` type from the wire. This provides support for channel consumers
    // /// to logically branch based on channel messages.
    // pub fn recv_either<T, U>(&mut self) -> Result<Either<T, U>, Error>
    //     where T: for<'de> Deserialize<'de> + Debug,
    //           U: for<'de> Deserialize<'de> + Debug,
    // {
    //     Either::<T, U>::deserialize_from(&mut self.1)
    // }
}

impl Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match (self.1.local_addr(), self.1.peer_addr()) {
            (Ok(l), Ok(p)) => write!(f, "Channel(info: {:?}, us: {:?}, them: {:?})", self.0, l, p),
            (Ok(l), Err(p)) => write!(f, "Channel(info: {:?}, us: {:?}, them: <{:?}>)", self.0, l, p),
            (Err(l), Ok(p)) => write!(f, "Channel(info: {:?}, us: <{:?}>, them: {:?})", self.0, l, p),
            (Err(l), Err(p)) => write!(f, "Channel(info: {:?}, us: <{:?}>, them: <{:?}>", self.0, l, p),
        }
    }
}

 impl Drop for Channel {
    fn drop(&mut self) {
        info!("dropping channel: {:?}", self);
    }
 }

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use super::*;

    // TODO: Remove hard-coded "nixpulvis"
    // Accept  = ?["nixpulvis"];⊕ [!["ok"],ε]
    // Connect = !["nixpulvis"];& [?["ok"],ε]
    #[test]
    fn accept_and_connect() {
        thread::spawn(move || {
            let result = Channel::accept_from_socket_addr("127.0.0.1:1337");
            assert!(result.is_ok());
        });
        thread::sleep(Duration::from_millis(10));
        thread::spawn(move || {
            let result = Channel::connect_to_socket_addr("nixpulvis".into(), "127.0.0.1:1337");
            assert!(result.is_ok());
        }).join().unwrap();
    }

    // Accept = ...
    // Connect = ...
    // A = Accept;  ?[u64]
    // B = Connect; ![u64]
    #[test]
    fn send_recv_number() {
        thread::spawn(move || {
            let mut channel= Channel::accept_from_socket_addr("127.0.0.1:1337").unwrap();
            let recv: u64 = channel.recv().unwrap();
            assert_eq!(1, recv);
        });
        thread::sleep(Duration::from_millis(10));
        thread::spawn(move || {
            let mut channel = Channel::connect_to_socket_addr("nixpulvis".into(), "127.0.0.1:1337").unwrap();
            assert!(channel.send(&1u64).is_ok());
        }).join().unwrap();
    }

    // Accept = ...
    // Connect = ...
    // A = Accept;  ?[u64];⊕ [![true],ε]
    // B = Connect; ![u64];& [?[true],ε]
    #[test]
    fn call_recv_send_number() {
        thread::spawn(move || {
            let mut channel = Channel::accept_from_socket_addr("127.0.0.1:1337").unwrap();
            let recv: u64 = channel.recv().unwrap();
            assert_eq!(1, recv);
            assert!(channel.send(&true).is_ok());
        });
        thread::sleep(Duration::from_millis(10));
        thread::spawn(move || {
            let mut channel = Channel::connect_to_socket_addr("nixpulvis".into(), "127.0.0.1:1337").unwrap();
            assert_eq!(true, channel.call(&1u64).unwrap());
        }).join().unwrap();
    }

    #[test]
    #[ignore]
    fn infinite_length_number() {
        thread::spawn(move || {
            let mut channel = Channel::accept_from_socket_addr("127.0.0.1:1337").unwrap();
            let recv: u32 = channel.recv().unwrap();
            // We never get here...
            assert_eq!(1, recv);
        });
        thread::sleep(Duration::from_millis(10));
        thread::spawn(move || {
            let mut channel = Channel::connect_to_socket_addr("info".into(), "127.0.0.1:1337").unwrap();
            // Write a 1.0, then 0s forever.
            channel.send(&1.0).unwrap();
            loop { channel.send(&0).unwrap() }
        }).join().unwrap();
    }
}

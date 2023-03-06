use std::fmt::Debug;
use std::marker::{self, PhantomData};
use std::mem::transmute;
use std::thread;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use log::info;

pub enum Branch<L, R> {
    Left(L),
    Right(R),
}

// protocols

pub struct Z;
pub struct Eps;
pub struct Send<T,P>(PhantomData<(T, P)>);
pub struct Recv<T,P>(PhantomData<(T, P)>);
pub struct Offer<P,Q>(PhantomData<(P, Q)>);
pub struct Choose<P,Q>(PhantomData<(P, Q)>);
pub struct Rec<P>(PhantomData<P>);
pub struct Var<N>(PhantomData<N>);

pub trait Dual {
    type Dual;
}

impl Dual for Eps {
    type Dual = Self;
}

impl<T, P: Dual> Dual for Send<T,P> {
    type Dual = Recv<T,P::Dual>;
}

impl<T, P: Dual> Dual for Recv<T,P> {
    type Dual = Send<T,P::Dual>;
}

impl<P: Dual, Q: Dual> Dual for Offer<P,Q> {
    type Dual = Choose<P::Dual, Q::Dual>;
}

impl<P: Dual, Q: Dual> Dual for Choose<P,Q> {
    type Dual = Offer<P::Dual, Q::Dual>;
}

impl<P: Dual> Dual for Rec<P> {
    type Dual = Rec<P::Dual>;
}

use channels::Channel;

pub struct Chan<E,P>(
    pub Channel,
    pub PhantomData<(E,P)>,
);

// TODO: We're working with serde/bincode TCP channels, this will be cool to
// add if we can abstract over something more like what the original authors
// did with crossbeam locally.
//
// /// A channel on which we will receive channels
// type ChanChan<P> = Offer<Eps, Recv<Chan<(), P>, Var<Z>>>;

/// Connect two functions using a session typed channel.
pub fn connect<F1, F2, P>(srv: F1, cli: F2)
where
    F1: Fn(Chan<(), P>) + marker::Send + 'static,
    F2: Fn(Chan<(), P::Dual>) + marker::Send + 'static,
    P: Dual + marker::Send + 'static,
    P::Dual: Dual + marker::Send + 'static
{
    let t = thread::spawn(move || {
        let s = Channel::accept_from_socket_addr("127.0.0.1:1337").unwrap();
        let c = Chan(s, PhantomData);
        srv(c);
    });
    thread::sleep(Duration::from_millis(10));
    let c = Channel::connect_to_socket_addr("nixpulvis".into(), "127.0.0.1:1337").unwrap();
    cli(Chan(c, PhantomData));
    t.join().unwrap();
}

impl<E> Chan<E, Eps> {
    /// Close a channel. Should always be used at the end of your program.
    pub fn close(self) {
        info!("closing session");
    }
}

impl<E, P> Chan<(P, E), Var<Z>> {
    /// Recurse to the environment on the top of the environment stack.
    #[must_use]
    pub fn zero(self) -> Chan<(P, E), P> {
        unsafe { transmute(self) }
    }
}

impl<E, P, T> Chan<E, Send<T, P>>
where T: Serialize + Debug
{
    /// Send a value of type `T` over the channel. Returns a channel with
    /// protocol `P`
    #[must_use]
    pub fn send(mut self, v: T) -> Chan<E, P> {
        info!("sending {:?}", v);
        self.0.send(&v).unwrap();
        unsafe { transmute(self) }
    }
}

impl<E, P, T> Chan<E, Recv<T, P>>
where T: for<'de> Deserialize<'de> + Debug
{
    /// Receives a value of type `T` from the channel. Returns a tuple
    /// containing the resulting channel and the received value.
    #[must_use]
    pub fn recv(mut self) -> (Chan<E, P>, T) {
        info!("receiving...");
        let v = self.0.recv().unwrap();
        info!("received {:?}", v);
        unsafe { (transmute(self), v) }
    }
}

impl<E, P, Q> Chan<E, Choose<P, Q>> {
    /// Perform an active choice, selecting protocol `P`.
    #[must_use]
    pub fn sel0(mut self) -> Chan<E, P> {
        info!("selecting 0");
        self.0.send(&true).unwrap();
        unsafe { transmute(self) }
    }

    /// Perform an active choice, selecting protocol `Q`.
    #[must_use]
    pub fn sel1(mut self) -> Chan<E, Q> {
        info!("selecting 1");
        self.0.send(&false).unwrap();
        unsafe { transmute(self) }
    }
}

impl<E, P, Q> Chan<E, Offer<P, Q>> {
    /// Passive choice. This allows the other end of the channel to select one
    /// of two options for continuing the protocol: either `P` or `Q`.
    #[must_use]
    pub fn offer(mut self) -> Branch<Chan<E, P>, Chan<E, Q>> {
        info!("offering...");
        if self.0.recv().unwrap() {
            info!("offered 0");
            Branch::Left(unsafe { transmute(self) })
        } else {
            info!("offered 1");
            Branch::Right(unsafe { transmute(self) })
        }
    }
}

impl<E, P> Chan<E, Rec<P>> {
    /// Enter a recursive environment, putting the current environment on the
    /// top of the environment stack.
    #[must_use]
    pub fn enter(self) -> Chan<(P, E), P> {
        info!("enter");
        unsafe { transmute(self) }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    type Hi = Send<String, Eps>;

    fn sender(c: Chan<(), Hi>) {
        c.send("hi".into()).close();
    }

    fn receiver(c: Chan<(), <Hi as Dual>::Dual>) {
        let (c, s) = c.recv();
        assert_eq!("hi", s);
        c.close();
    }

    #[test]
    fn send_recv() {
        connect(sender, receiver);
    }

    type Opf = Offer<Send<u64,Eps>,Eps>;

    fn offerer(c: Chan<(), Opf>) {
        match c.offer() {
            Branch::Left(c) => {
                c.send(42).close();
            }
            Branch::Right(c) => {
                c.close();
            }
        }
    }

    fn chooser(c: Chan<(), <Opf as Dual>::Dual>) {
        let r = 6; // TODO: gen rand.
        if r % 2 == 0 {
            let (c, v) = c.sel0().recv();
            assert_eq!(42, v);
            c.close();
        } else {
            c.sel1().close()
        }
    }

    #[test]
    fn offer_choose() {
        connect(offerer, chooser);
    }

    // #[test]
    // fn var() {}
    // #[test]
    // fn rec() {}
    // #[test]
    // fn z() {}
}

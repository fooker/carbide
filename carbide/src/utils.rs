pub mod stream {
    use futures::{Async, Future, Stream, stream, try_ready};
    use futures::sync::mpsc;

    pub mod broadcast {
        use super::*;

        pub struct Broadcast<S, I, E>
            where S: Stream<Item=I, Error=E>,
                  I: Clone {
            stream: stream::Fuse<S>,
            forks: Vec<mpsc::Sender<I>>,
        }

        impl <S, I, E> Broadcast<S, I, E>
            where S: Stream<Item=I, Error=E>,
                  I: Clone {

            pub fn new(stream: S) -> Self {
                return Self {
                    stream: stream.fuse(),
                    forks: Vec::new(),
                };
            }

            pub fn receive(&mut self) -> mpsc::Receiver<I> {
                let (tx, rx) = mpsc::channel(0);
                self.forks.push(tx);
                return rx;
            }
        }

        impl <S, I, E> Future for Broadcast<S, I, E>
            where S: Stream<Item=I, Error=E>,
                  I: Clone {

            type Item = ();
            type Error = E;

            fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
                for tx in self.forks.iter_mut() {
                    match tx.poll_ready() {
                        Ok(Async::Ready(_)) => continue,
                        Ok(Async::NotReady) => return Ok(Async::NotReady),
                        Err(err) => panic!(err),
                    }
                }

                let item = match try_ready!(self.stream.poll()) {
                    Some(item) => item,
                    None => return Ok(Async::Ready(())),
                };

                for tx in self.forks.iter_mut() {
                    tx.try_send(item.clone()).unwrap();
                }

                return Ok(Async::NotReady);
            }
        }
    }
}

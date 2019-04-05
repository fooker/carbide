use std::collections::VecDeque;

use futures::Async;
use futures::AsyncSink;
use futures::Sink;
use futures::Stream;
use futures::stream::Peekable;
use futures::sync::BiLock;
use futures::sync::oneshot;
use futures::try_ready;

use super::proto;

const BUFFER_SIZE: usize = 128;

pub type Command = (String, oneshot::Sender<proto::GrblResponse>);

struct Inner {
    outstanding: VecDeque<(oneshot::Sender<proto::GrblResponse>, usize)>,
    remaining: usize,
}

pub struct Sender<S, E>
    where S: Stream<Item=Command, Error=E> {
    stream: Peekable<S>,
    inner: BiLock<Inner>,
}

pub struct Tracker {
    inner: BiLock<Inner>,
}

pub fn sender<S, E>(stream: S) -> (Sender<S, E>, Tracker)
    where S: Stream<Item=Command, Error=E> {
    let (inner1, inner2) = BiLock::new(Inner {
        outstanding: VecDeque::new(),
        remaining: BUFFER_SIZE,
    });

    return (Sender {
        stream: stream.peekable(),
        inner: inner1,
    }, Tracker {
        inner: inner2,
    });
}

impl<S, E> Stream for Sender<S, E>
    where S: Stream<Item=Command, Error=E> {
    type Item = String;
    type Error = E;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        let next = try_ready!(self.stream.peek());
        if let Some(next) = next {
            let mut inner = match self.inner.poll_lock() {
                Async::Ready(inner) => inner,
                Async::NotReady => return Ok(Async::NotReady),
            };

            if inner.remaining >= next.0.len() {
                let next = try_ready!(self.stream.poll()).unwrap();

                inner.remaining -= next.0.len();
                inner.outstanding.push_back((next.1, next.0.len()));

                return Ok(Async::Ready(Some(next.0)));
            } else {
                return Ok(Async::NotReady);
            }
        } else {
            return Ok(Async::Ready(None));
        }
    }
}

impl Sink for Tracker {
    type SinkItem = proto::GrblResponse;
    type SinkError = ();

    fn start_send(&mut self, item: Self::SinkItem) -> Result<AsyncSink<Self::SinkItem>, Self::SinkError> {
        let mut inner = match self.inner.poll_lock() {
            Async::Ready(inner) => inner,
            Async::NotReady => return Ok(AsyncSink::NotReady(item)),
        };

        let (sender, length) = inner.outstanding.pop_front()
            .expect("Command tracking went totally wrong");

        sender.send(item)
            .expect("Receiver closed");

        inner.remaining += length;

        return Ok(AsyncSink::Ready);
    }

    fn poll_complete(&mut self) -> Result<Async<()>, Self::SinkError> {
        return Ok(Async::Ready(()));
    }

    fn close(&mut self) -> Result<Async<()>, Self::SinkError> {
        return Ok(Async::Ready(()));
    }
}
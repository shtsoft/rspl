//! This module provides an implementation of streams as overeager message receivers.
//! Here, 'overeager' means that one message is always received in advance.

use super::Stream;

use crossbeam::channel::{bounded, unbounded};
use crossbeam::channel::{Receiver, Sender};

/// Abstracts receivers of messages of type `X` that always buffer one message.
pub struct OvereagerReceiver<X> {
    message: X,
    receiver: Receiver<X>,
}

impl<X> OvereagerReceiver<X> {
    /// Creates a channel with an overeager receiver.
    /// - `cap` is the capacity of the channel; `0` indicates an unbounded channel.
    /// - `message` is the initial placeholder that the overeager receiver holds in advance.
    ///
    /// # Examples
    ///
    /// Creating a stream with the head `true` and a tail determined by `tx`:
    ///
    /// ```
    /// let (tx, stream) = rspl::streams::overeager_receivers::OvereagerReceiver::channel(0, true);
    /// ```
    pub fn channel(cap: usize, message: X) -> (Sender<X>, Self) {
        let (tx, receiver) = if cap > 0 { bounded(cap) } else { unbounded() };
        (tx, Self { message, receiver })
    }
}

/// Note that when calling `tail`, this implementation may block the current thread if no message has been sent yet, or panic if the sender has been disconnected.
impl<X> Stream<X> for OvereagerReceiver<X> {
    fn head(&self) -> &X {
        &self.message
    }

    fn tail(mut self) -> Self {
        self.message = self.receiver.recv().unwrap();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel::unbounded as channel;

    use crate::assert_head_eq;
    use crate::assert_tail_starts_with;

    #[macro_export]
    macro_rules! enqueue {
        ($tx:expr, $xs:expr) => {
            for x in $xs {
                $tx.send(x).unwrap();
            }
        };
    }

    #[test]
    fn test_overeager_channel() {
        let (tx, mut stream) = OvereagerReceiver::channel(1, false);
        enqueue!(tx, [true]);
        assert_head_eq!(stream, false);
        assert_tail_starts_with!(stream, [true]);
    }

    #[test]
    fn test_head() {
        let (_, rx) = channel();
        let stream = OvereagerReceiver {
            message: true,
            receiver: rx,
        };
        assert!(stream.head());
    }

    #[test]
    fn test_tail() {
        let (tx, rx) = channel();
        let stream = OvereagerReceiver {
            message: false,
            receiver: rx,
        };
        enqueue!(tx, [true]);
        assert!(stream.tail().head());
    }
}

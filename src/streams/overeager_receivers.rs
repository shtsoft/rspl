//! This module provides an implementation of streams as overeager receivers of messages.
//! Here 'overeager' means that always one message is received in advanced.

use super::Stream;

use crossbeam::channel::Receiver;

/// [`OvereagerReceiver<X>`] abstracts receivers of message of type `X` which can buffer one message.
pub struct OvereagerReceiver<X> {
    /// overeagerly received message
    message: X,
    /// receiver of messages
    receiver: Receiver<X>,
}

impl<X> Stream<X> for OvereagerReceiver<X>
where
    X: Copy,
{
    /// Make the overeagerly received part of `self` the head.
    fn head(&self) -> X {
        self.message
    }

    /// Updated the message-buffer of `self` with the next `self` can get and consider it the tail.
    ///
    /// A panic is caused if receiving fails (probably due to a disconnected channel).
    fn tail(mut self) -> Self {
        let x = self.receiver.recv().unwrap();
        self.message = x;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel::unbounded as channel;

    #[test]
    fn test_head() {
        let (_, rx) = channel();
        let channel = OvereagerReceiver {
            message: true,
            receiver: rx,
        };
        assert!(channel.head());
    }

    #[test]
    fn test_tail() {
        let (tx, rx) = channel();
        let channel = OvereagerReceiver {
            message: false,
            receiver: rx,
        };
        tx.send(true).unwrap();
        assert!(channel.tail().head());
    }
}

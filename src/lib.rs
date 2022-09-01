//! rspl is a stream processor language based on [Hancock et al.](https://arxiv.org/pdf/0905.4813) using rust as meta-language.
//!
//! ## Design
//!
//! The idea of this stream processor language is to split the processing of streams into two parts:
//! One part for reading (getting) the first element of an input stream to decide what to do with the rest of that input stream depending on that element.
//! Another part for writing (putting) something to the output stream and offering to process some input stream if needed.
//! Combining these parts in various ways allows to flexibly construct stream processors as programs comprising a generalization of the well-known map-function on lists from functional programming.
//!
//! The following graphic illustrates how the two different kinds of stream processors ('getting' and 'putting') work:
//!
//! <pre>
//! h--t1--t2--t3--...          ha--t1--t2--t3--...
//! -                           -
//! |                           |
//! | Get(h |-> [SP](h))        | Put(hb, LAZY-[SP])
//! |                           |
//! v                           |
//! t1--t2--t3--...             |   t1--t2--t3--...
//! -                           |   -
//! |                           v   |
//! | [SP](h)                   hb--| LAZY-[SP]
//! |                               |
//! v                               v
//! ...                             ...
//! </pre>
//!
//! ## Usage
//!
//! To program a rspl-[`StreamProcessor`] you can just combine existing ones (perhaps obtained with [`map`]) by using the stream processor constructors [`StreamProcessor::Get`] and [`StreamProcessor::Put`].
//! The program can then be evaluated with [`StreamProcessor::eval`] on some kind of input stream.
//! The 'kind' of input stream is either your own implementation of the [`Stream`]-interface or one
//! from the submodules of the [`streams`]-module.
//! Either way, as result, [`StreamProcessor::eval`] produces an [`InfiniteList`].
//! To observe streams - and i.p. [`InfiniteList`]s - you can destruct them with [`Stream::head`] and [`Stream::tail`].
//! Moreover there are various functions helping with the destruction and construction of streams.
//!
//! # Examples
//!
//! rspl can serve as a framework for the nifty idea of event-driven programming with finite state machines as suggested [here](https://barrgroup.com/Embedded-Systems/How-To/State-Machines-Event-Driven-Systems). The example for the pattern there is implemented concretely as [integration test](https://github.com/aronpaulson/rspl/blob/master/tests/events.rs) for rspl and abstractly in the following to demonstrate the [usage](#usage) of rspl:
//!
//! ```
//! use rspl::streams::overeager_receivers::OvereagerReceiver;
//! use rspl::streams::Stream;
//! use rspl::StreamProcessor;
//!
//! #[derive(Copy, Clone)]
//! enum Event {
//!     Event1,
//!     Event2,
//! }
//!
//! struct Initial<'a, A, B> {
//!     state: StreamProcessor<'a, A, B>,
//!     event: Event,
//! }
//!
//! fn action() -> bool {
//!     true
//! }
//!
//! fn state_1<'a>() -> StreamProcessor<'a, Event, bool> {
//!     fn transition<'a>(event: Event) -> StreamProcessor<'a, Event, bool> {
//!         match event {
//!             Event::Event1 => StreamProcessor::Put(action(), Box::new(state_1)),
//!             Event::Event2 => state_2(),
//!         }
//!     }
//!
//!     StreamProcessor::Get(Box::new(transition))
//! }
//!
//! fn state_2<'a>() -> StreamProcessor<'a, Event, bool> {
//!     fn transition<'a>(event: Event) -> StreamProcessor<'a, Event, bool> {
//!         match event {
//!             Event::Event1 => state_1(),
//!             Event::Event2 => StreamProcessor::Put(false, Box::new(state_2)),
//!         }
//!     }
//!
//!     StreamProcessor::Get(Box::new(transition))
//! }
//!
//! let initial = Initial {
//!     state: state_1(),
//!     event: Event::Event1,
//! };
//!
//! let (tevents, events) = OvereagerReceiver::channel(0, initial.event);
//!
//! tevents.send(Event::Event2).unwrap();
//!
//! let event_loop_body = initial.state.eval(events);
//!
//! assert!(event_loop_body.head());
//! ```

pub mod streams;

use streams::infinite_lists::InfiniteList;
use streams::Stream;

/// [`Lazy<T>`] types thunks of type `T`.
type Lazy<'a, T> = dyn FnOnce() -> T + 'a;

/// [`StreamProcessor<A, B>`] defines (the syntax of) a language describing the domain of stream processors, that is, terms which can be interpreted to turn streams of type `A` into streams of type `B`.
pub enum StreamProcessor<'a, A: 'a, B> {
    /// This stream processor first reads the `A` from the head of the input stream. Then it applies the its function argument to it yielding a stream processor. This stream processor is then applied to the tail of the input stream.
    Get(Box<dyn FnOnce(A) -> StreamProcessor<'a, A, B> + 'a>),
    /// This stream processor writes the `B` from its first argument to the output list and use its second argument to process the input stream to generate the rest of the output list if needed.
    Put(B, Box<Lazy<'a, StreamProcessor<'a, A, B>>>),
}

impl<'a, A, B> StreamProcessor<'a, A, B> {
    /// Evaluate `self` on an input stream essentially implementing a semantic of [`StreamProcessor<A, B>`].
    /// - `stream` is the input stream.
    ///
    /// Note that the function can block the current thread if the respective implementation of [`Stream::tail`] can.
    ///
    /// # Panics
    ///
    /// A panic may occur if
    /// - the stream processor contains rust-terms which can panic.
    /// - the respective implementation of [`Stream::head`] or [`Stream::tail`] can panic.
    ///
    /// # Examples
    ///
    /// Negating a stream of `true`s to obtain a stream of `false`s:
    ///
    /// ```
    /// use rspl::StreamProcessor;
    ///
    /// fn negate<'a>() -> StreamProcessor<'a, bool, bool> {
    ///     StreamProcessor::Get(Box::new(|b: bool| {
    ///         StreamProcessor::Put(!b, Box::new(|| negate()))
    ///     }))
    /// }
    ///
    /// let trues = rspl::streams::infinite_lists::InfiniteList::constant(true);
    ///
    /// negate().eval(trues);
    /// ```
    pub fn eval<S: Stream<A> + 'a>(self, stream: S) -> InfiniteList<'a, B>
    where
        A: Copy,
    {
        match self {
            StreamProcessor::Get(f) => Self::eval(f(stream.head()), stream.tail()),
            StreamProcessor::Put(b, lazy_sp) => {
                InfiniteList::Cons(b, Box::new(|| Self::eval(lazy_sp(), stream)))
            }
        }
    }
}

/// Construct the stream processor which applies a given closure to each element of the input stream.
/// - `f` is the closure to be applied.
///
/// The function is in analogy to the map-function on lists which is well-known in functional programming.
///
/// # Examples
///
/// Negating a stream of `true`s to obtain a stream of `false`s:
///
/// ```
/// use rspl::{map, StreamProcessor};
///
/// fn negate(b: bool) -> bool {
///     !b
/// }
///
/// let trues = rspl::streams::infinite_lists::InfiniteList::constant(true);
///
/// map(negate).eval(trues);
/// ```
pub fn map<'a, A, B, F>(f: F) -> StreamProcessor<'a, A, B>
where
    F: Fn(A) -> B + 'a,
{
    StreamProcessor::Get(Box::new(|a: A| {
        StreamProcessor::Put(f(a), Box::new(|| map(f)))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use streams::infinite_lists::InfiniteList;
    use streams::overeager_receivers::OvereagerReceiver;

    const fn id<X>(x: X) -> X {
        x
    }

    #[test]
    fn test_eval() {
        const N: usize = 2;

        let sp = StreamProcessor::Get(Box::new(|n: usize| {
            if n % 2 == 0 {
                StreamProcessor::Put(
                    n + N,
                    Box::new(move || StreamProcessor::Put(n, Box::new(|| map(id)))),
                )
            } else {
                StreamProcessor::Put(
                    n - N,
                    Box::new(move || StreamProcessor::Put(n, Box::new(|| map(id)))),
                )
            }
        }));
        let ns = InfiniteList::constant(N);
        let stream = sp.eval(ns);

        let stream_first = stream.head();
        assert_eq!(stream_first, N + N);

        let stream_second = stream.tail().head();
        assert_eq!(stream_second, N);
    }

    #[test]
    #[should_panic]
    fn test_eval_panic() {
        let sp = StreamProcessor::Get(Box::new(|b: bool| {
            StreamProcessor::Put(if b { panic!() } else { b }, Box::new(|| map(id)))
        }));
        let trues = InfiniteList::constant(true);
        sp.eval(trues);
    }

    #[test]
    fn test_map() {
        let sp = map(|n: usize| n + 1);

        let (tx, stream) = OvereagerReceiver::channel(0, 0);
        tx.send(1).unwrap();
        tx.send(10).unwrap();

        let result = sp.eval(stream);

        let one = result.head();
        assert_eq!(one, 1);

        let two = result.tail().head();
        assert_eq!(two, 2);
    }
}

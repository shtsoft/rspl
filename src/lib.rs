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
//! Encoding and (abstract) lazy event loop as stream processor:
//!
//! ```
//! use rspl::StreamProcessor;
//!
//! #[derive(Copy)]
//! #[derive(Clone)]
//! enum Event {
//!     Event,
//! }
//!
//! enum State {
//!     Hello,
//!     World,
//! }
//!
//! fn print_and_flip(_: Event, state: State) -> (Box<dyn FnOnce()>, State) {
//!     match state {
//!         State::Hello => (Box::new(|| println!("Hello")), State::World),
//!         State::World => (Box::new(|| println!("World")), State::Hello),
//!     }
//! }
//!
//! fn event_processor<'a>(state: State) -> StreamProcessor<'a, Event, ()> {
//!     StreamProcessor::Get(Box::new(move |event: Event| {
//!         let (action, new_state) = print_and_flip(event, state);
//!         StreamProcessor::Put(action(), Box::new(move || event_processor(new_state)))
//!     }))
//! }
//!
//! let events = rspl::streams::infinite_lists::InfiniteList::constant(Event::Event);
//!
//! let initial_state = State::Hello;
//!
//! let _lazy_event_loop = event_processor(initial_state).eval(events);
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
    /// # Panics
    ///
    /// A panic may occur if the stream processor contains rust-terms which can panic like e.g. function-calls which may panic.
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

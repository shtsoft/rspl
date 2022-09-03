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
//! To program a rspl-[`StreamProcessor`] you just have to compose the constructors [`StreamProcessor::Get`]/[`get`](`StreamProcessor::get`) and [`StreamProcessor::Put`]/[`put`](`StreamProcessor::put`) in the right way.
//! For a somewhat more high-level programming experience you might wish to look at the [`combinators`]-module.
//! The program can then be evaluated with [`eval`](`StreamProcessor::eval`)-method on some kind of input stream.
//! The 'kind' of input stream is either your own implementation of the [`Stream`]-interface or one
//! from the submodules of the [`streams`]-module.
//! Either way, as result, evaluation produces an [`InfiniteList`] (lazily).
//! To observe streams - and i.p. infinite lists - you can destruct them with [`head`](`Stream::head`)- and [`tail`](`Stream::tail`)-methods of the stream interface.
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
//!             Event::Event1 => StreamProcessor::put(action(), state_1()),
//!             Event::Event2 => state_2(),
//!         }
//!     }
//!
//!     StreamProcessor::get(transition)
//! }
//!
//! fn state_2<'a>() -> StreamProcessor<'a, Event, bool> {
//!     fn transition<'a>(event: Event) -> StreamProcessor<'a, Event, bool> {
//!         match event {
//!             Event::Event1 => state_1(),
//!             Event::Event2 => StreamProcessor::put(false, state_2()),
//!         }
//!     }
//!
//!     StreamProcessor::get(transition)
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
    /// The same as [`StreamProcessor::Get`] but with boxing of `f` hidden to make the resulting code less verbose.
    #[inline]
    pub fn get<F>(f: F) -> Self
    where
        F: FnOnce(A) -> Self + 'a,
    {
        StreamProcessor::Get(Box::new(f))
    }

    /// The same as [`StreamProcessor::Put`] but with thunking and boxing of `sp` hidden to make the resulting code less verbose.
    #[inline]
    pub fn put(b: B, sp: Self) -> Self
    where
        B: 'a,
    {
        StreamProcessor::Put(b, Box::new(|| sp))
    }
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
    ///     StreamProcessor::get(|b: bool| StreamProcessor::put(!b, negate()))
    /// }
    ///
    /// let trues = rspl::streams::infinite_lists::InfiniteList::constant(true);
    ///
    /// negate().eval(trues);
    /// ```
    pub fn eval<S: Stream<A> + 'a>(self, stream: S) -> InfiniteList<'a, B>
    where
        A: Clone,
    {
        match self {
            StreamProcessor::Get(f) => Self::eval(f(stream.head().clone()), stream.tail()),
            StreamProcessor::Put(b, lazy_sp) => {
                InfiniteList::Cons(b, Box::new(|| Self::eval(lazy_sp(), stream)))
            }
        }
    }
}

pub mod combinators {
    //! This module defines (parameterized) functions which combine existing stream processors into new ones.
    //! In particular, there are nullary combinators to get writing a stream processor off the ground.

    use super::StreamProcessor;

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
    /// use rspl::combinators::map;
    /// use rspl::StreamProcessor;
    ///
    /// let negate = |b: bool| !b;
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

    /// Construct the stream processor which filters the input stream according to a given predicate.
    /// - `p` is the predicate serving as filter.
    ///
    /// The function is in analogy to the filter-function on lists which is well-known in functional programming.
    ///
    /// # Examples
    ///
    /// Remove the `true`s from a stream of bools:
    ///
    /// ```
    /// use rspl::combinators::filter;
    /// use rspl::StreamProcessor;
    ///
    /// let is_false = |b: &bool| !b;
    ///
    /// let trues = rspl::streams::infinite_lists::InfiniteList::constant(false);
    ///
    /// filter(is_false).eval(trues);
    /// ```
    pub fn filter<'a, A, P>(p: P) -> StreamProcessor<'a, A, A>
    where
        P: Fn(&A) -> bool + 'a,
    {
        StreamProcessor::Get(Box::new(|a: A| {
            if p(&a) {
                StreamProcessor::Put(a, Box::new(|| filter(p)))
            } else {
                filter(p)
            }
        }))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::streams::overeager_receivers::OvereagerReceiver;
        use crate::streams::Stream;

        #[test]
        fn test_map() {
            let sp = map(|n: usize| n + 1);

            let (tx, stream) = OvereagerReceiver::channel(0, 0);
            tx.send(1).unwrap();
            tx.send(10).unwrap();

            let result = sp.eval(stream);
            assert_eq!(*result.head(), 1);

            let result_tail = result.tail();
            assert_eq!(*result_tail.head(), 2);
        }

        #[test]
        fn test_filter() {
            let is_greater_zero = |n: &usize| *n > 0;

            let sp = filter(is_greater_zero);

            let (tx, stream) = OvereagerReceiver::channel(0, 0);
            tx.send(1).unwrap();
            tx.send(0).unwrap();
            tx.send(2).unwrap();
            tx.send(10).unwrap();

            let result = sp.eval(stream);
            assert_eq!(*result.head(), 1);

            let result_tail = result.tail();
            assert_eq!(*result_tail.head(), 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use combinators::map;
    use streams::overeager_receivers::OvereagerReceiver;

    const fn id<X>(x: X) -> X {
        x
    }

    #[test]
    fn test_eval() {
        const N: usize = 2;

        let sp = StreamProcessor::get(|n: usize| {
            if n % 2 == 0 {
                StreamProcessor::put(n + N, StreamProcessor::put(n, map(id)))
            } else {
                StreamProcessor::put(n - N, StreamProcessor::put(n, map(id)))
            }
        });

        let (tx, stream) = OvereagerReceiver::channel(N, N);
        tx.send(N).unwrap();
        tx.send(0).unwrap();

        let result = sp.eval(stream);
        assert_eq!(*result.head(), N + N);

        let result_tail = result.tail();
        assert_eq!(*result_tail.head(), N);
    }

    #[test]
    #[should_panic]
    fn test_eval_panic() {
        let sp = StreamProcessor::get(|b: bool| {
            StreamProcessor::put(if b { panic!() } else { b }, map(id))
        });
        let trues = InfiniteList::constant(true);
        sp.eval(trues);
    }
}

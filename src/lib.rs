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
//! To program a rspl-[`StreamProcessor`] you can just combine existing ones (perhaps obtained using [`map`]) by using the stream processor constructors [`StreamProcessor::Get`] and [`StreamProcessor::Put`].
//! The program can then be evaluated with [`eval`] on some kind of input stream [`Stream`].
//! For the type of the input stream you can either use [`InfiniteList`] or implement the [`Stream`]-interface yourself e.g. as some kind of queue.
//! For the former there are some pre-defined stream constructors like [`InfiniteList::constant`]-constructor.
//! As result, [`eval`] produces an [`InfiniteList`].
//! To observe this infinite list you can destruct it yourself with [`Stream::head`] and [`Stream::tail`] or use methods like the [`InfiniteList::print`]-method.
//!
//! # Examples
//!
//! Encoding and (abstract) lazy event loop as stream processor:
//!
//! ```
//! use rspl::{eval, StreamProcessor};
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
//! fn event_processor(state: State) -> StreamProcessor<Event, ()> {
//!     StreamProcessor::Get(Box::new(move |event: Event| {
//!         let (action, new_state) = print_and_flip(event, state);
//!         StreamProcessor::Put(action(), Box::new(move || event_processor(new_state)))
//!     }))
//! }
//!
//! let events = rspl::InfiniteList::constant(Event::Event);
//!
//! let initial_state = State::Hello;
//!
//! let _lazy_event_loop = eval(event_processor(initial_state), events);
//! ```

mod streams {
    //! This module defines streams of some type extensionally as a trait and provides the standard implementation of this interface as infinite lists (the greatest fixpoint of `cons`ing).
    //! Moreover it contains implementations to destruct and construct such infinite lists.

    /// A characterization of streams of some type `X`: a stream of `X` is an object from which one can observe something of type `X` (the head of the stream) or another stream of `X` (the tail of the stream).
    pub trait Stream<X>
    where
        X: Copy,
    {
        /// Copy the first item of `self` and return that copy.
        fn head(&self) -> X;
        /// Throw away the first item of `self` and return what is left.
        fn tail(self) -> Self;
    }

    /// [`Lazy<T>`] types thunks of type `T`.
    type Lazy<T> = dyn FnOnce() -> T;

    /// [`InfiniteList<X>`] defines non-well-founded list of type `X`.
    pub enum InfiniteList<X> {
        /// Constructing a new infinite list by prepending a new entry to an existing (lazy) inifinite list.
        Cons(X, Box<Lazy<InfiniteList<X>>>),
    }

    impl<X> Stream<X> for InfiniteList<X>
    where
        X: Copy,
    {
        /// Make the first list enrty of `self` the head.
        fn head(&self) -> X {
            match self {
                InfiniteList::Cons(head, _) => *head,
            }
        }

        /// Make all but the first list entry of `self` the tail.
        fn tail(self) -> Self {
            match self {
                InfiniteList::Cons(_, tail) => tail(),
            }
        }
    }

    impl<X> InfiniteList<X> {
        /// Print a specified number of entries of `self`.
        /// - `n` is the number of entries to be printed.
        ///
        /// # Examples
        ///
        /// Printing the first five elements of the infinite list `0, 1, 2, ..., |usize|, 0, 1, ...`:
        ///
        /// ```
        /// use rspl::InfiniteList;
        ///
        /// fn ascending(n: usize) -> InfiniteList<usize> {
        ///     InfiniteList::Cons(n, Box::new(move || ascending(n + 1)))
        /// }
        ///
        /// ascending(0).print(100);
        /// ```
        pub fn print(mut self, n: usize) -> Self
        where
            X: std::fmt::Display + Copy + 'static,
        {
            for _ in 0..n {
                println!("{}", self.head());
                self = self.tail();
            }

            self
        }
    }

    impl<X> InfiniteList<X> {
        /// Create an infinte list of a certain constant.
        /// - `x` is the constant.
        ///
        /// # Examples
        ///
        /// Creating an infinite list of `true`s:
        ///
        /// ```
        /// let trues = rspl::InfiniteList::constant(true);
        /// ```
        pub fn constant(x: X) -> InfiniteList<X>
        where
            X: Copy + 'static,
        {
            InfiniteList::Cons(x, Box::new(move || InfiniteList::constant(x)))
        }
    }
}

pub use streams::{InfiniteList, Stream};

/// [`Lazy<T>`] types thunks of type `T`.
type Lazy<T> = dyn FnOnce() -> T;

/// [`StreamProcessor<A, B>`] defines (the syntax of) a language describing the domain of stream processors, that is, terms which can be interpreted to turn streams of type `A` into streams of type `B`.
pub enum StreamProcessor<A, B> {
    /// Read the head `a` of the input stream and use `f(a)` to process the tail of the input stream.
    Get(Box<dyn FnOnce(A) -> StreamProcessor<A, B>>),
    /// Write `b` to the output list and use the `lazy_stream_processor` to process the input stream if needed.
    Put(B, Box<Lazy<StreamProcessor<A, B>>>),
}

/// Evaluate a stream processor on an input stream essentially implementing a semantic of [`StreamProcessor<A, B>`].
/// - `sp` is the stream processor (program).
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
/// use rspl::{eval, StreamProcessor};
///
/// fn negate() -> StreamProcessor<bool, bool> {
///     StreamProcessor::Get(Box::new(move |b: bool| {
///         StreamProcessor::Put(!b, Box::new(move || negate()))
///     }))
/// }
///
/// let trues = rspl::InfiniteList::constant(true);
///
/// eval(negate(), trues);
/// ```
pub fn eval<A, B, S>(sp: StreamProcessor<A, B>, stream: S) -> InfiniteList<B>
where
    A: Copy + 'static,
    B: 'static,
    S: Stream<A> + 'static,
{
    match sp {
        StreamProcessor::Get(f) => eval(f(stream.head()), stream.tail()),
        StreamProcessor::Put(b, lazy_sp) => {
            InfiniteList::Cons(b, Box::new(move || eval(lazy_sp(), stream)))
        }
    }
}

/// Construct the stream processor which applies a given function to each piece of the input stream.
/// - `f` is the function to be applied.
///
/// The function is in analogy to the map-function on lists which is well-known in functional programming.
///
/// # Examples
///
/// Negating a stream of `true`s to obtain a stream of `false`s:
///
/// ```
/// use rspl::{eval, map, StreamProcessor};
///
/// fn negate(b: bool) -> bool {
///     !b
/// }
///
/// let trues = rspl::InfiniteList::constant(true);
///
/// eval(map(negate), trues);
/// ```
pub fn map<A, B>(f: fn(A) -> B) -> StreamProcessor<A, B>
where
    A: 'static,
    B: 'static,
{
    StreamProcessor::Get(Box::new(move |a: A| {
        StreamProcessor::Put(f(a), Box::new(move || map(f)))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn negate(b: bool) -> bool {
        !b
    }

    fn times_two(n: usize) -> usize {
        n * 2
    }

    fn ascending(n: usize) -> InfiniteList<usize> {
        InfiniteList::Cons(n, Box::new(move || ascending(n + 1)))
    }

    #[test]
    fn it_works() {
        let result = eval(map(negate), eval(map(negate), InfiniteList::constant(true)));
        result.print(10);
        //assert_eq!(result, ...);

        let result = eval(map(times_two), ascending(0));
        result.print(10);
        //assert_eq!(result, ...);
    }
}

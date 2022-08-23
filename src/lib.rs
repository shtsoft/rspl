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
//! To observe this infinite list you can destruct it yourself with [`Stream::head`] and [`Stream::tail`] or use functions like the [`streams::print`]-method.
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

mod streams;
pub use streams::*;

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

    const fn id<X>(x: X) -> X {
        x
    }

    const fn successor(n: usize) -> usize {
        n + 1
    }

    fn ascending(n: usize) -> InfiniteList<usize> {
        InfiniteList::Cons(n, Box::new(move || ascending(n + 1)))
    }

    #[test]
    fn test_eval() {
        const N: usize = 2;

        let sp = StreamProcessor::Get(Box::new(move |n: usize| {
            if n % 2 == 0 {
                StreamProcessor::Put(
                    n + N,
                    Box::new(move || StreamProcessor::Put(n, Box::new(move || map(id)))),
                )
            } else {
                StreamProcessor::Put(
                    n - N,
                    Box::new(move || StreamProcessor::Put(n, Box::new(move || map(id)))),
                )
            }
        }));
        let ns = InfiniteList::constant(N);
        let stream = eval(sp, ns);

        let stream_first = stream.head();
        assert_eq!(stream_first, N + N);

        let stream_second = stream.tail().head();
        assert_eq!(stream_second, N);
    }

    #[test]
    #[should_panic]
    fn test_eval_panic() {
        let sp = StreamProcessor::Get(Box::new(move |b: bool| {
            StreamProcessor::Put(if b { panic!() } else { b }, Box::new(move || map(id)))
        }));
        let trues = InfiniteList::constant(true);
        eval(sp, trues);
    }

    #[test]
    fn test_map() {
        let sp = map(successor);
        let usizes = ascending(0);
        let usizes_plus_one = eval(sp, usizes);

        let one = usizes_plus_one.head();
        assert_eq!(one, 1);

        let usizes_plus_two = usizes_plus_one.tail();
        let two = usizes_plus_two.head();
        assert_eq!(two, 2);
    }
}

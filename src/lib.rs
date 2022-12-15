//! rspl is a stream processor language based on the one discussed in [Generalising Monads to Arrows](https://www.sciencedirect.com/science/article/pii/S0167642399000234) using rust as meta-language.
//!
//! ## Design
//!
//! Essentially, rspl is a way to encode functions from streams to streams such that control is syntactically explicit (like in ordinary continuation-passing style) refining the orthodox functional approach to stream processing with combinators like 'map'.
//! More precisely, the idea of this stream processor language is to split the processing of streams into two parts:
//! One part for reading (getting) the first element of an input stream to direct the further processing.
//! Another part for writing (putting) something to the output stream and offering to process some input stream if needed.
//! Combining these parts in various ways allows to flexibly construct stream processors as programs.
//! The following graphic illustrates how those two different kinds of stream processors ('getting' and 'putting') work (whereas a textual description is contained in the docs of [`StreamProcessor`]):
//!
//! <pre>
//! h--t1--t2--t3--...                   ha--t1--t2--t3--...
//! -                                    -
//! |                                    |
//! | Get(h |-> [SP](h))                 | Put(hb, LAZY-[SP])
//! |                                    |
//! v                                    |
//! t1--t2--t3--...                      |   t1--t2--t3--...
//! -                                    |   -
//! |                                    v   |
//! | [SP](h) = Get(_)                   hb--| LAZY-[SP]() = Get(_)
//! |                                        |
//! v                                        v
//! ...                                      ...
//!
//!
//! h--t1--t2--t3--...                   ha--t1--t2--t3--...
//! -                                    -
//! |                                    |
//! | Get(h |-> [SP](h))                 | Put(hb, LAZY-[SP])
//! |                                    |
//! v                                    |
//! h--t1--t2--t3--...                   |   ha--t1--t2--t3--...
//! -                                    |   -
//! |                                    v   |
//! | [SP](h) = Put(_, _)                hb--| LAZY-[SP]() = Put(_, _)
//! |                                        |
//! v                                        v
//! ...                                      ...
//! </pre>
//!
//! Remarkably, the language constructs are somewhat dual and loosely correspond to (dual) programming paradigms:
//! - The `Get`-construct relates to event-driven programming as it reacts to input (events) eagerly.
//! - The `Put`-construct relates to demand-driven[^1] programming as it generates output (demands) iteratively by need.
//!
//! This will be discussed further in the [Examples](#examples)-section.
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
//! As alluded in the [Design](#design)-section, rspl supports orthodox 'combination-driven' stream processing as it is known from list processing with combinators like [`compose`](`combinators::compose`), [`filter`](`combinators::filter`) and [`map`](`combinators::map`).
//! For example, it is possible to first filter some 'bad' elements out of a stream in order to safely iterate some function over the resulting stream afterwards in a combinatorial way.
//! Such a [usage](#usage) of rspl looks like:
//!
//! ```
//! use rspl::combinators::{compose, filter, map};
//! use rspl::streams::infinite_lists::InfiniteList;
//! use rspl::streams::Stream;
//! use rspl::StreamProcessor;
//!
//! let is_greater_zero = |n: &usize| *n > 0;
//! let minus_one = |n: usize| n - 1;
//!
//! let zeroes = compose(filter(is_greater_zero), map(minus_one))
//!     .eval(InfiniteList::cons(0, || InfiniteList::constant(1)));
//!
//! assert_eq!(*zeroes.head(), 0);
//! ```
//!
//! More interestingly, rspl can also serve as a framework for the nifty idea of
//! - event-driven programming with state machines as suggested [here](https://barrgroup.com/Embedded-Systems/How-To/State-Machines-Event-Driven-Systems).
//!   Abstractly, that [usage](#usage) of rspl looks as follows:
//!
//!   ```
//!   use rspl::streams::infinite_lists::InfiniteList;
//!   use rspl::streams::Stream;
//!   use rspl::StreamProcessor;
//!
//!   #[derive(Copy, Clone)]
//!   enum Event {
//!       Event1,
//!       Event2,
//!   }
//!
//!   fn action() -> bool {
//!       true
//!   }
//!
//!   fn state_1<'a>() -> StreamProcessor<'a, Event, bool> {
//!       fn transition<'a>(event: Event) -> StreamProcessor<'a, Event, bool> {
//!           match event {
//!               Event::Event1 => StreamProcessor::put(action(), state_1),
//!               Event::Event2 => state_2(),
//!           }
//!       }
//!
//!       StreamProcessor::get(transition)
//!   }
//!
//!   fn state_2<'a>() -> StreamProcessor<'a, Event, bool> {
//!       fn transition<'a>(event: Event) -> StreamProcessor<'a, Event, bool> {
//!           match event {
//!               Event::Event1 => state_1(),
//!               Event::Event2 => StreamProcessor::put(false, state_2),
//!           }
//!       }
//!
//!       StreamProcessor::get(transition)
//!   }
//!
//!   let event_loop = state_2().eval(InfiniteList::constant(Event::Event1));
//!
//!   assert!(event_loop.head());
//!   ```
//!
//!   A slightly more concrete example using that pattern is available as [integration test](https://github.com/aronpaulson/rspl/blob/master/tests/events.rs).
//!   And a full-blown concrete example of a pelican crossing can be found [here (as .md file)](https://github.com/aronpaulson/rspl/blob/master/examples/pelican.md) and [here (as .rs file)](https://github.com/aronpaulson/rspl/blob/master/examples/hics.rs).
//!   Notably, it uses rspl to encode effectful hierarchical state machines with a capability-passing inspired effect-handling mechanism.
//! - demand-driven programming with generators as suggested [here](https://www.cse.chalmers.se/~rjmh/Papers/whyfp.pdf).
//!   Abstractly, that [usage](#usage) of rspl looks as follows:
//!
//!   ```
//!   use rspl::streams::infinite_lists::InfiniteList;
//!   use rspl::streams::Stream;
//!   use rspl::StreamProcessor;
//!
//!   struct State {
//!       toggle: bool,
//!   }
//!
//!   fn action(state: &mut State) {
//!       state.toggle = !state.toggle;
//!   }
//!
//!   fn pre_action(state: State) -> State {
//!       state
//!   }
//!
//!   fn post_action(state: State) -> State {
//!       state
//!   }
//!
//!   fn generator_name<'a>(mut state: State) -> StreamProcessor<'a, (), bool> {
//!       state = pre_action(state);
//!       StreamProcessor::get(|_| {
//!           action(&mut state);
//!           StreamProcessor::put(state.toggle, || generator_name(post_action(state)))
//!       })
//!   }
//!
//!   let generations = generator_name(State { toggle: false }).eval(InfiniteList::constant(()));
//!
//!   assert!(generations.head());
//!   ```
//!
//!   A slightly more concrete example using that pattern is available as [integration test](https://github.com/aronpaulson/rspl/blob/master/tests/demands.rs).
//!   And a full-blown concrete example of a heat index control system can be found [here (as .md file)](https://github.com/aronpaulson/rspl/blob/master/examples/hics.md) and [here (as .rs file)](https://github.com/aronpaulson/rspl/blob/master/examples/hics.rs).
//!
//! [^1]: Look at [Codata in Action](https://www.microsoft.com/en-us/research/uploads/prod/2020/01/CoDataInAction.pdf) for some more explanation on that term.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod combinators;

pub mod streams;

use streams::infinite_lists::InfiniteList;
use streams::Stream;

use alloc::boxed::Box;

/// [`Lazy<T>`] types thunks of type `T`.
type Lazy<'a, T> = dyn FnOnce() -> T + 'a;

/// [`StreamProcessor<A, B>`] defines (the syntax of) a language describing the domain of stream processors, that is, terms which can be interpreted to turn streams of type `A` into streams of type `B`.
pub enum StreamProcessor<'a, A: 'a, B> {
    /// This stream processor first reads the `A` from the head of the input stream to subsequently apply its function argument to that element yielding a stream processor.
    /// The resulting stream processor is then used to process the input stream further depending on its shape: if it is a
    /// - [`Get`](`StreamProcessor::Get`) it is applied to the tail of the input stream.
    /// - [`Put`](`StreamProcessor::Put`) it is applied to the whole input stream.
    Get(Box<dyn FnOnce(A) -> StreamProcessor<'a, A, B> + 'a>),
    /// This stream processor writes the `B` from its first argument to the output list.
    /// Then to construct the rest of the output list it uses its second argument to process the input stream depending on its shape: if it is a
    /// - [`Get`](`StreamProcessor::Get`) it is applied to the tail of the input stream.
    /// - [`Put`](`StreamProcessor::Put`) it is applied to the whole input stream.
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

    /// The same as [`StreamProcessor::Put`] but with boxing of `lazy_sp` hidden to make the resulting code less verbose.
    #[inline]
    pub fn put<T>(b: B, lazy_sp: T) -> Self
    where
        B: 'a,
        T: FnOnce() -> Self + 'a,
    {
        StreamProcessor::Put(b, Box::new(lazy_sp))
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
    ///     StreamProcessor::get(|b: bool| StreamProcessor::put(!b, negate))
    /// }
    ///
    /// let trues = rspl::streams::infinite_lists::InfiniteList::constant(true);
    ///
    /// negate().eval(trues);
    /// ```
    pub fn eval<S: Stream<A> + 'a>(mut self, mut stream: S) -> InfiniteList<'a, B>
    where
        A: Clone,
    {
        // This implementation deviates from the original for two reasons:
        // - rust does not guarantee tail-recursion elimination and rspl wants to prevent
        //   stack-overflows as much as possible. Therefore the loop in lieu of recursion.
        // - There are streams rspl programs can operate on where taking the tail can block as
        //   opposed to the original implementation. So the question arising here is when to take
        //   the tail of the input. The answer is, as late as possible, that is, only if the next
        //   step is 'getting'. Because then 'putting' is not hindered. And this is as it should be
        //   if taking rspl's idea of seperating input processing from output processing serious.
        loop {
            match self {
                StreamProcessor::Get(f) => {
                    self = f(stream.head().clone());
                    while let StreamProcessor::Get(f) = self {
                        stream = stream.tail();
                        self = f(stream.head().clone());
                    }
                    continue;
                }
                StreamProcessor::Put(b, lazy_sp) => {
                    return InfiniteList::Cons(
                        b,
                        Box::new(|| {
                            let sp = lazy_sp();
                            if let StreamProcessor::Get(_) = sp {
                                stream = stream.tail();
                            }
                            Self::eval(sp, stream)
                        }),
                    )
                }
            }
        }
    }
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    use super::*;
    use combinators::map;
    use streams::overeager_receivers::OvereagerReceiver;

    use crate::assert_head_eq;
    use crate::assert_tail_starts_with;
    use crate::enqueue;

    const fn id<X>(x: X) -> X {
        x
    }

    #[test]
    fn test_get() {
        assert!(matches!(
            StreamProcessor::get(|_: ()| { map(id) }),
            StreamProcessor::Get(_)
        ));
    }

    #[test]
    fn test_put() {
        assert!(matches!(
            StreamProcessor::put((), || map(id)),
            StreamProcessor::Put(_, _)
        ));
    }

    #[test]
    fn test_eval() {
        let sp = StreamProcessor::get(|n: usize| {
            StreamProcessor::put(n, || {
                StreamProcessor::get(|n1: usize| {
                    StreamProcessor::get(move |n2: usize| {
                        if n1 < n2 {
                            StreamProcessor::put(n2, move || StreamProcessor::put(n1, || map(id)))
                        } else {
                            StreamProcessor::put(n1, move || StreamProcessor::put(n2, || map(id)))
                        }
                    })
                })
            })
        });

        let (tx, stream) = OvereagerReceiver::channel(0, 0);
        enqueue!(tx, [1, 2]);

        let mut result = sp.eval(stream);
        assert_head_eq!(result, 0);
        assert_tail_starts_with!(result, [2, 1]);
    }

    #[test]
    #[should_panic]
    fn test_eval_panic() {
        let sp = StreamProcessor::get(|b: bool| {
            StreamProcessor::put(if b { panic!() } else { b }, || map(id))
        });

        let trues = InfiniteList::constant(true);

        sp.eval(trues);
    }
}

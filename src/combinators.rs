//! This module defines functions which combine existing stream processors into new ones.
//! In particular, there are nullary combinators to get writing a stream processor off the ground.

use super::StreamProcessor;

/// The function combines two stream processors into one alternating between the two whenever something is written to the ouput stream.
/// - `sp1` is the stream processor which is in control.
/// - `sp2` is the stream processor to which control is transferred.
///
/// This function is in analogy to running coroutines as it runs its arguments concurrently on the
/// input stream.
///
/// # Examples
///
/// Negate a stream of bools in any other position:
///
/// ```
/// use rspl::combinators::{alternate, map};
/// use rspl::streams::infinite_lists::InfiniteList;
/// use rspl::StreamProcessor;
///
/// let id = |b: bool| b;
/// let negate = |b: bool| !b;
///
/// let trues = InfiniteList::constant(true);
///
/// alternate(map(id), map(negate)).eval(trues);
/// ```
pub fn alternate<'a, A, B: 'a>(
    sp1: StreamProcessor<'a, A, B>,
    sp2: StreamProcessor<'a, A, B>,
) -> StreamProcessor<'a, A, B> {
    match sp1 {
        StreamProcessor::Get(f) => StreamProcessor::Get(Box::new(|a| alternate(f(a), sp2))),
        StreamProcessor::Put(b, lazy_sp) => {
            StreamProcessor::Put(b, Box::new(|| alternate(sp2, lazy_sp())))
        }
    }
}

/// The function combines a stream processor and a family of them into one processing with the given one until an element would be written using that element to choose a stream processor from the family to carry on processing instead of writing it to the output stream.
/// - `sp` is the stream processor.
/// - `f` is the family of stream processors.
///
/// This function is in analogy to the bind operation of monads (though we do **not** claim that it is the bind operation of an actual monad `StreamProcessor<X, _>`).
///
/// # Examples
///
/// Flip the signs in the tail of a stream of integers depending on the head of the stream:
///
/// ```
/// use rspl::combinators::{bind, map};
/// use rspl::streams::infinite_lists::InfiniteList;
/// use rspl::StreamProcessor;
///
/// let is_zero = |n: isize| n == 0;
/// let maybe_flip_sign = |b: bool| if b { map(|n: isize| -n) } else { map(|n: isize| n) };
///
/// let ones = InfiniteList::constant(1);
///
/// bind(map(is_zero), maybe_flip_sign).eval(ones);
/// ```
pub fn bind<'a, X, A: 'a, B, F>(sp: StreamProcessor<'a, X, A>, f: F) -> StreamProcessor<'a, X, B>
where
    F: FnOnce(A) -> StreamProcessor<'a, X, B> + 'a,
{
    match sp {
        StreamProcessor::Get(g) => StreamProcessor::Get(Box::new(|a| bind(g(a), f))),
        StreamProcessor::Put(b, _) => f(b),
    }
}

/// The function combines two stream processors into one applying the second to the result of the first.
/// - `sp1` is the stream processor applied first.
/// - `sp2` is the stream processor applied second.
///
/// This function is in analogy to ordinary function composition.
/// More generally, it is the composition operation in a category with stream processors as morphisms.
///
/// # Examples
///
/// Double-negate a stream of bools:
///
/// ```
/// use rspl::combinators::{compose, map};
/// use rspl::streams::infinite_lists::InfiniteList;
/// use rspl::StreamProcessor;
///
/// let negate = |b: bool| !b;
///
/// let trues = InfiniteList::constant(true);
///
/// compose(map(negate), map(negate)).eval(trues);
/// ```
pub fn compose<'a, A, B, C: 'a>(
    mut sp1: StreamProcessor<'a, A, B>,
    mut sp2: StreamProcessor<'a, B, C>,
) -> StreamProcessor<'a, A, C> {
    loop {
        match sp1 {
            StreamProcessor::Get(f) => {
                return StreamProcessor::Get(Box::new(|a| compose(f(a), sp2)))
            }
            StreamProcessor::Put(b, lazy_sp1) => match sp2 {
                StreamProcessor::Get(f) => {
                    sp1 = lazy_sp1();
                    sp2 = f(b);
                    continue;
                }
                StreamProcessor::Put(c, lazy_sp2) => {
                    return StreamProcessor::Put(
                        c,
                        Box::new(|| compose(StreamProcessor::Put(b, lazy_sp1), lazy_sp2())),
                    )
                }
            },
        }
    }
}

/// Construct the stream processor which filters the input stream according to a given predicate.
/// - `p` is the predicate serving as filter.
///
/// The function is in analogy to the filter-function on lists which is well-known in functional programming.
///
/// # Examples
///
/// Remove the `0`s from a stream of integers:
///
/// ```
/// use rspl::combinators::filter;
/// use rspl::streams::infinite_lists::InfiniteList;
/// use rspl::StreamProcessor;
///
/// let is_greater_zero = |n: &usize| *n > 0;
///
/// let ones = InfiniteList::constant(1);
///
/// filter(is_greater_zero).eval(InfiniteList::cons(0, ones));
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

/// Construct the stream processor which applies a given closure to each element of the input stream.
/// - `f` is the closure to be applied.
///
/// The function is in analogy to the map-function on lists which is well-known in functional programming.
///
/// # Examples
///
/// Negate a stream of bools:
///
/// ```
/// use rspl::combinators::map;
/// use rspl::streams::infinite_lists::InfiniteList;
/// use rspl::StreamProcessor;
///
/// let negate = |b: bool| !b;
///
/// let trues = InfiniteList::constant(true);
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
    use crate::streams::overeager_receivers::OvereagerReceiver;
    use crate::streams::Stream;

    use crate::assert_head_eq;
    use crate::assert_tail_starts_with;
    use crate::enqueue;

    #[test]
    fn test_alternate() {
        let is_greater_zero = |n: &i8| *n > 0;
        let is_less_zero = |n: &i8| *n < 0;

        let sp = alternate(filter(is_greater_zero), filter(is_less_zero));

        let (tx, stream) = OvereagerReceiver::channel(0, 0);
        enqueue!(tx, [1, 2, -1, -2, 1]);

        let mut result = sp.eval(stream);
        assert_head_eq!(result, 1);
        assert_tail_starts_with!(result, [-1, 1]);
    }

    #[test]
    fn test_bind() {
        let is_zero = |n: usize| n == 0;

        let sp = bind(map(is_zero), |b: bool| {
            if b {
                bind(map(is_zero), |b: bool| {
                    if b {
                        map(|n| n + 2)
                    } else {
                        map(|n| n + 1)
                    }
                })
            } else {
                filter(|n| *n > 0)
            }
        });

        let (tx, stream) = OvereagerReceiver::channel(0, 0);
        enqueue!(tx, [1, 0, 1, 2]);

        let mut result = sp.eval(stream);
        assert_head_eq!(result, 1);
        assert_tail_starts_with!(result, [2, 3]);
    }

    #[test]
    fn test_compose() {
        let plus_one = |n: usize| n + 1;

        let sp = compose(map(plus_one), map(plus_one));

        let (tx, stream) = OvereagerReceiver::channel(10, 0);
        enqueue!(tx, [1, 2, 10]);

        let mut result = sp.eval(stream);
        assert_head_eq!(result, 2);
        assert_tail_starts_with!(result, [3, 4]);
    }

    #[test]
    fn test_filter() {
        let is_greater_zero = |n: &usize| *n > 0;

        let sp = filter(is_greater_zero);

        let (tx, stream) = OvereagerReceiver::channel(0, 0);
        enqueue!(tx, [1, 0, 2]);

        let mut result = sp.eval(stream);
        assert_head_eq!(result, 1);
        assert_tail_starts_with!(result, [2]);
    }

    #[test]
    fn test_map() {
        let plus_one = |n: usize| n + 1;

        let sp = map(plus_one);

        let (tx, stream) = OvereagerReceiver::channel(10, 0);
        enqueue!(tx, [1]);

        let mut result = sp.eval(stream);
        assert_head_eq!(result, 1);
        assert_tail_starts_with!(result, [2]);
    }
}

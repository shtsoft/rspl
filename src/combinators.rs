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
/// use rspl::streams::infinite_lists::InfiniteList;
/// use rspl::StreamProcessor;
///
/// let is_false = |b: &bool| !b;
///
/// let falses = rspl::streams::infinite_lists::InfiniteList::constant(false);
///
/// filter(is_false).eval(InfiniteList::cons(true, falses));
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

/// The function combines two stream processors into one alternating between the two whenever something is written to the ouput stream.
/// - `sp1` is the stream processor which is in control.
/// - `sp2` is the stream processor to which control is transferred.
///
/// This is function is in analogy to running coroutines.
///
/// # Examples
///
/// Remove the `true`s from a stream of bools:
///
/// ```
/// use rspl::combinators::{alternate, map};
/// use rspl::StreamProcessor;
///
/// let id = |b: bool| b;
/// let flip = |b: bool| !b;
///
/// let trues = rspl::streams::infinite_lists::InfiniteList::constant(true);
///
/// alternate(map(id), map(flip)).eval(trues);
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

    #[test]
    fn test_alternate() {
        let is_greater_zero = |n: &i8| *n > 0;
        let is_less_zero = |n: &i8| *n < 0;

        let sp = alternate(filter(is_greater_zero), filter(is_less_zero));

        let (tx, stream) = OvereagerReceiver::channel(0, 0);
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(-1).unwrap();
        tx.send(-2).unwrap();
        tx.send(1).unwrap();
        tx.send(0).unwrap();
        tx.send(0).unwrap();

        let result = sp.eval(stream);
        assert_eq!(*result.head(), 1);

        let result_tail = result.tail();
        assert_eq!(*result_tail.head(), -1);

        let result_tail_tail = result_tail.tail();
        assert_eq!(*result_tail_tail.head(), 1);
    }
}

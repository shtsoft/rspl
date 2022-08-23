//! This module defines streams of some type extensionally by means of a trait.
//! Additionally, it declares modules with implementations of the trait and re-exports the name of the implementation.

mod infinite_lists;
pub use infinite_lists::InfiniteList;

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

/// Print a specified number of elements from some stream returning the not printed part.
/// - `stream` is the stream to be printed.
/// - `n` is the number of elements to be printed.
///
/// # Panics
///
/// A panic is caused if the respective implementation of [`Stream::head`] or [`Stream::tail`] panics.
///
/// # Examples
///
/// Printing the first five elements of the infinite list `0, 1, 2, ..., |usize|, 0, 1, ...`:
///
/// ```
/// use rspl::{InfiniteList, print};
///
/// fn ascending(n: usize) -> InfiniteList<usize> {
///     InfiniteList::Cons(n, Box::new(move || ascending(n + 1)))
/// }
///
/// print(ascending(0), 5);
/// ```
pub fn print<X, S>(mut stream: S, n: usize) -> S
where
    S: Stream<X>,
    X: std::fmt::Display + Copy,
{
    for _ in 0..n {
        println!("{}", stream.head());
        stream = stream.tail();
    }

    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print() {
        let stream = InfiniteList::Cons(
            false,
            Box::new(move || {
                InfiniteList::Cons(
                    false,
                    Box::new(move || {
                        InfiniteList::Cons(true, Box::new(move || InfiniteList::constant(true)))
                    }),
                )
            }),
        );

        let stream = print(stream, 2);
        assert!(stream.head());
    }
}

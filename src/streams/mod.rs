//! This module defines streams of some type extensionally by means of a trait.
//! Additionally, the module declares submodules with implementations of the trait.

pub mod infinite_lists;
pub mod overeager_receivers;

/// A characterization of streams of some type `X`: a stream of `X` is an object from which one can observe something of type `X` (the head of the stream) or another stream of `X` (the tail of the stream).
pub trait Stream<X> {
    /// Return a reference to the first item of `self`.
    fn head(&self) -> &X;
    /// Throw away the first item of `self` and return what is left.
    fn tail(self) -> Self;
}

/// Print a specified number of elements from some provided stream returning the not printed part.
/// - `stream` is the stream to be printed.
/// - `n` is the number of elements to be printed.
///
/// Note that the function can block the current thread if the respective implementation of [`Stream::tail`] can.
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
/// use rspl::streams::infinite_lists::InfiniteList;
///
/// fn ascending<'a>(n: usize) -> InfiniteList<'a, usize> {
///     InfiniteList::Cons(n, Box::new(move || ascending(n + 1)))
/// }
///
/// rspl::streams::print(ascending(0), 5);
/// ```
pub fn print<X, S>(mut stream: S, n: usize) -> S
where
    S: Stream<X>,
    X: std::fmt::Display,
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
    use infinite_lists::InfiniteList;

    #[test]
    fn test_print() {
        let stream = InfiniteList::Cons(
            false,
            Box::new(|| {
                InfiniteList::Cons(
                    false,
                    Box::new(|| {
                        InfiniteList::Cons(true, Box::new(|| InfiniteList::constant(true)))
                    }),
                )
            }),
        );

        let stream = print(stream, 2);
        assert!(stream.head());
    }
}

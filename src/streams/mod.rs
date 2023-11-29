//! This module defines streams of some type intensionally by means of a trait.
//! Additionally, the module declares submodules with implementations of the trait.

pub mod infinite_lists;

#[cfg(feature = "std")]
pub mod overeager_receivers;

/// A characterization of streams of some type `X`: a stream of `X` is an object from which one can observe something of type `X` (the head of the stream) or another stream of `X` (the tail of the stream).
pub trait Stream<X> {
    /// Returns a reference to the first item of `self`.
    fn head(&self) -> &X;
    /// Throws away the first item of `self` and returns what is left.
    ///
    /// # Panics
    ///
    /// Implementations may choose to panic.
    fn tail(self) -> Self;
}

/// Prints a specified number of elements from some provided stream returning the not printed part.
/// - `stream` is the stream to be printed.
/// - `n` is the number of elements to be printed.
///
/// # Panics
///
/// A panic is caused if the respective implementation of [`Stream`] panics.
///
/// # Notes
///
/// Note that the function can block the current thread if the respective implementation of [`Stream`] can.
/// However, if it does not, then you might find it slow as it uses `println!` in a loop.
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
#[cfg(feature = "std")]
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
    #[cfg(feature = "std")]
    use super::*;

    #[cfg(feature = "std")]
    use infinite_lists::InfiniteList;

    #[macro_export]
    macro_rules! assert_head_eq {
        ($stream:expr, $x:expr) => {
            assert_eq!(*$stream.head(), $x);
        };
    }

    #[macro_export]
    macro_rules! assert_tail_starts_with {
        ($stream:expr, $xs:expr) => {
            for x in $xs {
                $stream = $stream.tail();
                assert_head_eq!($stream, x);
            }
        };
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_print() {
        let stream = InfiniteList::cons(false, || {
            InfiniteList::cons(false, || {
                InfiniteList::cons(true, || InfiniteList::constant(true))
            })
        });

        let stream = print(stream, 2);
        assert!(stream.head());
    }
}

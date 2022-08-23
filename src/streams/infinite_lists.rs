//! This module provides the standard implementation of streams as infinite lists (the greatest fixpoint of `cons`ing).
//! Moreover it contains implementations to destruct and construct such infinite lists.

use super::Stream;

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
            Self::Cons(head, _) => *head,
        }
    }

    /// Make all but the first list entry of `self` the tail.
    fn tail(self) -> Self {
        match self {
            Self::Cons(_, tail) => tail(),
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
    pub fn constant(x: X) -> Self
    where
        X: Copy + 'static,
    {
        Self::Cons(x, Box::new(move || Self::constant(x)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head() {
        let inflist = InfiniteList::Cons(true, Box::new(move || InfiniteList::constant(false)));
        assert!(inflist.head());
    }

    #[test]
    fn test_tail() {
        let inflist = InfiniteList::Cons(
            false,
            Box::new(move || {
                InfiniteList::Cons(true, Box::new(move || InfiniteList::constant(true)))
            }),
        );
        assert!(inflist.tail().head());
    }

    #[test]
    fn test_print() {
        const N: usize = 2;

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

        let stream = stream.print(N);
        assert!(stream.head());
    }

    #[test]
    fn test_constant() {
        const X: usize = 0;
        let xs = InfiniteList::constant(X);

        let xs_head = xs.head();
        assert_eq!(xs_head, X);

        let xs = xs.tail();
        let xs_head = xs.head();
        assert_eq!(xs_head, X);
    }
}

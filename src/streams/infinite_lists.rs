//! This module provides the standard implementation of streams as infinite lists (the greatest fixpoint of `cons`ing).

use super::Stream;

use alloc::boxed::Box;

/// [`Lazy<T>`] types thunks of type `T`.
type Lazy<'a, T> = dyn FnOnce() -> T + 'a;

/// [`InfiniteList<X>`] defines non-well-founded list of type `X`.
pub enum InfiniteList<'a, X: 'a> {
    /// Constructing a new infinite list by prepending a new entry to an existing (lazy) infinite list.
    Cons(X, Box<Lazy<'a, InfiniteList<'a, X>>>),
}

impl<'a, X> InfiniteList<'a, X> {
    /// The same as [`InfiniteList::Cons`] but with thunking and boxing of `inflist` hidden to make the resulting code less verbose.
    #[inline]
    pub fn cons(x: X, inflist: Self) -> Self {
        InfiniteList::Cons(x, Box::new(|| inflist))
    }
}

impl<'a, X> InfiniteList<'a, X> {
    /// Create an infinte list of a certain constant.
    /// - `x` is the constant.
    ///
    /// # Examples
    ///
    /// Creating an infinite list of `true`s:
    ///
    /// ```
    /// let trues = rspl::streams::infinite_lists::InfiniteList::constant(true);
    /// ```
    pub fn constant(x: X) -> Self
    where
        X: Copy,
    {
        Self::Cons(x, Box::new(move || Self::constant(x)))
    }
}

impl<'a, X> Stream<X> for InfiniteList<'a, X> {
    /// Make the first list enrty of `self` the head.
    fn head(&self) -> &X {
        match self {
            Self::Cons(head, _) => head,
        }
    }

    /// Make all but the first list entry of `self` the tail.
    fn tail(self) -> Self {
        match self {
            Self::Cons(_, tail) => tail(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::assert_head_eq;
    use crate::assert_tail_starts_with;

    #[test]
    fn test_cons() {
        assert!(matches!(
            InfiniteList::cons((), InfiniteList::constant(())),
            InfiniteList::Cons(_, _)
        ));
    }

    #[test]
    fn test_constant() {
        const X: bool = true;

        let mut xs = InfiniteList::constant(X);
        assert_head_eq!(xs, X);
        assert_tail_starts_with!(xs, [X, X]);
    }

    #[test]
    fn test_head() {
        let inflist = InfiniteList::cons(true, InfiniteList::constant(false));
        assert!(inflist.head());
    }

    #[test]
    fn test_tail() {
        let inflist = InfiniteList::cons(
            false,
            InfiniteList::cons(true, InfiniteList::constant(true)),
        );
        assert!(inflist.tail().head());
    }
}

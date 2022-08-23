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

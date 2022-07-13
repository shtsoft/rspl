mod streams {
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

    /// [`InfiniteList<X>`] defines non-well-founded list of type `X`.
    pub enum InfiniteList<X> {
        /// Constructing a new infinite list by prepending a new entry to an existing (lazy) inifinite list.
        Cons(X, Box<dyn FnOnce() -> InfiniteList<X>>),
    }

    impl<X> Stream<X> for InfiniteList<X>
    where
        X: Copy,
    {
        /// Make the first list enrty of `self` the head.
        fn head(&self) -> X {
            match self {
                InfiniteList::Cons(head, _) => *head,
            }
        }

        /// Make all but the first list entry of `self` the tail.
        fn tail(self) -> Self {
            match self {
                InfiniteList::Cons(_, tail) => tail(),
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
        pub fn print(self, n: usize) -> Self
        where
            X: std::fmt::Display + Copy + 'static,
        {
            if n == 0 {
                return self;
            }
            println!("{}", self.head());
            InfiniteList::print(self.tail(), n - 1)
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
        pub fn constant(x: X) -> InfiniteList<X>
        where
            X: Copy + 'static,
        {
            InfiniteList::Cons(x, Box::new(move || InfiniteList::constant(x)))
        }
    }
}

pub use streams::{InfiniteList, Stream};

/// [`StreamProcessor<A, B>`] defines (the syntax of) a language describing the domain of stream processors, that is, terms which can be interpreted to turn streams of type `A` into streams of type `B`.
pub enum StreamProcessor<A, B> {
    /// Read the head `a` of the input stream and use `f(a)` to process the tail of the input stream.
    Get(Box<dyn FnOnce(A) -> StreamProcessor<A, B>>),
    /// Write `b` to the output list and use the `lazy_stream_processor` to process the input stream if needed.
    Put(B, Box<dyn FnOnce() -> StreamProcessor<A, B>>),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn negate(b: bool) -> bool {
        !b
    }

    fn times_two(n: usize) -> usize {
        n * 2
    }

    fn ascending(n: usize) -> InfiniteList<usize> {
        InfiniteList::Cons(n, Box::new(move || ascending(n + 1)))
    }

    #[test]
    fn it_works() {
        let result = eval(map(negate), eval(map(negate), InfiniteList::constant(true)));
        result.print(10);
        //assert_eq!(result, ...);

        let result = eval(map(times_two), ascending(0));
        result.print(10);
        //assert_eq!(result, ...);
    }
}

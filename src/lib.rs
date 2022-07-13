mod streams {
    pub trait Stream<X>
    where
        X: Copy,
    {
        fn head(&self) -> X;
        fn tail(self) -> Self;
    }

    pub enum InfiniteList<X> {
        Cons(X, Box<dyn FnOnce() -> InfiniteList<X>>),
    }

    impl<X> Stream<X> for InfiniteList<X>
    where
        X: Copy,
    {
        fn head(&self) -> X {
            match self {
                InfiniteList::Cons(head, _) => *head,
            }
        }

        fn tail(self) -> Self {
            match self {
                InfiniteList::Cons(_, tail) => tail(),
            }
        }
    }

    impl<X> InfiniteList<X> {
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
        pub fn constant(x: X) -> InfiniteList<X>
        where
            X: Copy + 'static,
        {
            InfiniteList::Cons(x, Box::new(move || InfiniteList::constant(x)))
        }
    }
}

pub use streams::{InfiniteList, Stream};

pub enum StreamProcessor<A, B> {
    Get(Box<dyn FnOnce(A) -> StreamProcessor<A, B>>),
    Put(B, Box<dyn FnOnce() -> StreamProcessor<A, B>>),
}

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

pub fn map<A, B>(f: fn(A) -> B) -> StreamProcessor<A, B>
where
    A: 'static,
    B: 'static,
{
    StreamProcessor::Get(Box::new(move |a: A| {
        StreamProcessor::Put(f(a), Box::new(move || map(f)))
    }))
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

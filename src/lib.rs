mod streams {
    pub enum Stream<X> {
        Cons(X, Box<dyn FnOnce() -> Stream<X>>),
    }

    impl<X> Stream<X> {
        pub fn split(self) -> (X, Stream<X>) {
            match self {
                Stream::Cons(head, tail) => (head, tail()),
            }
        }

        pub fn print(self, n: usize) -> Self
        where
            X: std::fmt::Display,
        {
            if n == 0 {
                return self;
            }
            let (head, tail) = Stream::split(self);
            println!("{}", head);
            Stream::print(tail, n - 1)
        }
    }

    impl<X> Stream<X> {
        pub fn constant(x: X) -> Stream<X>
        where
            X: Copy + 'static,
        {
            Stream::Cons(x, Box::new(move || Stream::constant(x)))
        }
    }
}

pub use streams::Stream;

pub enum StreamProcessor<A, B> {
    Get(Box<dyn FnOnce(A) -> StreamProcessor<A, B>>),
    Put(B, Box<dyn FnOnce() -> StreamProcessor<A, B>>),
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

pub fn eval<A, B>(sp: StreamProcessor<A, B>, stream: Stream<A>) -> Stream<B>
where
    A: 'static,
    B: 'static,
{
    match sp {
        StreamProcessor::Get(f) => match stream {
            Stream::Cons(head, tail) => eval(f(head), tail()),
        },
        StreamProcessor::Put(b, lazy_sp) => {
            Stream::Cons(b, Box::new(move || eval(lazy_sp(), stream)))
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

    fn ascending(n: usize) -> Stream<usize> {
        Stream::Cons(n, Box::new(move || ascending(n + 1)))
    }

    #[test]
    fn it_works() {
        let result = eval(map(negate), eval(map(negate), Stream::constant(true)));
        result.print(10);
        //assert_eq!(result, ...);

        let result = eval(map(times_two), ascending(0));
        result.print(10);
        //assert_eq!(result, ...);
    }
}

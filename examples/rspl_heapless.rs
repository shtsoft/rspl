//&# rspl Heapless
//&

//&```rust
use rspl::streams::Stream;

struct ComatchConstant<X> {
    constant: X,
}

impl<X> Stream<X> for ComatchConstant<X> {
    fn head(&self) -> &X {
        &self.constant
    }

    fn tail(self) -> Self {
        self
    }
}

enum StreamProcessor<A, B, F, L>
where
    F: Fun<A, B, F, L>,
    L: LazySP<A, B, F, L>,
{
    Get(F, std::marker::PhantomData<A>),
    Put(B, L),
}

trait Fun<A, B, F, L>
where
    F: Fun<A, B, F, L>,
    L: LazySP<A, B, F, L>,
{
    fn apply(self, a: A) -> StreamProcessor<A, B, F, L>;
}

trait LazySP<A, B, F, L>
where
    F: Fun<A, B, F, L>,
    L: LazySP<A, B, F, L>,
{
    fn force(self) -> StreamProcessor<A, B, F, L>;
}

struct ComatchEval<A, B, S, F, L>
where
    S: Stream<A>,
    F: Fun<A, B, F, L>,
    L: LazySP<A, B, F, L>,
{
    phantom_a: std::marker::PhantomData<A>,
    phantom_f: std::marker::PhantomData<F>,
    stream: S,
    output: B,
    lazy_sp: L,
}

impl<A, B, S, F, L> Stream<B> for ComatchEval<A, B, S, F, L>
where
    A: Clone,
    S: Stream<A>,
    F: Fun<A, B, F, L>,
    L: LazySP<A, B, F, L>,
{
    fn head(&self) -> &B {
        &self.output
    }

    fn tail(mut self) -> Self {
        let sp = self.lazy_sp.force();

        if let StreamProcessor::Get(_, _) = sp {
            self.stream = self.stream.tail();
        }

        StreamProcessor::eval(sp, self.stream)
    }
}

impl<A, B, F, L> StreamProcessor<A, B, F, L>
where
    F: Fun<A, B, F, L>,
    L: LazySP<A, B, F, L>,
{
    fn eval<S: Stream<A>>(mut self, mut stream: S) -> ComatchEval<A, B, S, F, L>
    where
        A: Clone,
    {
        loop {
            match self {
                StreamProcessor::Get(f, _) => {
                    self = f.apply(stream.head().clone());
                    while let StreamProcessor::Get(f, _) = self {
                        stream = stream.tail();
                        self = f.apply(stream.head().clone());
                    }
                    continue;
                }
                StreamProcessor::Put(b, lazy_sp) => {
                    return ComatchEval {
                        phantom_a: std::marker::PhantomData,
                        phantom_f: std::marker::PhantomData,
                        stream,
                        output: b,
                        lazy_sp,
                    }
                }
            }
        }
    }
}

struct ComatchMap<'a, A: 'a, B: 'a, F>
where
    F: Fn(A) -> B,
{
    a: std::marker::PhantomData<&'a A>,
    b: std::marker::PhantomData<&'a B>,
    function: F,
}

type SPMap<'a, A, B, F> = StreamProcessor<A, B, ComatchMap<'a, A, B, F>, ComatchMap<'a, A, B, F>>;

impl<'a, A, B, F> Fun<A, B, ComatchMap<'a, A, B, F>, ComatchMap<'a, A, B, F>>
    for ComatchMap<'a, A, B, F>
where
    F: Fn(A) -> B,
{
    fn apply(self, a: A) -> SPMap<'a, A, B, F> {
        StreamProcessor::Put((self.function)(a), self)
    }
}

impl<'a, A, B, F> LazySP<A, B, ComatchMap<'a, A, B, F>, ComatchMap<'a, A, B, F>>
    for ComatchMap<'a, A, B, F>
where
    F: Fn(A) -> B,
{
    fn force(self) -> SPMap<'a, A, B, F> {
        map(self.function)
    }
}

const fn map<'a, A: 'a, B: 'a, F>(f: F) -> SPMap<'a, A, B, F>
where
    F: Fn(A) -> B,
{
    StreamProcessor::Get(
        ComatchMap {
            a: std::marker::PhantomData,
            b: std::marker::PhantomData,
            function: f,
        },
        std::marker::PhantomData,
    )
}

fn main() {
    let trues = ComatchConstant { constant: true };

    let stream = map(|b: bool| !b).eval(trues);

    rspl::streams::print(stream, 1_000_000);
}
//&```rust

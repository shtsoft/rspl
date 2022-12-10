# rspl Heapless

This example explores the viability of an alternative implementation of rspl based on something like closure conversion to avoid the use of a heap.
Such an implementation would make rspl better suited for embedded systems as it could not only forgo the standard library but also an allocator.
In the rest of this file we explain and implement that heapless approach to draw a tentative conclusion on its viability.

rspl uses the heap essentially for laziness when it thunks stream processing and tails of streams.
This is because in those cases rust does not statically determine the size of the thunks.
Assisting rust in that regard by converting those closures eliminates the need for a heap.\
What we mean by converting closures here can be understood by looking at the example of streams as codata[^1].

[^1]: A modern take on OOP generalizing closures (see e.g. [Codata in Action](https://www.microsoft.com/en-us/research/uploads/prod/2020/01/CoDataInAction.pdf)).

In Agda one defines streams of type `X` (extensionally) by the codata type
```agda
record StreamX : Set where
  coinductive
  field
    head : X
    tail : StreamX
```
This can be read as 'any object from which one can observe something of type `X` (the `head`) and a something of type `StreamX` (the `tail`) is a `StreamX`'.
Defining a stream of `X`s can then be done by copatttern matching, that is, by specifying what the `head` and the `tail` of the stream shall be.
For example, given some `x : X` one can construct the constant stream of `x` by the comatch
```agda
constant : X -> StreamX
head (constant x) = x
tail (constant x) = constant x
```
Now, to encode `constant` in rust one can convert that generalized closure by making its environment (its domain) explicit by means of a struct (as usual) and implement the `Stream<X>`-trait accordingly:

```rust
trait Stream<X> {
    fn head(&self) -> &X;
    fn tail(self) -> Self;
}

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
```

This also works in greater generality for mutually recursive codata.
For ordniary stream processors in Agda
```agda
data StreamProcessor : Set where
  get : (A -> StreamProcessor) -> StreamProcessor
  put : B -> LazySP -> StreamProcessor

record LazySP where
  coinductive
  field
    force : StreamProcessor
```
we get the following rspl equivalent (if we also generalize `X -> Y` to the extensional definition of function with the help of codata) in rust:

```rust
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
```

Then, for a proof of concept we can reimplement the `map`-combinator and apply it to some stream:

```rust
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

    let mut stream = map(|b: bool| !b).eval(trues);

    for _ in 0..1_000_000 {
        println!("{}", stream.head());
        stream = stream.tail();
    }
}
```

The tentative conclusion is that while the approach seems doable it has significant negative consequences: stream processors become harder to understand and more tedious to write.
Therefore the approach is impractical.
At least, without a better language frontend.

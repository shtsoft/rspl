# rspl

[![crates.io][crates-badge]][crates-url]
[![GPL licensed][license-badge]][license-url]
[![CI][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/rspl.svg
[crates-url]: https://crates.io/crates/rspl
[license-badge]: https://img.shields.io/badge/license-GPL-blue.svg
[license-url]: ./Cargo.toml
[actions-badge]: https://github.com/shtsoft/rspl/actions/workflows/ci.yaml/badge.svg
[actions-url]: https://github.com/shtsoft/rspl/actions/workflows/ci.yaml

A **powerful** stream processor language with roots in functional programming (see [Generalising Monads to Arrows](https://www.sciencedirect.com/science/article/pii/S0167642399000234)) embedded in **Rust** and supplemented with adaptations of appreciated **design patterns**.

- power:
  * syntactically explicit process control
  * arbitrary mixing of reactive (event-driven) and generative (demand-driven) processing
  * high-level stream processor combinators (combinator-driven)
  * agnostic regarding input-stream implementation
- design patterns (also see [Released API docs](https://docs.rs/rspl)):
  * [type-state pattern for event-driven programming](https://github.com/shtsoft/rspl/blob/master/examples/pelican.md)
  * [state-passing pattern for demand-driven programming](https://github.com/shtsoft/rspl/blob/master/examples/hics.md)
- Rust:
  * safety:
    + no dependencies (apart from crossbeam-option)
    + thoroughly testet
    + memory-safety: no `unsafe`-code
  * `no_std`-option
  * high-level library for low-level purposes

For documentation see [Released API docs](https://docs.rs/rspl).
In particular, you can find a design- and usage-description there.

### Related Work

One of rspl's earliest ancestor seems to be [FUDGETS: a graphical user-interface in a lazy functional language](https://dl.acm.org/doi/pdf/10.1145/165180.165228) and the accroding [implementation](https://hackage.haskell.org/package/fudgets).
Fudgets can be thought of as stream processors which process the stream both high- and low-level where the high-level processing is responsible for coordination with the environment and the low-level with the actual task.
The use of fudgets is also briefly discussed in [Generalising Monads to Arrows](https://www.sciencedirect.com/science/article/pii/S0167642399000234).\
So, as the origins of rspl date back quite some time it is not surprising that there is some theoretical work on it.
For example, [Representations of Stream Processors Using Nested Fixed Points](https://arxiv.org/pdf/0905.4813) is a paper on the semantics of rspl-like stream processors while in [Termination Checking Nested Inductive and Coinductive Types](https://www.cs.nott.ac.uk/~psztxa/publ/InvertedQuantifiers.pdf) they serve as an example to understand termination checking of modern proof assistants.\
But there is also a more recent practial work which is worth mentioning.
[Quiver](https://hackage.haskell.org/package/quiver) is a Haskell-library which seems very similar to rspl (and claims to generalize the apparently more famous but harder to understand [pipes](https://hackage.haskell.org/package/pipes)).
The main difference is that Quiver's language constructs for in- and output are totally symmetric, whereas rspl rather reflects the intuitive asymmetry of stream processing w.r.t. in- and output.\
Last but not least, let us mention [strymonas](https://github.com/strymonas).
Although its take on stream processing differs from that of rspl, it is still interesting for rspl due to its property of stream fusion.
It would be nice to have an efficient composition combinator in rspl with the same property.
However, rspl's composition combinator does currently not live up to that.

### Future Work

rspl is not quite finished yet.
There remain some important things to do:
- There have been little efforts on improving efficiency so far.
  The first thing to do in that regard is some benchmarking to see how bad rspl really does.
  Here, interesting competitors are [Quiver](https://hackage.haskell.org/package/quiver) and [strymonas](https://github.com/strymonas).
  The former due its similarity to rspl and the latter due to its claim on performance.
  The results will then guide the further process.
  However, one thing we promise to check regardless of the results is whether rspl can somehow exploit parallelism.
- rspl aims to support its use in embedded rust.
  As of yet, while the standard library is not strictly, needed an allocator is.
  But we have two approaches in mind to get rid of the necessity of a heap:
    * We could try to reimplement rspl following the low-level approach discussed in [here (as .md file)](https://github.com/shtsoft/rspl/blob/master/examples/rspl_heapless.md) and [here (as .rs file)](https://github.com/shtsoft/rspl/blob/master/examples/rspl_heapless.rs).
    * rspl uses the allocator only for some `Box`es and it is conceivable to store those boxes on 'mini-heaps' residing in stack frames (compare [smallbox](https://github.com/andylokandy/smallbox)).
      However, this approach needs further realizabilty analyses first.
- You cannot have enough combinators.
  So you can expect more to come.
  Particularly, [fudgets](https://hackage.haskell.org/package/fudgets) are lacking.
  Moreover, asynchronous versions of exsiting combinators (like `map`) will be considered.

## Contributing

If you want to contribute: [CONTRIBUTING](CONTRIBUTING.md).

### Security

For security-related issues see: [SECURITY](SECURITY.md).

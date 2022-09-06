use rspl::combinators::{alternate, bind, map};
use rspl::streams::infinite_lists::InfiniteList;
use rspl::streams::overeager_receivers::OvereagerReceiver;
use rspl::streams::{print, Stream};
use rspl::StreamProcessor;

use std::thread;

#[test]
fn test_basic() {
    const fn id<X>(x: X) -> X {
        x
    }

    let is_zero = |n: usize| n == 0;

    // a silly stream processor
    let sp_aux = StreamProcessor::get(|n1: usize| {
        StreamProcessor::get(move |n2: usize| {
            StreamProcessor::put(n2, StreamProcessor::put(n1, map(id)))
        })
    });
    let sp = bind(map(is_zero), |b| {
        if b {
            alternate(sp_aux, map(|n| n + 1))
        } else {
            map(id)
        }
    });

    let (tx, stream) = OvereagerReceiver::channel(0, 0);

    // a silly way to construct the stream beginning with 0, 1, 2, 3, 4, 5, 6
    let fill_stream = thread::spawn(move || {
        fn ascending<'a>(n: usize) -> InfiniteList<'a, usize> {
            InfiniteList::Cons(n, Box::new(move || ascending(n + 1)))
        }

        let mut stream = ascending(1);

        for _ in 0..6 {
            tx.send(*stream.head()).unwrap();
            stream = stream.tail();
        }
    });

    let result = sp.eval(stream);
    assert_eq!(*result.head(), 2);

    let result_tail = result.tail();
    assert_eq!(*result_tail.head(), 4);

    let rest = print(result_tail, 3);
    assert_eq!(*rest.head(), 5);

    fill_stream.join().unwrap();
}

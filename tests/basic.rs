use rspl::combinators::map;
use rspl::streams::infinite_lists::InfiniteList;
use rspl::streams::overeager_receivers::OvereagerReceiver;
use rspl::streams::{print, Stream};
use rspl::StreamProcessor;

use std::thread;

#[test]
fn test_basic() {
    const N: usize = 3;

    let sp = StreamProcessor::get(|n: usize| {
        if n % 2 == 0 {
            StreamProcessor::put(N, map(|n| N * n))
        } else {
            StreamProcessor::put(N + 1, map(|n| N * n + 1))
        }
    });

    let (tx, stream) = OvereagerReceiver::channel(0, 0);

    let fill_stream = thread::spawn(move || {
        fn ascending<'a>(n: usize) -> InfiniteList<'a, usize> {
            InfiniteList::Cons(n, Box::new(move || ascending(n + 1)))
        }

        let mut stream = ascending(1);

        for _ in 0..5 {
            tx.send(*stream.head()).unwrap();
            stream = stream.tail();
        }
    });

    let result = sp.eval(stream);
    assert_eq!(*result.head(), N);

    let result_tail = result.tail();
    assert_eq!(*result_tail.head(), N);

    let rest = print(result_tail, 3);
    assert_eq!(*rest.head(), N * 4);

    fill_stream.join().unwrap();
}

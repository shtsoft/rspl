use rspl::map;
use rspl::streams::overeager_receivers::OvereagerReceiver;
use rspl::streams::print;

use std::thread;

#[test]
#[ignore]
fn test_load() {
    const N: usize = 9;

    const fn factorial(mut n: usize) -> usize {
        let mut acc = n;
        while n > 1 {
            acc = acc * n - 1;
            n -= 1;
        }
        acc
    }

    let sp = map(factorial);

    let (tx, stream) = OvereagerReceiver::channel(0, 0);

    let fill_stream = thread::spawn(move || {
        for _ in 0..factorial(N) {
            for n in 0..N {
                tx.send(n).unwrap();
            }
        }
    });

    let result = sp.eval(stream);

    let rest = print(result, factorial(N));

    fill_stream.join().unwrap();

    print(rest, (N - 1) * factorial(N) - 1);
}

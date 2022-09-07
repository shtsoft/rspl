use rspl::combinators::{compose, filter, map};
use rspl::streams::overeager_receivers::OvereagerReceiver;
use rspl::streams::print;

use std::thread;

#[test]
#[ignore]
fn test_load() {
    const N: usize = 10;

    const fn factorial(mut n: usize) -> usize {
        let mut acc = n;
        while n > 1 {
            acc *= n - 1;
            n -= 1;
        }
        acc
    }

    let is_even = |n: &usize| *n % 2 == 0;
    let plus_one = |n: usize| n + 1;

    let sp = compose(compose(filter(is_even), map(factorial)), map(plus_one));

    let (tx, stream) = OvereagerReceiver::channel(0, 0);
    let input_simulator = thread::spawn(move || {
        for _ in 0..factorial(N) {
            for n in 0..N {
                tx.send(n).unwrap();
            }
        }
    });

    let result = sp.eval(stream);

    let rest = print(result, factorial(N));

    input_simulator.join().unwrap();

    print(rest, factorial(N));
}

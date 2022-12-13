use rspl::streams::infinite_lists::InfiniteList;
use rspl::streams::print;
use rspl::streams::Stream;
use rspl::StreamProcessor;

#[test]
fn test_demands() {
    const IMPORTANT_NUMBER_REFERENCE: f64 = 12.077_005_857;
    const EPS: f64 = 0.001;

    const STEPS_SQRT2: usize = 10;
    const STEPS_PI: usize = 5;
    const STEPS_EXP: usize = 10;

    // compute the square root of 2 via the Babylonian method
    fn babylon2<'a>(x: f64) -> StreamProcessor<'a, (), f64> {
        StreamProcessor::put(x, move || babylon2((x + 2.0 / x) / 2.0))
    }

    // compute pi by Bailey-Borwein-Plouffe formula
    fn bbp<'a>(partial_sum: f64, k: u32) -> StreamProcessor<'a, (), f64> {
        let bbp_sequence = |k| {
            (1.0 / f64::from(i32::pow(16, k)))
                * (4.0 / f64::from(8 * k + 1)
                    - 2.0 / f64::from(8 * k + 4)
                    - 1.0 / f64::from(8 * k + 5)
                    - 1.0 / f64::from(8 * k + 6))
        };

        StreamProcessor::put(partial_sum, move || {
            bbp(partial_sum + bbp_sequence(k), k + 1)
        })
    }

    // compute Euler's number
    fn euler<'a>(partial_sum: f64, k: u32, kfac: u32) -> StreamProcessor<'a, (), f64> {
        let euler_sequence = |kfac| 1.0 / f64::from(kfac);

        StreamProcessor::put(partial_sum, move || {
            euler(partial_sum + euler_sequence(kfac), k + 1, kfac * (k + 1))
        })
    }

    let sqrt2 = *print(babylon2(1.0).eval(InfiniteList::constant(())), STEPS_SQRT2).head();
    let pi = *print(bbp(0.0, 0).eval(InfiniteList::constant(())), STEPS_PI).head();
    let exp = *print(euler(1.0, 1, 1).eval(InfiniteList::constant(())), STEPS_EXP).head();

    let important_number = sqrt2 * pi * exp;

    assert!(f64::abs(important_number - IMPORTANT_NUMBER_REFERENCE) < EPS);
}

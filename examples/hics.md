This example implements a Heat Index Control System (hics).
More precisely, the example implements a system to keep the heat index (a quantity depending on the real temperature and the humidity via a certain mapping) in a room located in a region with a hot and damp climate bearable.\
The system specification is that the heat index has to be brought down periodically to a certain value within a window of tolerance depending on the daytime by first trying to dehumidify the room and if that does not suffice to also cool down the room.
Furthermore, a thermohygrometer and a clock shall be accessible for an implementation.\
The hics-implementation presented in this example measures temeprature and humidity as well as the time to decide if it has to take action and if it actuates something it waits for it to take effect in order to decide whether to repeat or to go idle for a period.
Moreover it follows the structuring-suggestions from [Why Functional Programming Matters](https://www.cse.chalmers.se/~rjmh/Papers/whyfp.pdf).
There they describe how the use of higher-order functions and lazy evaluation can greatly improves the modularity of programs.
They illustrate that by modularizing on demand computations of perhaps infinite objects like real numbers and game trees exploiting that in languages with first-class functions and lazy evaluation everything is a generator in some sense.
As the hics described above shares that 'on demand computation'-aspect - most notably, it measures (that is, reads out the thermohygrometer) on demand - these modularization techniques will apply in that case, too.
So adapting the techniques specifically to rspl and rust yields a modular implementation of the hics in rust.

The intention of the example is to demonstrate rspl's applicability in demand-driven[^1] programming.

[^1]: Look at [Codata in Action](https://www.microsoft.com/en-us/research/uploads/prod/2020/01/CoDataInAction.pdf) for some more explanation on that term.

Now that we have said what we are going to implement and why let us explain our techniques before
presenting the code applying those techniques.
To this end we split the following discussion in two parts.
First, we introduce the general design pattern of encoding generators in rspl.
This is done without referring to the specific example of control systems.
Then, second, before discussing the actual hics code in place we briefly discuss the code's overall structure.\
So, first, rspl's stream processors can encode some sort of generator: regarding a sufficiently general definition of generator one can consider any stream processor a generator because output is generated on demand in an incremental manner.
But even for the more specific definition of generators as functions which can remember state infromation between calls rspl's stream processors offer an encoding.
To understand how, first note that rspl's stream processors would implement the `Fn`-trait if rust allowed users to arbitrarily implement that trait.
This is because implementing a stream processor from `A` to `B` corresponds to defining a function from `Stream<A>` to `Stream<B>`.
So, if `A` is the input signature and `B` the yield type of a generator then that generator could be encoded as stream processor from `A` to `B` provided that a way to remember state information is available.
However, the perhaps most common approach to state in functional programming - and rspl is functional programming - is state-passing-style[^2].
[^2]: Also see the concept of monads which kind of subsumes foobar-passing-style.
And, in fact, state-passing is applicable to rspl's stream processors.
One way is to construct a stream processor by a rust function with a single parameter representing the state returning a stream processor capturing that state within a (lazy) recursive call.
The returned stream processor of such a function is a generator and the pattern of such a function is as follows:
```rust
fn generator<'a, S, A, B>(state: S) -> StreamProcessor<'a, A, B> {
    ...;
    StreamProcessor::get(|a: A| StreamProcessor::put(..., || generator(state)))
}
```
Here, the dots are supposed to be replaced by the generators body.
(Note that if `A` is `()` it can make sense to omit the `get`-part.)\
After having discussed the encoding of generators as stream processors let us have a look at the structure of our hics implementation.
Essentially, it consists of four parts.
The first is a module encapsulating general aspects of control systems.
The second and the third part specialize to and use those aspects for heat index controlling.
Particularly, the second part implements the control system interface while the third part is a driver responsible for executing that implementation according to the measure-on-demand strategy.
Finally, the fourth part is the main-function simulating the hics environment and setting up the driver for the hics.

Let us now walk through the code together.

```rust
mod control {
    use rspl::streams::infinite_lists::InfiniteList;
    use rspl::streams::Stream;
    use rspl::StreamProcessor;

    use std::thread;
    use std::time::Duration;

    // This is a definition of control system. Importantly, it requires a `meter` to generate
    // measurements on demand and that this is statically enforced by the typing.
    pub trait System<'a, Space> {
        fn meter(&self) -> StreamProcessor<'a, (), Space>;
        fn reference(&self) -> f64;
        fn quantity(&self, position: Space) -> f64;
        fn controller(self, deviation: f64, status: f64, position: Space) -> Self;
    }

    pub trait Strategy<'a, Space> {
        fn execute(self, cs: impl System<'a, Space>, epsilon: f64);
    }

    pub struct MeasureOnDemand {
        pub dwell_time: Duration,
    }

    impl<'a, Space: 'a + Copy> Strategy<'a, Space> for MeasureOnDemand {
        fn execute(self, mut cs: impl System<'a, Space>, epsilon: f64) {
            let mut status;

            // Here the measurements are generated (lazily).
            let mut positions = cs.meter().eval(InfiniteList::constant(()));

            loop {
                // Here the actual measurement is made.
                positions = positions.tail();
                let position = *positions.head();

                status = cs.quantity(position);
                let setpoint = cs.reference();
                let deviation = status - setpoint;

                if f64::abs(deviation) < epsilon {
                    break;
                }

                cs = cs.controller(deviation, status, position);

                thread::sleep(self.dwell_time);
            }
        }
    }
}

use control::Strategy;

use rspl::streams::infinite_lists::InfiniteList;
use rspl::streams::Stream;
use rspl::StreamProcessor;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam::channel;
use crossbeam::channel::Sender;

// This constant is the window of tolerance for the heat index.
const EPSILON: f64 = 0.5;

const REFERENCE_HEAT_INDEX_DAY: f64 = 91.0;
const REFERENCE_HEAT_INDEX_NIGHT: f64 = 83.0;

const MINIMAL_TEMPERATURE: f64 = 80.0;
const MINIMAL_HUMIDITY: f64 = 50.0;

const INITIAL_TEMPERATURE: f64 = 87.0;
const INITIAL_HUMIDITY: f64 = 72.0;

const ACTUATOR_DECREASE: HeatIndexSpace = HeatIndexSpace {
    temperature: 0.25,
    humidity: 1.5,
};
const NATURAL_INCREASE: HeatIndexSpace = HeatIndexSpace {
    temperature: 0.02,
    humidity: 0.1,
};

// This block defines time-related constants. In particular, note that the `TICK` is intended to
// represent 10 real seconds.
const TICK_LENGTH: u64 = 5; // in (real) millis
const TICK: u64 = 1;
const DAY: u64 = 8640 * TICK;
const DWELL_TIME: u64 = 6 * TICK;
const CONTROL_PERIOD: u64 = 180 * TICK;
const NATURAL_INCREASE_PERIOD: u64 = 3 * TICK;

const UNSAFE_BARRIER: usize = 100_000;
const SERVICE_BARRIER: usize = UNSAFE_BARRIER - 5000;

type HeatIndex = f64;
type Time = u64;
type Clock = AtomicU64;

#[derive(Copy, Clone)]
struct HeatIndexSpace {
    temperature: f64, // in degree Fahrenheit
    humidity: f64,    // in percent
}

// This type defines the output signals of the hics. A signal is either status information
// (`Show(...)`) or orders for the actuator to execute (`Dehumidfy` and `Cool`).
enum HeatIndexSignal {
    Show(Time, HeatIndex),
    Dehumidify,
    Cool,
}

// This type is the actual hics. Essentially, it is the communication interface to its environment.
#[derive(Clone)]
struct Hics {
    clock_finger: Arc<Clock>,
    thermohygrometer_finger: Arc<Mutex<HeatIndexSpace>>,
    signals_s: Sender<HeatIndexSignal>,
}

impl<'a> control::System<'a, HeatIndexSpace> for Hics {
    fn meter(&self) -> StreamProcessor<'a, (), HeatIndexSpace> {
        fn read_out<'a, X: 'a + Copy>(finger: Arc<Mutex<X>>) -> StreamProcessor<'a, (), X> {
            StreamProcessor::Put(
                *Arc::clone(&finger).lock().unwrap(),
                Box::new(|| read_out(finger)),
            )
        }

        read_out(Arc::clone(&self.thermohygrometer_finger))
    }
    fn reference(&self) -> f64 {
        let time = self.clock_finger.load(Ordering::SeqCst);

        if time % DAY < DAY / 2 {
            REFERENCE_HEAT_INDEX_DAY
        } else {
            REFERENCE_HEAT_INDEX_NIGHT
        }
    }
    fn quantity(&self, position: HeatIndexSpace) -> f64 {
        // The body is the heat index formula from https://en.wikipedia.org/wiki/Heat_index.
        const C_1: f64 = -42.379;
        const C_2: f64 = 2.049_015_23;
        const C_3: f64 = 10.143_331_27;
        const C_4: f64 = -0.224_755_41;
        const C_5: f64 = -0.006_837_83;
        const C_6: f64 = -0.054_817_17;
        const C_7: f64 = 0.001_228_74;
        const C_8: f64 = 0.000_852_82;
        const C_9: f64 = -0.000_001_99;

        let t = position.temperature;
        let r = position.humidity;

        C_1 + C_2 * t
            + C_3 * r
            + C_4 * t * r
            + C_5 * t * t
            + C_6 * r * r
            + C_7 * t * t * r
            + C_8 * t * r * r
            + C_9 * t * t * r * r
    }
    fn controller(self, deviation: f64, status: f64, position: HeatIndexSpace) -> Self {
        let time = self.clock_finger.load(Ordering::SeqCst);
        self.signals_s
            .send(HeatIndexSignal::Show(time, status))
            .unwrap();

        if deviation > 0.0 {
            if position.humidity > MINIMAL_HUMIDITY {
                self.signals_s.send(HeatIndexSignal::Dehumidify).unwrap();
            } else if position.temperature > MINIMAL_TEMPERATURE {
                self.signals_s.send(HeatIndexSignal::Cool).unwrap();
            }
        }

        self
    }
}

#[allow(clippy::assertions_on_constants)]
fn driver(hics: Hics) {
    fn control<'a>(hics: Hics, mut counter: usize) -> StreamProcessor<'a, (), usize> {
        control::MeasureOnDemand {
            dwell_time: Duration::from_millis(DWELL_TIME * TICK_LENGTH),
        }
        .execute(hics.clone(), EPSILON);

        counter += 1;

        StreamProcessor::Put(counter, Box::new(move || control(hics, counter)))
    }

    assert!(UNSAFE_BARRIER > SERVICE_BARRIER);

    // Here the runs of the hics are generated (lazily).
    let mut runs = control(hics, 0).eval(InfiniteList::constant(()));

    loop {
        thread::sleep(Duration::from_millis(CONTROL_PERIOD * TICK_LENGTH));

        // Here an interation of the hics is started.
        runs = runs.tail();
        let run_count = *runs.head();

        if run_count > SERVICE_BARRIER {
            if run_count > UNSAFE_BARRIER {
                break;
            }
            println!(
                "Warning: Service needed. ({} runs > {} runs)",
                run_count, SERVICE_BARRIER
            );
        }
    }
}

fn main() {
    fn print_heat_index_event(time: Time, heat_index: HeatIndex) {
        let red = |x| (x * 16.0 - 1350.0) as u8;
        let green = 25;
        let blue = |x| (1450.0 - x * 16.0) as u8;
        // The cryptic part of the following `format!(...)` is just an ANSI escape code to get
        // `format!({:.1}°F, heat_index)` with a truecolor R(ed)G(reen)B(lue) background.
        let degree = format!(
            "\x1b[48;2;{};{};{}m{:.1}°F\x1b[0m",
            red(heat_index),
            green,
            blue(heat_index),
            heat_index,
        );

        let to_minutes = |x| (x as f64 / 6.0) % 1440.0;
        let time = format!("6am plus {:.1} minutes", to_minutes(time));

        println!("Heat Index Event: {} at {}", degree, time);
    }

    let clock = Arc::new(AtomicU64::new(0));
    let clock_finger = Arc::clone(&clock);

    let thermohygrometer = Arc::new(Mutex::new(HeatIndexSpace {
        temperature: INITIAL_TEMPERATURE,
        humidity: INITIAL_HUMIDITY,
    }));
    let thermohygrometer_finger = Arc::clone(&thermohygrometer);

    let (signals_s, signals_r) = channel::unbounded();

    let hics = Hics {
        clock_finger,
        thermohygrometer_finger,
        signals_s,
    };

    let _clock_simulator = thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(TICK_LENGTH));

        clock.store(clock.load(Ordering::SeqCst) + TICK, Ordering::SeqCst);
    });

    let _thermohygrometer_simulator = thread::spawn(move || {
        let thermohygrometer_finger = Arc::clone(&thermohygrometer);

        // This is the climate effect actuated by the hics.
        let _actuator_simulator = thread::spawn(move || loop {
            let signal = signals_r.recv().unwrap();

            let mut position = thermohygrometer_finger.lock().unwrap();
            match signal {
                HeatIndexSignal::Show(time, heat_index) => print_heat_index_event(time, heat_index),
                HeatIndexSignal::Dehumidify => position.humidity -= ACTUATOR_DECREASE.humidity,
                HeatIndexSignal::Cool => position.temperature -= ACTUATOR_DECREASE.temperature,
            }
        });

        // This is the climate effect actuated by nature.
        loop {
            thread::sleep(Duration::from_millis(NATURAL_INCREASE_PERIOD * TICK_LENGTH));

            let mut position = thermohygrometer.lock().unwrap();
            position.humidity += NATURAL_INCREASE.humidity;
            position.temperature += NATURAL_INCREASE.temperature;
        }
    });

    driver(hics);
}
```

Finally, let us conclude with the key take-away:
rspl can encode generators and is hence suited for demand-driven programming.
However, it is not so clear why to use rspl to encode generators in general.
Indeed, this is also not quite what we wanted to show.
The idea is rather to show that rspl's stream processors can naturally incorporate demand-driven programming making them particularly useful to stream processing problems with demand-driven aspects.
It might be that the hics implemented here is not the best possible example to do so but the best we could come up with as of yet which is real-world enough while still being focused on the `put`-construct of rspl.

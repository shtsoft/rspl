mod control {
    use rspl::streams::infinite_lists::InfiniteList;
    use rspl::streams::Stream;
    use rspl::StreamProcessor;

    use std::thread;
    use std::time::Duration;

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

            let mut positions = cs.meter().eval(InfiniteList::constant(()));

            loop {
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

const TICK_LENGTH: u64 = 5; // in (real) millis
const TICK: u64 = 1; // shall represent 10 (real) seconds
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
    temperature: f64, // degree Fahrenheit
    humidity: f64,    // percent
}

enum HeatIndexSignal {
    Show(Time, HeatIndex),
    Dehumidify,
    Cool,
}

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
        // the body is the heat index formula from https://en.wikipedia.org/wiki/Heat_index
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

    let mut runs = control(hics, 0).eval(InfiniteList::constant(()));
    loop {
        thread::sleep(Duration::from_millis(CONTROL_PERIOD * TICK_LENGTH));

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
        // The cryptic part of the following `format!(...)` is just an ANSI escape code to get `format!({:.1}°F, heat_index)` with a truecolor R(ed)G(reen)B(lue) background.
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

        let _actuator_simulator = thread::spawn(move || loop {
            let signal = signals_r.recv().unwrap();

            let mut position = thermohygrometer_finger.lock().unwrap();
            match signal {
                HeatIndexSignal::Show(time, heat_index) => print_heat_index_event(time, heat_index),
                HeatIndexSignal::Dehumidify => position.humidity -= ACTUATOR_DECREASE.humidity,
                HeatIndexSignal::Cool => position.temperature -= ACTUATOR_DECREASE.temperature,
            }
        });

        loop {
            thread::sleep(Duration::from_millis(NATURAL_INCREASE_PERIOD * TICK_LENGTH));

            let mut position = thermohygrometer.lock().unwrap();
            position.humidity += NATURAL_INCREASE.humidity;
            position.temperature += NATURAL_INCREASE.temperature;
        }
    });

    driver(hics);
}

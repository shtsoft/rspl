mod pelican_machine {
    use rspl::combinators::map;
    use rspl::StreamProcessor;

    use std::fmt;

    const LENGTH_VEHICLES_GREEN_MIN: u64 = 10000;
    const LENGTH_VEHICLES_YELLOW: u64 = 1000;
    const LENGTH_PEDESTRIANS_GREEN: u64 = 10000;
    const LENGTH_BOTH_RED: u64 = 2000;

    #[derive(Copy, Clone)]
    pub enum Color {
        Red,
        Yellow,
        Green,
        Black,
    }

    #[derive(Copy, Clone)]
    pub enum Event {
        Push,
        Timeout,
        Exit,
    }

    pub enum Capability {
        SetVehicleLights(Color),
        SetPedestrianLights(Color),
        EmitTimeoutAfter(u64),
        UnexpectedTimeout(&'static str),
        CallForHelp,
        Break,
    }

    type State<'a> = StreamProcessor<'a, Event, Capability>;

    macro_rules! case_capture_transition {
        () => {
            StreamProcessor::get(|event| match event {
                Event::Push => push(),
                Event::Timeout => timeout(),
                Event::Exit => exit(),
            })
        };
    }

    macro_rules! ignore {
        () => {
            case_capture_transition!()
        };
    }

    macro_rules! mealy {
        ($transition:expr) => {
            $transition
        };

        ( $output:expr, $( $rest:expr ),+ ) => {
            StreamProcessor::put($output, mealy!($( $rest ),+))
        };
    }

    pub fn on<'a>() -> State<'a> {
        mealy!(
            Capability::SetPedestrianLights(Color::Red),
            Capability::SetVehicleLights(Color::Red),
            operational()
        )
    }

    fn operational<'a>() -> State<'a> {
        fn exit<'a>() -> State<'a> {
            off()
        }

        fn vehicles<'a>() -> State<'a> {
            fn vehicles_green_guard<'a>() -> State<'a> {
                fn push<'a>() -> State<'a> {
                    ignore!()
                }
                fn timeout<'a>() -> State<'a> {
                    vehicles_green()
                }

                case_capture_transition!()
            }

            fn vehicles_green<'a>() -> State<'a> {
                fn push<'a>() -> State<'a> {
                    vehicles_green_pushed()
                }
                fn timeout<'a>() -> State<'a> {
                    vehicles_green_timedout()
                }

                mealy!(
                    Capability::SetVehicleLights(Color::Green),
                    Capability::EmitTimeoutAfter(LENGTH_VEHICLES_GREEN_MIN),
                    case_capture_transition!()
                )
            }

            fn vehicles_green_pushed<'a>() -> State<'a> {
                fn push<'a>() -> State<'a> {
                    ignore!()
                }
                fn timeout<'a>() -> State<'a> {
                    vehicles_yellow()
                }

                case_capture_transition!()
            }

            fn vehicles_green_timedout<'a>() -> State<'a> {
                fn push<'a>() -> State<'a> {
                    vehicles_yellow()
                }
                fn timeout<'a>() -> State<'a> {
                    mealy!(
                        Capability::UnexpectedTimeout("state: vehicles_green_timedout"),
                        error()
                    )
                }

                case_capture_transition!()
            }

            fn vehicles_yellow<'a>() -> State<'a> {
                fn push<'a>() -> State<'a> {
                    ignore!()
                }
                fn timeout<'a>() -> State<'a> {
                    pedestrians()
                }

                mealy!(
                    Capability::SetVehicleLights(Color::Yellow),
                    Capability::EmitTimeoutAfter(LENGTH_VEHICLES_YELLOW),
                    case_capture_transition!()
                )
            }

            mealy!(
                Capability::SetPedestrianLights(Color::Red),
                Capability::EmitTimeoutAfter(LENGTH_BOTH_RED),
                vehicles_green_guard()
            )
        }

        fn pedestrians<'a>() -> State<'a> {
            fn pedestrians_green_guard<'a>() -> State<'a> {
                fn push<'a>() -> State<'a> {
                    ignore!()
                }
                fn timeout<'a>() -> State<'a> {
                    pedestrians_green()
                }

                case_capture_transition!()
            }

            fn pedestrians_green<'a>() -> State<'a> {
                fn push<'a>() -> State<'a> {
                    ignore!()
                }
                fn timeout<'a>() -> State<'a> {
                    vehicles()
                }

                mealy!(
                    Capability::SetPedestrianLights(Color::Green),
                    Capability::EmitTimeoutAfter(LENGTH_PEDESTRIANS_GREEN),
                    case_capture_transition!()
                )
            }

            mealy!(
                Capability::SetVehicleLights(Color::Red),
                Capability::EmitTimeoutAfter(LENGTH_BOTH_RED),
                pedestrians_green_guard()
            )
        }

        vehicles()
    }

    fn error<'a>() -> State<'a> {
        mealy!(
            Capability::SetPedestrianLights(Color::Red),
            Capability::SetVehicleLights(Color::Red),
            Capability::CallForHelp,
            map(|_| Capability::CallForHelp)
        )
    }

    fn off<'a>() -> State<'a> {
        mealy!(
            Capability::SetPedestrianLights(Color::Black),
            Capability::SetVehicleLights(Color::Black),
            Capability::Break,
            map(|_| Capability::Break)
        )
    }

    impl fmt::Display for Color {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                Self::Red => write!(f, "[RED]"),
                Self::Yellow => write!(f, "[YELLOW]"),
                Self::Green => write!(f, "[GREEN]"),
                Self::Black => write!(f, "[BLACK]"),
            }
        }
    }
}

use pelican_machine::{Capability, Event};

use rspl::streams::overeager_receivers::OvereagerReceiver;
use rspl::streams::Stream;

use std::thread;
use std::time::Duration;

use crossbeam::channel;
use crossbeam::channel::Sender;

enum Feedback {
    TimeoutAfter(u64),
}

fn driver<S>(events: S, tfeedback: &Sender<Feedback>)
where
    S: Stream<Event>,
{
    let mut capabilities = pelican_machine::on().eval(events);

    loop {
        match *capabilities.head() {
            Capability::SetVehicleLights(color) => println!("Vehicles: {}", color),
            Capability::SetPedestrianLights(color) => println!("Pedestrians: {}", color),
            Capability::EmitTimeoutAfter(length) => {
                tfeedback.send(Feedback::TimeoutAfter(length)).unwrap();
            }
            Capability::UnexpectedTimeout(message) => {
                eprintln!("log: unexpected timeout event ({})", message);
            }
            Capability::CallForHelp => {
                println!("Call for help!");
                break;
            }
            Capability::Break => break,
        };
        capabilities = capabilities.tail();
    }
}

fn event_emitter(length: u64, tevents: &Sender<Event>, event: Event) {
    thread::sleep(Duration::from_millis(length));
    tevents.send(event).unwrap();
}

fn main() {
    let (tevents, events) = OvereagerReceiver::channel(0, Event::Push);
    let (tfeedback, rfeedback) = channel::unbounded();

    let _input_simulator = thread::spawn(move || {
        let tevents_feedback = tevents.clone();

        let _feedback = thread::spawn(move || loop {
            match rfeedback.recv().unwrap() {
                Feedback::TimeoutAfter(length) => {
                    event_emitter(length, &tevents_feedback, Event::Timeout);
                }
            }
        });

        for _ in 0..10 {
            event_emitter(5000, &tevents, Event::Push);
            event_emitter(500, &tevents, Event::Push);
        }

        event_emitter(0, &tevents, Event::Exit);
    });

    driver(events, &tfeedback);
}

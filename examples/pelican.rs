// This example implements a PEdestrians-LIight-CONtrolled (pelican) crossing (almost) as suggested
// in
//
//     https://barrgroup.com/embedded-systems/how-to/introduction-hierarchical-state-machines
//
// There they describe an implementation of a pelican crossing as hierarchical state machine in C
// using techniques from OOP. This example adapts these techniques specificly to rspl and rust
// yielding a corresponding implementation in rust.
//
// The intention of the example is to demonstrate rspl's applicability in event-driven programming
// (with hierarchical state-machines, of course).
//
// Now that we have said what we are going to implement and why let us explain our techniques before
// presenting the code applying those techniques. To this end we split the following discussion in
// three parts. First, we introduce the general design pattern of encoding state machines in rspl.
// After that we address the problem of how to deal with the hierarchy aspect in rust. Both is done
// without refering to the specific example of the  pelican crossing. Then, third and last, before
// discussing the actual pelican code in place we briefly discuss the code's overall structure.
// So, first, rspl's stream processors can encode finite state machines in a way reminiscent of the
// (type) state pattern: every machine state is represented as stream processor which behaves on the
// input as the transition function of the machine in that state. This works because rspl is
// somewhat CPS-ish in the sense that every stream processor has the one for further processing
// encoded in its arguments.
// On top, stream processors have two properties making it possible to generalize the domain of the
// design pattern in a natural way. On the one hand, stream processors are able to output something
// and hence allow to even encode Mealy machines. On the other, hand stream processors can have side
// effects expanding the domain by effectful machines.
// Now, while it is nice to be able to encode effectful (Mealy) machines instead of only ordinary
// finite state machines, having the effect-implementations baked into the machine can be
// unfavorable for reasons of modularity and control (see monads and effect handlers). To mitigate
// those problems a possible approach is to reflect all possible effects into the stream processors
// output type. Then the effects become a stream of capabilities the machine requires the operator
// to provide in order to make progress. This improves modularity since the machine logic is
// seperated from its side effects. Moreover an operator decides how to operate the machine further
// when an effect occurs and thus makes control more explicit. Generally, the approach is in analogy
// to effect handlers.
// After having introduced the design pattern for state machines let us address the hierarchy aspect
// next. First, note that state machines have a deficiency w.r.t. the DRY-principle: if there are
// several - let us say n - states which transition on a certain input to a certain state then a
// naive implemenetation of the transition function repeats some code n times. For large state
// machines like in event-driven programming this can be a problem. The solution is to use
// hierarchical state machines which organize the states in a tree rather than a list.
// The n states from above would then have a common ancestor from which they can inherit the
// implementation of the shared behavior.
// Now rust's inheritance features are scarce and do not natively apply to the problem at discourse.
// But rust's local functions, shadowing properties and macros can do the hierarchy-trick in rspl's
// stream processor approach to state machines. Namely, because function definitions can be
// arbitrarily nested one can encode trees whose nodes hold a list of function definitions which are
// accessible from the node itself and its descendants. Furthermore nodes can effectively redefine
// functions already defined in ancestors by shadowing. This manifests itself in the fact that a
// call to a function refers to the first implementation encountered when walking up the tree (which
// coincides with the lexically closest definition). On the whole, using local functions in such a
// way reflects OOP with the usual inheritance though in a somewhat cumbersome manner. The point now
// is that the definition of states in our approach is via 'global elements' of stream processors,
// that is, a function with no arguments to the type of stream processors. So it is clear how to
// implement a hierarchy of states. But it is also clear how to share transition behavior: just
// define a function carrying the implementation to be shared within the appropriate node. This is
// particularly useful if the actual transition is a pattern match on the input of the machine where
// the match arms call the lexically closest function with the name of the pattern. That always the
// same 'case-capture'-match can then be abstracted away by a macro finally completing the hack for
// hierarchy.
// Last but not least let us have a look at the structure of our pelican crossing implementation.
// Essentially, it consists of three parts: a module encapsulating the machine logic of the pelican
// crossing, a driver-function responsible for providing the pelican crossing's capabilities and a
// main-function simulating the input and setting up the driver for the pelican crossing. Here, the
// machine logic mainly determines when it needs which capability like resetting the actual lights
// or feeding back an event by processing the event stream. The driver then executes the actions
// like resetting the lights or feeding back an event by need.

// Let us now walk through the code together.

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

    // This type defines the input alphabet of the machine. The `Push`-event is intended to trigger
    // a transition from a vehicles-green phase to a pedestrians-green phase while a
    // `Timeout`-event signals the machine that it has been in a certain state long enough. The
    // `Exit`-event tells the machine to shut down.
    #[derive(Copy, Clone)]
    pub enum Event {
        Push,
        Timeout,
        Exit,
    }

    // This type defines the output alphabet of the machine. The letters meaning is hopefully rather
    // self-explanatory. Perhaps just note that `EmitTimeoutAfter` is ought to be the capabilty of
    // the machine to feed back a `Timeout`-event to its own input.
    pub enum Capability {
        SetVehicleLights(Color),
        SetPedestrianLights(Color),
        EmitTimeoutAfter(u64),
        UnexpectedTimeout(&'static str),
        CallForHelp,
        Break,
    }

    type State<'a> = StreamProcessor<'a, Event, Capability>;

    // This is the aformentioned macro crucial for behavioral inheritance.
    macro_rules! case_capture_transition {
        () => {
            StreamProcessor::get(|event| match event {
                Event::Push => push(),
                Event::Timeout => timeout(),
                Event::Exit => exit(),
            })
        };
    }

    // This macro is really just another name for the previous one to make a later hack easier to
    // understand.
    macro_rules! ignore {
        () => {
            case_capture_transition!()
        };
    }

    // This macros sequences arbitrarily many put-stream processors. It is called `mealy` and not
    // `puts` (which would be a better name in general) because its arguments are supposed to make
    // up a list of capabilities ending with a transition.
    macro_rules! mealy {
        ($transition:expr) => {
            $transition
        };

        ( $output:expr, $( $rest:expr ),+ ) => {
            StreamProcessor::put($output, mealy!($( $rest ),+))
        };
    }

    // The rest of the module is essentially the definition of the states and transitions of the
    // machine.

    // This defines the initial (top-level) state.
    pub fn on<'a>() -> State<'a> {
        // This code means that when the capabilities have been handled from outside in that order
        // the next step is a transition to the operational state.
        mealy!(
            Capability::SetPedestrianLights(Color::Red),
            Capability::SetVehicleLights(Color::Red),
            operational()
        )
    }

    fn operational<'a>() -> State<'a> {
        // This defines the implementation for the `exit`-case shared by all substates of the
        // operational state.
        fn exit<'a>() -> State<'a> {
            off()
        }

        fn vehicles<'a>() -> State<'a> {
            fn vehicles_green_guard<'a>() -> State<'a> {
                fn push<'a>() -> State<'a> {
                    // This is the aformentioned hack to just ignore `push`-events in the
                    // `vehicles_green_guard`-state.
                    ignore!()
                }
                fn timeout<'a>() -> State<'a> {
                    vehicles_green()
                }

                // The match in the following macro correctly 'captures' the local functions from
                // the `vehicles_green_guard`-state and the `exit`-function defined in the
                // `operational`-state because the are the lexically closests.
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

    // The cryptic strings in this implementation are just ANSI escape codes.
    impl fmt::Display for Color {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                Self::Red => write!(f, "\x1b[41m  \x1b[0m"),
                Self::Yellow => write!(f, "\x1b[43m  \x1b[0m"),
                Self::Green => write!(f, "\x1b[42m  \x1b[0m"),
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
    // This code starts the machine.
    let mut capabilities = pelican_machine::on().eval(events);

    loop {
        // The following match provides the machine with capabilities. Most notably, it signals the
        // feedback loop to trigger a `Timeout`-event after some time.
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

fn main() {
    fn event_emitter(length: u64, tevents: &Sender<Event>, event: Event) {
        thread::sleep(Duration::from_millis(length));
        tevents.send(event).unwrap();
    }

    // The input/event stream is encoded as `OvereagerReceiver` which allows to easily implement
    // feeding back by essentially connecting `rfeedback` with `tevents`.
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

// The closing remarks shall just outline possible future work on event-driven programming with
// rspl: One thing which is conceivable is to develop some sort of domain specific language helping
// with implementing arbitrary hierarchical state machines. It could consist of only some clever
// macros but it could also be something more sophisticated like an uml-like language which is
// compiled to rust with rspl. Another - admittedly somewhat more fantastical - possibility is a
// library of generic rspl-encoded machines which can specialized by client code according to their
// needs by providing capabilities.

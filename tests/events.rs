use rspl::streams::overeager_receivers::OvereagerReceiver;
use rspl::streams::Stream;
use rspl::StreamProcessor;

use std::thread;
use std::time::Duration;

#[test]
fn test_events() {
    const RUNS_REFERENCE: usize = 9;

    const CHANNEL_SIZE: usize = 100;
    const INPUT_LATENCY: u64 = 100; // in millis

    #[derive(Copy, Clone)]
    enum Event {
        ShiftDepressed,
        ShiftReleased,
        Key(u8),
    }

    fn key_action(sign: char, c: u8) -> bool {
        if c == 0 {
            false
        } else {
            println!("{}{}", sign, c);
            true
        }
    }

    // the state where shift is released
    fn default<'a>() -> StreamProcessor<'a, Event, bool> {
        fn transition<'a>(event: Event) -> StreamProcessor<'a, Event, bool> {
            match event {
                Event::ShiftDepressed => StreamProcessor::put(true, shifted),
                Event::ShiftReleased => default(),
                Event::Key(c) => StreamProcessor::put(key_action('+', c), default),
            }
        }

        StreamProcessor::get(transition)
    }

    // the state where shift is depressed
    fn shifted<'a>() -> StreamProcessor<'a, Event, bool> {
        fn transition<'a>(event: Event) -> StreamProcessor<'a, Event, bool> {
            match event {
                Event::ShiftDepressed => shifted(),
                Event::ShiftReleased => StreamProcessor::put(true, default),
                Event::Key(c) => StreamProcessor::put(key_action('-', c), shifted),
            }
        }

        StreamProcessor::get(transition)
    }

    fn looping<S>(mut body: S) -> usize
    where
        S: Stream<bool>,
    {
        let mut n = 0;

        while *body.head() {
            body = body.tail();
            n += 1;
        }

        n
    }

    let (tevents, events) = OvereagerReceiver::channel(CHANNEL_SIZE, Event::ShiftReleased);

    let input_simulator = thread::spawn(move || {
        let events = [
            Event::Key(1),
            Event::ShiftDepressed,
            Event::Key(1),
            Event::Key(5),
            Event::ShiftReleased,
            Event::Key(5),
            Event::Key(7),
            Event::ShiftReleased,
            Event::Key(3),
            Event::ShiftDepressed,
            Event::Key(0),
        ];

        for event in events {
            thread::sleep(Duration::from_millis(INPUT_LATENCY));
            tevents.send(event).unwrap();
        }
    });

    let runs = looping(default().eval(events));

    input_simulator.join().unwrap();

    assert_eq!(runs, RUNS_REFERENCE);
}

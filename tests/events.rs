use rspl::streams::overeager_receivers::OvereagerReceiver;
use rspl::streams::Stream;
use rspl::StreamProcessor;

use std::thread;
use std::time::Duration;

#[test]
fn test_events() {
    #[derive(Copy, Clone)]
    enum Event {
        ShiftDepressed,
        ShiftReleased,
        Key(u8),
    }

    struct Initial<'a, A, B> {
        state: StreamProcessor<'a, A, B>,
        event: Event,
    }

    fn key_action(sign: char, c: u8) -> bool {
        if c == 0 {
            false
        } else {
            println!("{}{}", sign, c);
            true
        }
    }

    fn default<'a>() -> StreamProcessor<'a, Event, bool> {
        fn transition<'a>(event: Event) -> StreamProcessor<'a, Event, bool> {
            match event {
                Event::ShiftDepressed => StreamProcessor::put(true, shifted()),
                Event::ShiftReleased => default(),
                Event::Key(c) => StreamProcessor::put(key_action('+', c), default()),
            }
        }

        StreamProcessor::get(transition)
    }

    fn shifted<'a>() -> StreamProcessor<'a, Event, bool> {
        fn transition<'a>(event: Event) -> StreamProcessor<'a, Event, bool> {
            match event {
                Event::ShiftDepressed => shifted(),
                Event::ShiftReleased => StreamProcessor::put(true, default()),
                Event::Key(c) => StreamProcessor::put(key_action('-', c), shifted()),
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

    let initial = Initial {
        state: default(),
        event: Event::ShiftReleased,
    };

    let (tevents, events) = OvereagerReceiver::channel(100, initial.event);

    let input_simulator = thread::spawn(move || {
        let wait = || thread::sleep(Duration::from_millis(100));

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
            Event::Key(0),
        ];

        for event in events {
            tevents.send(event).unwrap();
            wait();
        }
    });

    assert_eq!(looping(initial.state.eval(events)), 9);

    input_simulator.join().unwrap();
}

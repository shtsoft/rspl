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

    fn key_action(sign: char, c: u8) -> bool {
        if c == 0 {
            false
        } else {
            println!("{}{}", sign, c);
            true
        }
    }

    fn default() -> StreamProcessor<Event, bool> {
        fn transition(event: Event) -> StreamProcessor<Event, bool> {
            match event {
                Event::ShiftDepressed => StreamProcessor::Put(true, Box::new(shifted)),
                Event::ShiftReleased => default(),
                Event::Key(c) => StreamProcessor::Put(key_action('+', c), Box::new(default)),
            }
        }

        StreamProcessor::Get(Box::new(transition))
    }

    fn shifted() -> StreamProcessor<Event, bool> {
        fn transition(event: Event) -> StreamProcessor<Event, bool> {
            match event {
                Event::ShiftDepressed => shifted(),
                Event::ShiftReleased => StreamProcessor::Put(true, Box::new(default)),
                Event::Key(c) => StreamProcessor::Put(key_action('-', c), Box::new(shifted)),
            }
        }

        StreamProcessor::Get(Box::new(transition))
    }

    let (tx, events) = OvereagerReceiver::channel(100, Event::ShiftReleased);

    let input_simulator = thread::spawn(move || {
        let wait = || thread::sleep(Duration::from_millis(100));
        tx.send(Event::Key(1)).unwrap();
        wait();
        tx.send(Event::ShiftDepressed).unwrap();
        wait();
        tx.send(Event::Key(1)).unwrap();
        wait();
        tx.send(Event::Key(5)).unwrap();
        wait();
        tx.send(Event::ShiftReleased).unwrap();
        wait();
        tx.send(Event::Key(5)).unwrap();
        wait();
        tx.send(Event::Key(7)).unwrap();
        wait();
        tx.send(Event::ShiftReleased).unwrap();
        wait();
        tx.send(Event::Key(3)).unwrap();
        wait();
        tx.send(Event::ShiftDepressed).unwrap();
        wait();
        tx.send(Event::Key(0)).unwrap();
        tx.send(Event::Key(0)).unwrap();
    });

    looping(default().eval(events));

    input_simulator.join().unwrap();
}

fn looping<S>(mut body: S)
where
    S: Stream<bool>,
{
    while body.head() {
        body = body.tail();
    }
}

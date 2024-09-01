use rdev::{Event, EventType, Key};
use std::process::exit;
pub struct Events {
    capture: Key,
    pause: Key,
}

pub enum SREvent { // screen recording events
    Capture,
    Pause,
}

impl Events {
    pub fn init() -> Events {
        Events {
            capture: Key::F10,
            pause: Key::F11,
        }
    }

    pub fn update(&mut self, event: SREvent, new_key: Key) -> bool {
        match event {
            SREvent::Capture => {
                self.capture = new_key;
                true
            }
            SREvent::Pause => {
                self.pause = new_key;
                true
            }
            _ => false
        }
    }

    pub fn handle(&self, event: Event) -> Option<Event> {
        // Match on the event type
        match event.event_type {
            // If the event is a KeyPress and the key is F10
            EventType::KeyPress(key) => {
                self.handle_key_press(key, event)
            }
            _ => Some(event),  // Return Some(event) to propagate the event
        }
    }

    fn handle_key_press(&self, key: Key, event: Event) -> Option<Event> {
        if key == self.capture {
            // Call function to capture screens and save them
            return None  // Return None to consume the event
        }

        if key == self.pause {
            exit(0);
        }

        Some(event)
    }
}
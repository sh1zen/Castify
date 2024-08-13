use crate::capture;
use rdev::{Event, EventType, Key};
use std::process::exit;

pub struct Events {
    pause_resume: Key,
    end_session: Key,
    blank_screen: Key,
}

pub enum SREvent {
    // screen recording events
    PauseResume,
    EndSession,
    BlankScreen,
}

impl Events {
    pub fn init() -> Self {
        Self {
            pause_resume: Key::F10,
            end_session: Key::F11,
            blank_screen: Key::ControlLeft,
        }
    }

    pub fn update(&mut self, event: SREvent, new_key: Key) -> bool {
        match event {
            SREvent::PauseResume => {
                self.pause_resume = new_key;
                true
            }
            SREvent::EndSession => {
                self.end_session = new_key;
                true
            }
            SREvent::BlankScreen => {
                self.blank_screen = new_key;
                true
            }
            _ => false,
        }
    }

    pub fn handle(&self, event: Event) -> Option<Event> {
        println!("{:?}", event.name);

        // Match on the event type
        match event.event_type {
            // If the event is a KeyPress and the key is F10
            EventType::KeyPress(key) => self.handle_key_press(key, event),
            _ => Some(event), // Return Some(event) to propagate the event
        }
    }

    fn handle_key_press(&self, key: Key, event: Event) -> Option<Event> {
        if key == self.pause_resume {
            // Call function to capture screens and save them
            let vcap = capture::Capture::new();
            vcap.screen(0);
            return None; // Return None to consume the event
        }

        if key == self.end_session {
            // todo end session
            exit(0);
            return None;
        }

        if key == self.blank_screen {
            // todo blank_screen
            return None;
        }

        return Some(event);
    }
}

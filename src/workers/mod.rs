pub mod caster;
pub mod save_stream;
pub mod receiver;
pub mod key_listener;
pub mod tray_icon;

pub trait WorkerClose {
    fn close(&mut self);
}
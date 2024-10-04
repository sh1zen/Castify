pub mod caster;
pub mod save_stream;
pub mod sos;
pub mod client;

pub trait WorkerClose {
    fn close(&mut self);
}
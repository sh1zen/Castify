pub mod caster;
pub mod save_stream;
pub mod receiver;

pub trait WorkerClose {
    fn close(&mut self);
}
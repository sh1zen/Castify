use crate::workers::save_stream::SaveStream;
use crate::workers::WorkerClose;

#[derive(Debug, Clone)]
pub struct Client {
    save_stream: Option<SaveStream>
}

impl WorkerClose for Client {
    fn close(&mut self) {}
}

impl Client {
    pub fn save_stream(&mut self){
        self.save_stream = Some(SaveStream::new());
    }
}
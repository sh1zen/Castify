use crate::utils::sos::SignalOfStop;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::Mutex;


static INSTANCE: Lazy<Arc<Mutex<SignalOfStop>>> = Lazy::new(|| Arc::new(Mutex::new(SignalOfStop::new())));

pub fn get_instance() -> Arc<Mutex<SignalOfStop>> {
    INSTANCE.clone()
}
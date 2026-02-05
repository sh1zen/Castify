use std::sync::Arc;
use log::{info, error};
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct SaveStream {
    saver_channel: Arc<Mutex<Receiver<Vec<u8>>>>,
    is_saving: Arc<Mutex<bool>>,
    /// Segnale per interrompere il loop di scrittura
    stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl SaveStream {
    pub fn new(saver_channel: Arc<Mutex<Receiver<Vec<u8>>>>) -> Self {
        Self {
            saver_channel,
            is_saving: Arc::new(Mutex::new(false)),
            stop_tx: None,
        }
    }

    pub fn start(&mut self, path: String) {
        *self.is_saving.blocking_lock() = true;

        let is_saving = Arc::clone(&self.is_saving);
        let saver_channel = Arc::clone(&self.saver_channel);
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        self.stop_tx = Some(stop_tx);

        tokio::spawn(async move {
            // Apri il file di output
            let file = match tokio::fs::File::create(&path).await {
                Ok(f) => f,
                Err(e) => {
                    error!("Failed to create save file '{}': {}", path, e);
                    *is_saving.lock().await = false;
                    return;
                }
            };

            let mut writer = tokio::io::BufWriter::new(file);
            let mut stop_rx = stop_rx;

            info!("SaveStream started → {}", path);

            loop {
                let data = {
                    let mut rx = saver_channel.lock().await;
                    tokio::select! {
                        frame = rx.recv() => frame,
                        _ = &mut stop_rx => {
                            info!("SaveStream stop signal received");
                            break;
                        }
                    }
                };

                let Some(data) = data else {
                    info!("Saver channel closed");
                    break;
                };

                if !*is_saving.lock().await {
                    break;
                }

                if let Err(e) = writer.write_all(&data).await {
                    error!("Write error: {}", e);
                    break;
                }
            }

            // Flush e chiudi
            if let Err(e) = writer.flush().await {
                error!("Flush error on save file: {}", e);
            }

            *is_saving.lock().await = false;
            info!("SaveStream finished → {}", path);
        });
    }

    pub fn stop(&mut self) {
        // Invia il segnale di stop al task
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }

    pub fn is_saving(&self) -> bool {
        *self.is_saving.blocking_lock()
    }
}
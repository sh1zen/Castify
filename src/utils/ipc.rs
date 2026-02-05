use crate::config::app_id;
use crate::gui::common::messages::AppEvent;
use iced::{
    futures::{SinkExt, Stream},
    stream,
};
use interprocess::local_socket::{
    GenericNamespaced, ListenerOptions, ToNsName, traits::tokio::Listener,
};

pub fn ipc() -> impl Stream<Item = AppEvent> {
    stream::channel(
        10,
        |mut output: iced::futures::channel::mpsc::Sender<AppEvent>| async move {
            let name = app_id().to_ns_name::<GenericNamespaced>().unwrap();

            let listener_opts = ListenerOptions::new().name(name);

            if let Ok(listener) = listener_opts.create_tokio() {
                loop {
                    if let Ok(_stream) = listener.accept().await {
                        output.send(AppEvent::OpenMainWindow).await.unwrap();
                    }
                }
            }
        },
    )
}

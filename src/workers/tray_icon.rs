use iced::advanced::graphics::image::load;
use iced::advanced::image::Handle;
use iced::{
    futures::{SinkExt, Stream},
    stream,
};
use std::thread::spawn;
use tokio::sync::mpsc;
use tray_icon::{
    Icon,
    MouseButton::Left,
    TrayIcon, TrayIconAttributes, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

use crate::assets::ICON_BYTES;
use crate::config::app_name;
use crate::gui::common::messages::AppEvent;

fn spawn_forwarder<T, F>(sender: mpsc::Sender<T>, mut recv: F)
where
    T: Send + 'static,
    F: FnMut() -> Option<T> + Send + 'static,
{
    spawn(move || {
        loop {
            if let Some(event) = recv() {
                sender.blocking_send(event).unwrap()
            }
        }
    });
}

pub fn tray_icon() -> anyhow::Result<TrayIcon> {
    let icon_image = load(&Handle::from_bytes(ICON_BYTES))?;
    let icon = Icon::from_rgba(icon_image.to_vec(), icon_image.width(), icon_image.height())?;

    #[cfg(target_os = "linux")]
    gtk::init().unwrap();

    let menu = Menu::new();
    menu.append_items(&[
        &MenuItem::with_id("open", "Open", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("exit", "Exit", true, None),
    ])
    .expect("Tray icon set up failed.");

    Ok(TrayIcon::new(TrayIconAttributes {
        tooltip: Some(app_name()),
        menu: Some(Box::new(menu)),
        icon: Some(icon),
        icon_is_template: false,
        menu_on_left_click: false,
        title: Some(app_name()),
        ..Default::default()
    })?)
}

pub fn tray_icon_listener() -> impl Stream<Item = AppEvent> {
    stream::channel(
        16,
        |mut output: iced::futures::channel::mpsc::Sender<AppEvent>| async move {
            let (sender, mut receiver) = mpsc::channel(16);
            spawn_forwarder(sender, || TrayIconEvent::receiver().recv().ok());

            while let Some(event) = receiver.recv().await {
                if let TrayIconEvent::DoubleClick { button: Left, .. } = event {
                    output.send(AppEvent::OpenMainWindow).await.unwrap();
                }
            }
        },
    )
}

pub fn tray_menu_listener() -> impl Stream<Item = AppEvent> {
    stream::channel(
        16,
        |mut output: iced::futures::channel::mpsc::Sender<AppEvent>| async move {
            let (sender, mut receiver) = mpsc::channel(16);
            spawn_forwarder(sender, || MenuEvent::receiver().recv().ok());

            while let Some(MenuEvent { id: MenuId(id) }) = receiver.recv().await {
                let event = match id.as_str() {
                    "open" => AppEvent::OpenMainWindow,
                    "exit" => AppEvent::ExitApp,
                    _ => AppEvent::Ignore,
                };
                output.send(event).await.unwrap()
            }
        },
    )
}

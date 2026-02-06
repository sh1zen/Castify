use iced::{
    futures::{SinkExt, Stream},
    stream,
};
use std::thread::spawn;
use iced::advanced::graphics::image::load;
use iced::advanced::image::Handle;
use tokio::sync::mpsc;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    Icon,
    MouseButton::Left,
    TrayIcon, TrayIconAttributes, TrayIconEvent,
};

use crate::assets::ICON_BYTES;
use crate::config::{app_name};
use crate::gui::common::messages::AppEvent;

pub fn tray_icon() ->  anyhow::Result<TrayIcon> {
    let icon_image = load(&Handle::from_bytes(ICON_BYTES))?;
    let icon = Icon::from_rgba(icon_image.to_vec(), icon_image.width(), icon_image.height())?;

    #[cfg(target_os = "linux")]
    gtk::init().unwrap();

    let menu = Menu::new();
    menu.append_items(&[
        &MenuItem::with_id("open", "Open", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("exit", "Exit", true, None),
    ]).expect("Tray icon set up failed.");

    Ok(
        TrayIcon::new(TrayIconAttributes {
            tooltip: Some(app_name()),
            menu: Some(Box::new(menu)),
            icon: Some(icon),
            icon_is_template: false,
            menu_on_left_click: false,
            title: Some(app_name()),
            ..Default::default()
        })?
    )
}

pub fn tray_icon_listener() -> impl Stream<Item=AppEvent> {
    stream::channel(16, |mut output: iced::futures::channel::mpsc::Sender<AppEvent>| async move {
        let (sender, mut reciever) = mpsc::channel(16);

        spawn(move || loop {
            if let Ok(event) = TrayIconEvent::receiver().recv() {
                sender.blocking_send(event).unwrap()
            }
        });

        loop {
            if let Some(TrayIconEvent::DoubleClick { button: Left, .. }) = reciever.recv().await {
                output.send(AppEvent::OpenMainWindow).await.unwrap();
            }
        }
    })
}

pub fn tray_menu_listener() -> impl Stream<Item=AppEvent> {
    stream::channel(16, |mut output: iced::futures::channel::mpsc::Sender<AppEvent>| async move {
        let (sender, mut reciever) = mpsc::channel(16);

        spawn(move || loop {
            if let Ok(event) = MenuEvent::receiver().recv() {
                sender.blocking_send(event).unwrap()
            }
        });

        loop {
            if let Some(MenuEvent { id: MenuId(id) }) = reciever.recv().await {
                let event = match id.as_str() {
                    "open" => AppEvent::OpenMainWindow,
                    "exit" => AppEvent::ExitApp,
                    _ => AppEvent::Ignore,
                };
                output.send(event).await.unwrap()
            }
        }
    })
}
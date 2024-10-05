use iced::{
    advanced::graphics::image::image_rs::load_from_memory,
    futures::{SinkExt, Stream},
    stream,
};
use std::thread::spawn;
use tokio::sync::mpsc;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    Icon,
    MouseButton::Left,
    TrayIcon, TrayIconAttributes, TrayIconEvent,
};

use crate::assets::{APP_NAME, ICON_BYTES};
use crate::gui::common::messages::AppEvent;

pub fn tray_icon() -> TrayIcon {
    let icon_image = load_from_memory(ICON_BYTES).unwrap();
    let (width, height) = (icon_image.width(), icon_image.height());
    let icon = Icon::from_rgba(icon_image.into_bytes(), width, height).unwrap();

    #[cfg(target_os = "linux")]
    gtk::init().unwrap();

    let menu = Menu::new();
    menu.append_items(&[
        &MenuItem::with_id("open", "Open", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("exit", "Exit", true, None),
    ]).expect("Tray icon set up failed.");

    TrayIcon::new(TrayIconAttributes {
        tooltip: Some(APP_NAME.to_string()),
        menu: Some(Box::new(menu)),
        icon: Some(icon),
        icon_is_template: false,
        menu_on_left_click: false,
        title: Some(APP_NAME.to_string()),
        ..Default::default()
    }).unwrap()
}

pub fn tray_icon_listener() -> impl Stream<Item=AppEvent> {
    stream::channel(16, |mut output| async move {
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
    stream::channel(16, |mut output| async move {
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
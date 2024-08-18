use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use iced::widget::{Container, Row};
use iced::{Alignment, Length};
use std::borrow::BorrowMut;
use tokio::sync::mpsc::{Receiver, Sender};
use xcap::image::RgbaImage;

pub struct CasterOptions {
    pub channel: (Sender<RgbaImage>, Receiver<RgbaImage>),
    pub streaming: bool,
}

impl CasterOptions {
    pub fn new() -> Self {
        let (mut tx, mut rx) = tokio::sync::mpsc::channel(1);
        Self {
            channel: (tx, rx),
            streaming: false,
        }
    }

    pub fn get_tx(&mut self) -> &mut Sender<RgbaImage> {
        self.build_channel();
        self.channel.0.borrow_mut()
    }

    pub fn get_rx(&mut self) -> &mut Receiver<RgbaImage> {
        self.build_channel();
        self.channel.1.borrow_mut()
    }

    fn build_channel(&mut self) {
        /*if self.channel.is_none(){
            self.channel = Some(tokio::sync::mpsc::channel(1));
        }*/
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Rec,
    Pause,
}

pub fn caster_page(app: &App) -> Container<appMessage, StyleType> {
    let action = if app.caster_opt.lock().unwrap().streaming {
        FilledButton::new("Pause").icon(Icon::Pause).build().on_press(
            appMessage::Caster(Message::Pause)
        )
    } else {
        FilledButton::new("Rec").icon(Icon::Video).build().on_press(
            appMessage::Caster(Message::Rec)
        )
    };

    let content = Row::new()
        .align_items(iced::Alignment::Center).spacing(10)
        .push(action)
        .height(400)
        .align_items(Alignment::Center);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
}
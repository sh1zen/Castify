use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
use crate::gui::types::messages::{Message as appMessage, Message};
use iced::advanced::widget::Text;
use iced::advanced::Widget;
use iced::widget::{Column, Container, Space, TextInput};
use std::hash::Hash;

use crate::gui::theme::container::ContainerType;
use iced_aw::widgets::Modal;

#[derive(Debug, Clone)]
pub(crate) struct Interaction {
    pub text: String,
    pub p_type: PopupType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PopupType {
    IP
}

pub fn show_popup<'a>(app: &'a App, body: Container<'a, appMessage, StyleType>) -> Container<'a, appMessage, StyleType> {

    if app.show_popup.is_none() {
        return Container::new(Space::new(0, 0));
    }

    let popup_type = app.show_popup.clone().unwrap();

    let content = match popup_type {
        PopupType::IP => {
            ip_popup(app, body)
        }
    };

    Container::new(content)
}

fn ip_popup<'a>(app: &'a App, body: Container<'a, appMessage, StyleType>) -> iced_aw::Modal<'a, appMessage, StyleType> {
    let mut text_input = &"".to_string();

    if app.popup_msg.contains_key(&PopupType::IP) {
        text_input = app.popup_msg.get(&PopupType::IP).unwrap()
    }

    let input = TextInput::new("192.168.1.1", text_input)
        .on_input(move |new_value| {
            appMessage::PopupMessage(Interaction { text: new_value, p_type: PopupType::IP })
        })
        .padding([8, 12])
        .id(iced::widget::text_input::Id::new("ip_text_input"));

    let mut button = FilledButton::new("Connect").build();

    if !text_input.is_empty() {
        button = button.on_press(appMessage::Ignore/*appMessage::ConnectToCaster(entered_ip.clone())*/);
    }

    let content = Column::new()
        .spacing(10)
        .padding(20)
        .push(Text::new("Enter Receiver IP Address:").size(20))
        .push(input)
        .push(button);

    Modal::new(
        body,
        Some(
            Container::new(content.width(500).height(300)).style(ContainerType::Modal)
        )
    )
}
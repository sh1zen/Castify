use crate::config::Config;
use crate::gui::common::anybox::AnyBox;
use crate::gui::common::icons::Icon;
use crate::gui::components::awmodal::GuiInterface;
use crate::gui::components::button::{Dimensions, IconButton};
use crate::gui::widget::{Column, Element, IcedButtonExt, Row, TextInput};
use crate::gui::windows::main::MainWindowEvent;

pub struct IPModal {
    ip: String,
}

impl IPModal {
    pub fn new() -> Self {
        IPModal {
            ip: String::new()
        }
    }

    fn parse_ip(ip: String) -> String {
        ip.chars().filter(|c| ".0123456789:".contains(*c)).collect()
    }
}

impl GuiInterface for IPModal {
    type Message = MainWindowEvent;

    fn title(&self) -> String {
        String::from("Enter Receiver IP Address:")
    }

    fn update(&mut self, value: AnyBox, _config: &Config) {
        self.ip = value.downcast::<String>().unwrap().clone();
    }

    fn view<'a, 'b>(&'a self, _config: &Config) -> Element<'b, Self::Message>
    where
        'b: 'a,
        Self::Message: Clone + 'b,
    {
        let input = TextInput::new("192.168.1.2", &self.ip)
            .on_input(move |new_value| {
                MainWindowEvent::PopupMessage(
                    AnyBox::new(IPModal::parse_ip(new_value))
                )
            })
            .padding([8, 12]);

        let ip = self.ip.clone();

        let button =
            IconButton::new().label("Connect").icon(Icon::Connect).dim(Dimensions::Large)
                .build()
                .on_press_if(!ip.is_empty(), move || MainWindowEvent::ConnectToCaster(ip.clone()));

        Column::new()
            .spacing(12)
            .push(input)
            .push(
                Row::new().spacing(12)
                    .push(button)
                    .push(
                        IconButton::new().label("Manual").icon(Icon::Sync).dim(Dimensions::Large).build().on_press(MainWindowEvent::ShowSDP)
                    )
                    .push(
                        IconButton::new().label("Auto").icon(Icon::Auto).build().on_press(MainWindowEvent::ConnectToCaster("auto".parse().unwrap()))
                    )
            )
            .push(
                IconButton::new().label("Home").icon(Icon::Home).build().on_press(MainWindowEvent::Home)
            )
            .into()
    }
}
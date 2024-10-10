use crate::config::{Config, Mode};
use crate::gui::common::icons::Icon;
use crate::gui::components::custom::IconButton;
use crate::gui::style::container::ContainerType;
use crate::gui::video::{Video, VideoPlayer};
use crate::gui::widget::{Column, Container, Element, Row, Stack};
use crate::gui::windows::main::MainWindowEvent;
use iced::widget::{Space, Text};
use iced::{Alignment, Length};
use iced_core::{alignment, Padding};
use crate::assets::FONT_FAMILY_BOLD;
use crate::gui::style::text::TextType;

pub fn client_page<'a, 'b>(video: &'b Video, config: &Config) -> Element<'a, MainWindowEvent>
where
    'b: 'a,
{
    let Some(Mode::Receiver(client)) = &config.mode else {
        unreachable!("Mode must be Receiver here")
    };

    let actions = Row::new()
        .align_y(Alignment::Center).spacing(10)
        .push(
            if client.is_saving() {
                IconButton::new(Some(String::from("Stop")))
                    .icon(Icon::Save)
                    .build()
                    .on_press(MainWindowEvent::SaveCaptureStop)
            } else {
                IconButton::new(Some(String::from("Save")))
                    .icon(Icon::Download)
                    .build()
                    .on_press(MainWindowEvent::SaveCapture)
            }
        )
        .push(
            IconButton::new(Some(String::from("Exit")))
                .icon(Icon::Stop)
                .build()
                .on_press(MainWindowEvent::ExitApp)
        )
        .align_y(Alignment::Center);

    let video = {
        let client = match &config.mode {
            Some(Mode::Receiver(c)) => c,
            _ => unreachable!("Mode must be Client here"),
        };

        let video = if client.is_streaming() {
            Container::new(VideoPlayer::new(video))
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .class(ContainerType::Video)
        } else {
            Container::new(Text::new("Waiting for the Caster...").font(FONT_FAMILY_BOLD).size(22.0).class(TextType::White))
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .class(ContainerType::Video)
        };

        video
    };


    let content = Column::new()
        .spacing(20)
        .push(video)
        .push(actions)
        .align_x(Alignment::Center);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 10.0,
            left: 0.0,
        })
        .into()
}
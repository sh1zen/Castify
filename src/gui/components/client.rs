use crate::gui::common::icons::Icon;
use crate::gui::components::buttons::IconButton;
use crate::gui::style::container::ContainerType;
use crate::gui::video::{Video, VideoPlayer};
use crate::gui::widget::{Column, Container, Row};
use crate::windows::main::MainWindowEvent;
use crate::workers;
use iced::{Alignment, Length};
use iced_core::{alignment, Padding};

pub fn client_page<'a, 'b>(video: &'b Video) -> Container<'a, MainWindowEvent>
where 'b: 'a
{
    let actions = Row::new()
        .align_y(Alignment::Center).spacing(10)
        .push(
            if workers::save_stream::get_instance().lock().unwrap().is_saving {
                IconButton::new("Stop")
                    .icon(Icon::Save)
                    .build()
                    .on_press(MainWindowEvent::SaveCaptureStop)
            } else {
                IconButton::new("Save")
                    .icon(Icon::Download)
                    .build()
                    .on_press(MainWindowEvent::SaveCapture)
            }
        )
        .push(
            IconButton::new("Exit")
                .icon(Icon::Stop)
                .build()
                .on_press(MainWindowEvent::ExitApp)
        )
        .align_y(Alignment::Center);

    let video = Container::new(VideoPlayer::new(video))
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .class(ContainerType::Video);

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
}
use crate::assets::FONT_FAMILY_BOLD;
use crate::config::Config;
use crate::gui::common::messages::AppEvent;
use crate::gui::components::AreaSelector;
use crate::gui::style::container::ContainerType;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::{horizontal_space, vertical_space, Canvas, Column, Container, Element, Row, Stack, Text};
use crate::windows::GuiWindow;
use iced_core::window::Id;
use iced_core::Alignment::Center;
use iced_core::Length::Fill;
use iced_runtime::Task;

pub struct AnnotationWindow {
}

#[derive(Debug, Clone)]
pub enum AnnotationWindowEvent {

}


impl GuiWindow for AnnotationWindow {
    type Message = AnnotationWindowEvent;

    fn new() -> Self {
        AnnotationWindow {

        }
    }

    fn title(&self) -> String {
        String::from("")
    }

    fn update(&mut self, id: Id, message: Self::Message, config: &mut Config) -> Task<AppEvent> {
        Task::none()
    }

    fn view(&self, config: &Config) -> Element<Self::Message> {
        Stack::new()
            .push(
                Canvas::new(
                    AreaSelector::new()

                )
                    .width(Fill)
                    .height(Fill))
            .push(
                Row::new()
                    .push(horizontal_space().height(5))
                    .push(
                        Column::new()
                            .push(vertical_space().width(Fill))
                            .push(
                                Container::new(
                                    Text::new("text_hint").font(FONT_FAMILY_BOLD).size(15).align_x(Center).align_y(Center)
                                )
                                    .class(ContainerType::Standard)
                                    .padding(10)
                                    .align_x(Center)
                                    .align_x(Center)
                            )
                            .push(vertical_space().width(Fill))
                    )
            )
            .height(Fill)
            .width(Fill)
            .into()
    }

    fn theme(&self) -> StyleType {
        StyleType::Transparent
    }
}
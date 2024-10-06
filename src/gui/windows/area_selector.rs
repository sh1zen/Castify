use crate::assets::{APP_NAME, FONT_FAMILY_BOLD};
use crate::config::Config;
use crate::gui::common::datastructure::ScreenRect;
use crate::gui::common::messages::AppEvent;
use crate::gui::components::AreaSelector;
use crate::gui::style::container::ContainerType;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::{horizontal_space, vertical_space, Canvas, Column, Container, Element, Row, Stack, Text};
use crate::gui::windows::GuiWindow;
use iced_core::window::Id;
use iced_core::Alignment::Center;
use iced_core::Length::Fill;
use iced_runtime::Task;

pub struct ASWindow {
    area: Option<ScreenRect>,
    invalid: bool,
}

#[derive(Debug, Clone)]
pub enum ASWindowEvent {
    AreaSelected(ScreenRect),
    AreaAbort,
    Invalid,
    ExitAbort,
    ExitValid,
}


impl GuiWindow for ASWindow {
    type Message = ASWindowEvent;

    fn new() -> Self {
        ASWindow {
            area: None,
            invalid: false,
        }
    }

    fn title(&self) -> String {
        String::from(APP_NAME) + "::AreaSelection"
    }

    fn update(&mut self, id: Id, message: Self::Message, _config: &mut Config) -> Task<AppEvent> {
        match message {
            ASWindowEvent::AreaSelected(area) => {
                self.invalid = false;
                self.area = Some(area);
                Task::none()
            }
            ASWindowEvent::AreaAbort => {
                self.invalid = false;
                self.area = None;
                Task::none()
            }
            ASWindowEvent::Invalid => {
                self.invalid = true;
                self.area = None;
                Task::none()
            }
            ASWindowEvent::ExitAbort => {
                Task::done(AppEvent::CloseWindow(id))
            }
            ASWindowEvent::ExitValid => {
                if !self.invalid && self.area.is_some() {
                    Task::batch(vec![
                        Task::done(AppEvent::AreaSelected(self.area.take().unwrap())),
                        Task::done(AppEvent::CloseWindow(id))
                    ])
                } else {
                    Task::done(AppEvent::CloseWindow(id))
                }
            }
        }
    }

    fn view(&self, _config: &Config) -> Element<Self::Message> {
        let text_hint = if self.invalid {

            "Invalid selection"
        } else if let Some(_) = self.area {
            "Enter to Confirm | Click to Reset"
        } else {
            "Esc to Cancel"
        };

        Stack::new()
            .push(
                Canvas::new(
                    AreaSelector::new()
                        .on_release_rect(|rect| {
                            if rect.height < 50.0 || rect.width == 50.0 {
                                ASWindowEvent::Invalid
                            } else if rect.height < 1.0 || rect.width == 1.0 {
                                ASWindowEvent::AreaAbort
                            } else {
                                ASWindowEvent::AreaSelected(rect)
                            }
                        })
                        .on_esc(ASWindowEvent::ExitAbort)
                        .on_confirm(ASWindowEvent::ExitValid)
                )
                    .width(Fill)
                    .height(Fill))
            .push(
                Column::new()
                    .push(vertical_space().height(5))
                    .push(
                        Row::new()
                            .push(horizontal_space().width(Fill))
                            .push(
                                Container::new(
                                    Text::new(text_hint).font(FONT_FAMILY_BOLD).size(15).align_x(Center).align_y(Center)
                                )
                                    .class(ContainerType::Standard)
                                    .padding(10)
                                    .align_x(Center)
                                    .align_x(Center)
                            )
                            .push(horizontal_space().width(Fill))
                    )
            )
            .height(Fill)
            .width(Fill)
            .into()
    }

    fn theme(&self) -> StyleType {
        StyleType::SemiTransparent
    }
}
use crate::config::Config;
use crate::gui::common::icons::Icon;
use crate::gui::common::messages::AppEvent;
use crate::gui::components::button::IconButton;
use crate::gui::components::{Annotation, Shape, ShapeColor, ShapeStroke, ShapeType};
use crate::gui::style::button::ButtonType;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::{
    Canvas, Column, Container, Element, Row, Stack, horizontal_space, vertical_space,
};
use crate::gui::windows::GuiWindow;
use iced::Length::Fill;
use iced::Task;
use iced::alignment;
use iced::window::Id;

pub struct AnnotationWindow {
    shape: Shape,
    show_toolbar: bool,
}

#[derive(Debug, Clone)]
pub enum AnnotationWindowEvent {
    ChooseShapeType(ShapeType, bool, bool),
    ChangeColor(ShapeColor),
    ChangeStroke(ShapeStroke),
    Exit,
    Ignore,
    ToggleToolbar,
}

impl AnnotationWindow {
    pub fn new() -> Self {
        AnnotationWindow {
            shape: Default::default(),
            show_toolbar: false,
        }
    }

    fn toolbar(&self) -> Element<'_, AnnotationWindowEvent> {
        let panel = |row| {
            Container::new(row)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .padding(8)
        };

        let shapes_icon = |shape_type| {
            IconButton::new()
                .icon(match shape_type {
                    ShapeType::Personal => Icon::Pencil,
                    ShapeType::Rectangle => Icon::Square,
                    ShapeType::Line => Icon::Minus,
                    ShapeType::Eraser => Icon::Eraser,
                    ShapeType::Circle => Icon::Circle,
                })
                .build()
                .on_press(AnnotationWindowEvent::ChooseShapeType(
                    shape_type,
                    self.shape.is_filled,
                    self.shape.is_solid,
                ))
                .height(36)
                .width(36)
                .padding(0)
                .class(if self.shape.s_type == shape_type {
                    ButtonType::Disabled
                } else {
                    ButtonType::Standard
                })
        };

        let color_icon = |color: ShapeColor| {
            let button_class = if self.shape.color == color {
                ButtonType::Disabled
            } else {
                ButtonType::Standard
            };

            IconButton::new()
                .icon(Icon::Circle)
                .color(color.into_iced_color(true))
                .build()
                .on_press(AnnotationWindowEvent::ChangeColor(color))
                .height(36)
                .width(36)
                .padding(0)
                .class(button_class)
        };

        let stroke_icon = |stroke_type| {
            IconButton::new()
                .icon(Icon::Circle)
                .size(match stroke_type {
                    ShapeStroke::Thin => 8.0,
                    ShapeStroke::Medium => 12.0,
                    ShapeStroke::Broad => 15.0,
                })
                .build()
                .on_press(AnnotationWindowEvent::ChangeStroke(stroke_type))
                .height(36)
                .width(36)
                .padding(0)
                .class(if self.shape.stroke == stroke_type {
                    ButtonType::Disabled
                } else {
                    ButtonType::Standard
                })
        };

        Row::new()
            .push(horizontal_space().width(Fill))
            .push(panel(
                Row::new()
                    .push(shapes_icon(ShapeType::Line))
                    .push(shapes_icon(ShapeType::Circle))
                    .push(shapes_icon(ShapeType::Rectangle))
                    .push(shapes_icon(ShapeType::Personal))
                    .push(shapes_icon(ShapeType::Eraser))
                    .spacing(8),
            ))
            .push(horizontal_space().width(5))
            .push(panel(
                Row::new()
                    .push(color_icon(ShapeColor::Black))
                    .push(color_icon(ShapeColor::White))
                    .push(color_icon(ShapeColor::Green))
                    .push(color_icon(ShapeColor::Blue))
                    .push(color_icon(ShapeColor::Red))
                    .spacing(8),
            ))
            .push(horizontal_space().width(5))
            .push(panel(
                Row::new()
                    .push(stroke_icon(ShapeStroke::Thin))
                    .push(stroke_icon(ShapeStroke::Medium))
                    .push(stroke_icon(ShapeStroke::Broad))
                    .spacing(8),
            ))
            .push(horizontal_space().width(5))
            .push(panel(
                Row::new()
                    .push(
                        IconButton::new()
                            .icon(if self.shape.is_solid {
                                Icon::Circle
                            } else {
                                Icon::CircleHalf
                            })
                            .build()
                            .on_press(AnnotationWindowEvent::ChooseShapeType(
                                self.shape.s_type,
                                self.shape.is_filled,
                                !self.shape.is_solid,
                            ))
                            .height(36)
                            .width(36)
                            .padding(0),
                    )
                    .push(
                        IconButton::new()
                            .icon(if self.shape.is_filled {
                                Icon::Droplet
                            } else {
                                Icon::DropletSlash
                            })
                            .build()
                            .on_press(AnnotationWindowEvent::ChooseShapeType(
                                self.shape.s_type,
                                !self.shape.is_filled,
                                self.shape.is_solid,
                            ))
                            .height(36)
                            .width(36)
                            .padding(0),
                    )
                    .spacing(8),
            ))
            .push(horizontal_space().width(15))
            .push(panel(
                Row::new().push(
                    IconButton::new()
                        .icon(Icon::Close)
                        .build()
                        .width(36)
                        .height(36)
                        .padding(0)
                        .on_press(AnnotationWindowEvent::ToggleToolbar),
                ),
            ))
            .push(horizontal_space().width(Fill))
            .spacing(10)
            .into()
    }
}

impl GuiWindow for AnnotationWindow {
    type Message = AnnotationWindowEvent;

    fn title(&self) -> String {
        String::from("")
    }

    fn update(&mut self, id: Id, message: Self::Message, _config: &mut Config) -> Task<AppEvent> {
        match message {
            AnnotationWindowEvent::ChooseShapeType(shape_type, is_filled, is_solid) => {
                self.shape.s_type = shape_type;
                self.shape.is_filled = is_filled;
                self.shape.is_solid = is_solid;
                Task::none()
            }
            AnnotationWindowEvent::ChangeStroke(stroke_width) => {
                self.shape.stroke = stroke_width;
                Task::none()
            }
            AnnotationWindowEvent::ChangeColor(color) => {
                self.shape.color = color;
                Task::none()
            }
            AnnotationWindowEvent::Ignore => Task::none(),
            AnnotationWindowEvent::Exit => Task::done(AppEvent::CloseWindow(id)),
            AnnotationWindowEvent::ToggleToolbar => {
                self.show_toolbar = !self.show_toolbar;
                Task::none()
            }
        }
    }

    fn view(&self, _config: &Config) -> Element<'_, Self::Message> {
        let toolbar = if self.show_toolbar {
            self.toolbar()
        } else {
            Row::new()
                .push(horizontal_space().width(Fill))
                .push(
                    IconButton::new()
                        .icon(Icon::Menu)
                        .build()
                        .width(36)
                        .height(36)
                        .padding(0)
                        .on_press(AnnotationWindowEvent::ToggleToolbar),
                )
                .push(horizontal_space().width(8))
                .push(
                    IconButton::new()
                        .icon(Icon::Close)
                        .build()
                        .width(36)
                        .height(36)
                        .padding(0)
                        .on_press(AnnotationWindowEvent::Exit),
                )
                .push(horizontal_space().width(Fill))
                .into()
        };

        let overlay = Column::new()
            .push(vertical_space().height(5))
            .push(toolbar)
            .push(vertical_space().height(5));

        Stack::new()
            .push(
                Canvas::new(Annotation::new(self.shape).on_esc(AnnotationWindowEvent::Exit))
                    .width(Fill)
                    .height(Fill),
            )
            .push(overlay)
            .height(Fill)
            .width(Fill)
            .into()
    }

    fn theme(&self) -> StyleType {
        StyleType::Transparent
    }
}

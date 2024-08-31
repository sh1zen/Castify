use iced::widget::Container;
use crate::gui::appbase::App;
use crate::gui::components::raw::screenArea::AreaSelector;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::messages::Message;

#[derive(Debug, Clone, Copy)]
pub enum AreaSelectionMessage {
    /// Start the area selection
    StartSelection { x: f32, y: f32 },
    /// Update the area selection
    UpdateSelection { x: f32, y: f32 },
    /// End the area selection
    EndSelection,
}

pub fn screen_area_layer(app: &App) -> Container<Message, StyleType> {
    let area_selector = AreaSelector::new()
        .on_release(|(x, y)| {
            Message::AreaSelection(AreaSelectionMessage::StartSelection { x, y })
        })
        .on_press(|(x, y)| {
            Message::AreaSelection(AreaSelectionMessage::UpdateSelection { x, y })
        });

    let content = area_selector.view();

    Container::new(content)
}


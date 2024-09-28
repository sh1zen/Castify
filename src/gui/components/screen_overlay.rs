use crate::gui::components::screenArea::AreaSelector;
use crate::gui::common::messages::AppEvent as appMessage;
use crate::gui::widget::Container;
use iced_core::{alignment, Length};

pub fn screen_area_layer<'a>() -> Container<'a, appMessage> {
    let area_selector = AreaSelector::new()
        .on_release_rect(|rect| {
            appMessage::AreaSelected(rect)
        });

    Container::new(
        AreaSelector::view(area_selector)
    )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(alignment::Vertical::Center)
        .align_x(alignment::Horizontal::Center)
        .into()
}

use crate::gui::appbase::App;
use crate::gui::components::raw::screenArea::AreaSelector;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::messages::Message as appMessage;
use iced::widget::Container;
use iced_core::Length;


pub fn screen_area_layer(_: &App) -> Container<appMessage, StyleType> {

    let area_selector = AreaSelector::new()
        .on_release_rect(|rect| {
            appMessage::AreaSelected(rect)
        });

    Container::new(
        AreaSelector::view(area_selector)
    )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
        .into()
}

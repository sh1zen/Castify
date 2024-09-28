pub mod home;
pub mod caster;
pub mod client;
pub mod footer;
pub mod popup;
pub mod hotkeys;
pub mod screen_overlay;

mod screen_area;

pub use crate::gui::components::screen_area::area_selector as screenArea;
pub use crate::gui::components::screen_area::style as screenAreaStyle;

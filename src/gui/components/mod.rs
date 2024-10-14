pub mod home;
pub mod caster;
pub mod receiver;
pub mod footer;
pub mod popup;
pub mod hotkeys;
pub mod custom;
mod area_selector;
mod annotation;
pub mod info;

pub use area_selector::AreaSelector;
pub use annotation::{Annotation, Shape, ShapeColor, ShapeType, ShapeStroke};
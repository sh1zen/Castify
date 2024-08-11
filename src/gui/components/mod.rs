use std::fmt::Debug;
pub mod start;
pub mod recording;
pub mod client;
pub mod footer;
pub mod misc;

/*
pub trait Component<'a> {
    type Message: Into<appMessage> + Clone + Debug;
    type UpdateProps;
    type ViewProps;

    fn update(&mut self, message: Self::Message, props: Self::UpdateProps)
        -> Command<appMessage>;

    fn view(&self, props: Self::ViewProps) -> Element<'_, appMessage>;
}
*/
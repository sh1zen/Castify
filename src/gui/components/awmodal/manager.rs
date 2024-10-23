use crate::config::Config;
use crate::gui::common::icons::Icon;
use crate::gui::components::awmodal::GuiComponent;
use crate::gui::components::button::IconButton;
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{horizontal_line, vertical_space, Column, Container, Element, IcedParentExt, Space, Text};
use iced::Length;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct AwModalManager<T> {
    popup: Option<Arc<UnsafeCell<T>>>,
    visible: Arc<AtomicBool>,
}

impl<T, Message> AwModalManager<T>
where
    T: GuiComponent<Message=Message> + 'static,
    Message: Clone + 'static,
{
    pub fn new() -> Self {
        AwModalManager {
            popup: None,
            visible: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.popup.is_some() && self.visible.load(Ordering::SeqCst)
    }

    pub fn hide(&mut self) {
        self.visible.store(false, Ordering::SeqCst);
    }

    pub fn show(&mut self) {
        self.visible.store(true, Ordering::SeqCst);
    }

    pub fn set(&mut self, p0: T) {
        self.popup = Some(Arc::new(UnsafeCell::new(p0)));
    }

    pub fn remove(&mut self) {
        self.hide();
        self.popup = None;
    }

    pub fn render<'a, 'b>(&'b self, config: &Config) -> Element<'a, Message>
    where
        'b: 'a,
        Message: Clone,
    {
        if !self.is_visible() {
            return Container::new(Space::new(0, 0)).into();
        }

        let binding = self.get_ref().unwrap();
        let gui = binding.as_gui();

        let items = Column::new()
            .spacing(10)
            .padding(20)
            .push_if(!gui.title().is_empty(), || Text::new(gui.title()).size(20))
            .push(horizontal_line())
            .push(vertical_space().height(5))
            .push(gui.view(config))
            .push(vertical_space().height(5))
            .push_if(
                gui.on_close().is_some(),
                || IconButton::new().icon(Icon::Close).label("Close".to_string()).build().on_press(gui.on_close().unwrap()),
            )
            .width(Length::Fill)
            .height(Length::Fill);

        let content = Container::new(items)
            .class(ContainerType::Modal)
            .width(gui.width())
            .height(gui.height());

        Container::new(content)
            .center(Length::Fill)
            .into()
    }

    pub fn get_ref(&self) -> Option<&T>
    {
        self.popup.as_ref().and_then(move |p|
            Some(
                unsafe { &*p.get() }
            )
        )
    }

    pub fn get_mut_ref<'a, 'b>(&'b self) -> Option<&'a mut T>
    where
        'b: 'a,
    {
        self.popup.as_ref().and_then(move |p|
            Some(
                unsafe { &mut *p.get() }
            )
        )
    }
}

impl<T> Clone for AwModalManager<T> {
    fn clone(&self) -> Self {
        AwModalManager {
            popup: Some(Arc::clone(self.popup.as_ref().unwrap())),
            visible: Arc::clone(&self.visible),
        }
    }
}

// SAFETY: can implement `Send` if `T` is `Send`, because
// sending AwModalManager<T> to another thread is safe if `T` can be safely
// sent to another thread.
unsafe impl<T: Send> Send for AwModalManager<T> {}

// SAFETY: can implement `Sync` if `T` is `Sync`, because
// sharing references to AwModalManager<T> between threads is safe if `T` is
// itself `Sync`.
unsafe impl<T: Sync> Sync for AwModalManager<T> {}
use crate::assets::FONT_FAMILY_BOLD;
use crate::config::Config;
use crate::gui::common::anybox::AnyBox;
use crate::gui::common::icons::Icon;
use crate::gui::components::awmodal::GuiInterface;
use crate::gui::components::button::IconButton;
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{Column, Container, Element, IcedButtonExt, Row, Scrollable, Text, TextInput};
use crate::gui::windows::main::{MainWindowEvent, Page};
use crate::utils::net::webrtc::SDPICEExchangeWRTC;
use crate::utils::sos::SignalOfStop;
use crate::utils::status::Status;
use iced::Length;
use iced_wgpu::core::alignment;
use std::sync::Arc;
use tokio::sync::Mutex;

struct HandleSDP {
    sdp: String,
    watcher: SignalOfStop,
}

impl Clone for HandleSDP {
    fn clone(&self) -> Self {
        HandleSDP {
            sdp: self.sdp.clone(),
            watcher: self.watcher.clone(),
        }
    }
}

pub struct WrtcModal {
    sdp_provider: Arc<Mutex<Option<Arc<dyn SDPICEExchangeWRTC>>>>,
    doing_offer: bool,
    status: Status,
    local_sdp: HandleSDP,
    remote_sdp: HandleSDP,
}

impl WrtcModal {
    pub fn new(doing_offer: bool) -> Self {
        let status = Status::new(0);

        WrtcModal {
            sdp_provider: Arc::new(Mutex::new(None)),
            doing_offer,
            status,
            local_sdp: HandleSDP {
                sdp: String::new(),
                watcher: SignalOfStop::new(),
            },
            remote_sdp: HandleSDP {
                sdp: String::new(),
                watcher: SignalOfStop::new(),
            },
        }
    }

    pub async fn set_sdp_provider(&mut self, sdp: Arc<dyn SDPICEExchangeWRTC>) {
        self.sdp_provider.lock().await.replace(sdp);
    }

    pub async fn handle_sdp_negotiation(&mut self, offering: bool) {
        if self.doing_offer == offering {
            self.local_sdp.sdp = self.sdp_provider.lock().await.as_ref().unwrap().as_ref().get_sdp().await;
            self.local_sdp.watcher.wait().await;
        } else {
            loop {
                self.remote_sdp.watcher.wait().await;
                let res = self.sdp_provider.lock().await.as_ref().unwrap().as_ref().set_remote_sdp(self.remote_sdp.sdp.clone()).await;
                if res {
                    break;
                } else {
                    self.remote_sdp.watcher.restore();
                    self.status.set(400);
                }
            }
        }
        self.status.next();
    }

    fn show_sdp<'a>(&self) -> Element<'a, MainWindowEvent> {
        if self.local_sdp.sdp.is_empty() {
            return Column::new()
                .align_x(alignment::Alignment::Center)
                .push(Text::new("Loading...").font(FONT_FAMILY_BOLD).size(20).align_x(alignment::Alignment::Center))
                .into();
        }
        let local_sdp = self.local_sdp.clone();
        let receiver = !self.doing_offer;
        Column::new()
            .spacing(10)
            .push(
                Container::new(
                    Scrollable::new(
                        Text::new(local_sdp.sdp.clone()).size(12)
                    )
                ).class(ContainerType::Standard)
            )
            .push(
                Row::new()
                    .spacing(12)
                    .push(
                        IconButton::new().label("Copy").icon(Icon::Copy).build()
                            .on_press_with(move || {
                                local_sdp.watcher.cancel();
                                MainWindowEvent::CopyToClipboard(local_sdp.sdp.clone())
                            })
                    )
                    .push(
                        IconButton::new().label("Abort").icon(Icon::Close).build()
                            .on_press(MainWindowEvent::ClosePopup(if receiver { Some(Page::Home) } else { None }))
                    )
            )
            .into()
    }

    fn get_remote_sdp<'a>(&self) -> Element<'a, MainWindowEvent> {
        let rsdp_watcher = self.remote_sdp.watcher.clone();
        let receiver = !self.doing_offer;
        Column::new()
            .spacing(20)
            .push(
                TextInput::new("Paste here the remote SDP.", &self.remote_sdp.sdp)
                    .on_input(move |new_value| {
                        MainWindowEvent::PopupMessage(
                            AnyBox::new(new_value)
                        )
                    })
                    .padding([8, 12])
            )
            .push(
                Row::new()
                    .spacing(12)
                    .push(
                        IconButton::new().label("Ok").icon(Icon::Ok).build()
                            .on_press_if(!self.remote_sdp.sdp.is_empty(), move || {
                                rsdp_watcher.cancel();
                                MainWindowEvent::Ignore
                            })
                    )
                    .push(
                        IconButton::new().label("Abort").icon(Icon::Close).build()
                            .on_press(MainWindowEvent::ClosePopup(if receiver { Some(Page::Home) } else { None }))
                    )
            )
            .into()
    }
}

impl GuiInterface for WrtcModal {
    type Message = MainWindowEvent;

    fn title(&self) -> String {
        String::from("Manual SDP sharing.")
    }

    fn update(&mut self, value: AnyBox, _config: &Config) {
        self.remote_sdp.sdp = value.downcast::<String>().unwrap().clone();
    }

    fn view<'a, 'b>(&'a self, _config: &Config) -> Element<'b, Self::Message>
    where
        'b: 'a,
        Self::Message: Clone + 'b,
    {
        let content = match self.status.get() {
            0 => {
                if self.doing_offer {
                    self.show_sdp()
                } else {
                    self.get_remote_sdp()
                }
            }
            1 => {
                if self.doing_offer {
                    self.get_remote_sdp()
                } else {
                    self.show_sdp()
                }
            }
            400 => {
                Column::new()
                    .push(Text::new("Invalid Remote SDP").size(20).font(FONT_FAMILY_BOLD).align_x(alignment::Alignment::Center))
                    .push(IconButton::new().label("Retry").build().on_press(MainWindowEvent::ShowSDP))
                    .width(Length::Fill)
                    .into()
            }
            _ => {
                Column::new()
                    .push(
                        Text::new("Connecting...")
                            .size(20).font(FONT_FAMILY_BOLD)
                            .align_x(alignment::Alignment::Center)
                            .align_y(alignment::Alignment::Center)
                    )
                    .width(Length::Fill)
                    .align_x(alignment::Alignment::Center)
                    .into()
            }
        };

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn width(&self) -> Length {
        Length::Fixed(600.0)
    }

    fn height(&self) -> Length {
        Length::Fixed(450.0)
    }
}
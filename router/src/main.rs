mod config;
mod event;
mod peer;
mod router;

use anyhow::Result;
use iced::widget::{
    button, column, container, horizontal_space, mouse_area, opaque, row, scrollable, stack, text,
    text_input, vertical_space,
};
use iced::{Background, Border, Center, Color, Element, Length, Shadow, Task, Theme, Vector};
use iced_aw::ContextMenu;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct Router;
impl Router {
    pub fn new() -> Self {
        Self
    }
    pub async fn route(&self, _token: CancellationToken) {
        std::future::pending().await
    }
}

fn main() -> Result<()> {
    let token = CancellationToken::new();
    let router = Arc::new(Router::new());

    iced::application("floating", UIState::update, UIState::view)
        .theme(|_| Theme::Dracula)
        .run_with(move || {
            (
                UIState::new(router.clone()),
                Task::perform(async move { router.route(token).await }, |_| Message::Exit),
            )
        })?;
    Ok(())
}

#[derive(Debug, Clone)]
struct PeerInfo {
    id: String,
    ip: String,
    status: String,
}

struct UIState {
    router: Arc<Router>,
    my_ip: String,
    my_mask: String,
    state: String,
    peers: Vec<PeerInfo>,

    show_modal: bool,
    is_editing: bool,
    input_id: String,
    input_ip: String,
    editing_target_id: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    Exit,
    OpenAddModal,
    OpenEditModal(PeerInfo),
    CloseModal,
    InputIdChanged(String),
    InputIpChanged(String),
    SubmitForm,
    DeletePeer(String),
    BanPeer(String),
    CopyIp(String),
}

impl UIState {
    fn new(router: Arc<Router>) -> Self {
        Self {
            router,
            my_ip: "10.0.0.1".to_string(),
            my_mask: "255.255.255.0".to_string(),
            state: "Running".to_string(),
            peers: (0..5)
                .map(|i| PeerInfo {
                    id: format!("peer-{}", i),
                    ip: format!("10.0.0.{}", i + 2),
                    status: "Idle".into(),
                })
                .collect(),

            show_modal: false,
            is_editing: false,
            input_id: String::new(),
            input_ip: String::new(),
            editing_target_id: None,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Exit => return iced::exit(),

            Message::OpenAddModal => {
                self.input_id.clear();
                self.input_ip.clear();
                self.is_editing = false;
                self.show_modal = true;
            }
            Message::OpenEditModal(peer) => {
                self.input_id = peer.id.clone();
                self.input_ip = peer.ip;
                self.editing_target_id = Some(peer.id);
                self.is_editing = true;
                self.show_modal = true;
            }
            Message::CloseModal => {
                self.show_modal = false;
            }
            Message::InputIdChanged(val) => self.input_id = val,
            Message::InputIpChanged(val) => self.input_ip = val,

            Message::SubmitForm => {
                if self.is_editing {
                    if let Some(target) = &self.editing_target_id {
                        if let Some(peer) = self.peers.iter_mut().find(|p| &p.id == target) {
                            peer.id = self.input_id.clone();
                            peer.ip = self.input_ip.clone();
                        }
                    }
                } else {
                    self.peers.push(PeerInfo {
                        id: self.input_id.clone(),
                        ip: self.input_ip.clone(),
                        status: "New".into(),
                    });
                }
                self.show_modal = false;
            }

            Message::DeletePeer(id) => {
                self.peers.retain(|p| p.id != id);
            }
            Message::BanPeer(_id) => {}
            Message::CopyIp(_ip) => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let info_header = container(
            row![
                label_value("IP:", &self.my_ip),
                label_value("Mask:", &self.my_mask),
                label_value("State:", &self.state),
            ]
            .spacing(20),
        );

        let controls = row![
            horizontal_space(),
            button("Add Peer")
                .on_press(Message::OpenAddModal)
                .style(button::primary)
        ];

        let table_header = row![
            text("ID")
                .width(Length::FillPortion(1))
                .style(text::primary),
            text("IP Address")
                .width(Length::FillPortion(2))
                .style(text::primary),
            text("Status")
                .width(Length::FillPortion(1))
                .style(text::primary),
        ]
        .padding(10);

        let peers_list = column(self.peers.iter().map(|peer| {
            let row_content = container(
                row![
                    text(&peer.id).width(Length::FillPortion(1)),
                    text(&peer.ip).width(Length::FillPortion(2)),
                    text(&peer.status).width(Length::FillPortion(1)),
                ]
                .align_y(Center),
            )
            .padding(10)
            .style(|theme: &Theme| container::Style {
                background: Some(Background::Color(theme.palette().background)),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

            ContextMenu::new(row_content, move || {
                column![
                    button(text("Edit").size(14))
                        .on_press(Message::OpenEditModal(peer.clone()))
                        .style(button::text)
                        .width(Length::Fill),
                    button(text("Copy IP").size(14))
                        .on_press(Message::CopyIp(peer.ip.clone()))
                        .style(button::text)
                        .width(Length::Fill),
                    button(text("Delete").size(14))
                        .on_press(Message::DeletePeer(peer.id.clone()))
                        .style(button::danger)
                        .width(Length::Fill),
                ]
                .padding(5)
                .spacing(2)
                .width(150)
                .into()
            })
            .into()
        }))
        .spacing(5);

        let dashboard = container(
            column![
                info_header,
                vertical_space().height(20),
                controls,
                vertical_space().height(10),
                container(column![table_header, scrollable(peers_list)])
            ]
            .padding(20)
            .max_width(800)
            .align_x(Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill);

        if self.show_modal {
            stack![dashboard, self.view_modal_overlay()].into()
        } else {
            dashboard.into()
        }
    }

    fn view_modal_overlay(&self) -> Element<'_, Message> {
        let title = if self.is_editing {
            "Edit Peer"
        } else {
            "Add New Peer"
        };

        let modal_card = container(column![
            text(title).size(18).style(text::primary),
            vertical_space().height(15),
            text("Peer ID").size(12).style(text::secondary),
            text_input("e.g. peer-1", &self.input_id)
                .on_input(Message::InputIdChanged)
                .padding(10),
            vertical_space().height(10),
            text("IP Address").size(12).style(text::secondary),
            text_input("e.g. 10.0.0.5", &self.input_ip)
                .on_input(Message::InputIpChanged)
                .padding(10),
            vertical_space().height(20),
            row![
                button("Cancel")
                    .on_press(Message::CloseModal)
                    .style(button::secondary),
                horizontal_space(),
                button("Save")
                    .on_press(Message::SubmitForm)
                    .style(button::primary),
            ]
        ])
        .width(300)
        .padding(20)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(theme.palette().background)),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: theme.palette().primary,
            },
            shadow: Shadow {
                color: Color::BLACK,
                offset: Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
            ..Default::default()
        });

        let overlay = mouse_area(
            container(modal_card)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_theme: &Theme| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.8,
                        ..Color::BLACK
                    })),
                    ..Default::default()
                }),
        )
        .on_press(Message::CloseModal);

        overlay.into()
    }
}

fn label_value<'a>(label: &'a str, value: &'a str) -> Element<'a, Message> {
    row![
        text(label).style(text::secondary),
        text(value).style(text::primary)
    ]
    .spacing(5)
    .into()
}

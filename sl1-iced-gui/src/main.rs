mod device;
mod types;

use crate::device::Device;
use crate::types::Port;
use device::{DeviceSettings, DeviceWifiSettings, Preset};
use iced::{
    Alignment::Center,
    Color, Element, Font, Theme,
    widget::{Text, button, center, column, combo_box, row, slider, text},
};
use std::net::IpAddr;

const FONT: &str = "JetBrains Mono NL";

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    // let window_settings = iced::window::Settings {
    //     transparent: true,
    //     ..Default::default()
    // };

    iced::application(App::title, App::update, App::view)
        .settings(iced::Settings {
            default_font: Font::with_name(FONT),
            ..Default::default()
        })
        .theme(App::theme)
        // .window(window_settings)
        .run()
}

#[derive(Debug, Default)]
struct App {
    // State is not persistent application state which is calculated on each startup
    state: State,
    // Settings, on the other hand, are persistent and thus are stored and read on startup
    settings: Settings,
}

#[derive(Debug, Clone, Copy, Default)]
enum Page {
    #[default]
    Home,
    Settings,
}

impl App {
    pub fn title(&self) -> String {
        String::from("Smart Leds")
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::OpenHomePage => {
                self.state.page = Page::Home;
            }
            Message::OpenSettingsPage => {
                self.state.page = Page::Settings;
            }
            Message::ChangeTheme(theme) => {
                self.state.theme = theme;
            }
            Message::Device(message) => match message {
                DeviceMessage::SetBrightness(val) => {
                    self.state.brightness = val;
                }
                DeviceMessage::SetSpeed(val) => {
                    self.state.speed = val;
                }
                DeviceMessage::SetScale(val) => {
                    self.state.scale = val;
                }
                _ => unimplemented!(),
            },
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self.state.page {
            Page::Home => self.home_page(),
            Page::Settings => self.settings_page(),
        }
    }

    pub fn theme(&self) -> Theme {
        self.state.theme.clone()
    }

    fn home_page(&self) -> Element<Message> {
        let mut row = row![button("Settings").on_press(Message::OpenSettingsPage)]
            .padding(5)
            .spacing(10);

        if let Some(state) = &self.state.preset {
            row = row.push(combo_box(
                state,
                "Preset",
                self.state.selected_preset.as_ref(),
                |preset| Message::Device(DeviceMessage::SetPreset(preset)),
            ));
        }

        row = row.push(self.is_device_connected()).align_y(Center);

        column![row, self.slider_controls()].padding(10).into()
    }

    fn slider_controls(&self) -> Element<Message> {
        let brightness = &self.state.brightness;
        let speed = &self.state.speed;
        let scale = &self.state.scale;

        column![
            row![
                text!("Brightness:"),
                slider(0..=255, *brightness, |val| {
                    Message::Device(DeviceMessage::SetBrightness(val))
                }),
                text!("{}", *brightness).width(40),
            ]
            .padding(5)
            .spacing(20),
            row![
                text!("Speed:"),
                slider(0..=255, *speed, |val| {
                    Message::Device(DeviceMessage::SetSpeed(val))
                }),
                text!("{}", *speed).width(40),
            ]
            .padding(5)
            .spacing(20),
            row![
                text!("Scale:"),
                slider(0..=255, *scale, |val| {
                    Message::Device(DeviceMessage::SetScale(val))
                }),
                text!("{}", *scale).width(40),
            ]
            .padding(5)
            .spacing(20),
        ]
        .into()
    }

    fn is_device_connected(&self) -> Element<Message> {
        text!("Disconnected")
            .color(self.theme().palette().danger)
            .into()
    }

    fn settings_page(&self) -> Element<Message> {
        let settings = &self.settings;
        let device_title: Text = Text::new("Device Settings").size(30);

        column![
            button("Back").on_press(Message::OpenHomePage).padding(5),
            device_title,
            row![text!("IP: {}", settings.device.ip().to_string())],
            row![text!("Port: {}", settings.device.port())],
        ]
        .padding(10)
        .into()
    }
}

#[derive(Debug, Default)]
struct Settings {
    device: Device,
}

impl Settings {
    pub fn new(device: Device) -> Self {
        Self { device }
    }
}

#[derive(Debug)]
struct State {
    page: Page,
    theme: Theme,
    brightness: u8,
    speed: u8,
    scale: u8,
    // If preset is None => the connection with device was not established and the preset info has
    // not been fetched or loaded from config.
    preset: Option<combo_box::State<Preset>>,
    selected_preset: Option<Preset>,
}

impl Default for State {
    fn default() -> Self {
        let options = vec![Preset::new(0, "Static".to_string())];

        Self {
            page: Page::Home,
            theme: Theme::Dark,
            brightness: 128,
            speed: 128,
            scale: 128,
            // preset: None,
            preset: Some(combo_box::State::new(options)),
            selected_preset: None,
        }
    }
}
#[derive(Debug, Clone)]
pub enum Message {
    OpenSettingsPage,
    OpenHomePage,
    ChangeTheme(Theme),
    Device(DeviceMessage),
}

#[derive(Debug, Clone)]
pub enum DeviceMessage {
    TurnOn,
    TurnOff,
    Toggle,
    SetPreset(Preset),
    SetBrightness(u8),
    SetSpeed(u8),
    SetScale(u8),
    SetDeviceIp(IpAddr),
    SetDevicePort(Port),
    SetWifiSettings(DeviceWifiSettings),
    SetSettings(DeviceSettings),
}

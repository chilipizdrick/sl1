mod config;
mod device;
mod error;

use iced::{
    Alignment::Center,
    Element, Font, Subscription, Task, Theme,
    alignment::Vertical::Bottom,
    theme::Palette,
    widget::{
        self, Space, Text, TextEditor, button, column, combo_box, row, scrollable, slider, text,
        text_editor, text_input,
    },
};
use std::{net::IpAddr, time::Duration};

use crate::config::Config;
use crate::device::{Device, DeviceConnection, DeviceSettings, DeviceWifiSettings, Preset};
use crate::error::{Error, Result};

const FONT: &str = "JetBrains Mono NL";
const POLL_INTERVAL: Duration = Duration::from_secs(3);

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    let settings = iced::window::settings::Settings {
        transparent: true,
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: "sl1".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    iced::application(App::title, App::update, App::view)
        .settings(iced::Settings {
            default_font: Font::with_name(FONT),
            ..Default::default()
        })
        .window(settings)
        .subscription(App::subscription)
        .theme(App::theme)
        .run_with(App::new)
}

#[derive(Debug)]
struct App {
    state: State,
    settings: Config,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let settings = Config::load().unwrap_or_default();
        let presets = settings.preset_info().to_vec();
        let mut connection = settings.device().connect().ok();
        let mut device_settings = None;
        let mut selected_preset = None;
        let mut current_preset_settings = None;
        if let Some(connection) = &mut connection {
            device_settings = connection.get_device_settings().ok();
            if let Some(device_settings) = &device_settings {
                selected_preset = presets
                    .iter()
                    .find(|preset| preset.id() == device_settings.current_preset_id())
                    .cloned();
                current_preset_settings = Some(
                    device_settings.preset_settings()[device_settings.current_preset_id() as usize],
                );
            }
        }

        let state = StateBuilder::new()
            .brightness(current_preset_settings.map(|ps| ps.brightness()))
            .speed(current_preset_settings.map(|ps| ps.speed()))
            .scale(current_preset_settings.map(|ps| ps.scale()))
            .is_on(device_settings.map(|ps| ps.is_on()))
            .ip(&settings.device().ip())
            .port(&settings.device().port())
            .presets(presets)
            .selected_preset(selected_preset)
            .device_connection(connection)
            .build();
        let app = Self { state, settings };
        (app, Task::none())
    }

    pub fn title(&self) -> String {
        String::from("Smart lights")
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(POLL_INTERVAL).map(|_| Message::Device(DeviceMessage::Ping))
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Page(page) => self.state.page = page,
            Message::Theme(theme) => self.state.theme = theme,
            Message::Device(message) => self.handle_device_message(message),
            Message::Settings(message) => self.handle_settings_message(message),
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

    fn save_settings(&self) {
        if let Err(e) = self.settings.save() {
            log::error!("Error saving config: {e}");
        };
    }

    fn handle_device_message(&mut self, message: DeviceMessage) {
        if let DeviceMessage::Ping = message {
            self.handle_ping();
            return;
        }
        if let Some(connection) = &mut self.state.device_connection {
            match message {
                DeviceMessage::Ping => {}
                DeviceMessage::GetCurrentPresetId => self.handle_get_current_preset_id(),
                DeviceMessage::GetPresetInfo => self.handle_get_preset_info(),
                DeviceMessage::GetPresetSettings => self.handle_get_preset_settings(),
                DeviceMessage::SetToggle => match connection.toggle() {
                    Ok(_) => self.state.is_on = !self.state.is_on,
                    Err(e) => log::error!("{e}"),
                },
                DeviceMessage::SetTurnOn => {
                    if connection.turn_on().is_ok() {
                        self.state.is_on = true;
                    }
                }
                DeviceMessage::SetTurnOff => {
                    if connection.turn_off().is_ok() {
                        self.state.is_on = false;
                    }
                }
                DeviceMessage::SetPreset(preset) => {
                    if connection.set_preset(preset.id()).is_ok() {
                        self.state.selected_preset = Some(preset);
                        self.update(Message::Device(DeviceMessage::GetPresetSettings));
                    }
                }
                DeviceMessage::SetBrightness(value) => {
                    self.state.brightness = value;
                }
                DeviceMessage::SetBrightnessConfirm => {
                    let _ = connection.set_brightness(self.state.brightness);
                }
                DeviceMessage::SetSpeed(value) => {
                    self.state.speed = value;
                }
                DeviceMessage::SetSpeedConfirm => {
                    let _ = connection.set_speed(self.state.speed);
                }
                DeviceMessage::SetScale(value) => {
                    self.state.scale = value;
                }
                DeviceMessage::SetScaleConfirm => {
                    let _ = connection.set_scale(self.state.scale);
                }
                DeviceMessage::SetWifiSettings(settings) => {
                    let _ = connection.set_wifi_settings(settings);
                }
                DeviceMessage::SetSettings(settings) => {
                    let _ = connection.set_settings(settings);
                }
            }
        }
    }

    fn try_connect(&mut self) {
        if let Ok(new_connection) = self.settings.device().connect() {
            self.state.device_connection = Some(new_connection);
        } else {
            self.state.device_connection = None;
        }
    }

    fn handle_ping(&mut self) {
        match &mut self.state.device_connection {
            Some(connection) => match connection.ping() {
                Ok(_) => {}
                Err(err) => match err {
                    Error::DeviceConnection(_) | Error::DeviceSend(_) | Error::DeviceRecieve(_) => {
                        self.try_connect()
                    }
                    _ => {}
                },
            },
            None => {
                self.try_connect();
                if let Some(connection) = &mut self.state.device_connection {
                    if let Ok(settings) = connection.get_device_settings() {
                        self.state.is_on = settings.is_on();
                        let current_preset_settings =
                            settings.preset_settings()[settings.current_preset_id() as usize];
                        self.state.brightness = current_preset_settings.brightness();
                        self.state.speed = current_preset_settings.speed();
                        self.state.scale = current_preset_settings.scale();
                        self.state.selected_preset = self
                            .state
                            .preset
                            .options()
                            .iter()
                            .find(|preset| preset.id() == settings.current_preset_id())
                            .cloned();
                        self.view();
                    }
                }
            }
        }
    }

    fn handle_get_current_preset_id(&mut self) {
        if let Some(connection) = &mut self.state.device_connection {
            match connection.get_current_preset_id() {
                Ok(preset_id) => {
                    let preset = self
                        .state
                        .preset
                        .options()
                        .iter()
                        .find(|preset| preset.id() == preset_id)
                        .cloned();
                    self.state.selected_preset = preset;
                }
                Err(e) => log::error!("Error getting current preset id: {e}"),
            }
        }
    }

    fn handle_get_preset_info(&mut self) {
        if let Some(connection) = &mut self.state.device_connection {
            match connection.get_preset_info() {
                Ok(presets) => {
                    self.state.preset = combo_box::State::new(presets.clone());
                    self.settings.set_preset_info(presets);
                    self.save_settings();
                }
                Err(e) => log::error!("Error: {e}"),
            }
        }
    }

    fn handle_get_preset_settings(&mut self) {
        if let Some(connection) = &mut self.state.device_connection {
            match connection.get_preset_settings() {
                Ok(preset) => {
                    self.state.brightness = preset.brightness();
                    self.state.speed = preset.speed();
                    self.state.scale = preset.scale();
                }
                Err(e) => log::error!("Error getting preset settings: {e}"),
            }
        }
    }

    fn handle_settings_message(&mut self, message: SettingsMessage) {
        match message {
            SettingsMessage::Ip(ip) => self.state.ip_text = ip,
            SettingsMessage::Port(port) => self.state.port_text = port,
            SettingsMessage::SaveIpPort => self.handle_save_message(),
            SettingsMessage::IpError => {
                self.state.ip_port_error_message = Some(IpPortErrorMessage::InvalidIp)
            }
            SettingsMessage::PortError => {
                self.state.ip_port_error_message = Some(IpPortErrorMessage::InvalidPort)
            }
            SettingsMessage::EditDeviceSettings(action) => {
                self.state.device_settings_content.perform(action)
            }
            SettingsMessage::ImportDeviceSettings => match self.handle_import_fallible() {
                Ok(_) => {}
                Err(e) => log::error!("Error importing device settings: {e}"),
            },
            SettingsMessage::ExportDeviceSettings => self.handle_export(),
            SettingsMessage::ExportDeviceSettingsError => {
                self.state.device_settings_error_message =
                    Some(DeviceSettingsErrorMessage::DeviceSettingsDeserialization)
            }
        }
    }

    fn handle_save_message_fallible(&mut self) -> Result<()> {
        self.state.device_connection = None;
        let ip: IpAddr = self.state.ip_text.parse().map_err(Error::AddrParse)?;
        let port: u16 = self.state.port_text.parse().map_err(Error::PortParse)?;
        self.settings.set_device(Device::new(ip, port));
        self.save_settings();
        self.state.ip_port_error_message = None;
        let connection = self.settings.device().connect()?;
        self.state.device_connection = Some(connection);
        Ok(())
    }

    fn handle_import_fallible(&mut self) -> Result<()> {
        if let Some(connection) = &mut self.state.device_connection {
            let device_settings = connection.get_device_settings()?;
            let device_settings_string =
                serde_json::to_string_pretty(&device_settings).map_err(|_| Error::Serialization)?;
            self.state.device_settings_content =
                text_editor::Content::with_text(device_settings_string.trim());
        }
        Ok(())
    }

    fn handle_export(&mut self) {
        match self.handle_export_fallible() {
            Ok(_) => {}
            Err(err) => {
                if let Error::Deserialization = err {
                    self.update(Message::Settings(
                        SettingsMessage::ExportDeviceSettingsError,
                    ))
                }
            }
        }
    }

    fn handle_export_fallible(&mut self) -> Result<()> {
        if let Some(connection) = &mut self.state.device_connection {
            let device_settings: DeviceSettings =
                serde_json::from_str(&self.state.device_settings_content.text())
                    .map_err(|_| Error::Deserialization)?;
            connection.set_settings(device_settings)?;
            self.state.device_settings_error_message = None;
        }
        Ok(())
    }

    fn handle_save_message(&mut self) {
        match self.handle_save_message_fallible() {
            Ok(_) => {}
            Err(err) => {
                log::error!("Error saving settings: {err}");
                match err {
                    Error::AddrParse(_) => self.update(Message::Settings(SettingsMessage::IpError)),
                    Error::PortParse(_) => {
                        self.update(Message::Settings(SettingsMessage::PortError))
                    }
                    _ => {}
                }
            }
        }
    }

    fn home_page(&self) -> Element<Message> {
        let state = &self.state.preset;
        let page_title = Text::new("Smart Lights").size(30);

        let settings_button = button("Settings").on_press(Message::Page(Page::Settings));
        let load_presets_button =
            button("Load Presets").on_press(Message::Device(DeviceMessage::GetPresetInfo));

        let is_device_connected_text = self.is_device_connected();

        let top_row = row![
            settings_button,
            load_presets_button,
            Space::with_width(iced::Length::Fill),
            is_device_connected_text
        ]
        .padding(5)
        .spacing(10)
        .align_y(Center);

        let preset_combo_box = combo_box(
            state,
            "Preset",
            self.state.selected_preset.as_ref(),
            |preset| Message::Device(DeviceMessage::SetPreset(preset)),
        );

        let toggle_button = if self.state.is_on {
            button("On").style(widget::button::success)
        } else {
            button("Off").style(widget::button::danger)
        }
        .on_press(Message::Device(DeviceMessage::SetToggle));

        let control_row = row![preset_combo_box, toggle_button]
            .padding(5)
            .spacing(10)
            .align_y(Center);

        scrollable(
            column![
                top_row,
                row![page_title].padding(5),
                control_row,
                self.slider_controls()
            ]
            .padding(10),
        )
        .into()
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
                })
                .on_release(Message::Device(DeviceMessage::SetBrightnessConfirm)),
                text!("{}", *brightness).width(40),
            ]
            .padding(5)
            .spacing(20),
            row![
                text!("Speed:"),
                slider(0..=255, *speed, |val| {
                    Message::Device(DeviceMessage::SetSpeed(val))
                })
                .on_release(Message::Device(DeviceMessage::SetSpeedConfirm)),
                text!("{}", *speed).width(40),
            ]
            .padding(5)
            .spacing(20),
            row![
                text!("Scale:"),
                slider(0..=255, *scale, |val| {
                    Message::Device(DeviceMessage::SetScale(val))
                })
                .on_release(Message::Device(DeviceMessage::SetScaleConfirm)),
                text!("{}", *scale).width(40),
            ]
            .padding(5)
            .spacing(20),
        ]
        .padding(5)
        .into()
    }

    fn is_device_connected(&self) -> Element<Message> {
        match self.state.device_connection {
            Some(_) => text!("Connected")
                .color(self.theme().palette().success)
                .into(),
            None => text!("Disconnected")
                .color(self.theme().palette().danger)
                .into(),
        }
    }

    fn settings_page(&self) -> Element<Message> {
        let page_title = Text::new("Device Settings").size(30);

        scrollable(
            column![
                row![
                    button("Back").on_press(Message::Page(Page::Home)),
                    Space::with_width(iced::Length::Fill),
                    self.is_device_connected()
                ]
                .align_y(Center)
                .spacing(10)
                .padding(5),
                row![page_title].padding(5),
                self.view_ip_port_settings(),
                self.view_device_settings(),
            ]
            .spacing(10)
            .padding(10),
        )
        .into()
    }

    fn view_ip_port_settings(&self) -> Element<Message> {
        let settings = &self.settings;
        let section_title = Text::new("IP/Port Settings").size(24);
        let ip_input = text_input("IP", &self.state.ip_text)
            .on_input(|input| Message::Settings(SettingsMessage::Ip(input)))
            .on_submit(Message::Settings(SettingsMessage::SaveIpPort));
        let port_input = text_input("Post", &self.state.port_text)
            .on_input(|input| Message::Settings(SettingsMessage::Port(input)))
            .on_submit(Message::Settings(SettingsMessage::SaveIpPort));
        let save_button = button("Save").on_press(Message::Settings(SettingsMessage::SaveIpPort));
        let error_message = match &self.state.ip_port_error_message {
            Some(msg) => text!("{msg}").color(self.theme().palette().danger),
            None => Text::new(""),
        };

        column![
            row![section_title].padding(5),
            column![
                text!("IP: {}", settings.device().ip().to_string()),
                ip_input
            ]
            .padding(5),
            column![text!("Port: {}", settings.device().port()), port_input].padding(5),
            row![
                save_button,
                Space::with_width(iced::Length::Fill),
                error_message.align_y(Bottom)
            ]
            .spacing(10)
            .padding(5),
        ]
        .into()
    }

    fn view_device_settings(&self) -> Element<Message> {
        let section_title = Text::new("Import/Export Settings").size(24);
        let editor = TextEditor::new(&self.state.device_settings_content)
            .placeholder("Device settings")
            .on_action(|action| Message::Settings(SettingsMessage::EditDeviceSettings(action)));
        let import_button =
            button("Import").on_press(Message::Settings(SettingsMessage::ImportDeviceSettings));
        let export_button =
            button("Export").on_press(Message::Settings(SettingsMessage::ExportDeviceSettings));

        let error_message = match &self.state.device_settings_error_message {
            Some(msg) => text!("{msg}").color(self.theme().palette().danger),
            None => Text::new(""),
        };

        column![
            row![section_title].padding(5),
            editor.padding(5),
            row![
                import_button,
                export_button,
                Space::with_width(iced::Length::Fill),
                error_message,
            ]
            .spacing(10)
            .padding(5)
        ]
        .spacing(10)
        .padding(5)
        .into()
    }
}

#[derive(Debug)]
struct State {
    page: Page,
    theme: Theme,
    is_on: bool,
    brightness: u8,
    speed: u8,
    scale: u8,
    preset: combo_box::State<Preset>,
    selected_preset: Option<Preset>,
    ip_text: String,
    port_text: String,
    device_settings_content: text_editor::Content,
    device_connection: Option<DeviceConnection>,
    ip_port_error_message: Option<IpPortErrorMessage>,
    device_settings_error_message: Option<DeviceSettingsErrorMessage>,
}

#[derive(Debug, Clone)]
enum IpPortErrorMessage {
    InvalidIp,
    InvalidPort,
}

impl std::fmt::Display for IpPortErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            IpPortErrorMessage::InvalidIp => "Invalid IP has been entered!",
            IpPortErrorMessage::InvalidPort => "Invalid Port has been entered!",
        };
        write!(f, "{msg}")
    }
}

#[derive(Debug, Clone)]
enum DeviceSettingsErrorMessage {
    DeviceSettingsDeserialization,
}

impl std::fmt::Display for DeviceSettingsErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            DeviceSettingsErrorMessage::DeviceSettingsDeserialization => {
                "Invalid settings JSON has been entered!"
            }
        };
        write!(f, "{msg}")
    }
}

struct StateBuilder {
    page: Page,
    theme: Theme,
    is_on: Option<bool>,
    brightness: Option<u8>,
    speed: Option<u8>,
    scale: Option<u8>,
    selected_preset: Option<Preset>,
    preset: Option<combo_box::State<Preset>>,
    ip_text: Option<String>,
    port_text: Option<String>,
    device_settings_text: Option<String>,
    device_connection: Option<DeviceConnection>,
}

impl StateBuilder {
    pub fn new() -> Self {
        let palette = Palette {
            background: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.8),
            ..Palette::CATPPUCCIN_MOCHA
        };
        let theme = Theme::custom("Zalupa".to_string(), palette);

        Self {
            page: Page::Home,
            theme,
            is_on: None,
            brightness: None,
            speed: None,
            scale: None,
            selected_preset: None,
            preset: None,
            ip_text: None,
            port_text: None,
            device_settings_text: None,
            device_connection: None,
        }
    }

    pub fn brightness(mut self, brightness: Option<u8>) -> Self {
        self.brightness = brightness;
        self
    }

    pub fn speed(mut self, speed: Option<u8>) -> Self {
        self.speed = speed;
        self
    }

    pub fn scale(mut self, scale: Option<u8>) -> Self {
        self.scale = scale;
        self
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn is_on(mut self, is_on: Option<bool>) -> Self {
        self.is_on = is_on;
        self
    }

    pub fn ip(mut self, ip: &IpAddr) -> Self {
        self.ip_text = Some(ip.to_string());
        self
    }

    pub fn port(mut self, port: &u16) -> Self {
        self.port_text = Some(port.to_string());
        self
    }

    pub fn presets(mut self, presets: Vec<Preset>) -> Self {
        self.preset = Some(combo_box::State::new(presets));
        self
    }

    pub fn selected_preset(mut self, preset: Option<Preset>) -> Self {
        self.selected_preset = preset;
        self
    }

    pub fn device_connection(mut self, connection: Option<DeviceConnection>) -> Self {
        self.device_connection = connection;
        self
    }

    pub fn build(self) -> State {
        State {
            page: self.page,
            theme: self.theme,
            is_on: self.is_on.unwrap_or(false),
            brightness: self.brightness.unwrap_or(128),
            speed: self.speed.unwrap_or(128),
            scale: self.scale.unwrap_or(128),
            selected_preset: self.selected_preset,
            preset: self.preset.unwrap_or(combo_box::State::new(Vec::new())),
            ip_text: self.ip_text.unwrap_or("".to_string()),
            port_text: self.port_text.unwrap_or("".to_string()),
            device_settings_content: text_editor::Content::with_text(
                &self.device_settings_text.unwrap_or("".to_string()),
            ),
            device_connection: self.device_connection,
            ip_port_error_message: None,
            device_settings_error_message: None,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Theme(Theme),
    Page(Page),
    Device(DeviceMessage),
    Settings(SettingsMessage),
}

#[derive(Debug, Clone, Copy, Default)]
enum Page {
    #[default]
    Home,
    Settings,
}

#[derive(Debug, Clone)]
enum DeviceMessage {
    Ping,
    GetCurrentPresetId,
    GetPresetInfo,
    GetPresetSettings,
    SetToggle,
    SetTurnOn,
    SetTurnOff,
    SetPreset(Preset),
    SetBrightness(u8),
    SetBrightnessConfirm,
    SetSpeed(u8),
    SetSpeedConfirm,
    SetScale(u8),
    SetScaleConfirm,
    SetWifiSettings(DeviceWifiSettings),
    SetSettings(DeviceSettings),
}

#[derive(Debug, Clone)]
enum SettingsMessage {
    Ip(String),
    IpError,
    Port(String),
    PortError,
    SaveIpPort,
    EditDeviceSettings(text_editor::Action),
    ImportDeviceSettings,
    ExportDeviceSettings,
    ExportDeviceSettingsError,
}

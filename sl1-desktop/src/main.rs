mod config;
mod connection;
mod detector;
mod device;
mod error;

use std::net::IpAddr;
use std::time::{Duration, Instant};

use connection::{DeviceResponse, GetRequest, SetRequest};
use device::PresetId;
use iced::Alignment::Center;
use iced::alignment::Vertical::Bottom;
use iced::futures::channel::mpsc;
use iced::theme::Palette;
use iced::widget::{
    self, Space, button, column, combo_box, horizontal_space, row, scrollable, slider, text,
    text_editor, text_input,
};
use iced::{Element, Subscription, Task, Theme};
use ipnetwork::IpNetwork;

use crate::config::Config;
use crate::connection::{
    DeviceGetResponse, DeviceSetResponse, Request, Response, connection_worker,
};
use crate::device::{Device, DeviceSettings, Preset};

pub use crate::error::{Error, Result};

const DEVICE_POLL_INTERVAL: Duration = Duration::from_secs(3);
const DEVICE_DISCONNECT_INTERVAL: Duration = Duration::from_secs(5);

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscription)
        .transparent(true)
        .run_with(App::new)
}

#[derive(Debug)]
struct App {
    config: Config,
    sender: Option<mpsc::Sender<Request>>,

    theme: Theme,
    page: Page,

    last_handshake: Instant,
    is_device_connected: bool,
    is_on: bool,
    brightness: u8,
    speed: u8,
    scale: u8,
    preset: combo_box::State<Preset>,
    selected_preset: Option<Preset>,
    preset_info_message: Option<PresetInfoMessage>,
    ip_text: String,
    port_text: String,
    subnet_text: String,
    device_settings_content: text_editor::Content,
    detected_devices: DetectedDevicesState,

    ip_port_error_message: Option<IpPortErrorMessage>,
    device_settings_error_message: Option<DeviceSettingsErrorMessage>,
    detector_error_message: Option<DetectorErrorMessage>,
}

#[derive(Debug, Clone)]
enum Message {
    Page(Page),
    UI(UIMessage),
    Settings(SettingsMessage),
    Request(Request),
    Response(Response),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let config = Config::load().unwrap_or_default();
        let palette = Palette {
            background: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.8),
            ..Palette::CATPPUCCIN_MOCHA
        };
        let theme = Theme::custom("Catppuccin Black".to_string(), palette);

        let app = Self {
            config: config.clone(),
            sender: None,

            theme,
            page: Page::Home,

            last_handshake: Instant::now() - DEVICE_DISCONNECT_INTERVAL,
            is_device_connected: false,
            is_on: false,
            brightness: 128,
            speed: 128,
            scale: 128,
            preset: combo_box::State::new(config.preset_info().to_vec()),
            selected_preset: None,
            preset_info_message: None,
            ip_text: config.device().ip().to_string(),
            port_text: config.device().port().to_string(),
            subnet_text: "192.168.0.0/24".to_string(),
            device_settings_content: text_editor::Content::new(),
            detected_devices: DetectedDevicesState::None,

            ip_port_error_message: None,
            device_settings_error_message: None,
            detector_error_message: None,
        };

        (app, Task::none())
    }

    fn title(&self) -> String {
        "Smart Lights".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        self.is_device_connected = self.last_handshake.elapsed() < DEVICE_DISCONNECT_INTERVAL;

        match message {
            Message::Page(page) => self.handle_page(page),
            Message::Settings(message) => self.handle_settings_message(message),
            Message::Request(message) => self.handle_request_message(message),
            Message::Response(message) => self.handle_response_message(message),
            Message::UI(message) => self.handle_ui_message(message),
        }
    }

    fn view(&self) -> Element<Message> {
        match self.page {
            Page::Home => self.home_page(),
            Page::Settings => self.settings_page(),
        }
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            Subscription::run(connection_worker).map(Message::Response),
            iced::time::every(DEVICE_POLL_INTERVAL)
                .map(|_| Message::Request(Request::Get(GetRequest::Ping))),
        ])
    }

    fn handle_page(&mut self, page: Page) -> Task<Message> {
        self.page = page;
        Task::none()
    }

    fn handle_settings_message(&mut self, message: SettingsMessage) -> Task<Message> {
        use SettingsMessage as SM;

        match message {
            SM::SaveIpPort => self.handle_save(),
            SM::ImportDeviceSettings => self.handle_import(),
            SM::ExportDeviceSettings => self.handle_export(),
            SM::DetectDevice => self.handle_detect_device(),
            SM::DetectorOutput(devices) => self.handle_detector_output(devices),
            SM::SetDetectedDevice(device) => self.handle_set_detected_device(device),
        }
    }

    fn handle_request_message(&mut self, message: Request) -> Task<Message> {
        log::info!("Processing request: {:?}", &message);
        if let Some(sender) = &mut self.sender {
            if let Err(err) = sender.try_send(message.clone()) {
                log::error!("Error sending message: {err}");
            }
        }
        if let Request::Set(message) = &message {
            match message {
                SetRequest::Toggle => {
                    self.is_on = !self.is_on;
                    Task::none()
                }
                SetRequest::Preset(id) => {
                    self.set_selected_preset(id);
                    self.update(Message::Request(Request::Get(
                        GetRequest::CurrentPresetSettings,
                    )))
                }
                _ => Task::none(),
            }
        } else {
            Task::none()
        }
    }

    fn set_selected_preset(&mut self, id: &PresetId) {
        self.selected_preset = self
            .preset
            .options()
            .iter()
            .find(|p| p.id() == *id)
            .cloned();
    }

    fn handle_response_message(&mut self, response: Response) -> Task<Message> {
        match response {
            Response::Ready(mut sender) => {
                if let Err(err) =
                    sender.try_send(Request::SetDeviceAddr(self.config.device().addr()))
                {
                    log::error!("Error sending message throuh mpsc sender: {err}");
                }
                self.sender = Some(sender);
                self.update(Message::Request(Request::Get(GetRequest::Settings)))
            }
            Response::Device(response) => self.handle_device_response(response),
        }
    }

    fn handle_device_response(&mut self, response: DeviceResponse) -> Task<Message> {
        use DeviceGetResponse as DGR;
        use DeviceResponse as DR;
        use DeviceSetResponse as DSR;

        log::debug!("{:?}", &response);

        self.last_handshake = Instant::now();

        match response {
            DR::Error
            | DR::Get(DGR::Ping)
            | DR::Get(DGR::WifiSettings(_))
            | DR::Set(DSR::Toggle)
            | DR::Set(DSR::TurnOn)
            | DR::Set(DSR::TurnOff)
            | DR::Set(DSR::Preset)
            | DR::Set(DSR::Settings)
            | DR::Set(DSR::WifiSettings)
            | DR::Set(DSR::CurrentPresetSettings)
            | DR::Set(DSR::Brightness)
            | DR::Set(DSR::Speed)
            | DR::Set(DSR::SaveSettings)
            | DR::Set(DSR::Scale) => {}

            DR::Get(DGR::IsOn(is_on)) => {
                self.is_on = is_on;
            }
            DR::Get(DGR::CurrentPresetId(id)) => {
                self.set_selected_preset(&id);
            }
            DR::Get(DGR::PresetInfo(preset_info)) => {
                self.config.set_preset_info(preset_info.clone());
                self.preset = combo_box::State::new(preset_info);
                self.preset_info_message = Some(PresetInfoMessage::PresetInfoLoaded);
                self.save_config();
            }
            DR::Get(DGR::Settings(settings)) => {
                self.is_on = settings.is_on();
                self.set_selected_preset(&settings.current_preset_id());
                let current_preset_settings =
                    &settings.preset_settings()[settings.current_preset_id() as usize];
                self.brightness = current_preset_settings.brightness();
                self.speed = current_preset_settings.speed();
                self.scale = current_preset_settings.scale();

                match serde_json::to_string_pretty(&settings) {
                    Ok(text) => {
                        self.device_settings_content = text_editor::Content::with_text(&text)
                    }
                    Err(err) => log::error!("{err}"),
                }
            }
            DR::Get(DGR::CurrentPresetSettings(preset_settings)) => {
                self.brightness = preset_settings.brightness();
                self.speed = preset_settings.speed();
                self.scale = preset_settings.scale();
            }
        }

        // Fetch device info if it has been reconnected
        match self.is_device_connected {
            false => self.update(Message::Request(Request::Get(GetRequest::Settings))),
            true => Task::none(),
        }
    }

    fn handle_ui_message(&mut self, message: UIMessage) -> Task<Message> {
        match message {
            UIMessage::Brightness(val) => self.brightness = val,
            UIMessage::Speed(val) => self.speed = val,
            UIMessage::Scale(val) => self.scale = val,
            UIMessage::Ip(ip) => self.ip_text = ip,
            UIMessage::Port(port) => self.port_text = port,
            UIMessage::Subnet(subnet) => self.subnet_text = subnet,
            UIMessage::EditDeviceSettings(action) => self.device_settings_content.perform(action),
            UIMessage::IpError => self.ip_port_error_message = Some(IpPortErrorMessage::InvalidIp),
            UIMessage::PortError => {
                self.ip_port_error_message = Some(IpPortErrorMessage::InvalidPort)
            }
            UIMessage::ExportDeviceSettingsError => {
                self.device_settings_error_message =
                    Some(DeviceSettingsErrorMessage::DeviceSettingsDeserialization)
            }
        }
        Task::none()
    }

    fn handle_save(&mut self) -> Task<Message> {
        match self.handle_save_fallible() {
            Ok(task) => task,
            Err(err) => match err {
                Error::AddrParse(_) => self.update(Message::UI(UIMessage::IpError)),
                Error::PortParse(_) => self.update(Message::UI(UIMessage::PortError)),
                _ => Task::none(),
            },
        }
    }

    fn handle_save_fallible(&mut self) -> Result<Task<Message>> {
        self.is_device_connected = false;
        let ip: IpAddr = self.ip_text.parse().map_err(Error::AddrParse)?;
        let port: u16 = self.port_text.parse().map_err(Error::PortParse)?;
        self.config.set_device(Device::new(ip, port));
        self.save_config();
        self.ip_port_error_message = None;
        Ok(self.update(Message::Request(Request::SetDeviceAddr(
            self.config.device().addr(),
        ))))
    }

    fn save_config(&self) {
        if let Err(err) = self.config.save() {
            log::error!("Error saving config: {err}");
        }
    }

    fn handle_import(&mut self) -> Task<Message> {
        self.update(Message::Request(Request::Get(GetRequest::Settings)))
    }

    fn handle_export(&mut self) -> Task<Message> {
        if let Err(Error::DeserializeJson(_)) = self.handle_export_fallible() {
            self.update(Message::UI(UIMessage::ExportDeviceSettingsError))
        } else {
            Task::none()
        }
    }

    fn handle_export_fallible(&mut self) -> Result<Task<Message>> {
        let device_settings: DeviceSettings =
            serde_json::from_str(&self.device_settings_content.text())
                .map_err(Error::DeserializeJson)?;
        Ok(
            self.update(Message::Request(Request::Set(SetRequest::Settings(
                device_settings,
            )))),
        )
    }

    fn handle_detect_device(&mut self) -> Task<Message> {
        self.detector_error_message = None;
        match self.handle_detect_device_fallible() {
            Ok(task) => {
                self.detected_devices = DetectedDevicesState::Loading;
                task
            }
            Err(err) => {
                match err {
                    Error::IpNetworkParse(_) => {
                        self.detector_error_message = Some(DetectorErrorMessage::SubnetParse)
                    }
                    _ => log::error!("{err}"),
                }
                Task::none()
            }
        }
    }

    fn handle_detect_device_fallible(&mut self) -> Result<Task<Message>> {
        let timeout = Duration::from_secs(5);
        let subnet: IpNetwork = self.subnet_text.parse().map_err(Error::IpNetworkParse)?;
        let detector = detector::DeviceDetector::with_subnet(subnet);
        Ok(Task::perform(detector.run_with_timeout(timeout), |res| {
            Message::Settings(SettingsMessage::DetectorOutput(res.unwrap_or_default()))
        }))
    }

    fn handle_detector_output(&mut self, devices: Vec<Device>) -> Task<Message> {
        self.detected_devices = DetectedDevicesState::Devices(devices);
        log::debug!("{:?}", &self.detected_devices);
        Task::none()
    }

    fn handle_set_detected_device(&mut self, device: Device) -> Task<Message> {
        self.ip_text = device.ip().to_string();
        self.port_text = device.port().to_string();
        self.config.set_device(device);
        self.save_config();
        self.is_device_connected = false;
        self.ip_port_error_message = None;
        self.update(Message::Request(Request::SetDeviceAddr(
            self.config.device().addr(),
        )))
    }

    fn home_page(&self) -> Element<Message> {
        let page_title = text!("Smart Lights").size(30);

        let settings_button = button("Settings").on_press(Message::Page(Page::Settings));
        let load_presets_button =
            button("Load Presets").on_press(Message::Request(Request::Get(GetRequest::PresetInfo)));
        let preset_info_message = match &self.preset_info_message {
            Some(msg) => text!("{msg}").color(self.theme().palette().success),
            None => text!(""),
        };

        let device_save_settings_button = self.device_save_settings_button();
        let is_device_connected_text = self.device_connection_state();

        let top_row = row![
            settings_button,
            load_presets_button,
            device_save_settings_button,
            preset_info_message,
            horizontal_space(),
            is_device_connected_text
        ]
        .padding(5)
        .spacing(10)
        .align_y(Center);

        let preset_combo_box = combo_box(
            &self.preset,
            "Preset",
            self.selected_preset.as_ref(),
            |preset| Message::Request(Request::Set(SetRequest::Preset(preset.id()))),
        );

        let toggle_button = if self.is_on {
            button("On").style(button::success)
        } else {
            button("Off").style(button::danger)
        }
        .on_press(Message::Request(Request::Set(SetRequest::Toggle)));

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

    fn settings_page(&self) -> Element<Message> {
        let page_title = text!("Device Settings").size(30);
        let device_save_settings_button = self.device_save_settings_button();

        scrollable(
            column![
                row![
                    button("Back").on_press(Message::Page(Page::Home)),
                    device_save_settings_button,
                    horizontal_space(),
                    self.device_connection_state()
                ]
                .align_y(Center)
                .spacing(10)
                .padding(5),
                row![page_title].padding(5),
                self.view_device_detector_settings(),
                self.view_ip_port_settings(),
                self.view_device_settings(),
            ]
            .spacing(10)
            .padding(10),
        )
        .into()
    }

    fn detected_devices_button_style(
        theme: &Theme,
        status: widget::button::Status,
    ) -> widget::button::Style {
        let text_color = theme.palette().text;
        let border = iced::Border {
            radius: iced::border::radius(10),
            ..Default::default()
        };
        let style_with_background = |bg: iced::Background| button::Style {
            background: Some(bg),
            text_color,
            border,
            ..Default::default()
        };
        match status {
            button::Status::Active => style_with_background(iced::Color::TRANSPARENT.into()),
            button::Status::Hovered => {
                style_with_background(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.05).into())
            }
            button::Status::Pressed => {
                style_with_background(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.1).into())
            }
            _ => button::Style::default(),
        }
    }

    fn view_device_detector_settings(&self) -> Element<Message> {
        let devices = &self.detected_devices;
        let section_title = text!("Detect devices in network").size(24);

        use DetectedDevicesState as DDS;
        let device_widget: Element<Message> = match devices {
            DDS::None => text!("Use detector to detect devices in network").into(),
            DDS::Loading => text!("Loading...").into(),
            DDS::Devices(devices) => match devices.len() {
                0 => text!("No devices found").into(),
                _ => column(devices.iter().map(|device| {
                    button(text(format!("{device}")))
                        .on_press_with(|| {
                            Message::Settings(SettingsMessage::SetDetectedDevice(device.clone()))
                        })
                        .style(Self::detected_devices_button_style)
                        .into()
                }))
                .into(),
            },
        };

        let detect_button =
            button("Detect").on_press(Message::Settings(SettingsMessage::DetectDevice));
        let subnet_intput = text_input("192.168.1.0/24", &self.subnet_text)
            .on_input(|input| Message::UI(UIMessage::Subnet(input)));
        let error_message = match &self.detector_error_message {
            Some(msg) => text!("{msg}").color(self.theme().palette().danger),
            None => text!(""),
        };

        column![
            row![section_title].padding(5),
            column![text!("Subnet:"), subnet_intput].padding(5),
            row![device_widget].padding(5),
            row![
                detect_button,
                Space::with_width(iced::Length::Fill),
                error_message.align_y(Bottom)
            ]
            .spacing(10)
            .padding(5)
        ]
        .into()
    }

    fn view_ip_port_settings(&self) -> Element<Message> {
        let settings = &self.config;
        let section_title = text!("Device IP/Port Settings").size(24);
        let ip_input = text_input("IP", &self.ip_text)
            .on_input(|input| Message::UI(UIMessage::Ip(input)))
            .on_submit(Message::Settings(SettingsMessage::SaveIpPort));
        let port_input = text_input("Post", &self.port_text)
            .on_input(|input| Message::UI(UIMessage::Port(input)))
            .on_submit(Message::Settings(SettingsMessage::SaveIpPort));
        let save_button = button("Save").on_press(Message::Settings(SettingsMessage::SaveIpPort));
        let error_message = match &self.ip_port_error_message {
            Some(msg) => text!("{msg}").color(self.theme().palette().danger),
            None => text!(""),
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
                horizontal_space(),
                error_message.align_y(Bottom)
            ]
            .spacing(10)
            .padding(5),
        ]
        .into()
    }

    fn view_device_settings(&self) -> Element<Message> {
        let section_title = text!("Import/Export Settings").size(24);
        let editor = text_editor(&self.device_settings_content)
            .placeholder("Device settings")
            .on_action(|action| Message::UI(UIMessage::EditDeviceSettings(action)));
        let import_button =
            button("Import").on_press(Message::Settings(SettingsMessage::ImportDeviceSettings));
        let export_button =
            button("Export").on_press(Message::Settings(SettingsMessage::ExportDeviceSettings));

        let error_message = match &self.device_settings_error_message {
            Some(msg) => text!("{msg}").color(self.theme().palette().danger),
            None => text!(""),
        };

        column![
            row![section_title].padding(5),
            editor.padding(5),
            row![
                import_button,
                export_button,
                horizontal_space(),
                error_message,
            ]
            .spacing(10)
            .padding(5)
        ]
        .spacing(10)
        .padding(5)
        .into()
    }

    fn slider_controls(&self) -> Element<Message> {
        column![
            self.brightness_slider(),
            self.speed_slider(),
            self.scale_slider(),
        ]
        .padding(5)
        .into()
    }

    fn brightness_slider(&self) -> Element<Message> {
        let brightness = self.brightness;
        SliderBuilder::new("Brightness:", brightness)
            .on_change(|val| Message::UI(UIMessage::Brightness(val)))
            .on_release(move |_| Message::Request(Request::Set(SetRequest::Brightness(brightness))))
            .build()
    }

    fn speed_slider(&self) -> Element<Message> {
        let speed = self.speed;
        SliderBuilder::new("Speed:", self.speed)
            .on_change(|val| Message::UI(UIMessage::Speed(val)))
            .on_release(move |_| Message::Request(Request::Set(SetRequest::Speed(speed))))
            .build()
    }

    fn scale_slider(&self) -> Element<Message> {
        let scale = self.scale;
        SliderBuilder::new("Scale:", self.scale)
            .on_change(|val| Message::UI(UIMessage::Scale(val)))
            .on_release(move |_| Message::Request(Request::Set(SetRequest::Scale(scale))))
            .build()
    }

    fn device_connection_state(&self) -> Element<'_, Message> {
        match self.is_device_connected {
            true => text!("Connected")
                .color(self.theme.palette().success)
                .into(),
            false => text!("Disconnected")
                .color(self.theme.palette().danger)
                .into(),
        }
    }

    fn device_save_settings_button(&self) -> Element<Message> {
        let message = match self.is_device_connected {
            true => Some(Message::Request(Request::Set(SetRequest::SaveSettings))),
            false => None,
        };
        button(text!("Save Settings"))
            .on_press_maybe(message)
            .into()
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum Page {
    #[default]
    Home,
    Settings,
}

#[derive(Debug, Clone)]
enum UIMessage {
    Brightness(u8),
    Speed(u8),
    Scale(u8),
    Ip(String),
    Port(String),
    Subnet(String),
    EditDeviceSettings(text_editor::Action),
    IpError,
    PortError,
    ExportDeviceSettingsError,
}

#[derive(Debug, Clone)]
enum SettingsMessage {
    SaveIpPort,
    ImportDeviceSettings,
    ExportDeviceSettings,
    DetectDevice,
    DetectorOutput(Vec<Device>),
    SetDetectedDevice(Device),
}

#[derive(Debug, Clone)]
enum DetectedDevicesState {
    Devices(Vec<Device>),
    Loading,
    None,
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

#[derive(Debug, Clone)]
enum DetectorErrorMessage {
    SubnetParse,
}

impl std::fmt::Display for DetectorErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            DetectorErrorMessage::SubnetParse => "Invalid subnet has been entered!",
        };
        write!(f, "{msg}")
    }
}

#[derive(Debug, Clone)]
enum PresetInfoMessage {
    PresetInfoLoaded,
}

impl std::fmt::Display for PresetInfoMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            PresetInfoMessage::PresetInfoLoaded => "Preset info loaded!",
        };
        write!(f, "{msg}")
    }
}

struct SliderBuilder {
    label: String,
    value: u8,
    on_change: Box<dyn Fn(u8) -> Message>,
    on_release: Box<dyn Fn(u8) -> Message>,
}

impl SliderBuilder {
    fn new(label: &str, value: u8) -> Self {
        Self {
            label: label.to_string(),
            value,
            on_change: Box::new(|_| Message::UI(UIMessage::Brightness(0))),
            on_release: Box::new(|_| Message::Request(Request::Set(SetRequest::Brightness(0)))),
        }
    }

    fn on_change<F>(mut self, f: F) -> Self
    where
        F: Fn(u8) -> Message + 'static,
    {
        self.on_change = Box::new(f);
        self
    }

    fn on_release<F>(mut self, f: F) -> Self
    where
        F: Fn(u8) -> Message + 'static,
    {
        self.on_release = Box::new(f);
        self
    }

    fn build(self) -> Element<'static, Message> {
        let on_change = self.on_change;
        let on_release = self.on_release;
        row![
            text(self.label),
            slider(0..=255, self.value, move |val| on_change(val))
                .on_release(on_release(self.value)),
            text!("{}", self.value).width(40),
        ]
        .padding(5)
        .spacing(20)
        .into()
    }
}

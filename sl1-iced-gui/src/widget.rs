use iced::{
    Element,
    widget::{slider, text_input},
};

use crate::Message;

const DEFAULT_SETTING_VALUE: u8 = 128;

pub trait Slider<T> {
    type Message;
    fn view(&self) -> Element<Message>;
    fn value(&self) -> T;
    fn set_value(&mut self, value: T);
}

#[derive(Debug, Clone, Copy)]
pub struct SettingSlider {
    value: u8,
}

impl Default for SettingSlider {
    fn default() -> Self {
        Self { value: 128 }
    }
}

impl Slider<u8> for SettingSlider {
    type Message = crate::Message;
    fn view(&self) -> Element<Message> {
        slider(0..=255, self.value, |val| {
            Message::Device(crate::DeviceMessage::SetBrightness(val))
        })
        .into()
    }

    fn value(&self) -> u8 {
        self.value
    }

    fn set_value(&mut self, value: u8) {
        self.value = value;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpeedSlider {
    value: u8,
}

impl Default for SpeedSlider {
    fn default() -> Self {
        Self { value: 128 }
    }
}

impl Slider<u8> for SpeedSlider {
    type Message = crate::Message;
    fn view(&self) -> Element<Message> {
        slider(0..=255, self.value, |val| {
            Message::Device(crate::DeviceMessage::SetSpeed(val))
        })
        .into()
    }

    fn value(&self) -> u8 {
        self.value
    }

    fn set_value(&mut self, value: u8) {
        self.value = value;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScaleSlider {
    value: u8,
}

impl Default for ScaleSlider {
    fn default() -> Self {
        Self { value: 128 }
    }
}

impl Slider<u8> for ScaleSlider {
    type Message = crate::Message;
    fn view(&self) -> Element<Message> {
        slider(0..=255, self.value, |val| {
            Message::Device(crate::DeviceMessage::SetScale(val))
        })
        .into()
    }

    fn value(&self) -> u8 {
        self.value
    }

    fn set_value(&mut self, value: u8) {
        self.value = value;
    }
}

pub trait NumberInput<T> {
    type Message;
    fn view(&self) -> Element<Message>;
    fn value(&self) -> T;
    fn set_value(&mut self, value: T);
}

#[derive(Debug, Clone, Copy)]
pub struct BrightnessInput {
    value: u8,
}

impl Default for BrightnessInput {
    fn default() -> Self {
        Self { value: 128 }
    }
}

impl NumberInput<u8> for BrightnessInput {
    type Message = crate::Message;
    fn view(&self) -> Element<Message> {
        text_input(
            DEFAULT_SETTING_VALUE.to_string().as_str(),
            self.value.to_string().as_str(),
        )
        .into()
    }

    fn value(&self) -> u8 {
        self.value
    }

    fn set_value(&mut self, value: u8) {
        self.value = value;
    }
}

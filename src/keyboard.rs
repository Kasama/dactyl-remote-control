use anyhow::anyhow;
use hidapi::HidApi;
use log::trace;

const REPORT_LENGTH: usize = 32;

#[derive(Debug)]
pub struct HidInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub usage_page: u16,
    pub usage: u16,
}

#[derive(Debug)]
pub enum Layers {
    Qwerty = 0,
    Workman = 1,
    Sys = 2,
    Numrow = 3,
    Game = 5,
    Code = 6,
    Numpad = 7,
    LayerSel = 14,
    Trans = 15,
    Unknown = 255,
}

impl From<u8> for Layers {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Qwerty,
            1 => Self::Workman,
            2 => Self::Sys,
            3 => Self::Numrow,
            5 => Self::Game,
            6 => Self::Code,
            7 => Self::Numpad,
            14 => Self::LayerSel,
            15 => Self::Trans,
            _ => Self::Unknown,
        }
    }
}

impl ToString for Layers {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

pub enum Operation {
    Bootloader,
    GetLayer,
    ChangeLayer(u8),
}

impl Operation {
    fn report(&self) -> [u8; REPORT_LENGTH] {
        let mut ret = [0; REPORT_LENGTH];
        match self {
            Self::Bootloader => {
                ret[0] = 0x42;
            }
            Self::GetLayer => {
                ret[0] = 0x43;
            }
            Self::ChangeLayer(layer) => {
                ret[0] = 0x44;
                ret[1] = *layer;
            }
        }
        ret
    }
}

pub enum KeyboardResponse {
    None,
    CurrentLayer(u8),
}

impl KeyboardResponse {
    pub fn parse_response(buffer: [u8; REPORT_LENGTH]) -> Self {
        match buffer {
            [0x43, layer, ..] | [0x44, layer, ..] => Self::CurrentLayer(layer),
            _ => Self::None,
        }
    }
}

pub struct Keyboard {
    device: hidapi::HidDevice,
}

pub type Result<T> = std::result::Result<T, anyhow::Error>;

trait TransposableResult<T, U> {
    fn transpose(self) -> std::result::Result<U, T>;
}

impl<T, U> TransposableResult<T, U> for std::result::Result<T, U> {
    fn transpose(self) -> std::result::Result<U, T> {
        match self {
            Ok(o) => Err(o),
            Err(e) => Ok(e),
        }
    }
}

impl Keyboard {
    pub fn new(hid_info: &HidInfo) -> Result<Self> {
        match HidApi::new() {
            Ok(api) => {
                let device = api
                    .device_list()
                    .find(|device| {
                        device.vendor_id() == hid_info.vendor_id
                            && device.product_id() == hid_info.product_id
                            && device.usage_page() == hid_info.usage_page
                            && device.usage() == hid_info.usage
                    })
                    .expect("Unable to find expected device");

                let macropad = api
                    .open_path(device.path())
                    .expect("Could not open HID device");

                Ok(Keyboard { device: macropad })
            }
            Err(e) => Err(anyhow!(e)),
        }
    }

    pub fn send_message(&self, operation: crate::Operation) -> Result<KeyboardResponse> {
        let mut buffer = [0u8; REPORT_LENGTH + 1];

        buffer[1..].copy_from_slice(&operation.report());

        trace!("Writing: {:02x?}", buffer);

        let wrote = self
            .device
            .write(&buffer)
            .expect("Could not write to HID device");

        trace!("Wrote: {wrote:02x?} bytes");

        let mut resp_buf = [0u8; REPORT_LENGTH];

        let response = self
            .device
            .read_timeout(&mut resp_buf, 1000)
            .map(|_| ())
            .transpose()
            .and_then(|e| {
                if e.to_string().contains("device disconnected") {
                    Err(())
                } else {
                    Ok(e)
                }
            })
            .transpose()
            .map(|_| KeyboardResponse::parse_response(resp_buf))?;

        trace!("Response: {:02x?}", resp_buf);

        Ok(response)
    }
}

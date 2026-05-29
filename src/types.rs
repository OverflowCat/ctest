use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SampleMode {
    Quantitative = 0,
    Timed = 1,
}

impl SampleMode {
    pub fn from_flag(flag: &str) -> Option<Self> {
        match flag.to_ascii_lowercase().as_str() {
            "quant" | "quantitative" | "0" => Some(Self::Quantitative),
            "timed" | "time" | "1" => Some(Self::Timed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Idle,
    Waiting,
    Preparing,
    Sampling,
    TempDefrost,
    Defrosting,
    Paused,
    Stopped,
    Cooling,
    Unknown(u8),
}

impl DeviceStatus {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x00 => Self::Idle,
            0x01 => Self::Waiting,
            0x03 => Self::Preparing,
            0x04 => Self::Sampling,
            0x05 => Self::TempDefrost,
            0x06 => Self::Defrosting,
            0x07 => Self::Paused,
            0x08 => Self::Stopped,
            0x09 => Self::Cooling,
            other => Self::Unknown(other),
        }
    }
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "空闲"),
            Self::Waiting => write!(f, "等待"),
            Self::Preparing => write!(f, "准备"),
            Self::Sampling => write!(f, "采样"),
            Self::TempDefrost => write!(f, "临时化霜"),
            Self::Defrosting => write!(f, "化霜"),
            Self::Paused => write!(f, "暂停"),
            Self::Stopped => write!(f, "停止"),
            Self::Cooling => write!(f, "冷却"),
            Self::Unknown(v) => write!(f, "未知({:#04X})", v),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DeviceDateTime {
    pub fn from_bytes(b: &[u8]) -> Self {
        Self {
            year: 2000 + b[0] as u16,
            month: b[1],
            day: b[2],
            hour: b[3],
            minute: b[4],
            second: if b.len() >= 6 { b[5] } else { 0 },
        }
    }
}

impl std::fmt::Display for DeviceDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

pub fn parse_f32_be(b: &[u8]) -> f32 {
    f32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::transport::HdTransport;
use crate::types::{DeviceDateTime, DeviceStatus, SampleMode, parse_f32_be};

const DEVICE_TYPE: u8 = 0x04;

pub const BIT_ENV_TEMP: u16 = 1 << 1;
pub const BIT_INSTANT_FLOW: u16 = 1 << 3;
pub const BIT_ENV_HUMIDITY: u16 = 1 << 5;
pub const BIT_PRESSURE: u16 = 1 << 6;
pub const BIT_STANDARD_VOLUME: u16 = 1 << 7;
pub const BIT_STANDARD_MODE: u16 = 1 << 8;
pub const BIT_WORKING_VOLUME: u16 = 1 << 9;

pub const INSTANT_MASK_DEFAULT: u16 = BIT_ENV_TEMP
    | BIT_INSTANT_FLOW
    | BIT_ENV_HUMIDITY
    | BIT_PRESSURE
    | BIT_STANDARD_VOLUME
    | BIT_STANDARD_MODE
    | BIT_WORKING_VOLUME;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C14InstantData {
    pub env_temperature_c: Option<f32>,
    pub instant_flow_standard_lpm: Option<f32>,
    pub env_humidity_pct: Option<f32>,
    pub pressure_kpa: Option<f32>,
    pub standard_volume_l: Option<f32>,
    pub standard_mode_c: Option<i16>,
    pub working_volume_l: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C14HistoryRecord {
    pub index: u16,
    pub start_time: DeviceDateTime,
    pub end_time: DeviceDateTime,
    pub avg_flow_lpm: f32,
    pub standard_volume_l: f32,
    pub pressure_kpa: f32,
    pub standard_mode_c: i16,
    pub set_flow_lpm: f32,
    pub working_volume_l: f32,
    pub env_temperature_c: f32,
}

pub struct C14Sampler {
    transport: HdTransport,
}

impl C14Sampler {
    pub fn new(transport: HdTransport) -> Self {
        Self { transport }
    }

    pub const fn device_type() -> u8 {
        DEVICE_TYPE
    }

    pub async fn query_history_count(&mut self) -> Result<u16> {
        let payload = self.transport.query(0xA0).await?;
        if payload.len() < 5 {
            return Err(anyhow!("query_history_count: short response"));
        }
        Ok(u16::from_be_bytes([payload[3], payload[4]]))
    }

    pub async fn read_history(&mut self, index: u16) -> Result<C14HistoryRecord> {
        let payload = self.transport.send_recv(0xB0, &index.to_be_bytes()).await?;
        if payload.len() < 4 + 57 {
            return Err(anyhow!("read_history: short response {}", payload.len()));
        }
        parse_history_record(index, &payload[4..])
    }

    pub async fn query_status(&mut self) -> Result<DeviceStatus> {
        let payload = self.transport.query(0xC0).await?;
        if payload.len() < 4 {
            return Err(anyhow!("query_status: short response"));
        }
        Ok(DeviceStatus::from_byte(payload[3]))
    }

    pub async fn query_device_time(&mut self) -> Result<DeviceDateTime> {
        let payload = self.transport.query(0xC2).await?;
        if payload.len() < 9 {
            return Err(anyhow!("query_device_time: short response"));
        }
        Ok(DeviceDateTime::from_bytes(&payload[3..9]))
    }

    pub async fn query_instant(&mut self, mask: u16) -> Result<C14InstantData> {
        let payload = self.transport.send_recv(0xD0, &mask.to_be_bytes()).await?;
        if payload.len() < 4 {
            return Err(anyhow!("query_instant: short response"));
        }
        parse_instant_data(mask, &payload[4..])
    }

    pub async fn start_sampling(&mut self, mode: SampleMode) -> Result<()> {
        let payload = self.transport.send_recv(0xF0, &[mode as u8]).await?;
        check_response_ok(&payload, 0xF1, "start_sampling")
    }

    pub async fn stop_sampling(&mut self) -> Result<()> {
        let payload = self.transport.query(0x20).await?;
        check_response_ok(&payload, 0x21, "stop_sampling")
    }

    pub async fn set_cycle_time(&mut self, hours: u8, minutes: u8) -> Result<()> {
        if hours > 9 || minutes > 59 {
            return Err(anyhow!("invalid cycle time {}h{}m", hours, minutes));
        }
        let payload = self.transport.send_recv(0x26, &[hours, minutes]).await?;
        check_response_ok(&payload, 0x27, "set_cycle_time")
    }

    pub async fn set_scheduled_start(
        &mut self,
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
    ) -> Result<()> {
        let yy = (year.saturating_sub(2000)) as u8;
        let payload = self
            .transport
            .send_recv(0x28, &[yy, month, day, hour, minute])
            .await?;
        check_response_ok(&payload, 0x28, "set_scheduled_start")
    }

    pub async fn set_sample_flow(&mut self, flow_lpm: f32) -> Result<()> {
        let payload = self.transport.send_recv(0x30, &flow_lpm.to_be_bytes()).await?;
        check_response_ok(&payload, 0x31, "set_sample_flow")
    }

    pub async fn set_sample_duration(&mut self, hours: u16, minutes: u8) -> Result<()> {
        if hours > 999 || minutes > 59 {
            return Err(anyhow!("invalid sample duration {}h{}m", hours, minutes));
        }
        let hb = hours.to_be_bytes();
        let payload = self
            .transport
            .send_recv(0x40, &[hb[0], hb[1], minutes])
            .await?;
        check_response_ok(&payload, 0x41, "set_sample_duration")
    }

    pub async fn set_device_time(
        &mut self,
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<()> {
        let yy = (year.saturating_sub(2000)) as u8;
        let payload = self
            .transport
            .send_recv(0x60, &[yy, month, day, hour, minute, second])
            .await?;
        check_response_ok(&payload, 0x61, "set_device_time")
    }

    pub async fn link_test(&mut self) -> Result<()> {
        let payload = self.transport.query(0xFA).await?;
        if payload.first() != Some(&0xFB) {
            return Err(anyhow!("link_test failed: unexpected DF {:?}", payload.first()));
        }
        Ok(())
    }
}

fn parse_history_record(index: u16, d: &[u8]) -> Result<C14HistoryRecord> {
    if d.len() < 57 {
        return Err(anyhow!("history record too short: {}", d.len()));
    }
    Ok(C14HistoryRecord {
        index,
        start_time: DeviceDateTime::from_bytes(&d[19..24]),
        end_time: DeviceDateTime::from_bytes(&d[24..29]),
        avg_flow_lpm: parse_f32_be(&d[29..33]),
        standard_volume_l: parse_f32_be(&d[33..37]),
        pressure_kpa: parse_f32_be(&d[39..43]),
        standard_mode_c: i16::from_be_bytes([d[43], d[44]]),
        set_flow_lpm: parse_f32_be(&d[45..49]),
        working_volume_l: parse_f32_be(&d[49..53]),
        env_temperature_c: parse_f32_be(&d[53..57]),
    })
}

fn parse_instant_data(mask: u16, data: &[u8]) -> Result<C14InstantData> {
    let mut pos = 0usize;
    let mut out = C14InstantData {
        env_temperature_c: None,
        instant_flow_standard_lpm: None,
        env_humidity_pct: None,
        pressure_kpa: None,
        standard_volume_l: None,
        standard_mode_c: None,
        working_volume_l: None,
    };

    let take_f32 = |buf: &[u8], p: &mut usize| -> Result<f32> {
        if *p + 4 > buf.len() {
            return Err(anyhow!("instant payload truncated"));
        }
        let v = parse_f32_be(&buf[*p..*p + 4]);
        *p += 4;
        Ok(v)
    };

    if (mask & (1 << 0)) != 0 {
        pos += 2;
    }
    if (mask & BIT_ENV_TEMP) != 0 {
        out.env_temperature_c = Some(take_f32(data, &mut pos)?);
    }
    if (mask & BIT_INSTANT_FLOW) != 0 {
        out.instant_flow_standard_lpm = Some(take_f32(data, &mut pos)?);
    }
    if (mask & (1 << 4)) != 0 {
        pos += 2;
    }
    if (mask & BIT_ENV_HUMIDITY) != 0 {
        out.env_humidity_pct = Some(take_f32(data, &mut pos)?);
    }
    if (mask & BIT_PRESSURE) != 0 {
        out.pressure_kpa = Some(take_f32(data, &mut pos)?);
    }
    if (mask & BIT_STANDARD_VOLUME) != 0 {
        out.standard_volume_l = Some(take_f32(data, &mut pos)?);
    }
    if (mask & BIT_STANDARD_MODE) != 0 {
        if pos + 2 > data.len() {
            return Err(anyhow!("instant payload truncated"));
        }
        out.standard_mode_c = Some(i16::from_be_bytes([data[pos], data[pos + 1]]));
        pos += 2;
    }
    if (mask & BIT_WORKING_VOLUME) != 0 {
        out.working_volume_l = Some(take_f32(data, &mut pos)?);
    }

    Ok(out)
}

fn check_response_ok(payload: &[u8], expected_df: u8, op: &str) -> Result<()> {
    if payload.is_empty() {
        return Err(anyhow!("{}: empty response", op));
    }
    if payload[0] != expected_df {
        return Err(anyhow!(
            "{}: unexpected DF {:#04X}, expected {:#04X}",
            op,
            payload[0],
            expected_df
        ));
    }
    if payload.last() == Some(&0xFF) {
        return Err(anyhow!("{}: device returned failure", op));
    }
    Ok(())
}

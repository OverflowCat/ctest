use std::time::Duration;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use ctest::transport::HdTransport;
use ctest::tritium::{INSTANT_MASK_DEFAULT, TritiumSampler};
use ctest::types::SampleMode;

#[derive(Parser, Debug)]
#[command(name = "tritium-test", about = "ZC-Q0152 氚采样器串口验证工具 (TCP/ZLAN)")]
struct Cli {
    #[arg(short, long, default_value = "192.168.100.215:4196")]
    addr: String,

    #[arg(long, default_value_t = 5)]
    timeout: u64,

    #[arg(long, default_value_t = 3)]
    read_timeout: u64,

    #[arg(long)]
    raw: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    Test,
    Status,
    TimeGet,
    TimeSet {
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    },
    Instant {
        #[arg(long)]
        mask: Option<String>,
    },
    HistoryCount,
    History {
        index: u16,
    },
    Start {
        #[arg(long, default_value = "timed")]
        mode: String,
    },
    Stop,
    SetColdTrap {
        temp_c: f32,
    },
    SetDefrost {
        hours: u8,
        minutes: u8,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let transport = HdTransport::connect(
        &cli.addr,
        TritiumSampler::device_type(),
        Duration::from_secs(cli.timeout),
        Duration::from_secs(cli.read_timeout),
        cli.raw,
    )
    .await?;
    let mut dev = TritiumSampler::new(transport);

    match cli.cmd {
        Cmd::Test => {
            dev.link_test().await?;
            println!("OK: 通讯正常");
        }
        Cmd::Status => {
            let s = dev.query_status().await?;
            println!("status: {}", s);
        }
        Cmd::TimeGet => {
            let t = dev.query_device_time().await?;
            println!("device_time: {}", t);
        }
        Cmd::TimeSet {
            year,
            month,
            day,
            hour,
            minute,
            second,
        } => {
            dev.set_device_time(year, month, day, hour, minute, second)
                .await?;
            println!("OK: time updated");
        }
        Cmd::Instant { mask } => {
            let mask = parse_mask(mask)?;
            let data = dev.query_instant(mask).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        Cmd::HistoryCount => {
            let n = dev.query_history_count().await?;
            println!("history_count: {}", n);
        }
        Cmd::History { index } => {
            let rec = dev.read_history(index).await?;
            println!("{}", serde_json::to_string_pretty(&rec)?);
        }
        Cmd::Start { mode } => {
            let m = SampleMode::from_flag(&mode)
                .ok_or_else(|| anyhow!("invalid mode '{}', use timed|quant", mode))?;
            dev.start_sampling(m).await?;
            println!("OK: sampling started");
        }
        Cmd::Stop => {
            dev.stop_sampling().await?;
            println!("OK: sampling stopped");
        }
        Cmd::SetColdTrap { temp_c } => {
            dev.set_cold_trap_temp(temp_c).await?;
            println!("OK: cold trap temperature set to {} C", temp_c);
        }
        Cmd::SetDefrost { hours, minutes } => {
            dev.set_defrost_time(hours, minutes).await?;
            println!("OK: defrost set to {:02}:{:02}", hours, minutes);
        }
    }

    Ok(())
}

fn parse_mask(mask: Option<String>) -> Result<u16> {
    let Some(raw) = mask else {
        return Ok(INSTANT_MASK_DEFAULT);
    };
    let s = raw.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return Ok(u16::from_str_radix(hex, 16)?);
    }
    Ok(s.parse::<u16>()?)
}

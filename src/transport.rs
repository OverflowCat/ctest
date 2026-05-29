use anyhow::{Context, Result, anyhow};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};

const SOF: u8 = 0x68;
const EOF: u8 = 0x0D;

pub struct HdTransport {
    stream: TcpStream,
    device_type: u8,
    read_timeout: Duration,
    raw: bool,
}

impl HdTransport {
    pub async fn connect(
        addr: &str,
        device_type: u8,
        connect_timeout: Duration,
        read_timeout: Duration,
        raw: bool,
    ) -> Result<Self> {
        let target = addr.trim().trim_start_matches("tcp://");
        let stream = timeout(connect_timeout, TcpStream::connect(target))
            .await
            .map_err(|_| anyhow!("connect timeout: {}", target))?
            .with_context(|| format!("connect failed: {}", target))?;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream,
            device_type,
            read_timeout,
            raw,
        })
    }

    pub async fn query(&mut self, func: u8) -> Result<Vec<u8>> {
        self.send_recv(func, &[0x00]).await
    }

    pub async fn send_recv(&mut self, func: u8, ddf_and_data: &[u8]) -> Result<Vec<u8>> {
        let frame = build_frame(func, self.device_type, ddf_and_data);
        if self.raw {
            eprintln!(">> {}", to_hex(&frame));
        }
        self.stream.write_all(&frame).await.context("write failed")?;
        let payload = read_frame(&mut self.stream, self.read_timeout).await?;
        if self.raw {
            eprintln!("<< {}", to_hex_payload(&payload));
        }
        Ok(payload)
    }
}

async fn read_frame(stream: &mut TcpStream, read_timeout: Duration) -> Result<Vec<u8>> {
    let mut header = [0u8; 4];
    timeout(read_timeout, stream.read_exact(&mut header))
        .await
        .context("read header timeout")??;

    if header[0] != SOF || header[3] != SOF {
        return Err(anyhow!("invalid SOF header: {:02X?}", header));
    }
    if header[1] != header[2] {
        return Err(anyhow!(
            "length mismatch: header len bytes {} != {}",
            header[1],
            header[2]
        ));
    }

    let len = header[1] as usize;
    let mut tail = vec![0u8; len + 2];
    timeout(read_timeout, stream.read_exact(&mut tail))
        .await
        .context("read tail timeout")??;

    let payload = &tail[..len];
    let ch = tail[len];
    let eof = tail[len + 1];
    if eof != EOF {
        return Err(anyhow!("invalid EOF: {:#04X}", eof));
    }

    let calc = checksum(payload);
    if calc != ch {
        return Err(anyhow!(
            "checksum mismatch: expected {:#04X}, got {:#04X}",
            calc,
            ch
        ));
    }

    Ok(payload.to_vec())
}

pub fn build_frame(func: u8, device_type: u8, ddf_and_data: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(2 + ddf_and_data.len());
    payload.push(func);
    payload.push(device_type);
    payload.extend_from_slice(ddf_and_data);

    let n = payload.len() as u8;
    let ch = checksum(&payload);
    let mut frame = Vec::with_capacity(payload.len() + 6);
    frame.push(SOF);
    frame.push(n);
    frame.push(n);
    frame.push(SOF);
    frame.extend_from_slice(&payload);
    frame.push(ch);
    frame.push(EOF);
    frame
}

fn checksum(payload: &[u8]) -> u8 {
    (payload.iter().map(|&b| b as u32).sum::<u32>() & 0xFF) as u8
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn to_hex_payload(payload: &[u8]) -> String {
    let mut out = Vec::with_capacity(payload.len() + 6);
    let n = payload.len() as u8;
    out.push(SOF);
    out.push(n);
    out.push(n);
    out.push(SOF);
    out.extend_from_slice(payload);
    out.push(checksum(payload));
    out.push(EOF);
    to_hex(&out)
}

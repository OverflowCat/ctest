# ctest

Standalone 串口验证工具（通过 TCP/ZLAN 转串口）

包含两个可执行程序：
- `tritium-test`：ZC-Q0152 氚采样器（ST=0x03）
- `c14-test`：ZC-Q1401 C14 采样器（ST=0x04）

## Build

```powershell
cd d:\ctest
cargo build
```

## Tritium 示例

```powershell
# 通讯测试 (0xFA)
cargo run --bin tritium-test -- --addr 192.168.100.215:4196 test

# 查询状态 (0xC0)
cargo run --bin tritium-test -- --addr 192.168.100.215:4196 status

# 查询瞬时数据 (0xD0, 默认掩码)
cargo run --bin tritium-test -- --addr 192.168.100.215:4196 instant

# 读取第 1 条历史记录 (0xB0)
cargo run --bin tritium-test -- --addr 192.168.100.215:4196 history 1
```

## C14 示例

```powershell
# 通讯测试 (0xFA)
cargo run --bin c14-test -- --addr 192.168.100.215:4196 test

# 查询状态 (0xC0)
cargo run --bin c14-test -- --addr 192.168.100.215:4196 status

# 查询瞬时数据 (0xD0, 默认掩码)
cargo run --bin c14-test -- --addr 192.168.100.215:4196 instant

# 设置采样流量 (0x30)
cargo run --bin c14-test -- --addr 192.168.100.215:4196 set-flow 0.6
```

## 常用参数

- `--addr`：TCP 地址，格式 `host:port`
- `--timeout`：连接超时（秒）
- `--read-timeout`：读超时（秒）
- `--raw`：打印收发帧十六进制

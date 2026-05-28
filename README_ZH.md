# Serial MCP Server

基于 [Model Context Protocol](https://modelcontextprotocol.io) 的串口通信服务器。将串口操作暴露为 MCP 工具，使 AI 助手能够发现、连接串口设备并与之通信——包括微控制器、嵌入式系统、传感器和工业设备。

## 工具

服务器注册五个 MCP 工具：

### `list_ports`

列出系统可用串口。返回端口名、描述和硬件 ID（USB 转串口适配器会显示 USB VID/PID）。

无参数。

### `open`

打开串口连接。

| 参数 | 类型 | 默认值 | 说明 |
|-----------|------|---------|-------------|
| `port` | string | — | 端口名（如 `COM13`、`/dev/ttyUSB0`） |
| `baud_rate` | u32 | — | 波特率 |
| `data_bits` | string | `"8"` | 数据位：`"5"`, `"6"`, `"7"`, `"8"` |
| `stop_bits` | string | `"1"` | 停止位：`"1"`, `"2"` |
| `parity` | string | `"none"` | 校验位：`"none"`, `"odd"`, `"even"` |
| `flow_control` | string | `"none"` | 流控：`"none"`, `"software"`, `"hardware"` |

返回 `connection_id`（UUID），用于后续操作。

支持的波特率：300, 600, 1200, 2400, 4800, 9600, 14400, 19200, 28800, 38400, 57600, 115200, 230400, 460800, 921600。

### `close`

关闭串口连接。

| 参数 | 类型 | 说明 |
|-----------|------|-------------|
| `connection_id` | string | `open` 返回的连接 ID |

### `write`

发送数据到串口设备。

| 参数 | 类型 | 默认值 | 说明 |
|-----------|------|---------|-------------|
| `connection_id` | string | — | 连接 ID |
| `data` | string | — | 待发送数据 |
| `encoding` | string | `"utf8"` | 编码：`"utf8"`, `"hex"`, `"base64"` |

数据先按指定编码解码后再发送。Hex 编码支持空格分隔的字节（如 `"0D 0A"`）。

### `read`

从串口设备读取数据。

| 参数 | 类型 | 默认值 | 说明 |
|-----------|------|---------|-------------|
| `connection_id` | string | — | 连接 ID |
| `timeout_ms` | u64 | — | 读取超时毫秒数（可选） |
| `max_bytes` | usize | 1024 | 最大读取字节数 |
| `encoding` | string | `"utf8"` | 输出编码：`"utf8"`, `"hex"`, `"base64"` |

读取的数据按指定编码格式化后返回。超时返回成功结果（读取字节数为 0），而非错误。

## 架构

```
MCP 客户端 (Claude 等)
    │  JSON-RPC over stdio
    ▼
┌─────────────────────────┐
│  SerialHandler          │  rmcp 宏: #[tool], #[tool_router]
│  (tools/serial_handler) │  注册 5 个 MCP 工具
└───────────┬─────────────┘
            │
┌───────────▼─────────────┐
│  ConnectionManager      │  管理并发连接
│  (serial/mod)           │  基于 Arc<RwLock<HashMap>>
└───────────┬─────────────┘
            │
┌───────────▼─────────────┐
│  SerialConnection       │  异步读写，可配置超时
│  (serial/connection)    │  封装 tokio-serial SerialStream
└───────────┬─────────────┘
            │
┌───────────▼─────────────┐
│  tokio-serial           │  基于 Tokio 的异步串口 I/O
│  serialport             │  端口枚举与同步备选
└─────────────────────────┘
```

### 依赖

- **rmcp** — MCP 协议实现，提供 `#[tool]` / `#[tool_router]` / `#[tool_handler]` 宏系统
- **tokio-serial** — 基于 Tokio 的异步串口 I/O
- **serialport** — 跨平台串口端口枚举
- **clap** — 命令行参数解析（derive 模式）

### 模块结构

| 模块 | 用途 |
|--------|---------|
| `tools/serial_handler` | MCP 工具定义和 ServerHandler 实现 |
| `tools/types` | 请求/响应结构体及编解码辅助函数 |
| `serial/connection` | `SerialConnection` — 带超时的异步串口 I/O |
| `serial/port` | `PortInfo` — 系统端口枚举 |
| `serial/mod` | `ConnectionManager` — 并发连接生命周期管理 |
| `session` | 会话管理，含空闲清理和统计追踪 |
| `config` | CLI 参数、TOML 配置、验证与合并 |
| `error` | 基于 `thiserror` 的统一错误体系 |
| `utils` | 数据编码、校验和、缓冲区工具、校验器 |

## 安装

### 前置条件

- Rust 工具链 1.70+
- 串口设备或 USB 转串口适配器及相应驱动

### 从源码构建

```bash
git clone https://github.com/adancurusul/serial-mcp-server.git
cd serial-mcp-server
cargo build --release
```

二进制文件位于 `target/release/serial-mcp-server`（Windows 为 `.exe`）。

## 配置

### MCP 客户端设置

将服务器添加到 MCP 客户端配置文件。服务器通过 stdio 通信。

**Claude Desktop** (`claude_desktop_config.json`)：

```json
{
  "mcpServers": {
    "serial": {
      "command": "/path/to/serial-mcp-server",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

**Claude Code** (`.claude/settings.json` 或项目设置)：

```json
{
  "mcpServers": {
    "serial": {
      "command": "target/release/serial-mcp-server"
    }
  }
}
```

### 命令行参考

```
serial-mcp-server [OPTIONS]

选项：
  -c, --config <PATH>            TOML 配置文件路径
      --log-level <LEVEL>        日志级别：error, warn, info, debug, trace [默认: info]
      --log-file <PATH>          将日志写入文件而非 stderr
      --max-connections <N>      最大并发连接数 [默认: 10]
      --connection-timeout <S>   连接超时秒数 [默认: 30]
      --default-baud-rate <R>    默认波特率 [默认: 115200]
      --default-timeout-ms <MS>  默认操作超时毫秒数 [默认: 1000]
      --max-buffer-size <N>      最大读取缓冲区字节数 [默认: 8192]
      --retry-count <N>          连接重试次数 [默认: 3]
      --auto-discovery           启用自动端口发现
      --allow-port-sharing       允许同一端口多个连接
      --restrict-ports           限制端口访问至允许列表
      --generate-config          打印默认 TOML 配置并退出
      --validate-config          验证配置并退出
      --show-config              打印当前配置并退出
```

### 配置文件 (TOML)

生成默认配置文件：

```bash
serial-mcp-server --generate-config > config.toml
```

命令行参数会在运行时覆盖配置文件中的对应值。

## STM32 示例

仓库包含 STM32 固件示例（`examples/STM32_demo/`），基于 USART1 实现了交互命令接口，115200 baud 8N1：

| 命令 | 功能 |
|---------|----------|
| `H` | 显示帮助菜单 |
| `L` | 切换 LED (PB7) |
| `C` | 显示并递增计数器 |
| `R` | 复位计数器 |
| `B` | LED 闪烁 3 次 |
| 其他字符 | 回显 |

构建并烧录固件：

```bash
cd examples/STM32_demo
cargo run --release
```

## 开发

```bash
cargo build                          # Debug 构建
cargo test --lib                     # 仅单元测试（不依赖硬件）
cargo test -- --nocapture            # 全部测试，含硬件相关
cargo test <test_name> -- --nocapture  # 运行单项测试
```

### 重要：Future 导入

`rmcp` 0.3.2 的 `#[tool]` 宏生成的代码引用了 `Future`。每个使用 `#[tool]` 的文件必须导入：

```rust
use std::future::Future;
```

项目需要 Rust 1.95+，因为该版本对宏穷举性检查更为严格。

## 平台支持

| 平台 | 端口命名 | 示例 |
|----------|-------------|----------|
| Windows | `COMx` | COM1, COM13 |
| Linux | `/dev/ttyXXX` | /dev/ttyUSB0, /dev/ttyACM0 |
| macOS | `/dev/tty.*` | /dev/tty.usbserial-1234 |

## 许可证

MIT。详见 [LICENSE](LICENSE)。

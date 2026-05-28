# Serial MCP Server

A [Model Context Protocol](https://modelcontextprotocol.io) server that exposes serial port communication as MCP tools. Enables AI assistants to discover, connect to, and communicate with serial devices — microcontrollers, embedded systems, sensors, and industrial equipment.

## Tools

The server registers five MCP tools:

### `list_ports`

Discover available serial ports on the host system. Returns port name, description, and hardware ID (USB VID/PID for USB-serial adapters).

No parameters.

### `open`

Open a serial port connection.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `port` | string | — | Port name (e.g. `COM13`, `/dev/ttyUSB0`) |
| `baud_rate` | u32 | — | Baud rate |
| `data_bits` | string | `"8"` | Data bits: `"5"`, `"6"`, `"7"`, `"8"` |
| `stop_bits` | string | `"1"` | Stop bits: `"1"`, `"2"` |
| `parity` | string | `"none"` | Parity: `"none"`, `"odd"`, `"even"` |
| `flow_control` | string | `"none"` | Flow control: `"none"`, `"software"`, `"hardware"` |

Returns a `connection_id` (UUID) used for subsequent operations.

Supported baud rates: 300, 600, 1200, 2400, 4800, 9600, 14400, 19200, 28800, 38400, 57600, 115200, 230400, 460800, 921600.

### `close`

Close an open serial connection.

| Parameter | Type | Description |
|-----------|------|-------------|
| `connection_id` | string | Connection ID returned by `open` |

### `write`

Send data to a serial device.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `connection_id` | string | — | Connection ID |
| `data` | string | — | Data to send |
| `encoding` | string | `"utf8"` | Encoding: `"utf8"`, `"hex"`, `"base64"` |

Data is decoded from the specified encoding before transmission. Hex input accepts space-separated bytes (e.g. `"0D 0A"`).

### `read`

Read data from a serial device.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `connection_id` | string | — | Connection ID |
| `timeout_ms` | u64 | — | Read timeout in milliseconds (optional) |
| `max_bytes` | usize | 1024 | Maximum bytes to read |
| `encoding` | string | `"utf8"` | Output encoding: `"utf8"`, `"hex"`, `"base64"` |

Read data is encoded into the specified format before returning. A timeout returns a success result with zero bytes read rather than an error.

## Architecture

```
MCP Client (Claude, etc.)
    │  JSON-RPC over stdio
    ▼
┌─────────────────────────┐
│  SerialHandler          │  rmcp macros: #[tool], #[tool_router]
│  (tools/serial_handler) │  Registers 5 MCP tools
└───────────┬─────────────┘
            │
┌───────────▼─────────────┐
│  ConnectionManager      │  Manages concurrent connections
│  (serial/mod)           │  via Arc<RwLock<HashMap>>
└───────────┬─────────────┘
            │
┌───────────▼─────────────┐
│  SerialConnection       │  Async read/write with timeouts
│  (serial/connection)    │  Wraps tokio-serial SerialStream
└───────────┬─────────────┘
            │
┌───────────▼─────────────┐
│  tokio-serial           │  Async serial I/O via Tokio
│  serialport             │  Port enumeration and sync fallback
└─────────────────────────┘
```

### Dependencies

- **rmcp** — MCP protocol implementation, provides the `#[tool]` / `#[tool_router]` / `#[tool_handler]` macro system
- **tokio-serial** — Async serial port I/O backed by Tokio
- **serialport** — Cross-platform serial port enumeration
- **clap** — CLI argument parsing with derive macros

### Module layout

| Module | Purpose |
|--------|---------|
| `tools/serial_handler` | MCP tool definitions and ServerHandler implementation |
| `tools/types` | Request/response structs and encode/decode helpers |
| `serial/connection` | `SerialConnection` — async serial I/O with configurable timeouts |
| `serial/port` | `PortInfo` — system port enumeration |
| `serial/mod` | `ConnectionManager` — concurrent connection lifecycle management |
| `session` | Session management with idle cleanup and statistics tracking |
| `config` | CLI args, TOML config, validation, and merging |
| `error` | Unified error hierarchy via `thiserror` |
| `utils` | Data encoding, checksums, buffer utilities, validators |

## Installation

### Prerequisites

- Rust toolchain 1.70+
- Serial device or USB-to-serial adapter with appropriate drivers

### Build from source

```bash
git clone https://github.com/adancurusul/serial-mcp-server.git
cd serial-mcp-server
cargo build --release
```

The binary will be at `target/release/serial-mcp-server` (`.exe` on Windows).

## Configuration

### MCP Client Setup

Add the server to your MCP client's configuration file. The server communicates over stdio.

**Claude Desktop** (`claude_desktop_config.json`):

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

**Claude Code** (`.claude/settings.json` or project settings):

```json
{
  "mcpServers": {
    "serial": {
      "command": "target/release/serial-mcp-server"
    }
  }
}
```

### CLI Reference

```
serial-mcp-server [OPTIONS]

Options:
  -c, --config <PATH>           Path to TOML configuration file
      --log-level <LEVEL>       Log level: error, warn, info, debug, trace [default: info]
      --log-file <PATH>         Write logs to file instead of stderr
      --max-connections <N>     Maximum concurrent connections [default: 10]
      --connection-timeout <S>  Connection timeout in seconds [default: 30]
      --default-baud-rate <R>   Default baud rate [default: 115200]
      --default-timeout-ms <MS> Default operation timeout in ms [default: 1000]
      --max-buffer-size <N>     Maximum read buffer size in bytes [default: 8192]
      --retry-count <N>         Connection retry count [default: 3]
      --auto-discovery           Enable automatic port discovery
      --allow-port-sharing       Allow multiple connections to the same port
      --restrict-ports           Restrict port access to allowed list
      --generate-config          Print default TOML configuration and exit
      --validate-config          Validate configuration and exit
      --show-config              Print current configuration and exit
```

### Configuration File (TOML)

Generate a default configuration file:

```bash
serial-mcp-server --generate-config > config.toml
```

CLI arguments override corresponding config file values at runtime.

## STM32 Demo

The repository includes an example STM32 firmware (`examples/STM32_demo/`) that implements an interactive command interface over USART1 at 115200 baud 8N1:

| Command | Function |
|---------|----------|
| `H` | Display help menu |
| `L` | Toggle LED (PB7) |
| `C` | Show and increment counter |
| `R` | Reset counter |
| `B` | Blink LED 3 times |
| Any other | Echo character |

Build and flash the firmware:

```bash
cd examples/STM32_demo
cargo run --release
```

## Development

```bash
cargo build                          # Debug build
cargo test --lib                     # Unit tests only (no hardware)
cargo test -- --nocapture            # All tests including hardware-dependent
cargo test <test_name> -- --nocapture  # Single test
```

### Important: Future import

The `rmcp` 0.3.2 `#[tool]` macro generates code referencing `Future`. Each file using `#[tool]` must import it:

```rust
use std::future::Future;
```

The project requires Rust 1.95+ due to macro exhaustiveness checking changes.

## Platform Support

| Platform | Port naming | Examples |
|----------|-------------|----------|
| Windows | `COMx` | COM1, COM13 |
| Linux | `/dev/ttyXXX` | /dev/ttyUSB0, /dev/ttyACM0 |
| macOS | `/dev/tty.*` | /dev/tty.usbserial-1234 |

## License

MIT. See [LICENSE](LICENSE).

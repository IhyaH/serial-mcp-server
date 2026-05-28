# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 构建与测试命令

```bash
# 构建
cargo build                        # debug 构建
cargo build --release              # release 构建

# 运行所有测试（含串口硬件测试，需 COM13 设备）
cargo test -- --nocapture

# 仅运行单元测试（不依赖硬件）
cargo test --lib -- --nocapture

# 运行特定测试
cargo test test_list_ports -- --nocapture
cargo test test_mcp_full_integration -- --nocapture

# 直接测试 COM13 串口通信（同步 serialport）
cargo test test_com_ports_direct -- --nocapture

# 直接测试 COM13 串口通信（异步 tokio-serial）
cargo test test_com13_async_communication -- --nocapture
```

## 架构概览

本项目是一个 **MCP (Model Context Protocol) 服务器**，通过 stdio 传输层（JSON-RPC）对外暴露串口通信工具。AI 客户端通过标准输入/输出与其交互。

### 两层架构

```
MCP 协议层 (tools/serial_handler.rs)
    ↓ 使用 rmcp 的 #[tool] 宏注册工具
串口通信层 (serial/connection.rs)
    ↓ 基于 tokio-serial + serialport
硬件串口设备
```

1. **`src/tools/serial_handler.rs`** — MCP 工具定义。使用 `rmcp` 的 `#[tool]` + `#[tool_router]` + `#[tool_handler]` 宏模式注册 5 个工具：`list_ports`、`open`、`close`、`write`、`read`。**需要 `use std::future::Future;`**，否则 rmcp 宏展开后找不到 Future trait（Rust 1.95 编译失败）。
2. **`src/serial/connection.rs`** — 核心串口抽象。`SerialConnection` 封装 tokio-serial 的异步读写，`ConnectionManager` 管理多连接生命周期。
3. **`src/serial/port.rs`** — 端口发现，使用 serialport crate 枚举系统串口。
4. **`src/config.rs`** — CLI 参数（clap derive）+ TOML 配置文件 + Config 合并与验证。
5. **`src/error.rs`** — 统一错误类型 `SerialError`（thiserror），涵盖连接、会话、编码、配置等子错误。
6. **`src/session/`** — 会话管理层，提供 `SessionManager` + `SerialSession`，支持连接生命周期、空闲清理、重连。
7. **`src/utils.rs`** — 数据转换（`DataConverter`）、校验（`Validator`）、缓冲操作、校验和等工具函数。
8. **`src/main.rs`** — 入口。初始化日志 → 加载配置 → 创建 `SerialHandler` → `serve(stdio())` 启动 MCP 服务。

### rmcp 宏模式

`SerialHandler` 使用 rmcp 0.3.2 的标准模式：

- `#[tool_router]` 在 `impl SerialHandler` 上生成 `ToolRouter`
- `#[tool(description = "...")]` 标记每个工具方法，方法签名需包含 `&self` 和 `Parameters(args): Parameters<T>`（无参数工具可省略 Parameters）
- `#[tool_handler]` 在 `impl ServerHandler for SerialHandler` 上实现 MCP 协议处理

### 数据流

```
AI 客户端 → JSON-RPC(stdin) → rmcp 路由 → SerialHandler 工具方法
    → ConnectionManager → SerialConnection → tokio-serial → 硬件串口
    → 响应沿反向路径返回
```

## 关键约束

- **波特率白名单**：`Validator::validate_baud_rate` 只接受预定义列表（300-921600），自定义波特率会失败。
- **端口不能重复打开**：`ConnectionManager::open` 检查端口名是否已在使用（大小写不敏感）。
- **运行时不可重配置**：`SerialConnection::reconfigure` 返回错误，需先关闭再重新打开。
- **DTR 可能复位 STM32**：STLink VCP 的 DTR 常连接 STM32 NRST 引脚，切换 DTR 会复位 MCU。连接 STM32 设备后建议等待 500ms 再通信。

## COM13 测试设备

当前环境 COM13 连接 STM32（STLink VCP, USB VID:0483 PID:374B），115200 baud 8N1。设备行为：
- 每收到 `\r\n` 返回时间戳：`Date: 2026-01-01  Time: HH:MM:SS\r\n`
- 单字符直接回显
- 发送文件内容后回显并继续发时间戳

测试文件 `tests/com13_direct.rs` 使用同步 serialport，`tests/com13_test.rs` 使用异步 tokio-serial，`tests/mcp_integration_test.rs` 启动完整服务器进程进行 MCP 协议测试。

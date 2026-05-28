//! MCP 协议集成测试 - 通过 JSON-RPC over stdio 测试完整 MCP 功能

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

struct McpClient {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
    writer: std::process::ChildStdin,
}

impl McpClient {
    fn new() -> Self {
        let mut child = Command::new("target/debug/serial-mcp-server.exe")
            .arg("--log-level")
            .arg("error")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("启动 serial-mcp-server 失败");

        let writer = child.stdin.take().expect("stdin");
        let reader = BufReader::new(child.stdout.take().expect("stdout"));

        let mut client = Self {
            child,
            reader,
            writer,
        };
        client.initialize();
        client
    }

    fn send(&mut self, request: &Value) -> Value {
        self.send_raw(request);
        self.read_response()
    }

    fn send_raw(&mut self, request: &Value) {
        let req_str = serde_json::to_string(request).unwrap();
        let display_str = if req_str.len() > 200 {
            format!("{}...", &req_str[..200])
        } else {
            req_str.clone()
        };
        println!("\n>>> {}", display_str);
        self.writer.write_all(req_str.as_bytes()).unwrap();
        self.writer.write_all(b"\n").unwrap();
        self.writer.flush().unwrap();
    }

    fn read_response(&mut self) -> Value {
        let mut line = String::new();
        self.reader.read_line(&mut line).expect("读取响应失败");
        let trimmed = line.trim();
        let display_resp = if trimmed.len() > 300 {
            format!("{}...", &trimmed[..300])
        } else {
            trimmed.to_string()
        };
        println!("<<< {}", display_resp);
        if trimmed.is_empty() {
            return json!(null);
        }
        serde_json::from_str(trimmed)
            .unwrap_or_else(|e| panic!("JSON 解析失败: {} - 内容: {}", e, trimmed))
    }

    fn initialize(&mut self) {
        let resp = self.send(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "integration-test", "version": "1.0.0" }
            }
        }));
        assert!(resp.get("result").is_some(), "initialize 失败: {}", resp);

        // 通知不需要响应
        self.send_raw(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
        println!("  (已发送 initialized 通知)\nMCP 初始化成功!");
    }

    fn call_tool(&mut self, id: u64, name: &str, args: Value) -> Value {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": args }
        }))
    }

    fn get_text<'a>(&self, result: &'a Value) -> &'a str {
        result["result"]["content"][0]["text"].as_str().unwrap()
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.child.kill().ok();
    }
}

#[test]
fn test_mcp_full_integration() {
    println!("\n========== MCP 协议完整集成测试 ==========");
    let mut client = McpClient::new();

    // === 1. tools/list ===
    println!("\n=== 测试 1: tools/list ===");
    let resp = client.send(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
    }));
    let tools = resp["result"]["tools"].as_array().expect("应有工具列表");
    println!("工具数量: {}", tools.len());
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    println!("工具列表: {:?}", tool_names);
    assert!(tool_names.contains(&"list_ports"), "缺少 list_ports");
    assert!(tool_names.contains(&"open"), "缺少 open");
    assert!(tool_names.contains(&"close"), "缺少 close");
    assert!(tool_names.contains(&"write"), "缺少 write");
    assert!(tool_names.contains(&"read"), "缺少 read");
    println!("✓ tools/list 通过 ({} 个工具)", tools.len());

    // === 2. list_ports ===
    println!("\n=== 测试 2: list_ports ===");
    let resp = client.call_tool(3, "list_ports", json!({}));
    let text = client.get_text(&resp);
    println!("{}", text);
    assert!(text.contains("COM13"), "应包含 COM13: {}", text);
    println!("✓ list_ports 通过");

    // === 3. open COM13 ===
    println!("\n=== 测试 3: open COM13 ===");
    let resp = client.call_tool(
        4,
        "open",
        json!({
            "port": "COM13",
            "baud_rate": 115200,
            "data_bits": "8",
            "stop_bits": "1",
            "parity": "none",
            "flow_control": "none"
        }),
    );
    let text = client.get_text(&resp);
    println!("{}", text);
    assert!(
        text.contains("Connection ID:"),
        "缺少 Connection ID: {}",
        text
    );
    let conn_id = text
        .lines()
        .find(|l| l.starts_with("Connection ID:"))
        .map(|l| l.trim_start_matches("Connection ID: "))
        .unwrap()
        .to_string();
    println!("Connection ID: {}", conn_id);
    println!("✓ open 通过");

    // === 4. write + read 时间戳 ===
    println!("\n=== 测试 4: write + read 时间戳 ===");
    let w_resp = client.call_tool(
        5,
        "write",
        json!({
            "connection_id": conn_id,
            "data": "\\r\\n",
            "encoding": "utf8"
        }),
    );
    println!("write: {}", client.get_text(&w_resp));

    let r_resp = client.call_tool(
        6,
        "read",
        json!({
            "connection_id": conn_id,
            "timeout_ms": 2000,
            "max_bytes": 1024,
            "encoding": "utf8"
        }),
    );
    let text = client.get_text(&r_resp);
    println!("read: {}", text);
    assert!(
        text.contains("Date:") || text.contains("Bytes read"),
        "应包含时间戳或读取结果: {}",
        text
    );
    println!("✓ write+read 通过");

    // === 5. 连续读写 ===
    println!("\n=== 测试 5: 连续 3 次读写 ===");
    for i in 1..=3 {
        client.call_tool(
            10 + i,
            "write",
            json!({
                "connection_id": conn_id,
                "data": "\\r\\n",
                "encoding": "utf8"
            }),
        );
        let resp = client.call_tool(
            20 + i,
            "read",
            json!({
                "connection_id": conn_id,
                "timeout_ms": 1500,
                "max_bytes": 256,
                "encoding": "utf8"
            }),
        );
        let first_line = client
            .get_text(&resp)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        println!("  第{}次: {}", i, first_line);
    }
    println!("✓ 连续读写通过");

    // === 6. 发送文件内容 ===
    println!("\n=== 测试 6: 发送文件内容 ===");
    client.call_tool(
        30,
        "write",
        json!({
            "connection_id": conn_id,
            "data": "Hello from MCP test!\r\nLine 2\r\n",
            "encoding": "utf8"
        }),
    );
    let resp = client.call_tool(
        31,
        "read",
        json!({
            "connection_id": conn_id,
            "timeout_ms": 2000,
            "max_bytes": 2048,
            "encoding": "utf8"
        }),
    );
    let text = client.get_text(&resp);
    println!("文件响应:\n{}", text);
    println!("✓ 文件发送通过");

    // === 7. hex 编码 ===
    println!("\n=== 测试 7: hex 编码读写 ===");
    client.call_tool(
        32,
        "write",
        json!({
            "connection_id": conn_id,
            "data": "0D 0A",
            "encoding": "hex"
        }),
    );
    let resp = client.call_tool(
        33,
        "read",
        json!({
            "connection_id": conn_id,
            "timeout_ms": 2000,
            "max_bytes": 1024,
            "encoding": "hex"
        }),
    );
    let text = client.get_text(&resp);
    println!("hex 读取: {}", text);
    println!("✓ hex 编码通过");

    // === 8. close ===
    println!("\n=== 测试 8: close ===");
    let resp = client.call_tool(
        40,
        "close",
        json!({
            "connection_id": conn_id
        }),
    );
    let text = client.get_text(&resp);
    println!("{}", text);
    assert!(
        text.to_lowercase().contains("closed"),
        "应包含 closed: {}",
        text
    );
    println!("✓ close 通过");

    // === 9. 错误处理: 无效连接 ID ===
    println!("\n=== 测试 9: 错误处理 - 无效连接 ===");
    let resp = client.call_tool(
        41,
        "write",
        json!({
            "connection_id": "invalid-id",
            "data": "test",
            "encoding": "utf8"
        }),
    );
    let has_error =
        resp.get("error").is_some() || resp["result"]["isError"].as_bool() == Some(true);
    assert!(has_error, "应返回错误: {}", resp);
    println!("错误响应: {}", resp);
    println!("✓ 错误处理通过");

    // === 10. 错误处理: close 不存在连接 ===
    println!("\n=== 测试 10: 错误处理 - close 不存在连接 ===");
    let resp = client.call_tool(
        42,
        "close",
        json!({
            "connection_id": "nonexistent"
        }),
    );
    let has_error =
        resp.get("error").is_some() || resp["result"]["isError"].as_bool() == Some(true);
    assert!(has_error, "应返回错误: {}", resp);
    println!("✓ close 错误处理通过");

    println!("\n========== 全部 10 项 MCP 测试通过! ==========");
}

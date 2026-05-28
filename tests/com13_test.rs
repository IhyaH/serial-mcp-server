//! COM13 设备异步测试 (tokio-serial)
//! 验证 MCP 服务器的 SerialConnection 能正常与 COM13 通信

use serial_mcp_server::serial::{ConnectionConfig, DataBits, StopBits, Parity, FlowControl};
use tokio::time::{timeout, Duration};

fn config() -> ConnectionConfig {
    ConnectionConfig {
        port: "COM13".to_string(),
        baud_rate: 115200,
        data_bits: DataBits::Eight,
        stop_bits: StopBits::One,
        parity: Parity::None,
        flow_control: FlowControl::None,
    }
}

/// 测试异步连接和收发
#[tokio::test]
async fn test_com13_async_communication() {
    println!("\n=== COM13 异步通信测试 (tokio-serial) ===");

    let conn = serial_mcp_server::serial::SerialConnection::new(config())
        .await
        .expect("连接 COM13 失败");
    println!("已连接! id: {}", conn.id());

    let mut buf = vec![0u8; 8192];

    // 测试 1: 发送 \r\n 获取时间戳
    println!("\n--- 测试读取时间戳 ---");
    conn.write(b"\r\n").await.expect("发送失败");

    match timeout(Duration::from_secs(2), conn.read(&mut buf, Some(2000))).await {
        Ok(Ok(n)) => {
            let text = String::from_utf8_lossy(&buf[..n]);
            println!("收到 ({} 字节): {:?}", n, text);
        }
        Ok(Err(e)) => println!("错误: {:?}", e),
        Err(_) => println!("超时"),
    }

    // 测试 2: 连续获取多个时间戳
    println!("\n--- 连续获取时间戳 ---");
    for i in 1..=3 {
        conn.write(b"\r\n").await.expect("发送失败");
        tokio::time::sleep(Duration::from_millis(100)).await;

        match timeout(Duration::from_secs(1), conn.read(&mut buf, Some(1000))).await {
            Ok(Ok(n)) => {
                let text = String::from_utf8_lossy(&buf[..n]);
                println!("  第{}次: {:?}", i, text.trim());
            }
            _ => println!("  第{}次: 无回复", i),
        }
    }

    // 测试 3: 发送文件内容
    println!("\n--- 发送文件内容 ---");
    let file_content = b"Test file content line 1\r\nTest file content line 2\r\n";
    conn.write(file_content).await.expect("发送失败");

    let mut total = String::new();
    for _ in 0..5 {
        match timeout(Duration::from_millis(500), conn.read(&mut buf, Some(500))).await {
            Ok(Ok(n)) => {
                total.push_str(&String::from_utf8_lossy(&buf[..n]));
            }
            _ => break,
        }
    }
    println!("收到 ({} 字节): {:?}", total.len(), total);

    // 测试 4: 回显测试
    println!("\n--- 单字符回显 ---");
    for &ch in b"XYZ" {
        conn.write(&[ch]).await.expect("发送失败");
        tokio::time::sleep(Duration::from_millis(50)).await;

        match timeout(Duration::from_millis(500), conn.read(&mut buf, Some(500))).await {
            Ok(Ok(n)) => {
                println!("  发送 '{}' -> 收到 {:?}", ch as char, String::from_utf8_lossy(&buf[..n]));
            }
            _ => println!("  发送 '{}' -> 无回复", ch as char),
        }
    }

    let st = conn.status().await;
    println!("\n统计: 发送={} 字节, 接收={} 字节", st.bytes_sent, st.bytes_received);
    println!("=== 异步测试完成 ===");
}

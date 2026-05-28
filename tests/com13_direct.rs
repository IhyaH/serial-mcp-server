//! COM13 设备完整测试 - 已确认 115200 baud 可通信

use std::io::{Read, Write};
use std::time::Duration;

#[test]
fn test_com13_full() {
    println!("=== COM13 @ 115200 baud 全面测试 ===\n");

    let mut port = serialport::new("COM13", 115200)
        .data_bits(serialport::DataBits::Eight)
        .stop_bits(serialport::StopBits::One)
        .parity(serialport::Parity::None)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(5000))
        .open()
        .expect("打开 COM13 失败");

    // DTR 复位
    port.write_data_terminal_ready(false).ok();
    std::thread::sleep(Duration::from_millis(50));
    port.write_data_terminal_ready(true).ok();
    std::thread::sleep(Duration::from_millis(500));
    port.clear(serialport::ClearBuffer::All).ok();
    println!("串口已打开并复位\n");

    let mut buf = [0u8; 8192];

    // 测试 1: 发送 \r\n 获取时间戳
    println!("=== 测试 1: 发送 \\r\\n 获取时间戳 ===");
    port.clear(serialport::ClearBuffer::All).ok();
    port.write_all(b"\r\n").unwrap();
    println!("已发送: \\r\\n");

    std::thread::sleep(Duration::from_millis(500));
    port.set_timeout(Duration::from_millis(2000)).ok();

    let mut response = String::new();
    for _ in 0..10 {
        match port.read(&mut buf) {
            Ok(n) => {
                response.push_str(&String::from_utf8_lossy(&buf[..n]));
            }
            _ => break,
        }
    }
    println!("收到 ({} 字节): {:?}\n", response.len(), response);

    // 测试 2: 多次获取时间戳
    println!("=== 测试 2: 连续获取多个时间戳 ===");
    for i in 1..=3 {
        port.clear(serialport::ClearBuffer::All).ok();
        port.write_all(b"\r\n").unwrap();
        std::thread::sleep(Duration::from_millis(200));
        port.set_timeout(Duration::from_millis(1000)).ok();

        let mut resp = String::new();
        for _ in 0..5 {
            match port.read(&mut buf) {
                Ok(n) => resp.push_str(&String::from_utf8_lossy(&buf[..n])),
                _ => break,
            }
        }
        println!("  第{}次: {:?}", i, resp);
    }
    println!();

    // 测试 3: 发送文件内容
    println!("=== 测试 3: 模拟发送文件 ===");
    let test_file_content = b"Hello from serial-mcp-server!\r\nThis is a test file.\r\n";
    port.clear(serialport::ClearBuffer::All).ok();
    port.write_all(test_file_content).unwrap();
    println!("已发送 {} 字节文件内容", test_file_content.len());

    std::thread::sleep(Duration::from_millis(500));
    port.set_timeout(Duration::from_millis(3000)).ok();

    let mut file_resp = String::new();
    for _ in 0..15 {
        match port.read(&mut buf) {
            Ok(n) => file_resp.push_str(&String::from_utf8_lossy(&buf[..n])),
            _ => break,
        }
    }
    println!("收到回复 ({} 字节): {:?}\n", file_resp.len(), file_resp);

    // 测试 4: 发送单字符测试
    println!("=== 测试 4: 单字符测试 ===");
    for &ch in b"abcABC123" {
        port.clear(serialport::ClearBuffer::All).ok();
        port.write_all(&[ch]).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        port.set_timeout(Duration::from_millis(500)).ok();

        match port.read(&mut buf) {
            Ok(n) => {
                let r = String::from_utf8_lossy(&buf[..n]);
                println!("  发送 '{}' -> 收到 ({}字节): {:?}", ch as char, n, r);
            }
            _ => println!("  发送 '{}' -> 无回复", ch as char),
        }
    }
    println!();

    // 测试 5: 尝试各种波特率获取时间戳
    println!("=== 测试 5: 其他波特率 ===");
    drop(port);
    for &baud in &[9600, 38400, 57600] {
        std::thread::sleep(Duration::from_millis(100));

        let mut p = match serialport::new("COM13", baud)
            .timeout(Duration::from_millis(2000))
            .open()
        {
            Ok(p) => p,
            Err(e) => {
                println!("  {} baud: 打开失败 ({})", baud, e);
                continue;
            }
        };

        p.clear(serialport::ClearBuffer::All).ok();
        p.write_all(b"\r\n").unwrap();
        std::thread::sleep(Duration::from_millis(300));
        p.set_timeout(Duration::from_millis(1000)).ok();

        let mut resp = String::new();
        for _ in 0..5 {
            match p.read(&mut buf) {
                Ok(n) => resp.push_str(&String::from_utf8_lossy(&buf[..n])),
                _ => break,
            }
        }
        if resp.is_empty() {
            println!("  {} baud: 无回复", baud);
        } else {
            println!("  {} baud: {:?}", baud, resp);
        }
        drop(p);
    }

    println!("\n=== 测试完成 ===");
}

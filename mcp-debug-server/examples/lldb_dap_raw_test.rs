//! Raw lldb-dap communication test
//!
//! This example tests lldb-dap at the lowest level to see what it outputs.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use std::process::Stdio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Raw lldb-dap communication test\n");

    let adapter_path = r"C:\Program Files\LLVM\bin\lldb-dap.exe";
    println!("📍 Using adapter: {}\n", adapter_path);

    // Запустить lldb-dap с правильной рабочей директорией
    println!("🚀 Spawning lldb-dap...");

    let adapter_dir = std::path::Path::new(adapter_path)
        .parent()
        .expect("Failed to get adapter directory");

    println!("📂 Working directory: {:?}\n", adapter_dir);

    let mut child = Command::new(adapter_path)
        .current_dir(adapter_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);

    println!("✅ lldb-dap spawned\n");

    // Читать stderr в фоне и собирать все сообщения
    let stderr_handle = tokio::spawn(async move {
        let mut line = String::new();
        let mut messages = Vec::new();
        loop {
            line.clear();
            match stderr_reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let msg = line.trim().to_string();
                    if !msg.is_empty() {
                        println!("STDERR: {}", msg);
                        messages.push(msg);
                    }
                }
                Err(e) => {
                    eprintln!("Error reading stderr: {}", e);
                    break;
                }
            }
        }
        messages
    });

    // Отправить initialize request
    let init_request = serde_json::json!({
        "command": "initialize",
        "type": "request",
        "seq": 1,
        "arguments": {
            "clientID": "test",
            "adapterID": "lldb",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "pathFormat": "path"
        }
    });

    let body = serde_json::to_string(&init_request)?;
    let message = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    println!("📤 Sending initialize request:");
    println!("{}\n", message);

    stdin.write_all(message.as_bytes()).await?;
    stdin.flush().await?;

    println!("✅ Request sent\n");

    // Читать ответ построчно с timeout
    println!("📥 Reading response (first 10 lines):\n");

    for i in 0..10 {
        let mut line = String::new();

        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            stdout_reader.read_line(&mut line)
        ).await {
            Ok(Ok(0)) => {
                println!("EOF reached");
                break;
            }
            Ok(Ok(_)) => {
                println!("Line {}: {:?}", i + 1, line.trim());
            }
            Ok(Err(e)) => {
                println!("Error reading line: {}", e);
                break;
            }
            Err(_) => {
                println!("Timeout waiting for line {}", i + 1);
                break;
            }
        }
    }

    println!("\n🔚 Test complete");

    // Дождаться завершения процесса
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    match child.try_wait()? {
        Some(status) => println!("\n📊 Process exit status: {:?}", status),
        None => {
            println!("\n⚠️ Process still running, killing...");
            child.kill().await?;
        }
    }

    // Собрать stderr messages
    let stderr_messages = stderr_handle.await?;
    if !stderr_messages.is_empty() {
        println!("\n📋 All stderr messages ({} total):", stderr_messages.len());
        for msg in stderr_messages {
            println!("  - {}", msg);
        }
    } else {
        println!("\n✅ No stderr output");
    }

    Ok(())
}

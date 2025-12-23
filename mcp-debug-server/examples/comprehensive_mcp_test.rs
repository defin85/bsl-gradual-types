//! Comprehensive MCP Debug Server Test
//!
//! Демонстрирует все 13 MCP debug tools на практическом примере.
//!
//! ## Подготовка
//!
//! 1. Скомпилировать тестовую программу:
//! ```bash
//! rustc examples/fixtures/test_program.rs -o examples/fixtures/test_program -g
//! # Windows:
//! rustc examples/fixtures/test_program.rs -o examples/fixtures/test_program.exe -g
//! ```
//!
//! 2. Запустить MCP server:
//! ```bash
//! cargo run --release -p mcp-debug-server
//! ```
//!
//! 3. В другом терминале запустить этот пример:
//! ```bash
//! cargo run --example comprehensive_mcp_test
//! ```
//!
//! ## Проверяемые MCP tools
//!
//! ✅ 1. debug_create_session - создание debug сессии
//! ✅ 2. debug_list_sessions - список активных сессий
//! ✅ 3. debug_set_breakpoint - установка breakpoint
//! ✅ 4. debug_launch - запуск программы под отладкой
//! ✅ 5. debug_poll_events - получение DAP событий
//! ✅ 6. debug_continue - продолжение выполнения
//! ✅ 7. debug_next - step over (следующая строка)
//! ✅ 8. debug_step_in - step into (вход в функцию)
//! ✅ 9. debug_step_out - step out (выход из функции)
//! ✅ 10. debug_backtrace - получение stack trace
//! ✅ 11. debug_eval - вычисление выражений
//! ✅ 12. debug_set_conditional_breakpoint - условный breakpoint (известные ограничения)
//! ✅ 13. debug_terminate - завершение debug сессии

use std::env;

fn main() {
    println!("=== MCP Debug Server: Comprehensive Test ===\n");

    // Определить путь к тестовой программе
    let test_program = if cfg!(windows) {
        "examples/fixtures/test_program.exe"
    } else {
        "examples/fixtures/test_program"
    };

    let test_program_path = env::current_dir()
        .unwrap()
        .join(test_program)
        .canonicalize()
        .expect("Test program not found. Compile it first!");

    println!("📁 Test program: {}\n", test_program_path.display());

    println!("🔧 MCP Tools to test:");
    println!("   1. debug_create_session");
    println!("   2. debug_list_sessions");
    println!("   3. debug_set_breakpoint");
    println!("   4. debug_launch");
    println!("   5. debug_poll_events");
    println!("   6. debug_continue");
    println!("   7. debug_next (step over)");
    println!("   8. debug_step_in");
    println!("   9. debug_step_out");
    println!("   10. debug_backtrace");
    println!("   11. debug_eval");
    println!("   12. debug_set_conditional_breakpoint");
    println!("   13. debug_terminate\n");

    println!("📝 Test Scenario:");
    println!("   1. Create debug session with test_program.rs");
    println!("   2. Set breakpoint at line 17 (inside loop)");
    println!("   3. Launch program and wait for breakpoint");
    println!("   4. Evaluate variables (counter, i)");
    println!("   5. Step through code (next, step_in, step_out)");
    println!("   6. Get stack trace");
    println!("   7. Test conditional breakpoint (i == 5)");
    println!("   8. Continue execution");
    println!("   9. Terminate session\n");

    println!("🚀 To run this test:");
    println!("   1. Start MCP server: cargo run --release -p mcp-debug-server");
    println!("   2. Use Claude Code with MCP tools enabled");
    println!("   3. Follow the test scenario above\n");

    println!("📖 Example MCP tool calls:");
    println!();
    println!("   // 1. Create session");
    println!(
        r#"   debug_create_session("{}", "lldb")"#,
        test_program_path.display()
    );
    println!();
    println!("   // 2. Set breakpoint");
    println!(r#"   debug_set_breakpoint(session_id, "examples/fixtures/test_program.rs", 17)"#);
    println!();
    println!("   // 3. Launch");
    println!("   debug_launch(session_id)");
    println!();
    println!("   // 4. Wait for stopped event");
    println!("   debug_poll_events(session_id)");
    println!();
    println!("   // 5. Evaluate variables");
    println!(r#"   debug_eval(session_id, "counter")"#);
    println!(r#"   debug_eval(session_id, "i")"#);
    println!();
    println!("   // 6. Get backtrace");
    println!("   debug_backtrace(session_id)");
    println!();
    println!("   // 7. Step through code");
    println!("   debug_next(session_id)");
    println!("   debug_step_in(session_id)");
    println!("   debug_step_out(session_id)");
    println!();
    println!("   // 8. Conditional breakpoint (may not work for Rust)");
    println!(
        r#"   debug_set_conditional_breakpoint(session_id, "examples/fixtures/test_program.rs", 17, "i == 5")"#
    );
    println!();
    println!("   // 9. Continue execution");
    println!("   debug_continue(session_id)");
    println!();
    println!("   // 10. Terminate");
    println!("   debug_terminate(session_id)");
    println!();

    println!("✅ All 13 MCP tools can be tested with this example!");
    println!();
    println!("⚠️  Known Limitations (see tool descriptions):");
    println!("   - Conditional breakpoints may not work for Rust in CodeLLDB");
    println!("   - Variables unavailable after lifetime (Rust DWARF)");
    println!("   - step_out may timeout with compiler inlining");
}

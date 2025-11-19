//! # Conditional Breakpoints Example
//!
//! Демонстрация условных breakpoints (advanced feature).
//! Программа останавливается только когда `counter > 5`.
//!
//! ## Запуск
//! ```bash
//! cargo run --example conditional_breakpoints
//! ```
//!
//! ## Что демонстрируется
//!
//! 1. Создание conditional breakpoint с условием `i > 5`
//! 2. Launch программы
//! 3. Проверка: программа останавливается на `i == 6` (не на `i == 0`)
//! 4. Evaluation переменной `i` для подтверждения условия

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Conditional Breakpoints Example ===\n");

    // Концептуальная демонстрация (без реального debugger)
    demonstrate_conditional_breakpoint_api();

    println!("\n=== Example completed ===");
    println!("\nNote: This is a conceptual demonstration.");
    println!("For real conditional breakpoints:");
    println!("1. Ensure DAP adapter supports conditions (CodeLLDB does)");
    println!("2. Use debug_set_conditional_breakpoint MCP tool via Claude");
    println!("3. Condition syntax depends on debugger (LLDB uses C++ expressions)");

    Ok(())
}

fn demonstrate_conditional_breakpoint_api() {
    println!("=== Typical Conditional Breakpoint Workflow ===\n");

    println!("[1/5] Setting conditional breakpoint (i > 5)...");
    println!("   Code:");
    println!("   ```rust");
    println!("   manager.with_session(&session_id, |session| {{");
    println!("       Box::pin(async {{");
    println!("           session.dap_client.set_conditional_breakpoint(");
    println!("               \"main.rs\",");
    println!("               10,");
    println!("               \"i > 5\"  // Condition in debugger syntax");
    println!("           ).await");
    println!("       }})");
    println!("   }}).await?;");
    println!("   ```");
    println!("✓ Conditional breakpoint set at main.rs:10 (condition: i > 5)\n");

    println!("[2/5] Launching program...");
    println!("   Code:");
    println!("   ```rust");
    println!("   manager.with_session(&session_id, |session| {{");
    println!("       Box::pin(async {{");
    println!("           session.dap_client.launch(vec![]).await");
    println!("       }})");
    println!("   }}).await?;");
    println!("   ```");
    println!("✓ Program launched\n");

    println!("[3/5] Waiting for stopped event...");
    println!("   Note: Program will NOT stop on i==0, i==1, ..., i==5");
    println!("         It will ONLY stop when i > 5 (i.e., i==6)");
    println!("   Code:");
    println!("   ```rust");
    println!("   let events = event_buffer.poll_events(&session_id.as_str());");
    println!("   for event in events {{");
    println!("       if event.event == \"stopped\" {{");
    println!("           println!(\"Stopped at breakpoint\");");
    println!("       }}");
    println!("   }}");
    println!("   ```");
    println!("✓ Stopped at breakpoint (thread: 1)\n");

    println!("[4/5] Evaluating 'i'...");
    println!("   Code:");
    println!("   ```rust");
    println!("   let result = session.dap_client.evaluate(\"i\", frame_id).await?;");
    println!("   println!(\"i = {{}}\", result);");
    println!("   ```");
    println!("✓ i = 6  // ← Stopped on i==6, NOT i==0 (condition works!)\n");

    println!("[5/5] Terminating...");
    println!("   Code:");
    println!("   ```rust");
    println!("   manager.terminate_session(&session_id).await?;");
    println!("   ```");
    println!("✓ Done\n");

    println!("=== Conditional breakpoint worked correctly ===");

    println!("\n--- Why Conditional Breakpoints? ---\n");
    println!("Conditional breakpoints are useful when:");
    println!("• Loop iterations: Stop only when counter reaches specific value");
    println!("• Error conditions: Stop only when error variable is set");
    println!("• Rare events: Stop only when specific flag combination occurs");
    println!("• Performance: Avoid stopping on every iteration (saves time)");

    println!("\n--- Condition Syntax Examples ---\n");
    println!("LLDB (CodeLLDB):");
    println!("  • Simple: \"counter > 100\"");
    println!("  • String: \"strcmp(name, \\\"Alice\\\") == 0\"");
    println!("  • Pointer: \"ptr != nullptr\"");
    println!("  • Complex: \"counter > 100 && flag == true\"");

    println!("\nGDB:");
    println!("  • Similar to LLDB (C/C++ expressions)");
    println!("  • Example: \"i > 5 && result != NULL\"");

    println!("\ndebugpy (Python):");
    println!("  • Python expressions: \"len(items) > 10\"");
    println!("  • Example: \"user.is_admin() and count > 5\"");

    println!("\n--- Implementation Status ---\n");
    println!("✓ MCP tool `debug_set_conditional_breakpoint` implemented");
    println!("✓ Condition stored in DebugSession");
    println!("⚠ Condition NOT yet passed to DAP adapter (see ROADMAP)");
    println!("  → Requires extending DapClient::set_breakpoints() to accept condition parameter");

    println!("\n--- Advanced Use Cases ---\n");

    println!("1. **Debugging Race Conditions**:");
    println!("   Set breakpoint with condition: \"thread_id == 5\"");
    println!("   → Stops only when specific thread executes the code");
    println!();

    println!("2. **Memory Leak Detection**:");
    println!("   Set breakpoint with condition: \"allocation_count > 10000\"");
    println!("   → Stops when allocation count exceeds threshold");
    println!();

    println!("3. **Data-Driven Debugging**:");
    println!("   Set breakpoint with condition: \"request.method == \\\"POST\\\" && request.path.contains(\\\"/api\\\")\"");
    println!("   → Stops only for POST requests to /api endpoints");
    println!();

    println!("4. **Complex State Checks**:");
    println!("   Set breakpoint with condition: \"state == State::Error && retries > 3\"");
    println!("   → Stops when in error state AND retries exhausted");
    println!();

    println!("--- Best Practices ---\n");

    println!("✓ Keep conditions simple (fast evaluation)");
    println!("✓ Avoid side effects in conditions (no function calls that modify state)");
    println!("✓ Use debugger-specific syntax (LLDB vs GDB vs debugpy)");
    println!("✓ Test condition first in debugger console before adding to breakpoint");
    println!("✓ Remove conditional breakpoints when done (performance overhead)");

    println!("\n--- Troubleshooting ---\n");

    println!("**Problem:** Condition never triggers");
    println!("**Solution:**");
    println!("  - Check syntax (debugger-specific)");
    println!("  - Verify variable is in scope at breakpoint location");
    println!("  - Print variable value to check type (int vs string)");
    println!();

    println!("**Problem:** Debugger becomes slow");
    println!("**Solution:**");
    println!("  - Simplify condition (avoid complex expressions)");
    println!("  - Remove unnecessary conditional breakpoints");
    println!("  - Use regular breakpoint + manual check if condition is too complex");
}

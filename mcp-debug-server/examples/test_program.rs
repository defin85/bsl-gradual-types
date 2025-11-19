//! Простая тестовая программа для отладки через MCP Debug Server
//!
//! Программа демонстрирует:
//! - Переменные разных типов
//! - Простые вычисления
//! - Циклы
//!
//! Для отладки:
//! ```bash
//! cargo build --example test_program
//! # Затем используйте MCP Debug Tools через Claude Code
//! ```

fn main() {
    println!("🚀 Starting test program...");

    let x = 10;
    let y = 20;
    let sum = add_numbers(x, y);

    println!("Sum of {} and {} is {}", x, y, sum);

    let mut counter = 0;
    for i in 1..=5 {
        counter += i;
        println!("Iteration {}: counter = {}", i, counter);
    }

    let result = calculate_factorial(5);
    println!("Factorial of 5 is {}", result);

    println!("✅ Program completed successfully!");
}

fn add_numbers(a: i32, b: i32) -> i32 {
    let result = a + b;
    result
}

fn calculate_factorial(n: u32) -> u32 {
    if n <= 1 {
        return 1;
    }

    let mut result = 1;
    for i in 2..=n {
        result *= i;
    }

    result
}

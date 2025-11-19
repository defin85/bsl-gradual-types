//! Простая программа для тестирования debug примеров.
//!
//! Компилируется в examples/fixtures/test_program (binary).
//!
//! ## Сборка
//! ```bash
//! rustc examples/fixtures/test_program.rs -o examples/fixtures/test_program -g
//! ```

fn main() {
    let mut counter = 0;

    println!("Starting test program...");

    // Loop для демонстрации stepping
    for i in 0..10 {
        counter += i;  // <- Breakpoint здесь (line 14)
        println!("Iteration {}: counter = {}", i, counter);

        if i == 5 {
            special_function(counter);
        }
    }

    println!("Final counter: {}", counter);
}

fn special_function(value: i32) {
    println!("Special function called with value: {}", value);
    let result = value * 2;
    println!("Result: {}", result);
}

fn main() {
    let mut counter = 0;

    println!("Starting debug test program...");

    // Breakpoint здесь (строка 6)
    counter = 42;

    println!("Counter value: {}", counter);

    // Еще один breakpoint (строка 10)
    counter = counter * 2;

    println!("Final counter: {}", counter);
}

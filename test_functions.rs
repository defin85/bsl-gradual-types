fn add(a: i32, b: i32) -> i32 {
    let sum = a + b;
    sum
}

fn multiply(x: i32, y: i32) -> i32 {
    let product = x * y;
    product
}

fn main() {
    let result1 = add(5, 3);
    let result2 = multiply(result1, 2);
    println!("result1 = {}, result2 = {}", result1, result2);
}

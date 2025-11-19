fn main() {
    let x = 10;  // Строка 2 - breakpoint здесь
    let y = 20;  // Строка 3
    println!("x = {}, y = {}", x, y);  // Строка 4
    let z = x + y;  // Строка 5 - используем x и y
    let result = z * 2;  // Строка 6 - используем z
    println!("z = {}, result = {}", z, result);  // Строка 7
}

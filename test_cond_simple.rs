fn main() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
    let f = 6;
    println!("a={}, b={}, c={}, d={}, e={}, f={}", a, b, c, d, e, f);
    let _result = e * 2;  // Используем e, чтобы он был "в области видимости"
    println!("result = {}", _result);
}

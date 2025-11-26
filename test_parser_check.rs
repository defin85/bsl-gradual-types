// Простой тест для проверки парсинга
use bsl_parser::parse;

fn main() {
    let code = "Док = Документы.ЗаказКлиента;";
    let result = parse(code);
    
    println!("Parse result: {:?}", result);
    
    if let Ok(program) = result {
        for stmt in &program.body {
            println!("Statement: {:?}", stmt);
        }
    }
}

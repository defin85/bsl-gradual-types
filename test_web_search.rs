use bsl_gradual_types::core::platform_resolver::PlatformTypeResolver;

fn main() {
    println!("🌐 Тестируем поиск для веб-сервера...");
    
    let resolver = PlatformTypeResolver::new();
    
    println!("📊 Тестируем поиск 'Массив':");
    let completions = resolver.get_completions("Массив");
    println!("Найдено {} результатов", completions.len());
    
    for (i, completion) in completions.iter().take(5).enumerate() {
        println!("  {}. {} - {:?}", i+1, completion.label, completion.kind);
    }
    
    println!("\n📊 Тестируем поиск 'ТаблицаЗначений':");
    let completions = resolver.get_completions("ТаблицаЗначений");
    println!("Найдено {} результатов", completions.len());
    
    for (i, completion) in completions.iter().take(5).enumerate() {
        println!("  {}. {} - {:?}", i+1, completion.label, completion.kind);
    }
    
    println!("\n📊 Тестируем поиск 'Array':");
    let completions = resolver.get_completions("Array");
    println!("Найдено {} результатов", completions.len());
    
    for (i, completion) in completions.iter().take(5).enumerate() {
        println!("  {}. {} - {:?}", i+1, completion.label, completion.kind);
    }
}
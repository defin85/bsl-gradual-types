//! Пример использования platform_types loader
//!
//! Запуск: cargo run -p bsl-backend --example test_platform_types

use bsl_backend::data::loaders::platform_types::load_all_platform_types;
use bsl_shared::domain::types::FacetKind;

fn main() {
    println!("=== Test Platform Types Loader ===\n");

    // Загружаем все платформенные типы
    let platform_types = load_all_platform_types();

    println!("Loaded {} platform types\n", platform_types.len());

    // Находим тип ТабличнаяЧасть
    if let Some(tabular_type) = platform_types.iter().find(|t| t.name == "ТабличнаяЧасть") {
        println!("✅ Found type: {}", tabular_type.name);
        println!("   English name: {}", tabular_type.english_name);
        println!("   Category: {}", tabular_type.category);
        println!("   Description: {}", tabular_type.description);
        println!("   Facets: {:?}", tabular_type.facets);
        println!();

        // Проверяем facets
        assert!(
            tabular_type.facets.contains(&FacetKind::Collection),
            "ТабличнаяЧасть должен иметь facet Collection"
        );
        println!("✅ Facet Collection present");

        // Проверяем количество методов
        println!("\n📋 Methods ({} total):", tabular_type.methods.len());
        assert_eq!(
            tabular_type.methods.len(),
            16,
            "Должно быть 16 методов"
        );

        // Показываем методы с Generic параметром "T"
        println!("\n🔹 Generic methods (return type = T):");
        for method in &tabular_type.methods {
            if method.return_type == "T" || method.params.iter().any(|p| p.param_type == "T") {
                println!("   - {}(", method.name);
                for (i, param) in method.params.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!("{}: {}", param.name, param.param_type);
                }
                print!(")");
                if !method.return_type.is_empty() {
                    print!(" → {}", method.return_type);
                }
                println!();
            }
        }

        // Показываем методы БЕЗ Generic параметра
        println!("\n🔹 Non-generic methods:");
        for method in &tabular_type.methods {
            if method.return_type != "T"
                && !method.params.iter().any(|p| p.param_type == "T")
                && !method.return_type.contains("<T>")
            {
                println!("   - {}(", method.name);
                for (i, param) in method.params.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!("{}: {}", param.name, param.param_type);
                }
                print!(")");
                if !method.return_type.is_empty() && method.return_type != "Неопределено" {
                    print!(" → {}", method.return_type);
                }
                println!();
            }
        }

        // Проверяем свойства
        println!("\n🏷️ Properties ({} total):", tabular_type.properties.len());
        for prop in &tabular_type.properties {
            println!(
                "   - {}: {} (readonly: {})",
                prop.name, prop.prop_type, prop.is_readonly
            );
        }

        println!("\n✅ All checks passed!");
    } else {
        panic!("❌ Type 'ТабличнаяЧасть' not found!");
    }
}

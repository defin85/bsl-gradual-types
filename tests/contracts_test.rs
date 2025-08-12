#[cfg(test)]
mod tests {
    use bsl_gradual_types::core::{
        contracts::{ContractGenerator, ContractMode},
        types::{TypeResolution, Certainty, ResolutionResult, ConcreteType, PlatformType},
    };
    
    #[test]
    fn test_generate_contract_for_unknown_type() {
        let generator = ContractGenerator::new(0.8, ContractMode::Assert);
        
        // Создаём Unknown тип
        let resolution = TypeResolution::unknown();
        
        // Генерируем контракт
        let contract = generator.generate_contract(&resolution);
        
        assert!(contract.is_some(), "Должен сгенерировать контракт для Unknown типа");
        
        let contract = contract.unwrap();
        println!("Контракт для Unknown типа:");
        println!("{}", contract.check_code);
        
        assert!(contract.check_code.contains("ЗначениеЗаполнено"));
    }
    
    #[test]
    fn test_generate_contract_for_low_confidence() {
        let generator = ContractGenerator::new(0.8, ContractMode::Assert);
        
        // Создаём тип с низкой уверенностью
        let mut resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Число".to_string(),
            methods: vec![],
            properties: vec![],
        }));
        resolution.certainty = Certainty::Inferred(0.5); // Низкая уверенность
        
        // Генерируем контракт
        let contract = generator.generate_contract(&resolution);
        
        assert!(contract.is_some(), "Должен сгенерировать контракт для типа с низкой уверенностью");
        
        let contract = contract.unwrap();
        println!("\nКонтракт для типа с низкой уверенностью:");
        println!("{}", contract.check_code);
        
        assert!(contract.check_code.contains("Тип(\"Число\")"));
    }
    
    #[test]
    fn test_no_contract_for_known_type() {
        let generator = ContractGenerator::new(0.8, ContractMode::Assert);
        
        // Создаём Known тип
        let resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Строка".to_string(),
            methods: vec![],
            properties: vec![],
        }));
        
        // Генерируем контракт
        let contract = generator.generate_contract(&resolution);
        
        assert!(contract.is_none(), "Не должен генерировать контракт для Known типа");
    }
    
    #[test]
    fn test_generate_union_contract() {
        let generator = ContractGenerator::new(0.8, ContractMode::Assert);
        
        // Создаём Union тип
        let resolution = TypeResolution {
            certainty: Certainty::Inferred(0.6), // Низкая уверенность
            result: ResolutionResult::Union(vec![
                bsl_gradual_types::core::types::WeightedType {
                    type_: ConcreteType::Platform(PlatformType {
                        name: "Строка".to_string(),
                        methods: vec![],
                        properties: vec![],
                    }),
                    weight: 0.5,
                },
                bsl_gradual_types::core::types::WeightedType {
                    type_: ConcreteType::Platform(PlatformType {
                        name: "Число".to_string(),
                        methods: vec![],
                        properties: vec![],
                    }),
                    weight: 0.5,
                },
            ]),
            source: bsl_gradual_types::core::types::ResolutionSource::Inferred,
            metadata: Default::default(),
            active_facet: None,
            available_facets: vec![],
        };
        
        // Генерируем контракт
        let contract = generator.generate_contract(&resolution);
        
        assert!(contract.is_some(), "Должен сгенерировать контракт для Union типа");
        
        let contract = contract.unwrap();
        println!("\nКонтракт для Union типа:");
        println!("{}", contract.check_code);
        
        assert!(contract.check_code.contains("Тип(\"Строка\") ИЛИ"));
        assert!(contract.check_code.contains("Тип(\"Число\")"));
    }
    
    #[test]
    fn test_contract_modes() {
        let unknown = TypeResolution::unknown();
        
        // Warning mode
        let generator = ContractGenerator::new(0.8, ContractMode::Warning);
        let contract = generator.generate_contract(&unknown).unwrap();
        println!("\nWarning mode:");
        println!("{}", contract.check_code);
        assert!(contract.check_code.contains("ВНИМАНИЕ"));
        
        // Assert mode
        let generator = ContractGenerator::new(0.8, ContractMode::Assert);
        let contract = generator.generate_contract(&unknown).unwrap();
        println!("\nAssert mode:");
        println!("{}", contract.check_code);
        assert!(contract.check_code.contains("ВызватьИсключение"));
        
        // Report mode
        let generator = ContractGenerator::new(0.8, ContractMode::Report);
        let contract = generator.generate_contract(&unknown).unwrap();
        println!("\nReport mode:");
        println!("{}", contract.check_code);
        assert!(contract.check_code.contains("ЗаписьЖурналаРегистрации"));
    }
}
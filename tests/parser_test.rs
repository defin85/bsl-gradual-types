#[cfg(test)]
mod tests {
    use bsl_gradual_types::parser::{BslParser, Expression, Statement};

    #[test]
    fn test_parse_simple_program() {
        let code = r#"
            Перем Счетчик = 0;
            
            Процедура УвеличитьСчетчик() Экспорт
                Счетчик = Счетчик + 1;
            КонецПроцедуры
            
            Функция ПолучитьСчетчик()
                Возврат Счетчик;
            КонецФункции
        "#;

        let mut parser = BslParser::new(code).expect("Должен создать парсер");
        let program = parser.parse().expect("Должен распарсить программу");

        assert_eq!(program.statements.len(), 3);

        // Проверяем объявление переменной
        match &program.statements[0] {
            Statement::VarDeclaration {
                name,
                value,
                export,
            } => {
                assert_eq!(name, "Счетчик");
                assert!(!export);
                assert!(matches!(value, Some(Expression::Number(0.0))));
            }
            _ => panic!("Ожидалось объявление переменной"),
        }

        // Проверяем процедуру
        match &program.statements[1] {
            Statement::ProcedureDecl {
                name,
                params,
                body,
                export,
            } => {
                assert_eq!(name, "УвеличитьСчетчик");
                assert!(export);
                assert_eq!(params.len(), 0);
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Ожидалась процедура"),
        }

        // Проверяем функцию
        match &program.statements[2] {
            Statement::FunctionDecl {
                name,
                params,
                return_value,
                ..
            } => {
                assert_eq!(name, "ПолучитьСчетчик");
                assert_eq!(params.len(), 0);
                assert!(return_value.is_some());
            }
            _ => panic!("Ожидалась функция"),
        }
    }

    #[test]
    fn test_parse_if_statement() {
        let code = r#"
            Если Сумма > 1000 Тогда
                Скидка = 10;
            ИначеЕсли Сумма > 500 Тогда
                Скидка = 5;
            Иначе
                Скидка = 0;
            КонецЕсли;
        "#;

        let mut parser = BslParser::new(code).expect("Должен создать парсер");
        let program = parser.parse().expect("Должен распарсить программу");

        assert_eq!(program.statements.len(), 1);

        match &program.statements[0] {
            Statement::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                // Проверяем условие
                assert!(matches!(condition, Expression::Binary { .. }));

                // Проверяем then ветку
                assert_eq!(then_branch.len(), 1);

                // Проверяем else if ветки
                assert_eq!(else_if_branches.len(), 1);

                // Проверяем else ветку
                assert!(else_branch.is_some());
                assert_eq!(else_branch.as_ref().unwrap().len(), 1);
            }
            _ => panic!("Ожидался оператор Если"),
        }
    }

    #[test]
    fn test_parse_loops() {
        let code = r#"
            Для Индекс = 1 По 10 Цикл
                Массив[Индекс] = Индекс * 2;
            КонецЦикла;
            
            Для Каждого Элемент Из Коллекция Цикл
                Обработать(Элемент);
            КонецЦикла;
            
            Пока Счетчик < 100 Цикл
                Счетчик = Счетчик + 1;
            КонецЦикла;
        "#;

        let mut parser = BslParser::new(code).expect("Должен создать парсер");
        let program = parser.parse().expect("Должен распарсить программу");

        assert_eq!(program.statements.len(), 3);

        // Проверяем цикл Для
        assert!(matches!(&program.statements[0], Statement::For { .. }));

        // Проверяем цикл Для Каждого
        assert!(matches!(&program.statements[1], Statement::ForEach { .. }));

        // Проверяем цикл Пока
        assert!(matches!(&program.statements[2], Statement::While { .. }));
    }

    #[test]
    fn test_parse_expressions() {
        let code = r#"
            Результат = (А + Б) * 2;
            Объект.Свойство = Новый Массив;
            Элемент = Массив[0];
            Значение = ?(Условие, ЗначениеИстина, ЗначениеЛожь);
            Вызов = Функция(Параметр1, Параметр2);
        "#;

        let mut parser = BslParser::new(code).expect("Должен создать парсер");
        let program = parser.parse().expect("Должен распарсить программу");

        assert_eq!(program.statements.len(), 5);

        // Все операторы должны быть присваиваниями
        for stmt in &program.statements {
            assert!(matches!(stmt, Statement::Assignment { .. }));
        }
    }

    #[test]
    fn test_parse_try_catch() {
        let code = r#"
            Попытка
                РискованнаяОперация();
            Исключение
                ЗаписатьОшибку();
            КонецПопытки;
        "#;

        let mut parser = BslParser::new(code).expect("Должен создать парсер");
        let program = parser.parse().expect("Должен распарсить программу");

        assert_eq!(program.statements.len(), 1);

        match &program.statements[0] {
            Statement::Try {
                try_block,
                catch_block,
            } => {
                assert_eq!(try_block.len(), 1);
                assert!(catch_block.is_some());
                assert_eq!(catch_block.as_ref().unwrap().len(), 1);
            }
            _ => panic!("Ожидался блок Попытка-Исключение"),
        }
    }

    #[test]
    fn test_parse_platform_types() {
        let code = r#"
            Контрагент = Справочники.Контрагенты.НайтиПоКоду("123");
            НовыйДокумент = Документы.ЗаказПокупателя.СоздатьДокумент();
            Запрос = Новый Запрос;
            Запрос.Текст = "ВЫБРАТЬ * ИЗ Справочник.Контрагенты";
        "#;

        let mut parser = BslParser::new(code).expect("Должен создать парсер");
        let program = parser.parse().expect("Должен распарсить программу");

        assert_eq!(program.statements.len(), 4);

        // Проверяем доступ к членам
        match &program.statements[0] {
            Statement::Assignment { value, .. } => match value {
                Expression::Call { function, args } => {
                    assert!(matches!(**function, Expression::MemberAccess { .. }));
                    assert_eq!(args.len(), 1);
                }
                _ => panic!("Ожидался вызов метода"),
            },
            _ => panic!("Ожидалось присваивание"),
        }

        // Проверяем создание нового объекта
        match &program.statements[2] {
            Statement::Assignment { value, .. } => {
                assert!(
                    matches!(value, Expression::New { type_name, .. } if type_name == "Запрос")
                );
            }
            _ => panic!("Ожидалось присваивание"),
        }
    }
}

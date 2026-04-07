//! Unit tests for VSCode webview integration

#[cfg(test)]
mod vscode_webview_tests {
    use super::super::common::VsCodeMessage;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct TestData {
        value: String,
    }

    #[test]
    fn test_vscode_message_new() {
        let msg = VsCodeMessage::new(
            "updateTypeInfo",
            TestData {
                value: "test".to_string(),
            },
        );

        assert_eq!(msg.msg_type, "updateTypeInfo");
        assert!(msg.data.is_some());
        assert!(msg.error.is_none());
    }

    #[test]
    fn test_vscode_message_simple() {
        let msg = VsCodeMessage::<()>::simple("ready");

        assert_eq!(msg.msg_type, "ready");
        assert!(msg.data.is_none());
        assert!(msg.error.is_none());
    }

    #[test]
    fn test_vscode_message_error() {
        let msg = VsCodeMessage::<()>::error("error", "Test error message");

        assert_eq!(msg.msg_type, "error");
        assert!(msg.data.is_none());
        assert_eq!(msg.error, Some("Test error message".to_string()));
    }

    #[test]
    fn test_vscode_message_serialization() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct SimpleData {
            name: String,
        }

        let msg = VsCodeMessage::new(
            "test",
            SimpleData {
                name: "TestName".to_string(),
            },
        );

        let serialized = serde_json::to_string(&msg).unwrap();
        assert!(serialized.contains("test"));
        assert!(serialized.contains("TestName"));
    }

    #[test]
    fn test_vscode_message_type_rename() {
        let json_str = r#"{"type":"ready"}"#;
        let msg: VsCodeMessage<()> = serde_json::from_str(json_str).unwrap();

        assert_eq!(msg.msg_type, "ready");
    }

    #[test]
    fn test_vscode_message_skip_none() {
        let msg = VsCodeMessage {
            msg_type: "test".to_string(),
            data: None::<TestData>,
            error: None,
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        // Should not contain "data" or "error" fields if they are None
        assert!(serialized.contains("type"));
        assert!(!serialized.contains("\"error\""));
    }

    #[test]
    fn test_vscode_type_info_round_trip() {
        use super::super::type_details_app::{
            VsCodeMethod, VsCodeParameter, VsCodeProperty, VsCodeTypeInfo,
        };

        let type_info = VsCodeTypeInfo {
            name: "Массив".to_string(),
            certainty: "High".to_string(),
            facet: "Collection".to_string(),
            methods: vec![VsCodeMethod {
                name: "Add".to_string(),
                description: "Adds element".to_string(),
                returns: "Void".to_string(),
                parameters: vec![VsCodeParameter {
                    name: "element".to_string(),
                    param_type: "Any".to_string(),
                }],
            }],
            properties: vec![VsCodeProperty {
                name: "Count".to_string(),
                prop_type: "Integer".to_string(),
                readonly: true,
            }],
            documentation: "Array type".to_string(),
        };

        let serialized = serde_json::to_string(&type_info).unwrap();
        let deserialized: VsCodeTypeInfo = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, type_info.name);
        assert_eq!(deserialized.certainty, type_info.certainty);
        assert_eq!(deserialized.facet, type_info.facet);
        assert_eq!(deserialized.methods.len(), 1);
        assert_eq!(deserialized.properties.len(), 1);
    }

    #[test]
    fn test_vscode_certainty_values() {
        use super::super::type_details_app::VsCodeTypeInfo;

        let test_cases = vec![
            ("High", "Высокая", "Test russian high"),
            ("Medium", "Средняя", "Test russian medium"),
            ("Low", "Низкая", "Test russian low"),
        ];

        for (en, ru, desc) in test_cases {
            let type_info_en = VsCodeTypeInfo {
                name: "Test".to_string(),
                certainty: en.to_string(),
                facet: "TestFacet".to_string(),
                methods: vec![],
                properties: vec![],
                documentation: desc.to_string(),
            };

            let type_info_ru = VsCodeTypeInfo {
                name: "Test".to_string(),
                certainty: ru.to_string(),
                facet: "TestFacet".to_string(),
                methods: vec![],
                properties: vec![],
                documentation: desc.to_string(),
            };

            let serialized_en = serde_json::to_string(&type_info_en).unwrap();
            let serialized_ru = serde_json::to_string(&type_info_ru).unwrap();

            assert!(serialized_en.contains(en));
            assert!(serialized_ru.contains(ru));
        }
    }
}

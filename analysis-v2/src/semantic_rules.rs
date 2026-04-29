#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonModuleFactoryTargetMode {
    CommonModule,
    CommonModuleOrManager,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonModuleFactoryRule {
    pub id: String,
    pub owner: String,
    pub method: String,
    pub argument_index: usize,
    pub target_mode: CommonModuleFactoryTargetMode,
    pub enabled: bool,
}

impl CommonModuleFactoryRule {
    pub fn builtin_bsp_common_module() -> Self {
        Self {
            id: "bsp-common-purpose-common-module".to_string(),
            owner: "ОбщиеМодули.ОбщегоНазначения".to_string(),
            method: "ОбщийМодуль".to_string(),
            argument_index: 0,
            target_mode: CommonModuleFactoryTargetMode::CommonModuleOrManager,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonModuleFactoryRegistry {
    rules: Vec<CommonModuleFactoryRule>,
}

impl Default for CommonModuleFactoryRegistry {
    fn default() -> Self {
        Self::builtin_bsp()
    }
}

impl CommonModuleFactoryRegistry {
    pub fn builtin_bsp() -> Self {
        Self {
            rules: vec![CommonModuleFactoryRule::builtin_bsp_common_module()],
        }
    }

    pub fn new(rules: Vec<CommonModuleFactoryRule>) -> Self {
        Self { rules }
    }

    pub fn rules(&self) -> &[CommonModuleFactoryRule] {
        &self.rules
    }

    pub fn find_rule(&self, owner_type: &str, method: &str) -> Option<&CommonModuleFactoryRule> {
        let owner_key = normalized_rule_key(owner_type);
        let method_key = normalized_rule_key(method);

        self.rules.iter().find(|rule| {
            rule.enabled
                && normalized_rule_key(&rule.owner) == owner_key
                && normalized_rule_key(&rule.method) == method_key
        })
    }
}

pub fn normalized_rule_key(value: &str) -> String {
    value.trim().to_lowercase()
}

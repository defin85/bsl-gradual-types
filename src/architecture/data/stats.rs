#[derive(Debug, Clone, Default)]
pub struct RepositoryStats {
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
    pub user_defined_types: usize,
    pub types_count: usize,
}

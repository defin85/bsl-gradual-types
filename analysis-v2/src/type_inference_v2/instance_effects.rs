use super::*;
use bsl_shared::domain::type_id::normalize;
use bsl_shared::domain::types::{GenericType, SpecialType, StructuralMember, StructuralMemberSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) struct InstanceId(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InstanceBinding {
    Direct(InstanceId),
    ValueTableRow { table_instance: InstanceId },
}

#[derive(Debug, Clone, Default)]
pub(super) struct InstanceEffectStore {
    next_id: u64,
    states: HashMap<InstanceId, InstanceState>,
}

#[derive(Debug, Clone)]
struct InstanceState {
    kind: InstanceKind,
}

#[derive(Debug, Clone)]
enum InstanceKind {
    Map(MapEffects),
    Structure(StructureEffects),
    ValueTable(ValueTableEffects),
}

#[derive(Debug, Clone, Default)]
struct MapEffects {
    generic_key_type: Option<TypeResolution>,
    generic_value_type: Option<TypeResolution>,
    literal_keys: BTreeMap<String, StructuralEffectEntry>,
}

#[derive(Debug, Clone, Default)]
struct StructureEffects {
    fields: BTreeMap<String, StructuralEffectEntry>,
}

#[derive(Debug, Clone, Default)]
struct ValueTableEffects {
    columns: BTreeMap<String, StructuralEffectEntry>,
}

#[derive(Debug, Clone)]
struct StructuralEffectEntry {
    canonical_name: String,
    value_type: TypeResolution,
    source_span: Option<StructuralMemberSpan>,
    certainty: Certainty,
}

impl StructuralEffectEntry {
    fn from_resolution(
        canonical_name: impl Into<String>,
        value_type: TypeResolution,
        source_span: Option<StructuralMemberSpan>,
    ) -> Self {
        let certainty = value_type.certainty;
        Self {
            canonical_name: canonical_name.into(),
            value_type,
            source_span,
            certainty,
        }
    }

    fn to_structural_member(&self) -> StructuralMember {
        StructuralMember::new(
            self.canonical_name.clone(),
            self.value_type.clone(),
            self.source_span,
            self.certainty,
        )
    }

    fn downgrade_for_branch(&self) -> Self {
        let mut downgraded = self.clone();
        downgraded.certainty = downgrade_certainty(downgraded.certainty);
        downgraded.value_type.certainty = downgrade_certainty(downgraded.value_type.certainty);
        downgraded
    }
}

impl InstanceEffectStore {
    pub(super) fn new_map_instance(&mut self, base_resolution: &TypeResolution) -> InstanceBinding {
        let mut effects = MapEffects::default();
        if let ResolutionResult::Generic(GenericType {
            base_type,
            type_params,
        }) = &base_resolution.result
        {
            if base_type.eq_ignore_ascii_case("Соответствие") {
                effects.generic_key_type = type_params.first().and_then(concrete_param_resolution);
                effects.generic_value_type = type_params.get(1).and_then(concrete_param_resolution);
            }
        }
        InstanceBinding::Direct(self.insert_state(InstanceKind::Map(effects)))
    }

    pub(super) fn new_structure_instance(&mut self) -> InstanceBinding {
        InstanceBinding::Direct(
            self.insert_state(InstanceKind::Structure(StructureEffects::default())),
        )
    }

    pub(super) fn new_value_table_instance(&mut self) -> InstanceBinding {
        InstanceBinding::Direct(
            self.insert_state(InstanceKind::ValueTable(ValueTableEffects::default())),
        )
    }

    pub(super) fn value_table_row_binding(&self, table_instance: InstanceId) -> InstanceBinding {
        InstanceBinding::ValueTableRow { table_instance }
    }

    pub(super) fn direct_instance(binding: &InstanceBinding) -> Option<InstanceId> {
        match binding {
            InstanceBinding::Direct(id) => Some(*id),
            InstanceBinding::ValueTableRow { .. } => None,
        }
    }

    pub(super) fn is_map_instance(&self, instance_id: InstanceId) -> bool {
        matches!(
            self.states.get(&instance_id).map(|state| &state.kind),
            Some(InstanceKind::Map(_))
        )
    }

    pub(super) fn is_structure_instance(&self, instance_id: InstanceId) -> bool {
        matches!(
            self.states.get(&instance_id).map(|state| &state.kind),
            Some(InstanceKind::Structure(_))
        )
    }

    pub(super) fn is_value_table_instance(&self, instance_id: InstanceId) -> bool {
        matches!(
            self.states.get(&instance_id).map(|state| &state.kind),
            Some(InstanceKind::ValueTable(_))
        )
    }

    pub(super) fn materialize(
        &self,
        base_resolution: &TypeResolution,
        binding: Option<&InstanceBinding>,
    ) -> TypeResolution {
        let mut materialized = strip_structural_members(base_resolution.clone());

        match binding {
            Some(InstanceBinding::Direct(instance_id)) => match self.states.get(instance_id) {
                Some(InstanceState {
                    kind: InstanceKind::Structure(effects),
                }) => {
                    for entry in effects.fields.values() {
                        materialized.add_structural_member(entry.to_structural_member());
                    }
                }
                Some(InstanceState {
                    kind: InstanceKind::ValueTable(_),
                })
                | Some(InstanceState {
                    kind: InstanceKind::Map(_),
                })
                | None => {}
            },
            Some(InstanceBinding::ValueTableRow { table_instance }) => {
                if let Some(InstanceState {
                    kind: InstanceKind::ValueTable(effects),
                }) = self.states.get(table_instance)
                {
                    for entry in effects.columns.values() {
                        materialized.add_structural_member(entry.to_structural_member());
                    }
                }
            }
            None => {}
        }

        materialized
    }

    pub(super) fn insert_map_value(
        &mut self,
        instance_id: InstanceId,
        literal_key: Option<(String, String)>,
        key_type: Option<TypeResolution>,
        value_type: TypeResolution,
        source_span: bsl_shared::ir::Span,
    ) {
        let Some(InstanceState {
            kind: InstanceKind::Map(effects),
        }) = self.states.get_mut(&instance_id)
        else {
            return;
        };

        if let Some(key_type) = key_type {
            merge_optional_resolution(&mut effects.generic_key_type, key_type);
        }
        merge_optional_resolution(&mut effects.generic_value_type, value_type.clone());

        if let Some((normalized_key, canonical_key)) = literal_key {
            effects.literal_keys.insert(
                normalized_key,
                StructuralEffectEntry::from_resolution(
                    canonical_key,
                    value_type,
                    Some(span_to_structural(source_span)),
                ),
            );
        }
    }

    pub(super) fn resolve_map_value(
        &self,
        instance_id: InstanceId,
        literal_key: Option<&str>,
    ) -> Option<TypeResolution> {
        let InstanceState {
            kind: InstanceKind::Map(effects),
        } = self.states.get(&instance_id)?
        else {
            return None;
        };

        if let Some(key) = literal_key {
            let normalized = normalize(key);
            if let Some(entry) = effects.literal_keys.get(&normalized) {
                return Some(entry.value_type.clone());
            }
        }

        effects.generic_value_type.clone()
    }

    pub(super) fn insert_structure_field(
        &mut self,
        instance_id: InstanceId,
        field_name: &str,
        value_type: TypeResolution,
        source_span: bsl_shared::ir::Span,
    ) {
        let Some(InstanceState {
            kind: InstanceKind::Structure(effects),
        }) = self.states.get_mut(&instance_id)
        else {
            return;
        };

        effects.fields.insert(
            normalize(field_name),
            StructuralEffectEntry::from_resolution(
                field_name.to_string(),
                value_type,
                Some(span_to_structural(source_span)),
            ),
        );
    }

    pub(super) fn insert_value_table_column(
        &mut self,
        instance_id: InstanceId,
        column_name: &str,
        value_type: TypeResolution,
        source_span: bsl_shared::ir::Span,
    ) {
        let Some(InstanceState {
            kind: InstanceKind::ValueTable(effects),
        }) = self.states.get_mut(&instance_id)
        else {
            return;
        };

        effects.columns.insert(
            normalize(column_name),
            StructuralEffectEntry::from_resolution(
                column_name.to_string(),
                value_type,
                Some(span_to_structural(source_span)),
            ),
        );
    }

    pub(super) fn merge_branch(base: &Self, left: &Self, right: &Self) -> Self {
        let mut merged = base.clone();
        merged.next_id = merged.next_id.max(left.next_id).max(right.next_id);

        let instance_ids: BTreeSet<InstanceId> = left
            .states
            .keys()
            .chain(right.states.keys())
            .copied()
            .collect();

        for instance_id in instance_ids {
            let Some(left_state) = left.states.get(&instance_id) else {
                continue;
            };
            let Some(right_state) = right.states.get(&instance_id) else {
                merged.states.insert(instance_id, left_state.clone());
                continue;
            };

            merged
                .states
                .insert(instance_id, merge_instance_state(left_state, right_state));
        }

        merged
    }

    fn insert_state(&mut self, kind: InstanceKind) -> InstanceId {
        let id = InstanceId(self.next_id);
        self.next_id += 1;
        self.states.insert(id, InstanceState { kind });
        id
    }
}

impl TypeEnv {
    pub(super) fn variable_resolution(&self, key: &str) -> Option<TypeResolution> {
        let base = self.variables.get(key)?;
        Some(
            self.instance_effects
                .materialize(base, self.instance_bindings.get(key)),
        )
    }

    pub(super) fn variable_base_resolution(&self, key: &str) -> Option<&TypeResolution> {
        self.variables.get(key)
    }

    pub(super) fn variable_binding(&self, key: &str) -> Option<&InstanceBinding> {
        self.instance_bindings.get(key)
    }

    pub(super) fn set_variable_value(
        &mut self,
        key: String,
        base_resolution: TypeResolution,
        binding: Option<InstanceBinding>,
    ) {
        self.variables
            .insert(key.clone(), strip_structural_members(base_resolution));
        if let Some(binding) = binding {
            self.instance_bindings.insert(key, binding);
        } else {
            self.instance_bindings.remove(&key);
        }
    }
}

pub(super) fn strip_structural_members(mut resolution: TypeResolution) -> TypeResolution {
    resolution.metadata.structural_members.clear();
    resolution
}

fn concrete_param_resolution(concrete: &ConcreteType) -> Option<TypeResolution> {
    if matches!(concrete, ConcreteType::Special(SpecialType::Undefined)) {
        None
    } else {
        Some(TypeResolution::known(concrete.clone()))
    }
}

fn span_to_structural(span: bsl_shared::ir::Span) -> StructuralMemberSpan {
    StructuralMemberSpan::new(span.start, span.end)
}

fn downgrade_certainty(certainty: Certainty) -> Certainty {
    match certainty {
        Certainty::Known | Certainty::Inferred => Certainty::InferredWeak,
        Certainty::InferredWeak => Certainty::InferredWeak,
        Certainty::Unknown => Certainty::Unknown,
    }
}

fn merge_instance_state(left: &InstanceState, right: &InstanceState) -> InstanceState {
    let kind = match (&left.kind, &right.kind) {
        (InstanceKind::Map(left), InstanceKind::Map(right)) => {
            InstanceKind::Map(merge_map_effects(left, right))
        }
        (InstanceKind::Structure(left), InstanceKind::Structure(right)) => {
            InstanceKind::Structure(merge_structure_effects(left, right))
        }
        (InstanceKind::ValueTable(left), InstanceKind::ValueTable(right)) => {
            InstanceKind::ValueTable(merge_value_table_effects(left, right))
        }
        _ => left.kind.clone(),
    };

    InstanceState { kind }
}

fn merge_map_effects(left: &MapEffects, right: &MapEffects) -> MapEffects {
    let mut merged = MapEffects::default();
    if let Some(left_key) = left.generic_key_type.clone() {
        merge_optional_resolution(&mut merged.generic_key_type, left_key);
    }
    if let Some(right_key) = right.generic_key_type.clone() {
        merge_optional_resolution(&mut merged.generic_key_type, right_key);
    }
    if let Some(left_value) = left.generic_value_type.clone() {
        merge_optional_resolution(&mut merged.generic_value_type, left_value);
    }
    if let Some(right_value) = right.generic_value_type.clone() {
        merge_optional_resolution(&mut merged.generic_value_type, right_value);
    }
    merged.literal_keys = merge_structural_map(&left.literal_keys, &right.literal_keys);
    merged
}

fn merge_structure_effects(left: &StructureEffects, right: &StructureEffects) -> StructureEffects {
    StructureEffects {
        fields: merge_structural_map(&left.fields, &right.fields),
    }
}

fn merge_value_table_effects(
    left: &ValueTableEffects,
    right: &ValueTableEffects,
) -> ValueTableEffects {
    ValueTableEffects {
        columns: merge_structural_map(&left.columns, &right.columns),
    }
}

fn merge_structural_map(
    left: &BTreeMap<String, StructuralEffectEntry>,
    right: &BTreeMap<String, StructuralEffectEntry>,
) -> BTreeMap<String, StructuralEffectEntry> {
    let keys: BTreeSet<String> = left.keys().chain(right.keys()).cloned().collect();
    let mut merged = BTreeMap::new();

    for key in keys {
        let entry = match (left.get(&key), right.get(&key)) {
            (Some(left_entry), Some(right_entry)) => merge_effect_entry(left_entry, right_entry),
            (Some(left_entry), None) => left_entry.downgrade_for_branch(),
            (None, Some(right_entry)) => right_entry.downgrade_for_branch(),
            (None, None) => continue,
        };
        merged.insert(key, entry);
    }

    merged
}

fn merge_effect_entry(
    left: &StructuralEffectEntry,
    right: &StructuralEffectEntry,
) -> StructuralEffectEntry {
    StructuralEffectEntry {
        canonical_name: left.canonical_name.clone(),
        value_type: merge_resolutions(&left.value_type, &right.value_type),
        source_span: left.source_span.or(right.source_span),
        certainty: merge_certainty(left.certainty, right.certainty),
    }
}

fn merge_optional_resolution(slot: &mut Option<TypeResolution>, incoming: TypeResolution) {
    match slot.take() {
        Some(existing) => *slot = Some(merge_resolutions(&existing, &incoming)),
        None => *slot = Some(incoming),
    }
}

pub(super) fn merge_resolutions(left: &TypeResolution, right: &TypeResolution) -> TypeResolution {
    if left.type_name().eq_ignore_ascii_case(&right.type_name()) {
        let mut merged = left.clone();
        merged.certainty = merge_certainty(left.certainty, right.certainty);
        for member in right.structural_members() {
            merged.add_structural_member(member.clone());
        }
        return merged;
    }

    match (&left.result, &right.result) {
        (ResolutionResult::Concrete(left_concrete), ResolutionResult::Concrete(right_concrete)) => {
            TypeResolution {
                certainty: Certainty::InferredWeak,
                result: ResolutionResult::normalize_union(vec![
                    WeightedType::with_weight(left_concrete.clone(), 0.5),
                    WeightedType::with_weight(right_concrete.clone(), 0.5),
                ]),
                source: ResolutionSource::Inferred,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            }
        }
        _ => TypeResolution::unknown(),
    }
}

fn merge_certainty(left: Certainty, right: Certainty) -> Certainty {
    use Certainty::*;

    match (left, right) {
        (Unknown, _) | (_, Unknown) => Unknown,
        (InferredWeak, _) | (_, InferredWeak) => InferredWeak,
        (Inferred, Inferred) => Inferred,
        (Known, Known) => Known,
        _ => InferredWeak,
    }
}

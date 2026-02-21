use crate::ui::event::UiFieldValue;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MAX_COMPONENTS: usize = 64;
pub const MAX_DEPTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum ComponentKind {
    Markdown,
    Form,
    Code,
    Diff,
    Button,
    Unknown(String),
}

impl ComponentKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Markdown => "markdown",
            Self::Form => "form",
            Self::Code => "code",
            Self::Diff => "diff",
            Self::Button => "button",
            Self::Unknown(kind) => kind.as_str(),
        }
    }

    pub fn is_actionable(&self) -> bool {
        matches!(self, Self::Form | Self::Button)
    }
}

impl<'de> Deserialize<'de> for ComponentKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.as_str() {
            "markdown" => Self::Markdown,
            "form" => Self::Form,
            "code" => Self::Code,
            "diff" => Self::Diff,
            "button" => Self::Button,
            _ => Self::Unknown(raw),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FormFieldKind {
    Text,
    Number,
    Select,
    Checkbox,
    Unknown(String),
}

impl FormFieldKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Select => "select",
            Self::Checkbox => "checkbox",
            Self::Unknown(kind) => kind.as_str(),
        }
    }
}

impl<'de> Deserialize<'de> for FormFieldKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.as_str() {
            "text" => Self::Text,
            "number" => Self::Number,
            "select" => Self::Select,
            "checkbox" => Self::Checkbox,
            _ => Self::Unknown(raw),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonStyle {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
}

impl<'de> Deserialize<'de> for DiffLineKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        match raw.as_str() {
            "added" => Ok(Self::Added),
            "removed" => Ok(Self::Removed),
            "context" => Ok(Self::Context),
            _ => Err(serde::de::Error::custom(format!(
                "unknown diff line kind: {raw}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputContract {
    pub component_id: String,
    pub event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFormField {
    pub id: String,
    pub label: String,
    pub kind: FormFieldKind,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub default: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawComponent {
    pub id: String,
    pub kind: ComponentKind,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub fields: Vec<RawFormField>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub lines: Vec<DiffLine>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub variant: Option<ButtonStyle>,
    #[serde(default)]
    pub children: Vec<RawComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSchema {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub outputs: Vec<OutputContract>,
    #[serde(default)]
    pub components: Vec<RawComponent>,
}

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone)]
pub struct ValidatedSchema {
    pub schema_version: u32,
    pub components: Vec<ValidatedComponent>,
}

#[derive(Debug, Clone)]
pub enum ValidatedComponent {
    Markdown(MarkdownComponent),
    Form(FormComponent),
    Code(CodeComponent),
    Diff(DiffComponent),
    Button(ButtonComponent),
}

impl ValidatedComponent {
    pub fn children(&self) -> &[ValidatedComponent] {
        match self {
            Self::Markdown(component) => &component.children,
            Self::Form(component) => &component.children,
            Self::Code(component) => &component.children,
            Self::Diff(component) => &component.children,
            Self::Button(component) => &component.children,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarkdownComponent {
    pub id: String,
    pub text: String,
    pub children: Vec<ValidatedComponent>,
}

#[derive(Debug, Clone)]
pub struct FormComponent {
    pub id: String,
    pub title: Option<String>,
    pub fields: Vec<ValidatedFormField>,
    pub children: Vec<ValidatedComponent>,
}

#[derive(Debug, Clone)]
pub struct CodeComponent {
    pub id: String,
    pub language: Option<String>,
    pub code: String,
    pub children: Vec<ValidatedComponent>,
}

#[derive(Debug, Clone)]
pub struct DiffComponent {
    pub id: String,
    pub lines: Vec<DiffLine>,
    pub children: Vec<ValidatedComponent>,
}

#[derive(Debug, Clone)]
pub struct ButtonComponent {
    pub id: String,
    pub label: String,
    pub output_event_id: String,
    pub variant: ButtonStyle,
    pub children: Vec<ValidatedComponent>,
}

#[derive(Debug, Clone)]
pub enum ValidatedFormField {
    Text(TextField),
    Number(NumberField),
    Select(SelectField),
    Checkbox(CheckboxField),
}

impl ValidatedFormField {
    pub fn id(&self) -> &str {
        match self {
            Self::Text(field) => &field.id,
            Self::Number(field) => &field.id,
            Self::Select(field) => &field.id,
            Self::Checkbox(field) => &field.id,
        }
    }

    pub fn default_value(&self) -> UiFieldValue {
        match self {
            Self::Text(field) => UiFieldValue::Text {
                value: field.default.clone(),
            },
            Self::Number(field) => UiFieldValue::Number {
                value: field.default,
            },
            Self::Select(field) => UiFieldValue::Select {
                value: field.default.clone(),
            },
            Self::Checkbox(field) => UiFieldValue::Checkbox {
                value: field.default,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextField {
    pub id: String,
    pub label: String,
    pub default: String,
}

#[derive(Debug, Clone)]
pub struct NumberField {
    pub id: String,
    pub label: String,
    pub default: f64,
}

#[derive(Debug, Clone)]
pub struct SelectField {
    pub id: String,
    pub label: String,
    pub options: Vec<String>,
    pub default: String,
}

#[derive(Debug, Clone)]
pub struct CheckboxField {
    pub id: String,
    pub label: String,
    pub default: bool,
}

pub trait SchemaRegistry {
    fn supports_component(&self, kind: &ComponentKind) -> bool;
    fn supports_field_kind(&self, kind: &FormFieldKind) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    UnknownComponent {
        component_id: String,
        kind: String,
    },
    UnsupportedFieldType {
        form_id: String,
        field_id: String,
        kind: String,
    },
    MissingRequiredField {
        component_id: String,
        field: &'static str,
    },
    TooManyComponents {
        max: usize,
        actual: usize,
    },
    NestingTooDeep {
        max: usize,
        actual: usize,
        component_id: String,
    },
    DuplicateActionableId {
        component_id: String,
    },
    MissingButtonOutputContract {
        button_id: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownComponent { component_id, kind } => {
                write!(
                    f,
                    "unknown component kind `{kind}` for component `{component_id}`"
                )
            }
            Self::UnsupportedFieldType {
                form_id,
                field_id,
                kind,
            } => {
                write!(
                    f,
                    "unsupported field kind `{kind}` for form `{form_id}` field `{field_id}`"
                )
            }
            Self::MissingRequiredField {
                component_id,
                field,
            } => {
                write!(
                    f,
                    "missing required field `{field}` for component `{component_id}`"
                )
            }
            Self::TooManyComponents { max, actual } => {
                write!(f, "component count {actual} exceeds max {max}")
            }
            Self::NestingTooDeep {
                max,
                actual,
                component_id,
            } => {
                write!(
                    f,
                    "component `{component_id}` nesting depth {actual} exceeds max {max}"
                )
            }
            Self::DuplicateActionableId { component_id } => {
                write!(f, "duplicate actionable component id `{component_id}`")
            }
            Self::MissingButtonOutputContract { button_id } => {
                write!(f, "button `{button_id}` missing output contract mapping")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

fn as_string_or_default(value: &Value, default: &str) -> String {
    value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| default.to_string())
}

fn as_f64_or_default(value: &Value, default: f64) -> f64 {
    value.as_f64().unwrap_or(default)
}

fn as_bool_or_default(value: &Value, default: bool) -> bool {
    value.as_bool().unwrap_or(default)
}

pub fn field_key(form_id: &str, field_id: &str) -> String {
    format!("{form_id}:{field_id}")
}

pub fn validate_schema<R: SchemaRegistry>(
    schema: &UiSchema,
    registry: &R,
) -> Result<ValidatedSchema, ValidationError> {
    let output_map: BTreeMap<String, String> = schema
        .outputs
        .iter()
        .map(|output| (output.component_id.clone(), output.event_id.clone()))
        .collect();
    let mut component_counter: usize = 0;
    let mut actionable_ids = BTreeSet::new();

    let components = validate_components(
        &schema.components,
        registry,
        &output_map,
        1,
        &mut component_counter,
        &mut actionable_ids,
    )?;

    Ok(ValidatedSchema {
        schema_version: schema.schema_version,
        components,
    })
}

fn validate_components<R: SchemaRegistry>(
    raw_components: &[RawComponent],
    registry: &R,
    output_map: &BTreeMap<String, String>,
    depth: usize,
    component_counter: &mut usize,
    actionable_ids: &mut BTreeSet<String>,
) -> Result<Vec<ValidatedComponent>, ValidationError> {
    let mut validated = Vec::with_capacity(raw_components.len());

    for raw in raw_components {
        *component_counter += 1;
        if *component_counter > MAX_COMPONENTS {
            return Err(ValidationError::TooManyComponents {
                max: MAX_COMPONENTS,
                actual: *component_counter,
            });
        }

        if depth > MAX_DEPTH {
            return Err(ValidationError::NestingTooDeep {
                max: MAX_DEPTH,
                actual: depth,
                component_id: raw.id.clone(),
            });
        }

        if matches!(&raw.kind, ComponentKind::Unknown(_)) || !registry.supports_component(&raw.kind)
        {
            return Err(ValidationError::UnknownComponent {
                component_id: raw.id.clone(),
                kind: raw.kind.as_str().to_string(),
            });
        }

        if raw.kind.is_actionable() && !actionable_ids.insert(raw.id.clone()) {
            return Err(ValidationError::DuplicateActionableId {
                component_id: raw.id.clone(),
            });
        }

        let children = validate_components(
            &raw.children,
            registry,
            output_map,
            depth + 1,
            component_counter,
            actionable_ids,
        )?;

        let component = match &raw.kind {
            ComponentKind::Markdown => ValidatedComponent::Markdown(MarkdownComponent {
                id: raw.id.clone(),
                text: raw
                    .text
                    .clone()
                    .ok_or(ValidationError::MissingRequiredField {
                        component_id: raw.id.clone(),
                        field: "text",
                    })?,
                children,
            }),
            ComponentKind::Form => {
                let fields = validate_form_fields(&raw.id, &raw.fields, registry)?;
                ValidatedComponent::Form(FormComponent {
                    id: raw.id.clone(),
                    title: raw.title.clone(),
                    fields,
                    children,
                })
            }
            ComponentKind::Code => ValidatedComponent::Code(CodeComponent {
                id: raw.id.clone(),
                language: raw.language.clone(),
                code: raw
                    .code
                    .clone()
                    .ok_or(ValidationError::MissingRequiredField {
                        component_id: raw.id.clone(),
                        field: "code",
                    })?,
                children,
            }),
            ComponentKind::Diff => ValidatedComponent::Diff(DiffComponent {
                id: raw.id.clone(),
                lines: raw.lines.clone(),
                children,
            }),
            ComponentKind::Button => {
                let output_event_id = output_map.get(&raw.id).cloned().ok_or(
                    ValidationError::MissingButtonOutputContract {
                        button_id: raw.id.clone(),
                    },
                )?;
                ValidatedComponent::Button(ButtonComponent {
                    id: raw.id.clone(),
                    label: raw
                        .label
                        .clone()
                        .ok_or(ValidationError::MissingRequiredField {
                            component_id: raw.id.clone(),
                            field: "label",
                        })?,
                    output_event_id,
                    variant: raw.variant.clone().unwrap_or(ButtonStyle::Secondary),
                    children,
                })
            }
            ComponentKind::Unknown(kind) => {
                return Err(ValidationError::UnknownComponent {
                    component_id: raw.id.clone(),
                    kind: kind.clone(),
                });
            }
        };

        validated.push(component);
    }

    Ok(validated)
}

fn validate_form_fields<R: SchemaRegistry>(
    form_id: &str,
    raw_fields: &[RawFormField],
    registry: &R,
) -> Result<Vec<ValidatedFormField>, ValidationError> {
    let mut fields = Vec::with_capacity(raw_fields.len());
    for field in raw_fields {
        if matches!(&field.kind, FormFieldKind::Unknown(_))
            || !registry.supports_field_kind(&field.kind)
        {
            return Err(ValidationError::UnsupportedFieldType {
                form_id: form_id.to_string(),
                field_id: field.id.clone(),
                kind: field.kind.as_str().to_string(),
            });
        }

        let validated = match &field.kind {
            FormFieldKind::Text => ValidatedFormField::Text(TextField {
                id: field.id.clone(),
                label: field.label.clone(),
                default: as_string_or_default(&field.default, ""),
            }),
            FormFieldKind::Number => ValidatedFormField::Number(NumberField {
                id: field.id.clone(),
                label: field.label.clone(),
                default: as_f64_or_default(&field.default, 0.0),
            }),
            FormFieldKind::Select => {
                let default = as_string_or_default(
                    &field.default,
                    field
                        .options
                        .first()
                        .map(|option| option.as_str())
                        .unwrap_or(""),
                );
                ValidatedFormField::Select(SelectField {
                    id: field.id.clone(),
                    label: field.label.clone(),
                    options: field.options.clone(),
                    default,
                })
            }
            FormFieldKind::Checkbox => ValidatedFormField::Checkbox(CheckboxField {
                id: field.id.clone(),
                label: field.label.clone(),
                default: as_bool_or_default(&field.default, false),
            }),
            FormFieldKind::Unknown(kind) => {
                return Err(ValidationError::UnsupportedFieldType {
                    form_id: form_id.to_string(),
                    field_id: field.id.clone(),
                    kind: kind.clone(),
                });
            }
        };

        fields.push(validated);
    }

    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::registry::ComponentRegistry;

    fn validate(json: &str) -> Result<ValidatedSchema, ValidationError> {
        let schema: UiSchema = serde_json::from_str(json).expect("schema should deserialize");
        let registry = ComponentRegistry::new();
        validate_schema(&schema, &registry)
    }

    #[test]
    fn valid_schema_passes() {
        let schema = include_str!("fixture.json");
        assert!(validate(schema).is_ok());
    }

    #[test]
    fn unknown_component_fails_validation() {
        let schema = r#"{
          "schema_version": 1,
          "outputs": [],
          "components": [{"id":"x","kind":"unknown_widget"}]
        }"#;
        assert!(matches!(
            validate(schema),
            Err(ValidationError::UnknownComponent { .. })
        ));
    }

    #[test]
    fn unsupported_field_type_fails_validation() {
        let schema = r#"{
          "schema_version": 1,
          "outputs": [],
          "components": [{
            "id":"f1",
            "kind":"form",
            "fields":[{"id":"a","label":"A","kind":"slider"}]
          }]
        }"#;
        assert!(matches!(
            validate(schema),
            Err(ValidationError::UnsupportedFieldType { .. })
        ));
    }

    #[test]
    fn component_count_limit_enforced() {
        let mut components = Vec::new();
        for i in 0..(MAX_COMPONENTS + 1) {
            components.push(serde_json::json!({
                "id": format!("m{i}"),
                "kind": "markdown",
                "text": "x"
            }));
        }
        let schema = serde_json::json!({
            "schema_version": 1,
            "outputs": [],
            "components": components
        });
        assert!(matches!(
            validate(&schema.to_string()),
            Err(ValidationError::TooManyComponents { .. })
        ));
    }

    #[test]
    fn nesting_depth_limit_enforced() {
        let schema = r#"{
          "schema_version": 1,
          "outputs": [],
          "components": [{
            "id":"l1","kind":"markdown","text":"a",
            "children":[{
              "id":"l2","kind":"markdown","text":"b",
              "children":[{
                "id":"l3","kind":"markdown","text":"c",
                "children":[{
                  "id":"l4","kind":"markdown","text":"d",
                  "children":[{"id":"l5","kind":"markdown","text":"e"}]
                }]
              }]
            }]
          }]
        }"#;
        assert!(matches!(
            validate(schema),
            Err(ValidationError::NestingTooDeep { .. })
        ));
    }

    #[test]
    fn missing_button_output_contract_fails_validation() {
        let schema = r#"{
          "schema_version": 1,
          "outputs": [],
          "components": [{"id":"b1","kind":"button","label":"Go"}]
        }"#;
        assert!(matches!(
            validate(schema),
            Err(ValidationError::MissingButtonOutputContract { .. })
        ));
    }
}

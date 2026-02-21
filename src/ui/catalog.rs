use crate::ui::registry::ComponentRegistry;
use crate::ui::schema::{validate_schema, UiSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::PathBuf;

const BUILTIN_CODE_REVIEW_TEMPLATE: &str = include_str!("catalog_builtin/code_review.json");
const BUILTIN_PLAN_REVIEW_TEMPLATE: &str = include_str!("catalog_builtin/plan_review.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiIntent {
    pub primary: String,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl UiIntent {
    pub fn new(primary: impl Into<String>, operations: Vec<String>, tags: Vec<String>) -> Self {
        Self {
            primary: primary.into(),
            operations: normalize_terms(&operations),
            tags: normalize_terms(&tags),
        }
    }

    pub fn summary(&self) -> String {
        let operations = if self.operations.is_empty() {
            "-".to_string()
        } else {
            self.operations.join(",")
        };
        let tags = if self.tags.is_empty() {
            "-".to_string()
        } else {
            self.tags.join(",")
        };
        format!(
            "primary={} ops={} tags={}",
            self.primary.trim(),
            operations,
            tags
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateMeta {
    pub id: String,
    pub title: String,
    pub version: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateMatch {
    pub primary: String,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDocument {
    pub meta: TemplateMeta,
    #[serde(rename = "match")]
    pub match_rules: TemplateMatch,
    pub schema: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSourceKind {
    Org,
    User,
    Builtin,
}

impl CatalogSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Org => "org",
            Self::User => "user",
            Self::Builtin => "builtin",
        }
    }
}

impl fmt::Display for CatalogSourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSource {
    pub provider_id: String,
    pub kind: CatalogSourceKind,
    pub read_only: bool,
}

#[derive(Debug, Clone)]
pub struct CatalogTemplate {
    pub document: TemplateDocument,
    pub source: CatalogSource,
}

impl CatalogTemplate {
    pub fn template_id(&self) -> &str {
        self.document.meta.id.as_str()
    }

    pub fn schema_value(&self) -> &Value {
        &self.document.schema
    }
}

#[derive(Debug, Clone)]
pub struct CatalogLoadOutput {
    pub templates: Vec<CatalogTemplate>,
    pub diagnostics: Vec<CatalogLoadDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogLoadDiagnostic {
    pub provider_id: String,
    pub template_ref: String,
    pub reason: String,
}

impl CatalogLoadDiagnostic {
    pub fn to_log_line(&self) -> String {
        format!(
            "catalog load rejected provider={} template_ref={} reason={}",
            self.provider_id, self.template_ref, self.reason
        )
    }
}

#[derive(Debug, Clone)]
pub enum CatalogError {
    ReadOnlyProvider { provider_id: String },
    Io {
        provider_id: String,
        path: PathBuf,
        message: String,
    },
    Serialize(String),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadOnlyProvider { provider_id } => {
                write!(f, "provider {provider_id} is read-only")
            }
            Self::Io {
                provider_id,
                path,
                message,
            } => write!(
                f,
                "provider {provider_id} io error at {}: {message}",
                path.display()
            ),
            Self::Serialize(message) => write!(f, "template serialization error: {message}"),
        }
    }
}

impl std::error::Error for CatalogError {}

pub trait CatalogProvider: Send + Sync {
    fn source(&self) -> CatalogSource;

    fn load_templates(&self) -> Result<CatalogLoadOutput, CatalogError>;

    fn upsert_template(&self, _template: &TemplateDocument) -> Result<(), CatalogError> {
        Err(CatalogError::ReadOnlyProvider {
            provider_id: self.source().provider_id,
        })
    }

    fn delete_template(&self, _template_id: &str) -> Result<(), CatalogError> {
        Err(CatalogError::ReadOnlyProvider {
            provider_id: self.source().provider_id,
        })
    }
}

pub struct BuiltinCatalogProvider {
    source: CatalogSource,
    embedded_templates: Vec<&'static str>,
}

impl BuiltinCatalogProvider {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            source: CatalogSource {
                provider_id: provider_id.into(),
                kind: CatalogSourceKind::Builtin,
                read_only: true,
            },
            embedded_templates: vec![BUILTIN_CODE_REVIEW_TEMPLATE, BUILTIN_PLAN_REVIEW_TEMPLATE],
        }
    }
}

impl Default for BuiltinCatalogProvider {
    fn default() -> Self {
        Self::new("builtin-default")
    }
}

impl CatalogProvider for BuiltinCatalogProvider {
    fn source(&self) -> CatalogSource {
        self.source.clone()
    }

    fn load_templates(&self) -> Result<CatalogLoadOutput, CatalogError> {
        let mut output = CatalogLoadOutput {
            templates: Vec::new(),
            diagnostics: Vec::new(),
        };

        for (index, raw_template) in self.embedded_templates.iter().enumerate() {
            let template_ref = format!("embedded:{index}");
            match parse_and_validate_template(raw_template, &self.source, &template_ref) {
                Ok(template) => output.templates.push(template),
                Err(reason) => output.diagnostics.push(CatalogLoadDiagnostic {
                    provider_id: self.source.provider_id.clone(),
                    template_ref,
                    reason,
                }),
            }
        }

        Ok(output)
    }
}

pub struct UserCatalogProvider {
    source: CatalogSource,
    root_dir: PathBuf,
}

impl UserCatalogProvider {
    pub fn new(provider_id: impl Into<String>, root_dir: impl Into<PathBuf>) -> Self {
        Self {
            source: CatalogSource {
                provider_id: provider_id.into(),
                kind: CatalogSourceKind::User,
                read_only: false,
            },
            root_dir: root_dir.into(),
        }
    }

    fn template_path_for_id(&self, template_id: &str) -> PathBuf {
        self.root_dir
            .join(format!("{}.json", sanitize_filename(template_id)))
    }
}

impl CatalogProvider for UserCatalogProvider {
    fn source(&self) -> CatalogSource {
        self.source.clone()
    }

    fn load_templates(&self) -> Result<CatalogLoadOutput, CatalogError> {
        if !self.root_dir.exists() {
            return Ok(CatalogLoadOutput {
                templates: Vec::new(),
                diagnostics: Vec::new(),
            });
        }

        let mut entries = fs::read_dir(&self.root_dir).map_err(|err| CatalogError::Io {
            provider_id: self.source.provider_id.clone(),
            path: self.root_dir.clone(),
            message: err.to_string(),
        })?;

        let mut paths = Vec::new();
        while let Some(entry) = entries.next().transpose().map_err(|err| CatalogError::Io {
            provider_id: self.source.provider_id.clone(),
            path: self.root_dir.clone(),
            message: err.to_string(),
        })? {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                paths.push(path);
            }
        }
        paths.sort();

        let mut output = CatalogLoadOutput {
            templates: Vec::new(),
            diagnostics: Vec::new(),
        };

        for path in paths {
            let template_ref = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();

            let raw_template = fs::read_to_string(&path).map_err(|err| CatalogError::Io {
                provider_id: self.source.provider_id.clone(),
                path: path.clone(),
                message: err.to_string(),
            })?;

            match parse_and_validate_template(&raw_template, &self.source, &template_ref) {
                Ok(template) => output.templates.push(template),
                Err(reason) => output.diagnostics.push(CatalogLoadDiagnostic {
                    provider_id: self.source.provider_id.clone(),
                    template_ref,
                    reason,
                }),
            }
        }

        Ok(output)
    }

    fn upsert_template(&self, template: &TemplateDocument) -> Result<(), CatalogError> {
        let raw = serde_json::to_string_pretty(template)
            .map_err(|err| CatalogError::Serialize(err.to_string()))?;
        let template_path = self.template_path_for_id(&template.meta.id);

        if let Some(parent) = template_path.parent() {
            fs::create_dir_all(parent).map_err(|err| CatalogError::Io {
                provider_id: self.source.provider_id.clone(),
                path: parent.to_path_buf(),
                message: err.to_string(),
            })?;
        }

        fs::write(&template_path, raw).map_err(|err| CatalogError::Io {
            provider_id: self.source.provider_id.clone(),
            path: template_path,
            message: err.to_string(),
        })
    }

    fn delete_template(&self, template_id: &str) -> Result<(), CatalogError> {
        let template_path = self.template_path_for_id(template_id);
        if !template_path.exists() {
            return Ok(());
        }

        fs::remove_file(&template_path).map_err(|err| CatalogError::Io {
            provider_id: self.source.provider_id.clone(),
            path: template_path,
            message: err.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionCandidate {
    pub template_id: String,
    pub provider_id: String,
    pub provider_kind: CatalogSourceKind,
    pub score: i32,
    pub operation_overlap: usize,
    pub tag_overlap: usize,
    pub excluded_reason: Option<String>,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionTrace {
    pub intent: UiIntent,
    pub provider_precedence: Vec<CatalogSourceKind>,
    pub selected_template_id: Option<String>,
    pub selected_provider_id: Option<String>,
    pub selected_score: Option<i32>,
    pub ranked_candidates: Vec<ResolutionCandidate>,
    pub no_match_reasons: Vec<String>,
}

impl ResolutionTrace {
    pub fn diagnostic_lines(&self) -> Vec<String> {
        if let Some(template_id) = &self.selected_template_id {
            let provider = self
                .selected_provider_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            let score = self.selected_score.unwrap_or_default();
            return vec![format!(
                "catalog resolve selected template={} provider={} score={} intent={}",
                template_id,
                provider,
                score,
                self.intent.summary()
            )];
        }

        vec![format!(
            "catalog resolve no_match intent={} reasons={}",
            self.intent.summary(),
            if self.no_match_reasons.is_empty() {
                "none".to_string()
            } else {
                self.no_match_reasons.join(" | ")
            }
        )]
    }
}

#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub selected: Option<CatalogTemplate>,
    pub trace: ResolutionTrace,
}

pub struct CatalogManager {
    providers: Vec<Box<dyn CatalogProvider>>,
    templates: Vec<CatalogTemplate>,
    load_diagnostics: Vec<CatalogLoadDiagnostic>,
    org_enabled: bool,
}

impl CatalogManager {
    pub fn new(providers: Vec<Box<dyn CatalogProvider>>, org_enabled: bool) -> Self {
        let mut manager = Self {
            providers,
            templates: Vec::new(),
            load_diagnostics: Vec::new(),
            org_enabled,
        };
        manager.reload();
        manager
    }

    pub fn with_default_providers(user_catalog_dir: impl Into<PathBuf>, org_enabled: bool) -> Self {
        let providers: Vec<Box<dyn CatalogProvider>> = vec![
            Box::new(UserCatalogProvider::new("user-local", user_catalog_dir.into())),
            Box::new(BuiltinCatalogProvider::default()),
        ];
        Self::new(providers, org_enabled)
    }

    pub fn reload(&mut self) {
        self.templates.clear();
        self.load_diagnostics.clear();

        for provider in &self.providers {
            match provider.load_templates() {
                Ok(output) => {
                    self.templates.extend(output.templates);
                    self.load_diagnostics.extend(output.diagnostics);
                }
                Err(err) => {
                    let source = provider.source();
                    self.load_diagnostics.push(CatalogLoadDiagnostic {
                        provider_id: source.provider_id,
                        template_ref: "provider".to_string(),
                        reason: err.to_string(),
                    });
                }
            }
        }

        self.templates.sort_by(|left, right| {
            left.source
                .provider_id
                .cmp(&right.source.provider_id)
                .then_with(|| left.template_id().cmp(right.template_id()))
        });
    }

    pub fn load_diagnostics(&self) -> &[CatalogLoadDiagnostic] {
        &self.load_diagnostics
    }

    pub fn resolve(&self, intent: &UiIntent) -> ResolutionResult {
        let precedence = self.precedence();
        let mut ranked_candidates = Vec::new();

        let mut matches_by_tier: BTreeMap<usize, Vec<ResolutionCandidate>> = BTreeMap::new();

        for template in &self.templates {
            let Some(tier_index) = precedence
                .iter()
                .position(|kind| *kind == template.source.kind)
            else {
                continue;
            };

            let required_primary = template.document.match_rules.primary.trim();
            let intent_primary = intent.primary.trim();
            if required_primary != intent_primary {
                ranked_candidates.push(ResolutionCandidate {
                    template_id: template.template_id().to_string(),
                    provider_id: template.source.provider_id.clone(),
                    provider_kind: template.source.kind,
                    score: 0,
                    operation_overlap: 0,
                    tag_overlap: 0,
                    excluded_reason: Some(format!(
                        "primary mismatch expected={} actual={}",
                        required_primary, intent_primary
                    )),
                    selected: false,
                });
                continue;
            }

            let score = score_secondary(intent, template);
            let candidate = ResolutionCandidate {
                template_id: template.template_id().to_string(),
                provider_id: template.source.provider_id.clone(),
                provider_kind: template.source.kind,
                score: score.total,
                operation_overlap: score.operation_overlap,
                tag_overlap: score.tag_overlap,
                excluded_reason: None,
                selected: false,
            };
            matches_by_tier
                .entry(tier_index)
                .or_default()
                .push(candidate.clone());
            ranked_candidates.push(candidate);
        }

        let mut selected: Option<CatalogTemplate> = None;
        let mut selected_tier_index: Option<usize> = None;
        let mut selected_candidate_key: Option<(String, String)> = None;
        for (tier_index, _) in precedence.iter().enumerate() {
            let Some(tier_candidates) = matches_by_tier.get(&tier_index) else {
                continue;
            };
            let mut sorted = tier_candidates.clone();
            sorted.sort_by(rank_candidates);
            if let Some(best) = sorted.first() {
                selected_tier_index = Some(tier_index);
                selected_candidate_key =
                    Some((best.template_id.clone(), best.provider_id.clone()));
                selected = self
                    .templates
                    .iter()
                    .find(|template| {
                        template.template_id() == best.template_id
                            && template.source.provider_id == best.provider_id
                    })
                    .cloned();
                break;
            }
        }

        if let Some((selected_template_id, selected_provider_id)) = &selected_candidate_key {
            for candidate in &mut ranked_candidates {
                if &candidate.template_id == selected_template_id
                    && &candidate.provider_id == selected_provider_id
                {
                    candidate.selected = true;
                    continue;
                }

                if candidate.excluded_reason.is_none() {
                    let selected_tier = selected_tier_index.unwrap_or(usize::MAX);
                    let candidate_tier = precedence
                        .iter()
                        .position(|kind| *kind == candidate.provider_kind)
                        .unwrap_or(usize::MAX);

                    candidate.excluded_reason = Some(if candidate_tier > selected_tier {
                        format!(
                            "lower provider precedence than {}",
                            precedence[selected_tier].as_str()
                        )
                    } else {
                        "lower score or tie-break in same tier".to_string()
                    });
                }
            }
        }

        ranked_candidates.sort_by(|left, right| {
            rank_candidates(left, right).then_with(|| {
                precedence_index(left.provider_kind, &precedence)
                    .cmp(&precedence_index(right.provider_kind, &precedence))
            })
        });

        let selected_template_id = selected
            .as_ref()
            .map(|template| template.template_id().to_string());
        let selected_provider_id = selected
            .as_ref()
            .map(|template| template.source.provider_id.clone());
        let selected_score = ranked_candidates
            .iter()
            .find(|candidate| candidate.selected)
            .map(|candidate| candidate.score);

        let no_match_reasons = if selected.is_none() {
            if ranked_candidates.is_empty() {
                vec!["catalog index contains no templates".to_string()]
            } else {
                ranked_candidates
                    .iter()
                    .map(|candidate| {
                        let reason = candidate
                            .excluded_reason
                            .clone()
                            .unwrap_or_else(|| "no ranking winner".to_string());
                        format!(
                            "{}:{} {}",
                            candidate.provider_id, candidate.template_id, reason
                        )
                    })
                    .collect()
            }
        } else {
            Vec::new()
        };

        ResolutionResult {
            selected,
            trace: ResolutionTrace {
                intent: intent.clone(),
                provider_precedence: precedence,
                selected_template_id,
                selected_provider_id,
                selected_score,
                ranked_candidates,
                no_match_reasons,
            },
        }
    }

    fn precedence(&self) -> Vec<CatalogSourceKind> {
        if self.org_enabled {
            vec![
                CatalogSourceKind::Org,
                CatalogSourceKind::User,
                CatalogSourceKind::Builtin,
            ]
        } else {
            vec![CatalogSourceKind::User, CatalogSourceKind::Builtin]
        }
    }
}

fn parse_and_validate_template(
    raw_template: &str,
    source: &CatalogSource,
    template_ref: &str,
) -> Result<CatalogTemplate, String> {
    let mut document: TemplateDocument = serde_json::from_str(raw_template)
        .map_err(|err| format!("template parse failed ({template_ref}): {err}"))?;

    normalize_document(&mut document);

    if document.meta.id.trim().is_empty() {
        return Err("meta.id is required".to_string());
    }
    if document.meta.title.trim().is_empty() {
        return Err("meta.title is required".to_string());
    }
    if document.meta.version.trim().is_empty() {
        return Err("meta.version is required".to_string());
    }
    if document.match_rules.primary.trim().is_empty() {
        return Err("match.primary is required".to_string());
    }

    let ui_schema: UiSchema = serde_json::from_value(document.schema.clone())
        .map_err(|err| format!("schema deserialize error: {err}"))?;
    let registry = ComponentRegistry::new();
    validate_schema(&ui_schema, &registry).map_err(|err| format!("schema validation error: {err}"))?;

    Ok(CatalogTemplate {
        document,
        source: source.clone(),
    })
}

fn normalize_document(document: &mut TemplateDocument) {
    document.meta.id = document.meta.id.trim().to_string();
    document.meta.title = document.meta.title.trim().to_string();
    document.meta.version = document.meta.version.trim().to_string();
    document.meta.tags = normalize_terms(&document.meta.tags);

    document.match_rules.primary = document.match_rules.primary.trim().to_string();
    document.match_rules.operations = normalize_terms(&document.match_rules.operations);
    document.match_rules.tags = normalize_terms(&document.match_rules.tags);
}

fn normalize_terms(terms: &[String]) -> Vec<String> {
    let mut deduped = BTreeSet::new();
    for term in terms {
        let trimmed = term.trim();
        if !trimmed.is_empty() {
            deduped.insert(trimmed.to_string());
        }
    }
    deduped.into_iter().collect()
}

#[derive(Debug, Clone, Copy)]
struct SecondaryScore {
    total: i32,
    operation_overlap: usize,
    tag_overlap: usize,
}

fn score_secondary(intent: &UiIntent, template: &CatalogTemplate) -> SecondaryScore {
    let intent_operations: BTreeSet<&str> = intent.operations.iter().map(|value| value.as_str()).collect();
    let intent_tags: BTreeSet<&str> = intent.tags.iter().map(|value| value.as_str()).collect();

    let template_operations: BTreeSet<&str> = template
        .document
        .match_rules
        .operations
        .iter()
        .map(|value| value.as_str())
        .collect();
    let template_tags: BTreeSet<&str> = template
        .document
        .match_rules
        .tags
        .iter()
        .map(|value| value.as_str())
        .collect();

    let operation_overlap = template_operations.intersection(&intent_operations).count();
    let tag_overlap = template_tags.intersection(&intent_tags).count();

    let exact_operation_bonus = if !template_operations.is_empty()
        && template_operations == intent_operations
    {
        2
    } else {
        0
    };
    let exact_tag_bonus = if !template_tags.is_empty() && template_tags == intent_tags {
        1
    } else {
        0
    };

    SecondaryScore {
        total: (operation_overlap as i32 * 10)
            + (tag_overlap as i32 * 4)
            + exact_operation_bonus
            + exact_tag_bonus,
        operation_overlap,
        tag_overlap,
    }
}

fn rank_candidates(left: &ResolutionCandidate, right: &ResolutionCandidate) -> Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| left.template_id.cmp(&right.template_id))
        .then_with(|| left.provider_id.cmp(&right.provider_id))
}

fn precedence_index(kind: CatalogSourceKind, precedence: &[CatalogSourceKind]) -> usize {
    precedence
        .iter()
        .position(|precedence_kind| *precedence_kind == kind)
        .unwrap_or(usize::MAX)
}

fn sanitize_filename(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
            output.push(ch);
        } else {
            output.push('_');
        }
    }

    if output.trim_matches('_').is_empty() {
        "template".to_string()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::runtime::UiRuntime;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct MemoryCatalogProvider {
        source: CatalogSource,
        templates: Vec<String>,
    }

    impl MemoryCatalogProvider {
        fn new(kind: CatalogSourceKind, provider_id: &str, templates: Vec<String>) -> Self {
            Self {
                source: CatalogSource {
                    provider_id: provider_id.to_string(),
                    kind,
                    read_only: true,
                },
                templates,
            }
        }
    }

    impl CatalogProvider for MemoryCatalogProvider {
        fn source(&self) -> CatalogSource {
            self.source.clone()
        }

        fn load_templates(&self) -> Result<CatalogLoadOutput, CatalogError> {
            let mut output = CatalogLoadOutput {
                templates: Vec::new(),
                diagnostics: Vec::new(),
            };

            for (index, template) in self.templates.iter().enumerate() {
                match parse_and_validate_template(template, &self.source, &format!("mem:{index}")) {
                    Ok(parsed) => output.templates.push(parsed),
                    Err(reason) => output.diagnostics.push(CatalogLoadDiagnostic {
                        provider_id: self.source.provider_id.clone(),
                        template_ref: format!("mem:{index}"),
                        reason,
                    }),
                }
            }

            Ok(output)
        }
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("brownie_{prefix}_{}_{}", std::process::id(), nanos))
    }

    fn sample_template_json(
        template_id: &str,
        primary: &str,
        operations: &[&str],
        tags: &[&str],
    ) -> String {
        let operations = operations
            .iter()
            .map(|op| format!("\"{op}\""))
            .collect::<Vec<_>>()
            .join(",");
        let tags = tags
            .iter()
            .map(|tag| format!("\"{tag}\""))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"{{
  "meta": {{
    "id": "{template_id}",
    "title": "Template {template_id}",
    "version": "1.0.0",
    "tags": [{tags}]
  }},
  "match": {{
    "primary": "{primary}",
    "operations": [{operations}],
    "tags": [{tags}]
  }},
  "schema": {{
    "schema_version": 1,
    "outputs": [
      {{
        "component_id": "submit_{template_id}",
        "event_id": "event.{template_id}"
      }}
    ],
    "components": [
      {{
        "id": "note_{template_id}",
        "kind": "markdown",
        "text": "{template_id}"
      }},
      {{
        "id": "submit_{template_id}",
        "kind": "button",
        "label": "Submit",
        "variant": "primary"
      }}
    ]
  }}
}}"#
        )
    }

    #[test]
    fn builtin_provider_loads_embedded_templates() {
        let provider = BuiltinCatalogProvider::default();
        let loaded = provider.load_templates().expect("builtin load should succeed");
        assert!(loaded.diagnostics.is_empty());
        assert!(loaded.templates.len() >= 2);
    }

    #[test]
    fn builtin_provider_rejects_mutation_attempts() {
        let provider = BuiltinCatalogProvider::default();
        let template = serde_json::from_str::<TemplateDocument>(BUILTIN_PLAN_REVIEW_TEMPLATE)
            .expect("embedded template should deserialize");

        assert!(matches!(
            provider.upsert_template(&template),
            Err(CatalogError::ReadOnlyProvider { .. })
        ));
        assert!(matches!(
            provider.delete_template("builtin.plan_review.default"),
            Err(CatalogError::ReadOnlyProvider { .. })
        ));
    }

    #[test]
    fn user_provider_persists_and_reloads_templates() {
        let root = temp_dir("catalog_user_persist");
        let provider = UserCatalogProvider::new("user-test", root.clone());

        let template: TemplateDocument = serde_json::from_str(&sample_template_json(
            "user.template.alpha",
            "code_review",
            &["approve"],
            &["spec"],
        ))
        .expect("template should deserialize");

        provider
            .upsert_template(&template)
            .expect("upsert should persist template");

        let loaded = provider.load_templates().expect("load should succeed");
        assert_eq!(loaded.templates.len(), 1);
        assert_eq!(loaded.templates[0].template_id(), "user.template.alpha");

        provider
            .delete_template("user.template.alpha")
            .expect("delete should succeed");
        let loaded_after_delete = provider
            .load_templates()
            .expect("reload should succeed after delete");
        assert!(loaded_after_delete.templates.is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_templates_are_excluded_with_diagnostics() {
        let root = temp_dir("catalog_invalid");
        fs::create_dir_all(&root).expect("temp dir should be created");

        let invalid = r#"{
  "meta": {"id": "", "title": "Broken", "version": "1.0.0"},
  "match": {"primary": "code_review"},
  "schema": {"schema_version": 1, "outputs": [], "components": []}
}"#;
        fs::write(root.join("broken.json"), invalid).expect("invalid template should be written");

        let provider = UserCatalogProvider::new("user-invalid", root.clone());
        let loaded = provider.load_templates().expect("load should succeed");
        assert!(loaded.templates.is_empty());
        assert_eq!(loaded.diagnostics.len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolver_prefers_user_over_builtin_when_org_disabled() {
        let user_template = sample_template_json(
            "user.code_review",
            "code_review",
            &["approve", "reject"],
            &["spec"],
        );
        let providers: Vec<Box<dyn CatalogProvider>> = vec![
            Box::new(MemoryCatalogProvider::new(
                CatalogSourceKind::User,
                "user",
                vec![user_template],
            )),
            Box::new(BuiltinCatalogProvider::default()),
        ];

        let manager = CatalogManager::new(providers, false);
        let intent = UiIntent::new(
            "code_review",
            vec!["approve".to_string(), "reject".to_string()],
            vec!["spec".to_string()],
        );
        let result = manager.resolve(&intent);

        let selected = result
            .selected
            .expect("expected selection when user and builtin both match");
        assert_eq!(selected.source.kind, CatalogSourceKind::User);
        assert_eq!(selected.source.provider_id, "user");
    }

    #[test]
    fn resolver_prefers_org_over_user_and_builtin_when_enabled() {
        let org_template = sample_template_json(
            "org.code_review",
            "code_review",
            &["approve"],
            &["security"],
        );
        let user_template = sample_template_json(
            "user.code_review",
            "code_review",
            &["approve"],
            &["security"],
        );

        let providers: Vec<Box<dyn CatalogProvider>> = vec![
            Box::new(MemoryCatalogProvider::new(
                CatalogSourceKind::Org,
                "org",
                vec![org_template],
            )),
            Box::new(MemoryCatalogProvider::new(
                CatalogSourceKind::User,
                "user",
                vec![user_template],
            )),
            Box::new(BuiltinCatalogProvider::default()),
        ];

        let manager = CatalogManager::new(providers, true);
        let intent = UiIntent::new(
            "code_review",
            vec!["approve".to_string()],
            vec!["security".to_string()],
        );
        let result = manager.resolve(&intent);

        let selected = result.selected.expect("expected org template to win");
        assert_eq!(selected.source.kind, CatalogSourceKind::Org);
        assert_eq!(selected.source.provider_id, "org");
    }

    #[test]
    fn resolver_secondary_overlap_and_tie_breaking_are_deterministic() {
        let lower = sample_template_json(
            "user.code_review.a",
            "code_review",
            &["approve"],
            &["spec"],
        );
        let higher = sample_template_json(
            "user.code_review.b",
            "code_review",
            &["approve", "reject"],
            &["spec", "diff"],
        );

        let providers: Vec<Box<dyn CatalogProvider>> = vec![Box::new(MemoryCatalogProvider::new(
            CatalogSourceKind::User,
            "user",
            vec![lower, higher],
        ))];

        let manager = CatalogManager::new(providers, false);
        let intent = UiIntent::new(
            "code_review",
            vec!["approve".to_string(), "reject".to_string()],
            vec!["spec".to_string(), "diff".to_string()],
        );

        let first = manager.resolve(&intent);
        let second = manager.resolve(&intent);
        assert_eq!(
            first.trace.selected_template_id,
            second.trace.selected_template_id,
            "same input should produce stable winner"
        );
        assert_eq!(first.trace.ranked_candidates, second.trace.ranked_candidates);

        let winner = first.selected.expect("winner should exist");
        assert_eq!(winner.template_id(), "user.code_review.b");
    }

    #[test]
    fn resolver_returns_explicit_no_match_with_reasons() {
        let providers: Vec<Box<dyn CatalogProvider>> = vec![Box::new(BuiltinCatalogProvider::default())];
        let manager = CatalogManager::new(providers, false);
        let intent = UiIntent::new("unmatched_primary", Vec::new(), Vec::new());
        let result = manager.resolve(&intent);

        assert!(result.selected.is_none());
        assert!(result.trace.no_match_reasons.iter().any(|reason| {
            reason.contains("primary mismatch") || reason.contains("catalog index")
        }));
    }

    #[test]
    fn selected_template_schema_loads_into_runtime() {
        let providers: Vec<Box<dyn CatalogProvider>> = vec![Box::new(BuiltinCatalogProvider::default())];
        let manager = CatalogManager::new(providers, false);
        let intent = UiIntent::new(
            "code_review",
            vec!["approve".to_string(), "reject".to_string()],
            vec!["spec".to_string()],
        );
        let result = manager.resolve(&intent);
        let selected = result.selected.expect("a builtin template should match");

        let mut runtime = UiRuntime::new();
        runtime
            .load_schema_value(selected.schema_value())
            .expect("selected template schema should validate and load");
        assert!(runtime.has_schema());
        assert!(runtime.runtime_error().is_none());
    }
}

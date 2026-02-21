# intent-template-resolution

## ADDED Requirements

### Requirement: Primary intent field must match exactly

Template selection SHALL only consider templates whose `match.primary` value equals `UiIntent.primary` exactly.

#### Scenario: Exact primary match exists

- **WHEN** at least one template has `match.primary` equal to the incoming `UiIntent.primary`
- **THEN** those templates are considered candidate matches

#### Scenario: No exact primary match exists

- **WHEN** no template has `match.primary` equal to the incoming `UiIntent.primary`
- **THEN** resolution returns no-match and no template is selected

---

### Requirement: Secondary match fields refine deterministic ranking

For candidates passing primary match, the resolver SHALL apply deterministic ranking using secondary fields including `operations` overlap and contextual tag overlap.

#### Scenario: Candidate with stronger secondary overlap wins

- **WHEN** two templates share the same primary match and one template has greater overlap with intent operations or tags
- **THEN** the higher-overlap template receives a higher score and is selected

#### Scenario: Score tie resolves deterministically

- **WHEN** two candidates produce identical match scores
- **THEN** the resolver applies stable tie-breakers and returns the same winner across repeated runs

---

### Requirement: Agent declares intent only and cannot force template ids

The agent SHALL provide `UiIntent` only. The client resolver SHALL choose templates from catalog rules and MUST NOT accept direct template-id selection from the agent path.

#### Scenario: Agent payload includes only UiIntent

- **WHEN** the client receives an agent UI request in SPEC-3
- **THEN** template selection is performed by resolver rules over catalog entries, not by agent-chosen template id

---

### Requirement: Resolution is transparent and inspectable

Each resolution attempt SHALL emit structured diagnostics that explain selection outcome including selected template id, provider, score basis, or explicit no-match reasons.

#### Scenario: Successful selection is logged with reasoning

- **WHEN** a template is selected
- **THEN** diagnostics record the selected template id, provider source, and ranking rationale

#### Scenario: No-match is logged with reasons

- **WHEN** no template matches an intent
- **THEN** diagnostics record explicit no-match outcome and candidate exclusion reasons

---

### Requirement: Repeated identical intents yield identical outcomes

For identical intent inputs and unchanged catalog state, the resolver SHALL return the same selection result and rationale on every run.

#### Scenario: Deterministic repeated resolution

- **WHEN** the same `UiIntent` is resolved multiple times with unchanged provider content
- **THEN** the same template id or no-match outcome is returned every time


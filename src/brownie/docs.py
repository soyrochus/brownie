from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable


@dataclass
class DocContext:
    facts: list[dict]
    open_questions: list[str]
    api_applicable: bool
    ui_applicable: bool


def _facts_by_tag(facts: Iterable[dict], tag: str) -> list[dict]:
    results = []
    for fact in facts:
        if tag in fact.get("tags", []):
            results.append(fact)
    return results


def _format_evidence(fact: dict) -> str:
    evidence = fact.get("evidence", {})
    path = evidence.get("path", "unknown")
    start = evidence.get("line_start", "?")
    end = evidence.get("line_end", "?")
    return f"{path}:{start}-{end}"


def render_project_intent(ctx: DocContext) -> str:
    intent_facts = _facts_by_tag(ctx.facts, "intent")
    lines = ["# Project Intent & Business Frame", "", "## Observations"]
    if intent_facts:
        for fact in intent_facts:
            lines.append(f"- {fact['claim']} (evidence: {_format_evidence(fact)})")
    else:
        lines.append("- Insufficient evidence in included directories to infer intent.")
    lines.append("\n## Open Questions")
    if ctx.open_questions:
        for question in ctx.open_questions:
            lines.append(f"- {question}")
    else:
        lines.append("- None recorded.")
    return "\n".join(lines) + "\n"


def render_domain_landscape(ctx: DocContext) -> str:
    domain_facts = _facts_by_tag(ctx.facts, "domain")
    lines = ["# Domain Landscape", "", "## Signals"]
    if domain_facts:
        for fact in domain_facts:
            lines.append(f"- {fact['claim']} (evidence: {_format_evidence(fact)})")
    else:
        lines.append("- No domain-specific signals detected in included directories.")
    return "\n".join(lines) + "\n"


def render_canonical_data_model(ctx: DocContext) -> str:
    data_facts = _facts_by_tag(ctx.facts, "data-model")
    lines = ["# Canonical Data Model", "", "## Entities"]
    if data_facts:
        for fact in data_facts:
            lines.append(f"- {fact['claim']} (evidence: {_format_evidence(fact)})")
    else:
        lines.append("- No canonical entities inferred from bounded reads.")
    return "\n".join(lines) + "\n"


def render_service_capability_map(ctx: DocContext) -> str:
    service_facts = _facts_by_tag(ctx.facts, "service")
    lines = ["# Service & Capability Map", "", "## Services"]
    if service_facts:
        for fact in service_facts:
            lines.append(f"- {fact['claim']} (evidence: {_format_evidence(fact)})")
    else:
        lines.append("- No service boundaries detected in included directories.")
    return "\n".join(lines) + "\n"


def render_architectural_guardrails(ctx: DocContext) -> str:
    guardrail_facts = _facts_by_tag(ctx.facts, "guardrail")
    lines = ["# Architectural Guardrails", "", "## Constraints"]
    if guardrail_facts:
        for fact in guardrail_facts:
            lines.append(f"- {fact['claim']} (evidence: {_format_evidence(fact)})")
    else:
        lines.append("- No explicit guardrails detected in included directories.")
    return "\n".join(lines) + "\n"


def render_api_integration(ctx: DocContext) -> str:
    lines = ["# API / Integration Contracts", ""]
    api_facts = _facts_by_tag(ctx.facts, "api")
    if ctx.api_applicable:
        lines.append("## Evidence")
        for fact in api_facts:
            lines.append(f"- {fact['claim']} (evidence: {_format_evidence(fact)})")
        if not api_facts:
            lines.append("- API indicators detected, but no detailed contracts found in bounded reads.")
    else:
        lines.append("Not applicable based on bounded evidence in included directories.")
        lines.append("\nEvidence note: No API definitions or routing layers detected during analysis.")
    return "\n".join(lines) + "\n"


def render_user_journey(ctx: DocContext) -> str:
    lines = ["# User Journey & UI Intent", ""]
    ui_facts = _facts_by_tag(ctx.facts, "ui")
    if ctx.ui_applicable:
        lines.append("## Evidence")
        for fact in ui_facts:
            lines.append(f"- {fact['claim']} (evidence: {_format_evidence(fact)})")
        if not ui_facts:
            lines.append("- UI signals detected, but no detailed journey artifacts found in bounded reads.")
    else:
        lines.append("Not applicable based on bounded evidence in included directories.")
        lines.append("\nEvidence note: No UI frameworks or front-end directories detected during analysis.")
    return "\n".join(lines) + "\n"

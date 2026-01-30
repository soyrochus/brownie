from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Iterable


GENERIC_PROBES = [
    "main",
    "__main__",
    "cli",
    "command",
    "runner",
    "api",
    "route",
    "endpoint",
    "handler",
    "controller",
    "model",
    "schema",
    "entity",
    "repository",
    "migration",
    "config",
    "settings",
    ".env",
    "toml",
    "yaml",
    "json",
    "db",
    "cache",
    "queue",
    "worker",
    "scheduler",
]

TIER1_PATTERNS = [
    "cli",
    "__main__",
    "main",
    "app",
    "server",
    "runner",
    "entry",
    "bootstrap",
    "startup",
]

TIER2_PATTERNS = [
    "service",
    "domain",
    "workflow",
    "usecase",
    "use_case",
    "orchestr",
    "handler",
    "controller",
    "manager",
]


@dataclass
class CoreFileTiers:
    tier1: list[str]
    tier2: list[str]
    tier3: list[str]


def classify_core_files(paths: Iterable[str]) -> CoreFileTiers:
    tier1: list[str] = []
    tier2: list[str] = []
    tier3: list[str] = []
    for path in paths:
        name = path.lower()
        if any(token in name for token in TIER1_PATTERNS):
            tier1.append(path)
        elif any(token in name for token in TIER2_PATTERNS):
            tier2.append(path)
        else:
            tier3.append(path)
    return CoreFileTiers(tier1=tier1, tier2=tier2, tier3=tier3)


def parse_probe_terms(prompt_text: str) -> list[str]:
    lines = prompt_text.splitlines()
    probes: list[str] = []
    in_section = False
    for line in lines:
        stripped = line.strip()
        if stripped.lower().startswith("## probe terms"):
            in_section = True
            continue
        if in_section and stripped.startswith("## "):
            break
        if in_section and stripped.startswith("-"):
            term = stripped.lstrip("-").strip()
            if term:
                probes.append(term)
    return probes


def build_probe_plan(stack_prompt: str, stack_confidence: float, min_confidence: float) -> dict[str, list[str]]:
    stack_terms = []
    if stack_confidence >= min_confidence:
        stack_terms = parse_probe_terms(stack_prompt)
    return {"generic": list(GENERIC_PROBES), "stack": stack_terms}


def evidence_counts_by_tag(facts: Iterable[dict]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for fact in facts:
        for tag in fact.get("tags", []):
            key = normalize_tag(tag)
            counts[key] = counts.get(key, 0) + 1
    return counts


def normalize_tag(tag: str) -> str:
    return tag.strip().lower().replace("_", "-")


SHALLOW_PATTERNS = [
    re.compile(r"\\bcontains\\b"),
    re.compile(r"\\bincludes\\b"),
    re.compile(r"\\bdefines\\b"),
    re.compile(r"\\bmodule\\b"),
    re.compile(r"\\bfile\\b"),
    re.compile(r"\\brepository\\b"),
]


def is_shallow_fact(fact: dict) -> bool:
    claim = str(fact.get("claim", "")).lower()
    if not claim:
        return True
    if "behavior" in fact.get("tags", []) or "function" in fact.get("tags", []):
        return False
    return any(pattern.search(claim) for pattern in SHALLOW_PATTERNS)


def shallow_fact_ratio(facts: Iterable[dict]) -> float:
    facts_list = list(facts)
    if not facts_list:
        return 1.0
    shallow = sum(1 for fact in facts_list if is_shallow_fact(fact))
    return shallow / max(1, len(facts_list))


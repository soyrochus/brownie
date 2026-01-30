from __future__ import annotations

import json
import os
from dataclasses import dataclass
from datetime import datetime
from typing import Iterable


@dataclass
class Fact:
    claim: str
    evidence_path: str
    line_start: int
    line_end: int
    tags: list[str]

    def to_json(self) -> str:
        payload = {
            "claim": self.claim,
            "evidence": {
                "path": self.evidence_path,
                "line_start": self.line_start,
                "line_end": self.line_end,
            },
            "tags": self.tags,
            "collected_at": datetime.utcnow().isoformat() + "Z",
        }
        return json.dumps(payload, ensure_ascii=True)


@dataclass
class InventoryRecord:
    kind: str
    name: str
    fields: list | None = None
    params: list | None = None
    result: str | None = None
    defaults: dict | None = None
    evidence: dict | None = None
    extra: dict | None = None

    def to_json(self) -> str:
        payload = {
            "kind": self.kind,
            "name": self.name,
        }
        if self.fields is not None:
            payload["fields"] = self.fields
        if self.params is not None:
            payload["params"] = self.params
        if self.result is not None:
            payload["result"] = self.result
        if self.defaults is not None:
            payload["defaults"] = self.defaults
        if self.evidence is not None:
            payload["evidence"] = self.evidence
        if self.extra:
            payload.update(self.extra)
        return json.dumps(payload, ensure_ascii=True)


@dataclass
class FileSummary:
    path: str
    role: str
    key_functions: list
    inputs: list | None = None
    outputs: list | None = None
    constraints: list | None = None
    dependencies: list | None = None
    evidence: dict | None = None

    def to_json(self) -> str:
        payload = {
            "path": self.path,
            "role": self.role,
            "key_functions": self.key_functions,
        }
        if self.inputs is not None:
            payload["inputs"] = self.inputs
        if self.outputs is not None:
            payload["outputs"] = self.outputs
        if self.constraints is not None:
            payload["constraints"] = self.constraints
        if self.dependencies is not None:
            payload["dependencies"] = self.dependencies
        if self.evidence is not None:
            payload["evidence"] = self.evidence
        return json.dumps(payload, ensure_ascii=True)


def write_facts(path: str, facts: Iterable[Fact]) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        for fact in facts:
            handle.write(fact.to_json() + "\n")


def load_facts(path: str) -> list[dict]:
    facts: list[dict] = []
    if not os.path.exists(path):
        return facts
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                facts.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    return facts


def append_inventory_record(path: str, record: dict) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, ensure_ascii=True) + "\n")


def load_inventory(path: str) -> list[dict]:
    records: list[dict] = []
    if not os.path.exists(path):
        return records
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    return records


def append_file_summary(path: str, summary: dict) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(json.dumps(summary, ensure_ascii=True) + "\n")


def load_file_summaries(path: str) -> list[dict]:
    summaries: list[dict] = []
    if not os.path.exists(path):
        return summaries
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                summaries.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    return summaries


def write_open_questions(path: str, questions: Iterable[str]) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("# Open Questions\n\n")
        for question in questions:
            handle.write(f"- {question}\n")

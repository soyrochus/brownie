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


def write_open_questions(path: str, questions: Iterable[str]) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("# Open Questions\n\n")
        for question in questions:
            handle.write(f"- {question}\n")

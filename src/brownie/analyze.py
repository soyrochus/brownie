from __future__ import annotations

import asyncio
import json
import os
import shutil

from .agent_runtime import (
    create_agent_session,
    detect_stack,
    detect_stack_with_confidence,
    ensure_docs_dir,
    list_existing_facts,
    load_stack_prompt,
    run_agentic_docs,
    run_agentic_scan,
)
from .analysis_helpers import build_probe_plan, classify_core_files
from .config import BrownieConfig
from .feedback import AnalysisFeedback
from .fs import scan_files


class RunState:
    def __init__(self, scan_done: bool = False, facts_done: bool = False, docs_done: bool = False):
        self.scan_done = scan_done
        self.facts_done = facts_done
        self.docs_done = docs_done

    def to_dict(self) -> dict:
        return {
            "scan_done": self.scan_done,
            "facts_done": self.facts_done,
            "docs_done": self.docs_done,
        }


def analyze_repository(
    config: BrownieConfig,
    feedback: AnalysisFeedback,
    reset_cache: bool = False,
) -> None:
    root = config.root
    brownie_dir = os.path.join(root, ".brownie")
    cache_dir = os.path.join(brownie_dir, "cache")
    os.makedirs(cache_dir, exist_ok=True)

    if reset_cache:
        shutil.rmtree(cache_dir, ignore_errors=True)
        os.makedirs(cache_dir, exist_ok=True)

    run_state_path = os.path.join(cache_dir, "run-state.json")
    run_state = _load_run_state(run_state_path)

    stack = detect_stack(config)
    feedback.on_start(root, stack)

    asyncio.run(
        _run_analysis_phases(
            config=config,
            feedback=feedback,
            cache_dir=cache_dir,
        )
    )

    run_state.scan_done = True
    run_state.facts_done = True
    run_state.docs_done = True
    _write_run_state(run_state_path, run_state)


async def _run_analysis_phases(
    config: BrownieConfig,
    feedback: AnalysisFeedback,
    cache_dir: str,
) -> None:
    stack, stack_confidence = detect_stack_with_confidence(config)
    stack_prompt = load_stack_prompt(config, stack)
    probe_plan = build_probe_plan(stack_prompt, stack_confidence, 0.6)
    core_files = _core_file_candidates(config)
    client, session, ctx = await create_agent_session(
        config,
        feedback,
        stack,
        stack_confidence,
        probe_plan["stack"],
        core_files,
    )
    try:
        feedback.on_phase_start(1, "Scanning repository...")
        await run_agentic_scan(session, ctx, probe_plan["generic"], probe_plan["stack"])
        facts = list_existing_facts(config)
        feedback.on_phase_complete(1, f"Scanning complete. {len(facts)} facts collected.")

        feedback.on_phase_start(2, "Processing facts...")
        open_questions = _derive_open_questions(facts) if facts else []
        feedback.on_phase_complete(
            2,
            f"Processing complete. {len(open_questions)} open questions identified.",
        )

        feedback.on_phase_start(3, "Generating documentation...")
        ensure_docs_dir(config)
        await run_agentic_docs(session, ctx, feedback)
        for filename in _ensure_required_docs(config):
            feedback.on_doc_written(filename)
        feedback.on_phase_complete(3, "Documentation complete.")

        docs_dir = config.analysis.docs_dir
        if not os.path.isabs(docs_dir):
            feedback.on_finish(docs_dir)
        else:
            feedback.on_finish(docs_dir)
    finally:
        await client.stop()


def _ensure_required_docs(config: BrownieConfig) -> list[str]:
    required = [
        "project-intent-business-frame.md",
        "domain-landscape.md",
        "canonical-data-model.md",
        "service-capability-map.md",
        "architectural-guardrails.md",
        "api-integration-contracts.md",
        "user-journey-ui-intent.md",
    ]
    docs_dir = config.analysis.docs_dir
    if not os.path.isabs(docs_dir):
        docs_dir = os.path.join(config.root, docs_dir)
    created: list[str] = []
    for filename in required:
        path = os.path.join(docs_dir, filename)
        if os.path.exists(path):
            continue
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(
                f"# {filename.replace('-', ' ').replace('.md', '').title()}\n\n"
                "Not applicable or insufficient evidence found during bounded analysis.\n"
            )
        created.append(filename)
    return created


def _core_file_candidates(config: BrownieConfig) -> list[str]:
    files = scan_files(config.root, config.analysis.include_dirs, config.analysis.exclude_dirs)
    tiers = classify_core_files(files)
    return tiers.tier1 + tiers.tier2


def _derive_open_questions(facts: list[dict]) -> list[str]:
    tags = {str(tag).lower().replace("_", "-") for fact in facts for tag in fact.get("tags", [])}
    questions = []
    if "intent" not in tags:
        questions.append("What is the primary business goal or user outcome for this project?")
    if "data-model" not in tags:
        questions.append("What are the core domain entities and their relationships?")
    if "service" not in tags:
        questions.append("What are the primary services or bounded contexts?")
    if "api" not in tags:
        questions.append("Are there API contracts or integration points not captured in included directories?")
    if "ui" not in tags:
        questions.append("Is there a UI or user journey that lives outside the analyzed directories?")
    return questions


def _load_run_state(path: str) -> RunState:
    if not os.path.exists(path):
        return RunState()
    with open(path, "r", encoding="utf-8") as handle:
        try:
            data = json.load(handle)
        except json.JSONDecodeError:
            return RunState()
    return RunState(
        scan_done=bool(data.get("scan_done")),
        facts_done=bool(data.get("facts_done")),
        docs_done=bool(data.get("docs_done")),
    )


def _write_run_state(path: str, run_state: RunState) -> None:
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(run_state.to_dict(), handle, indent=2)

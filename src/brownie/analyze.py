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
    load_stack_prompt,
    run_agentic_refine,
    run_unified_analysis,
)
from .analysis_helpers import build_probe_plan, classify_core_files, is_source_file
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
    refining: bool = False,
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
            refining=refining,
        )
    )

    run_state.scan_done = True
    run_state.facts_done = True
    run_state.docs_done = True
    _write_run_state(run_state_path, run_state)


async def _run_analysis_phases(
    config: BrownieConfig,
    feedback: AnalysisFeedback,
    refining: bool,
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
        # Phase 1: Unified analysis and documentation (single turn)
        feedback.on_phase_start(1, "Analyzing and generating documentation...")
        ensure_docs_dir(config)

        # Scan all source files to pass to the agent, filtered by detected stack
        all_files = scan_files(
            config.root,
            config.analysis.include_dirs,
            config.analysis.exclude_dirs,
        )
        source_files = [f for f in all_files if is_source_file(f, stack)]
        await run_unified_analysis(session, ctx, feedback, source_files)
        for filename in _ensure_required_docs(config):
            feedback.on_doc_written(filename)
        feedback.on_phase_complete(1, "Documentation complete.")

        # Phase 2: Merge documents
        feedback.on_phase_start(2, "Merging documentation...")
        merged_path = _merge_docs(config)
        feedback.on_phase_complete(2, f"Merged documentation written to {merged_path}.")

        # Phase 3 (optional): Refine merged documentation
        if refining:
            feedback.on_phase_start(3, "Refining merged documentation...")
            final_path = _final_doc_path(config)
            await run_agentic_refine(session, merged_path, final_path)
            feedback.on_phase_complete(3, f"Refined documentation written to {final_path}.")

        feedback.on_finish(config.analysis.docs_dir)
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


def _derive_system_name(root: str) -> str:
    name = os.path.basename(os.path.abspath(root))
    pyproject = os.path.join(root, "pyproject.toml")
    if os.path.exists(pyproject):
        try:
            import tomllib

            with open(pyproject, "rb") as handle:
                data = tomllib.load(handle)
            project = data.get("project", {})
            project_name = project.get("name")
            if project_name:
                name = str(project_name)
        except Exception:
            pass
    return _sanitize_name(name)


def _sanitize_name(value: str) -> str:
    value = value.strip().replace(" ", "-")
    safe = []
    for char in value:
        if char.isalnum() or char in {"-", "_"}:
            safe.append(char.lower())
    return "".join(safe) or "system"


def _docs_dir_path(config: BrownieConfig) -> str:
    docs_dir = config.analysis.docs_dir
    if not os.path.isabs(docs_dir):
        docs_dir = os.path.join(config.root, docs_dir)
    return docs_dir


def _merged_doc_path(config: BrownieConfig) -> str:
    name = _derive_system_name(config.root)
    return os.path.join(_docs_dir_path(config), f"{name}-documentation.md")


def _final_doc_path(config: BrownieConfig) -> str:
    name = _derive_system_name(config.root)
    return os.path.join(_docs_dir_path(config), f"{name}-documentation-FINAL.md")


def _merge_docs(config: BrownieConfig) -> str:
    order = [
        "project-intent-business-frame.md",
        "domain-landscape.md",
        "canonical-data-model.md",
        "service-capability-map.md",
        "architectural-guardrails.md",
        "api-integration-contracts.md",
        "user-journey-ui-intent.md",
    ]
    docs_dir = _docs_dir_path(config)
    merged_path = _merged_doc_path(config)
    parts: list[str] = []
    for filename in order:
        path = os.path.join(docs_dir, filename)
        if not os.path.exists(path):
            continue
        with open(path, "r", encoding="utf-8") as handle:
            parts.append(handle.read().rstrip())
    content = "\n\n".join([part for part in parts if part])
    with open(merged_path, "w", encoding="utf-8") as handle:
        handle.write(content + ("\n" if content else ""))
    return merged_path


def _core_file_candidates(config: BrownieConfig) -> list[str]:
    files = scan_files(config.root, config.analysis.include_dirs, config.analysis.exclude_dirs)
    tiers = classify_core_files(files)
    return tiers.tier1 + tiers.tier2


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

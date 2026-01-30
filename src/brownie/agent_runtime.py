from __future__ import annotations

import os
import shutil
from dataclasses import dataclass
from typing import Any

from copilot import CopilotClient

from .analysis_helpers import GENERIC_PROBES
from .cache import load_facts
from .config import BrownieConfig, resolve_provider_settings
from .feedback import AnalysisFeedback, create_event_handler


@dataclass
class AgentContext:
    root: str
    include_dirs: list[str]
    exclude_dirs: list[str]
    docs_dir: str
    cache_dir: str
    max_file_lines: int
    chunk_lines: int
    max_grep_hits: int


REQUIRED_DOCS = [
    "project-intent-business-frame.md",
    "domain-landscape.md",
    "canonical-data-model.md",
    "service-capability-map.md",
    "architectural-guardrails.md",
    "api-integration-contracts.md",
    "user-journey-ui-intent.md",
]


def _resolve_docs_dir(ctx: AgentContext) -> str:
    docs_dir = ctx.docs_dir
    if not os.path.isabs(docs_dir):
        docs_dir = os.path.join(ctx.root, docs_dir)
    return docs_dir


def _system_prompt(
    config: BrownieConfig,
    stack: str,
    stack_confidence: float,
    stack_probe_terms: list[str],
    core_files: list[str],
) -> str:
    include_dirs = ", ".join(config.analysis.include_dirs)
    exclude_dirs = ", ".join(config.analysis.exclude_dirs)
    docs_dir = config.analysis.docs_dir
    stack_prompt = load_stack_prompt(config, stack)
    probe_terms = stack_probe_terms if stack_confidence >= 0.6 else []
    core_hint = ", ".join(core_files[:10]) if core_files else "none detected"
    return (
        "You are Brownie, an agentic technical writer. "
        "Your job is to inspect the repository using tools and produce the required documentation files.\n\n"
        "Rules:\n"
        "- Only explore included directories (and their subdirectories). Do not access excluded directories.\n"
        "- Use bounded reads; do not attempt full-repo ingestion.\n"
        "- Base claims strictly on observed evidence. Mark uncertainty explicitly.\n"
        "- Use built-in SDK tools for reading, searching, and writing files (names vary; use the available built-ins).\n"
        "- Write one documentation file at a time using write_file.\n"
        "- Do NOT output shell commands or code blocks for file writing.\n"
        "- Do NOT describe writing; invoke write_file directly.\n\n"
        "Analysis strategy:\n"
        "1) Pass 1: map structure, entrypoints, and dependencies.\n"
        "2) Pass 2: deep-read core files and any files they import.\n"
        f"- Core file candidates: {core_hint}\n"
        f"- Deep read target: {config.analysis.deep_read_min_lines}-{config.analysis.deep_read_max_lines} lines for Tier 1/2 files.\n"
        f"- Each Tier 1 file must yield at least {config.analysis.core_file_min_facts} function-level facts.\n"
        "- If evidence is shallow, rescan with deeper reads.\n\n"
        "Required probes:\n"
        f"- Generic probes (always): {', '.join(GENERIC_PROBES)}\n"
        f"- Stack probes (from prompt, only if confident): {', '.join(probe_terms) if probe_terms else 'none'}\n\n"
        f"Included directories: {include_dirs}\n"
        f"Excluded directories: {exclude_dirs}\n"
        f"Docs output directory: {docs_dir}\n\n"
        f"Stack-specific instructions:\n{stack_prompt}\n\n"
        "Required docs (exact filenames):\n"
        "1. project-intent-business-frame.md\n"
        "2. domain-landscape.md\n"
        "3. canonical-data-model.md\n"
        "4. service-capability-map.md\n"
        "5. architectural-guardrails.md\n"
        "6. api-integration-contracts.md (write stub if not applicable)\n"
        "7. user-journey-ui-intent.md (write stub if not applicable)\n"
    )


def load_stack_prompt(config: BrownieConfig, stack: str) -> str:
    prompts_dir = os.path.join(config.root, ".brownie", "prompts")
    candidates = [f"{stack}.md", "generic.md"]
    for name in candidates:
        path = os.path.join(prompts_dir, name)
        if os.path.exists(path):
            with open(path, "r", encoding="utf-8") as handle:
                return handle.read().strip()
    return "No stack-specific prompt found. Follow the general rules."


def detect_stack(config: BrownieConfig) -> str:
    stack, _confidence = detect_stack_with_confidence(config)
    return stack


def detect_stack_with_confidence(config: BrownieConfig) -> tuple[str, float]:
    include_roots = [os.path.join(config.root, d) for d in config.analysis.include_dirs]
    extensions: dict[str, int] = {}
    markers: dict[str, int] = {}

    def bump(mapping: dict[str, int], key: str, count: int = 1) -> None:
        mapping[key] = mapping.get(key, 0) + count

    for root_dir in include_roots:
        if not os.path.isdir(root_dir):
            continue
        for dirpath, dirnames, filenames in os.walk(root_dir):
            dirnames[:] = [name for name in dirnames if name not in set(config.analysis.exclude_dirs)]
            for filename in filenames:
                lower = filename.lower()
                _, ext = os.path.splitext(lower)
                if ext:
                    bump(extensions, ext)
                if lower in {"package.json", "pnpm-lock.yaml", "yarn.lock"}:
                    bump(markers, "nodejs", 3)
                if lower in {"requirements.txt", "pyproject.toml", "setup.py"}:
                    bump(markers, "python", 3)
                if lower == "go.mod":
                    bump(markers, "go", 3)
                if lower.endswith(".csproj") or lower == "global.json":
                    bump(markers, "dotnet", 3)
                if lower in {"pom.xml", "build.gradle", "build.gradle.kts"}:
                    bump(markers, "java", 3)
                if lower in {"next.config.js", "next.config.ts"}:
                    bump(markers, "react", 2)
                if lower.endswith((".tsx", ".jsx")):
                    bump(markers, "react", 1)

    score: dict[str, int] = {}
    score["python"] = markers.get("python", 0) + extensions.get(".py", 0)
    score["nodejs"] = markers.get("nodejs", 0) + extensions.get(".js", 0) + extensions.get(".ts", 0)
    score["react"] = markers.get("react", 0)
    score["go"] = markers.get("go", 0) + extensions.get(".go", 0)
    score["dotnet"] = markers.get("dotnet", 0) + extensions.get(".cs", 0) + extensions.get(".fs", 0)
    score["java"] = markers.get("java", 0) + extensions.get(".java", 0) + extensions.get(".kt", 0)

    best = max(score.items(), key=lambda item: item[1])
    if best[1] == 0:
        return "generic", 0.0
    second = sorted(score.values(), reverse=True)[1]
    if best[1] >= 3 and (second == 0 or best[1] >= second * 2):
        return best[0], 0.9
    return best[0], 0.5


async def create_agent_session(
    config: BrownieConfig,
    feedback: AnalysisFeedback,
    stack: str,
    stack_confidence: float,
    stack_probe_terms: list[str],
    core_files: list[str],
) -> tuple[CopilotClient, Any, AgentContext]:
    provider_settings = resolve_provider_settings(config)
    provider: dict[str, Any] | None = None
    if provider_settings["mode"] == "api-key":
        provider = {
            "type": provider_settings["type"],
            "base_url": provider_settings["base_url"],
            "api_key": provider_settings["api_key"],
        }
        if provider_settings["type"] == "azure":
            provider["azure"] = {"api_version": provider_settings["azure_api_version"]}

    ctx = AgentContext(
        root=config.root,
        include_dirs=config.analysis.include_dirs,
        exclude_dirs=config.analysis.exclude_dirs,
        docs_dir=config.analysis.docs_dir,
        cache_dir=os.path.join(config.root, ".brownie", "cache"),
        max_file_lines=config.analysis.max_file_lines,
        chunk_lines=config.analysis.chunk_lines,
        max_grep_hits=config.analysis.max_grep_hits,
    )

    session_config: dict[str, Any] = {
        "model": provider_settings["model"],
        "system_message": {
            "content": _system_prompt(
                config,
                stack,
                stack_confidence,
                stack_probe_terms,
                core_files,
            )
        },
    }
    if provider:
        session_config["provider"] = provider
    session_config["on_permission_request"] = _permission_handler

    client = CopilotClient()
    await client.start()

    session = await client.create_session(session_config)
    session.on(create_event_handler(feedback))
    return client, session, ctx


async def run_agentic_scan(
    session: Any,
    ctx: AgentContext,
    generic_probes: list[str],
    stack_probes: list[str],
) -> None:
    probe_lines = "\n".join([f"- {probe}" for probe in generic_probes + stack_probes])
    await session.send_and_wait(
        {
            "prompt": (
                "Stage 1: Inspect the repository using built-in tools (read_file, search_code, run_command). "
                "Perform a two-pass scan: pass 1 for structure and entrypoints, pass 2 for deep reads of core files "
                "and files they import. Use the following probes:\n"
                f"{probe_lines}\n"
                "Use run_command for directory listings when needed (e.g., ls, find). "
                "Do NOT write any docs yet."
            )
        },
        timeout=300.0,
    )


async def run_agentic_docs(
    session: Any,
    ctx: AgentContext,
    feedback: AnalysisFeedback,
) -> None:
    for filename in REQUIRED_DOCS:
        target_path = f"docs/{filename}"
        await session.send_and_wait(
            {
                "prompt": (
                    f"Stage 2: Write {filename} now. Use the built-in file tool to save it to: {target_path}. "
                    "Read relevant source files directly (bounded reads) and synthesize the document from evidence. "
                    "Base claims on observed evidence and note uncertainty. "
                    "If not applicable, write a stub with an evidence note. "
                    "Do not write any other files. "
                    "Do NOT output shell commands or code blocks; invoke write_file."
                )
            },
            timeout=300.0,
        )
        if not os.path.exists(_doc_path(ctx, filename)):
            await session.send_and_wait(
                {
                    "prompt": (
                        f"{filename} was not written. Invoke the built-in file tool now to write it to {target_path}. "
                        "Do not write any other files. Do NOT output shell commands."
                    )
                },
                timeout=300.0,
            )
        if os.path.exists(_doc_path(ctx, filename)):
            feedback.on_doc_written(filename)


def ensure_docs_dir(config: BrownieConfig) -> str:
    docs_dir = config.analysis.docs_dir
    if not os.path.isabs(docs_dir):
        docs_dir = os.path.join(config.root, docs_dir)
    if os.path.exists(docs_dir):
        shutil.rmtree(docs_dir)
    os.makedirs(docs_dir, exist_ok=True)
    return docs_dir


def list_existing_facts(config: BrownieConfig) -> list[dict]:
    facts_path = os.path.join(config.root, ".brownie", "cache", "facts.jsonl")
    return load_facts(facts_path)


def _doc_path(ctx: AgentContext, filename: str) -> str:
    return os.path.join(_resolve_docs_dir(ctx), filename)


def _permission_handler(request: dict, _env: dict[str, str]) -> dict:
    return {"kind": "approved", "rules": []}

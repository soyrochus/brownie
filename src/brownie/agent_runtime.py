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

# Document-specific prompts with reading focus, sections, and depth requirements
# These are stack-agnostic; stack-specific file patterns come from the stack prompt templates
DOC_PROMPTS: dict[str, dict[str, str | int]] = {
    "project-intent-business-frame.md": {
        "source_focus": "entry points, main modules, project manifest files (package.json, pyproject.toml, go.mod, etc.), README",
        "sections": """REQUIRED sections with numbered headings:
- §1.1 Purpose (with file:line evidence)
- §1.2 Business Goals table (columns: Goal | Description | Evidence)
- §1.3 Target Users (bullet list)
- §1.4 Value Proposition (evidence-backed bullets)
- §1.5 CLI/API Commands table (columns: Command | Flags | Defaults | Exit Codes | Evidence)""",
        "min_tables": 2,
        "min_bullets": 5,
        "required_elements": "Goals table with Evidence column, commands table with Evidence column",
    },
    "domain-landscape.md": {
        "source_focus": "data model definitions, domain entities, type definitions, enums, interfaces",
        "sections": """REQUIRED sections with numbered headings:
- §2.1 Core Domain Concepts table (columns: Concept | Description | Location)
- §2.2 Domain Boundaries (ASCII diagram showing module layers)
- §2.3 Key Types/Enums table (columns: Name | Values | Evidence)""",
        "min_tables": 2,
        "min_bullets": 5,
        "required_elements": "Concepts table with Location column, ASCII boundary diagram",
    },
    "canonical-data-model.md": {
        "source_focus": "data structures, configuration types, schema definitions, persistence/cache modules",
        "sections": """REQUIRED sections with numbered headings:
- §3.1 Configuration Models (code tree + field table with columns: Field | Type | Default | Evidence)
- §3.2 Domain Models (code tree + field table)
- §3.3 Runtime Models (code tree + field table)
- Include schema examples where serialization exists""",
        "min_tables": 3,
        "min_bullets": 5,
        "required_elements": "Code structure tree per major type, field tables with Evidence column",
    },
    "service-capability-map.md": {
        "source_focus": "module entry points, service definitions, orchestration logic, public interfaces",
        "sections": """REQUIRED sections with numbered headings:
- §4.1 Module Overview table (columns: Module | Responsibility | Key Functions)
- §4.2+ One subsection per major module with function signatures and evidence""",
        "min_tables": 2,
        "min_bullets": 5,
        "required_elements": "Module table with Key Functions column, function signatures with file:line evidence",
    },
    "architectural-guardrails.md": {
        "source_focus": "configuration defaults, validation logic, error handlers, limit constants, constraint definitions",
        "sections": """REQUIRED sections with numbered headings:
- §5.1 Limits/Constraints table (columns: Limit | Value | Evidence)
- §5.2 Scope Isolation rules with enforcement evidence
- §5.3 Validation Rules with file:line evidence
- §5.4 Error Handling paths""",
        "min_tables": 2,
        "min_bullets": 5,
        "required_elements": "Constraints table with Evidence column, validation rules with enforcement evidence",
    },
    "api-integration-contracts.md": {
        "source_focus": "external dependencies, SDK/library usage, HTTP clients, provider interfaces",
        "sections": """REQUIRED sections with numbered headings:
- §6.1 External Dependencies table (columns: Package | Usage | Evidence)
- §6.2 Provider/SDK Integration details with code examples
- §6.3 File Format Contracts (schemas, config formats)
- If no external APIs found, write stub stating "No external API integrations detected in bounded reads" """,
        "min_tables": 2,
        "min_bullets": 3,
        "required_elements": "External deps table with Evidence column, usage examples",
    },
    "user-journey-ui-intent.md": {
        "source_focus": "CLI/UI entry points, user-facing output, feedback/logging, error messages",
        "sections": """REQUIRED sections with numbered headings:
- §7.1 User Interface type (CLI/Web/API)
- §7.2 User Journeys (step sequences with file:line evidence)
- §7.3 Output Artifacts list
- §7.4 Error Handling (user-facing error messages with evidence)
- If no UI layer found, write stub stating "No UI layer detected in bounded reads" """,
        "min_tables": 1,
        "min_bullets": 5,
        "required_elements": "User flow diagram or step sequence with evidence",
    },
}


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
        "- Write one documentation file at a time using the built-in file tool.\n"
        "- Do NOT output shell commands or code blocks for file writing.\n"
        "- Do NOT describe writing; invoke the file tool directly.\n\n"
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
        target_path = os.path.join(ctx.docs_dir, filename)
        doc_config = DOC_PROMPTS.get(filename, {})
        prompt = _build_doc_prompt(filename, target_path, doc_config)
        await session.send_and_wait({"prompt": prompt}, timeout=300.0)
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


def _build_doc_prompt(filename: str, target_path: str, config: dict[str, str | int]) -> str:
    """Build a document-specific prompt with reading focus, sections, and depth requirements."""
    source_focus = config.get("source_focus", "relevant source files")
    sections = config.get("sections", "Include appropriate sections based on content found.")
    min_tables = config.get("min_tables", 1)
    min_bullets = config.get("min_bullets", 3)
    required_elements = config.get("required_elements", "tables with evidence")

    return f"""Write {filename} now.

STEP 1 - READ SOURCE FILES:
First, read these files: {source_focus}
Use bounded reads (200-400 lines per file). Search for additional relevant files if needed.

STEP 2 - WRITE DOCUMENT:
Save to: {target_path}

{sections}

DEPTH REQUIREMENTS:
- Minimum {min_tables} tables with Evidence column
- Minimum {min_bullets} evidence-anchored bullet points
- Required elements: {required_elements}

EVIDENCE FORMAT:
- Every claim must cite file:line (e.g., config.py:67)
- Tables must have an Evidence column
- Use format: **Evidence:** `filename.py:line` - `code snippet`

RULES:
- Use the built-in file tool to write the document
- Do NOT output shell commands or code blocks for file writing
- Do not write any other files"""


async def run_agentic_refine(
    session: Any,
    merged_path: str,
    final_path: str,
) -> None:
    await session.send_and_wait(
        {
            "prompt": (
                "Refinement pass: Read the merged documentation and produce a refined final version. "
                "Remove duplication, homogenize terminology, simplify without losing content, and "
                "focus on human readability.\n\n"
                f"Input file: {merged_path}\n"
                f"Output file: {final_path}\n\n"
                "Use built-in tools to read and write files. "
                "Do NOT output shell commands or code blocks; invoke the file tool directly."
            )
        },
        timeout=300.0,
    )


async def run_unified_analysis(
    session: Any,
    ctx: AgentContext,
    feedback: AnalysisFeedback,
    source_files: list[str],
) -> None:
    """Run analysis and documentation generation in a single turn.

    This approach maintains context throughout the entire process,
    allowing the agent to read files incrementally and write all
    documents with full awareness of what it has discovered.
    """
    docs_dir = ctx.docs_dir
    if not os.path.isabs(docs_dir):
        docs_dir = os.path.join(ctx.root, docs_dir)

    prompt = _build_unified_prompt(docs_dir, source_files)
    await session.send_and_wait({"prompt": prompt}, timeout=900.0)

    # Report which docs were written
    for filename in REQUIRED_DOCS:
        if os.path.exists(_doc_path(ctx, filename)):
            feedback.on_doc_written(filename)


def _build_unified_prompt(docs_dir: str, source_files: list[str]) -> str:
    """Build a single comprehensive prompt for analyzing and documenting the codebase.

    Args:
        docs_dir: Directory where documentation files should be written
        source_files: List of all source file paths that must be read
    """
    # Format the source file list for the prompt
    file_list = "\n".join([f"- {f}" for f in source_files])

    doc_instructions = []
    for i, filename in enumerate(REQUIRED_DOCS, 1):
        config = DOC_PROMPTS.get(filename, {})
        source_focus = config.get("source_focus", "relevant source files")
        sections = config.get("sections", "")
        min_tables = config.get("min_tables", 1)
        min_bullets = config.get("min_bullets", 3)
        target_path = os.path.join(docs_dir, filename)

        doc_instructions.append(f"""
### Document {i}: {filename}
**Focus on**: {source_focus}
**Write to**: {target_path}
{sections}
**Minimum**: {min_tables} tables with Evidence column, {min_bullets} evidence-anchored bullets
""")

    all_docs = "\n".join(doc_instructions)

    return f"""Analyze this codebase and write comprehensive documentation.

## SOURCE FILES TO READ
You MUST read ALL of these source files before writing documentation:
{file_list}

For each file, use bounded reads (200-400 lines). If a file is longer, read it in chunks.

## YOUR TASK
1. READ every source file listed above
2. For each document, EXTRACT concrete details: function names, class fields, config values, line numbers
3. WRITE each document with file:line evidence citations

## CRITICAL REQUIREMENTS
- You MUST read EVERY source file listed above - no shortcuts
- Every claim MUST cite file:line (e.g., `config.py:67`)
- Every table MUST have an Evidence column
- Use format: **Evidence:** `filename.py:line` - `code snippet`
- If evidence is not found in a file, state "Not found in bounded reads"

## DOCUMENTS TO WRITE
{all_docs}

## PROCESS
1. First, read ALL source files listed above (this is mandatory)
2. Then write each document in order, using the evidence you gathered
3. Each document should cite specific files and line numbers

Use the built-in file tool to write each document. Do NOT output shell commands.
Start by reading the source files now."""


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

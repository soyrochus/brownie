from __future__ import annotations

import os
import shutil
from dataclasses import dataclass
from typing import Any, Iterable

from copilot import CopilotClient, define_tool
from pydantic import BaseModel, Field

from .cache import Fact, load_facts, write_open_questions
from .config import BrownieConfig, resolve_provider_settings
from .feedback import AnalysisFeedback, create_event_handler
from .fs import read_file_chunked


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


def _is_excluded(path_parts: Iterable[str], exclude_dirs: Iterable[str]) -> bool:
    exclude_set = set(exclude_dirs)
    return any(part in exclude_set for part in path_parts)


def _is_under(path: str, parent: str) -> bool:
    path = os.path.abspath(path)
    parent = os.path.abspath(parent)
    return os.path.commonpath([path, parent]) == parent


def _resolve_docs_dir(ctx: AgentContext) -> str:
    docs_dir = ctx.docs_dir
    if not os.path.isabs(docs_dir):
        docs_dir = os.path.join(ctx.root, docs_dir)
    return docs_dir


class ListDirParams(BaseModel):
    path: str = Field(description="Directory path relative to project root or absolute path")
    max_entries: int = Field(default=200, description="Maximum entries to return")


class ReadFileParams(BaseModel):
    path: str = Field(description="File path relative to project root or absolute path")
    start_line: int = Field(default=1, description="1-based start line")
    max_lines: int = Field(default=200, description="Maximum lines to return")


class SearchParams(BaseModel):
    query: str = Field(description="Text to search for")
    max_hits: int = Field(default=50, description="Maximum number of hits to return")


class WriteDocParams(BaseModel):
    filename: str = Field(description="Docs filename to write (e.g. project-intent-business-frame.md)")
    content: str = Field(description="Full markdown content")


class WriteFactParams(BaseModel):
    claim: str = Field(description="Atomic claim")
    evidence_path: str = Field(description="Evidence file path")
    line_start: int = Field(description="Evidence start line")
    line_end: int = Field(description="Evidence end line")
    tags: list[str] = Field(default_factory=list, description="Tags for the claim")


class WriteOpenQuestionParams(BaseModel):
    question: str = Field(description="Open question to record")


@define_tool(description="List entries in a directory within the included scope.")
async def list_directory(params: ListDirParams) -> dict:
    ctx = list_directory._ctx  # type: ignore[attr-defined]
    path = _normalize_path(ctx, params.path, allow_root=True)

    if path is None:
        return {"error": "Path is outside allowed scope."}

    if os.path.abspath(path) == os.path.abspath(ctx.root):
        entries = [d for d in ctx.include_dirs if os.path.isdir(os.path.join(ctx.root, d))]
        return {"path": ctx.root, "directories": entries, "files": []}

    if not os.path.isdir(path):
        return {"error": "Not a directory."}

    entries = sorted(os.listdir(path))
    entries = [entry for entry in entries if entry not in set(ctx.exclude_dirs)]
    entries = entries[: params.max_entries]

    directories: list[str] = []
    files: list[str] = []
    for entry in entries:
        full = os.path.join(path, entry)
        if os.path.isdir(full):
            directories.append(entry)
        else:
            files.append(entry)

    return {"path": path, "directories": directories, "files": files}


@define_tool(description="Read a bounded slice of a file within the included scope.")
async def read_file_slice(params: ReadFileParams) -> dict:
    ctx = read_file_slice._ctx  # type: ignore[attr-defined]
    path = _normalize_path(ctx, params.path, allow_root=False)
    if path is None or not os.path.isfile(path):
        return {"error": "File is outside allowed scope or does not exist."}

    max_lines = min(params.max_lines, ctx.max_file_lines)
    start_line = max(1, params.start_line)

    lines: list[str] = []
    current_line = 0
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            current_line += 1
            if current_line < start_line:
                continue
            if len(lines) >= max_lines:
                break
            lines.append(line.rstrip("\n"))

    return {
        "path": path,
        "start_line": start_line,
        "end_line": start_line + len(lines) - 1 if lines else start_line,
        "lines": lines,
    }


@define_tool(description="Search for text within included directories (bounded hits).")
async def search_text(params: SearchParams) -> dict:
    ctx = search_text._ctx  # type: ignore[attr-defined]
    query = params.query.lower()
    hits: list[dict[str, Any]] = []
    max_hits = min(params.max_hits, ctx.max_grep_hits)

    include_roots = [os.path.join(ctx.root, d) for d in ctx.include_dirs]
    for root_dir in include_roots:
        if not os.path.isdir(root_dir):
            continue
        for dirpath, dirnames, filenames in os.walk(root_dir):
            dirnames[:] = [
                name
                for name in dirnames
                if name not in set(ctx.exclude_dirs)
            ]
            if _is_excluded(dirpath.split(os.sep), ctx.exclude_dirs):
                continue
            for filename in filenames:
                path = os.path.join(dirpath, filename)
                for slice_ in read_file_chunked(path, ctx.chunk_lines, ctx.max_file_lines):
                    for offset, line in enumerate(slice_.lines):
                        if query in line.lower():
                            hits.append(
                                {
                                    "path": path,
                                    "line": slice_.start_line + offset,
                                    "text": line,
                                }
                            )
                            if len(hits) >= max_hits:
                                return {"query": params.query, "hits": hits}
    return {"query": params.query, "hits": hits}


@define_tool(description="Write a documentation file into the docs directory.")
async def write_doc(params: WriteDocParams) -> dict:
    ctx = write_doc._ctx  # type: ignore[attr-defined]
    docs_dir = _resolve_docs_dir(ctx)
    os.makedirs(docs_dir, exist_ok=True)

    filename = os.path.basename(params.filename)
    if filename not in REQUIRED_DOCS:
        return {"error": f"Invalid filename. Must be one of: {', '.join(REQUIRED_DOCS)}"}
    path = os.path.join(docs_dir, filename)

    with open(path, "w", encoding="utf-8") as handle:
        handle.write(params.content)

    return {"path": path, "bytes": len(params.content)}


@define_tool(description="Record an evidence-backed fact into the cache.")
async def write_fact(params: WriteFactParams) -> dict:
    ctx = write_fact._ctx  # type: ignore[attr-defined]
    facts_path = os.path.join(ctx.cache_dir, "facts.jsonl")
    fact = Fact(
        claim=params.claim,
        evidence_path=params.evidence_path,
        line_start=params.line_start,
        line_end=params.line_end,
        tags=params.tags,
    )
    with open(facts_path, "a", encoding="utf-8") as handle:
        handle.write(fact.to_json() + "\n")
    return {"status": "ok"}


@define_tool(description="Record an open question into the cache.")
async def write_open_question(params: WriteOpenQuestionParams) -> dict:
    ctx = write_open_question._ctx  # type: ignore[attr-defined]
    questions_path = os.path.join(ctx.cache_dir, "open-questions.md")
    existing: list[str] = []
    if os.path.exists(questions_path):
        with open(questions_path, "r", encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if line.startswith("-"):
                    existing.append(line.lstrip("- ").strip())
    existing.append(params.question)
    write_open_questions(questions_path, existing)
    return {"status": "ok"}


def _normalize_path(ctx: AgentContext, path: str, allow_root: bool) -> str | None:
    if not path:
        return None
    if os.path.isabs(path):
        abs_path = os.path.abspath(path)
    else:
        abs_path = os.path.abspath(os.path.join(ctx.root, path))

    if not _is_under(abs_path, ctx.root):
        return None

    if os.path.abspath(abs_path) == os.path.abspath(ctx.root):
        return abs_path if allow_root else None

    rel = os.path.relpath(abs_path, ctx.root)
    parts = rel.split(os.sep)

    if _is_excluded(parts, ctx.exclude_dirs):
        return None

    include_roots = [os.path.normpath(d) for d in ctx.include_dirs]
    if not any(rel == inc or rel.startswith(f"{inc}{os.sep}") for inc in include_roots):
        return None

    return abs_path


def _attach_context(ctx: AgentContext) -> list[Any]:
    tools = [list_directory, read_file_slice, search_text, write_doc, write_fact, write_open_question]
    for tool in tools:
        tool._ctx = ctx  # type: ignore[attr-defined]
    return tools


def _system_prompt(config: BrownieConfig, stack: str) -> str:
    include_dirs = ", ".join(config.analysis.include_dirs)
    exclude_dirs = ", ".join(config.analysis.exclude_dirs)
    docs_dir = config.analysis.docs_dir
    stack_prompt = _load_stack_prompt(config, stack)
    return (
        "You are Brownie, an agentic technical writer. "
        "Your job is to inspect the repository using tools and produce the required documentation files.\n\n"
        "Rules:\n"
        "- Only explore included directories. Do not access excluded directories.\n"
        "- Use bounded reads; do not attempt full-repo ingestion.\n"
        "- Base claims strictly on observed evidence. Mark uncertainty explicitly.\n"
        "- Write one documentation file at a time using write_doc.\n"
        "- Record evidence-backed facts using write_fact with file path and line ranges.\n"
        "- Record open questions using write_open_question.\n\n"
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


def _load_stack_prompt(config: BrownieConfig, stack: str) -> str:
    prompts_dir = os.path.join(config.root, ".brownie", "prompts")
    candidates = [f"{stack}.md", "generic.md"]
    for name in candidates:
        path = os.path.join(prompts_dir, name)
        if os.path.exists(path):
            with open(path, "r", encoding="utf-8") as handle:
                return handle.read().strip()
    return "No stack-specific prompt found. Follow the general rules."


def detect_stack(config: BrownieConfig) -> str:
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
        return "generic"
    return best[0]


async def create_agent_session(
    config: BrownieConfig,
    feedback: AnalysisFeedback,
    stack: str,
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

    tools = _attach_context(ctx)

    session_config: dict[str, Any] = {
        "model": provider_settings["model"],
        "tools": tools,
        "systemMessage": {"content": _system_prompt(config, stack)},
    }
    if provider:
        session_config["provider"] = provider

    client = CopilotClient()
    await client.start()

    session = await client.create_session(session_config)
    session.on(create_event_handler(feedback))
    return client, session, ctx


async def run_agentic_scan(session: Any, ctx: AgentContext) -> None:
    await session.send_and_wait(
        {
            "prompt": (
                "Stage 1: Inspect the repository using list_directory, read_file_slice, and search_text. "
                "Collect evidence-backed facts with write_fact and open questions with write_open_question. "
                "Do NOT write any docs yet."
            )
        },
        timeout=300.0,
    )

    if not _facts_exist(ctx.cache_dir):
        await session.send_and_wait(
            {
                "prompt": (
                    "No facts were recorded. Re-scan included directories and record at least a handful "
                    "of evidence-backed facts before proceeding. Do NOT write docs yet."
                )
            },
            timeout=300.0,
        )


async def run_agentic_docs(session: Any, ctx: AgentContext, feedback: AnalysisFeedback) -> None:
    for filename in REQUIRED_DOCS:
        await session.send_and_wait(
            {
                "prompt": (
                    f"Stage 2: Write {filename} now. Use write_doc with the exact filename. "
                    "Base claims on observed evidence and note uncertainty. "
                    "If not applicable, write a stub with an evidence note. "
                    "Do not write any other files."
                )
            },
            timeout=300.0,
        )
        if not os.path.exists(_doc_path(ctx, filename)):
            await session.send_and_wait(
                {
                    "prompt": (
                        f"{filename} was not written. Write it now using write_doc with the exact filename. "
                        "Do not write any other files."
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


def _facts_exist(cache_dir: str) -> bool:
    facts_path = os.path.join(cache_dir, "facts.jsonl")
    return os.path.exists(facts_path) and os.path.getsize(facts_path) > 0


def _doc_path(ctx: AgentContext, filename: str) -> str:
    return os.path.join(_resolve_docs_dir(ctx), filename)

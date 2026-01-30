from __future__ import annotations

import dataclasses
import os
import tomllib
from dataclasses import dataclass
from enum import Enum
from typing import Any


class ConfigError(ValueError):
    pass


class ProviderMode(str, Enum):
    SUBSCRIPTION = "subscription"
    API_KEY = "api-key"


class ProviderType(str, Enum):
    OPENAI = "openai"
    AZURE = "azure"
    ANTHROPIC = "anthropic"


DEFAULT_INCLUDE_DIRS = ["src"]
DEFAULT_EXCLUDE_DIRS = ["node_modules", "dist", "build", ".git", ".brownie", "docs"]
DEFAULT_DOCS_DIR = "docs"
DEFAULT_DEEP_READ_MIN_LINES = 200
DEFAULT_DEEP_READ_MAX_LINES = 400
DEFAULT_CORE_FILE_MIN_FACTS = 2
DEFAULT_MIN_EVIDENCE_PER_DOC = {
    "intent": 3,
    "domain": 3,
    "data-model": 4,
    "service": 4,
    "guardrail": 3,
    "api": 2,
    "ui": 2,
}

DEFAULT_BASE_URLS = {
    ProviderType.OPENAI.value: "https://api.openai.com/v1",
    ProviderType.ANTHROPIC.value: "https://api.anthropic.com",
}

DEFAULT_MODELS = {
    ProviderType.OPENAI.value: "gpt-4o",
    ProviderType.ANTHROPIC.value: "claude-sonnet-4-20250514",
}

DEFAULT_AZURE_API_VERSION = "2024-10-21"


@dataclass
class ProviderConfig:
    mode: ProviderMode = ProviderMode.SUBSCRIPTION
    type: ProviderType | None = None
    api_key: str | None = None
    base_url: str | None = None
    model: str | None = None
    azure_api_version: str | None = None


@dataclass
class AnalysisConfig:
    include_dirs: list[str] = dataclasses.field(default_factory=lambda: list(DEFAULT_INCLUDE_DIRS))
    exclude_dirs: list[str] = dataclasses.field(default_factory=lambda: list(DEFAULT_EXCLUDE_DIRS))
    docs_dir: str = DEFAULT_DOCS_DIR
    max_files: int = 200
    max_file_lines: int = 400
    chunk_lines: int = 200
    max_grep_hits: int = 200
    deep_read_min_lines: int = DEFAULT_DEEP_READ_MIN_LINES
    deep_read_max_lines: int = DEFAULT_DEEP_READ_MAX_LINES
    core_file_min_facts: int = DEFAULT_CORE_FILE_MIN_FACTS
    min_evidence_per_doc: dict[str, int] = dataclasses.field(
        default_factory=lambda: dict(DEFAULT_MIN_EVIDENCE_PER_DOC)
    )


@dataclass
class BrownieConfig:
    root: str
    analysis: AnalysisConfig = dataclasses.field(default_factory=AnalysisConfig)
    provider: ProviderConfig = dataclasses.field(default_factory=ProviderConfig)
    model: str | None = None


def _coerce_str_list(value: Any, field: str) -> list[str]:
    if value is None:
        return []
    if isinstance(value, list):
        return [str(item).strip() for item in value if str(item).strip()]
    raise ConfigError(f"Invalid {field}: expected list[str]")


def _coerce_int(value: Any, field: str, default: int) -> int:
    if value is None:
        return default
    try:
        return int(value)
    except (TypeError, ValueError) as exc:
        raise ConfigError(f"Invalid {field}: expected int") from exc


def _coerce_int_map(value: Any, field: str, default: dict[str, int]) -> dict[str, int]:
    if value is None:
        return dict(default)
    if not isinstance(value, dict):
        raise ConfigError(f"Invalid {field}: expected dict[str,int]")
    result = dict(default)
    for key, raw in value.items():
        item_key = str(key).strip().lower().replace("_", "-")
        if item_key == "guardrails":
            item_key = "guardrail"
        if item_key == "data-models":
            item_key = "data-model"
        result[item_key] = _coerce_int(raw, f"{field}.{item_key}", result.get(item_key, 0))
    return result


def load_config(root: str) -> BrownieConfig:
    path = os.path.join(root, ".brownie", "brownie.toml")
    config = BrownieConfig(root=root)

    if not os.path.exists(path):
        return config

    with open(path, "rb") as handle:
        data = tomllib.load(handle)

    analysis_data = data.get("analysis", {})
    provider_data = data.get("provider", {})

    config.model = data.get("model")

    if analysis_data:
        include_dirs = _coerce_str_list(analysis_data.get("include_dirs"), "analysis.include_dirs")
        exclude_dirs = _coerce_str_list(analysis_data.get("exclude_dirs"), "analysis.exclude_dirs")
        docs_dir = analysis_data.get("docs_dir") or config.analysis.docs_dir
        config.analysis = AnalysisConfig(
            include_dirs=include_dirs or list(DEFAULT_INCLUDE_DIRS),
            exclude_dirs=exclude_dirs or list(DEFAULT_EXCLUDE_DIRS),
            docs_dir=str(docs_dir),
            max_files=_coerce_int(analysis_data.get("max_files"), "analysis.max_files", 200),
            max_file_lines=_coerce_int(analysis_data.get("max_file_lines"), "analysis.max_file_lines", 400),
            chunk_lines=_coerce_int(analysis_data.get("chunk_lines"), "analysis.chunk_lines", 200),
            max_grep_hits=_coerce_int(analysis_data.get("max_grep_hits"), "analysis.max_grep_hits", 200),
            deep_read_min_lines=_coerce_int(
                analysis_data.get("deep_read_min_lines"),
                "analysis.deep_read_min_lines",
                DEFAULT_DEEP_READ_MIN_LINES,
            ),
            deep_read_max_lines=_coerce_int(
                analysis_data.get("deep_read_max_lines"),
                "analysis.deep_read_max_lines",
                DEFAULT_DEEP_READ_MAX_LINES,
            ),
            core_file_min_facts=_coerce_int(
                analysis_data.get("core_file_min_facts"),
                "analysis.core_file_min_facts",
                DEFAULT_CORE_FILE_MIN_FACTS,
            ),
            min_evidence_per_doc=_coerce_int_map(
                analysis_data.get("min_evidence_per_doc"),
                "analysis.min_evidence_per_doc",
                DEFAULT_MIN_EVIDENCE_PER_DOC,
            ),
        )

    if provider_data:
        mode = provider_data.get("mode", ProviderMode.SUBSCRIPTION.value)
        type_value = provider_data.get("type")
        config.provider = ProviderConfig(
            mode=_parse_mode(mode),
            type=_parse_type(type_value),
            api_key=provider_data.get("api_key"),
            base_url=provider_data.get("base_url"),
            model=provider_data.get("model"),
            azure_api_version=provider_data.get("azure_api_version"),
        )
    else:
        config.provider = ProviderConfig()

    _validate_provider(config.provider)

    return config


def _parse_mode(value: Any) -> ProviderMode:
    if value is None:
        return ProviderMode.SUBSCRIPTION
    try:
        return ProviderMode(str(value))
    except ValueError as exc:
        raise ConfigError("Invalid provider configuration in .brownie/brownie.toml:\n  - mode: invalid") from exc


def _parse_type(value: Any) -> ProviderType | None:
    if value is None:
        return None
    try:
        return ProviderType(str(value))
    except ValueError as exc:
        raise ConfigError("Invalid provider configuration in .brownie/brownie.toml:\n  - type: invalid") from exc


def _validate_provider(provider: ProviderConfig) -> None:
    errors: list[str] = []
    if provider.mode == ProviderMode.API_KEY:
        if provider.type is None:
            errors.append("type: required when mode=\"api-key\"")
        if not provider.api_key:
            errors.append("api_key: required when mode=\"api-key\"")
        if provider.type == ProviderType.AZURE and not provider.base_url:
            errors.append("base_url: required when type=\"azure\"")
    if errors:
        message = "Invalid provider configuration in .brownie/brownie.toml:\n  - " + "\n  - ".join(errors)
        raise ConfigError(message)


def resolve_provider_settings(config: BrownieConfig) -> dict[str, Any]:
    provider = config.provider
    base_model = config.model

    if provider.mode == ProviderMode.SUBSCRIPTION:
        model = provider.model or base_model or "gpt-5"
        return {
            "mode": provider.mode.value,
            "type": None,
            "api_key": None,
            "base_url": None,
            "model": model,
            "azure_api_version": None,
        }

    provider_type = provider.type.value if provider.type else None
    base_url = provider.base_url or DEFAULT_BASE_URLS.get(provider_type)

    if provider.type == ProviderType.AZURE:
        azure_api_version = provider.azure_api_version or DEFAULT_AZURE_API_VERSION
    else:
        azure_api_version = None

    provider_default_model = DEFAULT_MODELS.get(provider_type)
    model = provider.model or provider_default_model or base_model

    return {
        "mode": provider.mode.value,
        "type": provider_type,
        "api_key": provider.api_key,
        "base_url": base_url,
        "model": model,
        "azure_api_version": azure_api_version,
    }


def merge_overrides(config: BrownieConfig, overrides: dict[str, Any]) -> BrownieConfig:
    analysis = dataclasses.replace(config.analysis)
    provider = dataclasses.replace(config.provider)

    if overrides.get("include_dirs") is not None:
        analysis.include_dirs = overrides["include_dirs"]
    if overrides.get("exclude_dirs") is not None:
        analysis.exclude_dirs = overrides["exclude_dirs"]
    if overrides.get("docs_dir") is not None:
        analysis.docs_dir = overrides["docs_dir"]

    if overrides.get("model") is not None:
        config.model = overrides["model"]

    config.analysis = analysis
    config.provider = provider

    return config


def write_config(path: str, config: BrownieConfig) -> None:
    lines = []
    if config.model:
        lines.append(f"model = \"{config.model}\"\n")

    analysis = config.analysis
    lines.append("\n[analysis]\n")
    lines.append(f"include_dirs = {analysis.include_dirs!r}\n")
    lines.append(f"exclude_dirs = {analysis.exclude_dirs!r}\n")
    lines.append(f"docs_dir = \"{analysis.docs_dir}\"\n")
    lines.append(f"max_files = {analysis.max_files}\n")
    lines.append(f"max_file_lines = {analysis.max_file_lines}\n")
    lines.append(f"chunk_lines = {analysis.chunk_lines}\n")
    lines.append(f"max_grep_hits = {analysis.max_grep_hits}\n")
    lines.append(f"deep_read_min_lines = {analysis.deep_read_min_lines}\n")
    lines.append(f"deep_read_max_lines = {analysis.deep_read_max_lines}\n")
    lines.append(f"core_file_min_facts = {analysis.core_file_min_facts}\n")
    lines.append(f"min_evidence_per_doc = {analysis.min_evidence_per_doc!r}\n")

    provider = config.provider
    lines.append("\n[provider]\n")
    lines.append(f"mode = \"{provider.mode.value}\"\n")
    if provider.type:
        lines.append(f"type = \"{provider.type.value}\"\n")
    if provider.api_key:
        lines.append(f"api_key = \"{provider.api_key}\"\n")
    if provider.base_url:
        lines.append(f"base_url = \"{provider.base_url}\"\n")
    if provider.model:
        lines.append(f"model = \"{provider.model}\"\n")
    if provider.azure_api_version:
        lines.append(f"azure_api_version = \"{provider.azure_api_version}\"\n")

    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("".join(lines))


def brownie_toml_template() -> str:
    return """# Brownie configuration file\n#\n# Top-level model used when provider doesn't define its own\n# model = \"gpt-5\"\n\n[analysis]\n# include_dirs: list of directories relative to root\ninclude_dirs = [\"src\"]\n\n# exclude_dirs: directories ignored during analysis\nexclude_dirs = [\"node_modules\", \"dist\", \"build\", \".git\", \".brownie\", \"docs\"]\n\n# docs_dir: output folder for generated docs\ndocs_dir = \"docs\"\n\n# Bounded analysis controls\n# max_files = 200\n# max_file_lines = 400\n# chunk_lines = 200\n# max_grep_hits = 200\n\n# Deep read policy\n# deep_read_min_lines = 200\n# deep_read_max_lines = 400\n# core_file_min_facts = 2\n# min_evidence_per_doc = { intent = 3, domain = 3, data_model = 4, service = 4, guardrails = 3, api = 2, ui = 2 }\n\n[provider]\n# Values: \"subscription\" | \"api-key\"\nmode = \"subscription\"\n\n# Required when mode = \"api-key\": \"openai\" | \"azure\" | \"anthropic\"\n# type = \"openai\"\n\n# Required when mode = \"api-key\"\n# api_key = \"sk-...\"\n\n# Optional: defaults per provider type\n# base_url = \"https://api.openai.com/v1\"\n\n# Optional: overrides model from config / CLI when specified\n# model = \"gpt-4o\"\n\n# Optional (Azure only): defaults to \"2024-10-21\"\n# azure_api_version = \"2024-12-01-preview\"\n"""

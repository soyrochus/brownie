from __future__ import annotations

import argparse
import os
import shutil
import sys

from .analyze import analyze_repository
from .config import (
    ConfigError,
    brownie_toml_template,
    load_config,
    merge_overrides,
    resolve_provider_settings,
    write_config,
)
from .feedback import DefaultFeedback, VerboseFeedback


def main() -> int:
    parser = argparse.ArgumentParser(prog="brownie", description="Brownie docs generator")
    subparsers = parser.add_subparsers(dest="command", required=True)

    init_parser = subparsers.add_parser("init", help="Initialize .brownie config")
    init_parser.add_argument("--root", default=".", help="Project root (default: .)")

    analyze_parser = subparsers.add_parser("analyze", help="Analyze repository and generate docs")
    analyze_parser.add_argument("--root", default=".", help="Project root (default: .)")
    analyze_parser.add_argument("--include_dirs", help="CSV list of include directories")
    analyze_parser.add_argument("--exclude_dirs", help="CSV list of exclude directories")
    analyze_parser.add_argument("--docs_dir", help="Docs output directory")
    analyze_parser.add_argument("--write-config", action="store_true", help="Write effective config to brownie.toml")
    analyze_parser.add_argument("--reset-cache", action="store_true", help="Reset cache before analysis")
    analyze_parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose output")
    analyze_parser.add_argument("-r", "--refining", action="store_true", help="Refine merged documentation")

    args = parser.parse_args()

    if args.command == "init":
        return _handle_init(args)
    if args.command == "analyze":
        return _handle_analyze(args)

    return 1


def _handle_init(args: argparse.Namespace) -> int:
    root = os.path.abspath(args.root)
    brownie_dir = os.path.join(root, ".brownie")
    cache_dir = os.path.join(brownie_dir, "cache")
    os.makedirs(cache_dir, exist_ok=True)

    config_path = os.path.join(brownie_dir, "brownie.toml")
    if not os.path.exists(config_path):
        with open(config_path, "w", encoding="utf-8") as handle:
            handle.write(brownie_toml_template())

    _ensure_prompt_templates(root)
    _ensure_gitignore(root)

    print(f"Initialized Brownie in {brownie_dir}")
    return 0


def _handle_analyze(args: argparse.Namespace) -> int:
    root = os.path.abspath(args.root)
    try:
        config = load_config(root)
    except ConfigError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 2

    overrides: dict[str, object] = {}
    if args.include_dirs is not None:
        overrides["include_dirs"] = _parse_csv(args.include_dirs)
    if args.exclude_dirs is not None:
        overrides["exclude_dirs"] = _parse_csv(args.exclude_dirs)
    if args.docs_dir is not None:
        overrides["docs_dir"] = args.docs_dir

    config = merge_overrides(config, overrides)

    if args.write_config:
        config_path = os.path.join(root, ".brownie", "brownie.toml")
        write_config(config_path, config)

    if config.provider.api_key and not _brownie_gitignored(root):
        print("Warning: API key present and .brownie/ is not in .gitignore.", file=sys.stderr)

    _ = resolve_provider_settings(config)

    feedback = VerboseFeedback() if args.verbose else DefaultFeedback()
    try:
        analyze_repository(config, feedback, reset_cache=args.reset_cache, refining=args.refining)
    except Exception as exc:  # noqa: BLE001
        feedback.on_error(f"Agent failed - {exc}")
        return 1
    return 0


def _parse_csv(value: str) -> list[str]:
    return [item.strip() for item in value.split(",") if item.strip()]


def _ensure_gitignore(root: str) -> None:
    path = os.path.join(root, ".gitignore")
    if os.path.exists(path):
        with open(path, "r", encoding="utf-8") as handle:
            content = handle.read()
        if ".brownie/" in content:
            return
        with open(path, "a", encoding="utf-8") as handle:
            handle.write("\n.brownie/\n")
        return

    with open(path, "w", encoding="utf-8") as handle:
        handle.write(".brownie/\n")


def _brownie_gitignored(root: str) -> bool:
    path = os.path.join(root, ".gitignore")
    if not os.path.exists(path):
        return False
    with open(path, "r", encoding="utf-8") as handle:
        return ".brownie/" in handle.read()


def _ensure_prompt_templates(root: str) -> None:
    from importlib import resources

    prompts_dir = os.path.join(root, ".brownie", "prompts")
    os.makedirs(prompts_dir, exist_ok=True)

    template_dir = resources.files("brownie.prompts").joinpath("templates")
    for entry in template_dir.iterdir():
        if entry.suffix != ".md":
            continue
        target = os.path.join(prompts_dir, entry.name)
        if os.path.exists(target):
            continue
        with entry.open("r", encoding="utf-8") as handle:
            content = handle.read()
        with open(target, "w", encoding="utf-8") as handle:
            handle.write(content)


if __name__ == "__main__":
    raise SystemExit(main())

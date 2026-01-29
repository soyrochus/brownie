from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Generator, Iterable


@dataclass
class FileSlice:
    path: str
    start_line: int
    end_line: int
    lines: list[str]
    truncated: bool


def is_hidden_dir(name: str) -> bool:
    return name.startswith(".") and name not in {".", ".."}


def scan_files(root: str, include_dirs: Iterable[str], exclude_dirs: Iterable[str]) -> list[str]:
    include_dirs = [os.path.join(root, path) for path in include_dirs]
    exclude_set = {name for name in exclude_dirs}
    files: list[str] = []

    for include_dir in include_dirs:
        if not os.path.isdir(include_dir):
            continue
        for dirpath, dirnames, filenames in os.walk(include_dir):
            dirnames[:] = [
                name
                for name in dirnames
                if name not in exclude_set and not is_hidden_dir(name)
            ]
            for filename in filenames:
                files.append(os.path.join(dirpath, filename))

    return files


def read_file_chunked(
    path: str,
    chunk_lines: int,
    max_lines: int,
) -> Generator[FileSlice, None, None]:
    current_lines: list[str] = []
    start_line = 1
    total_lines = 0
    truncated = False

    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            total_lines += 1
            if total_lines > max_lines:
                truncated = True
                break
            current_lines.append(line.rstrip("\n"))
            if len(current_lines) >= chunk_lines:
                end_line = start_line + len(current_lines) - 1
                yield FileSlice(path, start_line, end_line, current_lines, truncated=False)
                start_line = end_line + 1
                current_lines = []

    if current_lines:
        end_line = start_line + len(current_lines) - 1
        yield FileSlice(path, start_line, end_line, current_lines, truncated=truncated)


def iter_lines(path: str, max_lines: int) -> list[str]:
    lines: list[str] = []
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        for index, line in enumerate(handle, start=1):
            if index > max_lines:
                break
            lines.append(line.rstrip("\n"))
    return lines

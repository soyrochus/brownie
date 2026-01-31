from pathlib import Path

from brownie.cache import append_file_summary, append_inventory_record, load_file_summaries, load_inventory


def test_inventory_append_and_load(tmp_path: Path) -> None:
    path = tmp_path / "inventory.jsonl"
    append_inventory_record(str(path), {"kind": "cli_command", "name": "analyze"})
    append_inventory_record(str(path), {"kind": "dataclass", "name": "AnalysisConfig"})

    records = load_inventory(str(path))
    assert len(records) == 2
    assert records[0]["kind"] == "cli_command"
    assert records[1]["name"] == "AnalysisConfig"


def test_file_summary_append_and_load(tmp_path: Path) -> None:
    path = tmp_path / "file-summaries.jsonl"
    append_file_summary(str(path), {"path": "src/main.py", "role": "entry", "key_functions": ["main"]})

    records = load_file_summaries(str(path))
    assert len(records) == 1
    assert records[0]["role"] == "entry"

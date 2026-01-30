import os
import tempfile
import unittest

from brownie.cache import append_file_summary, append_inventory_record, load_file_summaries, load_inventory


class InventoryCacheTests(unittest.TestCase):
    def test_inventory_append_and_load(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            path = os.path.join(tempdir, "inventory.jsonl")
            append_inventory_record(path, {"kind": "cli_command", "name": "analyze"})
            append_inventory_record(path, {"kind": "dataclass", "name": "AnalysisConfig"})

            records = load_inventory(path)
            self.assertEqual(len(records), 2)
            self.assertEqual(records[0]["kind"], "cli_command")
            self.assertEqual(records[1]["name"], "AnalysisConfig")

    def test_file_summary_append_and_load(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            path = os.path.join(tempdir, "file-summaries.jsonl")
            append_file_summary(path, {"path": "src/main.py", "role": "entry", "key_functions": ["main"]})

            records = load_file_summaries(path)
            self.assertEqual(len(records), 1)
            self.assertEqual(records[0]["role"], "entry")


if __name__ == "__main__":
    unittest.main()

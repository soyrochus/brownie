import unittest

from brownie.analysis_helpers import (
    GENERIC_PROBES,
    build_probe_plan,
    classify_core_files,
    evidence_counts_by_tag,
    parse_probe_terms,
    shallow_fact_ratio,
)


class AnalysisHelpersTests(unittest.TestCase):
    def test_parse_probe_terms(self) -> None:
        prompt = """# Title

## Probe Terms
- alpha
- beta

## Other
text
"""
        self.assertEqual(parse_probe_terms(prompt), ["alpha", "beta"])

    def test_build_probe_plan(self) -> None:
        prompt = "## Probe Terms\n- alpha\n- beta\n"
        plan = build_probe_plan(prompt, stack_confidence=0.9, min_confidence=0.6)
        self.assertIn("main", plan["generic"])
        self.assertEqual(plan["stack"], ["alpha", "beta"])

        plan = build_probe_plan(prompt, stack_confidence=0.4, min_confidence=0.6)
        self.assertEqual(plan["stack"], [])

    def test_classify_core_files(self) -> None:
        files = ["src/app/main.py", "src/services/user_service.py", "src/utils/helpers.py"]
        tiers = classify_core_files(files)
        self.assertIn("src/app/main.py", tiers.tier1)
        self.assertIn("src/services/user_service.py", tiers.tier2)
        self.assertIn("src/utils/helpers.py", tiers.tier3)

    def test_evidence_counts_and_shallow_ratio(self) -> None:
        facts = [
            {"claim": "File contains helpers", "tags": ["domain"]},
            {"claim": "Function process_data transforms input", "tags": ["function", "service"]},
        ]
        counts = evidence_counts_by_tag(facts)
        self.assertEqual(counts["domain"], 1)
        self.assertEqual(counts["service"], 1)
        self.assertLess(shallow_fact_ratio(facts), 1.0)

    def test_generic_probes_present(self) -> None:
        self.assertIn("main", GENERIC_PROBES)


if __name__ == "__main__":
    unittest.main()

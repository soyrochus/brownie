from brownie.analysis_helpers import (
    GENERIC_PROBES,
    build_probe_plan,
    classify_core_files,
    evidence_counts_by_tag,
    parse_probe_terms,
    shallow_fact_ratio,
)


def test_parse_probe_terms() -> None:
    prompt = """# Title

## Probe Terms
- alpha
- beta

## Other
text
"""
    assert parse_probe_terms(prompt) == ["alpha", "beta"]


def test_build_probe_plan() -> None:
    prompt = "## Probe Terms\n- alpha\n- beta\n"
    plan = build_probe_plan(prompt, stack_confidence=0.9, min_confidence=0.6)
    assert "main" in plan["generic"]
    assert plan["stack"] == ["alpha", "beta"]

    plan = build_probe_plan(prompt, stack_confidence=0.4, min_confidence=0.6)
    assert plan["stack"] == []


def test_classify_core_files() -> None:
    files = ["src/app/main.py", "src/services/user_service.py", "src/utils/helpers.py"]
    tiers = classify_core_files(files)
    assert "src/app/main.py" in tiers.tier1
    assert "src/services/user_service.py" in tiers.tier2
    assert "src/utils/helpers.py" in tiers.tier3


def test_evidence_counts_and_shallow_ratio() -> None:
    facts = [
        {"claim": "File contains helpers", "tags": ["domain"]},
        {"claim": "Function process_data transforms input", "tags": ["function", "service"]},
    ]
    counts = evidence_counts_by_tag(facts)
    assert counts["domain"] == 1
    assert counts["service"] == 1
    assert shallow_fact_ratio(facts) < 1.0


def test_generic_probes_present() -> None:
    assert "main" in GENERIC_PROBES

from brownie.analysis_helpers import (
    GENERIC_PROBES,
    build_probe_plan,
    classify_core_files,
    evidence_counts_by_tag,
    get_source_extensions,
    is_source_file,
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


def test_get_source_extensions() -> None:
    python_exts = get_source_extensions("python")
    assert ".py" in python_exts
    assert ".json" in python_exts  # Config files always included

    nodejs_exts = get_source_extensions("nodejs")
    assert ".js" in nodejs_exts
    assert ".ts" in nodejs_exts
    assert ".py" not in nodejs_exts

    generic_exts = get_source_extensions("generic")
    assert ".json" in generic_exts
    assert ".py" not in generic_exts  # No code extensions for generic


def test_is_source_file() -> None:
    # Python stack
    assert is_source_file("/src/main.py", "python")
    assert is_source_file("/config.json", "python")
    assert not is_source_file("/src/main.js", "python")

    # Node.js stack
    assert is_source_file("/src/index.js", "nodejs")
    assert is_source_file("/src/app.ts", "nodejs")
    assert not is_source_file("/src/main.py", "nodejs")

    # Generic stack includes everything
    assert is_source_file("/src/main.py", "generic")
    assert is_source_file("/src/index.js", "generic")
    assert is_source_file("/random.xyz", "generic")

    # Config files by name
    assert is_source_file("/Dockerfile", "python")
    assert is_source_file("/Makefile", "nodejs")

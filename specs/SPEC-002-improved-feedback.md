# SPEC-002 — Improved Visual Feedback

Status: Draft
Owner: <TBD>
Last updated: 2026-01-29

---

## 1. Purpose

This spec defines improved visual feedback for Brownie's `analyze` command. Users currently see minimal output during analysis, making it difficult to understand progress or diagnose issues.

The goal is to provide:
- **Default mode**: Clear, human-readable status messages for each phase
- **Verbose mode**: Detailed streaming output including agent reasoning and tool invocations

---

## 2. Motivation

Current behavior:
- Analysis runs silently for extended periods
- User sees only "Docs generated." at completion
- No indication of progress or current phase
- Difficult to diagnose failures or hangs

Desired behavior:
- User understands what the application is doing at all times
- Progress through phases is clearly communicated
- Verbose mode available for debugging and transparency

---

## 3. Requirements

### 3.1 Default Mode (No Flags)

The application MUST display plain console messages for the following events:

| Event | Message Format |
|-------|----------------|
| Startup | `Brownie analyzing {project_root}...` |
| Stack detected | `Detected stack: {stack}` |
| Phase 1 start | `Phase 1/3: Scanning repository...` |
| Phase 1 complete | `Phase 1/3: Scanning complete. {n} facts collected.` |
| Phase 2 start | `Phase 2/3: Processing facts...` |
| Phase 2 complete | `Phase 2/3: Processing complete. {n} open questions identified.` |
| Phase 3 start | `Phase 3/3: Generating documentation...` |
| Doc written | `  - {filename}` |
| Phase 3 complete | `Phase 3/3: Documentation complete.` |
| Finish | `Done. Documentation written to {docs_dir}/` |

#### Example Default Output

```
Brownie analyzing /home/user/myproject...
Detected stack: python
Phase 1/3: Scanning repository...
Phase 1/3: Scanning complete. 23 facts collected.
Phase 2/3: Processing facts...
Phase 2/3: Processing complete. 4 open questions identified.
Phase 3/3: Generating documentation...
  - project-intent-business-frame.md
  - domain-landscape.md
  - canonical-data-model.md
  - service-capability-map.md
  - architectural-guardrails.md
  - api-integration-contracts.md
  - user-journey-ui-intent.md
Phase 3/3: Documentation complete.
Done. Documentation written to docs/
```

### 3.2 Verbose Mode (`-v` / `--verbose`)

When the `--verbose` or `-v` flag is passed, the application MUST display all default mode messages PLUS:

#### 3.2.1 Agent Reasoning (Streaming)

Display the agent's reasoning text as it streams, token-by-token:

```
[Agent] Examining the project structure to understand the codebase layout...
```

Reasoning output MUST be:
- Prefixed with `[Agent]` for clarity
- Streamed in real-time (token-by-token, not buffered)
- Displayed on its own line(s)

#### 3.2.2 Tool Invocations

Display tool calls and their results:

| Event | Format |
|-------|--------|
| Tool start | `  → {tool_name}({summary_of_params})` |
| Tool end | `  ← {tool_name}: {brief_result}` |

Parameter and result summaries MUST be truncated to reasonable lengths (max 80 characters).

#### Tool Summary Formats

| Tool | Start Summary | End Summary |
|------|---------------|-------------|
| `list_directory` | `path={path}` | `{n} entries` |
| `read_file_slice` | `path={path}, lines={start}-{end}` | `{n} lines read` |
| `search_text` | `query="{query}"` | `{n} hits` |
| `write_doc` | `filename={filename}` | `{n} bytes written` |
| `write_fact` | `claim="{claim_truncated}..."` | `ok` |
| `write_open_question` | `question="{question_truncated}..."` | `ok` |

#### Example Verbose Output

```
Brownie analyzing /home/user/myproject...
Detected stack: python
Phase 1/3: Scanning repository...
[Agent] I'll start by exploring the project structure to understand the codebase layout.
  → list_directory(path=.)
  ← list_directory: 3 entries
[Agent] Let me examine the src directory to find the main code.
  → list_directory(path=src)
  ← list_directory: 8 entries
  → read_file_slice(path=src/main.py, lines=1-200)
  ← read_file_slice: 145 lines read
[Agent] This appears to be a CLI application. Recording this as a fact.
  → write_fact(claim="Project is a CLI application with entry point in...")
  ← write_fact: ok
...
Phase 1/3: Scanning complete. 23 facts collected.
Phase 2/3: Processing facts...
Phase 2/3: Processing complete. 4 open questions identified.
Phase 3/3: Generating documentation...
[Agent] Now I'll write the project intent document based on collected evidence.
  → write_doc(filename=project-intent-business-frame.md)
  ← write_doc: 2847 bytes written
  - project-intent-business-frame.md
...
Phase 3/3: Documentation complete.
Done. Documentation written to docs/
```

### 3.3 Error Output

Errors MUST be written to stderr regardless of verbosity level.

| Scenario | Output |
|----------|--------|
| Configuration error | `Error: {message}` (exit 2) |
| Agent error | `Error: Agent failed - {message}` (exit 1) |
| Tool error | In verbose mode: `  ← {tool_name}: ERROR - {message}` |

---

## 4. CLI Interface

### 4.1 New Flag

Add `-v` / `--verbose` flag to the `analyze` subcommand:

```
brownie analyze [--root PATH] [--verbose/-v] [other flags...]
```

### 4.2 Flag Behavior

| Flag | Behavior |
|------|----------|
| (none) | Default mode - phase messages only |
| `-v` or `--verbose` | Verbose mode - streaming + tool invocations |

---

## 5. Implementation Architecture

### 5.1 Feedback Interface

Define an abstract feedback interface to decouple output from analysis logic:

```python
from abc import ABC, abstractmethod

class AnalysisFeedback(ABC):
    @abstractmethod
    def on_start(self, root: str, stack: str) -> None: ...

    @abstractmethod
    def on_phase_start(self, phase: int, description: str) -> None: ...

    @abstractmethod
    def on_phase_complete(self, phase: int, summary: str) -> None: ...

    @abstractmethod
    def on_doc_written(self, filename: str) -> None: ...

    @abstractmethod
    def on_finish(self, docs_dir: str) -> None: ...

    @abstractmethod
    def on_agent_message(self, delta: str) -> None: ...

    @abstractmethod
    def on_tool_start(self, tool_name: str, params_summary: str) -> None: ...

    @abstractmethod
    def on_tool_end(self, tool_name: str, result_summary: str) -> None: ...

    @abstractmethod
    def on_error(self, message: str) -> None: ...
```

### 5.2 Implementations

Two implementations of the feedback interface:

#### DefaultFeedback

- Implements phase and completion messages
- No-ops for agent message and tool events

#### VerboseFeedback

- Extends DefaultFeedback behavior
- Implements streaming agent messages
- Implements tool invocation logging

### 5.3 Event Handler Integration

The verbose feedback MUST register an event handler with the Copilot SDK session:

```python
from copilot.generated.session_events import SessionEventType

def create_event_handler(feedback: AnalysisFeedback):
    def handler(event):
        match event.type:
            case SessionEventType.ASSISTANT_MESSAGE_DELTA:
                feedback.on_agent_message(event.delta)
            case SessionEventType.TOOL_INVOCATION_START:
                feedback.on_tool_start(event.tool_name, summarize_params(event.params))
            case SessionEventType.TOOL_INVOCATION_END:
                feedback.on_tool_end(event.tool_name, summarize_result(event.result))
            case SessionEventType.ERROR:
                feedback.on_error(event.error)
    return handler
```

### 5.4 Module Structure

Create a new module for feedback:

```
src/brownie/
├── feedback.py          # NEW: AnalysisFeedback interface and implementations
├── agent_runtime.py     # Modified: Accept feedback, register event handler
├── analyze.py           # Modified: Accept feedback, call feedback methods
├── cli.py               # Modified: Parse --verbose, instantiate feedback
```

---

## 6. Behavioral Rules

### 6.1 Output Stream Rules

| Content | Stream |
|---------|--------|
| Normal progress messages | stdout |
| Agent reasoning (verbose) | stdout |
| Tool invocations (verbose) | stdout |
| Errors | stderr |
| Warnings | stderr |

### 6.2 Streaming Rules

- Agent reasoning MUST stream token-by-token using `print(..., end="", flush=True)`
- A newline MUST be printed after each complete agent message
- Tool invocation messages are NOT streamed (printed complete)

### 6.3 Truncation Rules

- Tool parameter summaries: max 60 characters
- Tool result summaries: max 60 characters
- Fact claims in verbose output: max 50 characters + "..."
- Questions in verbose output: max 50 characters + "..."

### 6.4 Prefix Rules

- Agent reasoning lines MUST start with `[Agent] `
- Tool start lines MUST start with `  → ` (2 spaces + arrow)
- Tool end lines MUST start with `  ← ` (2 spaces + arrow)
- Doc written lines MUST start with `  - ` (2 spaces + dash)

---

## 7. Testing Requirements

### 7.1 Unit Tests

- Test DefaultFeedback produces correct output for each event
- Test VerboseFeedback produces correct output for each event
- Test truncation logic for parameters and results
- Test event handler correctly routes SDK events to feedback

### 7.2 Integration Tests

- Test full analysis with default feedback (capture stdout)
- Test full analysis with verbose feedback (capture stdout)
- Test error handling writes to stderr

---

## 8. Acceptance Criteria

1. Running `brownie analyze` displays phase progress messages
2. Running `brownie analyze -v` displays streaming agent output
3. Running `brownie analyze --verbose` displays tool invocations
4. Errors appear on stderr in both modes
5. Output is human-readable without parsing
6. Verbose mode does not break default mode functionality
7. Feedback interface allows future alternative implementations (e.g., JSON, progress bars)

---

## 9. Future Considerations (Out of Scope)

The following are explicitly deferred:

- Progress bars or spinners (would require terminal capability detection)
- JSON output mode for machine parsing
- Log file output
- Color/formatting (would require terminal capability detection)
- Quiet mode (`-q`) to suppress all output

These may be addressed in future specs.

---

## 10. References

- SPEC-001: Initial Implementation (defines phases and run state)
- Copilot SDK Tutorial: Section 5.4 Events (event handler patterns)
- `src/brownie/agent_runtime.py`: Current implementation without feedback

---

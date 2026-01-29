from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any, Callable
import json
import sys


MAX_PARAM_SUMMARY = 60
MAX_RESULT_SUMMARY = 60
MAX_CLAIM_SUMMARY = 50
MAX_QUESTION_SUMMARY = 50


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
    def on_agent_message_complete(self) -> None: ...

    @abstractmethod
    def on_tool_start(self, tool_name: str, params_summary: str) -> None: ...

    @abstractmethod
    def on_tool_end(self, tool_name: str, result_summary: str) -> None: ...

    @abstractmethod
    def on_error(self, message: str) -> None: ...


class DefaultFeedback(AnalysisFeedback):
    def __init__(self, stdout=None, stderr=None) -> None:
        self._stdout = stdout or sys.stdout
        self._stderr = stderr or sys.stderr

    def on_start(self, root: str, stack: str) -> None:
        print(f"Brownie analyzing {root}...", file=self._stdout)
        print(f"Detected stack: {stack}", file=self._stdout)

    def on_phase_start(self, phase: int, description: str) -> None:
        print(f"Phase {phase}/3: {description}", file=self._stdout)

    def on_phase_complete(self, phase: int, summary: str) -> None:
        print(f"Phase {phase}/3: {summary}", file=self._stdout)

    def on_doc_written(self, filename: str) -> None:
        print(f"  - {filename}", file=self._stdout)

    def on_finish(self, docs_dir: str) -> None:
        suffix = "" if docs_dir.endswith("/") else "/"
        print(f"Done. Documentation written to {docs_dir}{suffix}", file=self._stdout)

    def on_agent_message(self, delta: str) -> None:
        return None

    def on_agent_message_complete(self) -> None:
        return None

    def on_tool_start(self, tool_name: str, params_summary: str) -> None:
        return None

    def on_tool_end(self, tool_name: str, result_summary: str) -> None:
        return None

    def on_error(self, message: str) -> None:
        print(f"Error: {message}", file=self._stderr)


class VerboseFeedback(DefaultFeedback):
    def __init__(self, stdout=None, stderr=None) -> None:
        super().__init__(stdout=stdout, stderr=stderr)
        self._agent_line_open = False

    def _close_agent_line(self) -> None:
        if self._agent_line_open:
            print("", file=self._stdout)
            self._agent_line_open = False

    def on_agent_message(self, delta: str) -> None:
        if not self._agent_line_open:
            print("[Agent] ", end="", file=self._stdout, flush=True)
            self._agent_line_open = True
        print(delta, end="", file=self._stdout, flush=True)

    def on_agent_message_complete(self) -> None:
        self._close_agent_line()

    def on_tool_start(self, tool_name: str, params_summary: str) -> None:
        self._close_agent_line()
        print(f"  \u2192 {tool_name}({params_summary})", file=self._stdout)

    def on_tool_end(self, tool_name: str, result_summary: str) -> None:
        self._close_agent_line()
        print(f"  \u2190 {tool_name}: {result_summary}", file=self._stdout)


def _truncate(text: str, limit: int, suffix: str = "...") -> str:
    if len(text) <= limit:
        return text
    return text[: max(0, limit - len(suffix))] + suffix


def _truncate_with_suffix(text: str, limit: int, suffix: str = "...") -> str:
    if len(text) <= limit:
        return text
    return text[:limit] + suffix


def _as_dict(value: Any) -> dict[str, Any]:
    if value is None:
        return {}
    if isinstance(value, dict):
        return value
    if hasattr(value, "dict"):
        return value.dict()  # type: ignore[no-any-return]
    if hasattr(value, "__dict__"):
        return dict(value.__dict__)
    return {"value": str(value)}


def summarize_params(tool_name: str, params: Any) -> str:
    data = _as_dict(params)
    summary = ""

    if tool_name == "list_directory":
        summary = f"path={data.get('path')}"
    elif tool_name == "read_file_slice":
        start = data.get("start_line", 1)
        max_lines = data.get("max_lines")
        end = start
        if isinstance(max_lines, int):
            end = start + max_lines - 1
        summary = f"path={data.get('path')}, lines={start}-{end}"
    elif tool_name == "search_text":
        summary = f"query=\"{data.get('query')}\""
    elif tool_name == "write_doc":
        summary = f"filename={data.get('filename')}"
    elif tool_name == "write_fact":
        claim = str(data.get("claim", ""))
        claim = _truncate_with_suffix(claim, MAX_CLAIM_SUMMARY)
        summary = f"claim=\"{claim}\""
    elif tool_name == "write_open_question":
        question = str(data.get("question", ""))
        question = _truncate_with_suffix(question, MAX_QUESTION_SUMMARY)
        summary = f"question=\"{question}\""
    else:
        summary = str(data)

    return _truncate(summary, MAX_PARAM_SUMMARY)


def summarize_result(tool_name: str, result: Any) -> str:
    data = _as_dict(result)
    if "content" in data and isinstance(data["content"], str):
        parsed = _coerce_json(data["content"])
        if isinstance(parsed, dict):
            data = parsed
    if "error" in data and data["error"]:
        message = str(data["error"])
        return _truncate(f"ERROR - {message}", MAX_RESULT_SUMMARY)

    summary = "ok"
    if tool_name == "list_directory":
        entries = len(data.get("directories", []) or []) + len(data.get("files", []) or [])
        summary = f"{entries} entries"
    elif tool_name == "read_file_slice":
        summary = f"{len(data.get('lines', []) or [])} lines read"
    elif tool_name == "search_text":
        summary = f"{len(data.get('hits', []) or [])} hits"
    elif tool_name == "write_doc":
        summary = f"{data.get('bytes', 0)} bytes written"
    elif tool_name in {"write_fact", "write_open_question"}:
        summary = "ok"

    return _truncate(str(summary), MAX_RESULT_SUMMARY)


def create_event_handler(feedback: AnalysisFeedback) -> Callable[[Any], None]:
    from copilot.generated.session_events import SessionEventType

    tool_calls: dict[str, str] = {}
    streamed_since_turn = False

    def handler(event: Any) -> None:
        nonlocal streamed_since_turn
        event_type = getattr(event, "type", None)
        delta_type = getattr(SessionEventType, "ASSISTANT_MESSAGE_DELTA", None)
        reasoning_delta_type = getattr(SessionEventType, "ASSISTANT_REASONING_DELTA", None)
        assistant_message_type = getattr(SessionEventType, "ASSISTANT_MESSAGE", None)
        assistant_reasoning_type = getattr(SessionEventType, "ASSISTANT_REASONING", None)
        turn_end_type = getattr(SessionEventType, "ASSISTANT_TURN_END", None)
        complete_type = getattr(SessionEventType, "ASSISTANT_MESSAGE_COMPLETE", None)
        tool_start_type = getattr(SessionEventType, "TOOL_INVOCATION_START", None)
        tool_end_type = getattr(SessionEventType, "TOOL_INVOCATION_END", None)
        tool_exec_start_type = getattr(SessionEventType, "TOOL_EXECUTION_START", None)
        tool_exec_end_type = getattr(SessionEventType, "TOOL_EXECUTION_COMPLETE", None)
        error_type = getattr(SessionEventType, "ERROR", None)
        session_error_type = getattr(SessionEventType, "SESSION_ERROR", None)

        def _event_is(target: Any, name: str) -> bool:
            if target is not None and event_type == target:
                return True
            if isinstance(event_type, str) and event_type == name:
                return True
            if hasattr(event_type, "name") and event_type.name == name:
                return True
            return False

        data = getattr(event, "data", None)

        def _event_text() -> str | None:
            for attr in ("delta", "delta_content", "partial_output", "content"):
                value = getattr(event, attr, None)
                if value:
                    return str(value)
            if data is not None:
                for attr in ("delta_content", "partial_output", "content", "transformed_content"):
                    value = getattr(data, attr, None)
                    if value:
                        return str(value)
            return None

        def _tool_name() -> str | None:
            for attr in ("tool_name", "name"):
                value = getattr(event, attr, None)
                if value:
                    return str(value)
            if data is not None:
                for attr in ("tool_name", "mcp_tool_name", "name"):
                    value = getattr(data, attr, None)
                    if value:
                        return str(value)
            return None

        def _tool_params() -> Any:
            for attr in ("params", "arguments", "input"):
                value = getattr(event, attr, None)
                if value is not None:
                    return _coerce_json(value)
            if data is not None:
                for attr in ("arguments", "input"):
                    value = getattr(data, attr, None)
                    if value is not None:
                        return _coerce_json(value)
            return {}

        def _tool_result() -> Any:
            for attr in ("result", "output"):
                value = getattr(event, attr, None)
                if value is not None:
                    return _coerce_json(value)
            if data is not None:
                for attr in ("output", "result"):
                    value = getattr(data, attr, None)
                    if value is not None:
                        return _coerce_json(value)
            return {}

        def _tool_call_id() -> str | None:
            for attr in ("tool_call_id", "toolCallId"):
                value = getattr(event, attr, None)
                if value:
                    return str(value)
            if data is not None:
                for attr in ("tool_call_id", "toolCallId", "parent_tool_call_id", "parentToolCallId"):
                    value = getattr(data, attr, None)
                    if value:
                        return str(value)
            return None

        if _event_is(delta_type, "ASSISTANT_MESSAGE_DELTA") or _event_is(reasoning_delta_type, "ASSISTANT_REASONING_DELTA"):
            delta = _event_text()
            if delta:
                feedback.on_agent_message(delta)
                streamed_since_turn = True
        elif (
            (complete_type is not None and _event_is(complete_type, "ASSISTANT_MESSAGE_COMPLETE"))
            or _event_is(assistant_message_type, "ASSISTANT_MESSAGE")
            or _event_is(assistant_reasoning_type, "ASSISTANT_REASONING")
            or _event_is(turn_end_type, "ASSISTANT_TURN_END")
        ):
            if not streamed_since_turn:
                content = _event_text()
                if content:
                    feedback.on_agent_message(content)
            feedback.on_agent_message_complete()
            streamed_since_turn = False
        elif _event_is(tool_start_type, "TOOL_INVOCATION_START") or _event_is(tool_exec_start_type, "TOOL_EXECUTION_START"):
            tool_name = _tool_name() or "tool"
            call_id = _tool_call_id()
            if call_id:
                tool_calls[call_id] = tool_name
            summary = summarize_params(tool_name, _tool_params())
            feedback.on_tool_start(tool_name, summary)
        elif _event_is(tool_end_type, "TOOL_INVOCATION_END") or _event_is(tool_exec_end_type, "TOOL_EXECUTION_COMPLETE"):
            tool_name = _tool_name()
            if tool_name is None:
                call_id = _tool_call_id()
                if call_id and call_id in tool_calls:
                    tool_name = tool_calls[call_id]
            tool_name = tool_name or "tool"
            result = _tool_result()
            summary = summarize_result(tool_name, result)
            feedback.on_tool_end(tool_name, summary)
            data_map = _as_dict(result)
            if data_map.get("error"):
                feedback.on_error(f"{tool_name}: {data_map.get('error')}")
        elif _event_is(error_type, "ERROR") or _event_is(session_error_type, "SESSION_ERROR"):
            message = getattr(event, "error", None)
            if message is None and data is not None:
                message = getattr(data, "message", None) or getattr(data, "error", None)
            feedback.on_error(str(message))

    return handler


def _coerce_json(value: Any) -> Any:
    if isinstance(value, str):
        stripped = value.strip()
        if stripped.startswith("{") or stripped.startswith("["):
            try:
                return json.loads(stripped)
            except json.JSONDecodeError:
                return value
    return value

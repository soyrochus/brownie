import io
import unittest

from brownie.feedback import (
    DefaultFeedback,
    VerboseFeedback,
    create_event_handler,
    summarize_params,
    summarize_result,
)
from copilot.generated.session_events import SessionEventType


class DummyData:
    def __init__(self, **kwargs) -> None:
        for key, value in kwargs.items():
            setattr(self, key, value)


class DummyEvent:
    def __init__(self, type_, data=None, **kwargs) -> None:
        self.type = type_
        self.data = data
        for key, value in kwargs.items():
            setattr(self, key, value)


class RecordingFeedback(DefaultFeedback):
    def __init__(self) -> None:
        self.calls = []
        super().__init__(stdout=io.StringIO(), stderr=io.StringIO())

    def on_agent_message(self, delta: str) -> None:
        self.calls.append(("agent", delta))

    def on_agent_message_complete(self) -> None:
        self.calls.append(("agent_complete", None))

    def on_tool_start(self, tool_name: str, params_summary: str) -> None:
        self.calls.append(("tool_start", tool_name, params_summary))

    def on_tool_end(self, tool_name: str, result_summary: str) -> None:
        self.calls.append(("tool_end", tool_name, result_summary))

    def on_error(self, message: str) -> None:
        self.calls.append(("error", message))


class FeedbackTests(unittest.TestCase):
    def test_default_feedback_output(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        feedback = DefaultFeedback(stdout=stdout, stderr=stderr)

        feedback.on_start("/repo", "python")
        feedback.on_phase_start(1, "Scanning repository...")
        feedback.on_phase_complete(1, "Scanning complete. 2 facts collected.")
        feedback.on_phase_start(2, "Processing facts...")
        feedback.on_phase_complete(2, "Processing complete. 1 open questions identified.")
        feedback.on_phase_start(3, "Generating documentation...")
        feedback.on_doc_written("project-intent-business-frame.md")
        feedback.on_phase_complete(3, "Documentation complete.")
        feedback.on_finish("docs")

        output = stdout.getvalue().strip().splitlines()
        self.assertEqual(output[0], "Brownie analyzing /repo...")
        self.assertEqual(output[1], "Detected stack: python")
        self.assertEqual(output[-1], "Done. Documentation written to docs/")
        self.assertEqual(stderr.getvalue(), "")

    def test_verbose_feedback_streaming(self) -> None:
        stdout = io.StringIO()
        feedback = VerboseFeedback(stdout=stdout)

        feedback.on_agent_message("Hello")
        feedback.on_agent_message(" world")
        feedback.on_agent_message_complete()
        self.assertEqual(stdout.getvalue(), "[Agent] Hello world\n")

    def test_verbose_feedback_tool_lines(self) -> None:
        stdout = io.StringIO()
        feedback = VerboseFeedback(stdout=stdout)

        feedback.on_tool_start("read_file", "path=src/main.py")
        feedback.on_tool_end("read_file", "10 lines read")
        output = stdout.getvalue().splitlines()
        self.assertEqual(output[0], "  → read_file(path=src/main.py)")
        self.assertEqual(output[1], "  ← read_file: 10 lines read")

    def test_summarize_truncation(self) -> None:
        long_claim = "x" * 80
        params = {"claim": long_claim}
        summary = summarize_params("write_fact", params)
        self.assertLessEqual(len(summary), 60)
        self.assertIn("...", summary)

        long_error = {"error": "y" * 100}
        result = summarize_result("list_directory", long_error)
        self.assertLessEqual(len(result), 60)
        self.assertIn("ERROR -", result)

    def test_event_handler_routing(self) -> None:
        feedback = RecordingFeedback()
        handler = create_event_handler(feedback)

        handler(DummyEvent(SessionEventType.ASSISTANT_MESSAGE_DELTA, data=DummyData(delta_content="Hi")))
        handler(DummyEvent(SessionEventType.ASSISTANT_TURN_END))
        handler(
            DummyEvent(
                SessionEventType.TOOL_EXECUTION_START,
                data=DummyData(tool_name="list_directory", arguments={"path": "."}),
            )
        )
        handler(
            DummyEvent(
                SessionEventType.TOOL_EXECUTION_COMPLETE,
                data=DummyData(tool_name="list_directory", output={"directories": [], "files": ["a", "b"]}),
            )
        )
        handler(DummyEvent(SessionEventType.SESSION_ERROR, data=DummyData(message="boom")))

        self.assertEqual(feedback.calls[0], ("agent", "Hi"))
        self.assertEqual(feedback.calls[1][0], "agent_complete")
        self.assertEqual(feedback.calls[2][0], "tool_start")
        self.assertEqual(feedback.calls[3][0], "tool_end")
        self.assertEqual(feedback.calls[-1], ("error", "boom"))


if __name__ == "__main__":
    unittest.main()

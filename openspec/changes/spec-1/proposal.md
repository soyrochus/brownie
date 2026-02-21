## Why

The full system cannot be built safely without first proving that the Copilot CLI agent runtime is reachable, stable, and observable from Rust. SPEC-1 establishes this foundation by delivering the minimal vertical slice that exercises session lifecycle, streaming output, and error recovery — before any dynamic UI or catalog logic is introduced.

## What Changes

- Introduce a new Rust binary (`brownie`) with a static three-column desktop shell (workspace selector, chat transcript, placeholder canvas)
- Implement `CopilotClient`: a wrapper around `copilot-sdk-rust` handling initialization, session creation, multi-turn message sending, streaming event consumption, and shutdown
- Render streaming assistant output incrementally in the transcript area without blocking the UI thread
- Enforce passive mode unconditionally — no file writes, no command execution, no tool-call dispatch
- Display connection status in the top bar and surface SDK/CLI errors in a diagnostics area
- Persist minimal session metadata locally so sessions can be resumed or reconstructed on restart
- Canvas panel renders a static placeholder message only ("Dynamic UI will render here") — no DSL logic

## Capabilities

### New Capabilities

- `copilot-client`: Wrapper around `copilot-sdk-rust` covering initialization, session lifecycle (create/resume/close), streaming message handling, error mapping, and SDK restart recovery
- `app-shell`: Static three-column desktop window with top bar (connection status, passive mode indicator), left workspace/session selector, central streaming transcript + input bar, and right-side canvas placeholder
- `session-persistence`: Local storage of minimal session metadata (session ID, workspace binding, transcript) enabling resume or reconstruction on restart

### Modified Capabilities

<!-- none — this is a greenfield slice -->

## Impact

- New Rust binary target; project moves from current state to a runnable desktop application
- Adds `copilot-sdk-rust` as a direct dependency (requires GitHub Copilot CLI installed and authenticated on the host)
- UI framework dependency introduced (to be decided in design — egui or similar immediate-mode GUI)
- No existing code paths are modified; this is purely additive
- Acceptance gate: app connects to Copilot CLI via SDK, sends and receives multi-turn streamed messages, survives SDK restarts without crashing

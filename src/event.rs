use copilot_sdk::ConnectionState;

#[derive(Debug, Clone)]
pub enum AppEvent {
    StreamDelta(String),
    StreamEnd,
    StatusChanged(ConnectionState),
    SdkError(String),
    SessionCreated(String),
    ToolCallSuppressed(String),
}

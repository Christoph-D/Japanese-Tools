use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("IRC error: {0}")]
    IrcError(#[from] irc::error::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Script execution failed: {0}")]
    ScriptExecutionError(String),

    #[error("Invalid script path: {0}")]
    InvalidScriptPath(String),
}

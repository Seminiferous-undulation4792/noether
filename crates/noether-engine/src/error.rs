use crate::checker::GraphTypeError;
use crate::executor::ExecutionError;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("type errors in composition graph")]
    TypeCheck(Vec<GraphTypeError>),
    #[error("execution error: {0}")]
    Execution(#[from] ExecutionError),
}

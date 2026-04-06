mod checker;
mod display;
mod primitive;

pub use checker::{is_subtype_of, IncompatibilityReason, TypeCompatibility};
pub use primitive::NType;

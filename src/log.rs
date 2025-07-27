use std::fmt;

pub fn error(error: &impl fmt::Display) {
    log::error!("{error}");
}

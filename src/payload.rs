use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload<T> {
    protocol_version: String,
    #[serde(flatten)]
    raw: T,
}

impl<T> Payload<T> {
    pub fn new(raw: T) -> Self {
        Self {
            protocol_version: "2025-06-18".to_owned(),
            raw,
        }
    }

    pub fn into_raw(self) -> T {
        self.raw
    }
}

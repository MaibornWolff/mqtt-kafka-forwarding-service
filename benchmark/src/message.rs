use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMessage {
    pub id: u64,
    pub last: bool,
}

use {
    crate::span::SerializableLevel,
    serde::{Deserialize, Serialize},
    std::{collections::HashMap, time::SystemTime},
    tracing::Level,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub message: String,
    pub level: SerializableLevel,
    pub target: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub module_path: Option<String>,
    pub fields: HashMap<String, String>,
    pub timestamp: SystemTime,
}
impl EventData {
    pub fn new(message: String, level: Level, target: String) -> Self {
        Self {
            message,
            level: level.into(),
            target,
            file: None,
            line: None,
            module_path: None,
            fields: HashMap::new(),
            timestamp: SystemTime::now(),
        }
    }

    pub fn level(&self) -> Level { self.level.clone().into() }

    pub fn add_field(&mut self, key: String, value: String) { self.fields.insert(key, value); }
}

use {
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        time::{Duration, SystemTime},
    },
    tracing::Level,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializableLevel(pub String);

impl std::fmt::Display for SerializableLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

impl PartialEq<Level> for SerializableLevel {
    fn eq(&self, other: &Level) -> bool {
        let self_level: Level = self.clone().into();
        self_level == *other
    }
}

impl From<Level> for SerializableLevel {
    fn from(level: Level) -> Self { SerializableLevel(level.to_string()) }
}

impl From<SerializableLevel> for Level {
    fn from(ser_level: SerializableLevel) -> Self {
        match ser_level.0.as_str() {
            "ERROR" => Level::ERROR,
            "WARN" => Level::WARN,
            "INFO" => Level::INFO,
            "DEBUG" => Level::DEBUG,
            "TRACE" => Level::TRACE,
            _ => Level::INFO, // Default fallback
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanInfo {
    pub id: u64,
    pub name: String,
    pub target: String,
    pub level: SerializableLevel,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub module_path: Option<String>,
    pub fields: HashMap<String, String>,
    pub entered_at: SystemTime,
    pub exited_at: Option<SystemTime>,
    pub duration: Option<Duration>,
    pub children: Vec<SpanInfo>,
}

impl SpanInfo {
    pub fn new(id: u64, name: String, target: String, level: Level) -> Self {
        Self {
            id,
            name,
            target,
            level: level.into(),
            file: None,
            line: None,
            module_path: None,
            fields: HashMap::new(),
            entered_at: SystemTime::now(),
            exited_at: None,
            duration: None,
            children: Vec::new(),
        }
    }

    pub fn level(&self) -> Level { self.level.clone().into() }

    pub fn add_field(&mut self, key: String, value: String) { self.fields.insert(key, value); }

    pub fn add_child(&mut self, child: SpanInfo) { self.children.push(child); }

    pub fn exit(&mut self) {
        let now = SystemTime::now();
        self.exited_at = Some(now);
        if let Ok(duration) = now.duration_since(self.entered_at) {
            self.duration = Some(duration);
        }
    }

    pub fn is_active(&self) -> bool { self.exited_at.is_none() }

    pub fn get_duration(&self) -> Option<Duration> {
        self.duration.or_else(|| SystemTime::now().duration_since(self.entered_at).ok())
    }
}

use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};
use tracing::Level;

#[derive(Debug, Clone)]
pub struct SpanInfo {
    pub id: u64,
    pub name: String,
    pub target: String,
    pub level: Level,
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
            level,
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

    pub fn add_field(&mut self, key: String, value: String) {
        self.fields.insert(key, value);
    }

    pub fn add_child(&mut self, child: SpanInfo) {
        self.children.push(child);
    }

    pub fn exit(&mut self) {
        let now = SystemTime::now();
        self.exited_at = Some(now);
        if let Ok(duration) = now.duration_since(self.entered_at) {
            self.duration = Some(duration);
        }
    }

    pub fn is_active(&self) -> bool {
        self.exited_at.is_none()
    }

    pub fn get_duration(&self) -> Option<Duration> {
        self.duration.or_else(|| SystemTime::now().duration_since(self.entered_at).ok())
    }
}
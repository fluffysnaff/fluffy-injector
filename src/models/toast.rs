use chrono::{DateTime, Duration, Utc};

pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

pub struct Toast {
    pub level: ToastLevel,
    pub message: String,
    created_at: DateTime<Utc>,
    duration: Duration,
}

impl Toast {
    pub fn new(level: ToastLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            created_at: Utc::now(),
            duration: Duration::seconds(5),
        }
    }

    pub fn is_alive(&self) -> bool {
        Utc::now() - self.created_at < self.duration
    }
}
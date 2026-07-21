use std::time::{Duration, Instant};

pub(crate) enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

pub(crate) struct Toast {
    pub level: ToastLevel,
    pub message: String,
    created_at: Instant,
}

impl Toast {
    pub(crate) fn new(level: ToastLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            created_at: Instant::now(),
        }
    }

    pub(crate) fn is_alive(&self) -> bool {
        self.created_at.elapsed() < Duration::from_secs(5)
    }
}

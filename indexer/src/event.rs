#[derive(Debug, serde::Serialize)]
pub struct Event {
    event_type: String,
    value: String,
}

impl Event {
    pub fn parse_event(log: &str) -> Option<Event> {
        if log.contains("YourEventIdentifier") {
            let parts: Vec<&str> = log.split(": ").collect();
            if parts.len() == 2 {
                return Some(Event {
                    event_type: parts[0].to_string(),
                    value: parts[1].to_string(),
                });
            }
        }
        None
    }
}

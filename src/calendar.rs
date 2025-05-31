use std::fmt;

use anyhow::Result;
use chrono::{DateTime, Local, TimeZone, Utc};
use icalendar::parser::unfold;
use log::debug;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CalendarError {
    #[error("Failed to fetch calendar: {0}")]
    FetchError(String),

    #[error("Failed to parse calendar: {0}")]
    ParseError(String),

    #[error("No events found in calendar")]
    NoEventsError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// Name/title of the event
    pub name: String,

    /// Event start time (in local time)
    pub start_time: DateTime<Local>,

    /// Event end time (in local time)
    pub end_time: DateTime<Local>,

    /// Event duration in minutes
    pub duration_minutes: i64,

    /// Optional location of the event
    pub location: Option<String>,

    /// Optional description of the event
    pub description: Option<String>,
}

impl fmt::Display for CalendarEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.format_time_range())
    }
}

impl CalendarEvent {
    /// Creates a new CalendarEvent
    pub fn new(
        name: String,
        start_time: DateTime<Local>,
        end_time: DateTime<Local>,
        location: Option<String>,
        description: Option<String>,
    ) -> Self {
        let duration = end_time.signed_duration_since(start_time);
        let duration_minutes = duration.num_minutes();

        Self {
            name,
            start_time,
            end_time,
            duration_minutes,
            location,
            description,
        }
    }

    /// Returns a formatted string of the time range (e.g., "09:00 - 10:30")
    pub fn format_time_range(&self) -> String {
        format!(
            "{} - {}",
            self.start_time.format("%H:%M"),
            self.end_time.format("%H:%M")
        )
    }

    /// Returns true if the event is currently ongoing
    pub fn is_current(&self) -> bool {
        let now = Local::now();
        now >= self.start_time && now < self.end_time
    }

    /// Returns true if the event is in the future
    pub fn is_future(&self) -> bool {
        Local::now() < self.start_time
    }
}

#[derive(Debug, Clone)]
pub struct Calendar {
    /// URL of the ICAL calendar
    url: String,

    /// Last time the calendar was fetched
    last_updated: Option<DateTime<Utc>>,

    /// Cached calendar events
    events: Vec<CalendarEvent>,

    /// How often to refresh the calendar data (in minutes)
    refresh_interval_minutes: u64,
}

impl Calendar {
    /// Creates a new Calendar with the given ICAL URL
    pub fn new(url: String, refresh_interval_minutes: u64) -> Self {
        Self {
            url,
            last_updated: None,
            events: Vec::new(),
            refresh_interval_minutes,
        }
    }

    /// Fetches the calendar data from the URL and updates the events
    pub async fn update(&mut self) -> Result<(), CalendarError> {
        // Check if we need to update based on the refresh interval
        if let Some(last_updated) = self.last_updated {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(last_updated);

            if elapsed.num_minutes() < self.refresh_interval_minutes as i64 {
                debug!(
                    "Using cached calendar data, last updated {} minutes ago",
                    elapsed.num_minutes()
                );
                return Ok(());
            }
        }

        debug!("Fetching calendar data from {}", self.url);

        // Fetch the calendar data
        let response = reqwest::get(&self.url)
            .await
            .map_err(|e| CalendarError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CalendarError::FetchError(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let calendar_data = response
            .text()
            .await
            .map_err(|e| CalendarError::FetchError(e.to_string()))?;

        // Parse the calendar data
        let unfolded_calendar = unfold(&calendar_data);
        let parsed_calendar = icalendar::parser::read_calendar(&unfolded_calendar)
            .map_err(|e| CalendarError::ParseError(e.to_string()))?;

        // Extract events
        let mut events = Vec::new();

        for component in parsed_calendar.components.iter() {
            // Only process VEVENT components
            if component.name != "VEVENT" {
                continue;
            }

            // Extract event properties
            let mut summary = None;
            let mut dtstart = None;
            let mut dtend = None;
            let mut location = None;
            let mut description = None;

            for property in &component.properties {
                match property.name.as_str() {
                    "SUMMARY" => summary = Some(property.val.to_string()),
                    "DTSTART" => dtstart = parse_datetime_property(Some(property)),
                    "DTEND" => dtend = parse_datetime_property(Some(property)),
                    "LOCATION" => location = Some(property.val.to_string()),
                    "DESCRIPTION" => description = Some(property.val.to_string()),
                    _ => {}
                }
            }

            if let (Some(summary), Some(dtstart), Some(dtend)) = (summary, dtstart, dtend) {
                let event = CalendarEvent::new(summary, dtstart, dtend, location, description);
                events.push(event);
            }
        }

        // Sort events by start time
        events.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        // Update the calendar
        self.events = events;
        self.last_updated = Some(Utc::now());

        debug!("Found {} events in calendar", self.events.len());

        Ok(())
    }

    /// Returns the current event (if any)
    pub fn get_current_event(&self) -> Option<&CalendarEvent> {
        let now = Local::now();
        self.events
            .iter()
            .find(|e| now >= e.start_time && now < e.end_time)
    }

    /// Returns the next event (if any)
    pub fn get_next_event(&self) -> Option<&CalendarEvent> {
        let now = Local::now();
        self.events.iter().find(|e| e.start_time > now)
    }

    /// Returns all future events (including current)
    pub fn get_future_events(&self) -> Vec<&CalendarEvent> {
        let now = Local::now();
        self.events.iter().filter(|e| e.end_time > now).collect()
    }
}

/// Helper function to parse datetime from iCalendar property
fn parse_datetime_property(
    property: Option<&icalendar::parser::Property>,
) -> Option<DateTime<Local>> {
    let property = property?;
    let value = property.val.to_string();

    // Try parsing as UTC time (ends with Z)
    if value.ends_with('Z') {
        if let Ok(dt) = DateTime::parse_from_str(&value, "%Y%m%dT%H%M%SZ") {
            return Some(dt.with_timezone(&Local));
        }
    }

    // Try parsing as local time
    if value.contains('T') {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&value, "%Y%m%dT%H%M%S") {
            return Some(
                Local
                    .from_local_datetime(&dt)
                    .single()
                    .unwrap_or_else(|| Local::now()),
            );
        }
    }

    // Try parsing as date (all-day event)
    if let Ok(date) = chrono::NaiveDate::parse_from_str(&value, "%Y%m%d") {
        let naive_dt = date.and_hms_opt(0, 0, 0).unwrap();
        return Some(
            Local
                .from_local_datetime(&naive_dt)
                .single()
                .unwrap_or_else(|| Local::now()),
        );
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_calendar_event() {
        let start = Local.with_ymd_and_hms(2023, 1, 1, 9, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2023, 1, 1, 10, 30, 0).unwrap();

        let event = CalendarEvent::new(
            "Test Meeting".to_string(),
            start,
            end,
            Some("Conference Room A".to_string()),
            Some("Project kickoff meeting".to_string()),
        );

        assert_eq!(event.name, "Test Meeting");
        assert_eq!(event.duration_minutes, 90);
        assert_eq!(event.format_time_range(), "09:00 - 10:30");
    }
}

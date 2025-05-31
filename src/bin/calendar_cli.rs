use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use trmnl_meeting_room_display::calendar::{Calendar, CalendarEvent};

#[derive(Parser, Debug)]
#[command(
    name = "calendar-cli",
    author = "Terminal Meeting Room Display",
    version = "1.0",
    about = "A CLI tool to display events from an iCalendar URL"
)]
struct Args {
    /// The URL of the iCalendar file
    #[arg(short, long)]
    url: String,

    /// Number of upcoming events to display (after the current one)
    #[arg(short = 'n', long, default_value = "3")]
    upcoming: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize a new calendar with the given URL
    // We'll use a 15-minute refresh interval, but it doesn't matter much for this CLI tool
    let mut calendar = Calendar::new(args.url.clone(), 15);

    // Fetch and parse the calendar data
    calendar
        .update()
        .await
        .context("Failed to fetch calendar data")?;

    // Display the current event (if any)
    match calendar.get_current_event() {
        Some(event) => {
            println!("\n=== CURRENT EVENT ===");
            print_event(event);
        }
        None => println!("\nNo events currently in progress."),
    }

    // Display upcoming events
    let future_events = calendar.get_future_events();
    let next_events: Vec<_> = future_events
        .into_iter()
        .filter(|e| !e.is_current()) // Filter out the current event
        .take(args.upcoming)
        .collect();

    if next_events.is_empty() {
        println!("\nNo upcoming events scheduled.");
    } else {
        println!("\n=== UPCOMING EVENTS ===");
        for (i, event) in next_events.iter().enumerate() {
            println!("\n--- Event {} ---", i + 1);
            print_event(event);
        }
    }

    Ok(())
}

/// Prints an event to the console
fn print_event(event: &CalendarEvent) {
    println!("Title: {}", event.name);
    println!("Time: {}", event.format_time_range());
    println!(
        "Date: {}",
        event.start_time.format("%A, %B %d, %Y").to_string()
    );
    println!("Duration: {} minutes", event.duration_minutes);

    if event.is_current() {
        let now = Local::now();
        let remaining_mins = event.end_time.signed_duration_since(now).num_minutes();
        println!("Status: In progress ({} minutes remaining)", remaining_mins);
    } else {
        let now = Local::now();
        let until_mins = event.start_time.signed_duration_since(now).num_minutes();
        let until_hours = until_mins / 60;
        let remaining_mins = until_mins % 60;

        if until_hours > 0 {
            println!(
                "Status: Starts in {} hour(s) and {} minute(s)",
                until_hours, remaining_mins
            );
        } else {
            println!("Status: Starts in {} minute(s)", until_mins);
        }
    }

    if let Some(location) = &event.location {
        println!("Location: {}", location);
    }
}

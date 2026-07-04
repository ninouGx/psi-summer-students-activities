use chrono::{DateTime, NaiveDateTime, TimeZone};
use icalendar::{Calendar, Component, Event, EventLike};
use scraper::Selector;
use std::collections::HashMap;
use std::fmt::Display;

struct Activity {
    date_time: DateTime<chrono::Utc>,
    location: String,
    topic: String,
    speakers: Vec<String>,
    link: Option<String>,
}

impl Display for Activity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Date and Time: {}\nLocation: {}\nTopic/Activity: {}\nSpeakers: {}\nLink: {}",
            self.date_time,
            self.location,
            self.topic,
            self.speakers.join(", "),
            self.link.as_deref().unwrap_or("N/A")
        )
    }
}

impl Activity {
    fn new(
        date_time: DateTime<chrono::Utc>,
        location: String,
        topic: String,
        speakers: Vec<String>,
        link: Option<String>,
    ) -> Self {
        Activity {
            date_time,
            location,
            topic,
            speakers,
            link,
        }
    }

    fn from_row(row: scraper::ElementRef) -> Option<Self> {
        let timezone = chrono_tz::Europe::Zurich;

        let cells_selector = Selector::parse("td").unwrap();
        let cells: Vec<String> = row
            .select(&cells_selector)
            .map(|cell| cell.inner_html())
            .collect();

        if cells.len() < 5 {
            return None;
        }

        let (date_str, link) =
            if let Some(link_element) = row.select(&Selector::parse("td a").unwrap()).next() {
                let link = link_element.value().attr("href").map(str::to_string);
                (link_element.text().collect::<Vec<_>>().join(" "), link)
            } else {
                (cells[0].clone(), None)
            };
        let datetime_str = format!("{} {}", date_str, cells[1]);
        let date_time = NaiveDateTime::parse_from_str(&datetime_str, "%d.%m.%Y %H:%M")
            .ok()
            .and_then(|naive| timezone.from_local_datetime(&naive).single())
            .map(|dt| dt.with_timezone(&chrono::Utc))?;
        let location = cells[2].clone();
        let topic = cells[3]
            .replace("<br>", "\n")
            .replace("&nbsp;", "")
            .trim()
            .to_string();

        let speakers: Vec<String> = cells[4]
            .split("<br>")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "&nbsp;")
            .collect();

        Some(Activity::new(date_time, location, topic, speakers, link))
    }

    fn to_icalendar_event(&self, uid: &str) -> Event {
        let mut event = Event::new();
        event.uid(uid);
        event.summary(&self.topic);
        event.description(&format!(
            "Speakers: {}\nLink: {}",
            self.speakers.join(", "),
            self.link.as_deref().unwrap_or("N/A")
        ));
        event.starts(self.date_time);
        event.ends(self.date_time + chrono::Duration::hours(1));
        event.location(&self.location);
        event
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const URL: &str = "https://www.psi.ch/en/summerstudents/activities";
    const DEFAULT_CALENDAR_FILE_PATH: &str = "psi_summer_students_activities.ics";

    let file_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CALENDAR_FILE_PATH.to_string());

    let activities = get_activities_from_url(URL).await?;
    let calendar = create_icalendar(&activities);
    write_calendar_file(&file_path, &calendar)?;

    Ok(())
}

async fn get_activities_from_url(url: &str) -> Result<Vec<Activity>, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    let activities = parse_activities_from_html(&body);
    Ok(activities)
}

fn parse_activities_from_html(html: &str) -> Vec<Activity> {
    let fragment = scraper::Html::parse_document(html);
    let table_selector = Selector::parse("table").unwrap();
    let table = fragment.select(&table_selector).next().unwrap();
    let rows_selector = Selector::parse("tr").unwrap();
    let activities: Vec<Activity> = table
        .select(&rows_selector)
        .filter_map(Activity::from_row)
        .collect();
    activities
}

fn create_icalendar(activities: &[Activity]) -> Calendar {
    let mut calendar = Calendar::new();
    // Stable UID = date + a per-day sequence. The date is the anchor, the sequence
    // only breaks ties between activities sharing the same date (e.g. the two on
    // 22.07). A within-day index does not shift when rows are added or removed
    // elsewhere in the table, so identities stay stable across daily runs.
    let mut per_day_seq: HashMap<String, u32> = HashMap::new();
    for activity in activities {
        let date_key = activity.date_time.format("%Y%m%d").to_string();
        let seq = per_day_seq.entry(date_key.clone()).or_insert(0);
        let uid = format!("{date_key}-{seq}@psi.ch-summerstudents");
        *seq += 1;
        calendar.push(activity.to_icalendar_event(&uid));
    }
    calendar.name("PSI Summer Students Activities");
    calendar.done()
}

fn write_calendar_file(file_path: &str, calendar: &Calendar) -> std::io::Result<()> {
    std::fs::write(file_path, calendar.to_string())
}

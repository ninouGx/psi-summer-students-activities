use std::fmt::Display;

use reqwest::Error;
use scraper::Selector;
use chrono::{ DateTime, NaiveDateTime, TimeZone };

struct Activity {
    date_time: DateTime<chrono::Utc>,
    location: String,
    topic_activity: String,
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
            self.topic_activity,
            self.speakers.join(", "),
            self.link.as_deref().unwrap_or("N/A")
        )
    }
}

impl Activity {
    fn new(
        date_time: DateTime<chrono::Utc>,
        location: String,
        topic_activity: String,
        speakers: Vec<String>,
        link: Option<String>
    ) -> Self {
        Activity {
            date_time,
            location,
            topic_activity,
            speakers,
            link,
        }
    }

    /*
    <td><a href="https://indico.psi.ch/event/18603/">02.07.2026</a></td>
    <td>10:00</td>
    <td>WBGB/019</td>
    <td>Welcome to the programme<br>Introduction to PSI<br>Welcome from the sports club<br>Welcome from the PhD and postdoc association<br>&nbsp;</td>
    <td>Clemens Lange<br>Ines Günther-Leopold<br>Ben Martin<br>Nicola Rizzi<br>&nbsp;</td>
    ["02.07.2026", "10:00", "WBGB/019 (coffee break at ~10:45)", "Welcome to the programme Introduction to PSI Welcome from the sports club Welcome from the PhD and postdoc association \u{a0}", "Clemens Lange Ines Günther-Leopold Ben Martin Nicola Rizzi \u{a0}"]
    ["08.07.2026", "10:00", "WBGB/019", "Accelerator Facilities at PSI", "Rasmus Ischebeck"]
    ["15.07.2026", "10:00", "WBGB/019", "Nuclear Engineering and Sciences (PANDA visit afterwards)", "Terttaliisa Lind"]
    ["22.07.2026", "10:00", "WBGB/019", "Particle Physics (tour afterwards)", "Klaus Kirch"]
    ["22.07.2026", "17:00", "OASE", "Barbecue", "\u{a0}"]
    */
    fn from_row(row: scraper::ElementRef) -> Option<Self> {
        let timezone = chrono_tz::Europe::Zurich;
        /*
        <td><a href="https://indico.psi.ch/event/18603/">02.07.2026</a></td>
        <td>10:00</td>
        <td>WBGB/019</td>
        <td>Welcome to the programme<br>Introduction to PSI<br>Welcome from the sports club<br>Welcome from the PhD and postdoc association<br>&nbsp;</td>
        <td>Clemens Lange<br>Ines Günther-Leopold<br>Ben Martin<br>Nicola Rizzi<br>&nbsp;</td>
         */
        let cells_selector = Selector::parse("td").unwrap();
        //need to keep the <br> tags to split the text later
        let cells: Vec<String> = row
            .select(&cells_selector)
            .map(|cell| cell.inner_html())
            .collect();

        if cells.len() < 5 {
            return None;
        }

        let (date_str, link) = if
            let Some(link_element) = row.select(&Selector::parse("td a").unwrap()).next()
        {
            let link = link_element
                .value()
                .attr("href")
                .map(|s| s.to_string());
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
        let topic_activity = cells[3]
            .replace("<br>", "\n")
            .replace("&nbsp;", "")
            .trim()
            .to_string();

        let speakers: Vec<String> = cells[4]
            .split("<br>")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "&nbsp;")
            .collect();

        Some(Activity::new(date_time, location, topic_activity, speakers, link))
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    const URL: &str = "https://www.psi.ch/en/summerstudents/activities";
    let client = reqwest::Client::new();
    let response = client.get(URL).send().await?;
    let body = response.text().await?;
    let fragment = scraper::Html::parse_document(&body);
    let table_selector = Selector::parse("table").unwrap();
    let table = fragment.select(&table_selector).next().unwrap();
    let rows_selector = Selector::parse("tr").unwrap();
    let activities: Vec<Activity> = table
        .select(&rows_selector)
        .filter_map(Activity::from_row)
        .collect();

    for activity in activities {
        println!("{}\n", activity);
    }

    Ok(())
}

# psi-summer-students-activities

Scrapes the [PSI summer-students activities](https://www.psi.ch/en/summerstudents/activities)
page and publishes it as a subscribable `.ics` calendar, refreshed daily. Add the URL
to any calendar app and the programme shows up automatically.

## Subscribe

```
https://calendar.nils.galloux.net/psi_summer_students_activities.ics
```

## How it works

- Scrapes the activities table from the PSI page (`reqwest` + `scraper`).
- Builds an iCalendar feed with the [`icalendar`](https://crates.io/crates/icalendar)
  crate — one event per activity.
- Each event gets a stable UID (`YYYYMMDD-<seq>@psi.ch-summerstudents`), so calendar
  apps reconcile changes by UID. There is no state to store and nothing to diff: each
  run regenerates the whole feed and overwrites the file.
- The page is checked once a day, so the calendar tracks new and changed activities.

## Usage

```sh
cargo build --release
./target/release/psi-summer-students-activities [output.ics]
```

Without an argument it writes `psi_summer_students_activities.ics` in the current
directory.

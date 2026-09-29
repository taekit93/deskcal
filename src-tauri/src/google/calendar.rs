use reqwest::Method;
use serde::Deserialize;

use super::models::{Calendar, Event, UNTITLED};
use super::{enc, Google};
use crate::error::AppError;

const DEFAULT_COLOR: &str = "#4285f4";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarListPage {
    #[serde(default)]
    items: Vec<RawCalendar>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCalendar {
    id: String,
    summary: Option<String>,
    summary_override: Option<String>,
    background_color: Option<String>,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventsPage {
    #[serde(default)]
    items: Vec<RawEvent>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEvent {
    id: String,
    summary: Option<String>,
    status: Option<String>,
    start: Option<RawTime>,
    end: Option<RawTime>,
    html_link: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTime {
    date: Option<String>,
    date_time: Option<String>,
}

impl Google {
    pub async fn list_calendars(&self) -> Result<Vec<Calendar>, AppError> {
        let url = format!("{}/users/me/calendarList", self.calendar_base);
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut q = vec![("maxResults", "250".to_string())];
            if let Some(t) = &page_token {
                q.push(("pageToken", t.clone()));
            }
            let page: CalendarListPage = self.send_json(Method::GET, &url, &q, None).await?;
            out.extend(page.items.into_iter().map(|c| Calendar {
                id: c.id,
                summary: c.summary_override.or(c.summary).unwrap_or_default(),
                color: c.background_color.unwrap_or_else(|| DEFAULT_COLOR.into()),
                primary: c.primary,
            }));
            match page.next_page_token {
                Some(t) => page_token = Some(t),
                None => return Ok(out),
            }
        }
    }

    pub async fn list_events(&self, calendar_id: &str, time_min: &str, time_max: &str) -> Result<Vec<Event>, AppError> {
        let url = format!("{}/calendars/{}/events", self.calendar_base, enc(calendar_id));
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut q = vec![
                ("timeMin", time_min.to_string()),
                ("timeMax", time_max.to_string()),
                ("singleEvents", "true".to_string()),
                ("orderBy", "startTime".to_string()),
                ("maxResults", "2500".to_string()),
            ];
            if let Some(t) = &page_token {
                q.push(("pageToken", t.clone()));
            }
            let page: EventsPage = self.send_json(Method::GET, &url, &q, None).await?;
            out.extend(page.items.into_iter().filter_map(|e| convert_event(calendar_id, e)));
            match page.next_page_token {
                Some(t) => page_token = Some(t),
                None => return Ok(out),
            }
        }
    }
}

fn convert_event(calendar_id: &str, e: RawEvent) -> Option<Event> {
    if e.status.as_deref() == Some("cancelled") {
        return None;
    }
    let (start, end) = (e.start?, e.end?);
    let (all_day, start, end) = match (start.date, start.date_time, end.date, end.date_time) {
        (Some(s), _, Some(en), _) => (true, s, en),
        (_, Some(s), _, Some(en)) => (false, s, en),
        _ => return None,
    };
    Some(Event {
        id: e.id,
        calendar_id: calendar_id.to_string(),
        title: e.summary.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| UNTITLED.into()),
        all_day,
        start,
        end,
        html_link: e.html_link,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use crate::error::AppError;
    use crate::google::models::Calendar;
    use crate::google::test_support::{mount_token, setup};

    #[tokio::test]
    async fn list_calendars_maps_fields() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .and(header("authorization", "Bearer at"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"id": "me@x.com", "summary": "나", "backgroundColor": "#9fe1e7", "primary": true},
                    {"id": "c2", "summary": "원래 이름", "summaryOverride": "별칭"}
                ]
            })))
            .mount(&server)
            .await;
        let cals = g.list_calendars().await.unwrap();
        assert_eq!(
            cals,
            vec![
                Calendar { id: "me@x.com".into(), summary: "나".into(), color: "#9fe1e7".into(), primary: true },
                Calendar { id: "c2".into(), summary: "별칭".into(), color: "#4285f4".into(), primary: false },
            ]
        );
    }

    #[tokio::test]
    async fn list_events_follows_pagination_and_skips_cancelled() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/c1/events"))
            .and(query_param("pageToken", "p2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [{"id": "e3", "summary": "회의",
                           "start": {"dateTime": "2026-09-15T10:00:00+09:00"},
                           "end": {"dateTime": "2026-09-15T11:00:00+09:00"}}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/c1/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"id": "e1", "summary": "휴가", "start": {"date": "2026-09-10"}, "end": {"date": "2026-09-12"},
                     "htmlLink": "https://calendar.google.com/e1"},
                    {"id": "e2", "status": "cancelled"}
                ],
                "nextPageToken": "p2"
            })))
            .mount(&server)
            .await;
        let events = g.list_events("c1", "a", "b").await.unwrap();
        let ids: Vec<_> = events.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["e1", "e3"]);
        assert!(events[0].all_day);
        assert_eq!(events[0].start, "2026-09-10");
        assert_eq!(events[0].end, "2026-09-12");
        assert_eq!(events[0].calendar_id, "c1");
        assert_eq!(events[0].html_link.as_deref(), Some("https://calendar.google.com/e1"));
        assert!(!events[1].all_day);
        assert_eq!(events[1].start, "2026-09-15T10:00:00+09:00");
    }

    #[tokio::test]
    async fn list_events_encodes_calendar_id_and_sends_range() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/ko.south_korea%23holiday%40group.v.calendar.google.com/events"))
            .and(query_param("timeMin", "2026-08-30T00:00:00+09:00"))
            .and(query_param("timeMax", "2026-10-11T00:00:00+09:00"))
            .and(query_param("singleEvents", "true"))
            .and(query_param("orderBy", "startTime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": []})))
            .expect(1)
            .mount(&server)
            .await;
        let events = g
            .list_events(
                "ko.south_korea#holiday@group.v.calendar.google.com",
                "2026-08-30T00:00:00+09:00",
                "2026-10-11T00:00:00+09:00",
            )
            .await
            .unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn event_without_title_gets_placeholder() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/c1/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [{"id": "e1", "start": {"date": "2026-09-10"}, "end": {"date": "2026-09-11"}},
                          {"id": "e2", "summary": "   ", "start": {"date": "2026-09-10"}, "end": {"date": "2026-09-11"}}]
            })))
            .mount(&server)
            .await;
        let events = g.list_events("c1", "a", "b").await.unwrap();
        assert_eq!(events[0].title, "(제목 없음)");
        assert_eq!(events[1].title, "(제목 없음)");
    }

    #[tokio::test]
    async fn retries_on_503_then_succeeds() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": []})))
            .mount(&server)
            .await;
        assert!(g.list_calendars().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn gives_up_after_three_retries() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(503))
            .expect(4)
            .mount(&server)
            .await;
        assert!(matches!(g.list_calendars().await, Err(AppError::Api { status: 503, .. })));
    }

    #[tokio::test]
    async fn refreshes_token_once_on_401() {
        let (server, g) = setup().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "at", "expires_in": 3600})))
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(401))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": []})))
            .mount(&server)
            .await;
        assert!(g.list_calendars().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn not_found_is_api_error() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/c1/events"))
            .respond_with(ResponseTemplate::new(404).set_body_string("nope"))
            .mount(&server)
            .await;
        assert!(matches!(g.list_events("c1", "a", "b").await, Err(AppError::Api { status: 404, .. })));
    }
}

use reqwest::Method;
use serde::Deserialize;
use serde_json::json;

use super::models::{Task, TaskList, UNTITLED};
use super::{enc, Google};
use crate::error::AppError;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListsPage {
    #[serde(default)]
    items: Vec<RawList>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct RawList {
    id: String,
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TasksPage {
    #[serde(default)]
    items: Vec<RawTask>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct RawTask {
    id: String,
    title: Option<String>,
    due: Option<String>,
    notes: Option<String>,
    status: Option<String>,
}

fn title_or_placeholder(title: Option<String>) -> String {
    title.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| UNTITLED.into())
}

impl Google {
    pub async fn list_tasklists(&self) -> Result<Vec<TaskList>, AppError> {
        let url = format!("{}/users/@me/lists", self.tasks_base);
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut q = vec![("maxResults", "100".to_string())];
            if let Some(t) = &page_token {
                q.push(("pageToken", t.clone()));
            }
            let page: ListsPage = self.send_json(Method::GET, &url, &q, None).await?;
            out.extend(page.items.into_iter().map(|l| TaskList { id: l.id, title: title_or_placeholder(l.title) }));
            match page.next_page_token {
                Some(t) => page_token = Some(t),
                None => return Ok(out),
            }
        }
    }

    pub async fn list_tasks(&self, list_id: &str) -> Result<Vec<Task>, AppError> {
        let url = format!("{}/lists/{}/tasks", self.tasks_base, enc(list_id));
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut q = vec![
                ("showCompleted", "false".to_string()),
                ("showHidden", "false".to_string()),
                ("maxResults", "100".to_string()),
            ];
            if let Some(t) = &page_token {
                q.push(("pageToken", t.clone()));
            }
            let page: TasksPage = self.send_json(Method::GET, &url, &q, None).await?;
            out.extend(
                page.items
                    .into_iter()
                    .filter(|t| t.status.as_deref() != Some("completed"))
                    .map(|t| Task {
                        id: t.id,
                        list_id: list_id.to_string(),
                        title: title_or_placeholder(t.title),
                        due: t.due.and_then(|d| d.get(..10).map(str::to_string)),
                        notes: t.notes,
                    }),
            );
            match page.next_page_token {
                Some(t) => page_token = Some(t),
                None => return Ok(out),
            }
        }
    }

    pub async fn set_task_completed(&self, list_id: &str, task_id: &str, completed: bool) -> Result<(), AppError> {
        let url = format!("{}/lists/{}/tasks/{}", self.tasks_base, enc(list_id), enc(task_id));
        let body = if completed {
            json!({ "status": "completed" })
        } else {
            json!({ "status": "needsAction", "completed": null })
        };
        let _: serde_json::Value = self.send_json(Method::PATCH, &url, &[], Some(&body)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use crate::google::models::{Task, TaskList};
    use crate::google::test_support::{mount_token, setup};

    #[tokio::test]
    async fn list_tasklists_maps_titles() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/tasks/v1/users/@me/lists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [{"id": "L1", "title": "내 할 일"}, {"id": "L2"}]
            })))
            .mount(&server)
            .await;
        assert_eq!(
            g.list_tasklists().await.unwrap(),
            vec![
                TaskList { id: "L1".into(), title: "내 할 일".into() },
                TaskList { id: "L2".into(), title: "(제목 없음)".into() },
            ]
        );
    }

    #[tokio::test]
    async fn list_tasks_requests_open_tasks_trims_due_and_handles_empty_title() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/tasks/v1/lists/L1/tasks"))
            .and(query_param("showCompleted", "false"))
            .and(query_param("showHidden", "false"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"id": "t1", "title": "보고서", "due": "2026-09-30T00:00:00.000Z", "status": "needsAction", "notes": "초안"},
                    {"id": "t2", "title": "", "status": "needsAction"},
                    {"id": "t3", "title": "끝남", "status": "completed"}
                ]
            })))
            .mount(&server)
            .await;
        assert_eq!(
            g.list_tasks("L1").await.unwrap(),
            vec![
                Task { id: "t1".into(), list_id: "L1".into(), title: "보고서".into(), due: Some("2026-09-30".into()), notes: Some("초안".into()) },
                Task { id: "t2".into(), list_id: "L1".into(), title: "(제목 없음)".into(), due: None, notes: None },
            ]
        );
    }

    #[tokio::test]
    async fn list_tasks_follows_pagination() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/tasks/v1/lists/L1/tasks"))
            .and(query_param("pageToken", "n2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": [{"id": "b", "title": "B"}]})))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/tasks/v1/lists/L1/tasks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": [{"id": "a", "title": "A"}], "nextPageToken": "n2"})))
            .mount(&server)
            .await;
        let ids: Vec<_> = g.list_tasks("L1").await.unwrap().into_iter().map(|t| t.id).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn set_completed_patches_status() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("PATCH"))
            .and(path("/tasks/v1/lists/L1/tasks/T1"))
            .and(body_json(json!({"status": "completed"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "T1"})))
            .expect(1)
            .mount(&server)
            .await;
        g.set_task_completed("L1", "T1", true).await.unwrap();
    }

    #[tokio::test]
    async fn set_uncompleted_clears_completed_date() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("PATCH"))
            .and(path("/tasks/v1/lists/L1/tasks/T1"))
            .and(body_json(json!({"status": "needsAction", "completed": null})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "T1"})))
            .expect(1)
            .mount(&server)
            .await;
        g.set_task_completed("L1", "T1", false).await.unwrap();
    }
}

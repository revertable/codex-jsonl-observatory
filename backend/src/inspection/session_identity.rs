use std::io::{self, BufRead, BufReader, Read};

use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionClassification {
    GuardianReview,
    Unclassified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionSourceIdentity {
    Named(String),
    Subagent(String),
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionIdentity {
    pub thread_id: Option<String>,
    pub session_id: Option<String>,
    pub parent_thread_id: Option<String>,
    pub originator: Option<String>,
    pub thread_source: Option<String>,
    pub source: SessionSourceIdentity,
    pub history_mode: Option<String>,
    pub subagent_history_start_ordinal: Option<u64>,
}

impl SessionIdentity {
    fn classify(&self) -> SessionClassification {
        let is_guardian_thread = self.thread_source.as_deref() == Some("guardian_review");
        let is_guardian_subagent =
            matches!(&self.source, SessionSourceIdentity::Subagent(kind) if kind == "guardian");

        if is_guardian_thread || is_guardian_subagent {
            SessionClassification::GuardianReview
        } else {
            SessionClassification::Unclassified
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionInspection {
    pub identity: Option<SessionIdentity>,
    pub classification: SessionClassification,
}

impl SessionInspection {
    fn unclassified() -> Self {
        Self {
            identity: None,
            classification: SessionClassification::Unclassified,
        }
    }
}

pub fn inspect_str(input: &str) -> SessionInspection {
    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        if string_field(&value, "type") != Some("session_meta") {
            continue;
        }

        let payload = value.get("payload").unwrap_or(&value);
        let identity = extract_identity(payload);
        let classification = identity.classify();

        return SessionInspection {
            identity: Some(identity),
            classification,
        };
    }

    SessionInspection::unclassified()
}

pub fn inspect_reader<R: Read>(reader: R) -> io::Result<SessionInspection> {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();

    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            return Ok(SessionInspection::unclassified());
        }

        let inspection = inspect_str(&String::from_utf8_lossy(&line));
        if inspection.identity.is_some() {
            return Ok(inspection);
        }
    }
}

fn extract_identity(payload: &Value) -> SessionIdentity {
    SessionIdentity {
        thread_id: owned_string_field(payload, "id"),
        session_id: owned_string_field(payload, "session_id"),
        parent_thread_id: owned_string_field(payload, "parent_thread_id"),
        originator: owned_string_field(payload, "originator"),
        thread_source: owned_string_field(payload, "thread_source"),
        source: extract_source(payload),
        history_mode: owned_string_field(payload, "history_mode"),
        subagent_history_start_ordinal: payload
            .get("subagent_history_start_ordinal")
            .and_then(Value::as_u64),
    }
}

fn extract_source(payload: &Value) -> SessionSourceIdentity {
    let Some(source) = payload.get("source") else {
        return SessionSourceIdentity::Unknown;
    };

    if let Some(name) = non_empty_string(source) {
        return SessionSourceIdentity::Named(name.to_owned());
    }

    let Some(subagent) = source.get("subagent") else {
        return SessionSourceIdentity::Unknown;
    };

    if let Some(kind) = non_empty_string(subagent) {
        return SessionSourceIdentity::Subagent(kind.to_owned());
    }

    subagent
        .get("other")
        .and_then(non_empty_string)
        .map(|kind| SessionSourceIdentity::Subagent(kind.to_owned()))
        .unwrap_or(SessionSourceIdentity::Unknown)
}

fn owned_string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(non_empty_string)
        .map(str::to_owned)
}

fn non_empty_string(value: &Value) -> Option<&str> {
    value
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field)?.as_str()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn extracts_guardian_review_session_identity() {
        let inspection = inspect_str(
            r#"{"timestamp":"2026-09-10T02:36:12Z","ordinal":0,"type":"session_meta","payload":{"id":"guardian-thread","session_id":"root-session","parent_thread_id":"parent-thread","originator":"codex_work_desktop","thread_source":"guardian_review","source":{"subagent":{"other":"guardian"}},"history_mode":"paginated","subagent_history_start_ordinal":110}}"#,
        );

        assert_eq!(
            inspection,
            SessionInspection {
                identity: Some(SessionIdentity {
                    thread_id: Some("guardian-thread".to_owned()),
                    session_id: Some("root-session".to_owned()),
                    parent_thread_id: Some("parent-thread".to_owned()),
                    originator: Some("codex_work_desktop".to_owned()),
                    thread_source: Some("guardian_review".to_owned()),
                    source: SessionSourceIdentity::Subagent("guardian".to_owned()),
                    history_mode: Some("paginated".to_owned()),
                    subagent_history_start_ordinal: Some(110),
                }),
                classification: SessionClassification::GuardianReview,
            }
        );
    }

    #[test]
    fn thread_source_alone_identifies_guardian_review() {
        let inspection = inspect_str(
            r#"{"type":"session_meta","payload":{"thread_source":"guardian_review","source":"app_server"}}"#,
        );

        assert_eq!(
            inspection.classification,
            SessionClassification::GuardianReview
        );
    }

    #[test]
    fn guardian_subagent_source_alone_identifies_guardian_review() {
        let inspection = inspect_str(
            r#"{"type":"session_meta","payload":{"source":{"subagent":{"other":"guardian"}}}}"#,
        );

        assert_eq!(
            inspection.classification,
            SessionClassification::GuardianReview
        );
    }

    #[test]
    fn named_guardian_subagent_source_is_supported() {
        let inspection =
            inspect_str(r#"{"type":"session_meta","payload":{"source":{"subagent":"guardian"}}}"#);

        assert_eq!(
            inspection.classification,
            SessionClassification::GuardianReview
        );
    }

    #[test]
    fn ordinary_cli_session_remains_unclassified() {
        let inspection = inspect_str(
            r#"{"type":"session_meta","payload":{"id":"cli-thread","originator":"codex_cli","thread_source":"user","source":"cli"}}"#,
        );

        assert_eq!(
            inspection.classification,
            SessionClassification::Unclassified
        );
        assert_eq!(
            inspection.identity.map(|identity| identity.source),
            Some(SessionSourceIdentity::Named("cli".to_owned()))
        );
    }

    #[test]
    fn ordinary_desktop_session_remains_unclassified() {
        let inspection = inspect_str(
            r#"{"type":"session_meta","payload":{"id":"desktop-thread","originator":"codex_work_desktop","thread_source":"user","source":"vscode"}}"#,
        );

        assert_eq!(
            inspection.classification,
            SessionClassification::Unclassified
        );
    }

    #[test]
    fn other_subagents_and_near_matches_are_not_guardian_reviews() {
        for source in [
            r#"{"subagent":{"other":"worker"}}"#,
            r#"{"subagent":{"other":"guardian_helper"}}"#,
            r#"{"subagent":"review"}"#,
        ] {
            let input = format!(
                r#"{{"type":"session_meta","payload":{{"parent_thread_id":"parent","source":{source}}}}}"#,
            );

            assert_eq!(
                inspect_str(&input).classification,
                SessionClassification::Unclassified
            );
        }
    }

    #[test]
    fn malformed_or_missing_session_metadata_is_unclassified() {
        let inspection = inspect_str(
            "not json\n{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}",
        );

        assert_eq!(inspection, SessionInspection::unclassified());
    }

    #[test]
    fn reader_inspection_skips_unrelated_records_before_session_metadata() {
        let input = concat!(
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n",
            "{\"type\":\"session_meta\",\"payload\":{\"thread_source\":\"guardian_review\"}}",
        );

        let inspection = inspect_reader(Cursor::new(input)).expect("inspection succeeds");

        assert_eq!(
            inspection.classification,
            SessionClassification::GuardianReview
        );
    }

    #[test]
    fn reader_inspection_preserves_lossy_handling_before_session_metadata() {
        let input = b"\xff\n{\"type\":\"session_meta\",\"payload\":{\"id\":\"thread\"}}";

        let inspection = inspect_reader(Cursor::new(input)).expect("inspection succeeds");

        assert_eq!(
            inspection
                .identity
                .and_then(|identity| identity.thread_id)
                .as_deref(),
            Some("thread")
        );
    }
}

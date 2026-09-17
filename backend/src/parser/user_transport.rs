use std::borrow::Cow;

use serde_json::Value;

use crate::domain::ReferencedConversation;

const REFERENCE_HEADING: &str = "## Referenced ChatGPT conversation:";
const REQUEST_HEADING: &str = "## My request:";
const AMBIENT_OPEN: &str = "<in-app-browser-context source=\"ambient-ui-state\">";
const AMBIENT_CLOSE: &str = "</in-app-browser-context>";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedUserTransport {
    HumanRequest {
        request: String,
        reference: Option<ReferencedConversation>,
    },
    DelegatedHandoff {
        reference: ReferencedConversation,
    },
}

pub(crate) enum BorrowedUserTransport<'a> {
    HumanRequest {
        request: Cow<'a, str>,
        reference: Option<ReferencedConversation>,
    },
    DelegatedHandoff {
        reference: ReferencedConversation,
    },
}

pub fn decode_user_transport(content: &str) -> Option<DecodedUserTransport> {
    decode_user_transport_borrowed(content).map(|decoded| match decoded {
        BorrowedUserTransport::HumanRequest { request, reference } => {
            DecodedUserTransport::HumanRequest {
                request: request.into_owned(),
                reference,
            }
        }
        BorrowedUserTransport::DelegatedHandoff { reference } => {
            DecodedUserTransport::DelegatedHandoff { reference }
        }
    })
}

pub(crate) fn decode_user_transport_borrowed(content: &str) -> Option<BorrowedUserTransport<'_>> {
    if content.contains("\r\n") {
        let normalized = content.replace("\r\n", "\n");
        return decode_normalized_user_transport(&normalized).map(|decoded| match decoded {
            BorrowedUserTransport::HumanRequest { request, reference } => {
                BorrowedUserTransport::HumanRequest {
                    request: Cow::Owned(request.into_owned()),
                    reference,
                }
            }
            BorrowedUserTransport::DelegatedHandoff { reference } => {
                BorrowedUserTransport::DelegatedHandoff { reference }
            }
        });
    }

    decode_normalized_user_transport(content)
}

fn decode_normalized_user_transport(content: &str) -> Option<BorrowedUserTransport<'_>> {
    decode_chatgpt_handoff(content).or_else(|| decode_ambient_ui_request(content))
}

fn decode_chatgpt_handoff(content: &str) -> Option<BorrowedUserTransport<'_>> {
    let body = content.trim().strip_prefix(REFERENCE_HEADING)?;
    let request_boundary = format!("\n{REQUEST_HEADING}\n");
    let (reference_section, request) = body.split_once(&request_boundary)?;
    let request = request.trim();
    if request.is_empty() {
        return None;
    }

    let json_start = reference_section.find('{')?;
    let reference_value =
        serde_json::from_str::<Value>(reference_section[json_start..].trim()).ok()?;
    let reference_object = reference_value.as_object()?;
    let reference = ReferencedConversation {
        conversation_id: optional_non_empty_string(reference_object.get("conversationId")),
        title: optional_non_empty_string(reference_object.get("title")),
        preview_available: has_valid_preview_message(reference_object.get("priorConversation")),
    };

    if matches_delegated_handoff(request, &reference) {
        return Some(BorrowedUserTransport::DelegatedHandoff { reference });
    }

    Some(BorrowedUserTransport::HumanRequest {
        request: Cow::Borrowed(request),
        reference: Some(reference),
    })
}

fn decode_ambient_ui_request(content: &str) -> Option<BorrowedUserTransport<'_>> {
    let body = content.trim();
    let after_open = body.strip_prefix(AMBIENT_OPEN)?;
    let (_, after_close) = after_open.split_once(AMBIENT_CLOSE)?;
    let after_close = after_close.trim_start();
    let request = after_close.strip_prefix(REQUEST_HEADING)?;
    let request = request.strip_prefix('\n')?.trim();
    if request.is_empty() {
        return None;
    }

    Some(BorrowedUserTransport::HumanRequest {
        request: Cow::Borrowed(request),
        reference: None,
    })
}

fn matches_delegated_handoff(request: &str, reference: &ReferencedConversation) -> bool {
    let (Some(title), Some(conversation_id)) = (
        reference.title.as_deref(),
        reference.conversation_id.as_deref(),
    ) else {
        return false;
    };
    let expected = format!("Continuing from [{title}](chatgpt-conversation://{conversation_id}):");
    request
        .strip_prefix(&expected)
        .is_some_and(|delegated_task| {
            delegated_task
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
                && !delegated_task.trim().is_empty()
        })
}

fn has_valid_preview_message(prior_conversation: Option<&Value>) -> bool {
    prior_conversation
        .and_then(Value::as_object)
        .and_then(|prior| prior.get("conversation"))
        .and_then(Value::as_array)
        .is_some_and(|conversation| conversation.iter().any(is_valid_preview_message))
}

fn is_valid_preview_message(message: &Value) -> bool {
    let Some(message) = message.as_object() else {
        return false;
    };
    optional_non_empty_string(message.get("role")).is_some()
        && message
            .get("content")
            .is_some_and(has_non_empty_message_content)
}

fn has_non_empty_message_content(content: &Value) -> bool {
    match content {
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(parts) => parts.iter().any(|part| {
            part.as_object()
                .and_then(|part| part.get("text"))
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
        }),
        _ => false,
    }
}

fn optional_non_empty_string(value: Option<&Value>) -> Option<String> {
    value?
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handoff(reference: &str, request: &str) -> String {
        format!(
            "{REFERENCE_HEADING}\nTransport guidance.\n{reference}\n{REQUEST_HEADING}\n{request}"
        )
    }

    fn reference(title: &str, id: &str, prior: &str) -> String {
        format!(r#"{{"conversationId":"{id}","title":"{title}","priorConversation":{prior}}}"#)
    }

    #[test]
    fn matching_reference_identifies_delegated_handoff_without_preview_body() {
        let decoded = decode_user_transport(&handoff(
            &reference(
                "Design discussion",
                "conversation-1",
                r#"{"conversation":[{"role":"user","content":[{"text":"Earlier request"}]}]}"#,
            ),
            "Continuing from [Design discussion](chatgpt-conversation://conversation-1): implement it.",
        ))
        .expect("handoff decoded");

        assert_eq!(
            decoded,
            DecodedUserTransport::DelegatedHandoff {
                reference: ReferencedConversation {
                    conversation_id: Some("conversation-1".to_owned()),
                    title: Some("Design discussion".to_owned()),
                    preview_available: true,
                }
            }
        );
    }

    #[test]
    fn mismatched_title_or_id_preserves_request_as_human_authored() {
        for request in [
            "Continuing from [Other title](chatgpt-conversation://conversation-1): continue.",
            "Continuing from [Design discussion](chatgpt-conversation://other-id): continue.",
            "Continuing from [Design discussion](chatgpt-conversation://conversation-1):",
            "Continuing from [Design discussion](chatgpt-conversation://conversation-1):not an observed envelope",
        ] {
            let decoded = decode_user_transport(&handoff(
                &reference("Design discussion", "conversation-1", "null"),
                request,
            ))
            .expect("reference envelope decoded");

            assert!(matches!(
                decoded,
                DecodedUserTransport::HumanRequest { request: value, .. } if value == request
            ));
        }
    }

    #[test]
    fn partial_preview_only_records_availability() {
        let decoded = decode_user_transport(&handoff(
            &reference(
                "Design discussion",
                "conversation-1",
                r#"{"conversation":[{"role":"assistant","content":null},{"role":"user","content":[{"text":"Valid"}]}]}"#,
            ),
            "Continue.",
        ))
        .expect("handoff decoded");

        let DecodedUserTransport::HumanRequest {
            reference: Some(reference),
            ..
        } = decoded
        else {
            panic!("expected human request with reference");
        };
        assert!(reference.preview_available);
    }

    #[test]
    fn null_or_invalid_preview_is_unavailable() {
        for prior in [
            "null",
            r#"{"conversation":[{"role":"assistant","content":null},{"content":[{"text":"missing role"}]}]}"#,
        ] {
            let decoded = decode_user_transport(&handoff(
                &reference("Design discussion", "conversation-1", prior),
                "Continue.",
            ))
            .expect("handoff decoded");
            let DecodedUserTransport::HumanRequest {
                reference: Some(reference),
                ..
            } = decoded
            else {
                panic!("expected reference");
            };
            assert!(!reference.preview_available);
        }
    }

    #[test]
    fn exact_ambient_envelope_yields_only_the_request() {
        let decoded = decode_user_transport(concat!(
            "<in-app-browser-context source=\"ambient-ui-state\">\n",
            "Automatically supplied state.\n",
            "</in-app-browser-context>\n\n",
            "## My request:\n",
            "이어서 작업해줘"
        ));

        assert_eq!(
            decoded,
            Some(DecodedUserTransport::HumanRequest {
                request: "이어서 작업해줘".to_owned(),
                reference: None,
            })
        );
    }

    #[test]
    fn ambient_variants_and_malformed_reference_fall_back() {
        assert!(
            decode_user_transport(concat!(
                "<in-app-browser-context source=\"other\">state</in-app-browser-context>\n",
                "## My request:\nContinue."
            ))
            .is_none()
        );
        assert!(
            decode_user_transport(concat!(
                "<in-app-browser-context source=\"ambient-ui-state\">state\n",
                "## My request:\nContinue."
            ))
            .is_none()
        );
        assert!(decode_user_transport(&handoff("{not-json}", "Continue.")).is_none());
    }
}

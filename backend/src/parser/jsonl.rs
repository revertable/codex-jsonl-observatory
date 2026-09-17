use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;

use crate::domain::{
    ParsedChatLog, ReferencedConversation, RenderedEntry, RenderedEntryKind, SessionProvenance,
};

use super::user_transport::{DecodedUserTransport, decode_user_transport};

#[derive(Clone, Debug)]
struct ParsedCandidate {
    entry: RenderedEntry,
    stable_key: Option<String>,
    normalized_text: String,
    source: CandidateSource,
    timestamp: Option<String>,
    original_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateSource {
    ResponseMessage,
    ResponseToolResult,
    EventToolResult,
    ResponseToolCall,
    EventUserMessage,
    EventAgentMessage,
    EventSystem,
    Fallback,
}

impl CandidateSource {
    fn priority(self) -> u8 {
        match self {
            Self::ResponseMessage | Self::ResponseToolResult => 4,
            Self::EventToolResult | Self::ResponseToolCall | Self::EventUserMessage => 3,
            Self::EventAgentMessage | Self::EventSystem => 2,
            Self::Fallback => 1,
        }
    }
}

struct CandidateSeed {
    entry: RenderedEntry,
    source: CandidateSource,
    stable_id: Option<String>,
    timestamp: Option<String>,
    is_user_message: bool,
}

pub fn parse_str(input: &str) -> ParsedChatLog {
    parse_lossy_lines(input)
}

pub fn parse_reader<R: Read>(mut reader: R) -> io::Result<ParsedChatLog> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(parse_lossy_lines(&String::from_utf8_lossy(&bytes)))
}

pub fn parse_file(path: impl AsRef<Path>) -> io::Result<ParsedChatLog> {
    let path = path.as_ref();
    if !path.is_file() {
        return Ok(ParsedChatLog::empty());
    }

    parse_reader(File::open(path)?)
}

fn parse_lossy_lines(input: &str) -> ParsedChatLog {
    let mut candidates = Vec::new();
    let mut session_provenance = SessionProvenance::default();
    let mut parsed_candidates = 0;
    let mut ignored_lines = 0;
    let mut malformed_lines = 0;
    let mut observed_event_counts = IndexMap::new();
    let mut next_candidate_index = 0;

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let value = match serde_json::from_str::<Value>(line) {
            Ok(value) => value,
            Err(_) => {
                malformed_lines += 1;
                continue;
            }
        };

        increment_observed_event_count(&mut observed_event_counts, &value);

        let extracted = extract_candidate_seeds(&value);
        if extracted.is_empty() {
            ignored_lines += 1;
        } else {
            parsed_candidates += extracted.len();
            for seed in extracted {
                let (candidate, reference) = parsed_candidate(seed, next_candidate_index);
                next_candidate_index += 1;
                if let Some(reference) = reference {
                    session_provenance.observe_reference(reference);
                }
                if let Some(candidate) = candidate {
                    candidates.push(candidate);
                }
            }
        }
    }

    let candidates = dedupe_candidates(candidates);
    let entry_timestamps = candidates
        .iter()
        .map(|candidate| candidate.timestamp.clone())
        .collect();
    let entries = candidates
        .into_iter()
        .map(|candidate| candidate.entry)
        .collect();

    ParsedChatLog {
        parsed_candidates,
        entries,
        entry_timestamps,
        session_provenance,
        ignored_lines,
        malformed_lines,
        observed_event_counts,
    }
}

fn increment_observed_event_count(counts: &mut IndexMap<String, usize>, value: &Value) {
    let top_type = string_field(value, "type");
    let payload_type = value
        .get("payload")
        .and_then(|payload| string_field(payload, "type"));

    let key = match (top_type, payload_type) {
        (Some(top_type), Some(payload_type)) => format!("{top_type}/{payload_type}"),
        (None, Some(payload_type)) => payload_type.to_owned(),
        (Some(top_type), None) => top_type.to_owned(),
        (None, None) => "unknown".to_owned(),
    };

    *counts.entry(key).or_insert(0) += 1;
}

fn parsed_candidate(
    seed: CandidateSeed,
    original_index: usize,
) -> (Option<ParsedCandidate>, Option<ReferencedConversation>) {
    let (entry, reference) = if seed.is_user_message {
        match decode_user_transport(&seed.entry.content) {
            Some(DecodedUserTransport::HumanRequest { request, reference }) => {
                (Some(classify_user_message(&request)), reference)
            }
            Some(DecodedUserTransport::DelegatedHandoff { reference }) => (None, Some(reference)),
            None => (Some(seed.entry), None),
        }
    } else {
        (Some(seed.entry), None)
    };
    let Some(entry) = entry else {
        return (None, reference);
    };
    let normalized_text = normalize_text(&entry.content);

    let candidate = ParsedCandidate {
        normalized_text,
        stable_key: seed
            .stable_id
            .map(|stable_id| format!("{}:{stable_id}", rendered_kind_key(entry.kind))),
        entry,
        source: seed.source,
        timestamp: seed.timestamp,
        original_index,
    };
    (Some(candidate), reference)
}

fn extract_candidate_seeds(value: &Value) -> Vec<CandidateSeed> {
    match extract_focused_seed(value) {
        Some(Some(entry)) => vec![entry],
        Some(None) => Vec::new(),
        None => extract_fallback_seeds(value),
    }
}

fn extract_focused_seed(value: &Value) -> Option<Option<CandidateSeed>> {
    let top_type = string_field(value, "type")?;
    if top_type == "session_meta" {
        let payload = value.get("payload").unwrap_or(value);
        return Some(extract_session_meta(payload).map(|entry| CandidateSeed {
            entry,
            source: CandidateSource::EventSystem,
            stable_id: stable_id(payload),
            timestamp: timestamp(value).or_else(|| timestamp(payload)),
            is_user_message: false,
        }));
    }

    let payload = value.get("payload")?;
    let payload_type = string_field(payload, "type");

    let source = match (top_type, payload_type) {
        ("event_msg", Some("user_message")) => CandidateSource::EventUserMessage,
        ("event_msg", Some("agent_message")) => CandidateSource::EventAgentMessage,
        ("event_msg", Some("exec_command_end" | "patch_apply_end")) => {
            CandidateSource::EventToolResult
        }
        ("event_msg", Some("task_started" | "task_complete")) => CandidateSource::EventSystem,
        ("response_item", Some("message")) => CandidateSource::ResponseMessage,
        ("response_item", Some("function_call" | "custom_tool_call")) => {
            CandidateSource::ResponseToolCall
        }
        ("response_item", Some("function_call_output" | "custom_tool_call_output")) => {
            CandidateSource::ResponseToolResult
        }
        _ => return None,
    };

    let entry = match (top_type, payload_type) {
        ("event_msg", Some("user_message")) => string_field(payload, "message")
            .map(str::trim)
            .filter(|content| !content.is_empty())
            .map(classify_user_message),
        ("event_msg", Some("agent_message")) => string_field(payload, "message")
            .map(str::trim)
            .filter(|content| !content.is_empty())
            .map(|content| RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: content.to_owned(),
            }),
        ("event_msg", Some("exec_command_end")) => extract_exec_command_end(payload),
        ("event_msg", Some("patch_apply_end")) => extract_patch_apply_end(payload),
        ("event_msg", Some("task_started")) => {
            Some(extract_task_lifecycle("Task started", payload))
        }
        ("event_msg", Some("task_complete")) => {
            Some(extract_task_lifecycle("Task complete", payload))
        }
        ("response_item", Some("message")) => extract_response_message(payload),
        ("response_item", Some("function_call")) => extract_tool_call(payload, "Function call"),
        ("response_item", Some("custom_tool_call")) => {
            extract_tool_call(payload, "Custom tool call")
        }
        ("response_item", Some("function_call_output")) => {
            extract_tool_result(payload, "Function call output")
        }
        ("response_item", Some("custom_tool_call_output")) => {
            extract_tool_result(payload, "Custom tool call output")
        }
        _ => unreachable!("focused source already matched supported envelope types"),
    };

    Some(entry.map(|entry| CandidateSeed {
        is_user_message: matches!(
            (top_type, payload_type),
            ("event_msg", Some("user_message"))
        ) || matches!(
            (top_type, payload_type, string_field(payload, "role")),
            ("response_item", Some("message"), Some("user"))
        ),
        entry,
        source,
        stable_id: focused_stable_id(payload_type, payload),
        timestamp: timestamp(value).or_else(|| timestamp(payload)),
    }))
}

fn extract_fallback_seeds(value: &Value) -> Vec<CandidateSeed> {
    let mut seeds = Vec::new();

    if let Some(seed) = extract_fallback_seed(value) {
        seeds.push(seed);
    }

    for field in ["message", "item", "delta"] {
        if let Some(seed) = value.get(field).and_then(extract_fallback_seed) {
            seeds.push(seed);
        }
    }

    if let Some(output) = value.get("output").and_then(Value::as_array) {
        seeds.extend(output.iter().filter_map(extract_fallback_seed));
    }

    seeds
}

fn extract_fallback_seed(value: &Value) -> Option<CandidateSeed> {
    extract_role_based_entry(value)
        .or_else(|| extract_type_based_entry(value))
        .map(|entry| CandidateSeed {
            is_user_message: is_fallback_user_message(value),
            entry,
            source: CandidateSource::Fallback,
            stable_id: stable_id(value),
            timestamp: timestamp(value),
        })
}

fn is_fallback_user_message(value: &Value) -> bool {
    string_field(value, "role") == Some("user")
        || string_field(value, "type")
            .is_some_and(|type_name| type_name.to_ascii_lowercase().contains("user"))
}

fn extract_role_based_entry(value: &Value) -> Option<RenderedEntry> {
    let role = string_field(value, "role")?;
    let content = extract_text(value)?.trim().to_owned();
    if content.is_empty() {
        return None;
    }

    if role == "user" {
        return Some(classify_user_message(&content));
    }

    let kind = match role {
        "assistant" | "model" => RenderedEntryKind::Codex,
        "tool" => RenderedEntryKind::ToolResult,
        "system" => RenderedEntryKind::System,
        _ => return None,
    };

    Some(RenderedEntry { kind, content })
}

fn extract_type_based_entry(value: &Value) -> Option<RenderedEntry> {
    let type_name = string_field(value, "type")?;
    let normalized_type = type_name.to_ascii_lowercase();
    let content = fallback_type_content(value, &normalized_type)?;

    if normalized_type.contains("user") {
        return Some(classify_user_message(&content));
    }

    let kind = if normalized_type.contains("assistant")
        || normalized_type.contains("model")
        || normalized_type.contains("agent")
    {
        RenderedEntryKind::Codex
    } else if normalized_type.contains("system") || normalized_type.contains("session") {
        RenderedEntryKind::System
    } else if is_tool_call_type(&normalized_type) {
        RenderedEntryKind::ToolCall
    } else if is_tool_result_type(&normalized_type) {
        RenderedEntryKind::ToolResult
    } else {
        return None;
    };

    Some(RenderedEntry { kind, content })
}

fn fallback_type_content(value: &Value, normalized_type: &str) -> Option<String> {
    if is_tool_call_type(normalized_type) {
        if let Some(name) = string_field(value, "name")
            .or_else(|| string_field(value, "call_id"))
            .or_else(|| string_field(value, "id"))
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            let mut content = format!("Tool call: {name}");
            if let Some(arguments) = extract_tool_arguments(value) {
                content.push('\n');
                content.push_str(&arguments);
            }
            return Some(content);
        }
    }

    extract_text(value)
        .map(|content| content.trim().to_owned())
        .filter(|content| !content.is_empty())
}

fn is_tool_call_type(normalized_type: &str) -> bool {
    (normalized_type.contains("tool_call")
        || normalized_type.contains("function_call")
        || normalized_type.contains("command"))
        && !normalized_type.contains("output")
        && !normalized_type.contains("result")
        && !normalized_type.contains("end")
}

fn is_tool_result_type(normalized_type: &str) -> bool {
    normalized_type.contains("tool_result")
        || normalized_type.contains("function_result")
        || normalized_type.contains("command_result")
        || normalized_type.contains("output")
        || normalized_type.contains("end")
}

fn dedupe_candidates(candidates: Vec<ParsedCandidate>) -> Vec<ParsedCandidate> {
    let mut selected_by_stable_key: IndexMap<String, ParsedCandidate> = IndexMap::new();
    let mut pending = Vec::new();

    for candidate in candidates {
        if let Some(stable_key) = candidate.stable_key.clone() {
            match selected_by_stable_key.get_mut(&stable_key) {
                Some(selected) => {
                    if should_replace_candidate(selected, &candidate) {
                        let original_index = selected.original_index;
                        *selected = candidate;
                        selected.original_index = original_index;
                    }
                }
                None => {
                    selected_by_stable_key.insert(stable_key, candidate);
                }
            }
        } else {
            pending.push(candidate);
        }
    }

    let mut combined = pending;
    combined.extend(selected_by_stable_key.into_values());
    combined.sort_by_key(|candidate| candidate.original_index);

    suppress_adjacent_duplicates(combined)
}

fn suppress_adjacent_duplicates(candidates: Vec<ParsedCandidate>) -> Vec<ParsedCandidate> {
    let mut deduped: Vec<ParsedCandidate> = Vec::new();

    for candidate in candidates {
        if let Some(previous) = deduped.last_mut() {
            if should_suppress_adjacent_duplicate(previous, &candidate) {
                if should_replace_candidate(previous, &candidate) {
                    let original_index = previous.original_index;
                    *previous = candidate;
                    previous.original_index = original_index;
                }
                continue;
            }
        }

        deduped.push(candidate);
    }

    deduped
}

fn should_suppress_adjacent_duplicate(
    previous: &ParsedCandidate,
    candidate: &ParsedCandidate,
) -> bool {
    if previous.entry.kind != candidate.entry.kind
        || previous.normalized_text != candidate.normalized_text
    {
        return false;
    }

    if let (Some(previous_key), Some(candidate_key)) = (&previous.stable_key, &candidate.stable_key)
        && previous_key != candidate_key
    {
        return false;
    }

    if previous.timestamp.is_some()
        && candidate.timestamp.is_some()
        && previous.timestamp == candidate.timestamp
    {
        return true;
    }

    is_known_duplicate_source_pair(previous.source, candidate.source)
}

fn is_known_duplicate_source_pair(left: CandidateSource, right: CandidateSource) -> bool {
    matches!(
        (left, right),
        (
            CandidateSource::EventUserMessage,
            CandidateSource::ResponseMessage
        ) | (
            CandidateSource::ResponseMessage,
            CandidateSource::EventUserMessage
        ) | (
            CandidateSource::EventAgentMessage,
            CandidateSource::ResponseMessage
        ) | (
            CandidateSource::ResponseMessage,
            CandidateSource::EventAgentMessage
        ) | (
            CandidateSource::EventToolResult,
            CandidateSource::ResponseToolResult
        ) | (
            CandidateSource::ResponseToolResult,
            CandidateSource::EventToolResult
        )
    )
}

fn should_replace_candidate(current: &ParsedCandidate, candidate: &ParsedCandidate) -> bool {
    let current_priority = current.source.priority();
    let candidate_priority = candidate.source.priority();

    candidate_priority > current_priority
        || (candidate_priority == current_priority
            && candidate.entry.content.len() > current.entry.content.len())
}

fn focused_stable_id(payload_type: Option<&str>, payload: &Value) -> Option<String> {
    match payload_type {
        Some(payload_type @ ("task_started" | "task_complete")) => {
            stable_id(payload).map(|stable_id| format!("{payload_type}:{stable_id}"))
        }
        _ => stable_id(payload),
    }
}

fn stable_id(value: &Value) -> Option<String> {
    for field in ["id", "call_id", "turn_id", "turnId"] {
        if let Some(id) = string_field(value, field)
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            return Some(id.to_owned());
        }
    }

    None
}

fn timestamp(value: &Value) -> Option<String> {
    for field in ["timestamp", "time", "created_at", "createdAt"] {
        if let Some(timestamp) = string_field(value, field)
            .map(str::trim)
            .filter(|timestamp| !timestamp.is_empty())
        {
            return Some(timestamp.to_owned());
        }
    }

    None
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase()
}

fn rendered_kind_key(kind: RenderedEntryKind) -> &'static str {
    match kind {
        RenderedEntryKind::Context => "Context",
        RenderedEntryKind::Task => "Task",
        RenderedEntryKind::You => "You",
        RenderedEntryKind::Codex => "Codex",
        RenderedEntryKind::ToolCall => "ToolCall",
        RenderedEntryKind::ToolResult => "ToolResult",
        RenderedEntryKind::System => "System",
    }
}

fn extract_exec_command_end(payload: &Value) -> Option<RenderedEntry> {
    let mut lines = vec!["Exec command finished".to_owned()];

    if let Some(command) = string_field(payload, "command")
        .or_else(|| string_field(payload, "cmd"))
        .map(str::trim)
        .filter(|command| !command.is_empty())
    {
        lines.push(format!("Command: {command}"));
    }

    if let Some(status) = string_field(payload, "status")
        .map(str::trim)
        .filter(|status| !status.is_empty())
    {
        lines.push(format!("Status: {status}"));
    }

    if let Some(exit_code) = value_field_to_string(payload, "exit_code")
        .or_else(|| value_field_to_string(payload, "exitCode"))
    {
        lines.push(format!("Exit code: {exit_code}"));
    }

    if let Some(output) = extract_first_text_field(
        payload,
        &["aggregated_output", "formatted_output", "stdout", "stderr"],
    ) {
        lines.push(output);
    }

    if lines.len() == 1 {
        None
    } else {
        Some(RenderedEntry {
            kind: RenderedEntryKind::ToolResult,
            content: lines.join("\n"),
        })
    }
}

fn extract_patch_apply_end(payload: &Value) -> Option<RenderedEntry> {
    let status = string_field(payload, "status")?.trim();
    if status.is_empty() {
        return None;
    }

    Some(RenderedEntry {
        kind: RenderedEntryKind::ToolResult,
        content: format!("Patch apply status: {status}"),
    })
}

fn extract_task_lifecycle(label: &str, payload: &Value) -> RenderedEntry {
    let mut content = label.to_owned();
    if let Some(turn_id) = string_field(payload, "turn_id")
        .or_else(|| string_field(payload, "turnId"))
        .map(str::trim)
        .filter(|turn_id| !turn_id.is_empty())
    {
        content.push('\n');
        content.push_str("Turn: ");
        content.push_str(turn_id);
    }

    RenderedEntry {
        kind: RenderedEntryKind::System,
        content,
    }
}

fn extract_session_meta(payload: &Value) -> Option<RenderedEntry> {
    let mut lines = Vec::new();

    if let Some(id) = string_field(payload, "id")
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        lines.push(format!("Session: {id}"));
    }

    if let Some(provider) = string_field(payload, "model_provider")
        .map(str::trim)
        .filter(|provider| !provider.is_empty())
    {
        lines.push(format!("Model provider: {provider}"));
    }

    if let Some(version) = string_field(payload, "cli_version")
        .map(str::trim)
        .filter(|version| !version.is_empty())
    {
        lines.push(format!("CLI version: {version}"));
    }

    if lines.is_empty() {
        None
    } else {
        Some(RenderedEntry {
            kind: RenderedEntryKind::System,
            content: lines.join("\n"),
        })
    }
}

fn extract_response_message(payload: &Value) -> Option<RenderedEntry> {
    let role = string_field(payload, "role")?;
    let content = extract_text(payload)?.trim().to_owned();
    if content.is_empty() {
        return None;
    }

    if role == "user" {
        return Some(classify_user_message(&content));
    }

    let kind = match role {
        "assistant" | "model" => RenderedEntryKind::Codex,
        "tool" => RenderedEntryKind::ToolResult,
        "system" => RenderedEntryKind::System,
        _ => return None,
    };

    Some(RenderedEntry { kind, content })
}

fn extract_tool_call(payload: &Value, label: &str) -> Option<RenderedEntry> {
    let name = string_field(payload, "name")
        .or_else(|| string_field(payload, "call_id"))
        .or_else(|| string_field(payload, "id"))?;

    let mut content = format!("{label}: {name}");
    if let Some(arguments) = extract_tool_arguments(payload) {
        content.push('\n');
        content.push_str(&arguments);
    }

    Some(RenderedEntry {
        kind: RenderedEntryKind::ToolCall,
        content,
    })
}

fn extract_tool_result(payload: &Value, label: &str) -> Option<RenderedEntry> {
    let content = extract_text(payload)
        .or_else(|| string_field(payload, "call_id").map(|call_id| format!("{label}: {call_id}")))?
        .trim()
        .to_owned();

    if content.is_empty() {
        None
    } else {
        Some(RenderedEntry {
            kind: RenderedEntryKind::ToolResult,
            content,
        })
    }
}

fn extract_tool_arguments(payload: &Value) -> Option<String> {
    if let Some(arguments) = payload.get("arguments") {
        return match arguments {
            Value::String(arguments) => {
                let arguments = arguments.trim();
                (!arguments.is_empty()).then(|| arguments.to_owned())
            }
            Value::Object(_) | Value::Array(_) => Some(arguments.to_string()),
            _ => None,
        };
    }

    string_field(payload, "input")
        .map(str::trim)
        .filter(|input| !input.is_empty())
        .map(str::to_owned)
        .or_else(|| extract_text(payload))
}

fn classify_user_message(content: &str) -> RenderedEntry {
    let trimmed = content.trim();
    let kind = classify_user_message_kind(trimmed);

    RenderedEntry {
        kind,
        content: match kind {
            RenderedEntryKind::Context => "AGENTS.md project instructions loaded".to_owned(),
            RenderedEntryKind::Task => "Task or prompt instructions loaded".to_owned(),
            _ => trimmed.to_owned(),
        },
    }
}

fn classify_user_message_kind(content: &str) -> RenderedEntryKind {
    if is_agents_instructions(content) {
        RenderedEntryKind::Context
    } else if is_structured_task_or_prompt(content) {
        RenderedEntryKind::Task
    } else {
        RenderedEntryKind::You
    }
}

fn is_agents_instructions(content: &str) -> bool {
    content.contains("# AGENTS.md")
        || content.contains("AGENTS.md instructions")
        || content.contains("<INSTRUCTIONS>")
}

fn is_structured_task_or_prompt(content: &str) -> bool {
    [
        "<environment_context>",
        "<user_instructions>",
        "<developer_context>",
        "<task>",
        "<prompt>",
    ]
    .iter()
    .any(|marker| content.contains(marker))
}

fn extract_text(value: &Value) -> Option<String> {
    for field in ["content", "text", "message", "output", "result", "summary"] {
        if let Some(raw_value) = value.get(field) {
            if let Some(text) = text_from_value(raw_value) {
                return Some(text);
            }
        }
    }

    None
}

fn extract_first_text_field(value: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| {
        value
            .get(*field)
            .and_then(text_from_value)
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
    })
}

fn text_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(values) => {
            let text = values
                .iter()
                .filter_map(text_from_value)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_owned();

            (!text.is_empty()).then_some(text)
        }
        Value::Object(_) => {
            if let Some(nested) = value.get("text").and_then(text_from_value) {
                return Some(nested);
            }

            for field in ["value", "content", "message"] {
                if let Some(nested) = value.get(field).and_then(text_from_value) {
                    return Some(nested);
                }
            }

            None
        }
        _ => None,
    }
}

fn value_field_to_string(value: &Value, field: &str) -> Option<String> {
    let value = value.get(field)?;
    match value {
        Value::String(text) => {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_owned())
        }
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field)?.as_str()
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor};

    use super::*;
    use crate::domain::ChatEntryFilter;

    fn event_user_message_line(message: &str) -> String {
        serde_json::json!({
            "type": "event_msg",
            "payload": {
                "type": "user_message",
                "message": message
            }
        })
        .to_string()
    }

    fn response_user_message_line(content: &str) -> String {
        serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": content
            }
        })
        .to_string()
    }

    fn event_agent_message_line(message: &str) -> String {
        serde_json::json!({
            "type": "event_msg",
            "payload": {
                "type": "agent_message",
                "message": message
            }
        })
        .to_string()
    }

    fn event_msg_line(payload_type: &str, fields: serde_json::Value) -> String {
        let mut payload = serde_json::Map::new();
        payload.insert(
            "type".to_owned(),
            serde_json::Value::String(payload_type.to_owned()),
        );

        let serde_json::Value::Object(fields) = fields else {
            panic!("test fields must be a JSON object");
        };

        payload.extend(fields);

        serde_json::json!({
            "type": "event_msg",
            "payload": payload
        })
        .to_string()
    }

    fn response_message_line(role: &str, content: &str) -> String {
        serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": role,
                "content": content
            }
        })
        .to_string()
    }

    fn response_item_line(payload_type: &str, fields: serde_json::Value) -> String {
        let mut payload = serde_json::Map::new();
        payload.insert(
            "type".to_owned(),
            serde_json::Value::String(payload_type.to_owned()),
        );

        let serde_json::Value::Object(fields) = fields else {
            panic!("test fields must be a JSON object");
        };

        payload.extend(fields);

        serde_json::json!({
            "type": "response_item",
            "payload": payload
        })
        .to_string()
    }

    fn temp_file_path(name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after Unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "codex-jsonl-observatory-{name}-{}-{unique}.jsonl",
            std::process::id(),
        ))
    }

    #[test]
    fn parses_event_msg_user_message_from_string() {
        let parsed = parse_str(
            r#"{"type":"event_msg","payload":{"type":"user_message","message":"hello"}}"#,
        );

        assert_eq!(parsed.malformed_lines, 0);
        assert_eq!(parsed.ignored_lines, 0);
        assert_eq!(parsed.parsed_candidates, 1);
        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::You,
                content: "hello".to_owned()
            }]
        );
    }

    #[test]
    fn parses_response_item_assistant_message_from_reader() {
        let input = Cursor::new(
            r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi from codex"}]}}"#,
        );

        let parsed = parse_reader(input).expect("reader parses");

        assert_eq!(parsed.parsed_candidates, 1);
        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: "hi from codex".to_owned()
            }]
        );
    }

    #[test]
    fn event_msg_agent_message_is_codex() {
        let parsed = parse_str(&event_agent_message_line("I can help with that."));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: "I can help with that.".to_owned()
            }]
        );
    }

    #[test]
    fn response_item_assistant_message_is_codex() {
        let parsed = parse_str(&response_message_line("assistant", "Assistant response"));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: "Assistant response".to_owned()
            }]
        );
    }

    #[test]
    fn response_item_model_message_is_codex() {
        let parsed = parse_str(&response_message_line("model", "Model response"));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: "Model response".to_owned()
            }]
        );
    }

    #[test]
    fn response_item_system_message_is_system() {
        let parsed = parse_str(&response_message_line("system", "System message"));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::System,
                content: "System message".to_owned()
            }]
        );
    }

    #[test]
    fn developer_messages_remain_ignored() {
        let parsed = parse_str(&response_message_line("developer", "Developer instruction"));

        assert_eq!(parsed.ignored_lines, 1);
        assert_eq!(parsed.parsed_candidates, 0);
        assert!(parsed.entries.is_empty());
    }

    #[test]
    fn event_msg_exec_command_end_is_tool_result() {
        let parsed = parse_str(&event_msg_line(
            "exec_command_end",
            serde_json::json!({
                "command": "cargo test",
                "status": "completed",
                "exit_code": 0,
                "aggregated_output": "test output"
            }),
        ));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::ToolResult,
                content:
                    "Exec command finished\nCommand: cargo test\nStatus: completed\nExit code: 0\ntest output"
                        .to_owned()
            }]
        );
    }

    #[test]
    fn event_msg_exec_command_end_uses_first_available_output() {
        let parsed = parse_str(&event_msg_line(
            "exec_command_end",
            serde_json::json!({
                "cmd": "cargo fmt",
                "formatted_output": "formatted",
                "stdout": "stdout",
                "stderr": "stderr"
            }),
        ));

        assert_eq!(parsed.entries[0].kind, RenderedEntryKind::ToolResult);
        assert_eq!(
            parsed.entries[0].content,
            "Exec command finished\nCommand: cargo fmt\nformatted"
        );
    }

    #[test]
    fn event_msg_patch_apply_end_is_tool_result() {
        let parsed = parse_str(&event_msg_line(
            "patch_apply_end",
            serde_json::json!({
                "status": "success"
            }),
        ));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::ToolResult,
                content: "Patch apply status: success".to_owned()
            }]
        );
    }

    #[test]
    fn event_msg_task_started_and_complete_are_system_entries() {
        let parsed = parse_str(
            &[
                event_msg_line("task_started", serde_json::json!({"turn_id": "turn_1"})),
                event_msg_line("task_complete", serde_json::json!({"turn_id": "turn_1"})),
            ]
            .join("\n"),
        );

        assert_eq!(
            parsed.entries,
            vec![
                RenderedEntry {
                    kind: RenderedEntryKind::System,
                    content: "Task started\nTurn: turn_1".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::System,
                    content: "Task complete\nTurn: turn_1".to_owned()
                }
            ]
        );
    }

    #[test]
    fn session_meta_is_system_entry() {
        let parsed = parse_str(
            r#"{"type":"session_meta","payload":{"id":"session_1","model_provider":"openai","cli_version":"1.2.3"}}"#,
        );

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::System,
                content: "Session: session_1\nModel provider: openai\nCLI version: 1.2.3"
                    .to_owned()
            }]
        );
    }

    #[test]
    fn new_envelopes_preserve_counts_for_ignored_and_observed_lines() {
        let parsed = parse_str(
            &[
                event_msg_line("exec_command_end", serde_json::json!({})),
                event_msg_line("patch_apply_end", serde_json::json!({"status": ""})),
                r#"{"type":"session_meta","payload":{}}"#.to_owned(),
                event_msg_line("task_started", serde_json::json!({})),
            ]
            .join("\n"),
        );

        assert_eq!(parsed.malformed_lines, 0);
        assert_eq!(parsed.ignored_lines, 3);
        assert_eq!(parsed.parsed_candidates, 1);
        assert_eq!(
            parsed.observed_event_counts["event_msg/exec_command_end"],
            1
        );
        assert_eq!(parsed.observed_event_counts["event_msg/patch_apply_end"], 1);
        assert_eq!(parsed.observed_event_counts["session_meta"], 1);
        assert_eq!(parsed.observed_event_counts["event_msg/task_started"], 1);
    }

    #[test]
    fn response_item_function_call_is_tool_call() {
        let parsed = parse_str(&response_item_line(
            "function_call",
            serde_json::json!({
                "name": "read_file",
                "arguments": "{\"path\":\"README.md\"}"
            }),
        ));

        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].kind, RenderedEntryKind::ToolCall);
        assert_eq!(
            parsed.entries[0].content,
            "Function call: read_file\n{\"path\":\"README.md\"}"
        );
    }

    #[test]
    fn response_item_custom_tool_call_is_tool_call() {
        let parsed = parse_str(&response_item_line(
            "custom_tool_call",
            serde_json::json!({
                "name": "shell_command",
                "input": "cargo test"
            }),
        ));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::ToolCall,
                content: "Custom tool call: shell_command\ncargo test".to_owned()
            }]
        );
    }

    #[test]
    fn response_item_function_call_output_is_tool_result() {
        let parsed = parse_str(&response_item_line(
            "function_call_output",
            serde_json::json!({
                "call_id": "call_1",
                "output": "file contents"
            }),
        ));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::ToolResult,
                content: "file contents".to_owned()
            }]
        );
    }

    #[test]
    fn response_item_custom_tool_call_output_is_tool_result() {
        let parsed = parse_str(&response_item_line(
            "custom_tool_call_output",
            serde_json::json!({
                "call_id": "call_2",
                "content": [{"text": "command output"}]
            }),
        ));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::ToolResult,
                content: "command output".to_owned()
            }]
        );
    }

    #[test]
    fn observed_event_counts_include_tool_lines() {
        let parsed = parse_str(
            &[
                response_item_line("function_call", serde_json::json!({"name": "read_file"})),
                response_item_line(
                    "custom_tool_call_output",
                    serde_json::json!({"output": "done"}),
                ),
            ]
            .join("\n"),
        );

        assert_eq!(
            parsed.observed_event_counts["response_item/function_call"],
            1
        );
        assert_eq!(
            parsed.observed_event_counts["response_item/custom_tool_call_output"],
            1
        );
    }

    #[test]
    fn incomplete_or_unsupported_tool_payloads_are_ignored_safely() {
        let parsed = parse_str(
            &[
                response_item_line("function_call", serde_json::json!({"arguments": "{}"})),
                response_item_line("custom_tool_call_output", serde_json::json!({})),
                response_item_line("unsupported_tool", serde_json::json!({"name": "noop"})),
            ]
            .join("\n"),
        );

        assert_eq!(parsed.malformed_lines, 0);
        assert_eq!(parsed.ignored_lines, 3);
        assert_eq!(parsed.parsed_candidates, 0);
        assert!(parsed.entries.is_empty());
    }

    #[test]
    fn fallback_extracts_role_based_entry_from_root_object() {
        let parsed =
            parse_str(r#"{"role":"user","content":"Please inspect this fallback shape."}"#);

        assert_eq!(parsed.ignored_lines, 0);
        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::You,
                content: "Please inspect this fallback shape.".to_owned()
            }]
        );
    }

    #[test]
    fn fallback_extracts_role_based_entry_from_nested_message() {
        let parsed = parse_str(
            r#"{"type":"unknown","message":{"role":"assistant","content":"Nested assistant text"}}"#,
        );

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: "Nested assistant text".to_owned()
            }]
        );
    }

    #[test]
    fn fallback_extracts_type_based_entry_from_nested_item() {
        let parsed =
            parse_str(r#"{"type":"unknown","item":{"type":"system_note","text":"System note"}}"#);

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::System,
                content: "System note".to_owned()
            }]
        );
    }

    #[test]
    fn fallback_extracts_type_based_tool_call_from_nested_delta() {
        let parsed = parse_str(
            r#"{"type":"unknown","delta":{"type":"function_call","name":"read_file","arguments":{"path":"README.md"}}}"#,
        );

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::ToolCall,
                content: r#"Tool call: read_file
{"path":"README.md"}"#
                    .to_owned()
            }]
        );
    }

    #[test]
    fn fallback_extracts_each_object_inside_root_output_array() {
        let parsed = parse_str(
            r#"{"type":"unknown","output":[{"type":"assistant_message","text":"Assistant output"},{"type":"command_output","output":"Command output"},{"type":"ignored"}]}"#,
        );

        assert_eq!(parsed.ignored_lines, 0);
        assert_eq!(parsed.parsed_candidates, 2);
        assert_eq!(
            parsed.entries,
            vec![
                RenderedEntry {
                    kind: RenderedEntryKind::Codex,
                    content: "Assistant output".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::ToolResult,
                    content: "Command output".to_owned()
                }
            ]
        );
    }

    #[test]
    fn fallback_role_based_developer_entries_remain_ignored() {
        let parsed = parse_str(r#"{"role":"developer","content":"Developer instruction"}"#);

        assert_eq!(parsed.ignored_lines, 1);
        assert_eq!(parsed.parsed_candidates, 0);
        assert!(parsed.entries.is_empty());
    }

    #[test]
    fn focused_envelope_empty_payload_does_not_fall_back() {
        let parsed = parse_str(
            r#"{"type":"event_msg","payload":{"type":"user_message"},"message":{"role":"assistant","content":"fallback should not render"}}"#,
        );

        assert_eq!(parsed.ignored_lines, 1);
        assert_eq!(parsed.parsed_candidates, 0);
        assert!(parsed.entries.is_empty());
    }

    #[test]
    fn realistic_inline_fixture_verifies_parser_output_contract_before_api() {
        let parsed = parse_str(
            &[
                event_user_message_line(
                    "# AGENTS.md instructions\n<INSTRUCTIONS>sanitized</INSTRUCTIONS>",
                ),
                r#"{"type":"event_msg","payload":{"type":"user_message","id":"user_1","message":"Please inspect this parser."}}"#.to_owned(),
                r#"{"type":"response_item","payload":{"type":"message","id":"user_1","role":"user","content":"Please inspect this parser."}}"#.to_owned(),
                event_agent_message_line("I will inspect it."),
                response_message_line("assistant", "I will inspect it."),
                r#"{"type":"session_meta","payload":{"id":"session_1","model_provider":"openai","cli_version":"1.0.0"}}"#.to_owned(),
                response_item_line(
                    "function_call",
                    serde_json::json!({
                        "call_id": "call_1",
                        "name": "read_file",
                        "arguments": "{\"path\":\"sanitized.jsonl\"}"
                    }),
                ),
                event_msg_line(
                    "exec_command_end",
                    serde_json::json!({
                        "call_id": "call_2",
                        "command": "cargo test",
                        "exit_code": 0,
                        "stdout": "tests ok"
                    }),
                ),
                response_item_line(
                    "function_call_output",
                    serde_json::json!({
                        "call_id": "call_2",
                        "output": "tests ok with details"
                    }),
                ),
                r#"{"type":"assistant_note","text":"Fallback Codex note"}"#.to_owned(),
                r#"{"type":"unknown","output":[{"type":"command_output","output":"Fallback command output"}]}"#.to_owned(),
                r#"{"type":"event_msg","payload":{"type":"unknown","message":"ignored"}}"#.to_owned(),
                r#"{"type":"response_item","payload":{"type":"message","role":"developer","content":"ignored"}}"#.to_owned(),
                "not json".to_owned(),
                String::new(),
            ]
            .join("\n"),
        );

        assert_eq!(parsed.parsed_candidates, 11);
        assert_eq!(parsed.entries.len(), 8);
        assert_eq!(parsed.ignored_lines, 2);
        assert_eq!(parsed.malformed_lines, 1);
        assert_eq!(parsed.observed_event_counts["event_msg/user_message"], 2);
        assert_eq!(parsed.observed_event_counts["response_item/message"], 3);
        assert_eq!(parsed.observed_event_counts["session_meta"], 1);
        assert_eq!(
            parsed.observed_event_counts["response_item/function_call"],
            1
        );
        assert_eq!(
            parsed.observed_event_counts["event_msg/exec_command_end"],
            1
        );
        assert_eq!(
            parsed.observed_event_counts["response_item/function_call_output"],
            1
        );
        assert_eq!(parsed.observed_event_counts["assistant_note"], 1);
        assert_eq!(parsed.observed_event_counts["unknown"], 1);
        assert_eq!(parsed.observed_event_counts["event_msg/unknown"], 1);

        assert_eq!(
            parsed.entries,
            vec![
                RenderedEntry {
                    kind: RenderedEntryKind::Context,
                    content: "AGENTS.md project instructions loaded".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::You,
                    content: "Please inspect this parser.".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::Codex,
                    content: "I will inspect it.".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::System,
                    content: "Session: session_1\nModel provider: openai\nCLI version: 1.0.0"
                        .to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::ToolCall,
                    content: "Function call: read_file\n{\"path\":\"sanitized.jsonl\"}".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::ToolResult,
                    content: "tests ok with details".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::Codex,
                    content: "Fallback Codex note".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::ToolResult,
                    content: "Fallback command output".to_owned()
                },
            ]
        );
    }

    #[test]
    fn parse_file_empty_file_returns_safe_empty_result() {
        let path = temp_file_path("empty");
        fs::write(&path, "").expect("empty fixture file is written");

        let parsed = parse_file(&path).expect("empty file parses safely");
        let _ = fs::remove_file(&path);

        assert_eq!(parsed, ParsedChatLog::empty());
    }

    #[test]
    fn parse_file_non_file_returns_safe_empty_result() {
        let path = temp_file_path("missing");
        let _ = fs::remove_file(&path);

        let parsed = parse_file(&path).expect("missing file parses safely");

        assert_eq!(parsed, ParsedChatLog::empty());
    }

    #[test]
    fn contract_malformed_lines_do_not_affect_ignored_or_observed_counts() {
        let parsed = parse_str(
            &[
                "not json".to_owned(),
                event_user_message_line("still parsed"),
                "{".to_owned(),
            ]
            .join("\n"),
        );

        assert_eq!(parsed.malformed_lines, 2);
        assert_eq!(parsed.ignored_lines, 0);
        assert_eq!(parsed.parsed_candidates, 1);
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.observed_event_counts["event_msg/user_message"], 1);
        assert_eq!(parsed.observed_event_counts.len(), 1);
    }

    #[test]
    fn contract_ignored_valid_json_lines_count_once_per_line() {
        let parsed = parse_str(
            &[
                r#"{"type":"event_msg","payload":{"type":"unknown","message":"ignored"}}"#.to_owned(),
                r#"{"type":"response_item","payload":{"type":"message","role":"developer","content":"ignored"}}"#.to_owned(),
                r#"{"type":"unrecognized"}"#.to_owned(),
            ]
            .join("\n"),
        );

        assert_eq!(parsed.malformed_lines, 0);
        assert_eq!(parsed.ignored_lines, 3);
        assert_eq!(parsed.parsed_candidates, 0);
        assert!(parsed.entries.is_empty());
        assert_eq!(parsed.observed_event_counts["event_msg/unknown"], 1);
        assert_eq!(parsed.observed_event_counts["response_item/message"], 1);
        assert_eq!(parsed.observed_event_counts["unrecognized"], 1);
    }

    #[test]
    fn identical_codex_messages_with_different_stable_ids_are_not_collapsed() {
        let parsed = parse_str(
            &[
                r#"{"type":"response_item","payload":{"type":"message","id":"codex_a","role":"assistant","content":"Repeat Codex"}}"#,
                r#"{"type":"response_item","payload":{"type":"message","id":"codex_b","role":"assistant","content":"Repeat Codex"}}"#,
            ]
            .join("\n"),
        );

        assert_eq!(parsed.parsed_candidates, 2);
        assert_eq!(
            parsed.entries,
            vec![
                RenderedEntry {
                    kind: RenderedEntryKind::Codex,
                    content: "Repeat Codex".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::Codex,
                    content: "Repeat Codex".to_owned()
                }
            ]
        );
    }

    #[test]
    fn user_and_codex_blocks_remain_separate_for_realistic_envelope_shapes() {
        let parsed = parse_str(
            &[
                event_user_message_line("What changed?"),
                response_message_line("assistant", "The parser changed."),
            ]
            .join("\n"),
        );

        assert_eq!(
            parsed.entries,
            vec![
                RenderedEntry {
                    kind: RenderedEntryKind::You,
                    content: "What changed?".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::Codex,
                    content: "The parser changed.".to_owned()
                }
            ]
        );
        assert_eq!(parsed.transcript_blocks()[0].label, "[YOU]");
        assert_eq!(parsed.transcript_blocks()[1].label, "[CODEX]");
    }

    #[test]
    fn filtering_over_parsed_output_preserves_contract_counters() {
        let parsed = parse_str(
            &[
                event_user_message_line("show me"),
                response_message_line("assistant", "visible"),
                response_item_line(
                    "function_call",
                    serde_json::json!({"name": "read_file", "arguments": "{}"}),
                ),
                response_item_line(
                    "function_call_output",
                    serde_json::json!({"call_id": "call_1", "output": "hidden"}),
                ),
                r#"{"type":"session_meta","payload":{"id":"session_1"}}"#.to_owned(),
            ]
            .join("\n"),
        );

        let filtered = parsed.filtered(&ChatEntryFilter {
            show_you: false,
            show_tool_result: false,
            show_meta: false,
            ..ChatEntryFilter::all()
        });

        assert_eq!(filtered.parsed_candidates, parsed.parsed_candidates);
        assert_eq!(filtered.ignored_lines, parsed.ignored_lines);
        assert_eq!(filtered.malformed_lines, parsed.malformed_lines);
        assert_eq!(filtered.observed_event_counts, parsed.observed_event_counts);
        assert_eq!(
            filtered.entries,
            vec![
                RenderedEntry {
                    kind: RenderedEntryKind::Codex,
                    content: "visible".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::ToolCall,
                    content: "Function call: read_file\n{}".to_owned()
                }
            ]
        );
    }

    #[test]
    fn transcript_blocks_over_parsed_output_preserve_entry_contract() {
        let parsed = parse_str(
            &[
                event_user_message_line("hello"),
                response_message_line("assistant", "hi"),
            ]
            .join("\n"),
        );

        assert_eq!(parsed.transcript_blocks().len(), 2);
        assert_eq!(
            parsed.transcript_blocks()[0].entry_type,
            RenderedEntryKind::You
        );
        assert_eq!(parsed.transcript_blocks()[0].label, "[YOU]");
        assert_eq!(parsed.transcript_blocks()[0].title, "[YOU]");
        assert_eq!(parsed.transcript_blocks()[0].content, "hello");
        assert_eq!(
            parsed.transcript_blocks()[1].entry_type,
            RenderedEntryKind::Codex
        );
        assert_eq!(parsed.transcript_blocks()[1].label, "[CODEX]");
        assert_eq!(parsed.transcript_blocks()[1].title, "[CODEX]");
        assert_eq!(parsed.transcript_blocks()[1].content, "hi");
    }

    #[test]
    fn duplicate_user_message_across_event_and_response_shapes_is_rendered_once() {
        let parsed = parse_str(
            &[
                r#"{"type":"event_msg","payload":{"type":"user_message","id":"msg_1","message":"Hello user"}}"#,
                r#"{"type":"response_item","payload":{"type":"message","id":"msg_1","role":"user","content":"Hello user"}}"#,
            ]
            .join("\n"),
        );

        assert_eq!(parsed.parsed_candidates, 2);
        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::You,
                content: "Hello user".to_owned()
            }]
        );
    }

    #[test]
    fn duplicate_codex_message_across_event_and_response_shapes_is_rendered_once() {
        let parsed = parse_str(
            &[
                r#"{"type":"event_msg","payload":{"type":"agent_message","message":"Codex reply"}}"#,
                r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":"Codex reply"}}"#,
            ]
            .join("\n"),
        );

        assert_eq!(parsed.parsed_candidates, 2);
        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: "Codex reply".to_owned()
            }]
        );
    }

    #[test]
    fn duplicate_tool_results_with_same_call_id_prefer_response_source() {
        let parsed = parse_str(
            &[
                event_msg_line(
                    "exec_command_end",
                    serde_json::json!({
                        "call_id": "call_1",
                        "command": "cargo test",
                        "stdout": "short"
                    }),
                ),
                response_item_line(
                    "function_call_output",
                    serde_json::json!({
                        "call_id": "call_1",
                        "output": "longer response output"
                    }),
                ),
            ]
            .join("\n"),
        );

        assert_eq!(parsed.parsed_candidates, 2);
        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::ToolResult,
                content: "longer response output".to_owned()
            }]
        );
    }

    #[test]
    fn duplicate_entries_with_same_stable_id_prefer_longer_content_at_same_priority() {
        let parsed = parse_str(
            &[
                r#"{"type":"response_item","payload":{"type":"message","id":"msg_2","role":"assistant","content":"short"}}"#,
                r#"{"type":"response_item","payload":{"type":"message","id":"msg_2","role":"assistant","content":"longer assistant response"}}"#,
            ]
            .join("\n"),
        );

        assert_eq!(parsed.parsed_candidates, 2);
        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: "longer assistant response".to_owned()
            }]
        );
    }

    #[test]
    fn identical_user_messages_with_different_stable_ids_are_not_collapsed() {
        let parsed = parse_str(
            &[
                r#"{"type":"response_item","payload":{"type":"message","id":"msg_a","role":"user","content":"Repeat me"}}"#,
                r#"{"type":"response_item","payload":{"type":"message","id":"msg_b","role":"user","content":"Repeat me"}}"#,
            ]
            .join("\n"),
        );

        assert_eq!(parsed.parsed_candidates, 2);
        assert_eq!(
            parsed.entries,
            vec![
                RenderedEntry {
                    kind: RenderedEntryKind::You,
                    content: "Repeat me".to_owned()
                },
                RenderedEntry {
                    kind: RenderedEntryKind::You,
                    content: "Repeat me".to_owned()
                }
            ]
        );
    }

    #[test]
    fn adjacent_duplicates_with_matching_timestamps_are_suppressed() {
        let parsed = parse_str(
            &[
                r#"{"role":"assistant","timestamp":"2026-06-18T00:00:00Z","content":"Same text"}"#,
                r#"{"role":"assistant","timestamp":"2026-06-18T00:00:00Z","content":" same   text "}"#,
            ]
            .join("\n"),
        );

        assert_eq!(parsed.parsed_candidates, 2);
        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: "same   text".to_owned()
            }]
        );
    }

    #[test]
    fn identical_fallback_messages_without_documented_duplicate_signal_remain_separate() {
        let parsed = parse_str(
            &[
                r#"{"role":"assistant","content":"Same fallback text"}"#,
                r#"{"role":"assistant","content":"Same fallback text"}"#,
            ]
            .join("\n"),
        );

        assert_eq!(parsed.parsed_candidates, 2);
        assert_eq!(parsed.entries.len(), 2);
    }

    #[test]
    fn counts_malformed_non_empty_lines_and_continues() {
        let parsed = parse_str(
            "\nnot json\n{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\",\"message\":\"still parsed\"}}\n{",
        );

        assert_eq!(parsed.malformed_lines, 2);
        assert_eq!(parsed.ignored_lines, 0);
        assert_eq!(parsed.parsed_candidates, 1);
        assert_eq!(parsed.entries[0].content, "still parsed");
    }

    #[test]
    fn counts_ignored_valid_json_lines_once() {
        let parsed = parse_str(
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"unknown\",\"message\":\"ignored\"}}\n{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"developer\",\"content\":\"ignored\"}}",
        );

        assert_eq!(parsed.malformed_lines, 0);
        assert_eq!(parsed.ignored_lines, 2);
        assert_eq!(parsed.parsed_candidates, 0);
        assert!(parsed.entries.is_empty());
    }

    #[test]
    fn collects_observed_event_counts_for_valid_json_only() {
        let parsed = parse_str(
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\",\"message\":\"one\"}}\n{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\",\"message\":\"two\"}}\n{\"payload\":{\"type\":\"payload_only\"}}\n{\"type\":\"top_only\",\"payload\":{}}\n{\"payload\":{}}\nnot json",
        );

        assert_eq!(parsed.malformed_lines, 1);
        assert_eq!(parsed.observed_event_counts["event_msg/user_message"], 2);
        assert_eq!(parsed.observed_event_counts["payload_only"], 1);
        assert_eq!(parsed.observed_event_counts["top_only"], 1);
        assert_eq!(parsed.observed_event_counts["unknown"], 1);
    }

    #[test]
    fn empty_trimmed_lines_are_not_ignored_or_malformed() {
        let parsed = parse_str("\n  \n\t\n");

        assert_eq!(parsed.malformed_lines, 0);
        assert_eq!(parsed.ignored_lines, 0);
        assert_eq!(parsed.parsed_candidates, 0);
        assert!(parsed.entries.is_empty());
        assert!(parsed.observed_event_counts.is_empty());
    }

    #[test]
    fn classifies_agents_instructions_as_context_summary() {
        let parsed = parse_str(&event_user_message_line(
            "# AGENTS.md instructions\n<INSTRUCTIONS>observe</INSTRUCTIONS>",
        ));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Context,
                content: "AGENTS.md project instructions loaded".to_owned()
            }]
        );
    }

    #[test]
    fn classifies_structured_task_body_as_task_summary() {
        let parsed = parse_str(&response_user_message_line(
            "<environment_context>local</environment_context>\nBuild this",
        ));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::Task,
                content: "Task or prompt instructions loaded".to_owned()
            }]
        );
    }

    #[test]
    fn classifies_ordinary_human_prompt_as_you() {
        let parsed = parse_str(&event_user_message_line(
            "Can you explain how this parser handles malformed JSONL?",
        ));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::You,
                content: "Can you explain how this parser handles malformed JSONL?".to_owned()
            }]
        );
    }

    #[test]
    fn long_ordinary_human_prompt_stays_you() {
        let prompt = "Please review this parser behavior carefully. ".repeat(80);
        let parsed = parse_str(&response_user_message_line(&prompt));

        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].kind, RenderedEntryKind::You);
        assert_eq!(parsed.entries[0].content, prompt.trim());
    }

    #[test]
    fn korean_utf8_user_message_stays_intact() {
        let message = "\u{c548}\u{b155}\u{d558}\u{c138}\u{c694}. JSONL \u{d30c}\u{c11c}\u{b97c} \u{d655}\u{c778}\u{d574} \u{c8fc}\u{c138}\u{c694}.";
        let parsed = parse_str(&event_user_message_line(message));

        assert_eq!(
            parsed.entries,
            vec![RenderedEntry {
                kind: RenderedEntryKind::You,
                content: message.to_owned()
            }]
        );
    }

    #[test]
    fn preserves_unicode_message_content() {
        let parsed = parse_str(&response_message_line(
            "assistant",
            "\u{c548}\u{b155}\u{d558}\u{c138}\u{c694}",
        ));

        assert_eq!(
            parsed.entries[0],
            RenderedEntry {
                kind: RenderedEntryKind::Codex,
                content: "\u{c548}\u{b155}\u{d558}\u{c138}\u{c694}".to_owned()
            }
        );
    }

    #[test]
    fn lossy_reader_replaces_invalid_utf8_without_counting_as_io_error() {
        let input = Cursor::new(vec![
            b'{', b'"', b't', b'y', b'p', b'e', b'"', b':', b'"', b'e', b'v', b'e', b'n', b't',
            b'_', b'm', b's', b'g', b'"', b',', b'"', b'p', b'a', b'y', b'l', b'o', b'a', b'd',
            b'"', b':', b'{', b'"', b't', b'y', b'p', b'e', b'"', b':', b'"', b'u', b's', b'e',
            b'r', b'_', b'm', b'e', b's', b's', b'a', b'g', b'e', b'"', b',', b'"', b'm', b'e',
            b's', b's', b'a', b'g', b'e', b'"', b':', b'"', 0xff, b'"', b'}', b'}',
        ]);

        let parsed = parse_reader(input).expect("invalid utf8 is replaced");

        assert_eq!(parsed.malformed_lines, 0);
        assert_eq!(parsed.entries[0].content, "\u{fffd}");
    }

    #[test]
    fn delegated_handoff_is_observed_without_rendering_a_you_entry() {
        let handoff = concat!(
            "## Referenced ChatGPT conversation:\n",
            "Transport guidance.\n",
            r#"{"conversationId":"conversation-1","title":"Design discussion","priorConversation":{"conversation":[{"role":"user","content":[{"content_type":"text","text":"Earlier request"}]},{"role":"assistant","content":null},{"role":"assistant","content":[{"content_type":"text","text":"Earlier response"}]}]}}"#,
            "\n## My request:\n",
            "Continuing from [Design discussion](chatgpt-conversation://conversation-1): continue."
        );
        let parsed = parse_str(&response_user_message_line(handoff));

        assert_eq!(parsed.parsed_candidates, 1);
        assert!(parsed.entries.is_empty());
        let references = &parsed.session_provenance.referenced_conversations;
        assert_eq!(references.len(), 1);
        assert_eq!(
            references[0].conversation_id.as_deref(),
            Some("conversation-1")
        );
        assert_eq!(references[0].title.as_deref(), Some("Design discussion"));
        assert!(references[0].preview_available);
    }

    #[test]
    fn ambient_context_projects_one_human_request_entry() {
        let ambient = concat!(
            "<in-app-browser-context source=\"ambient-ui-state\">\n",
            "runtime URL and local path\n",
            "</in-app-browser-context>\n\n",
            "## My request:\n",
            "이어서 작업해줘"
        );
        let parsed = parse_str(&response_user_message_line(ambient));

        assert_eq!(parsed.parsed_candidates, 1);
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].kind, RenderedEntryKind::You);
        assert_eq!(parsed.entries[0].content, "이어서 작업해줘");
    }

    #[test]
    fn ordinary_user_message_has_no_session_provenance() {
        let parsed = parse_str(&response_user_message_line("Ordinary request"));

        assert_eq!(parsed.entries[0].content, "Ordinary request");
        assert!(
            parsed
                .session_provenance
                .referenced_conversations
                .is_empty()
        );
    }
}
